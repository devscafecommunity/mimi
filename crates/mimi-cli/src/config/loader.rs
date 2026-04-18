use super::defaults::Config;
use crate::cli::error::{CliError, CliResult};
use std::path::PathBuf;

pub struct ConfigLoader;

impl ConfigLoader {
    /// Load config from file or use defaults
    pub fn load(config_path: Option<PathBuf>) -> CliResult<Config> {
        if let Some(path) = config_path {
            Self::load_from_file(&path)
        } else {
            Ok(Config::default())
        }
    }

    /// Load from specific file
    pub fn load_from_file(path: &PathBuf) -> CliResult<Config> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            CliError::ConfigError(format!("Cannot read config file {}: {}", path.display(), e))
        })?;

        toml::from_str(&content)
            .map_err(|e| CliError::ConfigError(format!("Invalid TOML in config file: {}", e)))
    }

    /// Save config to file
    pub fn save(config: &Config, path: &PathBuf) -> CliResult<()> {
        let toml_str = toml::to_string_pretty(config).map_err(|e| {
            CliError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Cannot serialize config: {}", e),
            ))
        })?;

        std::fs::write(path, toml_str).map_err(|e| {
            CliError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Cannot write config file: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Get config value by dot-notation key
    pub fn get_value(config: &Config, key: &str) -> CliResult<String> {
        match key {
            "adapter.default" => Ok(config.adapter.default.clone()),
            "adapter.gemini.model" => Ok(config.adapter.gemini.model.clone()),
            "adapter.ollama.url" => Ok(config.adapter.ollama.url.clone()),
            "bus.url" => Ok(config.bus.url.clone()),
            "log.level" => Ok(config.log.level.clone()),
            "memory.max_size" => Ok(config.memory.max_size.clone()),
            _ => Err(CliError::NotFound(format!("Unknown config key: {}", key))),
        }
    }

    /// Set config value by dot-notation key
    pub fn set_value(config: &mut Config, key: &str, value: &str) -> CliResult<()> {
        match key {
            "adapter.default" => config.adapter.default = value.to_string(),
            "adapter.gemini.model" => config.adapter.gemini.model = value.to_string(),
            "adapter.ollama.url" => config.adapter.ollama.url = value.to_string(),
            "bus.url" => config.bus.url = value.to_string(),
            "log.level" => {
                if !["trace", "debug", "info", "warn", "error"].contains(&value) {
                    return Err(CliError::ConfigError(
                        "log.level must be one of: trace, debug, info, warn, error".to_string(),
                    ));
                }
                config.log.level = value.to_string();
            },
            "memory.max_size" => config.memory.max_size = value.to_string(),
            _ => return Err(CliError::NotFound(format!("Unknown config key: {}", key))),
        }
        Ok(())
    }
}
