use super::{
    adapter::*, config::AdapterConfig, error::AdapterResult, GeminiAdapter, OllamaAdapter,
};

/// Adapter factory for creating adapters by type
pub struct AdapterFactory;

impl AdapterFactory {
    /// Create adapter instance from configuration
    pub async fn create(config: &AdapterConfig) -> AdapterResult<SharedAdapter> {
        match config.adapter_type.as_str() {
            "gemini" => {
                let adapter = GeminiAdapter::new(config.clone());
                Ok(std::sync::Arc::new(tokio::sync::Mutex::new(adapter)))
            },
            "ollama" => {
                let adapter = OllamaAdapter::new(config.clone());
                Ok(std::sync::Arc::new(tokio::sync::Mutex::new(adapter)))
            },
            _ => Err(super::error::AdapterError::AdapterNotFound(format!(
                "unknown adapter type: {}",
                config.adapter_type
            ))),
        }
    }

    /// Get list of supported adapter types
    pub fn supported_types() -> Vec<&'static str> {
        vec!["gemini", "ollama"]
    }
}
