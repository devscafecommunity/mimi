use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub adapter: AdapterConfig,
    pub bus: BusConfig,
    pub log: LogConfig,
    pub memory: MemoryConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub default: String,
    pub gemini: GeminiConfig,
    pub ollama: OllamaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    pub api_key: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusConfig {
    pub url: String,
    pub timeout_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_size: String,
    pub cleanup_interval: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_audit: bool,
    pub timeout_per_task: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            adapter: AdapterConfig {
                default: "gemini".to_string(),
                gemini: GeminiConfig {
                    api_key: None,
                    model: "gemini-pro".to_string(),
                },
                ollama: OllamaConfig {
                    url: "http://localhost:11434".to_string(),
                },
            },
            bus: BusConfig {
                url: "tcp://127.0.0.1:7447".to_string(),
                timeout_ms: 5000,
            },
            log: LogConfig {
                level: "info".to_string(),
                format: "text".to_string(),
            },
            memory: MemoryConfig {
                max_size: "1gb".to_string(),
                cleanup_interval: 3600,
            },
            security: SecurityConfig {
                enable_audit: true,
                timeout_per_task: 300,
            },
        }
    }
}

/// Get development profile config
pub fn development_profile() -> Config {
    let mut cfg = Config::default();
    cfg.log.level = "debug".to_string();
    cfg.security.enable_audit = false;
    cfg
}

/// Get production profile config
pub fn production_profile() -> Config {
    let mut cfg = Config::default();
    cfg.log.level = "warn".to_string();
    cfg.security.enable_audit = true;
    cfg
}
