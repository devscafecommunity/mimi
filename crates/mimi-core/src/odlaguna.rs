//! Odlaguna watchdog (M3.2) — timeouts, leases, audit events.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OdlagunaState {
    Idle,
    Monitoring,
    Alerting,
    Intervention,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoredTask {
    pub task_id: String,
    pub started_at: DateTime<Utc>,
    pub deadline: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub timestamp: DateTime<Utc>,
    pub task_id: String,
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone)]
struct TaskLease {
    deadline: Instant,
    container_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredTask {
    pub task_id: String,
    pub container_id: Option<String>,
}

pub struct OdlagunaWatchdog {
    state: OdlagunaState,
    tasks: HashMap<String, TaskLease>,
    audit: Vec<AuditEvent>,
    max_audit: usize,
}

impl OdlagunaWatchdog {
    pub fn new() -> Self {
        Self {
            state: OdlagunaState::Idle,
            tasks: HashMap::new(),
            audit: Vec::new(),
            max_audit: 500,
        }
    }

    pub fn state(&self) -> OdlagunaState {
        self.state
    }

    pub fn begin_monitoring(&mut self) {
        if self.state == OdlagunaState::Idle {
            self.state = OdlagunaState::Monitoring;
            self.record_audit("system", "begin_monitoring", "watchdog active");
        }
    }

    pub fn register_task(&mut self, task_id: &str, timeout: Duration) {
        self.register_task_with_container(task_id, timeout, None);
    }

    pub fn register_task_with_container(
        &mut self,
        task_id: &str,
        timeout: Duration,
        container_id: Option<String>,
    ) {
        self.begin_monitoring();
        let container_label = container_id
            .as_deref()
            .unwrap_or("-")
            .to_string();
        self.tasks.insert(
            task_id.to_string(),
            TaskLease {
                deadline: Instant::now() + timeout,
                container_id,
            },
        );
        self.record_audit(
            task_id,
            "register",
            format!(
                "timeout={}ms container={}",
                timeout.as_millis(),
                container_label
            ),
        );
    }

    /// Attach a Docker container id after the worker starts (M3.1.3).
    pub fn attach_container(&mut self, task_id: &str, container_id: String) {
        if let Some(lease) = self.tasks.get_mut(task_id) {
            let label = container_id.clone();
            lease.container_id = Some(container_id);
            self.record_audit(
                task_id,
                "attach_container",
                format!("container_id={}", label),
            );
        }
    }

    pub fn complete_task(&mut self, task_id: &str) {
        if self.tasks.remove(task_id).is_some() {
            self.record_audit(task_id, "complete", "within deadline");
        }
        if self.tasks.is_empty() && self.state == OdlagunaState::Monitoring {
            self.state = OdlagunaState::Idle;
        }
    }

    /// Returns tasks that exceeded their deadline (M3.2.3).
    pub fn poll_expired(&mut self) -> Vec<ExpiredTask> {
        let now = Instant::now();
        let expired_ids: Vec<String> = self
            .tasks
            .iter()
            .filter(|(_, lease)| lease.deadline <= now)
            .map(|(id, _)| id.clone())
            .collect();

        let mut out = Vec::new();
        for id in expired_ids {
            let lease = self.tasks.remove(&id);
            self.state = OdlagunaState::Intervention;
            warn!(target: "odlaguna.audit", task_id = %id, "task timeout — intervention");
            self.record_audit(&id, "timeout", "deadline exceeded");
            out.push(ExpiredTask {
                task_id: id,
                container_id: lease.and_then(|l| l.container_id),
            });
        }

        if !out.is_empty() {
            self.state = OdlagunaState::Alerting;
        }

        out
    }

    pub fn audit_trail(&self) -> &[AuditEvent] {
        &self.audit
    }

    pub fn shutdown(&mut self) {
        self.state = OdlagunaState::Shutdown;
        self.tasks.clear();
        self.record_audit("system", "shutdown", "watchdog stopped");
    }

    fn record_audit(&mut self, task_id: &str, action: &str, detail: impl Into<String>) {
        let event = AuditEvent {
            timestamp: Utc::now(),
            task_id: task_id.to_string(),
            action: action.to_string(),
            detail: detail.into(),
        };
        info!(
            target: "odlaguna.audit",
            task_id = %event.task_id,
            action = %event.action,
            detail = %event.detail
        );
        if self.audit.len() >= self.max_audit {
            self.audit.remove(0);
        }
        self.audit.push(event);
    }
}

impl Default for OdlagunaWatchdog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn detects_expired_task() {
        let mut wd = OdlagunaWatchdog::new();
        wd.register_task("t1", Duration::from_millis(10));
        sleep(Duration::from_millis(15));
        let expired = wd.poll_expired();
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].task_id, "t1");
        assert_eq!(wd.state(), OdlagunaState::Alerting);
        assert!(wd.audit_trail().iter().any(|e| e.action == "timeout"));
    }

    #[test]
    fn complete_before_deadline() {
        let mut wd = OdlagunaWatchdog::new();
        wd.register_task("t2", Duration::from_secs(60));
        wd.complete_task("t2");
        assert!(wd.poll_expired().is_empty());
    }
}
