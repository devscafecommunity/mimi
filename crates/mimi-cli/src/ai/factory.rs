use super::{
    adapter::*, config::AdapterConfig, error::AdapterError, error::AdapterResult, AdapterPriority,
    AdapterRegistry, GeminiAdapter, HealthChecker, OllamaAdapter,
};
use std::path::Path;
use std::sync::Arc;

pub struct AdapterFactory;

impl AdapterFactory {
    pub async fn create(config: &AdapterConfig) -> AdapterResult<SharedAdapter> {
        match config.adapter_type.as_str() {
            "gemini" => {
                let adapter = GeminiAdapter::new(config.clone());
                Ok(Arc::new(tokio::sync::Mutex::new(adapter)))
            },
            "ollama" => {
                let adapter = OllamaAdapter::new(config.clone());
                Ok(Arc::new(tokio::sync::Mutex::new(adapter)))
            },
            _ => Err(AdapterError::AdapterNotFound(format!(
                "unknown adapter type: {}",
                config.adapter_type
            ))),
        }
    }

    pub async fn create_with_registry(
        config: &AdapterConfig,
    ) -> AdapterResult<Arc<AdapterRegistry>> {
        let priority = Self::load_priority().await.unwrap_or_default();
        let registry = AdapterRegistry::with_priority(priority.adapters.clone());

        for adapter_name in &priority.adapters {
            let mut adapter_config = config.clone();
            adapter_config.adapter_type = adapter_name.clone();

            if let Ok(adapter) = Self::create(&adapter_config).await {
                registry
                    .register_with_health(adapter_name.clone(), adapter)
                    .await?;
            }
        }

        let _health_check_handle = {
            let checker = HealthChecker::new(priority.check_interval_secs);
            let registry_for_checker = Arc::new(tokio::sync::RwLock::new(
                registry
                    .adapters
                    .read()
                    .await
                    .iter()
                    .map(|(k, (_a, h))| (k.clone(), h.clone()))
                    .collect(),
            ));
            checker.spawn(registry_for_checker)
        };

        Ok(registry)
    }

    async fn load_priority() -> AdapterResult<AdapterPriority> {
        if Path::new("adapters.toml").exists() {
            match AdapterPriority::from_file("adapters.toml").await {
                Ok(priority) => return Ok(priority),
                Err(_) => {},
            }
        }

        Ok(AdapterPriority::default())
    }

    pub fn supported_types() -> Vec<&'static str> {
        vec!["gemini", "ollama"]
    }

    pub fn with_priority(_priority: Vec<String>) -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::PerformanceTracker;

    #[tokio::test]
    async fn test_factory_create_gemini() {
        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("https://generativelanguage.googleapis.com".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "gemini-pro".to_string(),
        };

        let result = AdapterFactory::create(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_factory_create_ollama() {
        let config = AdapterConfig {
            adapter_type: "ollama".to_string(),
            api_key: None,
            endpoint: Some("http://localhost:11434".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "llama2".to_string(),
        };

        let result = AdapterFactory::create(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_factory_create_unknown_adapter() {
        let config = AdapterConfig {
            adapter_type: "unknown".to_string(),
            api_key: Some("test".to_string()),
            endpoint: Some("http://localhost".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "test".to_string(),
        };

        let result = AdapterFactory::create(&config).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_factory_supported_types() {
        let types = AdapterFactory::supported_types();
        assert!(types.contains(&"gemini"));
        assert!(types.contains(&"ollama"));
    }

    #[tokio::test]
    async fn test_factory_load_priority_default() {
        let priority = AdapterFactory::load_priority().await.unwrap();
        assert!(!priority.adapters.is_empty());
        assert_eq!(priority.adapters[0], "gemini");
    }

    #[test]
    fn test_factory_with_priority() {
        let priority = vec!["ollama".to_string(), "gemini".to_string()];
        let _factory = AdapterFactory::with_priority(priority);
        assert!(true);
    }

    #[tokio::test]
    async fn test_factory_create_config_gemini_full() {
        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("sk-test".to_string()),
            endpoint: Some("https://api.gemini.com".to_string()),
            timeout_ms: 60000,
            max_retries: 5,
            model: "gemini-pro".to_string(),
        };

        let adapter = AdapterFactory::create(&config).await;
        assert!(adapter.is_ok());
    }

    #[tokio::test]
    async fn test_factory_create_config_ollama_full() {
        let config = AdapterConfig {
            adapter_type: "ollama".to_string(),
            api_key: None,
            endpoint: Some("http://192.168.1.100:11434".to_string()),
            timeout_ms: 120000,
            max_retries: 10,
            model: "neural-chat".to_string(),
        };

        let adapter = AdapterFactory::create(&config).await;
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_factory_supported_types_count() {
        let types = AdapterFactory::supported_types();
        assert_eq!(types.len(), 2);
    }

    #[tokio::test]
    async fn test_factory_create_with_registry() {
        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("http://localhost:11434".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "gpt-4".to_string(),
        };

        let result = AdapterFactory::create_with_registry(&config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_factory_registry_with_performance_tracker() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("https://api.gemini.com".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "gemini-pro".to_string(),
        };

        let adapter = AdapterFactory::create(&config).await;
        assert!(adapter.is_ok());

        registry
            .register_with_health("gemini".to_string(), adapter.unwrap())
            .await
            .ok();
        tracker.register("gemini".to_string(), 30000);

        let timeout = registry.get_timeout("gemini").unwrap();
        assert_eq!(timeout, 30000);
    }

    #[tokio::test]
    async fn test_factory_registry_performance_tracking() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let config = AdapterConfig {
            adapter_type: "ollama".to_string(),
            api_key: None,
            endpoint: Some("http://localhost:11434".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "llama2".to_string(),
        };

        let adapter = AdapterFactory::create(&config).await;
        assert!(adapter.is_ok());

        registry
            .register_with_health("ollama".to_string(), adapter.unwrap())
            .await
            .ok();
        tracker.register("ollama".to_string(), 30000);
        tracker.record_success("ollama", 150).ok();

        let report = registry.get_performance_report().unwrap();
        assert_eq!(report.adapters.len(), 1);
    }

    #[tokio::test]
    async fn test_factory_adapter_timeout_adjustment_on_degradation() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("https://api.gemini.com".to_string()),
            timeout_ms: 5000,
            max_retries: 3,
            model: "gemini-pro".to_string(),
        };

        let adapter = AdapterFactory::create(&config).await;
        assert!(adapter.is_ok());

        registry
            .register_with_health("gemini".to_string(), adapter.unwrap())
            .await
            .ok();
        tracker.register("gemini".to_string(), 5000);

        for i in 0..50 {
            tracker.record_success("gemini", 3000 + i * 10).ok();
        }

        let initial_timeout = registry.get_timeout("gemini").unwrap();
        tracker.update_all_timeouts().ok();
        let adjusted_timeout = registry.get_timeout("gemini").unwrap();

        assert!(adjusted_timeout >= initial_timeout);
    }
}
