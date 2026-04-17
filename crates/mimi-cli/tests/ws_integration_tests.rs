#[cfg(test)]
mod ws_tests {
    use mimi_cli::ws::{streaming, validation, WsEvent, WsMessage};
    use serde_json::json;

    #[test]
    fn test_ws_message_deserialization() {
        let json_text = r#"{
            "id": "test-1",
            "msg_type": "subscribe",
            "channel": "execute",
            "payload": {"task": "test task"}
        }"#;

        let msg: WsMessage = serde_json::from_str(json_text).expect("Failed to deserialize");
        assert_eq!(msg.id, "test-1");
        assert_eq!(msg.msg_type, "subscribe");
        assert_eq!(msg.channel, "execute");
    }

    #[test]
    fn test_ws_event_start() {
        let event = WsEvent::start(
            "msg-1".to_string(),
            "execute".to_string(),
            "Starting task".to_string(),
        );

        assert_eq!(event.event_type, "start");
        assert_eq!(event.id, "msg-1");
        assert_eq!(event.channel, "execute");
    }

    #[test]
    fn test_ws_event_progress() {
        let event = WsEvent::progress(
            "msg-1".to_string(),
            "execute".to_string(),
            50,
            "Half complete".to_string(),
        );

        assert_eq!(event.event_type, "progress");
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["data"]["percent"], 50);
    }

    #[test]
    fn test_ws_event_error() {
        let event = WsEvent::error(
            "msg-1".to_string(),
            "execute".to_string(),
            "Task failed".to_string(),
        );

        assert_eq!(event.event_type, "error");
        let json = serde_json::to_value(&event).unwrap();
        assert!(json["data"]["error_message"]
            .as_str()
            .unwrap()
            .contains("Task failed"));
    }

    #[test]
    fn test_ws_event_serialization() {
        let event = WsEvent::start(
            "test-1".to_string(),
            "execute".to_string(),
            "Starting".to_string(),
        );

        let json_str = serde_json::to_string(&event).expect("Failed to serialize");
        assert!(json_str.contains("\"event_type\":\"start\""));
        assert!(json_str.contains("\"channel\":\"execute\""));
    }

    #[test]
    fn test_validate_execute_empty_task() {
        let msg = WsMessage {
            id: "test-1".to_string(),
            msg_type: "execute".to_string(),
            channel: "execute".to_string(),
            payload: json!({"task": ""}),
        };

        let result = validation::validate_ws_execute_request(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_execute_valid() {
        let msg = WsMessage {
            id: "test-2".to_string(),
            msg_type: "execute".to_string(),
            channel: "execute".to_string(),
            payload: json!({
                "task": "valid task",
                "priority": "normal",
                "timeout": 300,
                "format": "json"
            }),
        };

        let result = validation::validate_ws_execute_request(&msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execute_invalid_priority() {
        let msg = WsMessage {
            id: "test-3".to_string(),
            msg_type: "execute".to_string(),
            channel: "execute".to_string(),
            payload: json!({
                "task": "task",
                "priority": "critical"
            }),
        };

        let result = validation::validate_ws_execute_request(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_query_empty_query() {
        let msg = WsMessage {
            id: "test-4".to_string(),
            msg_type: "query".to_string(),
            channel: "query".to_string(),
            payload: json!({"query": ""}),
        };

        let result = validation::validate_ws_query_request(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_query_valid() {
        let msg = WsMessage {
            id: "test-5".to_string(),
            msg_type: "query".to_string(),
            channel: "query".to_string(),
            payload: json!({
                "query": "valid query",
                "limit": 10,
                "format": "json"
            }),
        };

        let result = validation::validate_ws_query_request(&msg);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_query_invalid_limit() {
        let msg = WsMessage {
            id: "test-6".to_string(),
            msg_type: "query".to_string(),
            channel: "query".to_string(),
            payload: json!({
                "query": "query",
                "limit": 2000
            }),
        };

        let result = validation::validate_ws_query_request(&msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_streaming_progress_events() {
        let events =
            streaming::create_progress_stream("msg-1".to_string(), "execute".to_string(), 4);

        assert!(events.len() > 0);
        assert_eq!(events[0].event_type, "start");
        assert!(events.iter().any(|e| e.event_type == "progress"));
    }

    #[test]
    fn test_streaming_result_to_event() {
        let event = streaming::result_to_event(
            "msg-1".to_string(),
            "execute".to_string(),
            "Success".to_string(),
        );

        assert_eq!(event.event_type, "result");
    }

    #[test]
    fn test_streaming_error_to_event() {
        let event = streaming::error_to_event(
            "msg-1".to_string(),
            "execute".to_string(),
            "Error message".to_string(),
        );

        assert_eq!(event.event_type, "error");
    }
}
