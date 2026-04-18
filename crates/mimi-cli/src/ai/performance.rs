use std::collections::VecDeque;

pub struct PerformanceMetrics {
    pub adapter_name: String,
    pub latencies: VecDeque<u32>,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub last_recorded: chrono::DateTime<chrono::Utc>,
    pub throughput_rps: f32,
}

impl PerformanceMetrics {
    pub fn new(adapter_name: String) -> Self {
        Self {
            adapter_name,
            latencies: VecDeque::with_capacity(100),
            successful_requests: 0,
            failed_requests: 0,
            last_recorded: chrono::Utc::now(),
            throughput_rps: 0.0,
        }
    }

    pub fn record_success(&mut self, latency_ms: u32) {
        self.successful_requests += 1;
        self.latencies.push_back(latency_ms);

        if self.latencies.len() > 100 {
            self.latencies.pop_front();
        }

        self.last_recorded = chrono::Utc::now();
        self.update_throughput();
    }

    pub fn record_failure(&mut self) {
        self.failed_requests += 1;
        self.last_recorded = chrono::Utc::now();
        self.update_throughput();
    }

    pub fn success_rate(&self) -> f32 {
        let total = self.successful_requests + self.failed_requests;
        if total == 0 {
            return 0.0;
        }
        (self.successful_requests as f32 / total as f32) * 100.0
    }

    pub fn average_latency_ms(&self) -> f32 {
        if self.latencies.is_empty() {
            return 0.0;
        }

        let sum: u32 = self.latencies.iter().sum();
        sum as f32 / self.latencies.len() as f32
    }

    pub fn percentile(&self, p: f32) -> Option<u32> {
        if self.latencies.is_empty() {
            return None;
        }

        let mut sorted: Vec<u32> = self.latencies.iter().copied().collect();
        sorted.sort_unstable();

        let index = ((p / 100.0) * sorted.len() as f32).ceil() as usize;
        sorted.get(index.saturating_sub(1)).copied()
    }

    pub fn is_degraded(&self) -> bool {
        if self.latencies.len() < 20 {
            return false;
        }

        let recent_half = self.latencies.len() / 2;
        let older_half = self.latencies.len() - recent_half;

        let recent_avg: u32 =
            self.latencies.iter().skip(older_half).sum::<u32>() / recent_half.max(1) as u32;

        let older_avg: u32 =
            self.latencies.iter().take(older_half).sum::<u32>() / older_half.max(1) as u32;

        let increase_percent = ((recent_avg as f32 - older_avg as f32) / older_avg as f32) * 100.0;
        increase_percent > 50.0
    }

    pub fn reset(&mut self) {
        self.latencies.clear();
        self.successful_requests = 0;
        self.failed_requests = 0;
        self.throughput_rps = 0.0;
    }

    fn update_throughput(&mut self) {
        let total_requests = self.successful_requests + self.failed_requests;
        if total_requests == 0 {
            self.throughput_rps = 0.0;
            return;
        }

        self.throughput_rps = total_requests as f32 / 60.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics_new() {
        let metrics = PerformanceMetrics::new("gemini".to_string());
        assert_eq!(metrics.adapter_name, "gemini");
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert!(metrics.latencies.is_empty());
    }

    #[test]
    fn test_record_success_single() {
        let mut metrics = PerformanceMetrics::new("ollama".to_string());
        metrics.record_success(100);

        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.latencies.len(), 1);
        assert_eq!(metrics.latencies[0], 100);
    }

    #[test]
    fn test_record_success_multiple() {
        let mut metrics = PerformanceMetrics::new("gemini".to_string());
        metrics.record_success(100);
        metrics.record_success(150);
        metrics.record_success(120);

        assert_eq!(metrics.successful_requests, 3);
        assert_eq!(metrics.latencies.len(), 3);
    }

    #[test]
    fn test_record_failure() {
        let mut metrics = PerformanceMetrics::new("ollama".to_string());
        metrics.record_success(100);
        metrics.record_failure();

        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[test]
    fn test_success_rate_100_percent() {
        let mut metrics = PerformanceMetrics::new("gemini".to_string());
        metrics.record_success(100);
        metrics.record_success(150);

        assert_eq!(metrics.success_rate(), 100.0);
    }

    #[test]
    fn test_success_rate_50_percent() {
        let mut metrics = PerformanceMetrics::new("ollama".to_string());
        metrics.record_success(100);
        metrics.record_failure();

        assert_eq!(metrics.success_rate(), 50.0);
    }

    #[test]
    fn test_success_rate_zero_requests() {
        let metrics = PerformanceMetrics::new("gemini".to_string());
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_average_latency_ms() {
        let mut metrics = PerformanceMetrics::new("ollama".to_string());
        metrics.record_success(100);
        metrics.record_success(200);
        metrics.record_success(300);

        assert_eq!(metrics.average_latency_ms(), 200.0);
    }

    #[test]
    fn test_average_latency_no_requests() {
        let metrics = PerformanceMetrics::new("gemini".to_string());
        assert_eq!(metrics.average_latency_ms(), 0.0);
    }

    #[test]
    fn test_latency_percentile_ringbuffer_wrap() {
        let mut metrics = PerformanceMetrics::new("ollama".to_string());
        for i in 0..110 {
            metrics.record_success(100 + i as u32);
        }

        assert!(metrics.latencies.len() <= 100);
    }
}
