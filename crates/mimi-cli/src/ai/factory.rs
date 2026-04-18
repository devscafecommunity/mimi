use super::{adapter::*, config::AdapterConfig, error::AdapterResult};

/// Adapter factory for creating adapters by type
pub struct AdapterFactory;

impl AdapterFactory {
    /// Create adapter instance from configuration
    pub async fn create(config: &AdapterConfig) -> AdapterResult<SharedAdapter> {
        match config.adapter_type.as_str() {
            "gemini" => {
                // Placeholder - will implement in M1.5.2
                Err(super::error::AdapterError::AdapterNotFound(
                    "gemini adapter not yet implemented".to_string(),
                ))
            },
            "ollama" => {
                // Placeholder - will implement in M1.5.3
                Err(super::error::AdapterError::AdapterNotFound(
                    "ollama adapter not yet implemented".to_string(),
                ))
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
