//! Mimi State Machine FSM
//!
//! Implements the 10-state finite state machine for Mimi orchestrator core lifecycle.
//! Provides async execution, guard conditions, error recovery, and message bus integration.

use std::sync::{Arc, Mutex};

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
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}
