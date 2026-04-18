use mimi_cli::ai::{
    AdapterCapabilities, AdapterError, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest,
    AiResponse, SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct ValidationAdapter {
    name: String,
}

impl ValidationAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for ValidationAdapter {
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

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        if request.prompt.is_empty() {
            return Err(AdapterError::InvalidRequest(
                "Prompt cannot be empty".to_string(),
            ));
        }

        if request.prompt.len() > 100_000 {
            return Err(AdapterError::InvalidRequest(
                "Prompt exceeds maximum length".to_string(),
            ));
        }

        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 1.0 {
                return Err(AdapterError::InvalidRequest(
                    "Temperature must be between 0.0 and 1.0".to_string(),
                ));
            }
        }

        if let Some(tokens) = request.max_tokens {
            if tokens > 4096 {
                return Err(AdapterError::InvalidRequest(
                    "Max tokens exceeds context window".to_string(),
                ));
            }
        }

        Ok(AiResponse {
            content: format!("Response: {}", request.prompt),
            model: "test".to_string(),
            tokens_used: 100,
            cached: false,
        })
    }

    async fn health_check(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    fn adapter_name(&self) -> String {
        self.name.clone()
    }
}

#[tokio::test]
async fn test_empty_prompt_rejected() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "".to_string(),
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[tokio::test]
async fn test_excessive_prompt_length_rejected() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let long_prompt = "a".repeat(150_000);
    let request = AiRequest {
        prompt: long_prompt,
        model: None,
        temperature: None,
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("maximum length"));
}

#[tokio::test]
async fn test_invalid_temperature_too_high() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: Some(1.5),
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Temperature"));
}

#[tokio::test]
async fn test_invalid_temperature_negative() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: Some(-0.5),
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_max_tokens_exceeds_context_window() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: None,
        max_tokens: Some(10_000),
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("context window"));
}

#[tokio::test]
async fn test_valid_boundary_values() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Valid request".to_string(),
        model: None,
        temperature: Some(0.0),
        max_tokens: Some(4096),
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_temperature_boundary_1_0() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: Some(1.0),
        max_tokens: None,
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_max_tokens_boundary() {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(ValidationAdapter::new("validator")));

    registry
        .register_with_health("validator".to_string(), adapter)
        .await
        .unwrap();

    let request = AiRequest {
        prompt: "Test".to_string(),
        model: None,
        temperature: None,
        max_tokens: Some(4096),
        system_context: None,
    };

    let best = registry.get_best_available().await.unwrap();
    let result = best.lock().await.invoke(request).await;

    assert!(result.is_ok());
}
