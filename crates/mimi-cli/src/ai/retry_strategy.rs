use std::time::Duration;

use super::AdapterError;

pub struct RetryStrategy {
    max_retries: u32,
    initial_backoff_ms: u64,
    max_backoff_ms: u64,
    backoff_multiplier: f64,
}

impl RetryStrategy {
    pub fn new() -> Self {
        RetryStrategy {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10000,
            backoff_multiplier: 2.0,
        }
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_initial_backoff_ms(mut self, ms: u64) -> Self {
        self.initial_backoff_ms = ms;
        self
    }

    pub fn with_max_backoff_ms(mut self, ms: u64) -> Self {
        self.max_backoff_ms = ms;
        self
    }

    pub fn backoff_duration(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        let backoff_ms = (self.initial_backoff_ms as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1)) as u64;
        let capped_backoff = backoff_ms.min(self.max_backoff_ms);

        Duration::from_millis(capped_backoff)
    }

    pub fn is_retryable(&self, error: &AdapterError) -> bool {
        error.is_retryable()
    }

    pub async fn execute<F, Fut, T>(&self, mut f: F) -> Result<T, AdapterError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, AdapterError>>,
    {
        let mut attempt = 0;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if !self.is_retryable(&error) || attempt >= self.max_retries {
                        return Err(error);
                    }

                    attempt += 1;
                    let backoff = self.backoff_duration(attempt);
                    tokio::time::sleep(backoff).await;
                },
            }
        }
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_strategy_creation() {
        let strategy = RetryStrategy::new();
        assert_eq!(strategy.max_retries, 3);
        assert_eq!(strategy.initial_backoff_ms, 100);
    }

    #[test]
    fn test_backoff_calculation() {
        let strategy = RetryStrategy::new();

        assert_eq!(strategy.backoff_duration(0), Duration::from_millis(0));
        assert_eq!(strategy.backoff_duration(1), Duration::from_millis(100));
        assert_eq!(strategy.backoff_duration(2), Duration::from_millis(200));
        assert_eq!(strategy.backoff_duration(3), Duration::from_millis(400));
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let strategy = RetryStrategy::new()
            .with_initial_backoff_ms(1000)
            .with_max_backoff_ms(5000);

        assert!(strategy.backoff_duration(3) <= Duration::from_millis(5000));
    }

    #[test]
    fn test_retry_strategy_builder() {
        let strategy = RetryStrategy::new()
            .with_max_retries(5)
            .with_initial_backoff_ms(50)
            .with_max_backoff_ms(20000);

        assert_eq!(strategy.max_retries, 5);
        assert_eq!(strategy.initial_backoff_ms, 50);
        assert_eq!(strategy.max_backoff_ms, 20000);
    }

    #[tokio::test]
    async fn test_retry_execute_success_on_first_try() {
        let strategy = RetryStrategy::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = call_count.clone();

        let result = strategy
            .execute(|| {
                let cc = count_clone.clone();
                async move {
                    cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok::<_, AdapterError>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_execute_success_on_retry() {
        let strategy = RetryStrategy::new();
        let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count_clone = call_count.clone();

        let result = strategy
            .execute(|| {
                let cc = count_clone.clone();
                async move {
                    let count = cc.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    if count < 2 {
                        Err(AdapterError::Timeout(100))
                    } else {
                        Ok::<_, AdapterError>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
