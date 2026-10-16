use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, KillContainerOptions, LogOutput, LogsOptions,
    RemoveContainerOptions, StartContainerOptions, StatsOptions, WaitContainerOptions,
};
use bollard::models::HostConfig;
use bollard::Docker;
use futures_util::StreamExt;
use tokio::time::{timeout, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use crate::cgroup::{violation_from_stats, ResourceViolation};
use crate::config::{ResourceLimits, SkillRunRequest, SkillRunResult, SkillRunStatus};
use crate::heartbeat::{watch_heartbeat_loss, HeartbeatConfig};
use crate::lifecycle::RestartPolicy;
use crate::warm_pool::SkillWarmPool;
use crate::runtime::SkillRuntime;

pub struct BollardSkillRuntime {
    docker: Docker,
    pool: SkillWarmPool,
}

impl BollardSkillRuntime {
    pub fn connect() -> Result<Self> {
        Self::with_pool(8)
    }

    pub fn with_pool(max_concurrent: u32) -> Result<Self> {
        Ok(Self {
            docker: Docker::connect_with_local_defaults()
                .context("connect to Docker socket")?,
            pool: SkillWarmPool::new(max_concurrent),
        })
    }

    /// Pre-pull worker image (warm pool M3.1.3).
    pub async fn prewarm_worker_image(&self, image: &str) -> Result<()> {
        self.pool.prewarm_image(&self.docker, image).await
    }

    /// SIGKILL / stop for Odlaguna intervention (M3.1.3).
    pub async fn kill_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .kill_container(container_id, None::<KillContainerOptions<String>>)
            .await
            .context("kill container")?;
        let _ = self
            .docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;
        info!(
            target: "ryzu.audit",
            container_id = %container_id,
            "container killed by watchdog"
        );
        Ok(())
    }

    /// Create and start a skill container; returns Docker id (M3.1.3).
    pub async fn start_skill_container(&self, request: &SkillRunRequest) -> Result<String> {
        request.limits.validate().map_err(anyhow::Error::msg)?;

        let name = format!("mimi-skill-{}", Uuid::new_v4());
        let memory_bytes = (request.limits.memory_mb as i64) * 1024 * 1024;
        let cpu_quota = (request.limits.cpu_percent as i64) * 1000;

        let network_mode = if request.network_isolated {
            Some(String::from("none"))
        } else {
            None
        };

        let host_config = HostConfig {
            memory: Some(memory_bytes),
            cpu_quota: Some(cpu_quota),
            network_mode,
            ..Default::default()
        };

        let cmd = if request.command.is_empty() {
            None
        } else {
            Some(request.command.clone())
        };

        let mut env = Vec::new();
        if let Some(hb) = request.heartbeat_interval_secs {
            env.push(format!("MIMI_HEARTBEAT_INTERVAL={}", hb.max(1)));
        }

        let config = Config {
            image: Some(request.image.clone()),
            cmd,
            env: if env.is_empty() { None } else { Some(env) },
            host_config: Some(host_config),
            ..Default::default()
        };

        let created = self
            .docker
            .create_container(
                Some(CreateContainerOptions {
                    name,
                    platform: None,
                }),
                config,
            )
            .await
            .context("create container")?;

        let id = created.id;
        info!(
            target: "ryzu.audit",
            container_id = %id,
            image = %request.image,
            "skill container created"
        );

        self.docker
            .start_container(&id, None::<StartContainerOptions<String>>)
            .await
            .context("start container")?;

        Ok(id)
    }

    /// Single start → wait → logs → remove (with optional cgroup polling).
    pub async fn run_once(&self, request: &SkillRunRequest) -> Result<SkillRunResult> {
        let id = self.start_skill_container(request).await?;
        self.finish_skill_container(&id, request).await
    }

    /// Pool slot + restart policy (M3.1.3).
    pub async fn run_managed(
        &self,
        request: SkillRunRequest,
        policy: RestartPolicy,
    ) -> Result<SkillRunResult> {
        let _permit = self.pool.acquire().await;
        let mut attempt = 0u32;
        loop {
            match self.run_once(&request).await {
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

    /// Wait for exit, collect logs, remove container (cgroup + heartbeat monitors).
    pub async fn finish_skill_container(
        &self,
        container_id: &str,
        request: &SkillRunRequest,
    ) -> Result<SkillRunResult> {
        let timeout_ms = request.timeout_ms;
        let limits = &request.limits;
        let heartbeat = request.heartbeat_interval_secs;

        let docker = self.docker.clone();
        let cid = container_id.to_string();

        tokio::select! {
            result = self.finish_skill_container_inner(container_id, timeout_ms) => result,
            violation = async {
                let mut stream = docker.stats(
                    &cid,
                    Some(StatsOptions { stream: true, one_shot: false }),
                );
                while let Some(sample) = stream.next().await {
                    let stats = sample.context("docker stats sample")?;
                    if let Some(v) = violation_from_stats(&stats, limits) {
                        return Ok(v);
                    }
                }
                std::future::pending::<Result<ResourceViolation>>().await
            } => {
                warn!(
                    target: "ryzu.audit",
                    container_id = %container_id,
                    violation = ?violation,
                    "resource limit exceeded — killing container"
                );
                let _ = self.kill_container(container_id).await;
                Ok(SkillRunResult {
                    container_id: container_id.to_string(),
                    status: SkillRunStatus::ResourceExceeded,
                    exit_code: None,
                    logs: String::new(),
                })
            },
            hb = async {
                if let Some(interval) = heartbeat {
                    watch_heartbeat_loss(
                        &docker,
                        &cid,
                        HeartbeatConfig {
                            interval_secs: interval,
                            miss_tolerance: 3,
                        },
                    )
                    .await
                } else {
                    std::future::pending::<Result<()>>().await
                }
            } => {
                let _ = hb;
                warn!(
                    target: "ryzu.audit",
                    container_id = %container_id,
                    "heartbeat lost — killing container"
                );
                let _ = self.kill_container(container_id).await;
                Ok(SkillRunResult {
                    container_id: container_id.to_string(),
                    status: SkillRunStatus::HeartbeatLost,
                    exit_code: None,
                    logs: String::new(),
                })
            },
        }
    }

    async fn finish_skill_container_inner(
        &self,
        container_id: &str,
        timeout_ms: u64,
    ) -> Result<SkillRunResult> {
        let wait_fut = self.docker.wait_container(
            container_id,
            None::<WaitContainerOptions<String>>,
        );

        let wait_result = timeout(
            Duration::from_millis(timeout_ms.max(1)),
            wait_fut.collect::<Vec<_>>(),
        )
        .await;

        let (status, exit_code) = match wait_result {
            Ok(events) => {
                let code = events
                    .last()
                    .and_then(|e| e.as_ref().ok())
                    .map(|s| s.status_code);
                (SkillRunStatus::Completed, code)
            },
            Err(_) => {
                let _ = self
                    .docker
                    .kill_container(container_id, None::<KillContainerOptions<String>>)
                    .await;
                (SkillRunStatus::TimedOut, None)
            },
        };

        let mut logs = String::new();
        let log_stream = self.docker.logs(
            container_id,
            Some(LogsOptions::<String> {
                stdout: true,
                stderr: true,
                ..Default::default()
            }),
        );

        let mut log_stream = log_stream;
        while let Some(chunk) = log_stream.next().await {
            if let Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) = chunk {
                logs.push_str(&String::from_utf8_lossy(&message));
            }
        }

        let _ = self
            .docker
            .remove_container(
                container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await;

        info!(
            target: "ryzu.audit",
            container_id = %container_id,
            status = ?status,
            exit_code = ?exit_code,
            "skill container removed"
        );

        Ok(SkillRunResult {
            container_id: container_id.to_string(),
            status,
            exit_code,
            logs,
        })
    }
}

impl SkillRuntime for BollardSkillRuntime {
    async fn run(&self, request: SkillRunRequest) -> Result<SkillRunResult> {
        self.run_managed(request, RestartPolicy::default()).await
    }
}
