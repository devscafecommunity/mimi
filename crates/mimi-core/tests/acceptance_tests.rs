//! Acceptance Tests for State Machine
//!
//! High-level end-to-end scenarios validating system behavior

use chrono::Utc;
use mimi_core::state_machine::*;
use mimi_core::{HealthMetric, HealthMetricType, HealthMonitor};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// Scenario 1: Happy Path - Complete Task Execution
// ============================================================================

#[tokio::test]
async fn acceptance_happy_path_complete_workflow() {
    let manager = Arc::new(StateManager::new());
    let health_monitor = Arc::new(HealthMonitor::new());

    manager.transition_to(MimiState::Listening).unwrap();

    let task = Task::new(TaskType::Query, "user_query")
        .with_priority(TaskPriority::Normal)
        .with_execution_model(ExecutionModel::Blocking)
        .with_payload(b"What is the weather?".to_vec());

    manager.enqueue_task(task).unwrap();

    manager.transition_to(MimiState::Processing).unwrap();

    let metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "beatrice".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 100.0,
        threshold: 5000.0,
        is_healthy: true,
    };
    health_monitor.record_metric(metric).await.unwrap();

    manager.execute_next_task().await.unwrap();

    manager.transition_to(MimiState::Idle).unwrap();

    assert_eq!(manager.current_state(), MimiState::Idle);
    assert_eq!(manager.queue_size(), 0);

    let recent_metrics = health_monitor.get_recent_metrics(10);
    assert!(!recent_metrics.is_empty());
}

// ============================================================================
// Scenario 2: Component Failure Recovery
// ============================================================================

#[tokio::test]
async fn acceptance_component_failure_and_recovery() {
    let manager = Arc::new(StateManager::new());
    let health_monitor = Arc::new(HealthMonitor::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, Duration::from_secs(5)));

    manager.transition_to(MimiState::Listening).unwrap();

    let failure_metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "pandora".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 7000.0,
        threshold: 5000.0,
        is_healthy: false,
    };
    health_monitor.record_metric(failure_metric).await.unwrap();

    manager.force_error_state(MimiState::FailedComponent);

    circuit_breaker.record_failure();
    circuit_breaker.record_failure();
    circuit_breaker.record_failure();

    assert_eq!(circuit_breaker.state(), CircuitState::Open);

    manager.transition_to(MimiState::Recovering).unwrap();

    let recovery_metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "pandora".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 200.0,
        threshold: 5000.0,
        is_healthy: true,
    };
    health_monitor.record_metric(recovery_metric).await.unwrap();

    manager.transition_to(MimiState::Idle).unwrap();

    assert_eq!(manager.current_state(), MimiState::Idle);
}

// ============================================================================
// Scenario 3: Cascade Fallback with Retries
// ============================================================================

#[tokio::test]
async fn acceptance_cascade_fallback_retry_strategy() {
    let manager = Arc::new(StateManager::new());
    let retry_strategy = RetryStrategy::exponential_with_jitter();

    manager.transition_to(MimiState::Listening).unwrap();

    let mut task = Task::new(TaskType::Execute, "complex_api_call")
        .with_priority(TaskPriority::High)
        .with_execution_model(ExecutionModel::Async)
        .with_timeout(Duration::from_secs(10));

    task.max_retries = 5;

    manager.enqueue_task(task.clone()).unwrap();

    let mut retry_count = 0;
    loop {
        let result = manager.execute_next_task().await;

        if result.is_ok() {
            break;
        }

        if retry_count >= task.max_retries {
            manager.force_error_state(MimiState::Degraded);
            break;
        }

        let delay = retry_strategy.next_delay(retry_count);
        sleep(delay).await;

        retry_count += 1;
        manager.enqueue_task(task.clone()).unwrap();
    }

    let state = manager.current_state();
    assert!(state == MimiState::Idle || state == MimiState::Degraded);
}

// ============================================================================
// Scenario 4: Graceful Shutdown
// ============================================================================

#[tokio::test]
async fn acceptance_graceful_shutdown_with_pending_tasks() {
    let manager = Arc::new(StateManager::new());

    manager.transition_to(MimiState::Listening).unwrap();

    for i in 0..5 {
        let task = Task::new(TaskType::Execute, &format!("task_{}", i))
            .with_priority(TaskPriority::Normal);
        manager.enqueue_task(task).unwrap();
    }

    assert_eq!(manager.queue_size(), 5);

    manager.transition_to(MimiState::Shutdown).unwrap();

    while manager.queue_size() > 0 {
        let result = manager.execute_next_task().await;

        if result.is_err() {
            break;
        }
    }

    assert_eq!(manager.current_state(), MimiState::Shutdown);
}

// ============================================================================
// Scenario 5: Chaos Engineering - Multiple Simultaneous Failures
// ============================================================================

#[tokio::test]
async fn acceptance_chaos_multiple_failures() {
    let manager = Arc::new(StateManager::with_capacity(100));
    let health_monitor = Arc::new(HealthMonitor::new());
    let circuit_breaker = Arc::new(CircuitBreaker::new(3, Duration::from_secs(2)));

    manager.transition_to(MimiState::Listening).unwrap();

    for i in 0..50 {
        let task = Task::new(TaskType::Execute, &format!("burst_task_{}", i))
            .with_priority(TaskPriority::High);
        let _ = manager.enqueue_task(task);
    }

    let beatrice_metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "beatrice".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 8000.0,
        threshold: 5000.0,
        is_healthy: false,
    };
    health_monitor.record_metric(beatrice_metric).await.unwrap();

    let pandora_metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "pandora".to_string(),
        metric_type: HealthMetricType::MemoryUsage,
        value: 95.0,
        threshold: 80.0,
        is_healthy: false,
    };
    health_monitor.record_metric(pandora_metric).await.unwrap();

    let echidna_metric = HealthMetric {
        timestamp: Utc::now(),
        component_name: "echidna".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 6500.0,
        threshold: 5000.0,
        is_healthy: false,
    };
    health_monitor.record_metric(echidna_metric).await.unwrap();

    for _ in 0..3 {
        circuit_breaker.record_failure();
    }

    assert_eq!(circuit_breaker.state(), CircuitState::Open);

    let unhealthy_metrics: Vec<_> = health_monitor
        .get_recent_metrics(100)
        .into_iter()
        .filter(|m| !m.is_healthy)
        .collect();
    assert!(!unhealthy_metrics.is_empty());

    if unhealthy_metrics.len() >= 2 {
        manager.force_error_state(MimiState::Degraded);
    }

    assert!(!circuit_breaker.allow_request());

    manager.transition_to(MimiState::Recovering).unwrap();

    sleep(Duration::from_millis(100)).await;

    let beatrice_recovery = HealthMetric {
        timestamp: Utc::now(),
        component_name: "beatrice".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 200.0,
        threshold: 5000.0,
        is_healthy: true,
    };
    health_monitor
        .record_metric(beatrice_recovery)
        .await
        .unwrap();

    let pandora_recovery = HealthMetric {
        timestamp: Utc::now(),
        component_name: "pandora".to_string(),
        metric_type: HealthMetricType::MemoryUsage,
        value: 60.0,
        threshold: 80.0,
        is_healthy: true,
    };
    health_monitor
        .record_metric(pandora_recovery)
        .await
        .unwrap();

    let echidna_recovery = HealthMetric {
        timestamp: Utc::now(),
        component_name: "echidna".to_string(),
        metric_type: HealthMetricType::Latency,
        value: 300.0,
        threshold: 5000.0,
        is_healthy: true,
    };
    health_monitor
        .record_metric(echidna_recovery)
        .await
        .unwrap();

    sleep(Duration::from_secs(3)).await;

    manager.transition_to(MimiState::Idle).unwrap();

    assert_eq!(manager.current_state(), MimiState::Idle);
}
