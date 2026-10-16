use anyhow::Result;
use tokio::time::{sleep, Duration};
use tracing::info;
use uuid::Uuid;

use crate::config::{SkillRunRequest, SkillRunResult, SkillRunStatus};
use crate::runtime::SkillRuntime;

pub struct MockSkillRuntime;

impl MockSkillRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockSkillRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRuntime for MockSkillRuntime {
    async fn run(&self, request: SkillRunRequest) -> Result<SkillRunResult> {
        request.limits.validate().map_err(anyhow::Error::msg)?;
        let id = format!("mock-{}", Uuid::new_v4());
        info!(
            target: "ryzu.audit",
            container_id = %id,
            image = %request.image,
            network_isolated = request.network_isolated,
            memory_mb = request.limits.memory_mb,
            cpu_percent = request.limits.cpu_percent,
            "skill container start"
        );

        sleep(Duration::from_millis(5)).await;

        let logs = format!("mock run: {:?}", request.command);
        info!(
            target: "ryzu.audit",
            container_id = %id,
            status = "Completed",
            "skill container stop"
        );

        Ok(SkillRunResult {
            container_id: id,
            status: SkillRunStatus::Completed,
            exit_code: Some(0),
            logs,
        })
    }
}
