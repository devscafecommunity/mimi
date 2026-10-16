use super::{
    adapter::AiRequest, config::AdapterConfig, error::AdapterError, AdapterFactory, AdapterResult,
};
use crate::config::defaults::Config;

pub fn adapter_to_cli(err: AdapterError) -> crate::cli::error::CliError {
    match err {
        AdapterError::ConfigError(msg) | AdapterError::InitializationError(msg) => {
            crate::cli::error::CliError::ConfigError(msg)
        },
        AdapterError::AdapterNotFound(msg) => crate::cli::error::CliError::NotFound(msg),
        AdapterError::Timeout(ms) => crate::cli::error::CliError::NetworkError(format!(
            "adapter timeout after {}ms",
            ms
        )),
        AdapterError::RateLimited(secs) => crate::cli::error::CliError::NetworkError(format!(
            "rate limited, retry after {}s",
            secs
        )),
        AdapterError::NetworkError(msg) | AdapterError::ApiError(msg) => {
            crate::cli::error::CliError::NetworkError(msg)
        },
        other => crate::cli::error::CliError::ExecutionError(other.to_string()),
    }
}

/// Merge TOML defaults with `OLLAMA_*` / `MIMI_*` environment variables.
pub fn resolve_adapter_config(file: &Config) -> AdapterResult<AdapterConfig> {
    if std::env::var("OLLAMA_API_KEY").is_ok() || std::env::var("MIMI_ADAPTER_TYPE").is_ok() {
        return AdapterConfig::from_env();
    }

    let adapter_type = file.adapter.default.clone();

    let endpoint = std::env::var("MIMI_ADAPTER_ENDPOINT")
        .or_else(|_| std::env::var("OLLAMA_ENDPOINT"))
        .ok()
        .or_else(|| {
            if adapter_type == "ollama" {
                Some(file.adapter.ollama.url.clone())
            } else {
                None
            }
        });

    let api_key = match adapter_type.as_str() {
        "ollama" => std::env::var("OLLAMA_API_KEY")
            .or_else(|_| std::env::var("MIMI_API_KEY"))
            .ok(),
        _ => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("MIMI_API_KEY"))
            .ok(),
    };

    let model = std::env::var("OLLAMA_MODEL")
        .or_else(|_| std::env::var("MIMI_MODEL"))
        .unwrap_or_else(|_| {
            if adapter_type == "ollama" {
                "gemma4:31b".to_string()
            } else {
                file.adapter.gemini.model.clone()
            }
        });

    let config = AdapterConfig {
        adapter_type,
        api_key,
        endpoint,
        timeout_ms: 60_000,
        max_retries: 3,
        model,
    };

    config.validate()?;
    Ok(config)
}

pub async fn invoke_prompt(config: &AdapterConfig, prompt: &str) -> AdapterResult<super::AiResponse> {
    invoke_prompt_with_context(config, prompt, None).await
}

pub async fn invoke_prompt_with_context(
    config: &AdapterConfig,
    prompt: &str,
    system_context: Option<String>,
) -> AdapterResult<super::AiResponse> {
    let adapter = AdapterFactory::create(config).await?;
    let guard = adapter.lock().await;
    guard
        .invoke(AiRequest {
            prompt: prompt.to_string(),
            model: None,
            temperature: None,
            max_tokens: None,
            system_context,
        })
        .await
}
