//! AI invocation with circuit breaker + alternate adapter (M3.3.3).

use mimi_core::{CircuitBreaker, CircuitBreakerSnapshot};

use super::{
    adapter::AiRequest, config::AdapterConfig, error::AdapterResult,
    runtime::{invoke_prompt_with_context, resolve_adapter_config},
    AdapterFactory, AiResponse,
};
use crate::config::defaults::Config;
use tracing::warn;

pub async fn invoke_ai_with_fallback(
    breaker: &CircuitBreaker,
    file_config: &Config,
    primary: &AdapterConfig,
    prompt: &str,
    system_context: Option<String>,
) -> AdapterResult<AiResponse> {
    if !breaker.allow_request() {
        breaker.record_fallback();
        warn!(target: "circuit.ai", "circuit open — alternate adapter");
        return invoke_alternate_adapter(file_config, primary, prompt, system_context)
            .await
            .or_else(|_| Ok(open_circuit_response(primary)));
    }

    match invoke_prompt_with_context(primary, prompt, system_context.clone()).await {
        Ok(response) => {
            breaker.record_success();
            Ok(response)
        },
        Err(err) => {
            breaker.record_failure();
            warn!(
                target: "circuit.ai",
                error = %err,
                "primary adapter failed — trying alternate"
            );
            match invoke_alternate_adapter(file_config, primary, prompt, system_context).await {
                Ok(r) => Ok(r),
                Err(_) => Ok(open_circuit_response(primary)),
            }
        },
    }
}

async fn invoke_alternate_adapter(
    file_config: &Config,
    primary: &AdapterConfig,
    prompt: &str,
    system_context: Option<String>,
) -> AdapterResult<AiResponse> {
    let registry = AdapterFactory::create_with_registry(primary).await?;
    let adapters = registry.adapters.read().await;

    for name in &registry.priority {
        if name == &primary.adapter_type {
            continue;
        }
        if let Some((adapter, health)) = adapters.get(name) {
            let h = health.read().await;
            if !h.is_healthy {
                continue;
            }
            let guard = adapter.lock().await;
            return guard
                .invoke(AiRequest {
                    prompt: prompt.to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                    system_context: system_context.clone(),
                })
                .await;
        }
    }

    for (name, (adapter, health)) in adapters.iter() {
        if name == &primary.adapter_type {
            continue;
        }
        let h = health.read().await;
        if h.is_healthy {
            let guard = adapter.lock().await;
            return guard
                .invoke(AiRequest {
                    prompt: prompt.to_string(),
                    model: None,
                    temperature: None,
                    max_tokens: None,
                    system_context: system_context.clone(),
                })
                .await;
        }
    }

    let mut alt = resolve_adapter_config(file_config)?;
    if alt.adapter_type == primary.adapter_type {
        alt.adapter_type = if primary.adapter_type == "gemini" {
            "ollama".to_string()
        } else {
            "gemini".to_string()
        };
    }
    alt.validate()?;
    invoke_prompt_with_context(&alt, prompt, system_context).await
}

fn open_circuit_response(primary: &AdapterConfig) -> AiResponse {
    AiResponse {
        content: "AI services temporarily unavailable (circuit open). Context may be L1-only."
            .into(),
        model: primary.model.clone(),
        tokens_used: 0,
        cached: false,
    }
}

pub fn ai_breaker_snapshot(breaker: &CircuitBreaker) -> CircuitBreakerSnapshot {
    breaker.snapshot()
}
