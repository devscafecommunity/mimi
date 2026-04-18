use super::error::AdapterResult;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Adapter initialization parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdapterInitParams {
    pub api_key: String,
    pub endpoint: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
}

/// AI model request
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiRequest {
    pub prompt: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_context: Option<String>,
}

/// AI model response
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: u32,
    pub cached: bool,
}

/// Adapter capabilities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdapterCapabilities {
    pub supports_streaming: bool,
    pub supports_caching: bool,
    pub max_context_tokens: u32,
    pub supported_models: Vec<String>,
}

/// Core AI adapter trait
#[async_trait::async_trait]
pub trait AiAdapter: Send + Sync {
    /// Initialize adapter with parameters
    async fn initialize(&self, params: AdapterInitParams) -> AdapterResult<()>;

    /// Get adapter capabilities
    async fn capabilities(&self) -> AdapterResult<AdapterCapabilities>;

    /// Invoke AI model with a request
    async fn invoke(&self, request: AiRequest) -> AdapterResult<AiResponse>;

    /// Health check - verify adapter is functioning
    async fn health_check(&self) -> AdapterResult<()>;

    /// Cleanup - release resources
    async fn cleanup(&self) -> AdapterResult<()>;

    /// Get adapter name/identifier
    fn adapter_name(&self) -> String;
}

/// Shared adapter instance
pub type SharedAdapter = Arc<Mutex<dyn AiAdapter>>;
