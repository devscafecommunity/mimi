use serde_json::json;

use super::types::WsEvent;

/// Generate a stream of progress events for long-running operations
pub fn create_progress_stream(message_id: String, channel: String, num_steps: u32) -> Vec<WsEvent> {
    let mut events = Vec::new();

    events.push(WsEvent::start(
        message_id.clone(),
        channel.clone(),
        "Starting operation".to_string(),
    ));

    for step in 1..=num_steps {
        let percent = (step * 100) / num_steps;
        events.push(WsEvent::progress(
            message_id.clone(),
            channel.clone(),
            percent,
            format!("Step {}/{}", step, num_steps),
        ));
    }

    events
}

/// Convert a command result into a completion event
pub fn result_to_event(message_id: String, channel: String, result: String) -> WsEvent {
    WsEvent::result(
        message_id,
        channel,
        json!({
            "output": result,
            "status": "success"
        }),
    )
}

/// Convert an error into an error event
pub fn error_to_event(message_id: String, channel: String, error_message: String) -> WsEvent {
    WsEvent::error(message_id, channel, error_message)
}
