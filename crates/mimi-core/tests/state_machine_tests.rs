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

// ============================================================================
// Additional State Tests
// ============================================================================

#[test]
fn test_all_state_variants() {
    let states = vec![
        MimiState::Idle,
        MimiState::Listening,
        MimiState::Processing,
        MimiState::Executing,
        MimiState::Responding,
        MimiState::Degraded,
        MimiState::Recovering,
        MimiState::FailedComponent,
        MimiState::CriticalError,
        MimiState::Shutdown,
    ];

    assert_eq!(states.len(), 10);
}

#[test]
fn test_state_equality() {
    assert_eq!(MimiState::Idle, MimiState::Idle);
    assert_ne!(MimiState::Idle, MimiState::Listening);
}

#[test]
fn test_all_valid_normal_flow_transitions() {
    let transitions = vec![
        (MimiState::Idle, MimiState::Listening),
        (MimiState::Listening, MimiState::Processing),
        (MimiState::Processing, MimiState::Executing),
        (MimiState::Executing, MimiState::Responding),
        (MimiState::Responding, MimiState::Idle),
    ];

    for (from, to) in transitions {
        let t = StateTransition::new(from, to);
        assert!(t.is_valid(), "Expected {:?} -> {:?} to be valid", from, to);
    }
}

#[test]
fn test_error_escalation_from_any_state() {
    let states = vec![
        MimiState::Idle,
        MimiState::Listening,
        MimiState::Processing,
        MimiState::Executing,
        MimiState::Responding,
    ];

    for state in states {
        let t1 = StateTransition::new(state, MimiState::Degraded);
        assert!(t1.is_valid());

        let t2 = StateTransition::new(state, MimiState::FailedComponent);
        assert!(t2.is_valid());

        let t3 = StateTransition::new(state, MimiState::CriticalError);
        assert!(t3.is_valid());
    }
}

#[test]
fn test_recovery_paths() {
    let t1 = StateTransition::new(MimiState::Degraded, MimiState::Recovering);
    assert!(t1.is_valid());

    let t2 = StateTransition::new(MimiState::FailedComponent, MimiState::Recovering);
    assert!(t2.is_valid());

    let t3 = StateTransition::new(MimiState::Recovering, MimiState::Idle);
    assert!(t3.is_valid());
}

#[test]
fn test_invalid_transitions() {
    let invalid = vec![
        (MimiState::Idle, MimiState::Processing),
        (MimiState::Idle, MimiState::Executing),
        (MimiState::Listening, MimiState::Responding),
        (MimiState::Processing, MimiState::Idle),
    ];

    for (from, to) in invalid {
        let t = StateTransition::new(from, to);
        assert!(
            !t.is_valid(),
            "Expected {:?} -> {:?} to be invalid",
            from,
            to
        );
    }
}

#[test]
fn test_guard_all_thresholds() {
    // All healthy
    let h1 = ComponentHealth {
        latency_ms: 5000,
        memory_usage_percent: 80,
        last_heartbeat_secs: 30,
    };
    assert!(TransitionGuard::check_component_health(&h1));

    // Just over latency threshold
    let h2 = ComponentHealth {
        latency_ms: 5001,
        memory_usage_percent: 80,
        last_heartbeat_secs: 30,
    };
    assert!(!TransitionGuard::check_component_health(&h2));

    // Just over memory threshold
    let h3 = ComponentHealth {
        latency_ms: 5000,
        memory_usage_percent: 81,
        last_heartbeat_secs: 30,
    };
    assert!(!TransitionGuard::check_component_health(&h3));

    // Just over heartbeat threshold
    let h4 = ComponentHealth {
        latency_ms: 5000,
        memory_usage_percent: 80,
        last_heartbeat_secs: 31,
    };
    assert!(!TransitionGuard::check_component_health(&h4));
}

#[test]
fn test_guard_queue_capacity() {
    assert!(TransitionGuard::check_queue_capacity(99, 100));
    assert!(!TransitionGuard::check_queue_capacity(100, 100));
}

#[test]
fn test_guard_task_timeout() {
    let timeout1 = Duration::from_secs(30);
    let timeout2 = Duration::from_secs(60);
    let max = Duration::from_secs(60);

    assert!(TransitionGuard::check_task_timeout(&timeout1, &max));
    assert!(TransitionGuard::check_task_timeout(&timeout2, &max));
    assert!(!TransitionGuard::check_task_timeout(
        &Duration::from_secs(61),
        &max
    ));
}

#[test]
fn test_task_builder_chain() {
    let task = Task::new(TaskType::Execute, "complex_task")
        .with_priority(TaskPriority::High)
        .with_timeout(Duration::from_secs(120))
        .with_execution_model(ExecutionModel::Async)
        .with_payload(vec![1, 2, 3]);

    assert_eq!(task.priority, TaskPriority::High);
    assert_eq!(task.timeout.as_secs(), 120);
    assert_eq!(task.execution_model, ExecutionModel::Async);
    assert_eq!(task.payload, vec![1, 2, 3]);
}

#[test]
fn test_task_can_retry() {
    let mut task = Task::new(TaskType::Query, "test");

    assert!(task.can_retry());

    task.increment_retry();
    task.increment_retry();
    task.increment_retry();

    assert!(!task.can_retry());
}

#[test]
fn test_retry_strategy_progression() {
    let strategy = RetryStrategy::exponential();

    let delays: Vec<u64> = (0..5)
        .map(|i| strategy.next_delay(i).as_millis() as u64)
        .collect();

    assert_eq!(delays, vec![100, 200, 400, 800, 1600]);
}

#[test]
fn test_retry_strategy_max_cap() {
    let strategy = RetryStrategy::exponential();

    let delay = strategy.next_delay(20);
    assert_eq!(delay.as_millis(), 5000);
}

#[test]
fn test_circuit_breaker_initial_state() {
    let breaker = CircuitBreaker::new(3, Duration::from_secs(10));
    assert_eq!(breaker.state(), CircuitState::Closed);
    assert!(breaker.allow_request());
}

#[test]
fn test_circuit_breaker_blocks_when_open() {
    let breaker = CircuitBreaker::new(1, Duration::from_secs(10));

    breaker.record_failure();

    assert_eq!(breaker.state(), CircuitState::Open);
    assert!(!breaker.allow_request());
}

#[test]
fn test_circuit_breaker_reset() {
    let breaker = CircuitBreaker::new(1, Duration::from_secs(10));

    breaker.record_failure();
    assert_eq!(breaker.state(), CircuitState::Open);

    breaker.reset();
    assert_eq!(breaker.state(), CircuitState::Closed);
}

#[test]
fn test_state_manager_default_state() {
    let manager = StateManager::new();
    assert_eq!(manager.current_state(), MimiState::Idle);
}

#[test]
fn test_state_manager_queue_size() {
    let manager = StateManager::new();
    assert_eq!(manager.queue_size(), 0);

    let task = Task::new(TaskType::Query, "test");
    manager.enqueue_task(task).unwrap();

    assert_eq!(manager.queue_size(), 1);
}

#[test]
fn test_state_manager_force_error_state() {
    let manager = StateManager::new();

    manager.force_error_state(MimiState::CriticalError);
    assert_eq!(manager.current_state(), MimiState::CriticalError);
}
