//! Requires Neo4j from docker compose and applied schema.
//!
//! Run: `docker compose up -d neo4j && ./scripts/neo4j-apply-schema.sh`
//! Then: `cargo test -p mimi-core --test pandora_neo4j_integration -- --ignored`

use std::time::{Duration, Instant};

use mimi_core::{
    checkpoint_mimi_state, restore_mimi_state, ContextRelation, MimiCheckpoint, Neo4jConfig,
    PandoraClient, StateManager,
};
use mimi_core::state_machine::MimiState;

#[tokio::test]
#[ignore = "requires running Neo4j at NEO4J_URI"]
async fn neo4j_touch_and_hot_contexts_roundtrip() {
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .expect("client");

    assert!(
        client.is_neo4j_connected(),
        "expected live Neo4j; set NEO4J_URI/USER/PASSWORD"
    );

    let id = format!("ctx:it-{}", uuid::Uuid::new_v4());
    client
        .touch_context(&id, Some("integration test"))
        .await
        .expect("touch");

    let loaded = client.get_context(&id).await.expect("get");
    assert!(loaded.is_some(), "touched context should be retrievable");

    let flushed = client.flush_heatmap().await.expect("flush");
    assert!(flushed >= 1);
}

#[tokio::test]
#[ignore = "requires running Neo4j at NEO4J_URI"]
async fn neo4j_bfs_from_system_bootstrap() {
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .expect("client");

    assert!(client.is_neo4j_connected());

    let expanded = client
        .expand_context_bfs("ctx:system-bootstrap", 2, 20)
        .await
        .expect("bfs");

    assert!(
        expanded.iter().any(|n| n.id == "skill:echo" || n.id == "res:docs-quickstart"),
        "expected seed graph neighbors, got {:?}",
        expanded
    );
}

#[tokio::test]
#[ignore = "requires running Neo4j at NEO4J_URI"]
async fn neo4j_store_and_link_contexts() {
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .expect("client");
    assert!(client.is_neo4j_connected());

    let a = format!("ctx:store-a-{}", uuid::Uuid::new_v4());
    let b = format!("ctx:store-b-{}", uuid::Uuid::new_v4());
    client.store_context(&a, Some("A")).await.expect("store a");
    client.store_context(&b, Some("B")).await.expect("store b");
    client
        .link_contexts(&a, &b, ContextRelation::References)
        .await
        .expect("link");

    let expanded = client.expand_context_bfs(&a, 1, 10).await.expect("bfs");
    assert!(expanded.iter().any(|n| n.id == b));
}

#[tokio::test]
#[ignore = "requires running Neo4j at NEO4J_URI"]
async fn acceptance_neo4j_touch_latency_under_50ms() {
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .expect("client");
    assert!(client.is_neo4j_connected());

    let id = format!("ctx:lat-{}", uuid::Uuid::new_v4());
    for _ in 0..3 {
        client.touch_context(&id, Some("warmup")).await.expect("warmup");
    }

    let mut samples = Vec::new();
    for i in 0..10 {
        let tid = format!("{}-{}", id, i);
        let start = Instant::now();
        client.touch_context(&tid, Some("latency")).await.expect("touch");
        samples.push(start.elapsed());
    }

    samples.sort();
    let median = samples[samples.len() / 2];
    assert!(
        median < Duration::from_millis(50),
        "median touch latency {:?} exceeds 50ms (samples {:?})",
        median,
        samples
    );
}

#[tokio::test]
#[ignore = "requires running Neo4j at NEO4J_URI"]
async fn acceptance_mimi_checkpoint_neo4j_roundtrip() {
    let client = PandoraClient::with_config(Neo4jConfig::from_env())
        .await
        .expect("client");
    assert!(client.is_neo4j_connected());

    let state = StateManager::new();
    state.transition_to(MimiState::Listening).unwrap();
    state.transition_to(MimiState::Processing).unwrap();
    checkpoint_mimi_state(&client, &state).await.expect("save");

    let loaded: Option<MimiCheckpoint> = client.load_mimi_checkpoint().await.expect("load");
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().state, "Processing");

    let restored = StateManager::new();
    assert!(restore_mimi_state(&client, &restored).await.expect("apply"));
    assert_eq!(restored.current_state(), MimiState::Processing);
}
