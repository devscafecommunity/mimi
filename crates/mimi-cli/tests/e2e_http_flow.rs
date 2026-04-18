use mimi_cli::ai::{
    AdapterCapabilities, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest, AiResponse,
    SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MockHttpAdapter {
    name: String,
}

impl MockHttpAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for MockHttpAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: false,
            supports_caching: true,
            max_context_tokens: 32000,
            supported_models: vec!["gemini-pro".to_string(), "gemini-1.5-pro".to_string()],
        })
    }

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        Ok(AiResponse {
            content: format!("HTTP Response: {}", request.prompt),
            model: request.model.unwrap_or_else(|| "gemini-pro".to_string()),
            tokens_used: 250,
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
async fn test_http_post_query_endpoint() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockHttpAdapter::new("gemini")));

    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Explain async/await in Rust".to_string(),
        model: Some("gemini-pro".to_string()),
        temperature: Some(0.5),
        max_tokens: Some(512),
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response = best_adapter.lock().await.invoke(request).await.unwrap();

    assert!(response.content.contains("async/await"));
    assert_eq!(response.model, "gemini-pro");
    assert_eq!(response.tokens_used, 250);
}

#[tokio::test]
async fn test_http_json_response_format() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockHttpAdapter::new("gemini")));

    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "What is REST API?".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response = best_adapter.lock().await.invoke(request).await.unwrap();

    // Verify response structure
    assert!(!response.content.is_empty());
    assert!(!response.model.is_empty());
    assert!(response.tokens_used > 0);
}

#[tokio::test]
async fn test_http_query_with_system_context() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockHttpAdapter::new("gemini")));

    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Who invented Python?".to_string(),
        model: None,
        temperature: Some(0.3),
        max_tokens: Some(100),
        system_context: Some("You are a programming historian".to_string()),
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response = best_adapter.lock().await.invoke(request).await.unwrap();

    assert!(response.content.contains("HTTP Response"));
    assert!(!response.model.is_empty());
}

#[tokio::test]
async fn test_http_adapter_capabilities() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockHttpAdapter::new("gemini")));

    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    let best_adapter = registry.get_best_available().await.unwrap();
    let caps = best_adapter.lock().await.capabilities().await.unwrap();

    assert!(!caps.supports_streaming);
    assert!(caps.supports_caching);
    assert_eq!(caps.max_context_tokens, 32000);
    assert!(caps.supported_models.contains(&"gemini-pro".to_string()));
    assert!(caps
        .supported_models
        .contains(&"gemini-1.5-pro".to_string()));
}

#[tokio::test]
async fn test_http_health_check_endpoint() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockHttpAdapter::new("gemini")));

    registry
        .register_with_health("gemini".to_string(), adapter)
        .await
        .unwrap();

    let best_adapter = registry.get_best_available().await.unwrap();
    let health = best_adapter.lock().await.health_check().await;

    assert!(health.is_ok());
}
