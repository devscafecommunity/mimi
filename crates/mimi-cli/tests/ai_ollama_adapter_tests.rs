use mimi_cli::ai::{AdapterConfig, AdapterFactory, AdapterInitParams, AiAdapter, OllamaAdapter};

/// Helper to create local adapter config
fn create_local_config() -> AdapterConfig {
    AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        model: "llama2".to_string(),
        timeout_ms: 5000,
        endpoint: Some("http://localhost:11434".to_string()),
        max_retries: 3,
    }
}

/// Helper to create cloud adapter config
fn create_cloud_config() -> AdapterConfig {
    AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: Some("test-api-key".to_string()),
        model: "llama2".to_string(),
        timeout_ms: 5000,
        endpoint: Some("https://ollama.com".to_string()),
        max_retries: 3,
    }
}

#[test]
fn test_ollama_adapter_factory_creation_local() {
    let config = create_local_config();
    assert_eq!(config.adapter_type, "ollama");
    assert_eq!(config.model, "llama2");
    assert_eq!(config.endpoint, Some("http://localhost:11434".to_string()));
    assert_eq!(config.api_key, None);
}

#[test]
fn test_ollama_adapter_factory_creation_cloud() {
    let config = create_cloud_config();
    assert_eq!(config.adapter_type, "ollama");
    assert_eq!(config.api_key, Some("test-api-key".to_string()));
    assert_eq!(config.endpoint, Some("https://ollama.com".to_string()));
}

#[tokio::test]
async fn test_ollama_adapter_factory_create() {
    let config = create_local_config();
    let result = AdapterFactory::create(&config).await;
    assert!(result.is_ok(), "Factory should create ollama adapter");
}

#[tokio::test]
async fn test_ollama_adapter_factory_unsupported_type() {
    let config = AdapterConfig {
        adapter_type: "unknown".to_string(),
        api_key: None,
        model: "test".to_string(),
        timeout_ms: 5000,
        endpoint: Some("http://localhost:11434".to_string()),
        max_retries: 3,
    };
    let result = AdapterFactory::create(&config).await;
    assert!(
        result.is_err(),
        "Factory should reject unknown adapter type"
    );
}

#[tokio::test]
async fn test_ollama_adapter_capabilities_local() {
    let config = create_local_config();
    let adapter = OllamaAdapter::new(config);
    let caps = adapter.capabilities().await.unwrap();

    assert!(caps.supported_models.contains(&"llama2".to_string()));
    assert!(caps.supported_models.contains(&"codellama".to_string()));
    assert!(!caps.supports_streaming);
    assert!(!caps.supports_caching);
    assert_eq!(caps.max_context_tokens, 4096);
}

#[tokio::test]
async fn test_ollama_adapter_capabilities_cloud() {
    let config = create_cloud_config();
    let adapter = OllamaAdapter::new(config);
    let caps = adapter.capabilities().await.unwrap();

    assert!(caps.supported_models.contains(&"llama2".to_string()));
    assert!(!caps.supports_streaming);
    assert!(!caps.supports_caching);
}

#[tokio::test]
async fn test_ollama_adapter_name() {
    let config = create_local_config();
    let adapter = OllamaAdapter::new(config);
    assert_eq!(adapter.adapter_name(), "ollama");
}

#[tokio::test]
async fn test_ollama_adapter_cleanup() {
    let config = create_local_config();
    let adapter = OllamaAdapter::new(config);
    let result = adapter.cleanup().await;
    assert!(result.is_ok(), "Cleanup should succeed");
}

#[test]
fn test_adapter_factory_supported_types() {
    let types = AdapterFactory::supported_types();
    assert!(types.contains(&"gemini"));
    assert!(types.contains(&"ollama"));
}

#[tokio::test]
async fn test_ollama_adapter_initialization_missing_endpoint() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        model: "llama2".to_string(),
        timeout_ms: 5000,
        endpoint: None,
        max_retries: 3,
    };

    let adapter = OllamaAdapter::new(config);
    let params = AdapterInitParams {
        api_key: String::new(),
        timeout_ms: 5000,
        endpoint: None,
        max_retries: 3,
    };

    let result = adapter.initialize(params).await;
    assert!(
        result.is_err(),
        "Initialization should fail without endpoint"
    );
}

#[tokio::test]
async fn test_ollama_adapter_initialization_cloud_empty_key() {
    let config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        model: "llama2".to_string(),
        timeout_ms: 5000,
        endpoint: Some("https://ollama.com".to_string()),
        max_retries: 3,
    };

    let adapter = OllamaAdapter::new(config);
    let params = AdapterInitParams {
        api_key: String::new(),
        timeout_ms: 5000,
        endpoint: Some("https://ollama.com".to_string()),
        max_retries: 3,
    };

    let result = adapter.initialize(params).await;
    assert!(
        result.is_ok(),
        "Empty API key should use local mode, not cloud mode"
    );
}

#[test]
fn test_ollama_local_vs_cloud_endpoint_difference() {
    let local_config = create_local_config();
    let cloud_config = create_cloud_config();

    assert_eq!(
        local_config.endpoint,
        Some("http://localhost:11434".to_string())
    );
    assert_eq!(
        cloud_config.endpoint,
        Some("https://ollama.com".to_string())
    );

    assert!(local_config.api_key.is_none());
    assert!(cloud_config.api_key.is_some());
}
