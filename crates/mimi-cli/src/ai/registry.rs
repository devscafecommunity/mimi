use super::{
    adapter::*,
    error::{AdapterError, AdapterResult},
    health::AdapterHealth,
    performance_tracker::PerformanceTracker,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AdapterRegistry {
    pub adapters: RwLock<HashMap<String, (SharedAdapter, Arc<RwLock<AdapterHealth>>)>>,
    pub priority: Vec<String>,
    pub performance_tracker: Option<Arc<PerformanceTracker>>,
}

impl AdapterRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
            priority: vec!["gemini".to_string(), "ollama".to_string()],
            performance_tracker: None,
        })
    }

    pub fn new_with_tracker(tracker: Arc<PerformanceTracker>) -> Arc<Self> {
        Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
            priority: vec!["gemini".to_string(), "ollama".to_string()],
            performance_tracker: Some(tracker),
        })
    }

    pub fn with_priority(priority: Vec<String>) -> Arc<Self> {
        Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
            priority,
            performance_tracker: None,
        })
    }

    pub fn with_priority_and_tracker(
        priority: Vec<String>,
        tracker: Arc<PerformanceTracker>,
    ) -> Arc<Self> {
        Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
            priority,
            performance_tracker: Some(tracker),
        })
    }

    pub async fn register_with_health(
        &self,
        name: String,
        adapter: SharedAdapter,
    ) -> AdapterResult<()> {
        let mut adapters = self.adapters.write().await;
        let health = Arc::new(RwLock::new(AdapterHealth::new(&name)));
        adapters.insert(name, (adapter, health));
        Ok(())
    }

    pub async fn get(&self, name: &str) -> AdapterResult<SharedAdapter> {
        let adapters = self.adapters.read().await;
        adapters
            .get(name)
            .map(|(adapter, _)| adapter.clone())
            .ok_or_else(|| AdapterError::AdapterNotFound(format!("adapter not found: {}", name)))
    }

    pub async fn get_best_available(&self) -> AdapterResult<SharedAdapter> {
        let adapters = self.adapters.read().await;

        for adapter_name in &self.priority {
            if let Some((adapter, health)) = adapters.get(adapter_name) {
                let h = health.read().await;
                if h.is_healthy {
                    return Ok(adapter.clone());
                }
            }
        }

        for (_, (adapter, health)) in adapters.iter() {
            let h = health.read().await;
            if h.is_healthy {
                return Ok(adapter.clone());
            }
        }

        if let Some(first_name) = self.priority.first() {
            if let Some((adapter, _)) = adapters.get(first_name) {
                return Ok(adapter.clone());
            }
        }

        Err(AdapterError::AllAdaptersFailed(
            "no adapters available".to_string(),
        ))
    }

    pub async fn get_health(&self, name: &str) -> AdapterResult<AdapterHealth> {
        let adapters = self.adapters.read().await;
        if let Some((_, health)) = adapters.get(name) {
            let h = health.read().await;
            Ok(h.clone())
        } else {
            Err(AdapterError::AdapterNotFound(format!(
                "adapter not found: {}",
                name
            )))
        }
    }

    pub async fn record_success(&self, name: &str, latency_ms: u32) -> AdapterResult<()> {
        let adapters = self.adapters.read().await;
        if let Some((_, health)) = adapters.get(name) {
            let mut h = health.write().await;
            h.record_success(latency_ms);
            Ok(())
        } else {
            Err(AdapterError::AdapterNotFound(format!(
                "adapter not found: {}",
                name
            )))
        }
    }

    pub async fn record_failure(&self, name: &str) -> AdapterResult<()> {
        let adapters = self.adapters.read().await;
        if let Some((_, health)) = adapters.get(name) {
            let mut h = health.write().await;
            h.record_failure();
            Ok(())
        } else {
            Err(AdapterError::AdapterNotFound(format!(
                "adapter not found: {}",
                name
            )))
        }
    }

    pub async fn list(&self) -> Vec<String> {
        let adapters = self.adapters.read().await;
        adapters.keys().cloned().collect()
    }

    pub async fn list_healthy(&self) -> Vec<String> {
        let adapters = self.adapters.read().await;
        let mut healthy = vec![];
        for (name, (_, health)) in adapters.iter() {
            let h = health.read().await;
            if h.is_healthy {
                healthy.push(name.clone());
            }
        }
        healthy
    }

    pub async fn remove(&self, name: &str) -> AdapterResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.remove(name);
        Ok(())
    }

    pub async fn clear(&self) -> AdapterResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.clear();
        Ok(())
    }

    pub async fn count(&self) -> usize {
        let adapters = self.adapters.read().await;
        adapters.len()
    }

    pub fn get_timeout(&self, adapter_name: &str) -> AdapterResult<u32> {
        if let Some(tracker) = &self.performance_tracker {
            tracker
                .get_timeout(adapter_name)
                .map_err(|e| AdapterError::AllAdaptersFailed(e))
        } else {
            Err(AdapterError::AllAdaptersFailed(
                "performance tracker not initialized".to_string(),
            ))
        }
    }

    pub fn get_performance_report(
        &self,
    ) -> AdapterResult<super::performance_tracker::PerformanceReport> {
        if let Some(tracker) = &self.performance_tracker {
            Ok(tracker.get_performance_report())
        } else {
            Err(AdapterError::AllAdaptersFailed(
                "performance tracker not initialized".to_string(),
            ))
        }
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
            priority: vec!["gemini".to_string(), "ollama".to_string()],
            performance_tracker: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio::sync::Mutex;

    fn create_mock_adapter() -> SharedAdapter {
        Arc::new(Mutex::new(MockAdapter))
    }

    struct MockAdapter;

    #[async_trait]
    impl crate::ai::adapter::AiAdapter for MockAdapter {
        async fn initialize(
            &self,
            _params: crate::ai::adapter::AdapterInitParams,
        ) -> AdapterResult<()> {
            Ok(())
        }

        async fn capabilities(&self) -> AdapterResult<crate::ai::adapter::AdapterCapabilities> {
            Ok(crate::ai::adapter::AdapterCapabilities {
                supports_streaming: false,
                supports_caching: true,
                max_context_tokens: 4096,
                supported_models: vec!["mock".to_string()],
            })
        }

        async fn invoke(
            &self,
            _request: crate::ai::adapter::AiRequest,
        ) -> AdapterResult<crate::ai::adapter::AiResponse> {
            Ok(crate::ai::adapter::AiResponse {
                content: "mock".to_string(),
                model: "mock".to_string(),
                tokens_used: 10,
                cached: false,
            })
        }

        async fn health_check(&self) -> AdapterResult<()> {
            Ok(())
        }

        async fn cleanup(&self) -> AdapterResult<()> {
            Ok(())
        }

        fn adapter_name(&self) -> String {
            "mock".to_string()
        }
    }

    #[tokio::test]
    async fn test_registry_new() {
        let registry = AdapterRegistry::new();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_with_priority() {
        let priority = vec!["ollama".to_string(), "gemini".to_string()];
        let registry = AdapterRegistry::with_priority(priority.clone());
        assert_eq!(registry.priority, priority);
    }

    #[tokio::test]
    async fn test_registry_register_with_health() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_registry_get_health() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        let health = registry.get_health("test").await.unwrap();
        assert!(health.is_healthy);
        assert_eq!(health.adapter_name, "test");
    }

    #[tokio::test]
    async fn test_registry_record_success() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        registry.record_success("test", 50).await.unwrap();
        let health = registry.get_health("test").await.unwrap();
        assert_eq!(health.success_count, 1);
        assert_eq!(health.last_latency_ms, 50);
    }

    #[tokio::test]
    async fn test_registry_record_failure() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        registry.record_failure("test").await.unwrap();
        let health = registry.get_health("test").await.unwrap();
        assert_eq!(health.error_count, 1);
    }

    #[tokio::test]
    async fn test_registry_list_healthy() {
        let registry = AdapterRegistry::new();
        let adapter1 = create_mock_adapter();
        let adapter2 = create_mock_adapter();

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
    }

    #[tokio::test]
    async fn test_registry_get_best_available() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("gemini".to_string(), adapter)
            .await
            .unwrap();

        let best = registry.get_best_available().await;
        assert!(best.is_ok());
    }

    #[tokio::test]
    async fn test_registry_remove() {
        let registry = AdapterRegistry::new();
        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        assert_eq!(registry.count().await, 1);
        registry.remove("test").await.unwrap();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_clear() {
        let registry = AdapterRegistry::new();
        let adapter1 = create_mock_adapter();
        let adapter2 = create_mock_adapter();

        registry
            .register_with_health("test1".to_string(), adapter1)
            .await
            .unwrap();
        registry
            .register_with_health("test2".to_string(), adapter2)
            .await
            .unwrap();

        assert_eq!(registry.count().await, 2);
        registry.clear().await.unwrap();
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_not_found() {
        let registry = AdapterRegistry::new();
        let result = registry.get("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_with_performance_tracker() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let adapter = create_mock_adapter();
        registry
            .register_with_health("test".to_string(), adapter)
            .await
            .unwrap();

        tracker.register("test".to_string(), 30000);
        tracker.record_success("test", 100).ok();

        let timeout = registry.get_timeout("test").unwrap();
        assert_eq!(timeout, 30000);
    }

    #[tokio::test]
    async fn test_registry_get_performance_report() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let adapter = create_mock_adapter();
        registry
            .register_with_health("gemini".to_string(), adapter)
            .await
            .unwrap();

        tracker.register("gemini".to_string(), 30000);
        tracker.record_success("gemini", 100).ok();

        let report = registry.get_performance_report().unwrap();
        assert_eq!(report.adapters.len(), 1);
    }

    #[tokio::test]
    async fn test_registry_priority_with_tracker() {
        let tracker = Arc::new(PerformanceTracker::new());
        let priority = vec!["ollama".to_string(), "gemini".to_string()];
        let registry = AdapterRegistry::with_priority_and_tracker(priority.clone(), tracker);

        assert_eq!(registry.priority, priority);
    }

    #[tokio::test]
    async fn test_registry_get_timeout_no_tracker() {
        let registry = AdapterRegistry::new();
        let result = registry.get_timeout("test");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_multiple_adapters_performance_comparison() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let adapter1 = create_mock_adapter();
        let adapter2 = create_mock_adapter();

        registry
            .register_with_health("gemini".to_string(), adapter1)
            .await
            .unwrap();
        registry
            .register_with_health("ollama".to_string(), adapter2)
            .await
            .unwrap();

        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 20000);

        for i in 0..50 {
            tracker.record_success("gemini", 100 + i).ok();
            tracker.record_success("ollama", 80 + i).ok();
        }

        let gemini_timeout = registry.get_timeout("gemini").unwrap();
        let ollama_timeout = registry.get_timeout("ollama").unwrap();

        assert_eq!(gemini_timeout, 30000);
        assert_eq!(ollama_timeout, 20000);

        let report = registry.get_performance_report().unwrap();
        assert_eq!(report.adapters.len(), 2);
    }
}
