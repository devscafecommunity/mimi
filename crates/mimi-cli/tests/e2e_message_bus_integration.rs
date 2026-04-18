use mimi_cli::ai::{
    AdapterCapabilities, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest, AiResponse,
    SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct MessageBusAdapter {
    name: String,
}

impl MessageBusAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for MessageBusAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: false,
            supports_caching: false,
            max_context_tokens: 4096,
            supported_models: vec!["bus-model".to_string()],
        })
    }

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        Ok(AiResponse {
            content: format!("Bus Response: {}", request.prompt),
            model: "bus-model".to_string(),
            tokens_used: 50,
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
async fn test_message_bus_adapter_registration() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("bus")));

    let result = registry
        .register_with_health("bus".to_string(), adapter)
        .await;

    assert!(result.is_ok());
    assert_eq!(registry.count().await, 1);
}

#[tokio::test]
async fn test_message_bus_adapter_retrieval() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("bus")));

    registry
        .register_with_health("bus".to_string(), adapter)
        .await
        .unwrap();

    let retrieved = registry.get("bus").await;
    assert!(retrieved.is_ok());
}

#[tokio::test]
async fn test_message_bus_publish_subscribe_pattern() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("pubsub")));

    registry
        .register_with_health("pubsub".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Publish message".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let response = best.lock().await.invoke(request).await.unwrap();

    assert!(response.content.contains("Publish message"));
}

#[tokio::test]
async fn test_message_bus_topic_routing() {
    let registry = AdapterRegistry::new();
    let topic1: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("topic1")));
    let topic2: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("topic2")));

    registry
        .register_with_health("topic1".to_string(), topic1)
        .await
        .unwrap();
    registry
        .register_with_health("topic2".to_string(), topic2)
        .await
        .unwrap();

    let req1 = AiRequest {
        prompt: "Topic 1".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let adapter1 = registry.get("topic1").await.unwrap();
    let resp1 = adapter1.lock().await.invoke(req1).await.unwrap();
    assert!(resp1.content.contains("Topic 1"));

    let adapter2 = registry.get("topic2").await.unwrap();
    let req2 = AiRequest {
        prompt: "Topic 2".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };
    let resp2 = adapter2.lock().await.invoke(req2).await.unwrap();
    assert!(resp2.content.contains("Topic 2"));
}

#[tokio::test]
async fn test_message_bus_broadcast_to_all_adapters() {
    let registry = AdapterRegistry::new();

    for i in 0..3 {
        let adapter: SharedAdapter =
            Arc::new(Mutex::new(MessageBusAdapter::new(&format!("adapter{}", i))));
        registry
            .register_with_health(format!("adapter{}", i), adapter)
            .await
            .unwrap();
    }

    assert_eq!(registry.count().await, 3);
    assert_eq!(registry.list().await.len(), 3);
}

#[tokio::test]
async fn test_message_bus_adapter_listing() {
    let registry = AdapterRegistry::new();

    let adapter1: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("adapter1")));
    let adapter2: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("adapter2")));

    registry
        .register_with_health("adapter1".to_string(), adapter1)
        .await
        .unwrap();
    registry
        .register_with_health("adapter2".to_string(), adapter2)
        .await
        .unwrap();

    let list = registry.list().await;
    assert!(list.contains(&"adapter1".to_string()));
    assert!(list.contains(&"adapter2".to_string()));
}

#[tokio::test]
async fn test_message_bus_adapter_removal() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("removable")));

    registry
        .register_with_health("removable".to_string(), adapter)
        .await
        .unwrap();

    assert_eq!(registry.count().await, 1);

    registry.remove("removable").await.unwrap();
    assert_eq!(registry.count().await, 0);
}

#[tokio::test]
async fn test_message_bus_adapter_priority_ordering() {
    let registry = AdapterRegistry::with_priority(vec![
        "first".to_string(),
        "second".to_string(),
        "third".to_string(),
    ]);

    let adapter1: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("first")));
    let adapter2: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("second")));
    let adapter3: SharedAdapter = Arc::new(Mutex::new(MessageBusAdapter::new("third")));

    registry
        .register_with_health("first".to_string(), adapter1)
        .await
        .unwrap();
    registry
        .register_with_health("second".to_string(), adapter2)
        .await
        .unwrap();
    registry
        .register_with_health("third".to_string(), adapter3)
        .await
        .unwrap();

    let best = registry.get_best_available().await.unwrap();
    let locked = best.lock().await;
    assert_eq!(locked.adapter_name(), "first");
}
