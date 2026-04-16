use mimi_core::{config::Config, message::Message};

#[test]
fn test_config_creation() {
    let config = Config::default();
    assert_eq!(config.log_level, "info");
}

#[test]
fn test_message_roundtrip() {
    let payload = serde_json::json!({"test": "value"});
    let msg = Message::new("source", "dest", payload);

    let serialized = serde_json::to_string(&msg).unwrap();
    let deserialized: Message = serde_json::from_str(&serialized).unwrap();

    assert_eq!(msg.id, deserialized.id);
    assert_eq!(msg.source, deserialized.source);
}
