use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::gemini_client::{GeminiClient, GeminiRequest};
use super::{
    AdapterCapabilities, AdapterConfig, AdapterError, AdapterInitParams, AdapterResult, AiAdapter,
    AiRequest, AiResponse,
};

pub struct GeminiAdapter {
    client: Arc<Mutex<Option<GeminiClient>>>,
    config: AdapterConfig,
    initialized: Arc<Mutex<bool>>,
}

impl GeminiAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        GeminiAdapter {
            client: Arc::new(Mutex::new(None)),
            config,
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    async fn get_or_create_client(&self) -> AdapterResult<GeminiClient> {
        let mut client_lock = self.client.lock().await;

        if let Some(client) = client_lock.as_ref() {
            return Ok(client.clone());
        }

        let api_key = self
            .config
            .api_key
            .as_ref()
            .ok_or_else(|| AdapterError::ConfigError("API key not configured".to_string()))?
            .clone();

        let client = GeminiClient::new(api_key, self.config.timeout_ms)
            .map_err(|e| AdapterError::InitializationError(e.to_string()))?;

        *client_lock = Some(client.clone());
        Ok(client)
    }
}

#[async_trait]
impl AiAdapter for GeminiAdapter {
    async fn initialize(&self, params: AdapterInitParams) -> AdapterResult<()> {
        if params.api_key.is_empty() {
            return Err(AdapterError::ConfigError(
                "API key cannot be empty".to_string(),
            ));
        }

        let client = GeminiClient::new(params.api_key.clone(), params.timeout_ms)
            .map_err(|e| AdapterError::InitializationError(e.to_string()))?;

        client
            .health_check()
            .await
            .map_err(|e| AdapterError::InitializationError(e.to_string()))?;

        let mut client_lock = self.client.lock().await;
        *client_lock = Some(client);

        let mut initialized = self.initialized.lock().await;
        *initialized = true;

        Ok(())
    }

    async fn invoke(&self, request: AiRequest) -> AdapterResult<AiResponse> {
        let client = self.get_or_create_client().await?;

        let model = request
            .model
            .clone()
            .unwrap_or_else(|| self.config.model.clone());

        let gemini_request = GeminiRequest {
            prompt: request.prompt,
            model: model.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            system_context: request.system_context,
        };

        let response = client
            .invoke(gemini_request)
            .await
            .map_err(|e| AdapterError::ApiError(format!("Gemini API error: {}", e)))?;

        Ok(AiResponse {
            content: response.text,
            model: response.model,
            tokens_used: response.tokens_used,
            cached: false,
        })
    }

    async fn capabilities(&self) -> AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supported_models: vec!["gemini-pro".to_string(), "gemini-1.5-pro".to_string()],
            supports_streaming: false,
            supports_caching: true,
            max_context_tokens: 32000,
        })
    }

    async fn health_check(&self) -> AdapterResult<()> {
        let client = self.get_or_create_client().await?;
        client
            .health_check()
            .await
            .map_err(|e| AdapterError::ApiError(format!("Health check failed: {}", e)))
    }

    async fn cleanup(&self) -> AdapterResult<()> {
        let mut client_lock = self.client.lock().await;
        *client_lock = None;

        let mut initialized = self.initialized.lock().await;
        *initialized = false;

        Ok(())
    }

    fn adapter_name(&self) -> String {
        "gemini".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "gemini".to_string(),
            api_key: Some("test-key-123".to_string()),
            model: "gemini-pro".to_string(),
            timeout_ms: 5000,
            endpoint: None,
            max_retries: 3,
        }
    }

    #[test]
    fn test_gemini_adapter_creation() {
        let config = create_test_config();
        let adapter = GeminiAdapter::new(config);
        assert_eq!(adapter.config.adapter_type, "gemini");
    }

    #[tokio::test]
    async fn test_capabilities() {
        let config = create_test_config();
        let adapter = GeminiAdapter::new(config);
        let caps = adapter.capabilities().await.unwrap();

        assert!(caps.supported_models.contains(&"gemini-pro".to_string()));
        assert!(!caps.supports_streaming);
        assert!(caps.supports_caching);
        assert_eq!(caps.max_context_tokens, 32000);
    }
}
