# Module: Echidna — Skills Planner & Evolution Engine

> **Module ID:** `echidna-lab`  
> **Language:** Rust  
> **Status:** 🟡 Planned (M4)  
> **Purpose:** Autonomous skill generation via pattern detection and code synthesis  

---

## 1. Module Overview

**Echidna** is the evolution engine of MiMi — the module responsible for transforming repetitive tasks into reusable, executable skills. Unlike conventional automation frameworks that require manual scripting, Echidna **autonomously detects patterns**, **generates code**, and **deploys validated skills** that expand MiMi's capabilities over time.

### Core Capabilities

1. **Pattern Detection** — Analyzes task history from Pandora to identify repetitive workflows
2. **Code Generation** — Produces template-based Rhai scripts for simple automations
3. **WASM Compilation** — Compiles complex skills into sandboxed WebAssembly binaries
4. **Skill Registration** — Stores validated skills in Pandora as reusable ContextNodes
5. **Automability Scoring** — Quantifies how suitable a pattern is for automation (0.0–1.0 scale)

### Why Echidna?

- **Self-Evolution:** MiMi becomes more capable with each repetition
- **Zero Manual Effort:** Skills emerge from usage patterns automatically
- **Performance:** Rhai skills execute in <100ms, WASM skills in <500ms
- **Safety:** All generated code passes Odlaguna validation before deployment

---

## 2. Architecture

Echidna is structured as a **pipeline architecture** with four primary stages:

```
┌──────────────────────────────────────────────────────────────┐
│                      ECHIDNA PIPELINE                        │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─────────────┐   ┌──────────────┐   ┌─────────────┐      │
│  │  Pattern    │──▶│  Code        │──▶│  Compiler   │      │
│  │  Detector   │   │  Generator   │   │  (WASM)     │      │
│  └─────────────┘   └──────────────┘   └─────────────┘      │
│        │                   │                   │            │
│        └───────────────────┴───────────────────┘            │
│                            │                                │
│                   ┌────────▼────────┐                       │
│                   │  Skill Registry │                       │
│                   │  (Validation +  │                       │
│                   │   Pandora)      │                       │
│                   └─────────────────┘                       │
└──────────────────────────────────────────────────────────────┘
                            │
                            ▼
                    ┌───────────────┐
                    │     Ryzu      │
                    │  (Execution)  │
                    └───────────────┘
```

### Internal Components

| Component | Responsibility | Input | Output |
|-----------|----------------|-------|--------|
| **Pattern Detector** | Identify repetitive task clusters | Task history (Neo4j) | SkillCandidate list |
| **Rhai Generator** | Template-based script synthesis | SkillCandidate + samples | Rhai source code |
| **WASM Compiler** | Compile Rust → WASM binary | Rust source | `.wasm` binary |
| **Skill Registry** | Validation + storage | Skill (code/binary) | Registered skill ID |

---

## 3. API/Interfaces

### Message Bus Topics

Echidna subscribes and publishes to the following Message Bus topics:

| Topic | Direction | Payload | Purpose |
|-------|-----------|---------|---------|
| `skill/create` | Subscribe | `{ pattern_id: String }` | Triggered when Echidna detects automatable pattern |
| `skill/review` | Publish → Odlaguna | `{ skill_id: String, code: String }` | Submit skill for validation |
| `skill/deploy` | Subscribe | `{ validation_result: ValidationResult }` | Deploy validated skill to Pandora |
| `task/history_query` | Publish → Pandora | `{ query: Cypher, limit: u32 }` | Fetch recent tasks for pattern detection |

### Pandora Queries

**Fetch Recent Tasks:**
```cypher
MATCH (t:Task)
WHERE t.status = "completed"
RETURN t ORDER BY t.completed_at DESC LIMIT $limit
```

**Register Skill:**
```cypher
CREATE (s:Skill {
  id: $skill_id,
  name: $name,
  version: $version,
  language: $language,
  created_by: "echidna",
  created_at: datetime(),
  source_code: $code,
  validation_status: "passed",
  execution_count: 0,
  success_rate: 1.0,
  automability_score: $automability
})
```

**Link Skills (Evolution):**
```cypher
MATCH (s:Skill {id: $new_skill_id})
MATCH (existing:Skill)
WHERE existing.language = s.language 
  AND similarity(existing.name, s.name) > 0.8
CREATE (s)-[:EVOLVED_FROM]->(existing)
```

---

## 4. Key Algorithms

### Pattern Detection (Clustering)

**Algorithm:** Sliding Window + Fingerprint-Based Grouping

1. Query Pandora for last 1000 completed tasks
2. Compute **fingerprint** for each task:
   - Strip parameters (e.g., `convert_jpg_to_png` → `convert_<format>_to_<format>`)
   - Hash intent type + action structure
3. Group tasks by fingerprint using HashMap
4. Filter groups with ≥ `min_repetitions` (default: 5)
5. Calculate **automability score** for each cluster

### Automability Calculation

**Score Formula:**
```
automability = (determinism × 0.5) + (io_cost × 0.3) + ((1 - complexity) × 0.2)
```

**Thresholds:**
- **High (> 0.8):** Generate Rhai script (simple)
- **Medium (0.5–0.8):** Generate WASM binary (complex)
- **Low (< 0.5):** Ignore (not automatable)

**Determinism Score:**
- 1.0 if task always produces same output for same input
- 0.5 if output varies occasionally
- 0.0 if non-deterministic (e.g., random number generation)

**I/O Cost Score:**
- 1.0 for I/O-bound tasks (file processing, API calls)
- 0.5 for balanced tasks
- 0.0 for CPU-bound tasks

**Complexity Score:**
- 0.0 for simple linear workflows (< 5 steps)
- 0.5 for moderate branching
- 1.0 for complex multi-branch workflows (> 10 steps)

### Code Generation Templates

**Rhai Template (Simple Task):**
```rhai
// Auto-generated by Echidna at {{ timestamp }}
// Pattern: {{ pattern_description }}
// Automability: {{ automability_score }}

fn execute(params) {
    let mut result = #{};
    
    {% for input in inputs %}
    let {{ input }} = params["{{ input }}"];
    {% endfor %}
    
    // Core logic
    {% for step in steps %}
    {{ step.code }}
    {% endfor %}
    
    result["output"] = final_value;
    return result;
}

// Entry point
let params = engine_params();
let result = execute(params);
print(result);
```

**WASM Template (Complex Task):**
```rust
// Auto-generated by Echidna
// Pattern: {{ pattern_description }}

#[no_mangle]
pub extern "C" fn execute(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input = unsafe {
        std::slice::from_raw_parts(input_ptr, input_len)
    };
    
    let params: Params = serde_json::from_slice(input).unwrap();
    let result = process(params);
    
    let output = serde_json::to_vec(&result).unwrap();
    Box::into_raw(output.into_boxed_slice()) as *const u8
}

fn process(params: Params) -> Result {
    // {{ core_logic }}
}
```

---

## 5. Dependencies

### Internal Dependencies

| Module | Reason | Interface |
|--------|--------|-----------|
| **Pandora** | Read task history, store skills | Cypher queries via Message Bus |
| **Odlaguna** | Validate generated code | `skill/review` topic |
| **Ryzu** | Execute skills in isolated containers | `task/execute` topic |

### External Dependencies

| Crate/Tool | Version | Purpose |
|------------|---------|---------|
| `rhai` | ^1.15 | Embedded scripting engine |
| `wasmtime` | ^12.0 | WASM runtime |
| `askama` / `tera` | ^0.12 | Template rendering for code generation |
| `neo4j` | ^0.4 | Bolt driver for Pandora queries |
| `uuid` | ^1.4 | Unique skill IDs |
| `serde_json` | ^1.0 | Serialization |

---

## 6. Data Structures

### SkillCandidate

```rust
pub struct SkillCandidate {
    pub id: String,                     // UUID
    pub name: String,                   // e.g., "skill_auto_jpg_convert"
    pub pattern_description: String,    // Human-readable description
    pub automability_score: f64,        // 0.0 to 1.0
    pub repetition_count: usize,        // How many times pattern appeared
    pub estimated_time_saved: f64,      // Seconds saved per execution
    pub suggested_language: SkillLanguage, // Rhai or WASM
    pub sample_tasks: Vec<Task>,        // Representative tasks for training
}
```

### Skill

```rust
pub struct Skill {
    pub id: String,                     // UUID
    pub name: String,                   // Unique skill name
    pub version: String,                // Semantic version (e.g., "1.0.0")
    pub language: SkillLanguage,        // Rhai or WASM
    pub code: String,                   // Source code (Rhai) or Rust
    pub binary: Option<Vec<u8>>,        // Compiled WASM binary
    pub metadata: SkillMetadata,
    pub validation_result: ValidationResult,
}
```

### SkillLanguage

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SkillLanguage {
    Rhai,  // < 100ms execution, simple logic, no I/O
    Wasm,  // < 500ms execution, complex logic, I/O allowed
}
```

### Pattern

```rust
pub struct Pattern {
    pub fingerprint: String,            // Hash of task structure
    pub tasks: Vec<Task>,               // All tasks matching this pattern
    pub common_steps: Vec<Step>,        // Extracted common workflow
    pub variability: f64,               // 0.0 (identical) to 1.0 (highly variable)
}
```

### SkillMetadata

```rust
pub struct SkillMetadata {
    pub created_by: String,             // "echidna"
    pub created_at: DateTime<Utc>,
    pub repetitions_detected: usize,    // Original repetition count
    pub automability_score: f64,
    pub estimated_time_saved: f64,      // Per execution (seconds)
    pub execution_count: u64,           // Total invocations since creation
    pub success_rate: f64,              // 0.0 to 1.0
}
```

---

## 7. Integration Points

### Reads From

**Pandora (Task History):**
- Query: `MATCH (t:Task) WHERE t.status = "completed" RETURN t`
- Frequency: Every 5 minutes (configurable)
- Trigger: Pattern detector cron job

**Pandora (Existing Skills):**
- Query: `MATCH (s:Skill) RETURN s`
- Purpose: Avoid duplicate skill generation

### Writes To

**Odlaguna (Validation Request):**
- Topic: `skill/review`
- Payload: `{ skill_id, code, language }`
- Response: `ValidationResult { is_valid, issues[] }`

**Pandora (Skill Registration):**
- Topic: `skill/deploy`
- Payload: `Skill` struct
- Creates: `:Skill` node + `[:EVOLVED_FROM]` relationships

### Invokes

**Ryzu (Skill Execution):**
- Topic: `task/execute`
- Payload: `{ skill_id, params, timeout_ms }`
- Response: Execution result

---

## 8. Error Handling

### Invalid Pattern Detection

**Error:** Pattern detected but automability score < 0.5

**Response:**
- Log pattern to `echidna-rejected-patterns.log`
- Do not generate skill
- Optionally notify Mimi if pattern is borderline (0.45–0.5)

### Code Generation Failure

**Error:** Template rendering fails or generated code has syntax errors

**Response:**
- Rollback to previous skill version (if exists)
- Send alert to Message Bus: `skill/generation_failed`
- Human review required (flag in Pandora)

### Validation Rejection (Odlaguna)

**Error:** Odlaguna rejects skill due to security violations

**Response:**
- Mark skill as `validation_failed` in Pandora
- Block deployment
- Log detailed rejection reason
- Optionally auto-tune code generator to avoid similar issues

### Compilation Errors (WASM)

**Error:** `rustc` fails to compile skill to WASM

**Response:**
- Capture stderr output
- Store error in `compilation_errors.log`
- Fallback to Rhai version (if applicable)
- Alert developer if error persists for > 3 attempts

---

## 9. Performance Characteristics

### Latency Targets

| Operation | Target | Measured At |
|-----------|--------|-------------|
| Pattern Detection | < 1s | Per 1000-task batch |
| Rhai Code Generation | < 5s | Per skill candidate |
| WASM Compilation | < 10s | Per skill (optimized build) |
| Skill Execution (Rhai) | < 100ms | Per invocation |
| Skill Execution (WASM) | < 500ms | Per invocation |

### Caching Effects

**Skill Cache (Ryzu):**
- Hit rate target: > 80%
- Cache size: 50MB (in-memory LRU)
- Eviction policy: Least-recently-used + least-successful

**Compiled WASM Cache:**
- Store in Pandora as binary blob
- No recompilation on subsequent loads
- Versioned: only recompile on code change

### Throughput

- **Pattern Detection:** 1000 tasks/second (single-threaded)
- **Concurrent Skill Executions:** 100+ (via Ryzu worker pool)
- **Registry Operations:** 500 skills/second (Neo4j write)

---

## 10. Testing Strategy

### Pattern Detection Tests

**Unit Tests:**
- Fingerprint computation is deterministic
- Clustering groups similar tasks correctly
- Automability score calculation matches expected values

**Integration Tests:**
- Query Pandora for real task history
- Verify candidate generation from live data
- Test with synthetic task datasets (known patterns)

**Edge Cases:**
- Empty task history
- All tasks are unique (no patterns)
- Tasks with missing fields

### Code Generation Quality Tests

**Unit Tests:**
- Template rendering produces valid Rhai/Rust
- Variable substitution works correctly
- Generated code passes syntax validation

**Quality Metrics:**
- Generated Rhai compiles without errors
- Generated WASM compiles with `rustc`
- Code passes Odlaguna whitelist checks

**Human Review:**
- Sample 10% of generated skills for manual inspection
- Check for logical correctness, readability
- Maintain quality score in Pandora

### Compilation Verification

**WASM Tests:**
- Binary size < 1MB
- No forbidden imports (network, filesystem)
- Executes under Wasmtime without crashes
- Respects fueling limits (instruction count)

**Performance Tests:**
- Benchmark execution time (< 500ms)
- Memory usage < 256MB
- No memory leaks (Valgrind/ASAN)

---

## 11. Future Extensions

### Multi-Language Generation (M5+)

Currently, Echidna generates **Rhai** (simple) and **WASM** (complex). Future extensions:

- **Python Skills:** For data science workflows
- **JavaScript Skills:** For web automation
- **SQL Skills:** For database query patterns

**Design Consideration:**
- Add `SkillLanguage::Python`, `SkillLanguage::JavaScript`
- Extend Odlaguna validation to support new languages
- Sandboxing via Pyodide (Python), Deno (JS)

### Reinforcement Learning from Results (M6+)

**Problem:** Some generated skills fail in production despite passing validation.

**Solution:**
- Track skill success rate in `SkillMetadata`
- If success rate < 0.8 after 10 executions, trigger **skill refinement**
- Use execution logs as training data for code generator
- Auto-generate improved version (v1.1, v1.2, etc.)

**Algorithm:**
```
1. Detect failing skill (success_rate < 0.8)
2. Query Pandora for execution logs
3. Extract failure patterns (input/output mismatches)
4. Re-run code generator with failure feedback
5. Deploy v1.1 → retry
```

### Skill Marketplace (M7+)

**Vision:** Skills become shareable across MiMi instances

- Export skills as `.skill` files (JSON + WASM blob)
- Cryptographic signing (verify author)
- Import skills from community repository
- Reputation system (upvote/downvote)

**Security:**
- All imported skills pass Odlaguna validation
- Human review required for new sources
- Sandboxed execution always

---

## Skill Lifecycle Diagram

```
┌───────────────────────────────────────────────────────────┐
│                   SKILL LIFECYCLE                         │
└───────────────────────────────────────────────────────────┘

    User executes task N times
            │
            ▼
    ┌────────────────┐
    │ Pattern Detector│  ──▶ [Automability Score < 0.5]
    │ (Echidna)      │       └─▶ REJECT (log + skip)
    └────────────────┘
            │
            │ [Score ≥ 0.5]
            ▼
    ┌────────────────┐
    │ Code Generator │  ──▶ Rhai (simple) or WASM (complex)
    └────────────────┘
            │
            ▼
    ┌────────────────┐
    │ Odlaguna       │  ──▶ [Validation Failed]
    │ (Validation)   │       └─▶ REJECT (log + alert)
    └────────────────┘
            │
            │ [Validation Passed]
            ▼
    ┌────────────────┐
    │ Skill Registry │  ──▶ Store in Pandora (Neo4j)
    │ (Pandora)      │
    └────────────────┘
            │
            ▼
    ┌────────────────┐
    │ Ryzu Execution │  ──▶ Cache in LRU (Ryzu)
    │ (Docker/WASM)  │
    └────────────────┘
            │
            │ [Success Rate > 0.8]
            ▼
    ┌────────────────┐
    │ Skill Active   │  ──▶ Next task invokes skill
    │ (Reusable)     │       (no manual action)
    └────────────────┘
            │
            │ [Success Rate < 0.8]
            ▼
    ┌────────────────┐
    │ Skill Refinement│ ──▶ Re-generate v1.1
    │ (Future: RL)   │      (M6+ feature)
    └────────────────┘
```

---

## Skill Metadata Schema

**Stored in Pandora (Neo4j):**

```cypher
(:Skill {
  id: "550e8400-e29b-41d4-a716-446655440000",
  name: "convert_image_format",
  version: "1.0.0",
  language: "Rhai",
  created_by: "echidna",
  created_at: "2026-04-16T14:30:00Z",
  source_code: "fn execute(params) { ... }",
  validation_status: "passed",
  execution_count: 127,
  success_rate: 0.94,
  automability_score: 0.87,
  estimated_time_saved: 8.5,
  binary_blob: null  // Only for WASM skills
})
```

**Relationships:**
- `(:Skill)-[:EVOLVED_FROM]->(:Skill)` — Version lineage
- `(:Skill)-[:GENERATED_BY]->(:Pattern)` — Source pattern
- `(:Task)-[:USES]->(:Skill)` — Execution tracking

---

## Cross-References

- **Requirements:** [`REQUIREMENTS.md#RF-4`](../REQUIREMENTS.md#rf-4-criação-dinâmica-de-skills-echidna)
- **Milestone:** [`milestones/M4-ECHIDNA.md`](../milestones/M4-ECHIDNA.md)
- **Lifecycle Spec:** [`specs/SKILL-LIFECYCLE.md`](../specs/SKILL-LIFECYCLE.md) (to be created)
- **Dependencies:** [`modules/PANDORA.md`](PANDORA.md), [`modules/ODLAGUNA.md`](ODLAGUNA.md), [`modules/RYZU.md`](RYZU.md)

---

**End of Echidna Module Design**
