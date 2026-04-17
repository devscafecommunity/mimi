//! State Machine Unit Tests

use mimi_core::state_machine::{
    ComponentHealth, ComponentHealthCheck, MimiState, StateManager, StateTransition,
    TransitionGuard,
};

#[test]
fn test_initial_state_is_idle() {
    let manager = StateManager::new();
    assert_eq!(manager.current_state(), MimiState::Idle);
}

#[test]
fn test_valid_state_transition_idle_to_listening() {
    let transition = StateTransition::new(MimiState::Idle, MimiState::Listening);
    assert!(transition.is_valid());
}

#[test]
fn test_invalid_state_transition_idle_to_executing() {
    let transition = StateTransition::new(MimiState::Idle, MimiState::Executing);
    assert!(!transition.is_valid());
}

#[test]
fn test_guard_condition_healthy_component() {
    let health = ComponentHealth {
        latency_ms: 100,
        memory_usage_percent: 50,
        last_heartbeat_secs: 5,
    };

    assert!(TransitionGuard::check_component_health(&health));
}

#[test]
fn test_guard_condition_unhealthy_high_latency() {
    let health = ComponentHealth {
        latency_ms: 6000,
        memory_usage_percent: 50,
        last_heartbeat_secs: 5,
    };

    assert!(!TransitionGuard::check_component_health(&health));
}

#[test]
fn test_guard_condition_unhealthy_high_memory() {
    let health = ComponentHealth {
        latency_ms: 100,
        memory_usage_percent: 85,
        last_heartbeat_secs: 5,
    };

    assert!(!TransitionGuard::check_component_health(&health));
}

#[test]
fn test_transition_state_success() {
    let manager = StateManager::new();

    let result = manager.transition_to(MimiState::Listening);
    assert!(result.is_ok());
    assert_eq!(manager.current_state(), MimiState::Listening);
}

#[test]
fn test_transition_state_invalid() {
    let manager = StateManager::new();

    let result = manager.transition_to(MimiState::Executing);
    assert!(result.is_err());
    assert_eq!(manager.current_state(), MimiState::Idle);
}

#[test]
fn test_transition_with_health_check() {
    let manager = StateManager::new();

    let unhealthy = ComponentHealth {
        latency_ms: 6000,
        memory_usage_percent: 50,
        last_heartbeat_secs: 5,
    };

    let result = manager.check_and_transition(MimiState::Listening, &unhealthy);
    assert!(result.is_ok());
    assert_eq!(manager.current_state(), MimiState::Degraded);
}

#[test]
fn test_component_health_check_is_healthy() {
    use mimi_core::state_machine::ComponentHealthCheck;

    let health = ComponentHealthCheck::new(100, 50, 5);
    assert!(health.is_healthy());
}

#[test]
fn test_component_health_check_unhealthy_latency() {
    use mimi_core::state_machine::ComponentHealthCheck;

    // Latency >5s = DEGRADED
    let health = ComponentHealthCheck::new(6000, 50, 5);
    assert!(!health.is_healthy());
}

#[test]
fn test_component_health_check_unhealthy_memory() {
    use mimi_core::state_machine::ComponentHealthCheck;

    // Memory >80% = DEGRADED
    let health = ComponentHealthCheck::new(100, 85, 5);
    assert!(!health.is_healthy());
}

#[test]
fn test_component_health_check_unhealthy_heartbeat() {
    use mimi_core::state_machine::ComponentHealthCheck;

    // Heartbeat missing >30s = RECOVERING
    let health = ComponentHealthCheck::new(100, 50, 35);
    assert!(!health.is_healthy());
}

#[test]
fn test_health_monitoring_auto_degrade() {
    let manager = StateManager::new();

    let healthy = ComponentHealthCheck::new(100, 50, 5);
    manager.update_component_health(healthy).unwrap();
    manager.transition_to(MimiState::Listening).unwrap();
    assert_eq!(manager.current_state(), MimiState::Listening);

    let unhealthy = ComponentHealthCheck::new(6000, 50, 5);
    manager.update_component_health(unhealthy).unwrap();

    assert_eq!(manager.current_state(), MimiState::Degraded);
}

#[test]
fn test_health_monitoring_auto_recovering() {
    let manager = StateManager::new();

    let unhealthy = ComponentHealthCheck::new(100, 50, 35);
    manager.update_component_health(unhealthy).unwrap();
    manager.transition_to(MimiState::Listening).unwrap();

    assert_eq!(manager.current_state(), MimiState::Recovering);
}
