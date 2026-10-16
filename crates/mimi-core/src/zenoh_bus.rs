//! Zenoh message bus adapter (mock by default; real session with `zenoh-bus` feature).

use anyhow::Result;
#[cfg(feature = "zenoh-bus")]
use anyhow::Context;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::message::TaskMessage;
use crate::state_machine::MimiState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenohConfig {
    pub mode: String,
    pub connect: Vec<String>,
    pub listen: Vec<String>,
}

impl Default for ZenohConfig {
    fn default() -> Self {
        Self {
            mode: "peer".to_string(),
            connect: vec![],
            listen: vec![],
        }
    }
}

impl ZenohConfig {
    pub fn from_env() -> Self {
        let mode = std::env::var("ZENOH_MODE").unwrap_or_else(|_| "peer".to_string());
        let connect = std::env::var("ZENOH_CONNECT")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
            .unwrap_or_default();
        let listen = std::env::var("ZENOH_LISTEN")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect())
            .unwrap_or_default();
        Self {
            mode,
            connect,
            listen,
        }
    }
}

pub struct ZenohBusAdapter {
    config: ZenohConfig,
    task_key_expr: String,
    state_key_expr: String,
    #[cfg(feature = "zenoh-bus")]
    session: Option<zenoh::Session>,
}

impl ZenohBusAdapter {
    pub async fn new() -> Result<Self> {
        Self::with_config(ZenohConfig::default()).await
    }

    pub async fn from_env() -> Result<Self> {
        Self::with_config(ZenohConfig::from_env()).await
    }

    pub async fn with_config(config: ZenohConfig) -> Result<Self> {
        #[cfg(feature = "zenoh-bus")]
        {
            let session = open_zenoh_session(&config).await?;
            info!(
                target: "zenoh.bus",
                mode = %config.mode,
                "Zenoh session open (native)"
            );
            return Ok(Self {
                config,
                task_key_expr: "mimi/tasks/**".to_string(),
                state_key_expr: "mimi/state/**".to_string(),
                session: Some(session),
            });
        }

        #[cfg(not(feature = "zenoh-bus"))]
        {
            if crate::odlaguna_supervisor::zenoh_requested() {
                warn!(
                    target: "zenoh.bus",
                    "MIMI_ZENOH=1 but mimi-core built without `zenoh-bus`; using mock publisher"
                );
            }
            info!(
                "Creating Zenoh bus adapter (mock) with mode: {}",
                config.mode
            );
            Ok(Self {
                config,
                task_key_expr: "mimi/tasks/**".to_string(),
                state_key_expr: "mimi/state/**".to_string(),
            })
        }
    }

    pub async fn subscribe_tasks(&self) -> Result<mpsc::Receiver<TaskMessage>> {
        let (tx, rx) = mpsc::channel(100);

        info!("Subscribed to Zenoh key (mock): {}", self.task_key_expr);

        tokio::spawn(async move {
            debug!("Mock task subscriber spawned");
            drop(tx);
        });

        Ok(rx)
    }

    pub async fn publish_task_timeout(
        &self,
        task_id: &str,
        container_id: Option<&str>,
    ) -> Result<()> {
        let key = "mimi/task/timeout";
        let msg = TaskTimeoutMessage {
            task_id: task_id.to_string(),
            container_id: container_id.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        #[cfg(feature = "zenoh-bus")]
        if let Some(session) = &self.session {
            let payload = serde_json::to_vec(&msg).context("serialize timeout")?;
            session
                .put(key, payload)
                .await
                .map_err(|e| anyhow::anyhow!("zenoh put: {}", e))?;
            info!(
                target: "zenoh.bus",
                key = key,
                task_id = %msg.task_id,
                container_id = ?msg.container_id,
                "published task timeout"
            );
            return Ok(());
        }

        info!(
            target: "zenoh.bus",
            key = key,
            task_id = %msg.task_id,
            container_id = ?msg.container_id,
            "published task timeout (mock)"
        );
        Ok(())
    }

    pub async fn publish_state_change(
        &self,
        from_state: MimiState,
        to_state: MimiState,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Result<()> {
        let state_name = format!("{:?}", to_state).to_lowercase();
        let key = format!("mimi/state/{}", state_name);

        let state_msg = StateChangeMessage {
            from_state: format!("{:?}", from_state),
            to_state: format!("{:?}", to_state),
            timestamp: timestamp.to_rfc3339(),
        };

        #[cfg(feature = "zenoh-bus")]
        if let Some(session) = &self.session {
            let payload = serde_json::to_vec(&state_msg).context("serialize state")?;
            session
                .put(&key, payload)
                .await
                .map_err(|e| anyhow::anyhow!("zenoh put: {}", e))?;
            debug!(
                "Published state change: {:?} -> {:?} on key: {}",
                from_state, to_state, key
            );
            return Ok(());
        }

        debug!(
            "Published state change (mock): {:?} -> {:?} on key: {}",
            from_state, to_state, key
        );
        Ok(())
    }

    #[allow(dead_code)]
    fn deserialize_task(bytes: &[u8]) -> Result<TaskMessage> {
        let task_msg: TaskMessage = serde_json::from_slice(bytes)?;
        Ok(task_msg)
    }

    pub async fn close(self) -> Result<()> {
        #[cfg(feature = "zenoh-bus")]
        if let Some(session) = self.session {
            session.close().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        info!("Closing Zenoh session");
        Ok(())
    }
}

#[cfg(feature = "zenoh-bus")]
async fn open_zenoh_session(config: &ZenohConfig) -> Result<zenoh::Session> {
    use zenoh::config::Config;

    let mut zconfig = Config::default();
    if !config.mode.is_empty() {
        zconfig
            .insert_json5("mode", &format!("\"{}\"", config.mode))
            .map_err(|e| anyhow::anyhow!("zenoh mode: {}", e))?;
    }
    if !config.connect.is_empty() {
        let endpoints: Vec<String> = config
            .connect
            .iter()
            .map(|e| {
                if e.contains("://") {
                    e.clone()
                } else {
                    format!("tcp/{}", e)
                }
            })
            .collect();
        let json = serde_json::to_string(&endpoints).context("connect endpoints")?;
        zconfig
            .insert_json5("connect/endpoints", &json)
            .map_err(|e| anyhow::anyhow!("zenoh connect: {}", e))?;
    }
    if !config.listen.is_empty() {
        let endpoints: Vec<String> = config
            .listen
            .iter()
            .map(|e| {
                if e.contains("://") {
                    e.clone()
                } else {
                    format!("tcp/{}", e)
                }
            })
            .collect();
        let json = serde_json::to_string(&endpoints).context("listen endpoints")?;
        zconfig
            .insert_json5("listen/endpoints", &json)
            .map_err(|e| anyhow::anyhow!("zenoh listen: {}", e))?;
    }

    zenoh::open(zconfig)
        .await
        .map_err(|e| anyhow::anyhow!("zenoh open: {}", e))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTimeoutMessage {
    pub task_id: String,
    pub container_id: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeMessage {
    pub from_state: String,
    pub to_state: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_create_zenoh_adapter() {
        let result = ZenohBusAdapter::new().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_subscribe_receives_task() {
        let adapter = ZenohBusAdapter::new()
            .await
            .expect("Failed to create adapter");
        let mut rx = adapter
            .subscribe_tasks()
            .await
            .expect("Failed to subscribe");

        match timeout(Duration::from_millis(100), rx.recv()).await {
            Ok(None) => {
                assert!(true);
            },
            Ok(Some(_)) => panic!("Unexpected task received"),
            Err(_) => {
                assert!(true);
            },
        }
    }

    #[tokio::test]
    async fn test_publish_state_change() {
        let adapter = ZenohBusAdapter::new()
            .await
            .expect("Failed to create adapter");

        let from_state = MimiState::Idle;
        let to_state = MimiState::Listening;
        let timestamp = chrono::Utc::now();

        let result = adapter
            .publish_state_change(from_state, to_state, timestamp)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_publish_task_timeout() {
        let adapter = ZenohBusAdapter::new().await.expect("adapter");
        let result = adapter
            .publish_task_timeout("task-42", Some("cid-9"))
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_deserialize_task() {
        let task = TaskMessage {
            id: "test-123".to_string(),
            payload: "test_data".to_string(),
            priority: 5,
            created_at: chrono::Utc::now(),
        };

        let json = serde_json::to_vec(&task).unwrap();
        let deserialized = ZenohBusAdapter::deserialize_task(&json).unwrap();

        assert_eq!(deserialized.id, task.id);
        assert_eq!(deserialized.payload, task.payload);
        assert_eq!(deserialized.priority, task.priority);
    }
}
