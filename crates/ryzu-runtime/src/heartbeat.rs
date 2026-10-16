//! Worker heartbeat monitoring (M3.2.5).

use std::time::{Duration, Instant};

use anyhow::Result;
use bollard::container::{LogOutput, LogsOptions};
use bollard::Docker;
use futures_util::StreamExt;
use tracing::warn;

pub const HEARTBEAT_MARKER: &str = "mimi-heartbeat";

pub struct HeartbeatConfig {
    pub interval_secs: u64,
    pub miss_tolerance: u32,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 2,
            miss_tolerance: 3,
        }
    }
}

/// Poll container logs until heartbeat is missing for too long.
pub async fn watch_heartbeat_loss(
    docker: &Docker,
    container_id: &str,
    config: HeartbeatConfig,
) -> Result<()> {
    let poll = Duration::from_secs(config.interval_secs.max(1));
    let mut last_seen = Instant::now();
    let max_gap = poll * config.miss_tolerance.max(1);

    loop {
        tokio::time::sleep(poll).await;
        if logs_contain_heartbeat(docker, container_id).await? {
            last_seen = Instant::now();
        } else if last_seen.elapsed() >= max_gap {
            warn!(
                target: "ryzu.audit",
                container_id = %container_id,
                "worker heartbeat lost"
            );
            return Err(anyhow::anyhow!("heartbeat lost"));
        }
    }
}

async fn logs_contain_heartbeat(docker: &Docker, container_id: &str) -> Result<bool> {
    let log_stream = docker.logs(
        container_id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: "30".to_string(),
            ..Default::default()
        }),
    );

    let mut log_stream = log_stream;
    while let Some(chunk) = log_stream.next().await {
        let line = match chunk {
            Ok(LogOutput::StdOut { message }) | Ok(LogOutput::StdErr { message }) => {
                String::from_utf8_lossy(&message).to_string()
            },
            _ => continue,
        };
        if line.contains(HEARTBEAT_MARKER) {
            return Ok(true);
        }
    }

    Ok(false)
}
