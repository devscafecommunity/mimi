//! MiMi Core - Core system module for cognitive architecture
//!
//! This module contains the fundamental components and trait definitions
//! for the MiMi cognitive operating system.

pub mod config;
pub mod error;
pub mod message;
pub mod routing;
pub mod serialization;

pub use error::{Error, Result};
pub use routing::{MessageRouter, RoutingError, Topic, TopicPattern};
pub use serialization::{MessageSerializer, SerializationError};

/// Core version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
