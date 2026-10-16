//! Circuit breaker helpers (M3.3).

use std::future::Future;

use anyhow::Result;
use tracing::warn;

use crate::pandora_client::{ContextBundle, PandoraClient};
use crate::state_machine::CircuitBreaker;

pub async fn with_circuit_breaker<T, F, Fut>(
    breaker: &CircuitBreaker,
    fallback: T,
    operation: F,
) -> T
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    if !breaker.allow_request() {
        breaker.record_fallback();
        return fallback;
    }
    match operation().await {
        Ok(value) => {
            breaker.record_success();
            value
        },
        Err(_) => {
            breaker.record_failure();
            fallback
        },
    }
}

/// Pandora context with L1 fallback when the circuit is OPEN or Neo4j fails (M3.3.2).
pub async fn pandora_context_with_fallback(
    breaker: &CircuitBreaker,
    pandora: &PandoraClient,
    query: &str,
    limit: usize,
    max_hops: u32,
) -> Option<ContextBundle> {
    if !breaker.allow_request() {
        breaker.record_fallback();
        warn!(target: "circuit.pandora", "circuit open — L1 context fallback");
        return pandora.resolve_context_l1_fallback(query, limit).await;
    }

    match pandora
        .resolve_context_for_query(query, limit, max_hops)
        .await
    {
        Ok(bundle) => {
            breaker.record_success();
            Some(bundle)
        },
        Err(err) => {
            breaker.record_failure();
            warn!(
                target: "circuit.pandora",
                error = %err,
                "resolve failed — L1 context fallback"
            );
            pandora.resolve_context_l1_fallback(query, limit).await
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn breaker_open_returns_fallback() {
        let breaker = CircuitBreaker::new(1, Duration::from_secs(30));
        breaker.record_failure();
        let out = with_circuit_breaker(&breaker, 42, || async {
            Ok::<_, anyhow::Error>(99)
        })
        .await;
        assert_eq!(out, 42);
    }

    #[tokio::test]
    async fn breaker_closed_returns_operation() {
        let breaker = CircuitBreaker::new(3, Duration::from_secs(30));
        let out = with_circuit_breaker(&breaker, 0, || async { Ok(7) }).await;
        assert_eq!(out, 7);
    }

    #[tokio::test]
    async fn pandora_open_uses_l1_when_populated() {
        let breaker = CircuitBreaker::new(1, Duration::from_secs(30));
        breaker.record_failure();
        let pandora = PandoraClient::with_config(crate::pandora_client::Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "x".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");
        pandora
            .store_context("ctx:test", Some("test"))
            .await
            .expect("store");
        let bundle = pandora_context_with_fallback(&breaker, &pandora, "test", 5, 1)
            .await
            .expect("l1 bundle");
        assert!(!bundle.hot.is_empty());
    }

    #[tokio::test]
    async fn breaker_open_records_fallback_metric() {
        let breaker = CircuitBreaker::new(1, Duration::from_secs(30));
        breaker.record_failure();
        let _ = with_circuit_breaker(&breaker, 0, || async { Ok(1) }).await;
        assert_eq!(breaker.snapshot().fallback_calls, 1);
    }
}
