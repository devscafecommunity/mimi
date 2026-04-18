use actix_web_actors::ws;
use serde_json::json;
use std::fmt;

/// WebSocket-specific error type
#[derive(Debug)]
pub enum WsError {
    InvalidMessage(String),
    NotSubscribed(String),
    InvalidChannel(String),
    ConnectionClosed,
}

impl fmt::Display for WsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WsError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            WsError::NotSubscribed(channel) => write!(f, "Not subscribed to channel: {}", channel),
            WsError::InvalidChannel(channel) => write!(f, "Invalid channel: {}", channel),
            WsError::ConnectionClosed => write!(f, "WebSocket connection closed"),
        }
    }
}

impl From<WsError> for ws::Message {
    fn from(err: WsError) -> Self {
        let error_response = json!({
            "event_type": "error",
            "error_message": err.to_string()
        });
        ws::Message::Text(error_response.to_string().into())
    }
}
