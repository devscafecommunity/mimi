use mimi_cli::ai::{
    AdapterCapabilities, AdapterConfig, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest,
    AiResponse, SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MockAdapter {
    name: String,
}

impl MockAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for MockAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: false,
            supports_caching: false,
            max_context_tokens: 4096,
            supported_models: vec!["test-model".to_string()],
        })
    }

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        Ok(AiResponse {
            content: format!("Response to: {}", request.prompt),
            model: "test-model".to_string(),
            tokens_used: 100,
            cached: false,
        })
    }

    async fn health_check(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    fn adapter_name(&self) -> String {
        self.name.clone()
    }
}

#[tokio::test]
async fn test_cli_query_basic_flow() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockAdapter::new("test")));

    registry
        .register_with_health("test".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "What is Rust?".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let result = best_adapter.lock().await.invoke(request).await;

    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response.content.contains("What is Rust?"));
    assert_eq!(response.model, "test-model");
}

#[tokio::test]
async fn test_cli_query_with_custom_model() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockAdapter::new("test")));

    registry
        .register_with_health("test".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Explain async".to_string(),
        model: Some("custom-model".to_string()),
        temperature: Some(0.7),
        max_tokens: Some(200),
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let result = best_adapter.lock().await.invoke(request).await;
    assert!(result.is_ok());
    assert!(result.unwrap().content.contains("Explain async"));
}

#[tokio::test]
async fn test_cli_multiple_sequential_queries() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockAdapter::new("test")));

    registry
        .register_with_health("test".to_string(), adapter)
        .await
        .unwrap();

    for i in 0..3 {
        let request = AiRequest {
            prompt: format!("Query {}", i),
            model: None,
            temperature: None,
            max_tokens: None,
            system_context: None,
        };

        let best_adapter = registry.get_best_available().await.unwrap();
        let result = best_adapter.lock().await.invoke(request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().content.contains(&format!("Query {}", i)));
    }
}

#[tokio::test]
async fn test_cli_adapter_selection_by_priority() {
    let registry =
        AdapterRegistry::with_priority(vec!["primary".to_string(), "secondary".to_string()]);

    let primary: SharedAdapter = Arc::new(Mutex::new(MockAdapter::new("primary")));
    let secondary: SharedAdapter = Arc::new(Mutex::new(MockAdapter::new("secondary")));

    registry
        .register_with_health("primary".to_string(), primary)
        .await
        .unwrap();
    registry
        .register_with_health("secondary".to_string(), secondary)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let result = best_adapter.lock().await.invoke(request).await;
    assert!(result.is_ok());
}
