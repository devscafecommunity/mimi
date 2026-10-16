//! M2.6 — Pandora ↔ Mimi core (task context, periodic flush).

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::intervention_setup::{connect_zenoh_if_requested, intervention_context};
use crate::pandora_client::{ContextBundle, PandoraClient};
use crate::odlaguna::OdlagunaWatchdog;
use crate::skill_executor::{run_isolated_skill_supervised, skill_run_succeeded, IsolatedSkillPayload};
use crate::skill_supervision::odlaguna_deadline;
use crate::state_machine::{MimiState, StateManager, Task};

/// Zenoh / bus envelope for Pandora-aware tasks (M2.6.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PandoraBusMessage {
    TaskContextRequest {
        task_id: String,
        intent: String,
    },
    TaskContextResponse {
        task_id: String,
        context_id: String,
        prompt_block: String,
    },
    TaskOutcome {
        task_id: String,
        context_id: String,
        success: bool,
        summary: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContextPayload {
    pub intent: String,
    pub context_limit: usize,
    pub max_hops: u32,
}

#[derive(Debug, Clone)]
pub struct TaskRunSummary {
    pub task_id: Uuid,
    pub context_id: String,
    pub bundle: ContextBundle,
    pub success: bool,
    pub error: Option<String>,
}

impl TaskContextPayload {
    pub fn from_task(task: &Task) -> Self {
        if let Ok(payload) = serde_json::from_slice::<TaskContextPayload>(&task.payload) {
            return payload;
        }
        Self {
            intent: task.name.clone(),
            context_limit: 10,
            max_hops: 2,
        }
    }
}

/// Resolve Pandora context before running a task.
pub async fn fetch_task_context(
    pandora: &PandoraClient,
    intent: &str,
    limit: usize,
    max_hops: u32,
) -> Result<ContextBundle> {
    pandora.resolve_context_for_query(intent, limit, max_hops).await
}

/// Record skill/task outcome in the heatmap + graph (M2.6.1).
pub async fn record_task_outcome(
    pandora: &PandoraClient,
    context_id: &str,
    task: &Task,
    success: bool,
    summary: &str,
) -> Result<()> {
    let label = format!(
        "task:{} {} — {}",
        task.name,
        if success { "ok" } else { "fail" },
        summary
    );
    let outcome_id = format!("task:{}:{}", context_id, task.id);
    pandora.touch_context(&outcome_id, Some(&label)).await?;
    if success {
        pandora.store_context(context_id, Some(&task.name)).await?;
    }
    Ok(())
}

/// Flush heatmap to Neo4j with retries (M2.6.2).
pub async fn flush_with_retry(pandora: &PandoraClient, max_attempts: u32) -> Result<u32> {
    let mut attempt = 0u32;
    let mut last_err = None;
    while attempt < max_attempts {
        attempt += 1;
        match pandora.flush_heatmap().await {
            Ok(n) => {
                if n > 0 {
                    debug!("Pandora flush wrote {} nodes (attempt {})", n, attempt);
                }
                return Ok(n);
            },
            Err(e) => {
                last_err = Some(e);
                warn!("Pandora flush attempt {} failed: {}", attempt, last_err.as_ref().unwrap());
                tokio::time::sleep(Duration::from_millis(200 * attempt as u64)).await;
            },
        }
    }
    Err(last_err.unwrap())
}

pub fn flush_interval_from_env() -> Duration {
    let secs = std::env::var("PANDORA_FLUSH_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    Duration::from_secs(secs.max(30))
}

pub fn checkpoint_interval_from_env() -> Duration {
    let secs = std::env::var("MIMI_CHECKPOINT_INTERVAL_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(300);
    Duration::from_secs(secs.max(60))
}

/// Background periodic flush; stops when `shutdown` is signaled (M2.6.2).
pub struct PandoraFlushLoop {
    handle: JoinHandle<()>,
}

impl PandoraFlushLoop {
    pub fn spawn(pandora: Arc<PandoraClient>, period: Duration) -> (Self, watch::Sender<bool>) {
        Self::spawn_with_checkpoint(pandora, None, period)
    }

    pub fn spawn_with_checkpoint(
        pandora: Arc<PandoraClient>,
        state: Option<Arc<StateManager>>,
        period: Duration,
    ) -> (Self, watch::Sender<bool>) {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let handle = tokio::spawn(async move {
            let mut ticker = interval(period);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
            loop {
                ticker.tick().await;
                if *shutdown_rx.borrow() {
                    break;
                }
                if let Err(e) = flush_with_retry(&pandora, 3).await {
                    error!("Periodic Pandora flush failed: {}", e);
                }
                if let Some(state) = &state {
                    if let Err(e) = pandora.save_mimi_checkpoint(state).await {
                        error!("Mimi checkpoint failed: {}", e);
                    }
                }
            }
            info!("Pandora maintenance loop stopped");
        });
        (Self { handle }, shutdown_tx)
    }

    pub async fn stop(self) {
        self.handle.abort();
    }
}

/// Save + restore orchestrator snapshot (M2 state persistence acceptance).
pub async fn checkpoint_mimi_state(
    pandora: &PandoraClient,
    state: &StateManager,
) -> Result<()> {
    pandora.save_mimi_checkpoint(state).await
}

pub async fn restore_mimi_state(pandora: &PandoraClient, state: &StateManager) -> Result<bool> {
    pandora.apply_mimi_checkpoint(state).await
}

/// Run one queued task with Pandora context (M2.6.3).
pub async fn execute_task_with_context(
    state: &StateManager,
    pandora: &PandoraClient,
    task: Task,
) -> Result<TaskRunSummary> {
    let payload = TaskContextPayload::from_task(&task);
    let bundle = fetch_task_context(
        pandora,
        &payload.intent,
        payload.context_limit,
        payload.max_hops,
    )
    .await?;
    let context_id = bundle.query_context_id.clone();
    let task_id = task.id.to_string();
    let lease = task.timeout;
    let isolated = IsolatedSkillPayload::try_parse(&task.payload);
    let zenoh = connect_zenoh_if_requested().await?;
    let intervention = intervention_context(pandora, zenoh.as_deref());

    let run_result: Result<(), anyhow::Error> = if let Some(spec) = isolated {
        let mut watchdog = OdlagunaWatchdog::new();
        let _ = state.transition_to(MimiState::Listening);
        let _ = state.transition_to(MimiState::Processing);
        match run_isolated_skill_supervised(&mut watchdog, &task_id, spec, lease, intervention)
            .await
        {
            Ok(skill_result) => skill_run_succeeded(&skill_result),
            Err(e) => Err(anyhow::anyhow!(e.to_string())),
        }
    } else {
        odlaguna_deadline(
            &mut OdlagunaWatchdog::new(),
            &task_id,
            lease,
            None,
            intervention,
            || async {
                let _ = state.transition_to(MimiState::Listening);
                let _ = state.transition_to(MimiState::Processing);
                match task.execution_model {
                    crate::state_machine::ExecutionModel::Blocking => {
                        state.execute_blocking_task(task.clone()).await
                    },
                    crate::state_machine::ExecutionModel::Async => {
                        state.execute_async_task(task.clone()).await
                    },
                }
            },
        )
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
    };

    let (success, error) = match run_result {
        Ok(()) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };

    let summary = error.clone().unwrap_or_else(|| "completed".to_string());
    record_task_outcome(pandora, &context_id, &task, success, &summary).await?;
    let _ = flush_with_retry(pandora, 2).await;
    let _ = pandora.save_mimi_checkpoint(state).await;

    Ok(TaskRunSummary {
        task_id: task.id,
        context_id,
        bundle,
        success,
        error,
    })
}

/// Dequeue and run with Pandora (queue + context).
pub async fn execute_next_task_with_pandora(
    state: &StateManager,
    pandora: &PandoraClient,
) -> Result<TaskRunSummary> {
    let task = state.dequeue_task()?;
    execute_task_with_context(state, pandora, task).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_machine::{ExecutionModel, TaskPriority, TaskType};

    #[tokio::test]
    async fn task_context_payload_defaults_to_task_name() {
        let task = Task::new(TaskType::Execute, "deploy service");
        let p = TaskContextPayload::from_task(&task);
        assert_eq!(p.intent, "deploy service");
    }

    #[tokio::test]
    async fn execute_isolated_skill_via_task_payload() {
        let state = StateManager::new();
        let pandora = PandoraClient::with_config(crate::pandora_client::Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "x".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("pandora");

        let payload = serde_json::json!({
            "image": "mimi-worker:latest",
            "command": ["echo", "m3"],
        });
        let task = Task::new(TaskType::Execute, "isolated skill").with_payload(
            serde_json::to_vec(&payload).expect("json"),
        );

        let summary = execute_task_with_context(&state, &pandora, task)
            .await
            .expect("summary");
        assert!(summary.success);
    }

    #[tokio::test]
    async fn execute_task_with_context_offline_pandora() {
        let state = StateManager::new();
        let pandora = PandoraClient::with_config(crate::pandora_client::Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "x".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        let task = Task::new(TaskType::Query, "memory heatmap")
            .with_priority(TaskPriority::High)
            .with_execution_model(ExecutionModel::Blocking);

        let summary = execute_task_with_context(&state, &pandora, task)
            .await
            .expect("run");
        assert!(summary.success);
        assert!(!summary.context_id.is_empty());
        assert!(!summary.bundle.prompt_block.is_empty());
    }
}
