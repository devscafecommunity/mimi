//! M2.6 — Mimi state machine + Pandora (offline Neo4j).

use mimi_core::{
    execute_next_task_with_pandora, execute_task_with_context, Neo4jConfig, PandoraClient,
    StateManager,
};
use mimi_core::state_machine::{Task, TaskType};

async fn offline_pandora() -> PandoraClient {
    PandoraClient::with_config(Neo4jConfig {
        uri: "bolt://127.0.0.1:1".to_string(),
        username: "neo4j".to_string(),
        password: "x".to_string(),
        database: "neo4j".to_string(),
    })
    .await
    .expect("client")
}

#[tokio::test]
async fn mimi_pandora_task_flow_offline() {
    let state = StateManager::new();
    let pandora = offline_pandora().await;

    let task = Task::new(TaskType::Execute, "check heatmap");
    state.enqueue_task(task.clone()).expect("enqueue");

    let summary = execute_next_task_with_pandora(&state, &pandora)
        .await
        .expect("execute");
    assert!(summary.success);
    assert!(summary.bundle.prompt_block.contains("heatmap")
        || summary.bundle.prompt_block.contains("MiMi"));
}

#[tokio::test]
async fn mimi_pandora_records_outcome_in_heatmap() {
    let state = StateManager::new();
    let pandora = offline_pandora().await;
    let task = Task::new(TaskType::Query, "pandora integration");

    let summary = execute_task_with_context(&state, &pandora, task)
        .await
        .expect("run");
    let hot = pandora.hot_contexts(20).await.expect("hot");
    assert!(hot.iter().any(|n| n.id.contains("task:") || n.id.contains("ctx:")));
    assert!(summary.success);
}
