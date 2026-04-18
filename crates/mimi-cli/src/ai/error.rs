use thiserror::Error;

/// Adapter-specific error types
#[derive(Error, Debug)]
pub enum AdapterError {
    #[error("initialization failed: {0}")]
    InitializationError(String),

    #[error("API call failed: {0}")]
    ApiError(String),

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("timeout after {0}ms")]
    Timeout(u64),

    #[error("rate limit exceeded, retry after {0}s")]
    RateLimited(u64),

    #[error("adapter not found: {0}")]
    AdapterNotFound(String),

    #[error("configuration error: {0}")]
    ConfigError(String),

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("json parsing error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("unknown error: {0}")]
    Unknown(String),
}

impl AdapterError {
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AdapterError::Timeout(_) | AdapterError::RateLimited(_) | AdapterError::NetworkError(_)
        )
    }
}

/// Result type for adapter operations
pub type AdapterResult<T> = Result<T, AdapterError>;
