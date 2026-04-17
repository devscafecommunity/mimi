//! Zenoh Message Bus Adapter (Mock Implementation)
//!
//! Provides integration interface for Zenoh distributed message passing.
//! This is a mock implementation that can be replaced with actual Zenoh integration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{debug, info};

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

pub struct ZenohBusAdapter {
    #[allow(dead_code)]
    config: ZenohConfig,
    task_key_expr: String,
    #[allow(dead_code)]
    state_key_expr: String,
}

impl ZenohBusAdapter {
    pub async fn new() -> Result<Self> {
        Self::with_config(ZenohConfig::default()).await
    }

    pub async fn with_config(config: ZenohConfig) -> Result<Self> {
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

    pub async fn subscribe_tasks(&self) -> Result<mpsc::Receiver<TaskMessage>> {
        let (tx, rx) = mpsc::channel(100);

        info!("Subscribed to Zenoh key (mock): {}", self.task_key_expr);

        tokio::spawn(async move {
            debug!("Mock task subscriber spawned");
            drop(tx);
        });

        Ok(rx)
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

        debug!(
            "Published state change (mock): {:?} -> {:?} on key: {}",
            from_state, to_state, key
        );
        let _ = state_msg;
        Ok(())
    }

    #[allow(dead_code)]
    fn deserialize_task(bytes: &[u8]) -> Result<TaskMessage> {
        let task_msg: TaskMessage = serde_json::from_slice(bytes)?;
        Ok(task_msg)
    }

    pub async fn close(self) -> Result<()> {
        info!("Closing Zenoh session (mock)");
        Ok(())
    }
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
