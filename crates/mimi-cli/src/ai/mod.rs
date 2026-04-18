pub mod adapter;
pub mod config;
pub mod error;

pub use adapter::{AdapterCapabilities, AdapterInitParams, AiAdapter, AiRequest, AiResponse};
pub use config::AdapterConfig;
pub use error::{AdapterError, AdapterResult};
