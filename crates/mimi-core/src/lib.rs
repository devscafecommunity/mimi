//! MiMi Core - Core system module for cognitive architecture
//!
//! This module contains the fundamental components and trait definitions
//! for the MiMi cognitive operating system.

pub mod config;
pub mod context_query;
pub mod error;
pub mod health_monitor;
pub mod heatmap;
pub mod l1_cache;
pub mod pandora_client;
pub mod pandora_integration;
pub mod message;
pub mod odlaguna;
pub mod odlaguna_intervention;
pub mod odlaguna_supervisor;
pub mod intervention_setup;
pub mod mimi_daemon;
pub mod priscilla;
pub mod priscilla_failure;
pub mod circuit_guard;
pub mod circuit_metrics;
pub mod circuit_registry;
pub mod skill_executor;
pub mod skill_supervision;
pub mod routing;
pub mod serialization;
pub mod state_machine;
pub mod zenoh_bus;

pub use error::{Error, Result};
pub use health_monitor::{HealthMetric, HealthMetricType, HealthMonitor};
pub use circuit_guard::{pandora_context_with_fallback, with_circuit_breaker};
pub use circuit_metrics::{
    CircuitBreakerMetrics, CircuitBreakerSnapshot, SUSTAINED_OPEN_ALERT_MS,
};
pub use circuit_registry::{circuit_breakers, CircuitBreakerRegistry, RegistrySnapshots};
pub use odlaguna::{AuditEvent, ExpiredTask, MonitoredTask, OdlagunaState, OdlagunaWatchdog};
pub use skill_supervision::{
    intervene_expired_containers, odlaguna_deadline, OdlagunaDeadlineError,
};
pub use skill_executor::{
    run_isolated_skill_supervised, skill_run_succeeded, IsolatedSkillPayload,
    DEFAULT_WORKER_IMAGE,
};
#[cfg(feature = "docker-kill")]
pub use skill_supervision::run_bollard_skill_supervised;
pub use heatmap::HeatmapManager;
pub use l1_cache::{CachedContext, L1CacheStats, L1ContextCache, M2_L1_HIT_RATE_TARGET};
pub use odlaguna_intervention::{emit_intervention_side_effects, InterventionContext};
pub use odlaguna_supervisor::{
    odlaguna_global, odlaguna_supervisor_requested, spawn_odlaguna_supervisor,
    supervisor_is_running, zenoh_requested, OdlagunaSupervisorLoop,
};
pub use intervention_setup::{
    connect_zenoh_if_requested, intervention_context, start_daemon_odlaguna_supervisor,
    start_odlaguna_supervisor_if_requested, start_scoped_odlaguna_supervisor,
};
pub use mimi_daemon::run_daemon;
pub use pandora_client::{
    ContextBundle, ContextHeatRecord, ContextNeighbor, ContextRelation, FailurePattern,
    MimiCheckpoint, Neo4jConfig, OdlagunaAuditRecord, PandoraClient, StateHistoryRecord,
};
pub use priscilla::{
    CritiqueCommentaryMessage, PriscillaAdvisor, PriscillaConfig, TaskDraftMessage,
};
pub use zenoh_bus::TaskTimeoutMessage;
pub use pandora_integration::{
    checkpoint_interval_from_env, checkpoint_mimi_state, execute_next_task_with_pandora,
    execute_task_with_context, fetch_task_context, flush_interval_from_env, flush_with_retry,
    PandoraBusMessage, PandoraFlushLoop, restore_mimi_state, TaskContextPayload, TaskRunSummary,
};
pub use routing::{MessageRouter, RoutingError, Topic, TopicPattern};
pub use serialization::{MessageSerializer, SerializationError};
pub use state_machine::{
    CircuitBreaker, ComponentHealthCheck, MimiState, StateManager,
};
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
