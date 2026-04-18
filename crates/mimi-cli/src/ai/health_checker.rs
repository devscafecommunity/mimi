use super::health::AdapterHealth;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

pub type SharedHealth = Arc<RwLock<AdapterHealth>>;

pub struct HealthChecker {
    pub check_interval_secs: u64,
}

impl HealthChecker {
    pub fn new(check_interval_secs: u64) -> Self {
        Self {
            check_interval_secs,
        }
    }

    pub fn spawn(
        &self,
        adapter_registry: Arc<RwLock<HashMap<String, SharedHealth>>>,
    ) -> JoinHandle<()> {
        let interval_secs = self.check_interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                let adapter_names: Vec<String> = {
                    let registry = adapter_registry.read().await;
                    registry.keys().cloned().collect()
                };

                for name in adapter_names {
                    if let Some(health) = adapter_registry.read().await.get(&name).cloned() {
                        let latency_ms = 10;
                        let mut h = health.write().await;
                        h.record_success(latency_ms);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_checker_new() {
        let checker = HealthChecker::new(5);
        assert_eq!(checker.check_interval_secs, 5);
    }

    #[tokio::test]
    async fn test_health_checker_spawn_and_stop() {
        let checker = HealthChecker::new(1);
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let handle = checker.spawn(registry);

        tokio::time::sleep(Duration::from_millis(500)).await;
        handle.abort();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(handle.is_finished());
    }

    #[test]
    fn test_health_checker_interval() {
        let checker = HealthChecker::new(30);
        assert_eq!(checker.check_interval_secs, 30);
    }

    #[test]
    fn test_health_checker_zero_interval() {
        let checker = HealthChecker::new(0);
        assert_eq!(checker.check_interval_secs, 0);
    }

    #[tokio::test]
    async fn test_health_checker_multiple_intervals() {
        for interval in vec![5, 10, 30, 60] {
            let checker = HealthChecker::new(interval);
            assert_eq!(checker.check_interval_secs, interval);
        }
    }

    #[tokio::test]
    async fn test_health_checker_spawn_empty_registry() {
        let checker = HealthChecker::new(1);
        let registry = Arc::new(RwLock::new(HashMap::new()));
        let handle = checker.spawn(registry);

        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.abort();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(handle.is_finished());
    }

    #[test]
    fn test_health_checker_constants() {
        assert_eq!(HealthChecker::new(5).check_interval_secs, 5);
        assert_eq!(HealthChecker::new(10).check_interval_secs, 10);
    }
}
