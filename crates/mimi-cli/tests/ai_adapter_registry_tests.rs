use mimi_cli::ai::{AdapterError, AdapterRegistry};
use std::sync::Arc;
use tokio::sync::Mutex;

// Mock adapter for testing
struct MockAdapter {
    name: String,
}

#[async_trait::async_trait]
impl mimi_cli::ai::AiAdapter for MockAdapter {
    async fn initialize(
        &self,
        _params: mimi_cli::ai::adapter::AdapterInitParams,
    ) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(
        &self,
    ) -> mimi_cli::ai::AdapterResult<mimi_cli::ai::adapter::AdapterCapabilities> {
        Ok(mimi_cli::ai::adapter::AdapterCapabilities {
            supports_streaming: false,
            supports_caching: false,
            max_context_tokens: 2048,
            supported_models: vec!["test".to_string()],
        })
    }

    async fn invoke(
        &self,
        _request: mimi_cli::ai::adapter::AiRequest,
    ) -> mimi_cli::ai::AdapterResult<mimi_cli::ai::adapter::AiResponse> {
        Ok(mimi_cli::ai::adapter::AiResponse {
            content: "test response".to_string(),
            model: "test".to_string(),
            tokens_used: 10,
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
async fn test_registry_register_and_get() {
    let registry = AdapterRegistry::new();
    let adapter = Arc::new(Mutex::new(MockAdapter {
        name: "test".to_string(),
    }));

    registry
        .register("test".to_string(), adapter.clone())
        .await
        .unwrap();
    let _retrieved = registry.get("test").await.unwrap();
    assert_eq!(registry.list().await.len(), 1);
}

#[tokio::test]
async fn test_registry_get_nonexistent() {
    let registry = AdapterRegistry::new();
    let result = registry.get("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_registry_remove() {
    let registry = AdapterRegistry::new();
    let adapter = Arc::new(Mutex::new(MockAdapter {
        name: "test".to_string(),
    }));

    registry
        .register("test".to_string(), adapter)
        .await
        .unwrap();
    assert_eq!(registry.list().await.len(), 1);

    registry.remove("test").await.unwrap();
    assert_eq!(registry.list().await.len(), 0);
}

#[tokio::test]
async fn test_registry_list() {
    let registry = AdapterRegistry::new();
    let adapter1 = Arc::new(Mutex::new(MockAdapter {
        name: "test1".to_string(),
    }));
    let adapter2 = Arc::new(Mutex::new(MockAdapter {
        name: "test2".to_string(),
    }));

    registry
        .register("adapter1".to_string(), adapter1)
        .await
        .unwrap();
    registry
        .register("adapter2".to_string(), adapter2)
        .await
        .unwrap();

    let list = registry.list().await;
    assert_eq!(list.len(), 2);
    assert!(list.contains(&"adapter1".to_string()));
    assert!(list.contains(&"adapter2".to_string()));
}
