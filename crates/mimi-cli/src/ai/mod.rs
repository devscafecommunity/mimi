pub mod adapter;
pub mod adaptive_timeout;
pub mod config;
pub mod error;
pub mod factory;
pub mod guarded_invoke;
pub mod gemini_adapter;
pub mod gemini_client;
pub mod health;
pub mod health_checker;
pub mod ollama_adapter;
pub mod ollama_client;
pub mod performance;
pub mod performance_tracker;
pub mod priority_config;
pub mod prompt_templates;
pub mod registry;
pub mod runtime;
pub mod retry_strategy;

pub use adapter::{
    AdapterCapabilities, AdapterInitParams, AiAdapter, AiRequest, AiResponse, SharedAdapter,
};
pub use config::AdapterConfig;
pub use error::{AdapterError, AdapterResult};
pub use factory::AdapterFactory;
pub use gemini_adapter::GeminiAdapter;
pub use health::AdapterHealth;
pub use health_checker::HealthChecker;
pub use ollama_adapter::OllamaAdapter;
pub use performance_tracker::PerformanceTracker;
pub use priority_config::AdapterPriority;
pub use registry::AdapterRegistry;
pub use guarded_invoke::invoke_ai_with_fallback;
pub use runtime::{
    adapter_to_cli, invoke_prompt, invoke_prompt_with_context, resolve_adapter_config,
};
// `invoke_prompt` kept exported for debug adapter test and legacy callers.
