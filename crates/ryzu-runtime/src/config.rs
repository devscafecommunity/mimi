use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_mb: u32,
    pub cpu_percent: u32,
}

impl ResourceLimits {
    pub fn skill_default() -> Self {
        Self {
            memory_mb: 256,
            cpu_percent: 50,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.memory_mb == 0 || self.memory_mb > 4096 {
            return Err("memory_mb must be 1..4096".into());
        }
        if self.cpu_percent == 0 || self.cpu_percent > 100 {
            return Err("cpu_percent must be 1..100".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRunRequest {
    pub image: String,
    pub command: Vec<String>,
    pub limits: ResourceLimits,
    pub network_isolated: bool,
    pub timeout_ms: u64,
    /// When set, worker emits `mimi-heartbeat` lines (see `docker/worker-entry.sh`).
    #[serde(default)]
    pub heartbeat_interval_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillRunStatus {
    Completed,
    TimedOut,
    Failed,
    /// cgroup / Docker stats exceeded configured limits (M3.2.4)
    ResourceExceeded,
    /// Worker heartbeat missing (M3.2.5)
    HeartbeatLost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRunResult {
    pub container_id: String,
    pub status: SkillRunStatus,
    pub exit_code: Option<i64>,
    pub logs: String,
}
