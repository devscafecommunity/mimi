//! Odlaguna ↔ Ryzu intervention (M3.1.3).

use std::time::Duration;

use anyhow::Result;

use crate::odlaguna::{ExpiredTask, OdlagunaWatchdog};
use crate::odlaguna_intervention::{emit_intervention_side_effects, InterventionContext};

#[derive(Debug, thiserror::Error)]
pub enum OdlagunaDeadlineError {
    #[error("task exceeded Odlaguna deadline")]
    Timeout,
    #[error(transparent)]
    Operation(#[from] anyhow::Error),
}

/// Run `operation` under an Odlaguna lease; on deadline calls `intervene_expired_containers` (M3.2.3).
pub async fn odlaguna_deadline<T, F, Fut>(
    watchdog: &mut OdlagunaWatchdog,
    task_id: &str,
    timeout: Duration,
    container_id: Option<String>,
    ctx: InterventionContext<'_>,
    operation: F,
) -> Result<T, OdlagunaDeadlineError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    watchdog.register_task_with_container(task_id, timeout, container_id.clone());
    crate::odlaguna_supervisor::mirror_register_if_active(task_id, timeout, container_id)
        .await;

    match tokio::time::timeout(timeout, operation()).await {
        Ok(Ok(value)) => {
            watchdog.complete_task(task_id);
            crate::odlaguna_supervisor::mirror_complete_if_active(task_id).await;
            let _ = intervene_expired_containers_with(watchdog, ctx).await;
            Ok(value)
        },
        Ok(Err(err)) => {
            watchdog.complete_task(task_id);
            crate::odlaguna_supervisor::mirror_complete_if_active(task_id).await;
            let _ = intervene_expired_containers_with(watchdog, ctx).await;
            Err(OdlagunaDeadlineError::Operation(err))
        },
        Err(_) => {
            let _ = intervene_expired_containers_with(watchdog, ctx).await;
            crate::odlaguna_supervisor::mirror_complete_if_active(task_id).await;
            Err(OdlagunaDeadlineError::Timeout)
        },
    }
}

#[cfg(feature = "docker-kill")]
pub async fn run_bollard_skill_supervised(
    watchdog: &mut OdlagunaWatchdog,
    task_id: &str,
    request: ryzu_runtime::SkillRunRequest,
    ctx: InterventionContext<'_>,
) -> Result<ryzu_runtime::SkillRunResult, OdlagunaDeadlineError> {
    use ryzu_runtime::BollardSkillRuntime;

    let timeout = Duration::from_millis(request.timeout_ms.max(1));
    let rt = BollardSkillRuntime::connect().map_err(OdlagunaDeadlineError::Operation)?;
    let container_id = rt
        .start_skill_container(&request)
        .await
        .map_err(OdlagunaDeadlineError::Operation)?;

    odlaguna_deadline(
        watchdog,
        task_id,
        timeout,
        Some(container_id.clone()),
        ctx,
        || async {
            rt.finish_skill_container(&container_id, &request)
                .await
                .map_err(anyhow::Error::from)
        },
    )
    .await
}

/// Kill Docker containers for expired leases when `docker-kill` feature is enabled.
pub async fn intervene_expired_containers(watchdog: &mut OdlagunaWatchdog) -> Result<usize> {
    intervene_expired_containers_with(watchdog, InterventionContext::default()).await
}

pub async fn intervene_expired_containers_with(
    watchdog: &mut OdlagunaWatchdog,
    ctx: InterventionContext<'_>,
) -> Result<usize> {
    let expired = watchdog.poll_expired();
    intervene_on_expired(&expired, ctx).await?;
    kill_containers_for_expired(&expired).await
}

/// Audit + bus side effects for already-expired tasks (no poll).
pub async fn intervene_on_expired(
    expired: &[ExpiredTask],
    ctx: InterventionContext<'_>,
) -> Result<()> {
    emit_intervention_side_effects(expired, ctx).await
}

pub async fn kill_containers_for_expired(expired: &[ExpiredTask]) -> Result<usize> {
    #[cfg(feature = "docker-kill")]
    {
        let mut killed = 0usize;
        for task in expired {
            if let Some(ref container_id) = task.container_id {
                if let Ok(rt) = ryzu_runtime::BollardSkillRuntime::connect() {
                    if rt.kill_container(container_id).await.is_ok() {
                        killed += 1;
                    }
                }
            }
        }
        return Ok(killed);
    }

    #[cfg(not(feature = "docker-kill"))]
    {
        use tracing::warn;
        for task in expired {
            if let Some(ref container_id) = task.container_id {
                warn!(
                    target: "odlaguna.audit",
                    task_id = %task.task_id,
                    container_id = %container_id,
                    "would kill container (enable mimi-core/docker-kill)"
                );
            }
        }
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn deadline_completes_without_intervention() {
        let mut wd = OdlagunaWatchdog::new();
        let out = odlaguna_deadline(
            &mut wd,
            "t1",
            Duration::from_secs(5),
            None,
            InterventionContext::default(),
            || async { Ok(1) },
        )
        .await
        .expect("ok");
        assert_eq!(out, 1);
        assert!(wd.poll_expired().is_empty());
    }

    #[tokio::test]
    async fn intervene_reports_container_without_docker_kill() {
        let mut wd = OdlagunaWatchdog::new();
        wd.register_task_with_container(
            "t-exp",
            Duration::from_millis(1),
            Some("cid-123".into()),
        );
        std::thread::sleep(Duration::from_millis(5));
        let killed = intervene_expired_containers(&mut wd).await.expect("ok");
        assert_eq!(killed, 0);
    }
}

pub fn expired_container_ids(expired: &[ExpiredTask]) -> Vec<String> {
    expired
        .iter()
        .filter_map(|t| t.container_id.clone())
        .collect()
}
