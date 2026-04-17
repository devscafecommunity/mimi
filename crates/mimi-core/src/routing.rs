//! Message Routing Middleware
//!
//! Provides topic-based message dispatch with MQTT-style wildcard pattern matching,
//! handler registration, and thread-safe concurrent access.

use anyhow::{anyhow, Result};
use std::fmt;

/// Routing error types with detailed context
#[derive(Debug, Clone)]
pub enum RoutingError {
    /// No handler found for the given topic
    NoHandlerFound { topic: String },
    /// Topic format is invalid
    InvalidTopic { topic: String, reason: String },
    /// Pattern format is invalid
    InvalidPattern { pattern: String, reason: String },
    /// Handler execution failed
    HandlerFailed { pattern: String, error: String },
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHandlerFound { topic } => {
                write!(f, "No handler found for topic: {}", topic)
            },
            Self::InvalidTopic { topic, reason } => {
                write!(f, "Invalid topic '{}': {}", topic, reason)
            },
            Self::InvalidPattern { pattern, reason } => {
                write!(f, "Invalid pattern '{}': {}", pattern, reason)
            },
            Self::HandlerFailed { pattern, error } => {
                write!(f, "Handler failed for pattern '{}': {}", pattern, error)
            },
        }
    }
}

impl std::error::Error for RoutingError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_error_display() {
        let err = RoutingError::NoHandlerFound {
            topic: "mimi/commands/execute".to_string(),
        };
        assert!(err.to_string().contains("No handler found"));
        assert!(err.to_string().contains("mimi/commands/execute"));
    }
}
