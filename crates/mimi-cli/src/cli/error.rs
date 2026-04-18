use thiserror::Error;

/// Exit codes following sysexits.h convention
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_GENERAL_ERROR: i32 = 1;
pub const EXIT_USAGE_ERROR: i32 = 2;
pub const EXIT_CONFIG_ERROR: i32 = 3;
pub const EXIT_NETWORK_ERROR: i32 = 4;
pub const EXIT_AUTH_ERROR: i32 = 5;
pub const EXIT_NOT_FOUND: i32 = 6;
pub const EXIT_IO_ERROR: i32 = 64;
pub const EXIT_UNKNOWN_ERROR: i32 = 99;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("Usage error: {0}")]
    UsageError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Bus error: {0}")]
    BusError(String),

    #[error("Task execution failed: {0}")]
    ExecutionError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

impl CliError {
    /// Get exit code for this error
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::UsageError(_) => EXIT_USAGE_ERROR,
            CliError::ConfigError(_) => EXIT_CONFIG_ERROR,
            CliError::NetworkError(_) => EXIT_NETWORK_ERROR,
            CliError::AuthError(_) => EXIT_AUTH_ERROR,
            CliError::NotFound(_) => EXIT_NOT_FOUND,
            CliError::IoError(_) => EXIT_IO_ERROR,
            CliError::BusError(_) => EXIT_NETWORK_ERROR,
            CliError::ExecutionError(_) => EXIT_GENERAL_ERROR,
            CliError::UnknownError(_) => EXIT_UNKNOWN_ERROR,
        }
    }

    /// Format error for user display
    pub fn user_message(&self) -> String {
        match self {
            CliError::UsageError(msg) => format!("ERROR [E_USAGE]: {}\n  hint: Check syntax with 'mimi --help'", msg),
            CliError::ConfigError(msg) => format!("ERROR [E_CONFIG]: {}\n  hint: Run 'mimi config init production'", msg),
            CliError::NetworkError(msg) => format!("ERROR [E_NETWORK]: {}\n  hint: Ensure Zenoh bus is running", msg),
            CliError::AuthError(msg) => format!("ERROR [E_AUTH]: {}", msg),
            CliError::NotFound(msg) => format!("ERROR [E_NOT_FOUND]: {}", msg),
            CliError::IoError(e) => format!("ERROR [E_IO]: {}", e),
            CliError::BusError(msg) => format!("ERROR [E_BUS]: {}", msg),
            CliError::ExecutionError(msg) => format!("ERROR [E_EXEC]: {}", msg),
            CliError::UnknownError(msg) => format!("ERROR [E_UNKNOWN]: {}", msg),
        }
    }
}

/// Result type for CLI operations
pub type CliResult<T> = Result<T, CliError>;
