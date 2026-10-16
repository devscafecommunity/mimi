use std::future::Future;

use anyhow::Result;

use crate::config::{SkillRunRequest, SkillRunResult};

/// Skill container lifecycle (M3.1.3).
pub trait SkillRuntime: Send + Sync {
    fn run(
        &self,
        request: SkillRunRequest,
    ) -> impl Future<Output = Result<SkillRunResult>> + Send;
}
