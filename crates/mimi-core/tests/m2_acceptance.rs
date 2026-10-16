//! M2 milestone acceptance helpers (offline).

use mimi_core::l1_cache::{L1ContextCache, M2_L1_HIT_RATE_TARGET};
use mimi_core::state_machine::{MimiState, StateManager};
use mimi_core::{checkpoint_mimi_state, restore_mimi_state, Neo4jConfig, PandoraClient};

#[tokio::test]
async fn m2_checkpoint_roundtrip_offline() {
    let pandora = PandoraClient::with_config(Neo4jConfig {
        uri: "bolt://127.0.0.1:1".to_string(),
        username: "neo4j".to_string(),
        password: "x".to_string(),
        database: "neo4j".to_string(),
    })
    .await
    .expect("client");

    let state = StateManager::new();
    state.transition_to(MimiState::Listening).unwrap();
    checkpoint_mimi_state(&pandora, &state).await.expect("save");

    let fresh = StateManager::new();
    assert_eq!(fresh.current_state(), MimiState::Idle);
    assert!(restore_mimi_state(&pandora, &fresh).await.expect("restore"));
    assert_eq!(fresh.current_state(), MimiState::Listening);
}

#[test]
fn m2_l1_hit_rate_constant_matches_tests() {
    let mut cache = L1ContextCache::new(50);
    for i in 0..3 {
        cache.put(mimi_core::CachedContext {
            id: format!("k{}", i),
            label: None,
            heat: 1.0,
            last_accessed_secs: 1,
            access_count: 1,
            data: String::new(),
        });
    }
    for _ in 0..30 {
        cache.get("k1", 2);
    }
    assert!(cache.stats().hit_rate() >= M2_L1_HIT_RATE_TARGET);
}
