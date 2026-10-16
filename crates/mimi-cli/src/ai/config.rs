use super::error::AdapterResult;
use serde::{Deserialize, Serialize};

/// Adapter configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub adapter_type: String, // "gemini", "ollama", etc.
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub timeout_ms: u64,
    pub max_retries: u32,
    pub model: String,
}

impl AdapterConfig {
    /// Validate configuration
    pub fn validate(&self) -> AdapterResult<()> {
        if self.adapter_type.is_empty() {
            return Err(super::error::AdapterError::ConfigError(
                "adapter_type cannot be empty".to_string(),
            ));
        }

        if self.model.is_empty() {
            return Err(super::error::AdapterError::ConfigError(
                "model cannot be empty".to_string(),
            ));
        }

        if self.timeout_ms == 0 {
            return Err(super::error::AdapterError::ConfigError(
                "timeout_ms must be > 0".to_string(),
            ));
        }

        match self.adapter_type.as_str() {
            "gemini" => {
                if self.api_key.is_none() {
                    return Err(super::error::AdapterError::ConfigError(
                        "gemini adapter requires api_key".to_string(),
                    ));
                }
            },
            "ollama" => {
                if self.endpoint.is_none() {
                    return Err(super::error::AdapterError::ConfigError(
                        "ollama adapter requires endpoint".to_string(),
                    ));
                }
                if is_ollama_cloud_endpoint(self.endpoint.as_deref()) && self.api_key.is_none() {
                    return Err(super::error::AdapterError::ConfigError(
                        "ollama cloud requires OLLAMA_API_KEY (or MIMI_API_KEY)".to_string(),
                    ));
                }
            },
            _ => {},
        }

        Ok(())
    }

    /// Load from environment variables
    pub fn from_env() -> AdapterResult<Self> {
        let adapter_type =
            std::env::var("MIMI_ADAPTER_TYPE").unwrap_or_else(|_| "ollama".to_string());

        let api_key = match adapter_type.as_str() {
            "ollama" => std::env::var("OLLAMA_API_KEY")
                .or_else(|_| std::env::var("MIMI_API_KEY"))
                .ok(),
            _ => std::env::var("MIMI_API_KEY")
                .or_else(|_| std::env::var("GEMINI_API_KEY"))
                .ok(),
        };

        let endpoint = std::env::var("MIMI_ADAPTER_ENDPOINT")
            .or_else(|_| std::env::var("OLLAMA_ENDPOINT"))
            .ok()
            .or_else(|| {
                if adapter_type == "ollama" {
                    Some("https://ollama.com".to_string())
                } else {
                    None
                }
            });

        let timeout_ms = std::env::var("MIMI_ADAPTER_TIMEOUT")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(30000);

        let max_retries = std::env::var("MIMI_ADAPTER_MAX_RETRIES")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(3);

        let model = std::env::var("OLLAMA_MODEL")
            .or_else(|_| std::env::var("MIMI_MODEL"))
            .unwrap_or_else(|_| {
                if adapter_type == "ollama" {
                    "gemma4:31b".to_string()
                } else {
                    "gemini-pro".to_string()
                }
            });

        let config = AdapterConfig {
            adapter_type,
            api_key,
            endpoint,
            timeout_ms,
            max_retries,
            model,
        };

        config.validate()?;
        Ok(config)
    }
}

fn is_ollama_cloud_endpoint(endpoint: Option<&str>) -> bool {
    endpoint
        .map(|e| e.contains("ollama.com"))
        .unwrap_or(false)
}
