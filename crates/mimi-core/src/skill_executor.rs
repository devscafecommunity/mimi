//! Isolated skill tasks (Ryzu + Odlaguna) — M3.1.3 / M3.2.3.

use std::time::Duration;

use anyhow::Result;
#[cfg(not(feature = "docker-kill"))]
use anyhow::Context;
use serde::{Deserialize, Serialize};

use ryzu_runtime::{ResourceLimits, SkillRunRequest, SkillRunResult, SkillRunStatus};
#[cfg(not(feature = "docker-kill"))]
use ryzu_runtime::{MockSkillRuntime, SkillRuntime};

use crate::odlaguna::OdlagunaWatchdog;
use crate::odlaguna_intervention::InterventionContext;
use crate::skill_supervision::OdlagunaDeadlineError;
#[cfg(not(feature = "docker-kill"))]
use crate::skill_supervision::odlaguna_deadline;

pub const DEFAULT_WORKER_IMAGE: &str = "mimi-worker:latest";

/// JSON in `Task::payload` to run a containerized skill instead of the stub executor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IsolatedSkillPayload {
    pub image: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default = "default_memory_mb")]
    pub memory_mb: u32,
    #[serde(default = "default_cpu_percent")]
    pub cpu_percent: u32,
    #[serde(default = "default_true")]
    pub network_isolated: bool,
}

fn default_memory_mb() -> u32 {
    256
}

fn default_cpu_percent() -> u32 {
    50
}

fn default_true() -> bool {
    true
}

impl IsolatedSkillPayload {
    pub fn try_parse(payload: &[u8]) -> Option<Self> {
        if payload.is_empty() {
            return None;
        }
        serde_json::from_slice(payload).ok()
    }

    pub fn into_request(self, timeout: Duration) -> SkillRunRequest {
        SkillRunRequest {
            image: self.image,
            command: self.command,
            limits: ResourceLimits {
                memory_mb: self.memory_mb,
                cpu_percent: self.cpu_percent,
            },
            network_isolated: self.network_isolated,
            timeout_ms: timeout.as_millis().max(1) as u64,
            heartbeat_interval_secs: None,
        }
    }
}

/// Run an isolated skill under Odlaguna (Bollard when `docker-kill`, else mock).
pub async fn run_isolated_skill_supervised(
    watchdog: &mut OdlagunaWatchdog,
    task_id: &str,
    spec: IsolatedSkillPayload,
    timeout: Duration,
    ctx: InterventionContext<'_>,
) -> Result<SkillRunResult, OdlagunaDeadlineError> {
    let request = spec.into_request(timeout);

    #[cfg(feature = "docker-kill")]
    {
        return crate::skill_supervision::run_bollard_skill_supervised(
            watchdog,
            task_id,
            request,
            ctx,
        )
        .await;
    }

    #[cfg(not(feature = "docker-kill"))]
    {
        odlaguna_deadline(
            watchdog,
            task_id,
            timeout,
            None,
            ctx,
            || async {
                MockSkillRuntime::new()
                    .run(request)
                    .await
                    .context("mock skill run")
            },
        )
        .await
    }
}

pub fn skill_run_succeeded(result: &SkillRunResult) -> Result<()> {
    match result.status {
        SkillRunStatus::Completed => Ok(()),
        SkillRunStatus::ResourceExceeded => {
            Err(anyhow::anyhow!("isolated skill exceeded CPU/RAM limits (cgroup)"))
        },
        SkillRunStatus::HeartbeatLost => {
            Err(anyhow::anyhow!("isolated skill lost worker heartbeat"))
        },
        other => Err(anyhow::anyhow!("isolated skill finished with status {:?}", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_isolated_payload_json() {
        let json = r#"{"image":"mimi-worker:latest","command":["echo","hi"]}"#;
        let p = IsolatedSkillPayload::try_parse(json.as_bytes()).expect("parse");
        assert_eq!(p.image, "mimi-worker:latest");
        assert_eq!(p.command, vec!["echo", "hi"]);
    }

    #[tokio::test]
    async fn mock_supervised_skill_completes() {
        let spec = IsolatedSkillPayload {
            image: "mimi-worker:latest".into(),
            command: vec!["echo".into(), "test".into()],
            memory_mb: 256,
            cpu_percent: 50,
            network_isolated: true,
        };
        let mut wd = OdlagunaWatchdog::new();
        let result = run_isolated_skill_supervised(
            &mut wd,
            "skill-1",
            spec,
            Duration::from_secs(10),
            InterventionContext::default(),
        )
        .await
        .expect("run");
        assert_eq!(result.status, SkillRunStatus::Completed);
    }
}
