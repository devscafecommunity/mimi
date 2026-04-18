use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::ollama_client::{OllamaClient, OllamaRequest};
use super::{
    AdapterCapabilities, AdapterConfig, AdapterError, AdapterInitParams, AdapterResult, AiAdapter,
    AiRequest, AiResponse,
};

pub struct OllamaAdapter {
    client: Arc<Mutex<Option<OllamaClient>>>,
    config: AdapterConfig,
    initialized: Arc<Mutex<bool>>,
}

impl OllamaAdapter {
    pub fn new(config: AdapterConfig) -> Self {
        OllamaAdapter {
            client: Arc::new(Mutex::new(None)),
            config,
            initialized: Arc::new(Mutex::new(false)),
        }
    }

    async fn get_or_create_client(&self) -> AdapterResult<OllamaClient> {
        let mut client_lock = self.client.lock().await;

        if let Some(client) = client_lock.as_ref() {
            return Ok(client.clone());
        }

        let endpoint = self
            .config
            .endpoint
            .as_ref()
            .ok_or_else(|| AdapterError::ConfigError("Endpoint not configured".to_string()))?
            .clone();

        let client = if let Some(api_key) = &self.config.api_key {
            // Cloud mode: use provided API key
            OllamaClient::new_cloud(api_key.clone(), endpoint, self.config.timeout_ms)
                .map_err(|e| AdapterError::InitializationError(e.to_string()))?
        } else {
            // Local mode: no API key needed
            OllamaClient::new_local(endpoint, self.config.timeout_ms)
                .map_err(|e| AdapterError::InitializationError(e.to_string()))?
        };

        *client_lock = Some(client.clone());
        Ok(client)
    }
}

#[async_trait]
impl AiAdapter for OllamaAdapter {
    async fn initialize(&self, params: AdapterInitParams) -> AdapterResult<()> {
        let endpoint = params
            .endpoint
            .clone()
            .or_else(|| self.config.endpoint.clone())
            .ok_or_else(|| AdapterError::ConfigError("Endpoint must be configured".to_string()))?;

        let client = if !params.api_key.is_empty() {
            OllamaClient::new_cloud(params.api_key.clone(), endpoint, params.timeout_ms)
                .map_err(|e| AdapterError::InitializationError(e.to_string()))?
        } else {
            OllamaClient::new_local(endpoint, params.timeout_ms)
                .map_err(|e| AdapterError::InitializationError(e.to_string()))?
        };

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

        let ollama_request = OllamaRequest {
            prompt: request.prompt,
            model: model.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            system_context: request.system_context,
        };

        let response = client
            .invoke(ollama_request)
            .await
            .map_err(|e| AdapterError::ApiError(format!("Ollama API error: {}", e)))?;

        Ok(AiResponse {
            content: response.text,
            model: response.model,
            tokens_used: response.tokens_used,
            cached: false,
        })
    }

    async fn capabilities(&self) -> AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supported_models: vec![
                "llama2".to_string(),
                "codellama".to_string(),
                "mistral".to_string(),
                "neural-chat".to_string(),
            ],
            supports_streaming: false,
            supports_caching: false,
            max_context_tokens: 4096,
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
        "ollama".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config_local() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "ollama".to_string(),
            api_key: None,
            model: "llama2".to_string(),
            timeout_ms: 5000,
            endpoint: Some("http://localhost:11434".to_string()),
            max_retries: 3,
        }
    }

    fn create_test_config_cloud() -> AdapterConfig {
        AdapterConfig {
            adapter_type: "ollama".to_string(),
            api_key: Some("test-api-key".to_string()),
            model: "llama2".to_string(),
            timeout_ms: 5000,
            endpoint: Some("https://ollama.com".to_string()),
            max_retries: 3,
        }
    }

    #[test]
    fn test_ollama_adapter_creation_local() {
        let config = create_test_config_local();
        let adapter = OllamaAdapter::new(config);
        assert_eq!(adapter.config.adapter_type, "ollama");
        assert_eq!(adapter.config.model, "llama2");
    }

    #[test]
    fn test_ollama_adapter_creation_cloud() {
        let config = create_test_config_cloud();
        let adapter = OllamaAdapter::new(config);
        assert_eq!(adapter.config.adapter_type, "ollama");
        assert_eq!(adapter.config.api_key, Some("test-api-key".to_string()));
    }

    #[tokio::test]
    async fn test_capabilities() {
        let config = create_test_config_local();
        let adapter = OllamaAdapter::new(config);
        let caps = adapter.capabilities().await.unwrap();

        assert!(caps.supported_models.contains(&"llama2".to_string()));
        assert!(caps.supported_models.contains(&"codellama".to_string()));
        assert!(!caps.supports_streaming);
        assert!(!caps.supports_caching);
        assert_eq!(caps.max_context_tokens, 4096);
    }

    #[tokio::test]
    async fn test_adapter_name() {
        let config = create_test_config_local();
        let adapter = OllamaAdapter::new(config);
        assert_eq!(adapter.adapter_name(), "ollama");
    }
}
