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

// ============================================================================
// Task Tests
// ============================================================================

use mimi_core::state_machine::{Task, TaskPriority, TaskType};
use std::time::Duration;

#[test]
fn test_task_creation_with_defaults() {
    let task = Task::new(TaskType::Query, "test_task");

    assert_eq!(task.task_type, TaskType::Query);
    assert_eq!(task.priority, TaskPriority::Normal);
    assert_eq!(task.retries, 0);
    assert_eq!(task.max_retries, 3);
    assert!(task.timeout.as_secs() == 30);
}

#[test]
fn test_task_with_high_priority() {
    let task = Task::new(TaskType::Execute, "critical_task")
        .with_priority(TaskPriority::Critical)
        .with_timeout(Duration::from_secs(60));

    assert_eq!(task.priority, TaskPriority::Critical);
    assert_eq!(task.timeout.as_secs(), 60);
}

// ============================================================================
// Task Queue Tests
// ============================================================================

#[test]
fn test_task_queue_fifo_within_priority() {
    let manager = StateManager::new();

    let task1 = Task::new(TaskType::Query, "query1").with_priority(TaskPriority::Normal);
    let task2 = Task::new(TaskType::Execute, "exec1").with_priority(TaskPriority::High);
    let task3 = Task::new(TaskType::Query, "query2").with_priority(TaskPriority::Normal);

    manager.enqueue_task(task1.clone()).unwrap();
    manager.enqueue_task(task2.clone()).unwrap();
    manager.enqueue_task(task3.clone()).unwrap();

    let dequeued = manager.dequeue_task().unwrap();
    assert_eq!(dequeued.name, "exec1");

    let dequeued = manager.dequeue_task().unwrap();
    assert_eq!(dequeued.name, "query1");

    let dequeued = manager.dequeue_task().unwrap();
    assert_eq!(dequeued.name, "query2");
}

#[test]
fn test_task_queue_capacity_limit() {
    let manager = StateManager::with_capacity(2);

    let task1 = Task::new(TaskType::Query, "task1");
    let task2 = Task::new(TaskType::Query, "task2");
    let task3 = Task::new(TaskType::Query, "task3");

    assert!(manager.enqueue_task(task1).is_ok());
    assert!(manager.enqueue_task(task2).is_ok());

    let result = manager.enqueue_task(task3);
    assert!(result.is_err());
}

// ============================================================================
// Circuit Breaker Tests
// ============================================================================

use mimi_core::state_machine::{CircuitBreaker, CircuitState};

#[test]
fn test_circuit_breaker_opens_after_failures() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(10));

    assert_eq!(breaker.state(), CircuitState::Closed);

    // Record 3 failures
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_failure();

    // Should open after 3 failures
    assert_eq!(breaker.state(), CircuitState::Open);
}

#[test]
fn test_circuit_breaker_half_open_after_timeout() {
    let breaker = CircuitBreaker::new(3, Duration::from_millis(100));

    // Open the circuit
    breaker.record_failure();
    breaker.record_failure();
    breaker.record_failure();

    assert_eq!(breaker.state(), CircuitState::Open);

    // Wait for timeout
    std::thread::sleep(Duration::from_millis(150));

    // Should transition to HalfOpen
    assert_eq!(breaker.state(), CircuitState::HalfOpen);
}

#[test]
fn test_circuit_breaker_closes_on_success() {
    let breaker = CircuitBreaker::new(3, Duration::from_millis(100));

    // Open circuit
    for _ in 0..3 {
        breaker.record_failure();
    }

    // Wait for half-open
    std::thread::sleep(Duration::from_millis(150));
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    // Success should close circuit
    breaker.record_success();
    assert_eq!(breaker.state(), CircuitState::Closed);
}

// ============================================================================
// Async Task Execution Tests
// ============================================================================

use mimi_core::state_machine::ExecutionModel;

#[tokio::test]
async fn test_execute_task_blocking_mode() {
    let manager = StateManager::new();
    manager.transition_to(MimiState::Listening).unwrap();
    manager.transition_to(MimiState::Processing).unwrap();

    let task =
        Task::new(TaskType::Query, "fast_query").with_execution_model(ExecutionModel::Blocking);

    manager.enqueue_task(task).unwrap();

    let result = manager.execute_next_task().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_task_async_mode() {
    let manager = StateManager::new();
    manager.transition_to(MimiState::Listening).unwrap();
    manager.transition_to(MimiState::Processing).unwrap();

    let task = Task::new(TaskType::Execute, "slow_exec")
        .with_execution_model(ExecutionModel::Async)
        .with_timeout(Duration::from_secs(5));

    manager.enqueue_task(task).unwrap();

    let result = manager.execute_next_task().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_timeout_handling() {
    let manager = StateManager::new();
    manager.transition_to(MimiState::Listening).unwrap();
    manager.transition_to(MimiState::Processing).unwrap();

    let task = Task::new(TaskType::Execute, "timeout_task").with_timeout(Duration::from_millis(10));

    manager.enqueue_task(task).unwrap();

    let result = manager.execute_next_task().await;
    assert!(result.is_err());
}

// ============================================================================
// Retry Strategy Tests
// ============================================================================

use mimi_core::state_machine::RetryStrategy;

#[test]
fn test_exponential_backoff_sequence() {
    let strategy = RetryStrategy::exponential();

    let delay1 = strategy.next_delay(0);
    assert_eq!(delay1.as_millis(), 100);

    let delay2 = strategy.next_delay(1);
    assert_eq!(delay2.as_millis(), 200);

    let delay3 = strategy.next_delay(2);
    assert_eq!(delay3.as_millis(), 400);

    let delay4 = strategy.next_delay(10);
    assert_eq!(delay4.as_millis(), 5000);
}

#[test]
fn test_retry_with_jitter() {
    let strategy = RetryStrategy::exponential_with_jitter();

    let delay = strategy.next_delay(2);

    assert!(delay.as_millis() >= 320);
    assert!(delay.as_millis() <= 480);
}

#[tokio::test]
async fn test_execute_with_retry_success() {
    let manager = StateManager::new();
    manager.transition_to(MimiState::Listening).unwrap();
    manager.transition_to(MimiState::Processing).unwrap();

    let task =
        Task::new(TaskType::Execute, "retry_task").with_execution_model(ExecutionModel::Blocking);

    manager.enqueue_task(task).unwrap();

    let result = manager.execute_with_retry().await;
    assert!(result.is_ok());
}
