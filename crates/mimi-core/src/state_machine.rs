//! Mimi State Machine FSM
//!
//! Implements the 10-state finite state machine for Mimi orchestrator core lifecycle.
//! Provides async execution, guard conditions, error recovery, and message bus integration.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

/// Mimi system states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MimiState {
    /// System idle, waiting for input
    Idle,
    /// Listening for user commands via Zenoh
    Listening,
    /// Processing intent classification
    Processing,
    /// Executing task via workers
    Executing,
    /// Generating response via Liliana
    Responding,
    /// Degraded mode (partial functionality)
    Degraded,
    /// Recovering from failure
    Recovering,
    /// Component failure detected
    FailedComponent,
    /// Critical error requiring intervention
    CriticalError,
    /// System shutdown in progress
    Shutdown,
}

/// Task wrapper for priority queue ordering
#[derive(Clone)]
struct PrioritizedTask {
    task: Task,
    sequence: u64,
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.priority == other.task.priority && self.sequence == other.sequence
    }
}

impl Eq for PrioritizedTask {}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.task.priority.cmp(&other.task.priority) {
            Ordering::Equal => other.sequence.cmp(&self.sequence),
            other_ord => other_ord,
        }
    }
}

/// State manager with thread-safe access
pub struct StateManager {
    state: Arc<Mutex<MimiState>>,
    component_health: Arc<Mutex<Option<ComponentHealthCheck>>>,
    task_queue: Arc<Mutex<BinaryHeap<PrioritizedTask>>>,
    queue_capacity: usize,
    sequence_counter: Arc<Mutex<u64>>,
}

impl StateManager {
    /// Create new state manager starting in Idle state
    pub fn new() -> Self {
        Self::with_capacity(1000)
    }

    /// Create state manager with custom queue capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(MimiState::Idle)),
            component_health: Arc::new(Mutex::new(None)),
            task_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            queue_capacity: capacity,
            sequence_counter: Arc::new(Mutex::new(0)),
        }
    }

    /// Get current state
    pub fn current_state(&self) -> MimiState {
        *self.state.lock().unwrap()
    }

    /// Enqueue task with priority ordering
    pub fn enqueue_task(&self, task: Task) -> Result<()> {
        let mut queue = self.task_queue.lock().unwrap();

        if queue.len() >= self.queue_capacity {
            return Err(anyhow!(
                "Task queue full (capacity: {})",
                self.queue_capacity
            ));
        }

        let mut counter = self.sequence_counter.lock().unwrap();
        let sequence = *counter;
        *counter += 1;

        queue.push(PrioritizedTask { task, sequence });

        Ok(())
    }

    /// Dequeue highest priority task (FIFO within priority)
    pub fn dequeue_task(&self) -> Result<Task> {
        let mut queue = self.task_queue.lock().unwrap();

        queue
            .pop()
            .map(|pt| pt.task)
            .ok_or_else(|| anyhow!("Task queue is empty"))
    }

    /// Get current queue size
    pub fn queue_size(&self) -> usize {
        self.task_queue.lock().unwrap().len()
    }

    /// Update component health and trigger escalation if needed
    pub fn update_component_health(&self, health: ComponentHealthCheck) -> Result<()> {
        let mut health_guard = self.component_health.lock().unwrap();
        *health_guard = Some(health);

        if health.needs_recovery() {
            drop(health_guard);
            self.force_error_state(MimiState::Recovering);
        } else if health.needs_degraded() {
            drop(health_guard);
            self.force_error_state(MimiState::Degraded);
        }

        Ok(())
    }

    /// Transition to new state with validation
    pub fn transition_to(&self, new_state: MimiState) -> Result<()> {
        let health_guard = self.component_health.lock().unwrap();

        if let Some(health) = *health_guard {
            if health.needs_recovery() {
                drop(health_guard);
                self.force_error_state(MimiState::Recovering);
                return Ok(());
            } else if health.needs_degraded() {
                drop(health_guard);
                self.force_error_state(MimiState::Degraded);
                return Ok(());
            }
        }
        drop(health_guard);

        let mut state = self.state.lock().unwrap();
        let current = *state;

        let transition = StateTransition::new(current, new_state);

        if !transition.is_valid() {
            return Err(anyhow!(
                "Invalid state transition: {:?} -> {:?}",
                current,
                new_state
            ));
        }

        log::info!("State transition: {:?} -> {:?}", current, new_state);
        *state = new_state;

        Ok(())
    }

    /// Check component health and transition if needed
    pub fn check_and_transition(
        &self,
        target_state: MimiState,
        health: &ComponentHealth,
    ) -> Result<()> {
        if !TransitionGuard::check_component_health(health) {
            log::warn!("Component health check failed, transitioning to Degraded");
            return self.transition_to(MimiState::Degraded);
        }

        self.transition_to(target_state)
    }

    /// Force transition to error state (bypasses validation)
    pub fn force_error_state(&self, error_state: MimiState) {
        let mut state = self.state.lock().unwrap();

        log::error!("Forcing error state: {:?}", error_state);
        *state = error_state;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Component health metrics for guard conditions
#[derive(Debug, Clone, Copy)]
pub struct ComponentHealth {
    pub latency_ms: u64,
    pub memory_usage_percent: u8,
    pub last_heartbeat_secs: u64,
}

/// Component health check with thresholds
#[derive(Debug, Clone, Copy)]
pub struct ComponentHealthCheck {
    latency_ms: u64,
    memory_percent: u8,
    heartbeat_age_secs: u64,
}

impl ComponentHealthCheck {
    const LATENCY_THRESHOLD_MS: u64 = 5000;
    const MEMORY_THRESHOLD_PERCENT: u8 = 80;
    const HEARTBEAT_THRESHOLD_SECS: u64 = 30;

    pub fn new(latency_ms: u64, memory_percent: u8, heartbeat_age_secs: u64) -> Self {
        Self {
            latency_ms,
            memory_percent,
            heartbeat_age_secs,
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.latency_ms <= Self::LATENCY_THRESHOLD_MS
            && self.memory_percent <= Self::MEMORY_THRESHOLD_PERCENT
            && self.heartbeat_age_secs <= Self::HEARTBEAT_THRESHOLD_SECS
    }

    pub fn needs_recovery(&self) -> bool {
        self.heartbeat_age_secs > Self::HEARTBEAT_THRESHOLD_SECS
    }

    pub fn needs_degraded(&self) -> bool {
        (self.latency_ms > Self::LATENCY_THRESHOLD_MS
            || self.memory_percent > Self::MEMORY_THRESHOLD_PERCENT)
            && self.heartbeat_age_secs <= Self::HEARTBEAT_THRESHOLD_SECS
    }
}

/// State transition representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateTransition {
    pub from: MimiState,
    pub to: MimiState,
}

impl StateTransition {
    /// Create new state transition
    pub fn new(from: MimiState, to: MimiState) -> Self {
        Self { from, to }
    }

    /// Check if transition is valid according to FSM rules
    pub fn is_valid(&self) -> bool {
        use MimiState::*;

        if self.from == self.to {
            return true;
        }

        matches!(
            (self.from, self.to),
            // Normal flow
            |(Idle, Listening)| (Listening, Processing)
            | (Processing, Executing)
            | (Executing, Responding)
            | (Responding, Idle)

            // Recovery paths
            | (Degraded, Recovering)
            | (FailedComponent, Recovering)
            | (Recovering, Idle)

            // Error escalation from any state
            | (_, Degraded)
            | (_, FailedComponent)
            | (_, CriticalError)

            // Shutdown from any state
            | (_, Shutdown)
        )
    }
}

/// Guard condition evaluator for state transitions
pub struct TransitionGuard;

impl TransitionGuard {
    /// Latency threshold: 5 seconds
    const LATENCY_THRESHOLD_MS: u64 = 5000;

    /// Memory usage threshold: 80%
    const MEMORY_THRESHOLD_PERCENT: u8 = 80;

    /// Heartbeat timeout: 30 seconds
    const HEARTBEAT_TIMEOUT_SECS: u64 = 30;

    /// Check if component health is within acceptable thresholds
    pub fn check_component_health(health: &ComponentHealth) -> bool {
        health.latency_ms <= Self::LATENCY_THRESHOLD_MS
            && health.memory_usage_percent <= Self::MEMORY_THRESHOLD_PERCENT
            && health.last_heartbeat_secs <= Self::HEARTBEAT_TIMEOUT_SECS
    }

    /// Check if task queue has capacity
    pub fn check_queue_capacity(current: usize, max: usize) -> bool {
        current < max
    }

    /// Check if task timeout is within bounds
    pub fn check_task_timeout(timeout: &Duration, max: &Duration) -> bool {
        timeout <= max
    }
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Task types matching IntentType from schema.fbs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    Query,
    Execute,
    SkillPublish,
    StateUpdate,
    MemoryUpdate,
    ErrorReport,
    Control,
}

/// Execution model for task processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionModel {
    /// Synchronous blocking execution (<500ms expected)
    Blocking,
    /// Asynchronous with callback (>500ms expected)
    Async,
}

/// Task representation with full lifecycle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub task_type: TaskType,
    pub name: String,
    pub priority: TaskPriority,
    pub payload: Vec<u8>,
    pub timeout: Duration,
    pub retries: u32,
    pub max_retries: u32,
    pub created_at: DateTime<Utc>,
    pub execution_model: ExecutionModel,
}

impl Task {
    /// Create new task with defaults
    pub fn new(task_type: TaskType, name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            name: name.to_string(),
            priority: TaskPriority::Normal,
            payload: Vec::new(),
            timeout: Duration::from_secs(30),
            retries: 0,
            max_retries: 3,
            created_at: Utc::now(),
            execution_model: ExecutionModel::Async,
        }
    }

    /// Set task priority (builder pattern)
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set timeout (builder pattern)
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set payload (builder pattern)
    pub fn with_payload(mut self, payload: Vec<u8>) -> Self {
        self.payload = payload;
        self
    }

    /// Set execution model (builder pattern)
    pub fn with_execution_model(mut self, model: ExecutionModel) -> Self {
        self.execution_model = model;
        self
    }

    /// Check if task can be retried
    pub fn can_retry(&self) -> bool {
        self.retries < self.max_retries
    }

    /// Increment retry counter
    pub fn increment_retry(&mut self) {
        self.retries += 1;
    }
}
