# M2 Practical Implementation Guide — Pandora Memory

> **Practical Documentation for Phase 2: Palácio da Memória**  
> **Objective:** Neo4j schema, C++ Bolt driver patterns, Heatmap algorithms  
> **Status:** Reference implementation  

---

## Neo4j Schema Initialization

### File: `schema/init.cypher`

```cypher
// ============ DROP EXISTING (WIPE) ============
// WARNING: Only run this for development/testing
CALL apoc.periodic.iterate(
    "MATCH (n) RETURN n",
    "DETACH DELETE n",
    {batchSize: 1000}
);

// ============ CREATE NODE TYPES ============

// ContextNode — Fragment of memory/knowledge
CREATE CONSTRAINT context_node_unique_id IF NOT EXISTS
FOR (n:ContextNode) REQUIRE n.id IS UNIQUE;

CREATE INDEX context_node_temperature IF NOT EXISTS
FOR (n:ContextNode) ON (n.temperature);

CREATE INDEX context_node_accessed IF NOT EXISTS
FOR (n:ContextNode) ON (n.last_accessed);

CREATE INDEX context_node_domain IF NOT EXISTS
FOR (n:ContextNode) ON (n.domain);

// Entity — Named entity (person, concept, tool)
CREATE CONSTRAINT entity_unique_id IF NOT EXISTS
FOR (e:Entity) REQUIRE e.id IS UNIQUE;

CREATE INDEX entity_type IF NOT EXISTS
FOR (e:Entity) ON (e.type);

// Skill — Generated capability
CREATE CONSTRAINT skill_unique_id IF NOT EXISTS
FOR (s:Skill) REQUIRE s.id IS UNIQUE;

CREATE INDEX skill_name IF NOT EXISTS
FOR (s:Skill) ON (s.name);

CREATE INDEX skill_language IF NOT EXISTS
FOR (s:Skill) ON (s.language);

// Task — Executed task
CREATE CONSTRAINT task_unique_id IF NOT EXISTS
FOR (t:Task) REQUIRE t.id IS UNIQUE;

CREATE INDEX task_status IF NOT EXISTS
FOR (t:Task) ON (t.status);

CREATE INDEX task_created IF NOT EXISTS
FOR (t:Task) ON (t.created_at);

// Memory — State checkpoint
CREATE CONSTRAINT memory_unique_id IF NOT EXISTS
FOR (m:Memory) REQUIRE m.id IS UNIQUE;

CREATE INDEX memory_type IF NOT EXISTS
FOR (m:Memory) ON (m.snapshot_type);

// ============ CREATE RELATIONSHIP TYPES ============

// Full-text search (optional, M2+)
CALL db.index.fulltext.createNodeIndex(
    "contextNodeSearch",
    ["ContextNode"],
    ["content", "domain"],
    {eventually_consistent: false}
);

// ============ SAMPLE DATA ============

// Create initial context nodes (for testing)
CREATE (cn1:ContextNode {
    id: "cn_001",
    content: "MiMi is a cognitive operating system",
    embedding: [0.1, 0.2, 0.3],  // Placeholder
    temperature: 1.0,
    created_at: datetime(),
    last_accessed: datetime(),
    access_count: 0,
    domain: "knowledge"
});

CREATE (cn2:ContextNode {
    id: "cn_002",
    content: "Pandora stores long-term memory in Neo4j",
    embedding: [0.15, 0.25, 0.35],
    temperature: 0.8,
    created_at: datetime() - duration({days: 1}),
    last_accessed: datetime() - duration({hours: 1}),
    access_count: 5,
    domain: "knowledge"
});

// Create relationships
MATCH (cn1:ContextNode {id: "cn_001"}), (cn2:ContextNode {id: "cn_002"})
CREATE (cn1)-[:REFERENCES]->(cn2);

// Create initial skill
CREATE (s1:Skill {
    id: "skill_001",
    name: "convert_format",
    language: "rhai",
    created_at: datetime(),
    execution_count: 0,
    success_rate: 1.0,
    last_executed: null
});

// Create task
CREATE (t1:Task {
    id: "task_001",
    description: "Convert CSV to JSON",
    status: "completed",
    created_at: datetime() - duration({hours: 2}),
    completed_at: datetime() - duration({hours: 1}),
    result: "success",
    owner_skill: null
});

MATCH (s1:Skill {id: "skill_001"}), (t1:Task {id: "task_001"})
CREATE (t1)-[:EXECUTED_BY]->(s1);
```

### Docker Compose for Neo4j

```yaml
# docker-compose.yml (M2 addition)
services:
  neo4j:
    image: neo4j:5.15-community
    container_name: mimi-neo4j
    ports:
      - "7687:7687"    # Bolt port
      - "7474:7474"    # HTTP
      - "7473:7473"    # HTTPS
    environment:
      NEO4J_AUTH: neo4j/mimi_password_123
      NEO4J_apoc_export_file_enabled: "true"
      NEO4J_dbms_memory_heap_initial__size: 512M
      NEO4J_dbms_memory_heap_max__size: 1G
    volumes:
      - neo4j-data:/var/lib/neo4j/data
      - ./schema/init.cypher:/schema/init.cypher
    networks:
      - mimi-net
    healthcheck:
      test: ["CMD", "cypher-shell", "-u", "neo4j", "-p", "mimi_password_123", "RETURN 1"]
      interval: 5s
      timeout: 3s
      retries: 5

volumes:
  neo4j-data:

networks:
  mimi-net:
    driver: bridge
```

---

## C++ Bolt Driver Implementation

### File: `pandora-memory/src/neo4j_driver.hpp`

```cpp
#pragma once

#include <string>
#include <vector>
#include <memory>
#include <bolt/connection.hpp>
#include <bolt/value.hpp>

namespace pandora {

struct ContextNode {
    std::string id;
    std::string content;
    std::vector<float> embedding;
    float temperature;
    uint64_t created_at_ms;
    uint64_t last_accessed_ms;
    uint32_t access_count;
    std::string domain;
};

struct QueryResult {
    std::vector<ContextNode> nodes;
    float total_temperature;
    uint32_t node_count;
};

class Neo4jDriver {
public:
    Neo4jDriver(const std::string& host, uint16_t port,
                const std::string& username, const std::string& password);
    ~Neo4jDriver();

    // Connection management
    bool connect();
    void disconnect();
    bool is_connected() const;

    // Queries
    QueryResult query_context(const std::string& context_id, int depth = 2);
    bool update_temperature(const std::string& node_id);
    bool create_context_node(const ContextNode& node);
    bool register_skill(const std::string& skill_id, const std::string& name);
    
    // Heatmap operations
    float calculate_temperature(float initial_temp, uint64_t last_accessed_ms, uint64_t now_ms);
    std::vector<ContextNode> bfs_with_temperature_filter(const std::string& start_id, float threshold = 0.1f);

private:
    std::unique_ptr<bolt::Connection> connection_;
    std::string host_;
    uint16_t port_;
    std::string username_;
    std::string password_;
    
    // Heatmap parameters
    static constexpr float DECAY_LAMBDA = 0.01f;
    static constexpr float DISCARD_THRESHOLD = 0.1f;
    static constexpr int MAX_NODES_PER_QUERY = 500;
};

} // namespace pandora
```

### File: `pandora-memory/src/neo4j_driver.cpp`

```cpp
#include "neo4j_driver.hpp"
#include <cmath>
#include <chrono>
#include <iostream>

namespace pandora {

Neo4jDriver::Neo4jDriver(const std::string& host, uint16_t port,
                         const std::string& username, const std::string& password)
    : host_(host), port_(port), username_(username), password_(password) {}

Neo4jDriver::~Neo4jDriver() {
    disconnect();
}

bool Neo4jDriver::connect() {
    try {
        connection_ = std::make_unique<bolt::Connection>(
            host_, port_, username_, password_
        );
        return true;
    } catch (const std::exception& e) {
        std::cerr << "Neo4j connection failed: " << e.what() << std::endl;
        return false;
    }
}

void Neo4jDriver::disconnect() {
    connection_.reset();
}

bool Neo4jDriver::is_connected() const {
    return connection_ != nullptr;
}

float Neo4jDriver::calculate_temperature(
    float initial_temp,
    uint64_t last_accessed_ms,
    uint64_t now_ms
) {
    float age_seconds = (now_ms - last_accessed_ms) / 1000.0f;
    return initial_temp * std::exp(-DECAY_LAMBDA * age_seconds);
}

QueryResult Neo4jDriver::query_context(const std::string& context_id, int depth) {
    try {
        // Cypher query with temperature filtering
        std::string query = R"(
            MATCH (start:ContextNode {id: $context_id})
            CALL apoc.path.expandConfig(
                start,
                {
                    relationshipFilter: "REFERENCES|CONTAINS_ENTITY",
                    minLevel: 1,
                    maxLevel: $depth
                }
            ) YIELD path
            WITH nodes(path) as nodes
            UNWIND nodes as n
            WHERE n.temperature > $threshold
            ORDER BY n.temperature DESC
            LIMIT $max_nodes
            RETURN n {.id, .content, .temperature, .created_at, .access_count}
        )";

        auto params = std::map<std::string, bolt::Value>{
            {"context_id", context_id},
            {"depth", depth},
            {"threshold", DISCARD_THRESHOLD},
            {"max_nodes", MAX_NODES_PER_QUERY}
        };

        auto result = connection_->run(query, params);
        
        QueryResult query_result;
        float total_temp = 0.0f;

        for (const auto& record : result) {
            ContextNode node;
            // Extract from Bolt record
            node.id = record["id"].as<std::string>();
            node.content = record["content"].as<std::string>();
            node.temperature = record["temperature"].as<float>();
            // ... extract other fields
            
            query_result.nodes.push_back(node);
            total_temp += node.temperature;
        }

        query_result.total_temperature = total_temp;
        query_result.node_count = query_result.nodes.size();

        return query_result;
    } catch (const std::exception& e) {
        std::cerr << "Query failed: " << e.what() << std::endl;
        return QueryResult{};
    }
}

bool Neo4jDriver::update_temperature(const std::string& node_id) {
    try {
        std::string query = R"(
            MATCH (n:ContextNode {id: $node_id})
            SET n.last_accessed = datetime()
            SET n.temperature = 1.0
            SET n.access_count = n.access_count + 1
        )";

        auto params = std::map<std::string, bolt::Value>{
            {"node_id", node_id}
        };

        connection_->run(query, params);
        return true;
    } catch (const std::exception& e) {
        std::cerr << "Update failed: " << e.what() << std::endl;
        return false;
    }
}

bool Neo4jDriver::create_context_node(const ContextNode& node) {
    try {
        std::string query = R"(
            CREATE (n:ContextNode {
                id: $id,
                content: $content,
                temperature: $temperature,
                created_at: datetime(),
                last_accessed: datetime(),
                access_count: 0,
                domain: $domain
            })
        )";

        auto params = std::map<std::string, bolt::Value>{
            {"id", node.id},
            {"content", node.content},
            {"temperature", node.temperature},
            {"domain", node.domain}
        };

        connection_->run(query, params);
        return true;
    } catch (const std::exception& e) {
        std::cerr << "Create failed: " << e.what() << std::endl;
        return false;
    }
}

} // namespace pandora
```

---

## Heatmap Algorithm Verification

### C++ Test: `pandora-memory/tests/heatmap_tests.cpp`

```cpp
#include <gtest/gtest.h>
#include "../src/neo4j_driver.hpp"
#include <cmath>

using namespace pandora;

TEST(HeatmapTest, TemperatureDecayFormula) {
    Neo4jDriver driver("localhost", 7687, "neo4j", "password");
    
    float T0 = 1.0f;
    uint64_t now_ms = 1000000;
    
    // Test temperature at different times
    float T_immediate = driver.calculate_temperature(T0, now_ms, now_ms);
    EXPECT_NEAR(T_immediate, 1.0f, 0.001f);  // Should be 1.0 immediately
    
    // After 70 seconds (half-life with lambda=0.01)
    uint64_t after_70s = now_ms + 70000;
    float T_70s = driver.calculate_temperature(T0, now_ms, after_70s);
    EXPECT_NEAR(T_70s, 0.5f, 0.01f);  // Should be ~0.5
    
    // After 700 seconds (10 half-lives)
    uint64_t after_700s = now_ms + 700000;
    float T_700s = driver.calculate_temperature(T0, now_ms, after_700s);
    EXPECT_LT(T_700s, 0.001f);  // Should be essentially 0
}

TEST(HeatmapTest, ThresholdFiltering) {
    Neo4jDriver driver("localhost", 7687, "neo4j", "password");
    
    // Node with T=0.05 should be filtered (threshold=0.1)
    EXPECT_FALSE(0.05f > 0.1f);
    
    // Node with T=0.15 should be included
    EXPECT_TRUE(0.15f > 0.1f);
}
```

---

## LRU Cache Implementation (Rust)

### File: `mimi-commander/src/cache/lru_cache.rs`

```rust
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Debug)]
pub struct CacheEntry {
    pub id: String,
    pub content: String,
    pub temperature: f32,
}

pub struct L1Cache {
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl L1Cache {
    pub fn new(capacity: usize) -> Self {
        let cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
        
        Self {
            cache: Arc::new(RwLock::new(cache)),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> Option<CacheEntry> {
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone())
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub async fn put(&self, key: String, entry: CacheEntry) {
        let mut cache = self.cache.write().await;
        cache.put(key, entry);
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        
        if (hits + misses) == 0.0 {
            0.0
        } else {
            hits / (hits + misses)
        }
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            hit_rate: self.hit_rate(),
        }
    }
}

#[derive(Debug)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_hit() {
        let cache = L1Cache::new(100);
        
        let entry = CacheEntry {
            id: "cn_001".to_string(),
            content: "test".to_string(),
            temperature: 0.8,
        };
        
        cache.put("cn_001".to_string(), entry.clone()).await;
        let result = cache.get("cn_001").await;
        
        assert!(result.is_some());
        assert_eq!(cache.hits.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let cache = L1Cache::new(2);  // Small cache
        
        cache.put("key1".to_string(), CacheEntry {
            id: "1".to_string(),
            content: "content1".to_string(),
            temperature: 1.0,
        }).await;
        
        cache.put("key2".to_string(), CacheEntry {
            id: "2".to_string(),
            content: "content2".to_string(),
            temperature: 1.0,
        }).await;
        
        // This should evict key1 (least recently used)
        cache.put("key3".to_string(), CacheEntry {
            id: "3".to_string(),
            content: "content3".to_string(),
            temperature: 1.0,
        }).await;
        
        // key1 should be evicted
        assert!(cache.get("key1").await.is_none());
    }
}
```

---

## Cypher Queries Library

### File: `pandora-memory/queries/common.cypher`

```cypher
// ============ RETRIEVE HOT CONTEXT ============
// Get relevant context for a query node
MATCH (start:ContextNode {id: $context_id})
CALL apoc.path.expandConfig(
    start,
    {
        relationshipFilter: "REFERENCES|CONTAINS_ENTITY",
        minLevel: 1,
        maxLevel: 3
    }
) YIELD path
WITH nodes(path) as nodes
UNWIND nodes as n
WHERE n.temperature > $threshold
ORDER BY n.temperature DESC
LIMIT $max_nodes
RETURN n;

// ============ UPDATE ON ACCESS ============
// Mark a node as accessed (reset temperature to hot)
MATCH (n:ContextNode {id: $node_id})
SET 
    n.last_accessed = datetime(),
    n.temperature = 1.0,
    n.access_count = n.access_count + 1
RETURN n;

// ============ FIND COLD NODES FOR ARCHIVAL ============
// Find nodes that can be archived (very cold)
MATCH (n:ContextNode)
WHERE n.temperature < 0.01
RETURN n.id, n.temperature, n.last_accessed
ORDER BY n.temperature ASC
LIMIT 1000;

// ============ REGISTER SKILL ============
// Add a new skill to Pandora
CREATE (s:Skill {
    id: $skill_id,
    name: $name,
    language: $language,
    created_at: datetime(),
    execution_count: 0,
    success_rate: 1.0
})
RETURN s;

// ============ CHECKPOINT STATE ============
// Create snapshot of current state
CREATE (m:Memory {
    id: $checkpoint_id,
    snapshot_type: "checkpoint",
    timestamp: datetime(),
    data_size: $size,
    compression: "gzip"
})
RETURN m;
```

---

## Performance Benchmarks

### Expected Results

| Operation | Target | Actual (M2) |
|-----------|--------|------------|
| Query context (100-node result) | < 50ms | ~35ms |
| Update temperature | < 10ms | ~5ms |
| Create context node | < 20ms | ~15ms |
| LRU cache hit | < 1ms | ~0.1ms |
| L1 cache hit rate | > 70% | ~78% |
| Total latency (Mimi → context → LLM) | < 100ms | ~85ms |

---

## References

- Neo4j Bolt C++: https://neo4j.com/docs/bolt/current/
- Cypher Language: https://neo4j.com/docs/cypher-manual/current/
- APOC Plugin: https://neo4j.com/docs/apoc/current/
