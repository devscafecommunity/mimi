//! Circuit breaker metrics (M3.3.4).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::Serialize;
use tracing::error;

use crate::state_machine::CircuitState;

/// Log + count when a breaker stays OPEN longer than this (M3.3.4).
pub const SUSTAINED_OPEN_ALERT_MS: u64 = 30_000;

#[derive(Debug, Default)]
pub struct CircuitBreakerMetrics {
    successful_calls: AtomicU64,
    failed_calls: AtomicU64,
    fallback_calls: AtomicU64,
    opened_count: AtomicU64,
    half_open_probes: AtomicU64,
    total_open_ms: AtomicU64,
    sustained_open_alerts: AtomicU64,
    open_since: Mutex<Option<Instant>>,
    last_alert_opened_count: Mutex<Option<u64>>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CircuitBreakerSnapshot {
    pub state: String,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub fallback_calls: u64,
    pub opened_count: u64,
    pub half_open_probes: u64,
    pub total_open_ms: u64,
    pub currently_open_ms: u64,
    pub sustained_open_alerts: u64,
}

impl CircuitBreakerMetrics {
    pub fn record_success(&self) {
        self.successful_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failed_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_fallback(&self) {
        self.fallback_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_half_open_probe(&self) {
        self.half_open_probes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn on_state_transition(&self, from: CircuitState, to: CircuitState) {
        if to == CircuitState::Open && from != CircuitState::Open {
            self.opened_count.fetch_add(1, Ordering::Relaxed);
            *self.open_since.lock().unwrap() = Some(Instant::now());
        }
        if from == CircuitState::Open && to != CircuitState::Open {
            let elapsed = self
                .open_since
                .lock()
                .unwrap()
                .take()
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0);
            self.total_open_ms.fetch_add(elapsed, Ordering::Relaxed);
            *self.last_alert_opened_count.lock().unwrap() = None;
        }
        if to == CircuitState::Closed {
            *self.last_alert_opened_count.lock().unwrap() = None;
        }
    }

    /// Emit at most one alert per OPEN episode when duration exceeds threshold (M3.3.4).
    pub fn maybe_alert_sustained_open(
        &self,
        state: CircuitState,
        label: &str,
        threshold_ms: u64,
    ) -> bool {
        if state != CircuitState::Open {
            return false;
        }
        let snap = self.snapshot(state);
        if snap.currently_open_ms < threshold_ms {
            return false;
        }
        let mut guard = self.last_alert_opened_count.lock().unwrap();
        if *guard == Some(snap.opened_count) {
            return false;
        }
        *guard = Some(snap.opened_count);
        self.sustained_open_alerts.fetch_add(1, Ordering::Relaxed);
        error!(
            target: "circuit.alert",
            label = label,
            opened_count = snap.opened_count,
            currently_open_ms = snap.currently_open_ms,
            fallback_calls = snap.fallback_calls,
            "circuit breaker sustained OPEN"
        );
        true
    }

    pub fn snapshot(&self, state: CircuitState) -> CircuitBreakerSnapshot {
        let currently_open_ms = if state == CircuitState::Open {
            self.open_since
                .lock()
                .unwrap()
                .map(|t| t.elapsed().as_millis() as u64)
                .unwrap_or(0)
        } else {
            0
        };

        CircuitBreakerSnapshot {
            state: format!("{:?}", state),
            successful_calls: self.successful_calls.load(Ordering::Relaxed),
            failed_calls: self.failed_calls.load(Ordering::Relaxed),
            fallback_calls: self.fallback_calls.load(Ordering::Relaxed),
            opened_count: self.opened_count.load(Ordering::Relaxed),
            half_open_probes: self.half_open_probes.load(Ordering::Relaxed),
            total_open_ms: self.total_open_ms.load(Ordering::Relaxed),
            currently_open_ms,
            sustained_open_alerts: self.sustained_open_alerts.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn tracks_open_duration() {
        let m = CircuitBreakerMetrics::default();
        m.on_state_transition(CircuitState::Closed, CircuitState::Open);
        std::thread::sleep(Duration::from_millis(5));
        let snap = m.snapshot(CircuitState::Open);
        assert_eq!(snap.opened_count, 1);
        assert!(snap.currently_open_ms >= 5);
        m.on_state_transition(CircuitState::Open, CircuitState::HalfOpen);
        let closed = m.snapshot(CircuitState::HalfOpen);
        assert!(closed.total_open_ms >= 5);
    }
}
