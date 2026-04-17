//! Mimi State Machine FSM
//!
//! Implements the 10-state finite state machine for Mimi orchestrator core lifecycle.
//! Provides async execution, guard conditions, error recovery, and message bus integration.

use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// State manager with thread-safe access
pub struct StateManager {
    state: Arc<Mutex<MimiState>>,
}

impl StateManager {
    /// Create new state manager starting in Idle state
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MimiState::Idle)),
        }
    }

    /// Get current state
    pub fn current_state(&self) -> MimiState {
        *self.state.lock().unwrap()
    }

    /// Transition to new state with validation
    pub fn transition_to(&self, new_state: MimiState) -> Result<()> {
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
