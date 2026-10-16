//! Ryzu — isolated skill execution runtime (M3.1).

mod config;
mod lifecycle;
mod mock;
mod pool;
mod runtime;
mod warm_pool;

#[cfg(feature = "docker")]
mod cgroup;
#[cfg(feature = "docker")]
mod heartbeat;

#[cfg(feature = "docker")]
mod docker;

pub use config::{ResourceLimits, SkillRunRequest, SkillRunResult, SkillRunStatus};
pub use lifecycle::{RestartPolicy, run_with_restart};
pub use mock::MockSkillRuntime;
pub use pool::SkillContainerPool;
pub use warm_pool::SkillWarmPool;
pub use runtime::SkillRuntime;

#[cfg(feature = "docker")]
pub use docker::BollardSkillRuntime;
#[cfg(feature = "docker")]
pub use heartbeat::{HeartbeatConfig, HEARTBEAT_MARKER};
