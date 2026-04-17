//! MiMi Core - Core system module for cognitive architecture
//!
//! This module contains the fundamental components and trait definitions
//! for the MiMi cognitive operating system.

pub mod config;
pub mod error;
pub mod health_monitor;
pub mod message;
pub mod pandora_client;
pub mod routing;
pub mod serialization;
pub mod state_machine;
pub mod zenoh_bus;

pub use error::{Error, Result};
pub use health_monitor::{HealthMetric, HealthMetricType, HealthMonitor};
pub use pandora_client::{FailurePattern, Neo4jConfig, PandoraClient, StateHistoryRecord};
pub use routing::{MessageRouter, RoutingError, Topic, TopicPattern};
pub use serialization::{MessageSerializer, SerializationError};
pub use state_machine::{ComponentHealthCheck, MimiState, StateManager};
pub use zenoh_bus::{StateChangeMessage, ZenohBusAdapter, ZenohConfig};

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
