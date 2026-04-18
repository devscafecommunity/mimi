use mimi_cli::ai::{AdapterConfig, AdapterError};

#[test]
fn test_config_validation_empty_adapter_type() {
    let config = AdapterConfig {
        adapter_type: String::new(),
        api_key: Some("key".to_string()),
        endpoint: None,
        timeout_ms: 1000,
        max_retries: 3,
        model: "gpt-3.5".to_string(),
    };

    let result = config.validate();
    assert!(result.is_err());
    match result.unwrap_err() {
        AdapterError::ConfigError(msg) => assert!(msg.contains("adapter_type")),
        _ => panic!("Expected ConfigError"),
    }
}

#[test]
fn test_config_validation_empty_model() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("key".to_string()),
        endpoint: None,
        timeout_ms: 1000,
        max_retries: 3,
        model: String::new(),
    };

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_gemini_requires_api_key() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: None,
        endpoint: None,
        timeout_ms: 1000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_ollama_requires_endpoint() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: None,
        timeout_ms: 1000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_valid_gemini() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("valid-key".to_string()),
        endpoint: None,
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_valid_ollama() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    assert!(config.validate().is_ok());
}

#[test]
fn test_config_timeout_must_be_positive() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 0,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let result = config.validate();
    assert!(result.is_err());
}
