pub mod adapter;
pub mod config;
pub mod error;
pub mod factory;
pub mod gemini_adapter;
pub mod gemini_client;
pub mod registry;

pub use adapter::{
    AdapterCapabilities, AdapterInitParams, AiAdapter, AiRequest, AiResponse, SharedAdapter,
};
pub use config::AdapterConfig;
pub use error::{AdapterError, AdapterResult};
pub use factory::AdapterFactory;
pub use registry::AdapterRegistry;
