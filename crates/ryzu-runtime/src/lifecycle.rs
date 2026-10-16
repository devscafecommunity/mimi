//! Container restart policy (M3.1.3).

use std::time::Duration;

use anyhow::Result;
use tracing::warn;

use crate::config::{SkillRunRequest, SkillRunResult, SkillRunStatus};

#[derive(Debug, Clone)]
pub struct RestartPolicy {
    pub max_restarts: u32,
    pub base_backoff_ms: u64,
}

impl Default for RestartPolicy {
    fn default() -> Self {
        Self {
            max_restarts: 2,
            base_backoff_ms: 200,
        }
    }
}

impl RestartPolicy {
    pub fn backoff(&self, attempt: u32) -> Duration {
        let exp = attempt.min(6);
        Duration::from_millis(self.base_backoff_ms * 2u64.pow(exp))
    }
}

/// Retry `run_once` on start/runtime errors and on `Failed` status (not timeout / resource).
pub async fn run_with_restart<F, Fut>(
    policy: &RestartPolicy,
    request: &SkillRunRequest,
    mut run_once: F,
) -> Result<SkillRunResult>
where
    F: FnMut(&SkillRunRequest) -> Fut,
    Fut: std::future::Future<Output = Result<SkillRunResult>>,
{
    let mut attempt = 0u32;
    loop {
        match run_once(request).await {
            Ok(result) => match result.status {
                SkillRunStatus::Completed => return Ok(result),
                SkillRunStatus::TimedOut
                | SkillRunStatus::ResourceExceeded
                | SkillRunStatus::HeartbeatLost => return Ok(result),
                SkillRunStatus::Failed => {
                    if attempt >= policy.max_restarts {
                        return Ok(result);
                    }
                },
            },
            Err(err) => {
                if attempt >= policy.max_restarts {
                    return Err(err);
                }
                warn!(
                    target: "ryzu.audit",
                    attempt,
                    error = %err,
                    "skill run failed — retrying"
                );
            },
        }
        attempt += 1;
        tokio::time::sleep(policy.backoff(attempt)).await;
    }
}
