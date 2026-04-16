# PANDORA — Memory Manager Module Design

**Version:** 1.0  
**Status:** Design Phase  
**Primary Language:** C++ with Neo4j Bolt Driver  
**Related Requirements:** [REQUIREMENTS.md#RF-3](../REQUIREMENTS.md)  
**Related Specs:** `specs/HEATMAP-ALGORITHM.md` (planned)

---

## 1. Module Overview

**Pandora** is the Memory Manager module responsible for managing MiMi's short-term and long-term memory through a graph-based architecture. It provides context-aware retrieval using a thermal decay algorithm (Heatmap) to ensure that frequently accessed and relevant information remains "hot" while stale data naturally cools down and gets filtered out.

### Core Responsibilities:
- **Neo4j Integration:** Persistent graph storage via Bolt protocol
- **Heatmap Engine:** Thermal decay algorithm for context relevance scoring
- **Context Retrieval:** BFS traversal with temperature filtering for optimal subgraph extraction
- **LRU Cache (L1):** In-memory cache for immediate context access
- **Query Optimization:** Cypher query generation and execution tuning
- **FFI/IPC Bridge:** Low-latency communication with Rust modules (Mimi, Beatrice)

### Why Graph-Based Memory?
Traditional RAG (Retrieval-Augmented Generation) systems suffer from context poisoning when irrelevant data is retrieved. Pandora's Heatmap approach dynamically filters context based on access patterns, reducing token waste and improving response quality.

---

## 2. Architecture

### Internal Components:

```
┌─────────────────────────────────────────────────┐
│              Pandora Memory Manager             │
├─────────────────────────────────────────────────┤
│                                                 │
│  ┌──────────────┐         ┌─────────────────┐  │
│  │  Query API   │◄────────┤  FFI/IPC Layer  │  │
│  │  (C++ Entry) │         │  (Unix Socket)  │  │
│  └──────┬───────┘         └─────────────────┘  │
│         │                                       │
│         ▼                                       │
│  ┌──────────────────────────────────────────┐  │
│  │        LRU Cache (L1)                    │  │
│  │  • ~1000 nodes in RAM                    │  │
│  │  • O(1) lookup, O(1) eviction            │  │
│  └──────┬───────────────────────────────────┘  │
│         │ Cache Miss                            │
│         ▼                                       │
│  ┌──────────────────────────────────────────┐  │
│  │      Query Optimizer & Generator         │  │
│  │  • Cypher query builder                  │  │
│  │  • Parameter binding                     │  │
│  │  • Query plan caching                    │  │
│  └──────┬───────────────────────────────────┘  │
│         │                                       │
│         ▼                                       │
│  ┌──────────────────────────────────────────┐  │
│  │        Bolt Driver (Neo4j C++)           │  │
│  │  • Connection pooling (5 connections)    │  │
│  │  • Transaction management (ACID)         │  │
│  │  • Retry logic with exponential backoff  │  │
│  └──────┬───────────────────────────────────┘  │
│         │                                       │
│         ▼                                       │
│  ┌──────────────────────────────────────────┐  │
│  │         Heatmap Engine                   │  │
│  │  • Temperature calculation               │  │
│  │  • Decay function: T(t) = T0*e^(-λΔt)   │  │
│  │  • BFS with thermal filtering            │  │
│  └──────────────────────────────────────────┘  │
│                                                 │
└─────────────────────────────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │    Neo4j DB    │
              │  (Graph Store) │
              └────────────────┘
```

### Thread Model:
- **Main Thread:** Query API and cache management
- **Bolt Driver Pool:** Dedicated connection pool (5 workers) for concurrent queries
- **Background Thread:** Periodic cache cleanup and temperature decay updates

### Memory Layout:
```cpp
struct CachedNode {
    std::string id;           // Neo4j node UUID
    nlohmann::json data;      // Node properties
    double temperature;       // Current heat value
    uint64_t last_accessed;   // Unix timestamp (ms)
    size_t access_count;      // Frequency counter
};

class LRUCache {
    std::unordered_map<std::string, std::list<CachedNode>::iterator> cache_map;
    std::list<CachedNode> lru_list;
    const size_t capacity = 1000;
};
```

---

## 3. API/Interfaces

### 3.1 FFI Interface (Rust ↔ C++)

**Entry Point:**
```cpp
// pandora.h
extern "C" {
    // Query context subgraph
    char* pandora_query_context(
        const char* intent_json,      // User intent with entities
        double temp_threshold,         // Minimum temperature (default: 0.1)
        int max_nodes                  // Max nodes in response (default: 500)
    );

    // Update node temperature (explicit access)
    void pandora_touch_node(const char* node_id);

    // Create new context node
    char* pandora_create_node(
        const char* node_type,         // ContextNode, Entity, Skill, etc.
        const char* properties_json
    );

    // Create relationship between nodes
    void pandora_create_edge(
        const char* from_id,
        const char* to_id,
        const char* relationship_type  // RELATES_TO, DEPENDS_ON, etc.
    );

    // Free memory allocated by Pandora
    void pandora_free_string(char* ptr);
}
```

**Rust Bridge Example:**
```rust
// mimi-commander/src/memory/pandora_ffi.rs
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn pandora_query_context(
        intent_json: *const c_char,
        temp_threshold: f64,
        max_nodes: i32
    ) -> *mut c_char;
    fn pandora_free_string(ptr: *mut c_char);
}

pub fn query_memory(intent: &Intent) -> Result<ContextGraph, PandoraError> {
    let intent_json = CString::new(serde_json::to_string(intent)?)?;
    unsafe {
        let result_ptr = pandora_query_context(intent_json.as_ptr(), 0.1, 500);
        if result_ptr.is_null() {
            return Err(PandoraError::QueryFailed);
        }
        let result = CStr::from_ptr(result_ptr).to_string_lossy().into_owned();
        pandora_free_string(result_ptr);
        Ok(serde_json::from_str(&result)?)
    }
}
```

### 3.2 Neo4j Bolt Protocol

**Connection Configuration:**
```cpp
// Connection URI: bolt://localhost:7687
// Auth: neo4j / password
// Max Pool Size: 5
// Connection Timeout: 10s
// Query Timeout: 30s
```

**Query Methods:**
```cpp
class BoltDriver {
public:
    // Execute read query
    neo4j::Result executeQuery(const std::string& cypher, 
                               const neo4j::Values& params);
    
    // Execute write transaction
    void executeTransaction(std::function<void(neo4j::Transaction&)> work);
    
    // Execute batch write (bulk insert)
    void executeBatch(const std::vector<std::string>& queries);
};
```

---

## 4. Key Algorithms

### 4.1 Heatmap Decay Formula

**Exponential Decay:**
```
T(t) = T₀ * e^(-λ * Δt)

Where:
- T(t) = current temperature
- T₀ = initial temperature (default: 1.0)
- λ = decay constant (default: 0.0001 per second)
- Δt = time elapsed since last access (seconds)
```

**Temperature Boost on Access:**
```
T_new = min(1.0, T_old + 0.3)
```

**C++ Implementation:**
```cpp
double calculateTemperature(const CachedNode& node) {
    auto now = std::chrono::system_clock::now();
    auto elapsed_ms = std::chrono::duration_cast<std::chrono::milliseconds>(
        now.time_since_epoch()).count() - node.last_accessed;
    
    double elapsed_sec = elapsed_ms / 1000.0;
    double lambda = 0.0001; // Decay rate
    return node.temperature * std::exp(-lambda * elapsed_sec);
}
```

### 4.2 BFS with Temperature Filtering

**Query Strategy:**
```cypher
// Start from intent entities
MATCH path = (start:Entity)-[*1..3]-(related)
WHERE start.id IN $entity_ids
  AND related.temperature > $threshold
WITH related, related.temperature AS temp
ORDER BY temp DESC
LIMIT $max_nodes
RETURN related, temp
```

**Cypher Pattern Expansion:**
- **Depth 1:** Direct relationships (e.g., `User → Task`)
- **Depth 2:** Second-degree connections (e.g., `User → Task → Skill`)
- **Depth 3:** Contextual environment (e.g., `User → Task → Skill → Documentation`)

**C++ Traversal Logic:**
```cpp
std::vector<Node> bfsWithHeat(const std::vector<std::string>& start_ids, 
                               double threshold, int max_nodes) {
    std::queue<std::string> queue;
    std::unordered_set<std::string> visited;
    std::vector<Node> result;
    
    for (const auto& id : start_ids) {
        queue.push(id);
    }
    
    while (!queue.empty() && result.size() < max_nodes) {
        auto current_id = queue.front();
        queue.pop();
        
        if (visited.count(current_id)) continue;
        visited.insert(current_id);
        
        auto node = fetchNodeWithTemp(current_id);
        if (node.temperature > threshold) {
            result.push_back(node);
            
            // Expand neighbors
            auto neighbors = getNeighbors(current_id);
            for (const auto& neighbor : neighbors) {
                queue.push(neighbor);
            }
        }
    }
    
    return result;
}
```

### 4.3 LRU Eviction Policy

**Eviction Trigger:**
- Cache size exceeds 1000 nodes
- Evict least recently used node with temperature < 0.05

**C++ Implementation:**
```cpp
void LRUCache::evictCold() {
    while (lru_list.size() > capacity) {
        // Remove from back (LRU)
        auto& oldest = lru_list.back();
        if (calculateTemperature(oldest) < 0.05) {
            cache_map.erase(oldest.id);
            lru_list.pop_back();
        } else {
            break; // All remaining nodes are hot
        }
    }
}
```

---

## 5. Dependencies

### 5.1 External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| **Neo4j** | 4.4+ or 5.x | Graph database backend |
| **neo4j-cpp-driver** | Latest | Bolt protocol client |
| **nlohmann/json** | 3.11+ | JSON parsing/serialization |
| **spdlog** | 1.x | Logging framework |
| **FlatBuffers** | 23.5+ | Message serialization (Bus protocol) |

### 5.2 Internal Module Dependencies

**Pandora depends on:**
- **Message Bus** (RF-7): Receives memory update commands
- **Mimi Commander** (RF-1): Provides context queries

**Modules that depend on Pandora:**
- **Mimi Commander** (RF-1): Queries context before generating responses
- **Echidna** (RF-4): Queries existing skills to detect repetition patterns
- **Odlaguna** (RF-6): Logs audit trail to Neo4j via Pandora

---

## 6. Data Structures

### 6.1 Neo4j Schema

**Node Types:**

```cypher
// Base node with thermal properties
(:ContextNode {
    id: STRING (UUID),           // Primary key
    type: STRING,                 // Subtype identifier
    temperature: FLOAT,           // Current heat value [0.0, 1.0]
    last_accessed: INT,           // Unix timestamp (ms)
    access_count: INT,            // Total access frequency
    created_at: INT,              // Creation timestamp
    metadata: MAP                 // Type-specific properties
})

// User intent/command
(:Intent :ContextNode {
    user_message: STRING,
    intent_type: STRING,          // "execute_task", "query_info", etc.
    confidence: FLOAT,
    entities: [STRING],           // Extracted entity IDs
    timestamp: INT
})

// Extracted entity (person, tool, concept)
(:Entity :ContextNode {
    name: STRING,
    entity_type: STRING,          // "person", "tool", "concept", "location"
    aliases: [STRING],
    description: TEXT
})

// Executable skill/tool
(:Skill :ContextNode {
    name: STRING,
    skill_type: STRING,           // "rhai_script", "wasm_binary"
    code_hash: STRING,            // SHA256 of source/binary
    execution_count: INT,
    avg_execution_time_ms: FLOAT,
    last_failure_reason: STRING
})

// Task/workflow
(:Task :ContextNode {
    title: STRING,
    status: STRING,               // "pending", "running", "completed", "failed"
    priority: STRING,             // "HIGH", "MEDIUM", "LOW"
    assigned_to: STRING,          // Module name (Ryzu, Echidna)
    result: TEXT
})

// Episodic memory (conversation context)
(:Memory :ContextNode {
    session_id: STRING,
    speaker: STRING,              // "user" or "mimi"
    message: TEXT,
    sentiment: FLOAT              // [-1.0, 1.0]
})
```

**Relationship Types:**

```cypher
// Semantic relationships
(:Entity)-[:RELATES_TO {weight: FLOAT}]->(:Entity)
(:Intent)-[:MENTIONS]->(:Entity)
(:Task)-[:REQUIRES]->(:Skill)
(:Task)-[:DEPENDS_ON]->(:Task)
(:Skill)-[:CREATED_BY]->(:Task)

// Temporal relationships
(:Memory)-[:NEXT {time_delta_ms: INT}]->(:Memory)
(:Intent)-[:TRIGGERED]->(:Task)

// Hierarchical relationships
(:Entity)-[:IS_A]->(:Entity)
(:Skill)-[:VERSION_OF]->(:Skill)
```

### 6.2 Indexes and Constraints

**Required Indexes:**
```cypher
// Unique constraint on node ID
CREATE CONSTRAINT node_id_unique IF NOT EXISTS
FOR (n:ContextNode) REQUIRE n.id IS UNIQUE;

// Index on temperature for fast filtering
CREATE INDEX temp_index IF NOT EXISTS
FOR (n:ContextNode) ON (n.temperature);

// Full-text search on entities
CREATE FULLTEXT INDEX entity_search IF NOT EXISTS
FOR (n:Entity) ON EACH [n.name, n.description];

// Composite index for time-based queries
CREATE INDEX time_temp_index IF NOT EXISTS
FOR (n:ContextNode) ON (n.last_accessed, n.temperature);
```

**Index Strategy:**
- Use range index on `temperature` for BFS filtering
- Use full-text index on `Entity.name` for NLP entity resolution
- Composite index on `(last_accessed, temperature)` for decay calculations

---

## 7. Integration Points

### 7.1 How Mimi Queries Pandora

**Flow:**
1. User sends message → Beatrice extracts `Intent`
2. Mimi receives `Intent` from Message Bus
3. **Before calling AI Adapter**, Mimi queries Pandora:
   ```rust
   let context = pandora::query_memory(&intent)?;
   let augmented_prompt = format!(
       "Context:\n{}\n\nUser: {}",
       context.to_prompt_string(),
       intent.user_message
   );
   let response = ai_adapter.generate(&augmented_prompt)?;
   ```
4. Pandora performs:
   - Entity resolution (match `intent.entities` to Neo4j `:Entity` nodes)
   - Heatmap filtering (exclude nodes with `temperature < 0.1`)
   - BFS traversal (max depth 3, max 500 nodes)
   - LRU cache lookup before hitting Neo4j
5. Returns serialized subgraph as JSON

**Message Bus Topic:**
```
Request: memory/query
Payload: {
    "intent_id": "uuid",
    "entities": ["entity_id_1", "entity_id_2"],
    "temp_threshold": 0.1,
    "max_nodes": 500
}

Response: memory/result
Payload: {
    "nodes": [...],
    "relationships": [...],
    "query_time_ms": 42
}
```

### 7.2 How Echidna Registers Skills

**Flow:**
1. Echidna generates new skill (Rhai script or WASM binary)
2. **After Odlaguna validates** the skill, Echidna calls:
   ```rust
   pandora::create_node("Skill", json!({
       "name": "auto_docker_cleanup",
       "skill_type": "rhai_script",
       "code_hash": "sha256_hash",
       "temperature": 1.0  // Start hot
   }))?;
   ```
3. Pandora creates `:Skill` node in Neo4j
4. Creates relationship: `(:Task)-[:CREATED]->(:Skill)`
5. Future queries for "docker cleanup" will return this skill with high temperature

**Message Bus Topic:**
```
Request: memory/create_skill
Payload: {
    "skill_name": "auto_docker_cleanup",
    "skill_type": "rhai_script",
    "metadata": {...}
}
```

### 7.3 How Odlaguna Logs Audit Trail

**Flow:**
1. Every critical operation (skill execution, task completion, failure) publishes to Message Bus
2. Odlaguna listens on `audit/log` topic
3. For each audit event, Odlaguna calls:
   ```rust
   pandora::create_node("AuditLog", json!({
       "event_type": "skill_executed",
       "actor": "ryzu-worker-03",
       "target": "skill_id",
       "result": "success",
       "timestamp": now_ms()
   }))?;
   ```
4. Creates relationship: `(:Skill)-[:AUDIT_ENTRY]->(:AuditLog)`
5. Immutable audit trail enables forensics and compliance

**Message Bus Topic:**
```
Publish: audit/log
Payload: {
    "event_type": "skill_executed",
    "actor": "module_name",
    "details": {...}
}
```

---

## 8. Error Handling

### 8.1 Neo4j Connection Loss

**Scenario:** Network partition or Neo4j crash

**Handling:**
```cpp
class BoltDriver {
    std::optional<neo4j::Result> safeQuery(const std::string& cypher) {
        int retry_count = 0;
        const int max_retries = 3;
        
        while (retry_count < max_retries) {
            try {
                auto result = session->run(cypher);
                return result;
            } catch (const neo4j::ConnectionException& e) {
                spdlog::warn("Neo4j connection lost, retrying ({}/{})", 
                            retry_count + 1, max_retries);
                std::this_thread::sleep_for(std::chrono::seconds(2 << retry_count));
                reconnect();
                retry_count++;
            }
        }
        
        spdlog::error("Neo4j connection failed after {} retries", max_retries);
        return std::nullopt;
    }
};
```

**Fallback Strategy:**
- Return cached context (L1 cache) if available
- If cache cold, return empty context with warning flag
- Mimi proceeds with zero-context mode (fallback to AI adapter without RAG)

### 8.2 Query Timeout

**Scenario:** Complex Cypher query exceeds 30s

**Handling:**
```cpp
// Set query timeout in Bolt session
neo4j::Config config;
config.maxConnectionLifetime = std::chrono::seconds(30);

// Timeout detection
auto future = std::async(std::launch::async, [&]() {
    return executeQuery(cypher);
});

if (future.wait_for(std::chrono::seconds(30)) == std::future_status::timeout) {
    spdlog::error("Query timeout: {}", cypher);
    // Cancel query and return partial results
    return cached_fallback();
}
```

**Prevention:**
- Enforce `LIMIT` clause in all queries
- Use query planner hints: `USING INDEX`, `USING SCAN`
- Periodic index maintenance

### 8.3 Graph Consistency

**Scenario:** Orphaned nodes or dangling relationships

**Detection:**
```cypher
// Find orphaned nodes (no incoming/outgoing relationships)
MATCH (n:ContextNode)
WHERE NOT (n)--()
RETURN n.id, n.type
```

**Cleanup:**
```cypher
// Periodic garbage collection (run by background thread)
MATCH (n:ContextNode)
WHERE n.temperature < 0.01
  AND n.last_accessed < timestamp() - 2592000000  // 30 days
DETACH DELETE n
```

**Transaction Safety:**
```cpp
void createNodeWithEdge(const Node& node, const Edge& edge) {
    driver->executeTransaction([&](neo4j::Transaction& tx) {
        // Create node
        tx.run("CREATE (n:ContextNode $props)", {{"props", node.toMap()}});
        
        // Create relationship atomically
        tx.run("MATCH (a {id: $from}), (b {id: $to}) "
               "CREATE (a)-[r:RELATES_TO]->(b)",
               {{"from", edge.from}, {"to", edge.to}});
    });
}
```

---

## 9. Performance Characteristics

### 9.1 Target Metrics

| Operation | Target Latency | Measurement Method |
|-----------|---------------|-------------------|
| **FFI Call Overhead** | < 1ms | `std::chrono` in C++, measure `pandora_query_context` entry to return |
| **Cache Hit (L1)** | < 0.5ms | Hash map lookup + temperature calculation |
| **Cache Miss → Neo4j Query** | < 50ms | Bolt roundtrip + Cypher execution + deserialization |
| **Node Creation** | < 10ms | Single `CREATE` statement with transaction |
| **BFS Traversal (depth=3)** | < 40ms | Cypher `MATCH` with variable-length path pattern |
| **Temperature Decay (1000 nodes)** | < 5ms | Batch update with single query |

### 9.2 Cache Hit Rate

**Target:** > 70% cache hit rate under normal workload

**Factors:**
- Working set size: ~500 unique nodes per session
- Cache capacity: 1000 nodes
- Temporal locality: Users typically work on related tasks (high hit rate)

**Monitoring:**
```cpp
struct CacheStats {
    uint64_t hits = 0;
    uint64_t misses = 0;
    
    double hitRate() const {
        return static_cast<double>(hits) / (hits + misses);
    }
};

// Log every 1000 queries
if ((cache_stats.hits + cache_stats.misses) % 1000 == 0) {
    spdlog::info("Cache hit rate: {:.2f}%", cache_stats.hitRate() * 100);
}
```

### 9.3 Throughput

**Target:** 1000 queries/second (single Pandora instance)

**Bottlenecks:**
- Neo4j Bolt connection pool (5 connections) → max ~500 concurrent queries
- Cache lock contention → use `std::shared_mutex` for read-heavy workload

**Scaling Strategy:**
```cpp
// Read-write lock for cache
std::shared_mutex cache_mutex;

CachedNode* getCached(const std::string& id) {
    std::shared_lock lock(cache_mutex);  // Multiple readers
    auto it = cache_map.find(id);
    return (it != cache_map.end()) ? &(*it->second) : nullptr;
}

void putCached(const CachedNode& node) {
    std::unique_lock lock(cache_mutex);  // Exclusive writer
    // ... update cache
}
```

### 9.4 Neo4j Tuning Tips

**Configuration (`neo4j.conf`):**
```properties
# Memory allocation
dbms.memory.heap.initial_size=2G
dbms.memory.heap.max_size=4G
dbms.memory.pagecache.size=2G

# Query optimization
cypher.min_replan_interval=10s
cypher.statistics_divergence_threshold=0.75

# Connection pooling
dbms.connector.bolt.thread_pool_min_size=5
dbms.connector.bolt.thread_pool_max_size=400
```

**Index Warming (Startup):**
```cypher
// Force index load into memory
MATCH (n:ContextNode)
WHERE n.temperature > 0
RETURN count(n)
```

---

## 10. Testing Strategy

### 10.1 Unit Tests (C++)

**Framework:** Google Test (gtest)

**Test Coverage:**
```cpp
// test/heatmap_test.cpp
TEST(HeatmapTest, TemperatureDecaysExponentially) {
    CachedNode node{"id", {}, 1.0, now_ms() - 10000, 1};
    double temp = calculateTemperature(node);
    EXPECT_LT(temp, 1.0);
    EXPECT_GT(temp, 0.9);  // After 10s, should be ~0.999
}

TEST(HeatmapTest, TemperatureBoostOnAccess) {
    CachedNode node{"id", {}, 0.5, now_ms(), 1};
    touchNode(&node);
    EXPECT_DOUBLE_EQ(node.temperature, 0.8);  // 0.5 + 0.3
}

TEST(LRUCacheTest, EvictsLeastRecentlyUsed) {
    LRUCache cache(3);
    cache.put("a", node_a);
    cache.put("b", node_b);
    cache.put("c", node_c);
    cache.put("d", node_d);  // Evicts "a"
    EXPECT_FALSE(cache.get("a").has_value());
}
```

### 10.2 Integration Tests (Neo4j)

**Framework:** Docker Compose + pytest (Python)

**Setup:**
```yaml
# docker-compose.test.yml
services:
  neo4j-test:
    image: neo4j:5.13
    environment:
      NEO4J_AUTH: neo4j/testpassword
    ports:
      - "7688:7687"
```

**Test Cases:**
```python
# test/integration/test_neo4j.py
def test_create_and_query_node(neo4j_client):
    # Create node via Pandora FFI
    node_id = pandora.create_node("Entity", {"name": "Docker"})
    
    # Query directly from Neo4j
    result = neo4j_client.run("MATCH (n {id: $id}) RETURN n", id=node_id)
    assert result.single()["n"]["name"] == "Docker"

def test_bfs_respects_temperature_threshold(neo4j_client):
    # Create graph with mixed temperatures
    setup_test_graph(neo4j_client)
    
    # Query with threshold 0.5
    subgraph = pandora.query_context(intent, temp_threshold=0.5)
    
    # Verify all returned nodes have temp > 0.5
    for node in subgraph["nodes"]:
        assert node["temperature"] > 0.5
```

### 10.3 Cypher Query Validation

**Tool:** `cypher-shell` + EXPLAIN/PROFILE

**Example:**
```cypher
// Verify query uses index
EXPLAIN
MATCH (n:ContextNode)
WHERE n.temperature > 0.1
RETURN n
LIMIT 500;

// Expected plan: NodeIndexSeekByRange (not NodeByLabelScan)
```

**Automated Validation:**
```python
def test_query_uses_index(neo4j_client):
    result = neo4j_client.run("EXPLAIN <query>")
    plan = result.summary().plan
    assert "NodeIndexSeekByRange" in str(plan)
```

### 10.4 Cache Behavior Tests

**Test Cases:**
```cpp
TEST(CacheTest, HitRateUnderWorkload) {
    LRUCache cache(1000);
    CacheStats stats;
    
    // Simulate 10k queries with 80% temporal locality
    for (int i = 0; i < 10000; i++) {
        std::string id = (i % 2 == 0) ? random_hot_id() : random_cold_id();
        if (cache.get(id)) {
            stats.hits++;
        } else {
            stats.misses++;
            cache.put(id, fetch_from_neo4j(id));
        }
    }
    
    EXPECT_GT(stats.hitRate(), 0.70);  // > 70% hit rate
}
```

### 10.5 Performance Regression Tests

**Benchmarking:**
```cpp
// benchmark/query_benchmark.cpp
static void BM_QueryContext(benchmark::State& state) {
    for (auto _ : state) {
        auto result = pandora_query_context(intent_json, 0.1, 500);
        benchmark::DoNotOptimize(result);
        pandora_free_string(result);
    }
}
BENCHMARK(BM_QueryContext)->Unit(benchmark::kMillisecond);
```

**CI Integration:**
```bash
# Run benchmarks and fail if latency > 50ms
./query_benchmark --benchmark_filter=BM_QueryContext
if [ $? -ne 0 ]; then
    echo "Performance regression detected!"
    exit 1
fi
```

---

## 11. Future Extensions (M2+)

### 11.1 Graph Clustering (M3)

**Goal:** Automatically detect semantic clusters in the graph for better context retrieval

**Algorithm:**
- Louvain community detection on `(:Entity)-[:RELATES_TO]->(:Entity)` subgraph
- Assign cluster ID to nodes: `SET n.cluster_id = $id`
- Query optimization: "Find all nodes in same cluster as intent entities"

**Cypher:**
```cypher
CALL gds.louvain.stream('context-graph')
YIELD nodeId, communityId
MATCH (n) WHERE id(n) = nodeId
SET n.cluster_id = communityId
```

### 11.2 Recommendation Engine (M4)

**Goal:** Proactively suggest skills/entities before user requests them

**Algorithm:**
- Collaborative filtering: "Users who accessed node A also accessed node B"
- Content-based: "Nodes similar to recently accessed nodes"
- Temporal patterns: "Node typically accessed after sequence X"

**Implementation:**
```cypher
// Find nodes frequently accessed together
MATCH (a:ContextNode)<-[:ACCESSED]-(session)-[:ACCESSED]->(b:ContextNode)
WHERE a <> b
WITH a, b, count(*) AS co_occurrence
ORDER BY co_occurrence DESC
LIMIT 10
CREATE (a)-[:OFTEN_WITH {score: co_occurrence}]->(b)
```

### 11.3 Distributed Pandora (M5+)

**Challenge:** Single Neo4j instance limits to ~1M nodes

**Solution:**
- Shard graph by semantic domain (e.g., skills in DB1, entities in DB2)
- Use Neo4j Fabric for federated queries
- Consistent hashing for node placement

**Architecture:**
```
Pandora Query Router
    ├─ Neo4j Shard 1 (Skills)
    ├─ Neo4j Shard 2 (Entities)
    └─ Neo4j Shard 3 (Episodic Memory)
```

### 11.4 Real-time Temperature Updates (M3)

**Current:** Temperature updated on query (lazy evaluation)

**Future:** Background worker continuously updates temperatures

**Implementation:**
```cpp
void temperatureUpdateWorker() {
    while (running) {
        std::this_thread::sleep_for(std::chrono::seconds(60));
        
        // Batch update all cached nodes
        for (auto& [id, node] : cache_map) {
            node.temperature = calculateTemperature(node);
        }
        
        // Persist to Neo4j (batch update)
        std::vector<std::pair<std::string, double>> updates;
        for (auto& [id, node] : cache_map) {
            updates.push_back({id, node.temperature});
        }
        batchUpdateTemperatures(updates);
    }
}
```

### 11.5 Multi-Modal Nodes (M6+)

**Goal:** Store images, audio embeddings alongside text

**Schema Extension:**
```cypher
(:ImageNode :ContextNode {
    embedding: [FLOAT],  // 512-dim vector from CLIP
    image_url: STRING,
    alt_text: STRING
})

// Vector similarity index
CREATE VECTOR INDEX image_embedding IF NOT EXISTS
FOR (n:ImageNode) ON (n.embedding)
OPTIONS {indexConfig: {
    `vector.dimensions`: 512,
    `vector.similarity_function`: 'cosine'
}}
```

**Query:**
```cypher
// Find similar images
MATCH (n:ImageNode)
WHERE n.embedding <~> $query_embedding
RETURN n
LIMIT 10
```

---

## Appendix: Cypher Query Examples

### A1. Create Context Node
```cypher
CREATE (n:ContextNode:Entity {
    id: $uuid,
    name: $name,
    entity_type: $type,
    temperature: 1.0,
    last_accessed: timestamp(),
    access_count: 0,
    metadata: $metadata
})
RETURN n.id
```

### A2. Query Hot Context
```cypher
MATCH path = (start:Entity)-[*1..3]-(related:ContextNode)
WHERE start.id IN $entity_ids
  AND related.temperature > $threshold
WITH related, 
     related.temperature AS temp,
     length(path) AS distance
ORDER BY temp DESC, distance ASC
LIMIT $max_nodes
RETURN related, temp, distance
```

### A3. Update Temperature on Access
```cypher
MATCH (n:ContextNode {id: $id})
SET n.temperature = $new_temp,
    n.last_accessed = timestamp(),
    n.access_count = n.access_count + 1
```

### A4. Batch Temperature Decay
```cypher
UNWIND $updates AS update
MATCH (n:ContextNode {id: update.id})
SET n.temperature = update.temperature
```

### A5. Find Related Skills
```cypher
MATCH (task:Task {id: $task_id})-[:REQUIRES]->(skill:Skill)
WHERE skill.temperature > 0.2
RETURN skill.name, skill.skill_type, skill.temperature
ORDER BY skill.temperature DESC
```

---

## Cross-References

- **Requirements:** [REQUIREMENTS.md#RF-3](../REQUIREMENTS.md) (Memory in Graphs)
- **Specifications:** `specs/HEATMAP-ALGORITHM.md` (Detailed decay formula derivation)
- **Architecture:** `ARCHITECTURE.md` (System-wide communication patterns)
- **Related Modules:** 
  - `MIMI-COMMANDER.md` (Primary consumer of Pandora queries)
  - `ECHIDNA.md` (Skill registration via Pandora)
  - `ODLAGUNA.md` (Audit trail storage in Pandora)

---

**Document Status:** ✅ Ready for Review  
**Last Updated:** 2026-04-16  
**Author:** MiMi Development Team
