//! Pandora Neo4j integration with in-memory heatmap (M2.2.3).

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use neo4rs::{Graph, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MIMI_CHECKPOINT_METADATA_ID: &str = "meta:mimi-checkpoint";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MimiCheckpoint {
    pub version: u32,
    pub state: String,
    pub queue_size: usize,
    pub saved_at: DateTime<Utc>,
}
use tracing::{debug, info, warn};

use crate::heatmap::{HeatmapManager, INITIAL_TEMPERATURE};
use crate::l1_cache::{CachedContext, L1CacheStats, L1ContextCache, DEFAULT_L1_CAPACITY};
use crate::state_machine::MimiState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    pub uri: String,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: "bolt://localhost:7687".to_string(),
            username: "neo4j".to_string(),
            password: "mimi-password".to_string(),
            database: "neo4j".to_string(),
        }
    }
}

impl Neo4jConfig {
    pub fn from_env() -> Self {
        Self {
            uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| Self::default().uri),
            username: std::env::var("NEO4J_USER").unwrap_or_else(|_| Self::default().username),
            password: std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| Self::default().password),
            database: std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| Self::default().database),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHeatRecord {
    pub id: String,
    pub label: Option<String>,
    pub heat: f64,
    pub last_accessed_secs: u64,
    pub access_count: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContextRelation {
    References,
    Has,
    DependsOn,
}

impl ContextRelation {
    fn cypher_type(self) -> &'static str {
        match self {
            ContextRelation::References => "REFERENCES",
            ContextRelation::Has => "HAS",
            ContextRelation::DependsOn => "DEPENDS_ON",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextNeighbor {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub heat: f64,
    pub depth: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBundle {
    pub query_context_id: String,
    pub hot: Vec<ContextHeatRecord>,
    pub expanded: Vec<ContextNeighbor>,
    pub prompt_block: String,
}

enum PandoraBackend {
    Mock,
    Neo4j(Graph),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OdlagunaAuditRecord {
    pub id: String,
    pub task_id: String,
    pub action: String,
    pub detail: String,
    pub container_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

pub struct PandoraClient {
    config: Neo4jConfig,
    backend: PandoraBackend,
    heatmap: Arc<Mutex<HeatmapManager>>,
    l1_cache: Arc<Mutex<L1ContextCache>>,
    checkpoint_store: Arc<Mutex<Option<MimiCheckpoint>>>,
    odlaguna_audit: Arc<Mutex<Vec<OdlagunaAuditRecord>>>,
}

impl PandoraClient {
    pub async fn new() -> Result<Self> {
        Self::with_config(Neo4jConfig::from_env()).await
    }

    pub async fn with_config(config: Neo4jConfig) -> Result<Self> {
        let heatmap = Arc::new(Mutex::new(HeatmapManager::default()));
        let l1_cache = Arc::new(Mutex::new(L1ContextCache::new(DEFAULT_L1_CAPACITY)));
        let checkpoint_store = Arc::new(Mutex::new(None));
        let odlaguna_audit = Arc::new(Mutex::new(Vec::new()));

        let backend = match Graph::new(&config.uri, &config.username, &config.password).await {
            Ok(graph) => {
                let ping = graph
                    .run(Query::new("RETURN 1 AS ok".to_string()))
                    .await;
                if ping.is_ok() {
                    info!("Pandora connected to Neo4j at {}", config.uri);
                    PandoraBackend::Neo4j(graph)
                } else {
                    warn!(
                        "Pandora Neo4j ping failed at {}; using in-memory heatmap only",
                        config.uri
                    );
                    PandoraBackend::Mock
                }
            },
            Err(err) => {
                warn!(
                    "Pandora Neo4j unavailable at {} ({}); using in-memory heatmap only",
                    config.uri,
                    err
                );
                PandoraBackend::Mock
            },
        };

        Ok(Self {
            config,
            backend,
            heatmap,
            l1_cache,
            checkpoint_store,
            odlaguna_audit,
        })
    }

    pub async fn odlaguna_audit_log(&self) -> Vec<OdlagunaAuditRecord> {
        self.odlaguna_audit.lock().await.clone()
    }

    /// Persist Odlaguna intervention (M3.2.6) — in-memory + Neo4j when connected.
    pub async fn record_odlaguna_intervention(
        &self,
        task_id: &str,
        action: &str,
        detail: &str,
        container_id: Option<&str>,
    ) -> Result<()> {
        let record = OdlagunaAuditRecord {
            id: format!(
                "odl:{}:{}:{}",
                task_id,
                action,
                Utc::now().timestamp_millis()
            ),
            task_id: task_id.to_string(),
            action: action.to_string(),
            detail: detail.to_string(),
            container_id: container_id.map(|s| s.to_string()),
            recorded_at: Utc::now(),
        };

        self.odlaguna_audit.lock().await.push(record.clone());

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "MERGE (a:OdlagunaAudit {id: $id}) \
                 SET a.task_id = $task_id, \
                     a.action = $action, \
                     a.detail = $detail, \
                     a.container_id = $container_id, \
                     a.recorded_at = datetime({epochSeconds: $recorded_at})"
                    .to_string(),
            )
            .param("id", record.id.clone())
            .param("task_id", record.task_id.clone())
            .param("action", record.action.clone())
            .param("detail", record.detail.clone())
            .param(
                "container_id",
                record.container_id.clone().unwrap_or_default(),
            )
            .param("recorded_at", record.recorded_at.timestamp());

            graph.run(query).await.context("odlaguna audit write")?;
        }

        Ok(())
    }

    pub fn heatmap_handle(&self) -> Arc<Mutex<HeatmapManager>> {
        self.heatmap.clone()
    }

    pub async fn l1_cache_stats(&self) -> L1CacheStats {
        self.l1_cache.lock().await.stats().clone()
    }

    pub fn is_neo4j_connected(&self) -> bool {
        matches!(self.backend, PandoraBackend::Neo4j(_))
    }

    pub fn neo4j_uri(&self) -> &str {
        &self.config.uri
    }

    /// Record access in L1 heatmap and persist to Neo4j when connected.
    pub async fn touch_context(&self, id: &str, label: Option<&str>) -> Result<()> {
        let now_secs = Utc::now().timestamp() as u64;
        {
            let mut heatmap = self.heatmap.lock().await;
            heatmap.touch(id, now_secs);
        }

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            self.neo4j_touch_context(graph, id, label, now_secs, INITIAL_TEMPERATURE)
                .await?;
        }

        self.l1_put_record(id, label, now_secs, INITIAL_TEMPERATURE, 1, String::new())
            .await;

        debug!("Touched context {}", id);
        Ok(())
    }

    /// Persist or update a context node (M2.4.3).
    pub async fn store_context(&self, id: &str, label: Option<&str>) -> Result<()> {
        let now_secs = Utc::now().timestamp() as u64;
        {
            let mut heatmap = self.heatmap.lock().await;
            heatmap.touch(id, now_secs);
        }

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            self.neo4j_upsert_context(
                graph,
                id,
                label,
                now_secs,
                INITIAL_TEMPERATURE,
                1,
            )
            .await?;
        }

        self.l1_put_record(id, label, now_secs, INITIAL_TEMPERATURE, 1, String::new())
            .await;
        Ok(())
    }

    /// Link two context nodes with a typed relationship (M2.4.5).
    pub async fn link_contexts(
        &self,
        from_id: &str,
        to_id: &str,
        relation: ContextRelation,
    ) -> Result<()> {
        if let PandoraBackend::Neo4j(graph) = &self.backend {
            self.neo4j_link_contexts(graph, from_id, to_id, relation.cypher_type())
                .await?;
        }
        Ok(())
    }

    /// Register a skill node in the graph (M2.4.6).
    pub async fn register_skill(&self, id: &str, name: &str, status: &str) -> Result<()> {
        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "MERGE (s:Skill {id: $id}) SET s.name = $name, s.status = $status"
                    .to_string(),
            )
            .param("id", id)
            .param("name", name)
            .param("status", status);
            graph.run(query).await.context("register skill")?;
        }
        Ok(())
    }

    /// Fetch a single context (L1 → Neo4j → heatmap).
    pub async fn get_context(&self, id: &str) -> Result<Option<ContextHeatRecord>> {
        let now_secs = Utc::now().timestamp() as u64;

        {
            let mut cache = self.l1_cache.lock().await;
            if let Some(cached) = cache.get(id, now_secs) {
                return Ok(Some(cached_to_record(cached)));
            }
        }

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            if let Some(record) = self.neo4j_get_context(graph, id).await? {
                self.l1_put_from_record(&record, String::new()).await;
                return Ok(Some(record));
            }
        }

        let heatmap = self.heatmap.lock().await;
        let heat = heatmap.temperature_at(id, now_secs);
        if heat > 0.0 {
            let record = ContextHeatRecord {
                id: id.to_string(),
                label: None,
                heat,
                last_accessed_secs: now_secs,
                access_count: 1,
            };
            self.l1_put_from_record(&record, String::new()).await;
            return Ok(Some(record));
        }

        Ok(None)
    }

    /// Push all in-memory heatmap nodes to Neo4j (batch persistence).
    pub async fn flush_heatmap(&self) -> Result<u32> {
        let now_secs = Utc::now().timestamp() as u64;
        let snapshot = {
            let heatmap = self.heatmap.lock().await;
            heatmap.snapshot_for_persist(now_secs)
        };

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            for node in &snapshot {
                self.neo4j_upsert_context(
                    graph,
                    &node.id,
                    None,
                    node.last_accessed_secs,
                    node.temperature,
                    node.access_count,
                )
                .await?;
            }
        }

        Ok(snapshot.len() as u32)
    }

    /// Hot contexts: L1 heatmap first; enriches from Neo4j when connected.
    pub async fn hot_contexts(&self, limit: usize) -> Result<Vec<ContextHeatRecord>> {
        let now_secs = Utc::now().timestamp() as u64;
        let local = {
            let heatmap = self.heatmap.lock().await;
            heatmap.hot_nodes(now_secs)
        };

        if !self.is_neo4j_connected() {
            return Ok(local
                .into_iter()
                .take(limit)
                .map(|n| ContextHeatRecord {
                    id: n.id,
                    label: None,
                    heat: n.temperature,
                    last_accessed_secs: n.last_accessed_secs,
                    access_count: n.access_count,
                })
                .collect());
        }

        let graph = match &self.backend {
            PandoraBackend::Neo4j(g) => g,
            PandoraBackend::Mock => unreachable!(),
        };

        let remote = self.neo4j_load_contexts(graph).await?;
        let merged = merge_hot_contexts(local, remote, now_secs, limit);
        for record in &merged {
            self.l1_put_from_record(record, String::new()).await;
        }
        Ok(merged)
    }

    /// Hot nodes above `min_heat`, sorted by heat descending.
    pub async fn get_hot_nodes(&self, min_heat: f64, limit: usize) -> Result<Vec<ContextHeatRecord>> {
        let hot = self.hot_contexts(limit).await?;
        Ok(hot
            .into_iter()
            .filter(|n| n.heat >= min_heat)
            .collect())
    }

    pub async fn get_by_temperature_range(
        &self,
        min_heat: f64,
        max_heat: f64,
        limit: usize,
    ) -> Result<Vec<ContextHeatRecord>> {
        let hot = self.hot_contexts(limit.saturating_mul(2)).await?;
        Ok(hot
            .into_iter()
            .filter(|n| n.heat >= min_heat && n.heat <= max_heat)
            .take(limit)
            .collect())
    }

    /// BFS expansion from a context root across REFERENCES|HAS|DEPENDS_ON edges.
    pub async fn expand_context_bfs(
        &self,
        root_id: &str,
        max_hops: u32,
        limit: usize,
    ) -> Result<Vec<ContextNeighbor>> {
        if let PandoraBackend::Neo4j(graph) = &self.backend {
            return self.neo4j_expand_context(graph, root_id, max_hops, limit).await;
        }
        Ok(vec![])
    }

    pub async fn search_contexts(&self, needle: &str, limit: usize) -> Result<Vec<ContextHeatRecord>> {
        if let PandoraBackend::Neo4j(graph) = &self.backend {
            return self.neo4j_search_contexts(graph, needle, limit).await;
        }

        let now_secs = Utc::now().timestamp() as u64;
        let needle_lower = needle.to_lowercase();
        let heatmap = self.heatmap.lock().await;
        Ok(heatmap
            .hot_nodes(now_secs)
            .into_iter()
            .filter(|n| n.id.to_lowercase().contains(&needle_lower))
            .take(limit)
            .map(|n| ContextHeatRecord {
                id: n.id,
                label: None,
                heat: n.temperature,
                last_accessed_secs: n.last_accessed_secs,
                access_count: n.access_count,
            })
            .collect())
    }

    /// Build hot + BFS context for LLM `system_context` injection.
    pub async fn resolve_context_for_query(
        &self,
        query: &str,
        limit: usize,
        max_hops: u32,
    ) -> Result<ContextBundle> {
        use crate::context_query::query_context_id;

        let ctx_id = query_context_id(query);
        self.touch_context(&ctx_id, Some(query)).await?;

        let hot = self
            .get_hot_nodes(crate::heatmap::DEFAULT_THRESHOLD, limit)
            .await?;
        let search = self.search_contexts(query, 5).await?;

        let root = pick_root_context(query, &hot)
            .or_else(|| search.first().map(|n| n.id.clone()))
            .or_else(|| Some("ctx:system-bootstrap".to_string()));

        let expanded = if let Some(root_id) = root {
            self.expand_context_bfs(&root_id, max_hops, limit).await?
        } else {
            vec![]
        };

        let prompt_block = format_context_prompt(query, &hot, &expanded);
        Ok(ContextBundle {
            query_context_id: ctx_id,
            hot,
            expanded,
            prompt_block,
        })
    }

    /// L1-only context when Pandora/Neo4j is unavailable or circuit is OPEN (M3.3.2).
    pub async fn resolve_context_l1_fallback(
        &self,
        query: &str,
        limit: usize,
    ) -> Option<ContextBundle> {
        use crate::context_query::query_context_id;

        let ctx_id = query_context_id(query);
        let hot = {
            let cache = self.l1_cache.lock().await;
            cache
                .hot_snapshot(limit.max(1))
                .into_iter()
                .map(|c| cached_to_record(c))
                .collect::<Vec<_>>()
        };

        if hot.is_empty() {
            return None;
        }

        let prompt_block = format_context_prompt(query, &hot, &[]);
        Some(ContextBundle {
            query_context_id: ctx_id,
            hot,
            expanded: vec![],
            prompt_block,
        })
    }

    /// Periodic Mimi orchestrator checkpoint (M2 T2.4 / acceptance).
    pub async fn save_mimi_checkpoint(&self, state: &crate::state_machine::StateManager) -> Result<()> {
        let checkpoint = MimiCheckpoint {
            version: 1,
            state: format!("{:?}", state.current_state()),
            queue_size: state.queue_size(),
            saved_at: Utc::now(),
        };
        let json = serde_json::to_string(&checkpoint).context("serialize checkpoint")?;

        {
            let mut store = self.checkpoint_store.lock().await;
            *store = Some(checkpoint.clone());
        }

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "MERGE (m:Metadata {id: $id}) \
                 SET m.kind = 'mimi_checkpoint', \
                     m.snapshot = $snapshot, \
                     m.updated_at = datetime()"
                    .to_string(),
            )
            .param("id", MIMI_CHECKPOINT_METADATA_ID)
            .param("snapshot", json);

            graph.run(query).await.context("save mimi checkpoint")?;
        }

        debug!("Saved Mimi checkpoint: {:?}", checkpoint.state);
        Ok(())
    }

    pub async fn load_mimi_checkpoint(&self) -> Result<Option<MimiCheckpoint>> {
        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "MATCH (m:Metadata {id: $id}) \
                 RETURN m.snapshot AS snapshot LIMIT 1"
                    .to_string(),
            )
            .param("id", MIMI_CHECKPOINT_METADATA_ID);

            let mut rows = graph.execute(query).await.context("load checkpoint")?;
            if let Some(row) = rows.next().await? {
                let snapshot = row.get::<String>("snapshot").unwrap_or_default();
                if let Ok(cp) = serde_json::from_str::<MimiCheckpoint>(&snapshot) {
                    let mut store = self.checkpoint_store.lock().await;
                    *store = Some(cp.clone());
                    return Ok(Some(cp));
                }
            }
        }

        let store = self.checkpoint_store.lock().await;
        Ok(store.clone())
    }

    pub async fn apply_mimi_checkpoint(
        &self,
        state: &crate::state_machine::StateManager,
    ) -> Result<bool> {
        let Some(cp) = self.load_mimi_checkpoint().await? else {
            return Ok(false);
        };
        if let Some(parsed) = parse_mimi_state(&cp.state) {
            state.restore_state(parsed);
            return Ok(true);
        }
        warn!("Unknown checkpoint state string: {}", cp.state);
        Ok(false)
    }

    pub async fn persist_critical_state(
        &self,
        state: MimiState,
        timestamp: DateTime<Utc>,
        metadata: serde_json::Value,
    ) -> Result<String> {
        let node_id = uuid::Uuid::new_v4().to_string();

        debug!(
            "Persisting critical state: {:?} at {} -> {}",
            state,
            timestamp,
            node_id
        );

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "CREATE (s:StateSnapshot {id: $id, state: $state, at: datetime($at), metadata: $metadata})"
                    .to_string(),
            )
            .param("id", node_id.clone())
            .param("state", format!("{:?}", state))
            .param("at", timestamp.to_rfc3339())
            .param("metadata", metadata.to_string());

            graph
                .run(query)
                .await
                .context("failed to persist state snapshot")?;
        }

        Ok(node_id)
    }

    pub async fn query_state_history(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        state_filter: Option<MimiState>,
    ) -> Result<Vec<StateHistoryRecord>> {
        debug!(
            "Querying state history: {} to {} filter={:?}",
            from,
            to,
            state_filter
        );

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let mut query = Query::new(
                "MATCH (s:StateSnapshot) WHERE s.at >= datetime($from) AND s.at <= datetime($to) RETURN s.id AS id, s.state AS state, s.at AS at, s.metadata AS metadata ORDER BY s.at DESC LIMIT 100"
                    .to_string(),
            )
            .param("from", from.to_rfc3339())
            .param("to", to.to_rfc3339());

            if let Some(filter) = state_filter {
                query = Query::new(
                    "MATCH (s:StateSnapshot) WHERE s.at >= datetime($from) AND s.at <= datetime($to) AND s.state = $state RETURN s.id AS id, s.state AS state, s.at AS at, s.metadata AS metadata ORDER BY s.at DESC LIMIT 100"
                        .to_string(),
                )
                .param("from", from.to_rfc3339())
                .param("to", to.to_rfc3339())
                .param("state", format!("{:?}", filter));
            }

            let mut rows = graph.execute(query).await.context("state history query")?;
            let mut records = Vec::new();

            while let Some(row) = rows.next().await? {
                let node_id = row_get_string(&row, "id");
                let state = row_get_string(&row, "state");
                let at = row_get_string(&row, "at");
                let metadata_raw = row_get_string(&row, "metadata");
                let metadata_raw = if metadata_raw.is_empty() {
                    "{}".to_string()
                } else {
                    metadata_raw
                };
                let metadata = serde_json::from_str(&metadata_raw).unwrap_or(serde_json::json!({}));

                records.push(StateHistoryRecord {
                    node_id,
                    state,
                    timestamp: DateTime::parse_from_rfc3339(&at)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    metadata,
                });
            }

            return Ok(records);
        }

        Ok(vec![])
    }

    pub async fn query_failure_patterns(&self, window_hours: u32) -> Result<Vec<FailurePattern>> {
        debug!("Querying failure patterns: window={}h", window_hours);
        let window = window_hours.max(1);
        let cutoff = Utc::now() - chrono::Duration::hours(window as i64);

        let mut patterns = patterns_from_odlaguna_audit(&self.odlaguna_audit.lock().await, cutoff);

        if let PandoraBackend::Neo4j(graph) = &self.backend {
            let query = Query::new(
                "MATCH (a:OdlagunaAudit) \
                 WHERE a.recorded_at > datetime() - duration({hours: $window}) \
                 RETURN a.action AS action, count(*) AS frequency, max(a.recorded_at) AS last_at"
                    .to_string(),
            )
            .param("window", window as i64);

            if let Ok(mut rows) = graph.execute(query).await {
                while let Ok(Some(row)) = rows.next().await {
                    let action = row.get::<String>("action").unwrap_or_else(|_| "unknown".into());
                    let frequency = row.get::<i64>("frequency").unwrap_or(0) as u32;
                    let last_at = row
                        .get::<chrono::DateTime<chrono::FixedOffset>>("last_at")
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    merge_failure_pattern(&mut patterns, action, frequency, last_at);
                }
            }

            let ctx_query = Query::new(
                "MATCH (c:Context) \
                 WHERE c.label CONTAINS 'fail' AND c.last_accessed > datetime() - duration({hours: $window}) \
                 RETURN c.label AS label, coalesce(c.access_count, 1) AS frequency, c.last_accessed AS last_at \
                 LIMIT 20"
                    .to_string(),
            )
            .param("window", window as i64);

            if let Ok(mut rows) = graph.execute(ctx_query).await {
                while let Ok(Some(row)) = rows.next().await {
                    let label = row.get::<String>("label").unwrap_or_default();
                    let frequency = row.get::<i64>("frequency").unwrap_or(1) as u32;
                    let last_at = row
                        .get::<chrono::DateTime<chrono::FixedOffset>>("last_at")
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());
                    merge_failure_pattern(&mut patterns, label, frequency, last_at);
                }
            }
        }

        patterns.sort_by(|a, b| b.last_occurrence.cmp(&a.last_occurrence));
        Ok(patterns)
    }

    pub async fn close(self) -> Result<()> {
        info!("Closing Pandora client");
        Ok(())
    }

    async fn neo4j_touch_context(
        &self,
        graph: &Graph,
        id: &str,
        label: Option<&str>,
        last_accessed_secs: u64,
        heat: f64,
    ) -> Result<()> {
        let label_value = label.unwrap_or(id);
        let query = Query::new(
            "MERGE (c:Context {id: $id}) \
             SET c.label = coalesce($label, c.label, $id), \
                 c.heat = $heat, \
                 c.last_accessed = datetime({epochSeconds: $last_accessed}), \
                 c.access_count = coalesce(c.access_count, 0) + 1, \
                 c.content_hash = coalesce(c.content_hash, $content_hash)"
                .to_string(),
        )
        .param("id", id)
        .param("label", label_value)
        .param("heat", heat)
        .param("last_accessed", last_accessed_secs as i64)
        .param("content_hash", format!("heatmap:{}", id));

        graph.run(query).await.context("context touch upsert")?;
        Ok(())
    }

    async fn neo4j_upsert_context(
        &self,
        graph: &Graph,
        id: &str,
        label: Option<&str>,
        last_accessed_secs: u64,
        heat: f64,
        access_count: u64,
    ) -> Result<()> {
        let label_value = label.unwrap_or(id);
        let query = Query::new(
            "MERGE (c:Context {id: $id}) \
             SET c.label = coalesce($label, c.label, $id), \
                 c.heat = $heat, \
                 c.last_accessed = datetime({epochSeconds: $last_accessed}), \
                 c.access_count = $access_count, \
                 c.content_hash = coalesce(c.content_hash, $content_hash)"
                .to_string(),
        )
        .param("id", id)
        .param("label", label_value)
        .param("heat", heat)
        .param("last_accessed", last_accessed_secs as i64)
        .param("access_count", access_count as i64)
        .param("content_hash", format!("heatmap:{}", id));

        graph.run(query).await.context("context upsert")?;
        Ok(())
    }

    async fn l1_put_from_record(&self, record: &ContextHeatRecord, data: String) {
        let cached = record_to_cached(record, data);
        let mut cache = self.l1_cache.lock().await;
        cache.put(cached);
    }

    async fn l1_put_record(
        &self,
        id: &str,
        label: Option<&str>,
        last_accessed_secs: u64,
        heat: f64,
        access_count: u64,
        data: String,
    ) {
        let record = ContextHeatRecord {
            id: id.to_string(),
            label: label.map(|s| s.to_string()),
            heat,
            last_accessed_secs,
            access_count,
        };
        self.l1_put_from_record(&record, data).await;
    }

    async fn neo4j_link_contexts(
        &self,
        graph: &Graph,
        from_id: &str,
        to_id: &str,
        rel_type: &str,
    ) -> Result<()> {
        let cypher = format!(
            "MATCH (a:Context {{id: $from_id}}) \
             MERGE (b:Context {{id: $to_id}}) \
             MERGE (a)-[r:{}]->(b)",
            rel_type
        );
        let query = Query::new(cypher)
            .param("from_id", from_id)
            .param("to_id", to_id);
        graph.run(query).await.context("link contexts")?;
        Ok(())
    }

    async fn neo4j_get_context(&self, graph: &Graph, id: &str) -> Result<Option<ContextHeatRecord>> {
        let query = Query::new(
            "MATCH (c:Context {id: $id}) RETURN c.id AS id, c.label AS label, coalesce(c.heat, 0.0) AS heat, c.last_accessed AS last_accessed, coalesce(c.access_count, 0) AS access_count LIMIT 1"
                .to_string(),
        )
        .param("id", id);

        let mut rows = graph.execute(query).await.context("get context")?;
        if let Some(row) = rows.next().await? {
            return Ok(Some(ContextHeatRecord {
                id: row_get_string(&row, "id"),
                label: row.get::<String>("label").ok(),
                heat: row.get::<f64>("heat").unwrap_or(0.0),
                last_accessed_secs: parse_neo4j_datetime_secs(&row)?,
                access_count: row.get::<i64>("access_count").unwrap_or(0) as u64,
            }));
        }
        Ok(None)
    }

    async fn neo4j_load_contexts(&self, graph: &Graph) -> Result<Vec<ContextHeatRecord>> {
        let query = Query::new(
            "MATCH (c:Context) WHERE c.heat IS NOT NULL RETURN c.id AS id, c.label AS label, c.heat AS heat, c.last_accessed AS last_accessed, coalesce(c.access_count, 0) AS access_count LIMIT 500"
                .to_string(),
        );

        let mut rows = graph.execute(query).await.context("load contexts")?;
        let mut out = Vec::new();

        while let Some(row) = rows.next().await? {
            let id = row_get_string(&row, "id");
            let label = row.get::<String>("label").ok();
            let stored_heat = row.get::<f64>("heat").unwrap_or(0.0);
            let access_count = row.get::<i64>("access_count").unwrap_or(0) as u64;
            let last_accessed_secs = parse_neo4j_datetime_secs(&row)?;

            out.push(ContextHeatRecord {
                id,
                label,
                heat: stored_heat,
                last_accessed_secs,
                access_count,
            });
        }

        Ok(out)
    }

    async fn neo4j_search_contexts(
        &self,
        graph: &Graph,
        needle: &str,
        limit: usize,
    ) -> Result<Vec<ContextHeatRecord>> {
        let query = Query::new(
            "MATCH (c:Context) \
             WHERE toLower(c.label) CONTAINS toLower($needle) OR toLower(c.id) CONTAINS toLower($needle) \
             RETURN c.id AS id, c.label AS label, coalesce(c.heat, 0.0) AS heat, c.last_accessed AS last_accessed, coalesce(c.access_count, 0) AS access_count \
             ORDER BY heat DESC LIMIT $limit"
                .to_string(),
        )
        .param("needle", needle)
        .param("limit", limit as i64);

        let mut rows = graph.execute(query).await.context("search contexts")?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(ContextHeatRecord {
                id: row_get_string(&row, "id"),
                label: row.get::<String>("label").ok(),
                heat: row.get::<f64>("heat").unwrap_or(0.0),
                last_accessed_secs: parse_neo4j_datetime_secs(&row)?,
                access_count: row.get::<i64>("access_count").unwrap_or(0) as u64,
            });
        }
        Ok(out)
    }

    async fn neo4j_expand_context(
        &self,
        graph: &Graph,
        root_id: &str,
        max_hops: u32,
        limit: usize,
    ) -> Result<Vec<ContextNeighbor>> {
        let hops = max_hops.clamp(1, 4);
        let cypher = format!(
            "MATCH (root:Context {{id: $root_id}}) \
             MATCH path = (root)-[:REFERENCES|HAS|DEPENDS_ON*1..{}]-(n) \
             WHERE n:Context OR n:Skill OR n:Resource \
             WITH DISTINCT n, min(length(path)) AS depth \
             RETURN coalesce(labels(n)[0], 'Node') AS kind, \
                    coalesce(n.id, n.name, '') AS id, \
                    coalesce(n.label, n.name, n.id, '') AS label, \
                    coalesce(n.heat, 0.0) AS heat, \
                    coalesce(n.uri, n.description, '') AS detail, \
                    depth \
             ORDER BY depth, heat DESC \
             LIMIT $limit",
            hops
        );

        let query = Query::new(cypher)
            .param("root_id", root_id)
            .param("limit", limit as i64);

        let mut rows = graph.execute(query).await.context("expand context bfs")?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let depth = row.get::<i64>("depth").unwrap_or(1) as u32;
            let detail = row_get_string(&row, "detail");
            out.push(ContextNeighbor {
                id: row_get_string(&row, "id"),
                kind: row_get_string(&row, "kind"),
                label: row_get_string(&row, "label"),
                heat: row.get::<f64>("heat").unwrap_or(0.0),
                depth,
                detail: if detail.is_empty() {
                    None
                } else {
                    Some(detail)
                },
            });
        }
        Ok(out)
    }
}

fn pick_root_context(query: &str, hot: &[ContextHeatRecord]) -> Option<String> {
    let needle = query.to_lowercase();
    let words: Vec<&str> = needle.split_whitespace().filter(|w| w.len() > 2).collect();

    let mut best: Option<(usize, String)> = None;
    for node in hot {
        let label = node.label.as_deref().unwrap_or(&node.id).to_lowercase();
        let score = words.iter().filter(|w| label.contains(*w)).count();
        if score > 0 {
            match &best {
                Some((best_score, _)) if score <= *best_score => {},
                _ => best = Some((score, node.id.clone())),
            }
        }
    }

    if let Some((_, id)) = best {
        return Some(id);
    }

    hot.first().map(|n| n.id.clone())
}

fn record_to_cached(record: &ContextHeatRecord, data: String) -> CachedContext {
    CachedContext {
        id: record.id.clone(),
        label: record.label.clone(),
        heat: record.heat,
        last_accessed_secs: record.last_accessed_secs,
        access_count: record.access_count,
        data,
    }
}

fn cached_to_record(cached: CachedContext) -> ContextHeatRecord {
    ContextHeatRecord {
        id: cached.id,
        label: cached.label,
        heat: cached.heat,
        last_accessed_secs: cached.last_accessed_secs,
        access_count: cached.access_count,
    }
}

fn parse_mimi_state(raw: &str) -> Option<MimiState> {
    use MimiState::*;
    match raw {
        "Idle" => Some(Idle),
        "Listening" => Some(Listening),
        "Processing" => Some(Processing),
        "Executing" => Some(Executing),
        "Responding" => Some(Responding),
        "Degraded" => Some(Degraded),
        "Recovering" => Some(Recovering),
        "FailedComponent" => Some(FailedComponent),
        "CriticalError" => Some(CriticalError),
        "Shutdown" => Some(Shutdown),
        _ => None,
    }
}

fn format_context_prompt(
    user_query: &str,
    hot: &[ContextHeatRecord],
    expanded: &[ContextNeighbor],
) -> String {
    let mut lines = Vec::new();
    lines.push(
        "You are answering about **MiMi** (Modular Integrated Memory Instance), this repository's cognitive OS — not other products named MiMi.".to_string(),
    );
    lines.push(
        "MiMi memory stack: Pandora (Neo4j graph + heatmap exponential decay on Context nodes), Redis/Zenoh in docker-compose, Beatrice CLI → Ollama Cloud.".to_string(),
    );
    lines.push(format!("User question: {}", user_query));
    lines.push(String::new());
    lines.push("## Hot memory contexts".to_string());
    for node in hot {
        let label = node.label.as_deref().unwrap_or(&node.id);
        lines.push(format!(
            "- {} (heat={:.3}, accesses={})",
            label,
            node.heat,
            node.access_count
        ));
    }

    if !expanded.is_empty() {
        lines.push(String::new());
        lines.push("## Related graph nodes (BFS)".to_string());
        for n in expanded {
            let detail = n
                .detail
                .as_deref()
                .filter(|d| !d.is_empty())
                .map(|d| format!(" — {}", d))
                .unwrap_or_default();
            lines.push(format!(
                "- [{} depth={}] {}{} (heat={:.3})",
                n.kind,
                n.depth,
                n.label,
                detail,
                n.heat
            ));
        }
    }

    lines.join("\n")
}

fn row_get_string(row: &neo4rs::Row, key: &str) -> String {
    row.get::<String>(key).unwrap_or_default()
}

fn parse_neo4j_datetime_secs(row: &neo4rs::Row) -> Result<u64> {
    if let Ok(secs) = row.get::<i64>("last_accessed") {
        return Ok(secs as u64);
    }
    if let Ok(text) = row.get::<String>("last_accessed") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&text) {
            return Ok(dt.timestamp() as u64);
        }
    }
    Ok(Utc::now().timestamp() as u64)
}

fn merge_hot_contexts(
    local: Vec<crate::heatmap::HeatmapNode>,
    remote: Vec<ContextHeatRecord>,
    now_secs: u64,
    limit: usize,
) -> Vec<ContextHeatRecord> {
    use crate::heatmap::{decayed_temperature, DEFAULT_LAMBDA};

    let mut map: std::collections::HashMap<String, ContextHeatRecord> = remote
        .into_iter()
        .map(|r| {
            let age = now_secs.saturating_sub(r.last_accessed_secs);
            let heat = decayed_temperature(r.heat, DEFAULT_LAMBDA, age);
            (
                r.id.clone(),
                ContextHeatRecord {
                    heat,
                    ..r
                },
            )
        })
        .collect();

    for node in local {
        let entry = map.entry(node.id.clone()).or_insert(ContextHeatRecord {
            id: node.id.clone(),
            label: None,
            heat: 0.0,
            last_accessed_secs: node.last_accessed_secs,
            access_count: node.access_count,
        });
        if node.temperature > entry.heat {
            entry.heat = node.temperature;
            entry.last_accessed_secs = node.last_accessed_secs;
            entry.access_count = node.access_count;
        }
    }

    let mut merged: Vec<ContextHeatRecord> = map.into_values().collect();
    merged.sort_by(|a, b| b.heat.partial_cmp(&a.heat).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistoryRecord {
    pub node_id: String,
    pub state: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

fn patterns_from_odlaguna_audit(
    audit: &[OdlagunaAuditRecord],
    cutoff: DateTime<Utc>,
) -> Vec<FailurePattern> {
    let mut patterns: Vec<FailurePattern> = Vec::new();
    for record in audit {
        if record.recorded_at < cutoff {
            continue;
        }
        if record.action != "timeout" && !record.detail.to_lowercase().contains("fail") {
            continue;
        }
        let states = record
            .container_id
            .as_ref()
            .map(|c| vec!["docker".to_string(), c.clone()])
            .unwrap_or_default();
        merge_failure_pattern(
            &mut patterns,
            record.action.clone(),
            1,
            record.recorded_at,
        );
        if let Some(last) = patterns.last_mut() {
            if !states.is_empty() && last.states_involved.is_empty() {
                last.states_involved = states;
            }
        }
    }
    patterns
}

fn merge_failure_pattern(
    patterns: &mut Vec<FailurePattern>,
    pattern_type: String,
    frequency: u32,
    last_occurrence: DateTime<Utc>,
) {
    if let Some(existing) = patterns.iter_mut().find(|p| p.pattern_type == pattern_type) {
        existing.frequency += frequency;
        if last_occurrence > existing.last_occurrence {
            existing.last_occurrence = last_occurrence;
        }
        return;
    }
    patterns.push(FailurePattern {
        pattern_type,
        frequency,
        last_occurrence,
        states_involved: Vec::new(),
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_type: String,
    pub frequency: u32,
    pub last_occurrence: DateTime<Utc>,
    pub states_involved: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_pandora_client() {
        let client = PandoraClient::new().await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_touch_context_in_memory() {
        let client = PandoraClient::with_config(Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "invalid".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        client
            .touch_context("ctx:test", Some("Test context"))
            .await
            .expect("touch");

        let hot = client.hot_contexts(10).await.expect("hot");
        assert_eq!(hot.len(), 1);
        assert_eq!(hot[0].id, "ctx:test");
        assert!(hot[0].heat >= 0.99);
    }

    #[tokio::test]
    async fn test_store_context_in_memory() {
        let client = PandoraClient::with_config(Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "invalid".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        client
            .store_context("ctx:stored", Some("Stored"))
            .await
            .expect("store");
        let ctx = client.get_context("ctx:stored").await.expect("get");
        assert!(ctx.is_some());
    }

    #[tokio::test]
    async fn test_get_hot_nodes_filters_threshold() {
        let client = PandoraClient::with_config(Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "invalid".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        client.touch_context("ctx:hot", None).await.unwrap();
        let hot = client.get_hot_nodes(0.5, 10).await.expect("hot");
        assert_eq!(hot.len(), 1);
    }

    async fn test_resolve_context_bundle_in_memory() {
        let client = PandoraClient::with_config(Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "invalid".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        let bundle = client
            .resolve_context_for_query("system status", 5, 2)
            .await
            .expect("bundle");
        assert!(!bundle.prompt_block.is_empty());
        assert!(!bundle.query_context_id.is_empty());
    }

    async fn test_flush_heatmap_without_neo4j() {
        let client = PandoraClient::with_config(Neo4jConfig {
            uri: "bolt://127.0.0.1:1".to_string(),
            username: "neo4j".to_string(),
            password: "invalid".to_string(),
            database: "neo4j".to_string(),
        })
        .await
        .expect("client");

        client.touch_context("ctx:a", None).await.unwrap();
        let count = client.flush_heatmap().await.expect("flush");
        assert_eq!(count, 1);
    }
}
