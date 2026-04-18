pub mod adapter;
pub mod config;
pub mod error;
pub mod factory;
pub mod gemini_adapter;
pub mod gemini_client;
pub mod health;
pub mod health_checker;
pub mod ollama_adapter;
pub mod ollama_client;
pub mod priority_config;
pub mod prompt_templates;
pub mod registry;
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
pub use priority_config::AdapterPriority;
pub use registry::AdapterRegistry;
