use crate::ai::adaptive_timeout::AdaptiveTimeout;
use crate::ai::performance::PerformanceMetrics;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct PerformanceTracker {
    adapters: RwLock<
        HashMap<
            String,
            (
                Arc<RwLock<PerformanceMetrics>>,
                Arc<RwLock<AdaptiveTimeout>>,
            ),
        >,
    >,
}

#[derive(Clone, Debug)]
pub struct PerformanceReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub adapters: Vec<AdapterPerformance>,
    pub overall_health: SystemHealth,
}

#[derive(Clone, Debug)]
pub struct AdapterPerformance {
    pub name: String,
    pub p50_latency_ms: u32,
    pub p95_latency_ms: u32,
    pub p99_latency_ms: u32,
    pub success_rate: f32,
    pub throughput_rps: f32,
    pub current_timeout_ms: u32,
    pub degraded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SystemHealth {
    Healthy,
    Degraded,
    Critical,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            adapters: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, adapter_name: String, baseline_timeout: u32) {
        let metrics = Arc::new(RwLock::new(PerformanceMetrics::new(adapter_name.clone())));
        let timeout = Arc::new(RwLock::new(AdaptiveTimeout::new(
            adapter_name.clone(),
            baseline_timeout,
        )));

        let mut adapters = self.adapters.write().unwrap();
        adapters.insert(adapter_name, (metrics, timeout));
    }

    pub fn record_success(&self, adapter_name: &str, latency_ms: u32) -> Result<(), String> {
        let adapters = self.adapters.read().unwrap();
        if let Some((metrics, _)) = adapters.get(adapter_name) {
            metrics.write().unwrap().record_success(latency_ms);
            Ok(())
        } else {
            Err(format!("Adapter {} not registered", adapter_name))
        }
    }

    pub fn record_failure(&self, adapter_name: &str) -> Result<(), String> {
        let adapters = self.adapters.read().unwrap();
        if let Some((metrics, _)) = adapters.get(adapter_name) {
            metrics.write().unwrap().record_failure();
            Ok(())
        } else {
            Err(format!("Adapter {} not registered", adapter_name))
        }
    }

    pub fn get_timeout(&self, adapter_name: &str) -> Result<u32, String> {
        let adapters = self.adapters.read().unwrap();
        if let Some((_, timeout)) = adapters.get(adapter_name) {
            Ok(timeout.read().unwrap().get_timeout_ms())
        } else {
            Err(format!("Adapter {} not registered", adapter_name))
        }
    }

    pub fn get_metrics(
        &self,
        adapter_name: &str,
    ) -> Result<Arc<RwLock<PerformanceMetrics>>, String> {
        let adapters = self.adapters.read().unwrap();
        if let Some((metrics, _)) = adapters.get(adapter_name) {
            Ok(Arc::clone(metrics))
        } else {
            Err(format!("Adapter {} not registered", adapter_name))
        }
    }

    pub fn get_performance_report(&self) -> PerformanceReport {
        let adapters = self.adapters.read().unwrap();
        let mut report_adapters = Vec::new();

        for (name, (metrics, timeout)) in adapters.iter() {
            let m = metrics.read().unwrap();
            let t = timeout.read().unwrap();

            report_adapters.push(AdapterPerformance {
                name: name.clone(),
                p50_latency_ms: m.percentile(50.0).unwrap_or(0),
                p95_latency_ms: m.percentile(95.0).unwrap_or(0),
                p99_latency_ms: m.percentile(99.0).unwrap_or(0),
                success_rate: m.success_rate(),
                throughput_rps: m.throughput_rps,
                current_timeout_ms: t.get_timeout_ms(),
                degraded: m.is_degraded(),
            });
        }

        // Sort adapters by name for consistent ordering
        report_adapters.sort_by(|a, b| a.name.cmp(&b.name));

        let mut overall_health = SystemHealth::Healthy;
        let mut degraded_count = 0;

        for ap in &report_adapters {
            if ap.degraded {
                degraded_count += 1;
            }
            if ap.success_rate < 50.0 {
                overall_health = SystemHealth::Critical;
            }
        }

        if degraded_count > report_adapters.len() / 2 {
            overall_health = SystemHealth::Degraded;
        }

        PerformanceReport {
            timestamp: chrono::Utc::now(),
            adapters: report_adapters,
            overall_health,
        }
    }

    pub fn recommend_adapter(&self) -> Result<String, String> {
        let adapters = self.adapters.read().unwrap();

        if adapters.is_empty() {
            return Err("No adapters registered".to_string());
        }

        let mut best_adapter = None;
        let mut best_score = -1.0;

        for (name, (metrics, _)) in adapters.iter() {
            let m = metrics.read().unwrap();
            let success_rate = m.success_rate();
            let avg_latency = m.average_latency_ms();

            let score = success_rate - (avg_latency / 100.0);

            if score > best_score {
                best_score = score;
                best_adapter = Some(name.clone());
            }
        }

        best_adapter.ok_or_else(|| "Could not recommend adapter".to_string())
    }

    pub fn update_all_timeouts(&self) -> Result<(), String> {
        let adapters = self.adapters.read().unwrap();

        for (_, (metrics, timeout)) in adapters.iter() {
            let m = metrics.read().unwrap();
            let mut t = timeout.write().unwrap();
            t.adjust_based_on_metrics(&m);
        }

        Ok(())
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_tracker_new() {
        let tracker = PerformanceTracker::new();
        let adapters = tracker.adapters.read().unwrap();
        assert!(adapters.is_empty());
    }

    #[test]
    fn test_register_adapter() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);

        let adapters = tracker.adapters.read().unwrap();
        assert_eq!(adapters.len(), 1);
    }

    #[test]
    fn test_record_success() {
        let tracker = PerformanceTracker::new();
        tracker.register("ollama".to_string(), 30000);

        let result = tracker.record_success("ollama", 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_success_nonexistent_adapter() {
        let tracker = PerformanceTracker::new();
        let result = tracker.record_success("nonexistent", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_failure() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);

        let result = tracker.record_failure("gemini");
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_timeout() {
        let tracker = PerformanceTracker::new();
        tracker.register("ollama".to_string(), 15000);

        let timeout = tracker.get_timeout("ollama").unwrap();
        assert_eq!(timeout, 15000);
    }

    #[test]
    fn test_get_performance_report_healthy() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);
        tracker.record_success("gemini", 100).unwrap();
        tracker.record_success("gemini", 150).unwrap();

        let report = tracker.get_performance_report();
        assert_eq!(report.overall_health, SystemHealth::Healthy);
        assert_eq!(report.adapters.len(), 1);
    }

    #[test]
    fn test_recommend_adapter() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 30000);

        tracker.record_success("gemini", 100).unwrap();
        tracker.record_success("gemini", 100).unwrap();
        tracker.record_failure("ollama").unwrap();

        let recommended = tracker.recommend_adapter().unwrap();
        assert_eq!(recommended, "gemini");
    }

    #[test]
    fn test_recommend_adapter_no_adapters() {
        let tracker = PerformanceTracker::new();
        let result = tracker.recommend_adapter();
        assert!(result.is_err());
    }

    #[test]
    fn test_update_all_timeouts() {
        let tracker = PerformanceTracker::new();
        tracker.register("ollama".to_string(), 100);

        for i in 0..100 {
            tracker.record_success("ollama", 50 + i as u32).ok();
        }

        let result = tracker.update_all_timeouts();
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_adapters_performance_report() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);
        tracker.register("ollama".to_string(), 30000);

        tracker.record_success("gemini", 100).unwrap();
        tracker.record_success("ollama", 200).unwrap();

        let report = tracker.get_performance_report();
        assert_eq!(report.adapters.len(), 2);
    }

    #[test]
    fn test_get_metrics() {
        let tracker = PerformanceTracker::new();
        tracker.register("gemini".to_string(), 30000);

        let metrics = tracker.get_metrics("gemini").unwrap();
        let m = metrics.read().unwrap();
        assert_eq!(m.adapter_name, "gemini");
    }
}
