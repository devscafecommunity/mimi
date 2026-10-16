//! Process-wide circuit breakers (M3.3.2).

use std::sync::OnceLock;
use std::time::Duration;

use crate::circuit_metrics::CircuitBreakerSnapshot;
use crate::state_machine::CircuitBreaker;

pub struct CircuitBreakerRegistry {
    pub pandora: CircuitBreaker,
    pub ai: CircuitBreaker,
}

static GLOBAL: OnceLock<CircuitBreakerRegistry> = OnceLock::new();

impl CircuitBreakerRegistry {
    pub fn new() -> Self {
        Self {
            pandora: CircuitBreaker::new(3, Duration::from_secs(30)),
            ai: CircuitBreaker::new(3, Duration::from_secs(60)),
        }
    }

    pub fn snapshots(&self) -> RegistrySnapshots {
        self.check_alerts();
        RegistrySnapshots {
            pandora: self.pandora.snapshot(),
            ai: self.ai.snapshot(),
        }
    }

    pub fn check_alerts(&self) {
        self.pandora.check_sustained_open_alert("pandora");
        self.ai.check_sustained_open_alert("ai");
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistrySnapshots {
    pub pandora: CircuitBreakerSnapshot,
    pub ai: CircuitBreakerSnapshot,
}

/// Shared breakers for CLI / daemon (survive across `exec` / `query` invocations).
pub fn circuit_breakers() -> &'static CircuitBreakerRegistry {
    GLOBAL.get_or_init(CircuitBreakerRegistry::new)
}
