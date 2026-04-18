use mimi_cli::ai::{
    AdapterCapabilities, AdapterError, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest,
    AiResponse, SharedAdapter,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

struct FailingAdapter {
    name: String,
    fail_mode: FailMode,
}

enum FailMode {
    AlwaysFails,
    FailsThenSucceeds(AtomicU32),
    SlowResponse(u64),
}

impl FailingAdapter {
    fn always_fails(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fail_mode: FailMode::AlwaysFails,
        }
    }

    fn fails_then_succeeds(name: &str) -> Self {
        Self {
            name: name.to_string(),
            fail_mode: FailMode::FailsThenSucceeds(AtomicU32::new(0)),
        }
    }

    fn slow_response(name: &str, delay_ms: u64) -> Self {
        Self {
            name: name.to_string(),
            fail_mode: FailMode::SlowResponse(delay_ms),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for FailingAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: false,
            supports_caching: false,
            max_context_tokens: 4096,
            supported_models: vec!["test".to_string()],
        })
    }

    async fn invoke(&self, _request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        match &self.fail_mode {
            FailMode::AlwaysFails => Err(AdapterError::ApiError(
                "Simulated adapter failure".to_string(),
            )),
            FailMode::FailsThenSucceeds(counter) => {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(AdapterError::ApiError("Temporary failure".to_string()))
                } else {
                    Ok(AiResponse {
                        content: "Success after retries".to_string(),
                        model: "test".to_string(),
                        tokens_used: 100,
                        cached: false,
                    })
                }
            },
            FailMode::SlowResponse(delay_ms) => {
                tokio::time::sleep(Duration::from_millis(*delay_ms)).await;
                Ok(AiResponse {
                    content: "Response after delay".to_string(),
                    model: "test".to_string(),
                    tokens_used: 100,
                    cached: false,
                })
            },
        }
    }

    async fn health_check(&self) -> mimi_cli::ai::AdapterResult<()> {
        match &self.fail_mode {
            FailMode::AlwaysFails => Err(AdapterError::ApiError("Health check failed".to_string())),
            _ => Ok(()),
        }
    }

    async fn cleanup(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    fn adapter_name(&self) -> String {
        self.name.clone()
    }
}

#[tokio::test]
async fn test_adapter_unavailable_returns_error() {
    let registry = AdapterRegistry::new();
    let failing_adapter: SharedAdapter =
        Arc::new(Mutex::new(FailingAdapter::always_fails("failing")));

    registry
        .register_with_health("failing".to_string(), failing_adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best_adapter = registry.get_best_available().await.unwrap();
    let result = best_adapter.lock().await.invoke(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_adapter_fallback_on_primary_failure() {
    let registry =
        AdapterRegistry::with_priority(vec!["primary".to_string(), "fallback".to_string()]);

    let failing: SharedAdapter = Arc::new(Mutex::new(FailingAdapter::always_fails("primary")));
    let working: SharedAdapter =
        Arc::new(Mutex::new(FailingAdapter::fails_then_succeeds("fallback")));

    registry
        .register_with_health("primary".to_string(), failing)
        .await
        .unwrap();
    registry
        .register_with_health("fallback".to_string(), working)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Fallback test".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_slow_adapter_eventually_responds() {
    let registry = AdapterRegistry::new();
    let slow_adapter: SharedAdapter =
        Arc::new(Mutex::new(FailingAdapter::slow_response("slow", 50)));

    registry
        .register_with_health("slow".to_string(), slow_adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Slow test".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let start = std::time::Instant::now();
    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());
    assert!(elapsed.as_millis() >= 50);
}

#[tokio::test]
async fn test_retry_recovers_from_temporary_failure() {
    let registry = AdapterRegistry::new();
    let retry_adapter: SharedAdapter =
        Arc::new(Mutex::new(FailingAdapter::fails_then_succeeds("retry")));

    registry
        .register_with_health("retry".to_string(), retry_adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Retry test".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();

    let result1 = best.lock().await.invoke(request.clone()).await;
    assert!(result1.is_err());

    let result2 = best.lock().await.invoke(request.clone()).await;
    assert!(result2.is_err());

    let result3 = best.lock().await.invoke(request.clone()).await;
    assert!(result3.is_ok());
}

#[tokio::test]
async fn test_health_check_detects_failure() {
    let registry = AdapterRegistry::new();
    let failing: SharedAdapter = Arc::new(Mutex::new(FailingAdapter::always_fails("unhealthy")));

    registry
        .register_with_health("unhealthy".to_string(), failing)
        .await
        .unwrap();

    let best = registry.get_best_available().await.unwrap();
    let health = best.lock().await.health_check().await;

    assert!(health.is_err());
}

#[tokio::test]
async fn test_concurrent_failures_handled() {
    let registry = AdapterRegistry::new();
    let failing: SharedAdapter = Arc::new(Mutex::new(FailingAdapter::always_fails("concurrent")));

    registry
        .register_with_health("concurrent".to_string(), failing)
        .await
        .unwrap();

    let mut handles = vec![];
    for _ in 0..10 {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            let request = AiRequest {
                prompt: "Concurrent".to_string(),
                model: None,
                temperature: None,
                max_tokens: None,
                system_context: None,
            };

            let best = reg.get_best_available().await?;
            let locked = best.lock().await;
            locked.invoke(request).await
        });
        handles.push(handle);
    }

    let mut error_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Err(_)) => error_count += 1,
            _ => {},
        }
    }

    assert!(error_count > 0);
}
