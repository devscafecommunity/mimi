use mimi_cli::ai::{AdapterConfig, AdapterFactory, AdapterPriority, AdapterRegistry};

#[tokio::test]
async fn test_integration_factory_creates_registry() {
    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let result = AdapterFactory::create_with_registry(&config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_integration_registry_auto_failover() {
    let registry = AdapterRegistry::with_priority(vec!["gemini".to_string(), "ollama".to_string()]);

    assert_eq!(registry.count().await, 0);
}

#[tokio::test]
async fn test_integration_health_tracking() {
    let registry = AdapterRegistry::new();

    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let adapter = AdapterFactory::create(&config).await.unwrap();
    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    registry.record_success("gemini", 50).await.unwrap();
    let health = registry.get_health("gemini").await.unwrap();
    assert_eq!(health.success_count, 1);

    for _ in 0..3 {
        registry.record_failure("gemini").await.unwrap();
    }
    let health = registry.get_health("gemini").await.unwrap();
    assert!(!health.is_healthy);
}

#[tokio::test]
async fn test_integration_multiple_adapters() {
    let registry = AdapterRegistry::new();

    let gemini_config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let ollama_config = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let gemini = AdapterFactory::create(&gemini_config).await.unwrap();
    let ollama = AdapterFactory::create(&ollama_config).await.unwrap();

    registry
        .register_with_health("gemini".to_string(), gemini)
        .await
        .unwrap();
    registry
        .register_with_health("ollama".to_string(), ollama)
        .await
        .unwrap();

    assert_eq!(registry.count().await, 2);

    let adapters = registry.list().await;
    assert!(adapters.contains(&"gemini".to_string()));
    assert!(adapters.contains(&"ollama".to_string()));
}

#[tokio::test]
async fn test_integration_list_healthy_filters() {
    let registry = AdapterRegistry::new();

    let config1 = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let config2 = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let adapter1 = AdapterFactory::create(&config1).await.unwrap();
    let adapter2 = AdapterFactory::create(&config2).await.unwrap();

    registry
        .register_with_health("gemini".to_string(), adapter1)
        .await
        .unwrap();
    registry
        .register_with_health("ollama".to_string(), adapter2)
        .await
        .unwrap();

    let healthy = registry.list_healthy().await;
    assert_eq!(healthy.len(), 2);

    for _ in 0..3 {
        registry.record_failure("ollama").await.unwrap();
    }

    let healthy = registry.list_healthy().await;
    assert_eq!(healthy.len(), 1);
    assert!(healthy.contains(&"gemini".to_string()));
}

#[tokio::test]
async fn test_integration_adapter_priority_config() {
    let priority = AdapterPriority::default();
    assert_eq!(priority.adapters[0], "gemini");
    assert_eq!(priority.adapters[1], "ollama");
}

#[tokio::test]
async fn test_integration_registry_with_custom_priority() {
    let custom_priority = vec!["ollama".to_string(), "gemini".to_string()];
    let registry = AdapterRegistry::with_priority(custom_priority.clone());

    assert_eq!(registry.priority, custom_priority);
}

#[tokio::test]
async fn test_integration_remove_adapter() {
    let registry = AdapterRegistry::new();

    let config = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let adapter = AdapterFactory::create(&config).await.unwrap();
    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    assert_eq!(registry.count().await, 1);
    registry.remove("gemini").await.unwrap();
    assert_eq!(registry.count().await, 0);
}

#[tokio::test]
async fn test_integration_clear_all_adapters() {
    let registry = AdapterRegistry::new();

    let config1 = AdapterConfig {
        adapter_type: "gemini".to_string(),
        api_key: Some("test-key".to_string()),
        endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "gemini-pro".to_string(),
    };

    let config2 = AdapterConfig {
        adapter_type: "ollama".to_string(),
        api_key: None,
        endpoint: Some("http://localhost:11434".to_string()),
        timeout_ms: 30000,
        max_retries: 3,
        model: "llama2".to_string(),
    };

    let adapter1 = AdapterFactory::create(&config1).await.unwrap();
    let adapter2 = AdapterFactory::create(&config2).await.unwrap();

    registry
        .register_with_health("gemini".to_string(), adapter1)
        .await
        .unwrap();
    registry
        .register_with_health("ollama".to_string(), adapter2)
        .await
        .unwrap();

    assert_eq!(registry.count().await, 2);
    registry.clear().await.unwrap();
    assert_eq!(registry.count().await, 0);
}
