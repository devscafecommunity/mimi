use std::time::{SystemTime, UNIX_EPOCH};

/// Tracks health status of an adapter
#[derive(Clone)]
pub struct AdapterHealth {
    pub adapter_name: String,
    pub is_healthy: bool,
    pub consecutive_failures: u32,
    pub success_count: u32,
    pub error_count: u32,
    pub last_latency_ms: u32,
    last_check_timestamp: u64, // seconds since epoch
}

impl AdapterHealth {
    /// Create new health tracker
    pub fn new(adapter_name: &str) -> Self {
        Self {
            adapter_name: adapter_name.to_string(),
            is_healthy: true,
            consecutive_failures: 0,
            success_count: 0,
            error_count: 0,
            last_latency_ms: 0,
            last_check_timestamp: current_timestamp(),
        }
    }

    /// Record successful health check
    pub fn record_success(&mut self, latency_ms: u32) {
        self.is_healthy = true;
        self.consecutive_failures = 0;
        self.success_count += 1;
        self.last_latency_ms = latency_ms;
        self.last_check_timestamp = current_timestamp();
    }

    /// Record failed health check
    pub fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        self.error_count += 1;
        self.last_check_timestamp = current_timestamp();

        // Mark unhealthy after 3 consecutive failures
        if self.consecutive_failures >= 3 {
            self.is_healthy = false;
        }
    }

    /// Calculate success rate (0.0 to 1.0)
    pub fn success_rate(&self) -> f64 {
        let total = self.success_count as f64 + self.error_count as f64;
        if total == 0.0 {
            0.0
        } else {
            self.success_count as f64 / total
        }
    }

    /// Calculate average latency in milliseconds
    pub fn average_latency_ms(&self) -> f64 {
        if self.success_count == 0 {
            0.0
        } else {
            self.last_latency_ms as f64 // Simplified: use last latency
        }
    }

    /// Check if adapter was checked recently (within last N seconds)
    pub fn is_recently_checked(&self, within_seconds: u64) -> bool {
        let now = current_timestamp();
        now.saturating_sub(self.last_check_timestamp) <= within_seconds
    }
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_new_is_healthy() {
        let health = AdapterHealth::new("test");
        assert_eq!(health.adapter_name, "test");
        assert!(health.is_healthy);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.success_count, 0);
        assert_eq!(health.error_count, 0);
    }

    #[test]
    fn test_health_record_success() {
        let mut health = AdapterHealth::new("test");
        health.record_success(42);
        assert!(health.is_healthy);
        assert_eq!(health.success_count, 1);
        assert_eq!(health.error_count, 0);
        assert_eq!(health.consecutive_failures, 0);
        assert_eq!(health.last_latency_ms, 42);
    }

    #[test]
    fn test_health_record_failure() {
        let mut health = AdapterHealth::new("test");
        health.record_failure();
        assert_eq!(health.consecutive_failures, 1);
        assert_eq!(health.error_count, 1);
        assert!(health.is_healthy); // Still healthy at 1 failure
    }

    #[test]
    fn test_health_failure_threshold() {
        let mut health = AdapterHealth::new("test");
        health.record_failure();
        health.record_failure();
        assert!(health.is_healthy); // Still healthy at 2 failures

        health.record_failure();
        assert!(!health.is_healthy); // Unhealthy at 3 failures
        assert_eq!(health.consecutive_failures, 3);
    }

    #[test]
    fn test_health_reset_on_success() {
        let mut health = AdapterHealth::new("test");
        health.record_failure();
        health.record_failure();
        health.record_failure();
        assert!(!health.is_healthy);

        health.record_success(50);
        assert_eq!(health.consecutive_failures, 0);
        assert!(health.is_healthy);
        assert_eq!(health.success_count, 1);
    }

    #[test]
    fn test_health_success_rate() {
        let mut health = AdapterHealth::new("test");
        health.record_success(10);
        health.record_success(20);
        health.record_failure();

        let rate = health.success_rate();
        assert!((rate - (2.0 / 3.0)).abs() < 0.01);
    }

    #[test]
    fn test_health_average_latency() {
        let mut health = AdapterHealth::new("test");
        health.record_success(100);
        let avg = health.average_latency_ms();
        assert_eq!(avg, 100.0);
    }

    #[test]
    fn test_health_is_recently_checked() {
        let mut health = AdapterHealth::new("test");
        health.record_success(10);
        assert!(health.is_recently_checked(10)); // Checked within 10 seconds
    }

    #[test]
    fn test_health_clone() {
        let health = AdapterHealth::new("test");
        let health_clone = health.clone();
        assert_eq!(health.adapter_name, health_clone.adapter_name);
        assert_eq!(health.is_healthy, health_clone.is_healthy);
    }

    #[test]
    fn test_health_empty_stats() {
        let health = AdapterHealth::new("test");
        assert_eq!(health.success_rate(), 0.0);
        assert_eq!(health.average_latency_ms(), 0.0);
    }
}
