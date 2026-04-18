#[cfg(test)]
mod tests {
    use mimi_cli::ai::{AdapterConfig, AdapterFactory, AdapterRegistry, PerformanceTracker};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_full_performance_monitoring_flow() {
        let tracker = Arc::new(PerformanceTracker::new());

        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 20000);

        for i in 0..50 {
            tracker.record_success("gemini", 100 + i as u32).ok();
            tracker.record_success("ollama", 80 + i as u32).ok();
        }

        let report = tracker.get_performance_report();
        assert_eq!(report.adapters.len(), 2);
        assert_eq!(report.adapters[0].name, "gemini");
    }

    #[tokio::test]
    async fn test_registry_adapter_selection_with_performance() {
        let tracker = Arc::new(PerformanceTracker::new());
        let _registry = AdapterRegistry::new_with_tracker(tracker.clone());

        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 30000);

        tracker.record_success("gemini", 100).ok();
        tracker.record_success("gemini", 100).ok();
        tracker.record_success("ollama", 200).ok();
        tracker.record_failure("ollama").ok();

        let recommended = tracker.recommend_adapter().unwrap();
        assert_eq!(recommended, "gemini");
    }

    #[tokio::test]
    async fn test_factory_with_tracker_initialization() {
        let config = AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key".to_string()),
            endpoint: Some("https://api.gemini.com".to_string()),
            timeout_ms: 30000,
            max_retries: 3,
            model: "gemini-pro".to_string(),
        };

        let registry = AdapterFactory::create_with_registry(&config).await.unwrap();
        let count = registry.count().await;
        assert!(count > 0);
    }

    #[tokio::test]
    async fn test_adaptive_timeout_adjustment() {
        let tracker = Arc::new(PerformanceTracker::new());
        tracker.register("test".to_string(), 100);

        for i in 0..100 {
            tracker.record_success("test", 50 + i as u32).ok();
        }

        tracker.update_all_timeouts().ok();

        let timeout = tracker.get_timeout("test").unwrap();
        assert!(timeout >= 100);
    }

    #[tokio::test]
    async fn test_performance_degradation_detection() {
        let tracker = Arc::new(PerformanceTracker::new());
        tracker.register("gemini".to_string(), 30000);

        for i in 0..50 {
            tracker.record_success("gemini", 100 + i as u32).ok();
        }

        let report = tracker.get_performance_report();
        let gemini_perf = &report.adapters[0];
        assert!(!gemini_perf.degraded);
    }

    #[tokio::test]
    async fn test_multiple_adapter_fallback_chain() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 30000);

        tracker.record_success("gemini", 100).ok();
        tracker.record_success("ollama", 150).ok();

        let gemini_timeout = registry.get_timeout("gemini").unwrap();
        let ollama_timeout = registry.get_timeout("ollama").unwrap();

        assert_eq!(gemini_timeout, 30000);
        assert_eq!(ollama_timeout, 30000);
    }

    #[tokio::test]
    async fn test_performance_report_system_health_calculation() {
        let tracker = Arc::new(PerformanceTracker::new());

        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 30000);

        tracker.record_success("gemini", 100).ok();
        tracker.record_success("ollama", 100).ok();

        let report = tracker.get_performance_report();
        assert_eq!(report.adapters.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_timeout_retrieval_multiple_adapters() {
        let tracker = Arc::new(PerformanceTracker::new());
        let registry = AdapterRegistry::new_with_tracker(tracker.clone());

        tracker.register("gemini".to_string(), 25000);
        tracker.register("ollama".to_string(), 35000);

        let gemini_timeout = registry.get_timeout("gemini").unwrap();
        let ollama_timeout = registry.get_timeout("ollama").unwrap();

        assert_eq!(gemini_timeout, 25000);
        assert_eq!(ollama_timeout, 35000);
    }

    #[tokio::test]
    async fn test_performance_tracker_success_rate_calculation() {
        let tracker = Arc::new(PerformanceTracker::new());
        tracker.register("test".to_string(), 30000);

        for _ in 0..80 {
            tracker.record_success("test", 100).ok();
        }

        for _ in 0..20 {
            tracker.record_failure("test").ok();
        }

        let report = tracker.get_performance_report();
        assert_eq!(report.adapters[0].success_rate, 80.0);
    }
}
