# MIMI-COMMANDER — Core Orchestrator Module

> **Module:** Mimi Core Orchestrator  
> **Language:** Rust  
> **Milestone:** M1 (Foundation) — Critical Path  
> **Requirements:** RF-1.1, RF-1.2, RF-1.3, RF-1.4  
> **Status:** 🟡 Design Complete — Implementation Pending  

---

## 1. Module Overview

**Mimi** (Multimodal Instruction Master Interface) is the **central orchestrator** and cognitive coordinator of the MiMi system. It acts as the decision-making hub that receives structured intents from Beatrice, routes tasks to specialized modules (Pandora, Echidna, Ryzu), and manages system-wide state.

### Responsibilities

| Responsibility | Description |
|----------------|-------------|
| **Intent Reception** | Consume `Intent` messages from Message Bus topic `intent/structured` |
| **Task Routing** | Decide which module handles each task based on intent type, context availability, and module capabilities |
| **State Management** | Maintain in-memory state machine tracking active tasks, leases, and module health |
| **Priority Scheduling** | Order task execution by priority (HIGH, MEDIUM, LOW) and deadline constraints |
| **Supervision Coordination** | Report to Odlaguna for watchdog monitoring and request execution authorization |
| **Context Queries** | Invoke Pandora to fetch relevant context before delegating to AI adapters or skill execution |
| **Telemetry & Logging** | Emit structured logs and metrics for observability |

### Role in System

Mimi is the **message-driven brain** of the system:

- **Does NOT execute tasks directly** — delegates to Ryzu workers or AI adapters
- **Does NOT store long-term memory** — queries Pandora for context retrieval
- **Does NOT generate skills** — requests Echidna to create new tools when needed
- **DOES orchestrate** — coordinates, routes, prioritizes, and supervises all operations

```
User Input (NL) → Beatrice → Intent → [MIMI] → Task → Specialized Module → Result
                                        ↓
                                     Pandora (Context)
                                        ↓
                                     Odlaguna (Watchdog)
```

---

## 2. Architecture

### State Machine

Mimi operates as an **async event-driven state machine** with the following states:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimiState {
    Idle,               // Waiting for next Intent
    ContextFetch,       // Querying Pandora for context
    TaskRouting,        // Deciding which module handles task
    WaitingApproval,    // Awaiting Odlaguna authorization (M3+)
    Executing,          // Task dispatched to worker/adapter
    Aggregating,        // Collecting results from multiple tasks
    Responding,         // Sending final response via Bus
    Error(ErrorKind),   // Recoverable error state
}

#[derive(Debug)]
pub enum ErrorKind {
    ContextTimeout,      // Pandora didn't respond
    NoRouteFound,        // No module can handle intent
    ExecutionFailure,    // Worker crashed or timed out
    BusDisconnect,       // Message Bus unavailable
}
```

### Internal Components

```
┌─────────────────────────────────────────────────────────────┐
│                       MimiCore                              │
├─────────────────────────────────────────────────────────────┤
│  ┌───────────────┐  ┌──────────────┐  ┌─────────────────┐  │
│  │ Message Router│  │ State Machine│  │ Priority Queue  │  │
│  │ (Topic Match) │  │ (FSM)        │  │ (Binary Heap)   │  │
│  └───────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
│          │                  │                   │           │
│  ┌───────▼──────────────────▼───────────────────▼────────┐  │
│  │            Task Executor (Tokio Runtime)              │  │
│  └───────────────────────────┬───────────────────────────┘  │
│                              │                              │
│  ┌───────────────────────────▼───────────────────────────┐  │
│  │         Connection Pool (Bus Clients)                 │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
           │                    │                    │
      ┌────▼────┐         ┌─────▼─────┐       ┌─────▼──────┐
      │ Pandora │         │ Odlaguna  │       │   Ryzu     │
      │(Context)│         │(Watchdog) │       │ (Workers)  │
      └─────────┘         └───────────┘       └────────────┘
```

### Message Router

Routes incoming messages to appropriate handlers based on topic:

| Topic | Handler | Purpose |
|-------|---------|---------|
| `intent/structured` | `handle_intent()` | New Intent from Beatrice |
| `context/response` | `handle_context()` | Context from Pandora |
| `task/result` | `handle_result()` | Task completion from Ryzu |
| `task/failed` | `handle_failure()` | Task failure notification |
| `system/health_check` | `handle_health()` | Liveness probe from Odlaguna |

### Priority Queue

Uses Rust's `BinaryHeap` with custom `Ord` implementation:

```rust
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub intent: Intent,
    pub priority: Priority,
    pub deadline: Instant,
    pub context: Option<ContextNode>,
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier deadline
        self.priority.cmp(&other.priority)
            .then_with(|| other.deadline.cmp(&self.deadline))
    }
}
```

---

## 3. API/Interfaces

### Input: Intent Message (from Beatrice)

**Topic:** `intent/structured`  
**Format:** FlatBuffers  
**Schema:**

```fbs
// proto/intent.fbs
namespace mimi.proto;

enum IntentType : byte {
    Query = 0,              // Information retrieval
    Command = 1,            // Execute action
    CreateSkill = 2,        // Request new skill from Echidna
    Memory = 3,             // Store/retrieve from Pandora
}

enum Priority : byte {
    LOW = 0,
    MEDIUM = 1,
    HIGH = 2,
}

table Intent {
    id: string;                     // UUID
    user_message: string;           // Original natural language input
    intent_type: IntentType;
    entities: [Entity];             // Extracted entities (name, type, value)
    confidence: float;              // 0.0-1.0 from Beatrice NLP
    priority: Priority;
    timestamp: uint64;              // Unix epoch milliseconds
    session_id: string;             // For multi-turn conversations
}

table Entity {
    name: string;
    entity_type: string;            // e.g., "file", "url", "person"
    value: string;
}
```

### Output: Task Dispatch (to Modules)

**Topic:** `task/execute`  
**Format:** FlatBuffers  
**Schema:**

```fbs
// proto/task.fbs
namespace mimi.proto;

table Task {
    id: string;                     // Task UUID
    intent_id: string;              // Reference to original Intent
    target_module: string;          // "pandora", "echidna", "ryzu"
    action: string;                 // Module-specific action
    parameters: [KeyValue];         // Flexible key-value pairs
    context: [ubyte];               // Serialized ContextNode from Pandora
    lease_ms: uint32;               // Max execution time (Odlaguna enforced)
    priority: Priority;
    created_at: uint64;
}

table KeyValue {
    key: string;
    value: string;
}
```

### Message Bus Topics

| Direction | Topic | Subscriber | Purpose |
|-----------|-------|------------|---------|
| ← IN | `intent/structured` | Mimi | New intents from Beatrice |
| ← IN | `context/response` | Mimi | Context data from Pandora |
| ← IN | `task/result` | Mimi | Success results from Ryzu |
| ← IN | `task/failed` | Mimi | Failure notifications |
| → OUT | `context/query` | Pandora | Request relevant context |
| → OUT | `task/execute` | Ryzu/Echidna | Dispatch task to worker |
| → OUT | `response/final` | Beatrice | Final response to user |
| ↔ BOTH | `system/health_check` | Odlaguna | Liveness monitoring |

### FFI Interface (M2+)

In Milestone 2, Mimi will directly call Pandora via FFI for low-latency context queries:

```rust
// mimi-commander/src/ffi.rs
#[link(name = "pandora", kind = "static")]
extern "C" {
    fn pandora_query_context(
        query: *const c_char,
        max_nodes: u32,
        out_buffer: *mut u8,
        buffer_size: u32,
    ) -> i32;
}
```

**Latency Target:** < 5ms for FFI call + Neo4j query

---

## 4. Key Algorithms

### State Transition Logic

```rust
impl MimiCore {
    async fn process_intent(&mut self, intent: Intent) -> Result<(), MimiError> {
        self.state = MimiState::ContextFetch;
        
        // Step 1: Query Pandora for context
        let context = match self.fetch_context(&intent).await {
            Ok(ctx) => ctx,
            Err(e) if e.is_timeout() => {
                warn!("Context fetch timeout, proceeding without context");
                None
            }
            Err(e) => {
                self.state = MimiState::Error(ErrorKind::ContextTimeout);
                return Err(e.into());
            }
        };
        
        // Step 2: Route to appropriate module
        self.state = MimiState::TaskRouting;
        let task = self.route_task(intent, context)?;
        
        // Step 3: Enqueue by priority
        self.task_queue.push(task.clone());
        
        // Step 4: Dispatch highest-priority task
        self.state = MimiState::Executing;
        self.dispatch_task(task).await?;
        
        // Step 5: Wait for result (async)
        self.state = MimiState::Idle;
        Ok(())
    }
}
```

### Routing Decision Tree

```rust
fn route_task(&self, intent: Intent, context: Option<ContextNode>) -> Result<Task, MimiError> {
    let target_module = match intent.intent_type {
        IntentType::Query => {
            if context.is_some() {
                "ai_adapter"  // Use LLM with context
            } else {
                "pandora"     // Need more context first
            }
        }
        IntentType::Command => {
            if self.has_skill_for(&intent)? {
                "ryzu"        // Execute existing skill
            } else {
                "echidna"     // Create new skill
            }
        }
        IntentType::CreateSkill => "echidna",
        IntentType::Memory => "pandora",
    };
    
    Ok(Task {
        id: TaskId::new(),
        target_module: target_module.into(),
        // ... rest of Task construction
    })
}
```

### Priority Scheduling

```rust
async fn dispatch_next_task(&mut self) {
    while let Some(task) = self.task_queue.pop() {
        if task.deadline < Instant::now() {
            warn!("Task {} expired, skipping", task.id);
            continue;
        }
        
        match self.dispatch_task(task).await {
            Ok(_) => break,
            Err(e) => {
                error!("Failed to dispatch task: {}", e);
                // Retry with exponential backoff (up to 3 attempts)
            }
        }
    }
}
```

---

## 5. Dependencies

### Runtime Dependencies

| Dependency | Purpose | Version | Criticality |
|------------|---------|---------|-------------|
| **Message Bus** | Communication backbone | Zenoh 0.11+ / NATS 2.x | 🔴 Critical |
| **Pandora** | Context retrieval | C++ FFI (M2+) | 🟡 High |
| **Odlaguna** | Watchdog supervision | Message Bus only | 🟢 Medium |
| **Beatrice** | Intent source | Message Bus only | 🟡 High |

### Rust Crate Dependencies

```toml
[dependencies]
tokio = { version = "1.40", features = ["full"] }
zenoh = "0.11"                    # Or nats = "0.26"
flatbuffers = "24.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.10", features = ["v4"] }
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
thiserror = "1.0"

[dev-dependencies]
tokio-test = "0.4"
criterion = "0.5"                 # Benchmarking
```

### Build Dependencies

- Rust 1.70+ (stable)
- FlatBuffers compiler (`flatc`)
- Docker (for Message Bus broker)

---

## 6. Data Structures

### Core Types

```rust
// mimi-commander/src/types.rs

use uuid::Uuid;
use std::time::{Duration, Instant};

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Medium = 1,
    High = 2,
}

/// Intent structure (deserialized from FlatBuffers)
#[derive(Debug, Clone)]
pub struct Intent {
    pub id: String,
    pub user_message: String,
    pub intent_type: IntentType,
    pub entities: Vec<Entity>,
    pub confidence: f32,
    pub priority: Priority,
    pub timestamp: u64,
    pub session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentType {
    Query,
    Command,
    CreateSkill,
    Memory,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub entity_type: String,
    pub value: String,
}

/// Task ready for execution
#[derive(Debug, Clone)]
pub struct Task {
    pub id: TaskId,
    pub intent_id: String,
    pub target_module: String,
    pub action: String,
    pub parameters: Vec<(String, String)>,
    pub context: Option<Vec<u8>>,  // Serialized ContextNode
    pub lease: Duration,
    pub priority: Priority,
    pub deadline: Instant,
}

/// Lease tracking for Odlaguna
#[derive(Debug, Clone)]
pub struct LeaseInfo {
    pub task_id: TaskId,
    pub module: String,
    pub started_at: Instant,
    pub deadline: Instant,
    pub lease_duration: Duration,
}

impl LeaseInfo {
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.deadline
    }
    
    pub fn remaining(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }
}

/// Module health status
#[derive(Debug, Clone)]
pub struct ModuleHealth {
    pub module_name: String,
    pub status: HealthStatus,
    pub last_heartbeat: Instant,
    pub failure_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

### State Management

```rust
// mimi-commander/src/state.rs

use std::collections::{HashMap, BinaryHeap};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MimiState {
    /// Current FSM state
    pub current_state: Arc<RwLock<MimiStateEnum>>,
    
    /// Priority queue of pending tasks
    pub task_queue: Arc<RwLock<BinaryHeap<Task>>>,
    
    /// Active leases (task_id → lease_info)
    pub active_leases: Arc<RwLock<HashMap<TaskId, LeaseInfo>>>,
    
    /// Module health tracking
    pub module_health: Arc<RwLock<HashMap<String, ModuleHealth>>>,
    
    /// Message Bus connection pool
    pub bus_client: Arc<BusClient>,
    
    /// Metrics collector
    pub metrics: Arc<MetricsCollector>,
}

impl MimiState {
    pub fn new(bus_client: BusClient) -> Self {
        Self {
            current_state: Arc::new(RwLock::new(MimiStateEnum::Idle)),
            task_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            active_leases: Arc::new(RwLock::new(HashMap::new())),
            module_health: Arc::new(RwLock::new(HashMap::new())),
            bus_client: Arc::new(bus_client),
            metrics: Arc::new(MetricsCollector::new()),
        }
    }
}
```

---

## 7. Integration Points

### How Other Modules Invoke Mimi

Mimi is **message-driven** — other modules do not call Mimi directly. Instead:

1. **Beatrice** publishes `Intent` to topic `intent/structured`
2. **Pandora** publishes context responses to `context/response`
3. **Ryzu** publishes task results to `task/result` or `task/failed`
4. **Odlaguna** sends health checks to `system/health_check`

### How Mimi Invokes Other Modules

#### To Pandora (Context Query)

```rust
async fn fetch_context(&self, intent: &Intent) -> Result<Option<ContextNode>, MimiError> {
    let query = ContextQuery {
        query_text: intent.user_message.clone(),
        session_id: intent.session_id.clone(),
        max_nodes: 100,
        heat_threshold: 0.3,
    };
    
    let response = self.bus_client
        .request("context/query", &query)
        .timeout(Duration::from_millis(50))  // 50ms timeout
        .await?;
    
    Ok(deserialize_context(response))
}
```

#### To Ryzu (Task Execution)

```rust
async fn dispatch_task(&self, task: Task) -> Result<(), MimiError> {
    // Register lease with Odlaguna
    let lease = LeaseInfo {
        task_id: task.id,
        module: task.target_module.clone(),
        started_at: Instant::now(),
        deadline: Instant::now() + task.lease,
        lease_duration: task.lease,
    };
    
    self.bus_client.publish("system/lease_register", &lease).await?;
    
    // Dispatch task
    self.bus_client.publish("task/execute", &task).await?;
    
    // Track active lease
    self.active_leases.write().await.insert(task.id, lease);
    
    Ok(())
}
```

#### To Beatrice (Final Response)

```rust
async fn send_response(&self, intent_id: &str, result: TaskResult) -> Result<(), MimiError> {
    let response = FinalResponse {
        intent_id: intent_id.to_string(),
        status: if result.success { "completed" } else { "failed" },
        message: result.output,
        metadata: result.metadata,
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64,
    };
    
    self.bus_client.publish("response/final", &response).await?;
    
    Ok(())
}
```

---

## 8. Error Handling

### Failure Modes

| Failure Mode | Cause | Recovery Strategy |
|-------------|-------|-------------------|
| **Context Timeout** | Pandora slow/unresponsive | Proceed without context after 50ms |
| **No Route Found** | Unknown intent type | Return error to Beatrice with suggestion |
| **Task Execution Failure** | Worker crash/exception | Retry up to 3 times with exponential backoff |
| **Bus Disconnect** | Zenoh/NATS broker down | Reconnect with circuit breaker (max 5 retries) |
| **Lease Expiry** | Task exceeded deadline | Odlaguna sends SIGKILL, Mimi logs and cleans up |
| **Queue Overflow** | Too many pending tasks | Drop LOW priority tasks, alert Odlaguna |

### Error Types

```rust
// mimi-commander/src/error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum MimiError {
    #[error("Context query timeout: {0}")]
    ContextTimeout(String),
    
    #[error("No route found for intent type: {0:?}")]
    NoRoute(IntentType),
    
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),
    
    #[error("Message Bus disconnected")]
    BusDisconnect,
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] flatbuffers::InvalidFlatbuffer),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Task queue full (capacity: {0})")]
    QueueOverflow(usize),
}
```

### Recovery Strategies

```rust
impl MimiCore {
    async fn handle_error(&mut self, error: MimiError) {
        match error {
            MimiError::ContextTimeout(_) => {
                // Non-critical: proceed without context
                warn!("{}", error);
                self.state = MimiState::TaskRouting;
            }
            
            MimiError::BusDisconnect => {
                // Critical: attempt reconnect
                error!("{}", error);
                self.reconnect_bus().await;
            }
            
            MimiError::QueueOverflow(capacity) => {
                // Drop LOW priority tasks
                self.evict_low_priority_tasks().await;
                warn!("Queue overflow ({}), evicted low-priority tasks", capacity);
            }
            
            _ => {
                error!("Unhandled error: {}", error);
                self.state = MimiState::Error(error.into());
            }
        }
    }
}
```

---

## 9. Performance Characteristics

### Latency Targets

| Operation | Target | Measurement | Criticality |
|-----------|--------|-------------|-------------|
| **Intent → Task Routing** | < 1ms | 95th percentile | 🔴 Critical |
| **Context Query (Message Bus)** | < 50ms | 99th percentile | 🟡 High |
| **Context Query (FFI, M2+)** | < 5ms | 99th percentile | 🔴 Critical |
| **Task Dispatch** | < 1ms | 95th percentile | 🟡 High |
| **State Transition** | < 100μs | Mean | 🟢 Medium |
| **Health Check Response** | < 500μs | 99th percentile | 🟡 High |

### Throughput Targets

- **Messages/sec:** 100+ (M1), 500+ (M2+)
- **Concurrent Tasks:** 50+ (M1), 200+ (M2+)
- **Queue Depth:** 1000 tasks max

### Resource Usage

| Resource | M1 Target | M2+ Target | Notes |
|----------|-----------|-----------|-------|
| **Memory (RSS)** | < 50 MB | < 200 MB | Depends on queue depth |
| **CPU (Idle)** | < 1% | < 2% | Event-driven, not polling |
| **CPU (Peak)** | < 30% | < 50% | During burst workload |
| **File Descriptors** | < 100 | < 500 | Message Bus connections |
| **Network I/O** | < 10 MB/s | < 100 MB/s | Depends on message volume |

### Optimization Strategies

1. **Zero-Copy Serialization:** Use FlatBuffers to avoid allocation overhead
2. **Connection Pooling:** Reuse Message Bus clients across tasks
3. **Async I/O:** Tokio runtime ensures non-blocking operations
4. **Lock-Free Queues:** Minimize contention on task queue (consider `crossbeam`)
5. **Batch Processing:** Group multiple low-priority tasks when possible (M2+)

---

## 10. Testing Strategy

### Unit Tests

```rust
// mimi-commander/tests/routing_tests.rs

#[tokio::test]
async fn test_route_query_intent_with_context() {
    let intent = Intent {
        intent_type: IntentType::Query,
        // ... other fields
    };
    let context = Some(mock_context_node());
    
    let core = MimiCore::new(mock_bus_client());
    let task = core.route_task(intent, context).unwrap();
    
    assert_eq!(task.target_module, "ai_adapter");
}

#[tokio::test]
async fn test_priority_queue_ordering() {
    let mut queue = BinaryHeap::new();
    
    queue.push(Task { priority: Priority::Low, deadline: Instant::now() + Duration::from_secs(10), ..default() });
    queue.push(Task { priority: Priority::High, deadline: Instant::now() + Duration::from_secs(5), ..default() });
    queue.push(Task { priority: Priority::Medium, deadline: Instant::now() + Duration::from_secs(8), ..default() });
    
    let first = queue.pop().unwrap();
    assert_eq!(first.priority, Priority::High);
}
```

### Integration Tests

```rust
// mimi-commander/tests/integration_tests.rs

#[tokio::test]
async fn test_end_to_end_intent_processing() {
    // Start Message Bus (Docker Compose)
    let bus = start_test_bus().await;
    
    // Start Mimi
    let mimi = MimiCore::new(bus.client()).spawn();
    
    // Publish Intent
    bus.publish("intent/structured", &mock_intent()).await;
    
    // Wait for task dispatch
    let task = bus.subscribe("task/execute")
        .timeout(Duration::from_secs(1))
        .next()
        .await
        .unwrap();
    
    assert_eq!(task.target_module, "pandora");
    
    // Cleanup
    mimi.shutdown().await;
    bus.stop().await;
}
```

### Benchmarks

```rust
// mimi-commander/benches/routing_bench.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_routing(c: &mut Criterion) {
    let core = MimiCore::new(mock_bus_client());
    let intent = mock_intent();
    
    c.bench_function("route_task", |b| {
        b.iter(|| {
            core.route_task(black_box(intent.clone()), None)
        })
    });
}

criterion_group!(benches, bench_routing);
criterion_main!(benches);
```

**Benchmark Targets:**
- `route_task`: < 1μs (mean)
- `state_transition`: < 100ns (mean)
- `serialize_task`: < 10μs (mean)

### Load Tests (M1 DoD)

Use `k6` or `wrk` to simulate:
- 100 msg/sec sustained for 5 minutes
- Burst of 500 msg/sec for 10 seconds
- Verify: < 5ms p99 latency, 0 message loss

---

## 11. Future Extensions

### M2: Persistent State (Pandora Integration)

- **Checkpoint/Restore:** Save Mimi state to Neo4j on shutdown, restore on startup
- **Task History:** Log all task executions to Pandora for audit trail
- **FFI Context Queries:** Replace Message Bus with direct C++ FFI for < 5ms latency

```rust
// M2+ FFI integration
extern "C" {
    fn pandora_checkpoint_state(state: *const MimiState) -> i32;
    fn pandora_restore_state(state: *mut MimiState) -> i32;
}
```

### M3: Security Enhancements (Odlaguna Integration)

- **Pre-Execution Authorization:** Wait for Odlaguna approval before dispatching sensitive tasks
- **Circuit Breaker:** Track failure rates per module, block unhealthy modules
- **Rate Limiting:** Throttle intents from specific sessions (anti-abuse)

### M4: Multi-Agent Coordination (Echidna Integration)

- **Parallel Task Execution:** Dispatch multiple independent tasks concurrently
- **Task DAG Execution:** Support tasks with dependencies (A → B → C)
- **Skill Registry Cache:** Cache Echidna's skill metadata to avoid repeated queries

### Post-M4: Advanced Features

- **Distributed Mimi:** Run multiple Mimi instances with leader election (Raft/Paxos)
- **Predictive Routing:** Use ML to predict best module based on historical performance
- **Self-Healing:** Auto-restart failed modules via Docker/systemd integration
- **Metrics Dashboard:** Grafana + Prometheus for real-time monitoring

---

## Cross-References

- **Message Bus Protocol:** [`specs/BUS-PROTOCOL.md`](../specs/BUS-PROTOCOL.md)
- **Pandora Integration:** [`modules/PANDORA.md`](PANDORA.md)
- **Beatrice Interface:** [`modules/BEATRICE.md`](BEATRICE.md)
- **Odlaguna Watchdog:** [`modules/ODLAGUNA.md`](ODLAGUNA.md)
- **M1 Milestone:** [`milestones/M1-FOUNDATION.md`](../milestones/M1-FOUNDATION.md)
- **Requirements:** [`REQUIREMENTS.md`](../REQUIREMENTS.md) (RF-1.1 to RF-1.4)

---

**Document Version:** 1.0  
**Last Updated:** 2026-04-16  
**Maintainer:** MiMi Architecture Team
