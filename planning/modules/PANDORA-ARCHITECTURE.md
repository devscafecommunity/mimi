# PANDORA: Memory Engine Architecture & Query Generation System

> **Module**: Pandora (ST&LT Memory Manager)  
> **Language**: C++ (with Rust FFI for Mimi integration)  
> **Database**: Neo4j 5.x with Bolt protocol  
> **Status**: Architecture Phase (Evidence-driven design)  
> **Last Updated**: 2026-04-16  

---

## Executive Summary

Pandora is MiMi's intelligent memory curator. Unlike passive databases, Pandora **actively shapes context** through a 4-layer architecture:

1. **Metadata Layer** - Schema enforcement preventing invalid queries
2. **DSL Templates** - Type-safe Cypher generation (no hallucinations)
3. **Injection Filter** - Strict parameter binding (no injection attacks)
4. **Refinement Engine** - Query optimization and execution prep
5. **Context Curation** - Multi-dimensional ranking (Temporal + Frequency + Semantic)

The goal: **Perfect queries every time** + **Best context always delivered**.

---

## Part 1: Perfect Query Generation (4-Layer Architecture)

### Why This Matters

Traditional LLM-based query generation fails because:
- ❌ LLMs hallucinate non-existent relationships or properties
- ❌ String concatenation leads to injection vulnerabilities
- ❌ No compile-time guarantees against invalid Cypher
- ❌ Query plans bloat without parameter reuse

**Pandora's answer**: Structured, type-safe query builders that emit valid Cypher by construction.

### Layer 1: Metadata (Graph Schema Definitions)

**Purpose**: Rigid schema map prevents invalid queries before execution.

**What it contains**:

```cpp
// Schema definition in C++ structures
struct PropertySchema {
    std::string name;
    std::string type;  // "STRING", "FLOAT", "INT", "BOOL", "DATETIME"
    bool required;
    std::string description;
};

struct NodeSchema {
    std::string label;  // e.g., "ContextNode", "Skill", "Error"
    std::vector<PropertySchema> properties;
    std::vector<std::string> identifyingKeys;  // e.g., ["id"]
    bool abstract;
};

struct RelationshipSchema {
    std::string type;  // e.g., "CREATED", "REFERENCES", "FAILED_ON"
    NodeSchema sourceLabel;
    NodeSchema targetLabel;
    std::vector<PropertySchema> properties;
    Cardinality cardinality;  // ONE_TO_ONE, ONE_TO_MANY, MANY_TO_MANY
};

class GraphSchema {
public:
    bool validateNodeAccess(const std::string& label, const std::string& property);
    bool validateRelationship(const std::string& type, 
                             const std::string& sourceLabel,
                             const std::string& targetLabel);
    std::optional<PropertySchema> getProperty(const std::string& label, 
                                             const std::string& property);
};
```

**MiMi's Core Nodes**:
- `ContextNode` - memory with heat, timestamp, content_hash
- `Skill` - executable with version, status
- `Error` - diagnostic info, stack trace
- `Configuration` - system state
- `Pattern` - recognized behavior

**MiMi's Core Relationships**:
- `[:CREATED]` - Echidna → Skill
- `[:REFERENCES]` - ContextNode → ContextNode (edges in graph)
- `[:FAILED_ON]` - Skill → Error (failure history)
- `[:CACHED_IN]` - ContextNode → L1Cache

**Runtime Validation**:
```cpp
// Before any query executes:
if (!schema.validateNodeAccess("ContextNode", "heat")) {
    throw InvalidSchemaException("ContextNode has no 'heat' property");
}
if (!schema.validateRelationship("KNOWS", "Person", "Skill")) {
    throw InvalidSchemaException("No KNOWS relationship from Person to Skill");
}
```

---

### Layer 2: DSL Templates (Type-Safe Query Construction)

**Purpose**: Express queries as ASTs, not strings. Leverage C++ type system.

**Core Concept**: Queries are built declaratively, then rendered to Cypher + Parameters.

**Evidence Base**: 
- Cypher Builder (JS/TS): `neo4j/cypher-builder` - produces typed queries → `.build()` returns `{cypher, params}`
- Cypher-DSL (Java): `neo4j-contrib/cypher-dsl` - AST-based, type-safe Cypher generation
- Pattern: All use parameter placeholders (`$param0`) + separate params map

**Pandora DSL Structure**:

```cpp
// AST Node types
class CyNode {
public:
    std::string variable;
    std::string label;
    std::unordered_map<std::string, std::shared_ptr<Parameter>> properties;
};

class CyEdge {
public:
    std::string variable;
    std::string type;
    int minHops, maxHops;  // For patterns like -[r*1..2]->
};

class CyPredicate {
public:
    enum Op { EQ, NE, GT, LT, GTE, LTE, IN, CONTAINS, REGEX };
    std::shared_ptr<CyNode> subject;
    Op op;
    std::shared_ptr<Parameter> value;
};

class CyQuery {
private:
    std::vector<std::shared_ptr<CyClause>> clauses;
    
public:
    // Fluent API
    CyQuery& match(const CyNode& n) { /* add MATCH clause */ return *this; }
    CyQuery& where(const CyPredicate& p) { /* add WHERE clause */ return *this; }
    CyQuery& return_nodes(const std::vector<std::string>& vars) { /* RETURN */ return *this; }
    CyQuery& order_by(const std::string& var, bool descending = false) { /* ORDER BY */ return *this; }
    CyQuery& limit(int n) { /* LIMIT */ return *this; }
    
    // Render to Cypher + params
    QueryResult build() const;  // Returns {cypher_string, params_map}
};

// QueryResult = { string cypher, unordered_map<string, any> params }
```

**Template Examples**:

```cpp
// Template 1: Search by Context Heatmap
template<typename T>
CyQuery searchContextByHeat(const T& minThreshold, int limit = 100) {
    CyQuery q;
    auto node = std::make_shared<CyNode>("n", "ContextNode");
    auto threshold_param = std::make_shared<Parameter>("min_heat", minThreshold);
    
    q.match(*node)
     .where(CyPredicate{node, Op::GTE, threshold_param})
     .return_nodes({"n"})
     .order_by("n.heat", true)  // descending
     .limit(limit);
    
    return q;
}

// Template 2: Expansion Query (Related Nodes)
CyQuery expandContextWithRelations(const std::string& nodeId, int maxHops = 2) {
    CyQuery q;
    auto root = std::make_shared<CyNode>("n", "ContextNode");
    auto edge = std::make_shared<CyEdge>("r", "", 1, maxHops);
    auto target = std::make_shared<CyNode>("related", "ContextNode");
    auto heat_min = std::make_shared<Parameter>("heat_threshold", 0.1);
    
    q.match(*root)
     .match_pattern(*edge, *target)
     .where(CyPredicate{target, Op::GTE, heat_min})
     .return_nodes({"n", "related", "r"})
     .order_by("n.heat", true)
     .limit(500);
    
    return q;
}

// Template 3: Error Analysis
CyQuery findErrorsForSkill(const std::string& skillId) {
    CyQuery q;
    auto skill = std::make_shared<CyNode>("s", "Skill");
    auto error = std::make_shared<CyNode>("e", "Error");
    auto skillIdParam = std::make_shared<Parameter>("skill_id", skillId);
    
    q.match(*skill)
     .where(CyPredicate{skill, Op::EQ, skillIdParam})
     .match_pattern(std::make_shared<CyEdge>("r", "FAILED_ON", 1, 1), *error)
     .return_nodes({"e"})
     .order_by("e.timestamp", false);  // most recent first
    
    return q;
}
```

**Rendered Output Example**:
```
Input:  searchContextByHeat(0.5, 100)
Output: {
  cypher: "MATCH (n:ContextNode) WHERE n.heat >= $param0 RETURN n ORDER BY n.heat DESC LIMIT $param1",
  params: { param0: 0.5, param1: 100 }
}
```

---

### Layer 3: Injection Filter (Strict Parameter Binding)

**Purpose**: Convert arbitrary user input into safe, typed parameters.

**Pattern** (Evidence from Cypher Builder + Cypher-DSL):
- All user data becomes a Parameter object
- Parameters never interpolate into Cypher strings
- Cypher uses `$paramX` placeholders exclusively

```cpp
class Parameter {
private:
    std::string name_;
    std::variant<int, double, std::string, bool, std::vector<std::string>> value_;
    
public:
    Parameter(const std::string& name, const auto& value) 
        : name_(name), value_(value) {}
    
    template<typename T>
    std::optional<T> get() const {
        if (std::holds_alternative<T>(value_)) {
            return std::get<T>(value_);
        }
        return std::nullopt;
    }
    
    std::string asString() const;  // Safe conversion
    bool validate() const;  // Type + range checks
};

class InjectionFilter {
public:
    // Convert external input to safe Parameters
    std::shared_ptr<Parameter> bind(const std::string& name, const nlohmann::json& userInput) {
        // Validation happens here: type checks, bounds, whitelist/blacklist
        if (userInput.is_string() && userInput.get<std::string>().length() > 10000) {
            throw InjectionException("String parameter exceeds max length");
        }
        return std::make_shared<Parameter>(name, userInput);
    }
    
    // Sanitize search queries (for full-text search parameters)
    std::string sanitizeSearchTerm(const std::string& raw) {
        // Escape special regex chars, limit length
        std::string safe = raw;
        safe.erase(remove_if(safe.begin(), safe.end(), 
                            [](char c) { return !std::isalnum(c) && c != ' ' && c != '-'; }),
                  safe.end());
        return safe.substr(0, 1000);
    }
};
```

**Validation Rules**:
- String length: max 10,000 characters
- Integer range: int32_t bounds
- Float precision: double (no arbitrary large decimals)
- Array length: max 1,000 elements
- Enum values: must be in predefined set

---

### Layer 4: Refinement Engine (Query Optimization & Execution Prep)

**Purpose**: Render AST to canonical Cypher + optimize for execution.

```cpp
class RefinementEngine {
private:
    std::string cypherVersion_;  // "5.0", "4.4", etc.
    GraphSchema schema_;
    
public:
    struct QueryResult {
        std::string cypher;
        std::unordered_map<std::string, std::any> params;
        std::string executionPlan;  // For logging/debugging
        uint64_t estimatedCost;  // For caching decisions
    };
    
    QueryResult refine(const CyQuery& q) {
        // 1. Render AST to string
        std::string cypher = renderAST(q);
        
        // 2. Extract all parameters
        auto params = collectParameters(q);
        
        // 3. Validate against schema
        validateCypherAgainstSchema(cypher);
        
        // 4. Optimize (add hints, reorder clauses)
        cypher = optimizeCypher(cypher);
        
        // 5. Generate execution plan preview
        std::string plan = generateExecutionPlan(cypher);
        
        // 6. Estimate cost (for circuit breaker)
        uint64_t cost = estimateCost(cypher, params);
        if (cost > MAX_QUERY_COST) {
            throw QueryTooExpensiveException(cost);
        }
        
        return QueryResult{cypher, params, plan, cost};
    }
    
private:
    std::string renderAST(const CyQuery& q) {
        // Convert AST to canonical Cypher string
        // Handles: MATCH, WHERE, RETURN, ORDER BY, LIMIT, WITH, etc.
    }
    
    void validateCypherAgainstSchema(const std::string& cypher) {
        // Parse Cypher, extract referenced nodes/relationships/properties
        // Verify each exists in schema
    }
    
    std::string optimizeCypher(const std::string& cypher) {
        // Reorder WHERE clauses by selectivity
        // Add index hints if applicable
        // Remove redundant patterns
        return cypher;
    }
};
```

**Optimization Rules**:
- Place most selective WHERE clauses first
- Hint index usage for equality checks on identifying keys
- Rewrite `(a)-[r*0..n]->(b)` patterns for efficiency
- Cache execution plans by query shape

---

## Part 2: Context Curation (Relevance Engine)

### Why Context Curation Matters

Raw database results are **not always best context**:
- Old data crowds out fresh data (stale but high-relevance content dominates)
- Frequency bias (popular patterns override unique but critical info)
- Semantic drift (vector similarity alone misses context intent)
- Lack of hierarchy (too much context floods the LLM's reasoning)

**Pandora's solution**: Multi-dimensional ranking + hierarchical delivery.

---

### Design Pattern: Multi-Signal Scoring

**Evidence Base** (from search results):
- **QRRanker** (arXiv 2602.12192): Multi-signal ranking in retrieval
- **Temporal Decay** (Milvus tutorial + OpenClaw): Exponential decay `score × e^(-λ×age)`
- **Hierarchical Delivery** (OpenViking + Meilisearch): Layer-wise context progression
- **Negative Context** (Retrieval Rerankers paper): Detect and warn on failed patterns

### Scoring Formula

```
FinalScore = w_sem × SemanticRelevance 
           + w_temp × TemporalScore(age, last_accessed)
           + w_freq × FrequencyScore(access_count)
           + w_conf × ConfidenceScore(quality_metrics)
           - penalty_negative_context
```

**Component Breakdown**:

#### 1. Semantic Relevance (w_sem = 0.35)

```cpp
// Vector embedding similarity to query intent
double semanticRelevance(const std::vector<float>& queryVector,
                        const std::vector<float>& nodeVector) {
    // Cosine similarity: (a·b) / (||a|| × ||b||)
    double dotProduct = 0.0, normA = 0.0, normB = 0.0;
    for (size_t i = 0; i < queryVector.size(); ++i) {
        dotProduct += queryVector[i] * nodeVector[i];
        normA += queryVector[i] * queryVector[i];
        normB += nodeVector[i] * nodeVector[i];
    }
    return dotProduct / (std::sqrt(normA) * std::sqrt(normB));
}
```

#### 2. Temporal Decay (w_temp = 0.30)

```cpp
// Exponential decay: T(t) = T₀ × e^(-λ×age)
double temporalScore(uint64_t lastAccessedTimestamp, double decayLambda = 0.1) {
    uint64_t now = std::chrono::system_clock::now().time_since_epoch().count();
    double ageInDays = (now - lastAccessedTimestamp) / (24.0 * 3600 * 1e9);
    
    // e^(-0.1 × ageInDays) means:
    // 1.0 at age 0, 0.90 at 1 day, 0.37 at 10 days, 0.01 at 46 days
    return std::exp(-decayLambda * ageInDays);
}
```

#### 3. Frequency Boost (w_freq = 0.20)

```cpp
// Logarithmic scaling: prevents "popular" from drowning out "relevant"
double frequencyScore(uint32_t accessCount) {
    // log2(1 + count) maps: 0→0, 1→1, 10→3.46, 100→6.64, 1000→9.96
    return std::log2(1.0 + accessCount) / 10.0;  // normalized to [0, 1]
}
```

#### 4. Confidence Score (w_conf = 0.15)

```cpp
// Quality metrics: data freshness, validation status
double confidenceScore(const ContextNode& node) {
    double conf = 0.5;  // base
    
    if (node.validation_status == VALIDATED) conf += 0.3;
    if (node.data_source == AUTHORITATIVE) conf += 0.2;
    if (node.contradiction_count == 0) conf += 0.1;  // no conflicting data
    
    return std::min(conf, 1.0);
}
```

#### 5. Negative Context Penalty (constant deduction)

```cpp
// If this node led to errors or hallucinations recently, penalize
double getNegativeContextPenalty(const ContextNode& node) {
    // Check: has this been used in failed queries in the last 7 days?
    auto recentFailures = queryFailureHistory.getFailuresFor(node.id, 7);
    
    if (recentFailures.size() > 5) {
        return 0.5;  // Heavy penalty: this context is problematic
    } else if (recentFailures.size() > 0) {
        return 0.2;  // Light penalty: caution, but not blacklisted
    }
    return 0.0;  // No penalty
}
```

**Full Ranking Function**:

```cpp
struct RankedContext {
    std::shared_ptr<ContextNode> node;
    double finalScore;
    std::vector<double> scoreBreakdown;  // For debugging
};

std::vector<RankedContext> rankContext(
    const std::vector<std::shared_ptr<ContextNode>>& candidates,
    const std::vector<float>& queryVector,
    const ScoringWeights& weights = {0.35, 0.30, 0.20, 0.15}
) {
    std::vector<RankedContext> ranked;
    
    for (const auto& node : candidates) {
        double sem = semanticRelevance(queryVector, node->embedding);
        double temp = temporalScore(node->last_accessed_timestamp);
        double freq = frequencyScore(node->access_count);
        double conf = confidenceScore(*node);
        double penalty = getNegativeContextPenalty(*node);
        
        double finalScore = weights.sem * sem
                          + weights.temp * temp
                          + weights.freq * freq
                          + weights.conf * conf
                          - penalty;
        
        ranked.push_back({node, finalScore, {sem, temp, freq, conf}});
    }
    
    // Sort by final score descending
    std::sort(ranked.begin(), ranked.end(),
              [](const auto& a, const auto& b) { return a.finalScore > b.finalScore; });
    
    return ranked;
}
```

---

### Hierarchical Context Delivery (L1/L2/L3)

**Why Hierarchy**: LLMs have token budgets. Deliver best data first, let caller request deeper context if needed.

```cpp
struct ContextDeliveryPacket {
    // L1: Immediate, high-confidence essentials
    std::vector<std::shared_ptr<ContextNode>> primary_focus;  // Top 3-5
    
    // L2: Referential (caller can request)
    std::vector<std::string> suggested_skill_ids;  // Can explore
    std::vector<std::string> related_error_ids;    // Can explore
    
    // L3: Historical metadata
    std::vector<FailureRecord> recent_failures;  // Warnings
    std::string confidence_explanation;  // "Why these results"
    
    // Anti-patterns (negative context)
    std::vector<std::string> antipattern_warnings;  // "Avoid X because..."
    double overall_confidence_score;  // 0.0 to 1.0
};

ContextDeliveryPacket distillContext(
    const std::vector<RankedContext>& rankedCandidates,
    const DistillationConfig& config  // token budgets, depth
) {
    ContextDeliveryPacket packet;
    
    // L1: Top 3-5 nodes (ordered by score)
    for (size_t i = 0; i < std::min(5ul, rankedCandidates.size()); ++i) {
        if (rankedCandidates[i].finalScore > config.l1_threshold) {
            packet.primary_focus.push_back(rankedCandidates[i].node);
        }
    }
    
    // L2: Suggested deep-dives (nodes 5-20, with clickable IDs)
    for (size_t i = 5; i < std::min(20ul, rankedCandidates.size()); ++i) {
        if (rankedCandidates[i].finalScore > config.l2_threshold) {
            packet.suggested_skill_ids.push_back(rankedCandidates[i].node->id);
        }
    }
    
    // L3: Historical context (anomalies, failures)
    auto failures = queryFailureHistory.getRecentFailures(7);  // Last 7 days
    for (const auto& failure : failures) {
        packet.recent_failures.push_back(failure);
        packet.antipattern_warnings.push_back(
            "Avoid: " + failure.failed_query + " (" + failure.reason + ")"
        );
    }
    
    // Calculate confidence
    double avgScore = std::accumulate(rankedCandidates.begin(),
                                     rankedCandidates.begin() + packet.primary_focus.size(),
                                     0.0,
                                     [](double sum, const auto& rc) { return sum + rc.finalScore; });
    packet.overall_confidence_score = avgScore / std::max(1ul, packet.primary_focus.size());
    
    return packet;
}
```

---

### Output Format: The Perfect Context Message

When Pandora delivers context to Mimi (via Message Bus):

```json
{
  "module": "PANDORA",
  "target": "MIMI",
  "context_payload": {
    "query_id": "q_abc123",
    "intent": "Find socket errors in Ryzu worker",
    
    "L1_primary_focus": [
      {
        "id": "ctx_001",
        "type": "Error",
        "content": "Socket ECONNREFUSED at 2026-04-15T14:32:10Z",
        "heat": 0.92,
        "timestamp": 1713181930000,
        "confidence": 0.98
      },
      {
        "id": "ctx_002",
        "type": "Configuration",
        "content": "Ryzu Docker image: v2.1.5 with port 9090 binding",
        "heat": 0.87,
        "confidence": 0.95
      }
    ],
    
    "L2_referential": {
      "related_skills": ["NETWORK_DIAGNOSTICS_v3", "DOCKER_TROUBLESHOOT_v2"],
      "related_errors": ["err_socket_timeout", "err_port_conflict"],
      "message": "Can explore these skills/errors if L1 insufficient"
    },
    
    "L3_historical": {
      "recent_failures": [
        {
          "query": "MATCH (e:Error) WHERE e.type='ECONNREFUSED' RETURN e",
          "failed_at": "2026-04-14T12:00:00Z",
          "reason": "Timeout (query touched 50k nodes, exceeded budget)"
        }
      ],
      "warnings": [
        "Caution: This error pattern has caused LLM hallucinations about port binding (see failure 2026-04-14)",
        "Note: Ryzu Docker config changed 2 days ago; old troubleshooting docs may not apply"
      ]
    },
    
    "metadata": {
      "confidence_score": 0.93,
      "ranking_breakdown": {
        "semantic_relevance": 0.95,
        "temporal_freshness": 0.89,
        "frequency": 0.67,
        "data_quality": 0.98
      },
      "execution_time_ms": 47,
      "nodes_examined": 5832,
      "nodes_filtered_by_threshold": 5794
    }
  }
}
```

---

## Part 3: Query Generation Pipeline (End-to-End)

### Full Flow Diagram

```
User Intent (from Mimi)
    ↓
[1. Metadata Validation]
    ├─ Check: node labels exist?
    ├─ Check: properties exist?
    ├─ Check: relationships allowed?
    └─ → Validation Pass/Fail
    
    ↓ (if pass)
    
[2. DSL Template Selection]
    ├─ Parse intent keywords
    ├─ Match to template (searchContextByHeat, expandContext, etc.)
    ├─ Instantiate template with parameters
    └─ → CyQuery AST
    
    ↓
    
[3. Injection Filter]
    ├─ Extract user-provided values
    ├─ Validate types & bounds
    ├─ Create Parameter objects
    ├─ Escape/sanitize strings
    └─ → Safe Parameters map
    
    ↓
    
[4. Refinement Engine]
    ├─ Render AST to Cypher string
    ├─ Validate against schema again
    ├─ Optimize query clauses
    ├─ Estimate execution cost
    ├─ Check circuit breaker
    └─ → { cypher, params, executionPlan, cost }
    
    ↓
    
[5. Neo4j Bolt Execution]
    ├─ Send to driver (parameterized)
    ├─ Receive result set
    ├─ Parse nodes/relationships
    └─ → Raw results
    
    ↓
    
[6. Context Curation]
    ├─ Rank results (multi-signal scoring)
    ├─ Distill into L1/L2/L3 layers
    ├─ Detect negative contexts
    ├─ Generate confidence score
    └─ → ContextDeliveryPacket
    
    ↓
    
[7. Message Bus Delivery]
    └─ Publish to `context/ready` topic for Mimi
```

---

## Part 4: Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Implement `GraphSchema` class with validation
- [ ] Define MiMi node/relationship schemas
- [ ] Write schema validation tests (100+ test cases)

### Phase 2: DSL & Query Builder (Week 2-4)
- [ ] Implement `CyNode`, `CyEdge`, `CyQuery` classes
- [ ] Implement 3 template functions: `searchContextByHeat`, `expandContext`, `findErrorsForSkill`
- [ ] Implement AST rendering to Cypher strings
- [ ] Validate generated queries against test cases

### Phase 3: Safety Layers (Week 4-5)
- [ ] Implement `Parameter` class with type validation
- [ ] Implement `InjectionFilter` with sanitization
- [ ] Implement `RefinementEngine` with optimization & cost estimation
- [ ] Add circuit breaker (reject queries exceeding cost threshold)

### Phase 4: Context Curation (Week 5-7)
- [ ] Implement multi-signal scoring (semantic, temporal, frequency, confidence)
- [ ] Implement hierarchical delivery (L1/L2/L3)
- [ ] Implement negative context detection & warnings
- [ ] Add execution time & node-count metrics

### Phase 5: Integration & Testing (Week 7-8)
- [ ] Integration with Neo4j driver (Bolt protocol)
- [ ] Integration with Message Bus (Zenoh)
- [ ] End-to-end tests (query generation → execution → ranking)
- [ ] Performance benchmarks (target: 95% of queries < 100ms)

---

## References & Evidence

### Query Builder Libraries
- **Cypher Builder (JS/TS)**: https://github.com/neo4j/cypher-builder
- **Cypher-DSL (Java)**: https://github.com/neo4j-contrib/cypher-dsl
- **Official Neo4j Cypher Builder Docs**: https://neo4j.com/docs/cypher-builder/

### Context Ranking & RAG Patterns
- **QRRanker** (Multi-signal ranking): arXiv 2602.12192
- **Temporal Decay in Production**: Milvus tutorial + OpenClaw commit (exponential decay formula)
- **Hierarchical Context Delivery**: OpenViking, Meilisearch hierarchical facets, Algolia
- **Retrieval Rerankers & Query Decay**: https://medium.com/@Praxen/retrieval-rerankers-10-evals-that-expose-query-decay-9aba218d20ff

### Neo4j & Graph Databases
- **Neo4j Graph Type** (Schema enforcement 2026): https://neo4j.com/docs/cypher-manual/current/schema/graph-types/
- **Cypher: An Evolving Query Language for Property Graphs** (SIGMOD 2018): https://dl.acm.org/doi/10.1145/3183713.3190657
- **OpenCypher Specification**: https://github.com/opencypher/openCypher

### Heatmap & Thermal Memory
- **Existing MiMi Specs**: `.planning/specs/HEATMAP-ALGORITHM.md`
- **Temperature Decay Formula**: `T(t) = T₀ × e^(-λ × age)`

---

## Questions for Next Phase

1. **Vector Embeddings**: What embedding model should Pandora use for semantic similarity? (e.g., OpenAI embeddings, local sentence-transformers, custom)
2. **Query Performance**: Should we pre-compute query execution plans for common templates, or generate on-demand?
3. **Caching Strategy**: Should refined queries (cypher + params) be cached in Redis, or always regenerated?
4. **Negative Context Management**: How long should antipatterns stay in the "avoid" list? (suggestion: 7 days with exponential decay)
5. **Cost Estimation Accuracy**: Should we use Neo4j EXPLAIN/PROFILE, or rough heuristics based on pattern matching?

---

**Next Action**: Begin Phase 1 implementation (GraphSchema class + validation tests).
