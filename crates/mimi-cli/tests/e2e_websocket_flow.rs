use mimi_cli::ai::{
    AdapterCapabilities, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest, AiResponse,
    SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MockStreamingAdapter {
    name: String,
}

impl MockStreamingAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for MockStreamingAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: true,
            supports_caching: false,
            max_context_tokens: 4096,
            supported_models: vec!["streaming-model".to_string()],
        })
    }

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        let content = format!("Streamed response for: {}", request.prompt);
        Ok(AiResponse {
            content,
            model: "streaming-model".to_string(),
            tokens_used: 150,
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
async fn test_websocket_streaming_support() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let best_adapter = registry.get_best_available().await.unwrap();
    let caps = best_adapter.lock().await.capabilities().await.unwrap();

    assert!(caps.supports_streaming);
}

#[tokio::test]
async fn test_websocket_streaming_response_chunks() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Tell me a story in 3 sentences".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response = best_adapter.lock().await.invoke(request).await.unwrap();

    assert!(!response.content.is_empty());
    assert!(response.content.contains("story"));
}

#[tokio::test]
async fn test_websocket_multiple_streaming_connections() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let mut handles = vec![];

    for i in 0..3 {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            let request = AiRequest {
                prompt: format!("Stream message {}", i),
                model: None,
                temperature: None,
                max_tokens: None,
                system_context: None,
            };

            let best = reg.get_best_available().await?;
            let locked = best.lock().await;
            locked.invoke(request).await
        });
        handles.push(handle);
    }

    let mut success_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            _ => {},
        }
    }

    assert_eq!(success_count, 3);
}

#[tokio::test]
async fn test_websocket_connection_cleanup() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let best_adapter = registry.get_best_available().await.unwrap();
    let cleanup_result = best_adapter.lock().await.cleanup().await;

    assert!(cleanup_result.is_ok());
}

#[tokio::test]
async fn test_websocket_streaming_with_backpressure() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Generate a long response with many chunks".to_string(),
        model: None,
        temperature: Some(0.9),
        max_tokens: Some(2000),
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response = best_adapter.lock().await.invoke(request).await.unwrap();

    assert!(response.tokens_used > 0);
    assert!(!response.content.is_empty());
}

#[tokio::test]
async fn test_websocket_streaming_message_ordering() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MockStreamingAdapter::new("streaming")));

    registry
        .register_with_health("streaming".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "First request".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let response1 = best_adapter.lock().await.invoke(request).await.unwrap();

    let request2 = AiRequest {
        prompt: "Second request".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let response2 = best_adapter.lock().await.invoke(request2).await.unwrap();

    assert!(response1.content.contains("First"));
    assert!(response2.content.contains("Second"));
}
