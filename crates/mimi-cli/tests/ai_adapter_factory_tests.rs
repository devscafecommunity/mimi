use mimi_cli::ai::{AdapterConfig, AdapterFactory};

#[test]
fn test_factory_supported_types() {
    let types = AdapterFactory::supported_types();
    assert!(types.contains(&"gemini"));
    assert!(types.contains(&"ollama"));
}

#[tokio::test]
async fn test_factory_unknown_adapter() {
    let config = AdapterConfig {
        adapter_type: "unknown".to_string(),
        api_key: None,
        endpoint: None,
        timeout_ms: 30000,
        max_retries: 3,
        model: "gpt-3.5".to_string(),
    };

    let result = AdapterFactory::create(&config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_factory_gemini_not_yet_implemented() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("key".to_string()),
        endpoint: None,
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let result = AdapterFactory::create(&config).await;
    assert!(result.is_err());
    // Will change once Gemini adapter is implemented
}

#[tokio::test]
async fn test_factory_ollama_not_yet_implemented() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let result = AdapterFactory::create(&config).await;
    assert!(result.is_err());
    // Will change once Ollama adapter is implemented
}
