use serde_json::Value;

use super::error::WsError;
use super::types::WsMessage;

/// Validate WebSocket execute request
pub fn validate_ws_execute_request(msg: &WsMessage) -> Result<(), WsError> {
    let task = msg
        .payload
        .get("task")
        .and_then(Value::as_str)
        .unwrap_or("");

    if task.is_empty() {
        return Err(WsError::InvalidMessage(
            "task field is required and must be non-empty".to_string(),
        ));
    }

    if task.len() > 10_000 {
        return Err(WsError::InvalidMessage(
            "task must be less than 10,000 characters".to_string(),
        ));
    }

    if let Some(priority) = msg.payload.get("priority").and_then(Value::as_str) {
        if !["low", "normal", "high"].contains(&priority) {
            return Err(WsError::InvalidMessage(
                "priority must be one of: low, normal, high".to_string(),
            ));
        }
    }

    if let Some(timeout) = msg.payload.get("timeout").and_then(Value::as_u64) {
        if timeout < 1 || timeout > 3600 {
            return Err(WsError::InvalidMessage(
                "timeout must be between 1 and 3600 seconds".to_string(),
            ));
        }
    }

    if let Some(format) = msg.payload.get("format").and_then(Value::as_str) {
        if !["text", "json", "yaml"].contains(&format) {
            return Err(WsError::InvalidMessage(
                "format must be one of: text, json, yaml".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate WebSocket query request
pub fn validate_ws_query_request(msg: &WsMessage) -> Result<(), WsError> {
    let query = msg
        .payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("");

    if query.is_empty() {
        return Err(WsError::InvalidMessage(
            "query field is required and must be non-empty".to_string(),
        ));
    }

    if query.len() > 10_000 {
        return Err(WsError::InvalidMessage(
            "query must be less than 10,000 characters".to_string(),
        ));
    }

    if let Some(limit) = msg.payload.get("limit").and_then(Value::as_u64) {
        if limit < 1 || limit > 1000 {
            return Err(WsError::InvalidMessage(
                "limit must be between 1 and 1000".to_string(),
            ));
        }
    }

    if let Some(format) = msg.payload.get("format").and_then(Value::as_str) {
        if !["text", "json", "yaml"].contains(&format) {
            return Err(WsError::InvalidMessage(
                "format must be one of: text, json, yaml".to_string(),
            ));
        }
    }

    Ok(())
}
