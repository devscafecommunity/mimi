use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Access denied: insufficient permissions")]
    AccessDenied,

    #[error("Authentication required")]
    AuthenticationRequired,

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("User not found")]
    UserNotFound,

    #[error("Role not found: {0}")]
    RoleNotFound(String),

    #[error("Internal auth error: {0}")]
    Internal(String),
}

impl AuthError {
    pub fn error_code(&self) -> u32 {
        match self {
            AuthError::InvalidToken(_) => 5001,
            AuthError::TokenExpired => 5002,
            AuthError::AccessDenied => 5003,
            AuthError::AuthenticationRequired => 5004,
            AuthError::InvalidCredentials => 5005,
            AuthError::JwtError(_) => 5006,
            AuthError::UserNotFound => 5007,
            AuthError::RoleNotFound(_) => 5008,
            AuthError::Internal(_) => 5999,
        }
    }
}
