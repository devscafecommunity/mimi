use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message trait for inter-module communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub payload: serde_json::Value,
}

/// Task message for Zenoh bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMessage {
    pub id: String,
    pub payload: String,
    pub priority: u8,
    pub created_at: DateTime<Utc>,
}

impl Message {
    pub fn new(
        source: impl Into<String>,
        destination: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source: source.into(),
            destination: destination.into(),
            payload,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::new("test", "dest", serde_json::json!({"test": "data"}));
        assert_eq!(msg.source, "test");
        assert_eq!(msg.destination, "dest");
        assert!(!msg.id.is_empty());
    }
}
