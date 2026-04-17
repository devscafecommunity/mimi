//! State Machine Integration Tests

use mimi_core::state_machine::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Full Lifecycle Tests
// ============================================================================

#[tokio::test]
async fn test_full_task_lifecycle() {
    let manager = Arc::new(StateManager::new());

    // Idle -> Listening
    manager.transition_to(MimiState::Listening).unwrap();
    assert_eq!(manager.current_state(), MimiState::Listening);

    // Queue task
    let task = Task::new(TaskType::Execute, "lifecycle_test")
        .with_execution_model(ExecutionModel::Blocking);
    manager.enqueue_task(task).unwrap();

    // Listening -> Processing
    manager.transition_to(MimiState::Processing).unwrap();

    // Processing -> Executing -> Responding
    manager.execute_next_task().await.unwrap();

    // Responding -> Idle
    manager.transition_to(MimiState::Idle).unwrap();
    assert_eq!(manager.current_state(), MimiState::Idle);
}

#[tokio::test]
async fn test_error_recovery_flow() {
    let manager = Arc::new(StateManager::new());

    manager.transition_to(MimiState::Listening).unwrap();

    // Simulate component failure
    manager.force_error_state(MimiState::FailedComponent);
    assert_eq!(manager.current_state(), MimiState::FailedComponent);

    // Enter recovery
    manager.transition_to(MimiState::Recovering).unwrap();
    assert_eq!(manager.current_state(), MimiState::Recovering);

    // Recover to Idle
    manager.transition_to(MimiState::Idle).unwrap();
    assert_eq!(manager.current_state(), MimiState::Idle);
}

#[tokio::test]
async fn test_degraded_mode_operation() {
    let manager = Arc::new(StateManager::new());

    manager.transition_to(MimiState::Listening).unwrap();

    // Enter degraded mode
    manager.transition_to(MimiState::Degraded).unwrap();

    // Should still be able to queue tasks
    let task = Task::new(TaskType::Query, "degraded_task").with_priority(TaskPriority::Low);
    assert!(manager.enqueue_task(task).is_ok());

    // Recover
    manager.transition_to(MimiState::Recovering).unwrap();
    manager.transition_to(MimiState::Idle).unwrap();
}

// ============================================================================
// Retry and Circuit Breaker Integration
// ============================================================================

#[tokio::test]
async fn test_circuit_breaker_prevents_overload() {
    let breaker = Arc::new(CircuitBreaker::new(3, Duration::from_secs(5)));
    let manager = Arc::new(StateManager::new());

    // Simulate 3 failures
    for _ in 0..3 {
        breaker.record_failure();
    }

    assert_eq!(breaker.state(), CircuitState::Open);

    // Circuit should block requests
    assert!(!breaker.allow_request());

    // Wait for half-open
    sleep(Duration::from_secs(6)).await;
    assert_eq!(breaker.state(), CircuitState::HalfOpen);

    // Test request allowed
    assert!(breaker.allow_request());
}

#[tokio::test]
async fn test_task_queue_priority_under_load() {
    let manager = Arc::new(StateManager::with_capacity(100));

    // Queue mixed-priority tasks
    for i in 0..50 {
        let priority = match i % 3 {
            0 => TaskPriority::Critical,
            1 => TaskPriority::High,
            _ => TaskPriority::Normal,
        };

        let task = Task::new(TaskType::Execute, &format!("task_{}", i)).with_priority(priority);
        manager.enqueue_task(task).unwrap();
    }

    assert_eq!(manager.queue_size(), 50);

    // First dequeued should be critical
    let first = manager.dequeue_task().unwrap();
    assert_eq!(first.priority, TaskPriority::Critical);
}

// ============================================================================
// Concurrent Access Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_task_enqueue() {
    let manager = Arc::new(StateManager::new());

    let mut handles = vec![];

    for i in 0..10 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let task = Task::new(TaskType::Query, &format!("task_{}", i));
            mgr.enqueue_task(task).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(manager.queue_size(), 10);
}

#[tokio::test]
async fn test_concurrent_state_transitions() {
    let manager = Arc::new(StateManager::new());

    let mut handles = vec![];

    for _ in 0..5 {
        let mgr = manager.clone();
        let handle = tokio::spawn(async move {
            let _ = mgr.transition_to(MimiState::Listening);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Should end up in Listening state
    assert_eq!(manager.current_state(), MimiState::Listening);
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_high_throughput_task_processing() {
    let manager = Arc::new(StateManager::with_capacity(10000));

    // Enqueue 1000 tasks
    for i in 0..1000 {
        let task =
            Task::new(TaskType::Query, &format!("task_{}", i)).with_priority(if i % 2 == 0 {
                TaskPriority::High
            } else {
                TaskPriority::Normal
            });
        manager.enqueue_task(task).unwrap();
    }

    assert_eq!(manager.queue_size(), 1000);

    // Dequeue all
    for _ in 0..1000 {
        assert!(manager.dequeue_task().is_ok());
    }

    assert_eq!(manager.queue_size(), 0);
}

#[tokio::test]
async fn test_state_transition_sequence() {
    let manager = Arc::new(StateManager::new());

    let sequence = vec![
        MimiState::Listening,
        MimiState::Processing,
        MimiState::Executing,
        MimiState::Responding,
        MimiState::Idle,
    ];

    for state in sequence {
        let result = manager.transition_to(state);
        assert!(result.is_ok(), "Failed to transition to {:?}", state);
        assert_eq!(manager.current_state(), state);
    }
}
