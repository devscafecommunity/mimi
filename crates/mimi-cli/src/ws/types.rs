use serde::{Deserialize, Serialize};

/// WebSocket message sent by client
#[derive(Debug, Deserialize, Clone)]
pub struct WsMessage {
    /// Message ID for correlation
    pub id: String,
    /// Message type: "subscribe", "unsubscribe", "ping"
    pub msg_type: String,
    /// Subscription channel: "execute", "query"
    pub channel: String,
    /// Request payload (task, query, etc.)
    pub payload: serde_json::Value,
}

/// Event streamed from server to client
#[derive(Debug, Serialize, Clone)]
pub struct WsEvent {
    /// Message ID being responded to
    pub id: String,
    /// Event type: "start", "progress", "result", "error", "complete"
    pub event_type: String,
    /// Channel the event came from
    pub channel: String,
    /// Event timestamp (ISO 8601)
    pub timestamp: String,
    /// Event data
    pub data: serde_json::Value,
}

impl WsEvent {
    /// Create a new WebSocket event
    pub fn new(id: String, event_type: String, channel: String, data: serde_json::Value) -> Self {
        WsEvent {
            id,
            event_type,
            channel,
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
        }
    }

    /// Create a "start" event
    pub fn start(id: String, channel: String, description: String) -> Self {
        Self::new(
            id,
            "start".to_string(),
            channel,
            serde_json::json!({
                "message": description,
                "status": "started"
            }),
        )
    }

    /// Create a "progress" event
    pub fn progress(id: String, channel: String, percent: u32, message: String) -> Self {
        Self::new(
            id,
            "progress".to_string(),
            channel,
            serde_json::json!({
                "percent": percent,
                "message": message
            }),
        )
    }

    /// Create a "result" event
    pub fn result(id: String, channel: String, result: serde_json::Value) -> Self {
        Self::new(
            id,
            "result".to_string(),
            channel,
            serde_json::json!({
                "result": result
            }),
        )
    }

    /// Create an "error" event
    pub fn error(id: String, channel: String, error_message: String) -> Self {
        Self::new(
            id,
            "error".to_string(),
            channel,
            serde_json::json!({
                "error_code": "WS_ERROR",
                "error_message": error_message
            }),
        )
    }

    /// Create a "complete" event
    pub fn complete(id: String, channel: String, details: serde_json::Value) -> Self {
        Self::new(
            id,
            "complete".to_string(),
            channel,
            serde_json::json!({
                "message": "Operation completed",
                "details": details
            }),
        )
    }
}

/// Status of a WebSocket connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connected,
    Authenticated,
    Subscribed,
    Error,
    Closed,
}

/// WebSocket connection context
#[derive(Debug, Clone)]
pub struct ConnectionContext {
    pub connection_id: String,
    pub status: ConnectionStatus,
    pub subscribed_channels: Vec<String>,
}
