//! MiMi Core - Core system module for cognitive architecture
//!
//! This module contains the fundamental components and trait definitions
//! for the MiMi cognitive operating system.

pub mod error;
pub mod message;
pub mod config;

pub use error::{Error, Result};

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
