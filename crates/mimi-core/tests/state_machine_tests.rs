//! State Machine Unit Tests

use mimi_core::state_machine::{MimiState, StateManager};

#[test]
fn test_initial_state_is_idle() {
    // This will fail because StateManager doesn't exist yet
    let manager = StateManager::new();
    assert_eq!(manager.current_state(), MimiState::Idle);
}
