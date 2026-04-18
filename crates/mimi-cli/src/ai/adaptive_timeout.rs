use chrono::Utc;

pub struct AdaptiveTimeout {
    pub adapter_name: String,
    pub baseline_ms: u32,
    pub current_timeout_ms: u32,
    pub p95_margin_percent: f32,
    pub last_adjusted: chrono::DateTime<chrono::Utc>,
    pub adjustment_interval_secs: u64,
}

impl AdaptiveTimeout {
    pub fn new(adapter_name: String, baseline_ms: u32) -> Self {
        Self {
            adapter_name,
            baseline_ms,
            current_timeout_ms: baseline_ms,
            p95_margin_percent: 120.0,
            last_adjusted: Utc::now(),
            adjustment_interval_secs: 5,
        }
    }

    pub fn get_timeout_ms(&self) -> u32 {
        self.current_timeout_ms
    }

    pub fn can_adjust(&self) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.last_adjusted)
            .num_seconds();
        elapsed >= self.adjustment_interval_secs as i64
    }

    pub fn adjust_based_on_metrics(
        &mut self,
        metrics: &super::performance::PerformanceMetrics,
    ) -> bool {
        if !self.can_adjust() {
            return false;
        }

        if let Some(p95_val) = metrics.percentile(95.0) {
            let new_timeout = ((p95_val as f32 * self.p95_margin_percent) / 100.0) as u32;
            let new_timeout = new_timeout.max(self.baseline_ms);

            if new_timeout != self.current_timeout_ms {
                self.current_timeout_ms = new_timeout;
                self.last_adjusted = Utc::now();
                return true;
            }
        }

        false
    }

    pub fn reset_to_baseline(&mut self) {
        self.current_timeout_ms = self.baseline_ms;
        self.last_adjusted = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_timeout_new() {
        let timeout = AdaptiveTimeout::new("gemini".to_string(), 30000);
        assert_eq!(timeout.adapter_name, "gemini");
        assert_eq!(timeout.baseline_ms, 30000);
        assert_eq!(timeout.current_timeout_ms, 30000);
        assert_eq!(timeout.p95_margin_percent, 120.0);
    }

    #[test]
    fn test_get_timeout_ms() {
        let timeout = AdaptiveTimeout::new("ollama".to_string(), 15000);
        assert_eq!(timeout.get_timeout_ms(), 15000);
    }

    #[test]
    fn test_can_adjust_initially_true() {
        let mut timeout = AdaptiveTimeout::new("gemini".to_string(), 30000);
        timeout.last_adjusted = Utc::now() - chrono::Duration::seconds(10);
        assert!(timeout.can_adjust());
    }

    #[test]
    fn test_can_adjust_after_interval() {
        let mut timeout = AdaptiveTimeout::new("ollama".to_string(), 30000);
        timeout.last_adjusted = Utc::now();
        timeout.adjustment_interval_secs = 0;

        assert!(timeout.can_adjust());
    }

    #[test]
    fn test_adjust_based_on_metrics_no_adjustment_needed() {
        let mut timeout = AdaptiveTimeout::new("gemini".to_string(), 30000);
        let mut metrics = super::super::performance::PerformanceMetrics::new("gemini".to_string());
        metrics.record_success(100);
        metrics.record_success(150);

        let adjusted = timeout.adjust_based_on_metrics(&metrics);

        assert!(!adjusted);
        assert_eq!(timeout.current_timeout_ms, 30000);
    }

    #[test]
    fn test_adjust_based_on_metrics_increases_timeout() {
        let mut timeout = AdaptiveTimeout::new("ollama".to_string(), 100);
        let mut metrics = super::super::performance::PerformanceMetrics::new("ollama".to_string());

        for i in 0..100 {
            metrics.record_success(50 + i as u32);
        }

        if let Some(p95) = metrics.percentile(95.0) {
            let old_timeout = timeout.current_timeout_ms;
            timeout.adjust_based_on_metrics(&metrics);

            assert!(timeout.current_timeout_ms >= old_timeout);
        }
    }

    #[test]
    fn test_reset_to_baseline() {
        let mut timeout = AdaptiveTimeout::new("gemini".to_string(), 30000);
        timeout.current_timeout_ms = 50000;

        timeout.reset_to_baseline();

        assert_eq!(timeout.current_timeout_ms, 30000);
    }

    #[test]
    fn test_respects_min_adjustment_interval() {
        let mut timeout = AdaptiveTimeout::new("ollama".to_string(), 100);
        timeout.adjustment_interval_secs = 100;

        let mut metrics = super::super::performance::PerformanceMetrics::new("ollama".to_string());
        for i in 0..100 {
            metrics.record_success(50 + i as u32);
        }

        let old_timeout = timeout.current_timeout_ms;
        let adjusted = timeout.adjust_based_on_metrics(&metrics);

        assert!(!adjusted);
        assert_eq!(timeout.current_timeout_ms, old_timeout);
    }
}
