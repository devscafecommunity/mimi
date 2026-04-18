use mimi_cli::ai::{
    gemini_adapter::GeminiAdapter, AdapterConfig, AdapterError, AdapterInitParams, AiAdapter,
    AiRequest, AiResponse,
};

fn create_test_adapter() -> GeminiAdapter {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key-123".to_string()),
        model: "gemini-pro".to_string(),
        timeout_ms: 30000,
        endpoint: None,
        max_retries: 3,
    };
    GeminiAdapter::new(config)
}

#[tokio::test]
async fn test_adapter_initialize_with_valid_key() {
    let adapter = create_test_adapter();

    let params = AdapterInitParams {
        api_key: "test-key-123".to_string(),
        timeout_ms: 30000,
        endpoint: None,
        max_retries: 3,
    };

    let result = adapter.initialize(params).await;
    assert_eq!(adapter.adapter_name(), "gemini");
}

#[tokio::test]
async fn test_adapter_initialize_with_empty_key() {
    let adapter = create_test_adapter();

    let params = AdapterInitParams {
        api_key: String::new(),
        timeout_ms: 30000,
        endpoint: None,
        max_retries: 3,
    };

    let result = adapter.initialize(params).await;
    assert!(result.is_err());
    match result {
        Err(AdapterError::ConfigError(msg)) => {
            assert!(msg.contains("API key cannot be empty"));
        },
        _ => panic!("Expected ConfigError"),
    }
}

#[tokio::test]
async fn test_adapter_capabilities() {
    let adapter = create_test_adapter();

    let caps = adapter.capabilities().await.unwrap();

    assert!(caps.supported_models.contains(&"gemini-pro".to_string()));
    assert_eq!(caps.max_context_tokens, 32000);
    assert!(!caps.supports_streaming);
    assert!(caps.supports_caching);
}

#[tokio::test]
async fn test_adapter_cleanup() {
    let adapter = create_test_adapter();

    let result = adapter.cleanup().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_ai_request_with_all_fields() {
    let request = AiRequest {
        prompt: "What is Rust?".to_string(),
        model: Some("gemini-pro".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(1000),
        system_context: Some("You are a programming expert.".to_string()),
    };

    assert_eq!(request.prompt, "What is Rust?");
    assert_eq!(request.model, Some("gemini-pro".to_string()));
    assert_eq!(request.temperature, Some(0.7));
    assert_eq!(request.max_tokens, Some(1000));
    assert_eq!(
        request.system_context,
        Some("You are a programming expert.".to_string())
    );
}

#[tokio::test]
async fn test_ai_response_creation() {
    let response = AiResponse {
        content: "Rust is a systems programming language.".to_string(),
        model: "gemini-pro".to_string(),
        tokens_used: 42,
        cached: false,
    };

    assert_eq!(response.content, "Rust is a systems programming language.");
    assert_eq!(response.model, "gemini-pro");
    assert_eq!(response.tokens_used, 42);
    assert!(!response.cached);
}

#[test]
fn test_adapter_config_creation() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        model: "gemini-pro".to_string(),
        timeout_ms: 30000,
        endpoint: None,
        max_retries: 3,
    };

    assert_eq!(config.adapter_type, "gemini");
    assert_eq!(config.api_key, Some("test-key".to_string()));
    assert_eq!(config.model, "gemini-pro");
    assert_eq!(config.timeout_ms, 30000);
}
