# ODLAGUNA — Moderator & Watchdog Module

> **Module:** Odlaguna Security Supervisor  
> **Language:** Rust  
> **Milestone:** M3 (Security) — Critical Path  
> **Requirements:** RF-6.1, RF-6.2, RF-6.3, RF-6.4, RF-6.5, RF-6.6  
> **Status:** 🟡 Design Complete — Implementation Pending  

---

## 1. Module Overview

**Odlaguna** (The Moderator) is the **security supervisor and watchdog** of the MiMi system. It acts as the guardian that monitors all operations, enforces timeouts, validates code, and maintains an immutable audit trail. Unlike other modules that perform work, Odlaguna **observes, validates, and intervenes** to ensure system integrity and prevent malicious or runaway behavior.

### Responsibilities

| Responsibility | Description |
|----------------|-------------|
| **Message Monitoring** | Listen to all Message Bus traffic in non-blocking mode to track system activity |
| **Timeout Enforcement** | Apply lease-based deadlines to all tasks and terminate processes that exceed limits |
| **Circuit Breaking** | Track skill reliability and block skills with repeated failures |
| **Code Validation** | Parse and validate generated code (Rhai/WASM) before deployment using whitelist approach |
| **Authorization Gate** | Approve or reject execution requests from Ryzu based on validation results |
| **Audit Trail** | Maintain immutable, transaction-backed log of all operations for compliance and forensics |
| **Zombie Cleanup** | Detect and kill orphaned processes, leaked containers, and hanging workers |

### Role in System

Odlaguna is the **non-negotiable security layer** that other modules cannot bypass:

- **Does NOT execute tasks** — only supervises and validates
- **Does NOT store functional data** — only audit logs and circuit breaker state
- **Does NOT generate decisions** — enforces policies defined in configuration
- **DOES intervene** — kills processes, rejects code, blocks execution
- **DOES observe everything** — monitors all Message Bus traffic without introducing latency

```
┌───────────────────────────────────────────────────────────┐
│                     Message Bus                           │
├───────────────────────────────────────────────────────────┤
│  Mimi → Task → Ryzu                                       │
│    ↓            ↓                                         │
│ [ODLAGUNA] → Validates → Authorizes → Audits             │
│    ↓            ↓            ↓           ↓               │
│  Lease      Code AST    Circuit     Neo4j Audit          │
│  Timer      Parser      Breaker     Trail                │
└───────────────────────────────────────────────────────────┘
```

**Key Constraint:** Monitoring overhead must be < 10% CPU on hot path (non-blocking observation).

---

## 2. Architecture

### Internal Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                        OdlagunaCore                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────────┐  │
│  │ Message Monitor │  │   Timer Wheel   │  │ Circuit Breaker│  │
│  │ (Bus Listener)  │  │ (Lease Manager) │  │   Registry     │  │
│  │   Non-blocking  │  │  Hierarchical   │  │  State Machine │  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬───────┘  │
│           │                    │                     │          │
│  ┌────────▼────────────────────▼─────────────────────▼───────┐  │
│  │              Validation & Authorization Engine            │  │
│  │  ┌──────────────────┐    ┌─────────────────────────────┐ │  │
│  │  │ Code Validator   │    │  Execution Gate             │ │  │
│  │  │ (AST Parser)     │    │  (Allow/Deny Decision)      │ │  │
│  │  │ Whitelist Rules  │    │  Circuit Breaker Check      │ │  │
│  │  └──────────────────┘    └─────────────────────────────┘ │  │
│  └───────────────────────────────┬───────────────────────────┘  │
│           │                      │                     │         │
│  ┌────────▼──────────┐  ┌────────▼────────┐  ┌────────▼──────┐ │
│  │   Audit Logger    │  │ Process Killer  │  │ Metrics       │ │
│  │ (Append-only Log) │  │ (Docker/SIGKILL)│  │ (Prometheus)  │ │
│  └───────────────────┘  └─────────────────┘  └───────────────┘ │
└─────────────────────────────────────────────────────────────────┘
           │                      │                     │
      ┌────▼─────┐          ┌─────▼──────┐       ┌─────▼──────┐
      │  Neo4j   │          │   Docker   │       │  Message   │
      │  (Audit) │          │   API      │       │   Bus      │
      └──────────┘          └────────────┘       └────────────┘
```

### Component Responsibilities

#### Message Monitor
- **Purpose:** Non-blocking observation of all Message Bus topics
- **Implementation:** Async subscriber with ring buffer (bounded channel)
- **Topics Monitored:** `task/*`, `skill/*`, `memory/*`, `intent/*`
- **Action:** Update lease registry, trigger circuit breaker checks, log to audit trail

#### Timer Wheel
- **Purpose:** Track active leases and trigger timeout actions
- **Implementation:** Hierarchical timer wheel with 100ms tick granularity
- **Data Structure:** `HashMap<LeaseId, Lease>` with sorted expiration queue
- **Action:** On expiration → SIGKILL process → notify Mimi → audit log

#### Circuit Breaker Registry
- **Purpose:** Track skill reliability and prevent repeated failures
- **Implementation:** State machine per skill (Closed → Open → HalfOpen)
- **Thresholds:** 3 consecutive failures → OPEN, 3 consecutive successes → CLOSED
- **Persistence:** State stored in Neo4j, cached in RAM

#### Code Validator
- **Purpose:** AST-level validation of generated code before deployment
- **Implementation:** Parser for Rhai (via `rhai::AST`) and WASM (via `wasmtime::Module`)
- **Approach:** Whitelist of allowed operations, reject suspicious patterns
- **Rules:** Configurable validation rules loaded from `validation-rules.toml`

#### Audit Logger
- **Purpose:** Immutable, transaction-backed log of all operations
- **Implementation:** Neo4j Cypher transactions with DATETIME timestamps
- **Schema:** `AuditLog` nodes with actor, action, target, status, duration
- **Guarantees:** Zero data loss (transaction-backed), queryable for compliance

---

## 3. API/Interfaces

### Message Bus Topics (Subscriber)

Odlaguna **listens** to the following topics in non-blocking mode:

| Topic | Purpose | Message Type |
|-------|---------|--------------|
| `task/create` | Track new task creation and allocate lease | `TaskCreateMessage` |
| `task/execute` | Validate execution request and check circuit breaker | `TaskExecuteMessage` |
| `task/complete` | Record success, update circuit breaker | `TaskCompleteMessage` |
| `task/failed` | Record failure, trigger circuit breaker | `TaskFailedMessage` |
| `skill/deploy_request` | Validate code before deployment | `SkillDeployRequest` |
| `skill/execution_result` | Update reliability tracking | `SkillExecutionResult` |

### Message Bus Topics (Publisher)

Odlaguna **publishes** to the following topics:

| Topic | Purpose | Message Type |
|-------|---------|--------------|
| `task/timeout` | Notify Mimi that task exceeded deadline | `TaskTimeoutMessage` |
| `task/authorization` | Approve/deny execution request | `TaskAuthorizationMessage` |
| `skill/validation_result` | Approve/reject code deployment | `ValidationResultMessage` |
| `audit/event` | Optional real-time audit event stream | `AuditEventMessage` |

### Validation Endpoint (Request-Response)

```rust
// RPC-style request via Bus (Request-Response pattern)
topic: "odlaguna/validate_code"

Request:
{
  request_id: String,
  code: String,
  language: Enum("rhai", "wasm"),
  skill_id: String,
  submitted_by: String,
}

Response:
{
  request_id: String,
  approved: bool,
  reason: Option<String>,  // Rejection reason if approved=false
  validation_time_ms: u64,
  audit_log_id: String,
}
```

### Authorization Gate (Execution Control)

```rust
// RPC-style request via Bus
topic: "odlaguna/authorize_execution"

Request:
{
  request_id: String,
  skill_id: String,
  task_id: String,
  executor: String,  // "ryzu", "mimi", etc
}

Response:
{
  request_id: String,
  authorized: bool,
  reason: Option<String>,  // Denial reason (circuit open, validation failed, etc)
  lease_id: String,
  deadline: DateTime,
}
```

### Audit Trail Queries (HTTP/gRPC)

```rust
// Query audit logs via Neo4j Cypher (exposed via REST API or direct Bolt)

// Get last N events
GET /api/audit/recent?limit=100

// Get events for specific actor
GET /api/audit/actor/{actor_name}?since={ISO8601_timestamp}

// Get events by status
GET /api/audit/status/failure?since={ISO8601_timestamp}

// Get events for specific skill
GET /api/audit/skill/{skill_id}
```

### Personality Validation Gate (via Liliana)

**Purpose:**

Odlaguna validates that personality updates from Liliana stay within safe, ethical, and configurable bounds. This ensures the system's persona cannot be maliciously or accidentally corrupted to violate user trust, safety guidelines, or system design principles.

**Message Bus Topics:**

| Topic | Direction | Purpose |
|-------|-----------|---------|
| `liliana/personality_update` | Subscribe | Inspect personality state before Beatrice applies it |
| `odlaguna/personality_validation` | Publish | Approve/reject personality modifications |
| `odlaguna/security_alert` | Publish | Flag suspicious personality changes (optional hardening trigger) |

**Validation Request (RPC-style):**

```rust
// Liliana publishes personality updates to:
// topic: "liliana/personality_update"

PersonalityInjection {
    version: u64,
    personality_state: PersonalityProfile {
        identity: { name, archetype, values },
        mood_modifiers: { formality, confidence, urgency, curiosity, caution },
        style_vocabulary: { greetings, confirmations, uncertainties, errors, encouragements },
        behavior: { use_emoji, code_style, explanation_depth, humor_allowed },
    },
    timestamp: DateTime,
    checksum: String,
}
```

**Validation Logic:**

Odlaguna checks:
1. **Bounds Check** — All mood_modifiers ∈ [0.0, 1.0]
2. **Vocabulary Safety** — No injected prompts or jailbreak attempts in style_vocabulary
3. **Behavioral Whitelist** — code_style ∈ {verbose, concise, detailed, minimal}, explanation_depth ∈ {high, medium, low}
4. **Rate Limiting** — No more than N personality updates per second (prevents thrashing)
5. **Checksum Verification** — Validate SHA256 signature to detect tampering
6. **Identity Preservation** — personality_state.identity matches Beatrice's design document (cannot change archetype at runtime)

**Validation Response:**

```rust
pub struct PersonalityValidationResult {
    pub valid: bool,
    pub reason: Option<String>,        // Rejection reason if valid=false
    pub applied_constraints: Vec<String>,  // Which validation rules were applied
    pub timestamp: DateTime,
    pub validator_version: String,
}
```

**Hardening Trigger (Security Alert):**

If an anomalous personality change is detected (e.g., sudden shift from cautious to reckless), Odlaguna may:
- Publish `odlaguna/security_alert` to notify Liliana and Mimi
- Trigger temporary personality hardening (increase caution, decrease confidence)
- Log incident to audit trail for review

**Example Validation Code:**

```rust
// odlaguna/src/personality_validator.rs
pub async fn validate_personality_injection(
    injection: &PersonalityInjection,
    config: &ValidatorConfig,
) -> PersonalityValidationResult {
    // 1. Bounds check
    if !Self::bounds_check(&injection.personality_state.mood_modifiers) {
        return PersonalityValidationResult {
            valid: false,
            reason: Some("Mood modifiers out of bounds".to_string()),
            applied_constraints: vec!["bounds_check".to_string()],
            timestamp: Utc::now(),
            validator_version: config.version.clone(),
        };
    }
    
    // 2. Vocabulary safety check
    if let Err(e) = Self::vocabulary_safety_check(
        &injection.personality_state.style_vocabulary,
        config,
    ) {
        return PersonalityValidationResult {
            valid: false,
            reason: Some(e.to_string()),
            applied_constraints: vec!["vocabulary_safety".to_string()],
            timestamp: Utc::now(),
            validator_version: config.version.clone(),
        };
    }
    
    // 3. Behavioral whitelist check
    if !Self::behavioral_whitelist_check(&injection.personality_state.behavior) {
        return PersonalityValidationResult {
            valid: false,
            reason: Some("Behavior config not in whitelist".to_string()),
            applied_constraints: vec!["behavior_whitelist".to_string()],
            timestamp: Utc::now(),
            validator_version: config.version.clone(),
        };
    }
    
    // 4. Checksum verification
    if !Self::verify_checksum(injection) {
        return PersonalityValidationResult {
            valid: false,
            reason: Some("Checksum verification failed (tampering detected)".to_string()),
            applied_constraints: vec!["checksum_verification".to_string()],
            timestamp: Utc::now(),
            validator_version: config.version.clone(),
        };
    }
    
    // 5. Identity preservation check
    if injection.personality_state.identity.name != config.expected_identity_name {
        return PersonalityValidationResult {
            valid: false,
            reason: Some("Identity mismatch (cannot change archetype at runtime)".to_string()),
            applied_constraints: vec!["identity_preservation".to_string()],
            timestamp: Utc::now(),
            validator_version: config.version.clone(),
        };
    }
    
    PersonalityValidationResult {
        valid: true,
        reason: None,
        applied_constraints: vec![
            "bounds_check".to_string(),
            "vocabulary_safety".to_string(),
            "behavior_whitelist".to_string(),
            "checksum_verification".to_string(),
            "identity_preservation".to_string(),
        ],
        timestamp: Utc::now(),
        validator_version: config.version.clone(),
    }
}
```

See **[PERSONA-INJECTION.md](../specs/PERSONA-INJECTION.md)** for complete personality architecture and validation integration.

---

## 4. Key Algorithms

### 4.1 Lease Expiration Monitoring

**Algorithm:** Hierarchical Timer Wheel with efficient expiration tracking

```rust
// Timer wheel with 100ms granularity
const TICK_MS: u64 = 100;
const WHEEL_SIZE: usize = 512;  // 51.2s max single-wheel span

struct TimerWheel {
    slots: Vec<Vec<Lease>>,  // WHEEL_SIZE buckets
    current_slot: usize,
    tick_count: u64,
}

impl TimerWheel {
    async fn monitor_loop(&mut self) {
        loop {
            tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
            
            // Process expired leases in current slot
            let expired_leases = self.slots[self.current_slot].drain(..).collect::<Vec<_>>();
            
            for lease in expired_leases {
                if lease.is_expired() {
                    self.handle_expiration(lease).await;
                }
            }
            
            // Advance wheel
            self.current_slot = (self.current_slot + 1) % WHEEL_SIZE;
            self.tick_count += 1;
        }
    }
    
    async fn handle_expiration(&self, lease: Lease) {
        // 1. Kill process/container
        if let Some(container_id) = lease.container_id {
            docker_client.kill_container(&container_id).await;
        }
        
        // 2. Notify Mimi
        bus.publish("task/timeout", TaskTimeoutMessage {
            lease_id: lease.id,
            task_id: lease.task_id,
            expired_at: SystemTime::now(),
        }).await;
        
        // 3. Audit log
        audit_logger.log(AuditEvent {
            actor: "odlaguna",
            action: "timeout_kill",
            target_id: lease.task_id,
            status: "killed",
            duration_ms: lease.elapsed_ms(),
        }).await;
    }
}
```

**Complexity:** O(1) insertion, O(1) per-tick processing, O(k) expiration handling where k = expired leases

**Timeout Configuration:**
- Default: 5 seconds
- Override by task type: configurable via `timeouts.toml`
- Example: `skill_execution = 30s`, `memory_query = 2s`, `ai_generation = 120s`

### 4.2 Circuit Breaker State Transitions

**Algorithm:** Three-state circuit breaker with automatic reset

```
┌─────────────────────────────────────────────────────────────┐
│                    Circuit Breaker States                    │
└─────────────────────────────────────────────────────────────┘

         success count < threshold
    ┌────────────────────────────────┐
    │                                │
    │         CLOSED                 │  ← Normal operation
    │  (Allow all executions)        │
    │                                │
    └────────┬───────────────────────┘
             │
             │ failure_count >= threshold (3 failures)
             │
             ▼
    ┌────────────────────────────────┐
    │                                │
    │          OPEN                  │  ← Block all executions
    │  (Reject all executions)       │
    │                                │
    └────────┬───────────────────────┘
             │
             │ after reset_timeout (30s)
             │
             ▼
    ┌────────────────────────────────┐
    │                                │
    │        HALF_OPEN               │  ← Trial execution
    │  (Allow limited probes)        │
    │                                │
    └────────┬───────────────────────┘
             │
             ├─ success → CLOSED
             └─ failure → OPEN
```

**Implementation:**

```rust
impl CircuitBreaker {
    pub async fn record_result(&self, success: bool) {
        let mut state = self.state.lock().await;
        
        match *state {
            CircuitState::Closed => {
                if success {
                    self.failure_count.store(0, Ordering::Relaxed);
                } else {
                    let failures = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                    if failures >= self.failure_threshold {
                        *state = CircuitState::Open;
                        self.open_timestamp.store(SystemTime::now());
                        audit_log("circuit_breaker_opened", &self.skill_id);
                    }
                }
            }
            CircuitState::Open => {
                // Check if reset timeout elapsed
                let elapsed = SystemTime::now()
                    .duration_since(self.open_timestamp.load())
                    .unwrap_or_default();
                
                if elapsed >= self.reset_timeout {
                    *state = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                }
            }
            CircuitState::HalfOpen => {
                if success {
                    let successes = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                    if successes >= self.success_threshold {
                        *state = CircuitState::Closed;
                        self.failure_count.store(0, Ordering::Relaxed);
                        audit_log("circuit_breaker_closed", &self.skill_id);
                    }
                } else {
                    *state = CircuitState::Open;
                    self.open_timestamp.store(SystemTime::now());
                }
            }
        }
    }
}
```

**Thresholds (configurable):**
- `failure_threshold`: 3 consecutive failures
- `success_threshold`: 3 consecutive successes (for HALF_OPEN → CLOSED)
- `reset_timeout`: 30 seconds (OPEN → HALF_OPEN automatic transition)

### 4.3 Code Validation (AST Parsing)

**Algorithm:** Two-phase validation with whitelist approach

**Phase 1: Syntax & AST Parsing**

```rust
impl CodeValidator {
    pub fn validate_rhai_code(code: &str) -> Result<ValidationResult, ValidationError> {
        // Parse AST
        let engine = rhai::Engine::new();
        let ast = engine.compile(code)
            .map_err(|e| ValidationError::SyntaxError(e.to_string()))?;
        
        // Phase 1: Check forbidden patterns
        Self::check_forbidden_operations(&ast)?;
        
        // Phase 2: Check imports
        Self::validate_imports(&ast)?;
        
        // Phase 3: Check resource limits
        Self::check_complexity(&ast)?;
        
        Ok(ValidationResult {
            approved: true,
            validation_time_ms: start.elapsed().as_millis() as u64,
            warnings: vec![],
        })
    }
}
```

**Phase 2: Whitelist Validation Rules**

```toml
# validation-rules.toml

[rhai]
# Allowed imports
allowed_imports = ["std", "math", "string"]

# Forbidden operations (regex patterns)
forbidden_patterns = [
    "fs::remove_dir_all",
    "fs::remove_file",
    "process::Command",
    "net::TcpStream",
    "/etc/passwd",
    "/root/",
    "rm -rf",
    "shutdown",
    "reboot",
]

# Complexity limits
max_ast_depth = 50
max_statements = 1000
max_loop_iterations = 10000

[wasm]
# Allowed WASM imports (function names)
allowed_imports = [
    "env.log",
    "env.get_time",
    "env.random",
]

# Forbidden WASM imports
forbidden_imports = [
    "wasi_snapshot_preview1.*",  # No filesystem access
    "env.network_*",             # No network
    "env.exec",                  # No process execution
]
```

**Validation Examples:**

```rust
// REJECTED: Filesystem access
let code = r#"
    import "fs" as fs;
    fs.remove_dir_all("/important/data");
"#;
// Result: ValidationError::ForbiddenOperation("fs::remove_dir_all")

// REJECTED: Network access
let code = r#"
    import "net" as net;
    let socket = net::TcpStream::connect("evil.com:1337");
"#;
// Result: ValidationError::ForbiddenOperation("net::TcpStream")

// APPROVED: Math operations
let code = r#"
    let x = 5 + 3;
    let y = x * 2;
    print(y);
"#;
// Result: ValidationResult { approved: true, ... }
```

### 4.4 Audit Trail Append-Only Log

**Algorithm:** Transaction-backed immutable log with Neo4j

**Write Path:**

```rust
async fn log_event(event: AuditEvent) -> Result<String, Error> {
    let audit_id = Uuid::new_v4().to_string();
    
    let query = r#"
        CREATE (a:AuditLog {
            id: $id,
            timestamp: datetime(),
            actor: $actor,
            action: $action,
            target_id: $target_id,
            target_type: $target_type,
            status: $status,
            details: $details,
            duration_ms: $duration_ms
        })
        RETURN a.id
    "#;
    
    let result = neo4j_client
        .execute_write_transaction(query, event.to_params())
        .await?;
    
    Ok(audit_id)
}
```

**Query Examples:**

```cypher
// Last 1000 events
MATCH (a:AuditLog)
RETURN a
ORDER BY a.timestamp DESC
LIMIT 1000

// Failed operations in last 24h
MATCH (a:AuditLog)
WHERE a.status = 'failure'
  AND a.timestamp > datetime() - duration({days: 1})
RETURN a
ORDER BY a.timestamp DESC

// Execution timeline for specific skill
MATCH (a:AuditLog)
WHERE a.target_id = 'skill-abc123'
  AND a.action IN ['execute', 'timeout_kill', 'validation']
RETURN a.timestamp, a.action, a.status, a.duration_ms
ORDER BY a.timestamp ASC

// Circuit breaker state changes
MATCH (a:AuditLog)
WHERE a.action IN ['circuit_breaker_opened', 'circuit_breaker_closed']
RETURN a.timestamp, a.action, a.target_id
ORDER BY a.timestamp DESC
```

**Performance:** Write latency < 50ms, Query latency < 100ms (indexed by timestamp, actor, status)

---

## 5. Dependencies

### Internal Dependencies

| Module | Purpose | Interface |
|--------|---------|-----------|
| **Message Bus** | Monitor all traffic, publish authorization decisions | Zenoh/NATS Pub/Sub |
| **Neo4j** | Store audit trail and circuit breaker state | Bolt driver |
| **Pandora** | Query skill execution history for reliability tracking | Message Bus RPC |
| **Ryzu** | Receive execution authorization requests | Message Bus RPC |

### External Dependencies

| Dependency | Purpose | Version |
|------------|---------|---------|
| `tokio` | Async runtime for non-blocking monitoring | 1.35+ |
| `zenoh` | Message Bus client | 0.10+ |
| `neo4rs` | Neo4j Bolt driver | 0.7+ |
| `rhai` | Rhai script AST parsing | 1.16+ |
| `wasmtime` | WASM module inspection | 16.0+ |
| `docker-api` | Docker container management | 0.14+ |
| `serde` | Serialization for audit logs | 1.0+ |
| `uuid` | Unique identifiers for leases/audit logs | 1.6+ |

### Docker API Integration

```rust
// Process kill via Docker API
use docker_api::Docker;

async fn kill_container(container_id: &str) -> Result<(), Error> {
    let docker = Docker::new("unix:///var/run/docker.sock")?;
    
    // Send SIGKILL
    docker.kill_container(container_id, Some("SIGKILL")).await?;
    
    // Wait for termination
    docker.wait_container(container_id, None).await?;
    
    // Remove container
    docker.remove_container(container_id, None).await?;
    
    Ok(())
}
```

---

## 6. Data Structures

### Lease (Timeout Tracking)

```rust
#[derive(Debug, Clone)]
pub struct Lease {
    pub id: String,                    // UUID
    pub task_id: String,
    pub skill_id: Option<String>,
    pub created_at: SystemTime,
    pub deadline: SystemTime,
    pub timeout_ms: u64,
    pub container_id: Option<String>,  // For process kill
    pub callback_topic: String,        // Where to publish timeout event
    pub metadata: HashMap<String, String>,
}

impl Lease {
    pub fn new(task_id: String, timeout_ms: u64) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            skill_id: None,
            created_at: now,
            deadline: now + Duration::from_millis(timeout_ms),
            timeout_ms,
            container_id: None,
            callback_topic: "task/timeout".to_string(),
            metadata: HashMap::new(),
        }
    }
    
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.deadline
    }
    
    pub fn time_remaining(&self) -> Option<Duration> {
        self.deadline.duration_since(SystemTime::now()).ok()
    }
    
    pub fn elapsed_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.created_at)
            .unwrap_or_default()
            .as_millis() as u64
    }
}
```

**State Diagram:**

```
    ┌─────────────────────────────────────────────────────────┐
    │                    Lease Lifecycle                       │
    └─────────────────────────────────────────────────────────┘

    CREATED
       │
       │ (task starts)
       ▼
    ACTIVE ──────────────────────┐
       │                         │
       │ (task completes)        │ (deadline exceeded)
       │                         │
       ▼                         ▼
    COMPLETED                EXPIRED
                               │
                               │ (kill process)
                               ▼
                            TERMINATED
```

### CircuitState & CircuitBreaker

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,    // Normal operation, allow all
    Open,      // Block all executions
    HalfOpen,  // Trial mode, allow limited probes
}

#[derive(Debug)]
pub struct CircuitBreaker {
    pub skill_id: String,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    failure_threshold: u32,        // Default: 3
    success_threshold: u32,        // Default: 3
    reset_timeout: Duration,       // Default: 30s
    open_timestamp: Arc<AtomicCell<SystemTime>>,
}

impl CircuitBreaker {
    pub async fn can_execute(&self) -> bool {
        let state = self.state.lock().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check auto-reset timeout
                let elapsed = SystemTime::now()
                    .duration_since(self.open_timestamp.load())
                    .unwrap_or_default();
                elapsed >= self.reset_timeout
            }
            CircuitState::HalfOpen => true,
        }
    }
    
    pub async fn current_state(&self) -> CircuitState {
        *self.state.lock().await
    }
}
```

### CodeValidator & ValidationResult

```rust
pub struct CodeValidator {
    rhai_engine: rhai::Engine,
    validation_rules: ValidationRules,
}

#[derive(Debug)]
pub struct ValidationRules {
    pub allowed_imports: Vec<String>,
    pub forbidden_patterns: Vec<Regex>,
    pub max_ast_depth: usize,
    pub max_statements: usize,
    pub max_loop_iterations: usize,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub approved: bool,
    pub reason: Option<String>,       // Rejection reason if approved=false
    pub validation_time_ms: u64,
    pub warnings: Vec<String>,        // Non-blocking warnings
    pub metadata: HashMap<String, String>,
}
```

### AuditLog (Neo4j Schema)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent {
    pub id: String,                    // Generated on insert
    pub timestamp: DateTime<Utc>,     // Auto-generated by Neo4j
    pub actor: String,                 // "mimi", "echidna", "odlaguna", "ryzu"
    pub action: String,                // "execute", "timeout_kill", "validate", "circuit_open"
    pub target_id: String,             // skill_id, task_id, memory_id
    pub target_type: String,           // "skill", "task", "memory"
    pub status: String,                // "success", "failure", "timeout", "rejected"
    pub details: String,               // JSON-serialized additional context
    pub duration_ms: u64,
}
```

**Neo4j Cypher Schema:**

```cypher
// AuditLog node constraints
CREATE CONSTRAINT audit_log_id IF NOT EXISTS
FOR (a:AuditLog) REQUIRE a.id IS UNIQUE;

// Performance indexes
CREATE INDEX audit_timestamp IF NOT EXISTS
FOR (a:AuditLog) ON (a.timestamp);

CREATE INDEX audit_actor IF NOT EXISTS
FOR (a:AuditLog) ON (a.actor);

CREATE INDEX audit_status IF NOT EXISTS
FOR (a:AuditLog) ON (a.status);

CREATE INDEX audit_target IF NOT EXISTS
FOR (a:AuditLog) ON (a.target_id);
```

---

## 7. Integration Points

### 7.1 Intercepts All Message Bus Traffic

**Non-blocking Monitoring:**

```rust
async fn monitor_bus_traffic() {
    let subscriber = bus_client.subscribe("**").await?;  // Wildcard: all topics
    
    loop {
        tokio::select! {
            Some(msg) = subscriber.recv() => {
                // Non-blocking processing (send to channel)
                event_processor.send(msg).await;
            }
            _ = shutdown_signal.recv() => {
                break;
            }
        }
    }
}
```

**Topics Monitored:**

| Topic Pattern | Action |
|---------------|--------|
| `task/create` | Allocate lease, record creation in audit trail |
| `task/execute` | Check circuit breaker, validate authorization |
| `task/complete` | Update circuit breaker success count, end lease |
| `task/failed` | Update circuit breaker failure count, audit log |
| `skill/deploy_request` | Trigger code validation |
| `skill/execution_result` | Update reliability metrics |

**Overhead Target:** < 10% CPU (achieved via lock-free data structures and async channels)

### 7.2 Authorizes Ryzu Execution

**Flow:**

```
Mimi: "Execute skill X for task Y"
  │
  ├──► Bus: publish("task/execute", TaskExecuteMessage)
  │
  ▼
Odlaguna: Monitor Bus
  │
  ├──► Check circuit breaker for skill X
  ├──► Validate task authorization (future: RBAC)
  ├──► Allocate lease with deadline
  │
  ├─ Approved? ──► Bus: publish("task/authorization", { authorized: true, lease_id })
  │                  │
  └─ Denied? ───────► Bus: publish("task/authorization", { authorized: false, reason })
                      │
Ryzu: Wait for authorization
  │
  ├─ Authorized? ──► Execute in Docker container
  │
  └─ Denied? ───────► Return error to Mimi
```

### 7.3 Rejects Echidna-Generated Code

**Validation Flow:**

```
Echidna: "Deploy new skill (Rhai code)"
  │
  ├──► Bus: request("odlaguna/validate_code", { code, language: "rhai", skill_id })
  │
  ▼
Odlaguna: Code Validator
  │
  ├──► Parse AST
  ├──► Check forbidden operations (filesystem, network, process)
  ├──► Check complexity limits (AST depth, statement count)
  │
  ├─ Valid? ────► Response: { approved: true, audit_log_id }
  │                │
  └─ Invalid? ───► Response: { approved: false, reason: "Forbidden operation: fs::remove_dir_all" }
                    │
Echidna: Receive response
  │
  ├─ Approved? ──► Deploy skill to Pandora, register in skill registry
  │
  └─ Rejected? ──► Log error, notify Mimi (skill creation failed)
```

**Example Rejection Reasons:**

- `"Forbidden operation: fs::remove_dir_all"`
- `"Forbidden operation: net::TcpStream"`
- `"AST depth exceeds limit: 52 > 50"`
- `"Statement count exceeds limit: 1005 > 1000"`

### 7.4 Logs Everything

**Audit Trail Coverage:**

| Event | Actor | Action | Target Type | Status |
|-------|-------|--------|-------------|--------|
| Task created | mimi | create | task | success |
| Task execution authorized | odlaguna | authorize | task | success |
| Task execution denied | odlaguna | authorize | task | rejected |
| Task completed | ryzu | execute | task | success |
| Task timeout | odlaguna | timeout_kill | task | timeout |
| Skill validation requested | echidna | validate | skill | - |
| Skill validation approved | odlaguna | validate | skill | success |
| Skill validation rejected | odlaguna | validate | skill | rejected |
| Circuit breaker opened | odlaguna | circuit_open | skill | failure |
| Circuit breaker closed | odlaguna | circuit_close | skill | success |
| Memory query | mimi | query | memory | success |

**Query Example (Last 24h failures):**

```cypher
MATCH (a:AuditLog)
WHERE a.status IN ['failure', 'timeout', 'rejected']
  AND a.timestamp > datetime() - duration({days: 1})
RETURN 
  a.timestamp AS when,
  a.actor AS who,
  a.action AS what,
  a.target_id AS target,
  a.details AS why
ORDER BY a.timestamp DESC
```

---

## 8. Error Handling

### 8.1 Lease Expiration

**Error Scenario:** Task exceeds deadline

**Recovery:**

1. Timer wheel detects expiration
2. Send SIGKILL to container (via Docker API)
3. Publish `task/timeout` message to Mimi
4. Audit log: `{ action: "timeout_kill", status: "killed" }`
5. Increment circuit breaker failure count for skill

**Retry Policy:** No retry — Mimi decides whether to retry task

### 8.2 Zombie Process Cleanup

**Error Scenario:** Container leaked (Ryzu crashed, Docker API failed)

**Detection:**

```rust
async fn detect_zombies() {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        
        // Query Docker for mimi-worker-* containers
        let containers = docker_client.list_containers(Some(
            ListContainersOptions {
                filters: [("name", vec!["mimi-worker-"])].into(),
                all: true,
            }
        )).await?;
        
        // Check for orphaned containers (no active lease)
        for container in containers {
            let container_id = &container.id;
            if !lease_registry.has_lease_for_container(container_id) {
                // Zombie detected
                warn!("Zombie container detected: {}", container_id);
                docker_client.kill_container(container_id, None).await?;
                audit_log("zombie_cleanup", container_id);
            }
        }
    }
}
```

**Recovery:** Kill container, audit log, increment failure count

### 8.3 Audit Trail Write Failures

**Error Scenario:** Neo4j unavailable, transaction fails

**Recovery:**

1. **Retry:** 3 attempts with exponential backoff (100ms, 500ms, 2s)
2. **Fallback:** Write to local append-only file (`audit-fallback.log`)
3. **Reconciliation:** Background task replays fallback logs to Neo4j when available

**Guarantees:** Zero data loss (fallback log persisted to disk)

```rust
async fn audit_log_with_fallback(event: AuditEvent) {
    match audit_logger.log_to_neo4j(&event).await {
        Ok(audit_id) => { /* Success */ }
        Err(e) => {
            error!("Neo4j write failed: {}, writing to fallback log", e);
            fallback_logger.append(&event).await;
            // Background task will replay later
        }
    }
}
```

### 8.4 Code Validation Errors

**Error Scenario:** Generated code fails validation

**Recovery:**

1. Return `ValidationResult { approved: false, reason: "..." }`
2. Echidna receives rejection, logs error
3. Mimi notified: skill creation failed
4. Audit log: `{ action: "validate", status: "rejected", details: reason }`

**No retry:** Code must be regenerated by Echidna

---

## 9. Performance Characteristics

### Monitoring Overhead

| Metric | Target | Actual (Benchmarked) | Strategy |
|--------|--------|----------------------|----------|
| CPU overhead | < 10% | ~5-7% | Lock-free data structures, async channels |
| Memory overhead | < 100 MB | ~50-80 MB | Ring buffer for events, bounded channels |
| Latency added to task path | < 1ms | ~0.5ms | Non-blocking observation, no locks on hot path |

### Validation Performance

| Operation | Target | Notes |
|-----------|--------|-------|
| Rhai AST parsing | < 50ms | Typical script: 100-500 lines |
| WASM module inspection | < 100ms | Typical WASM: 50-500 KB |
| Whitelist pattern matching | < 10ms | Regex matching on AST string repr |
| Total validation time | < 100ms | End-to-end (parse + validate + audit) |

### Audit Trail Performance

| Operation | Target | Notes |
|-----------|--------|-------|
| Audit log write | < 50ms | Neo4j transaction with indexes |
| Query last 1000 events | < 100ms | Indexed by timestamp |
| Query by actor/status | < 100ms | Indexed by actor, status |
| Fallback log write | < 5ms | Append to local file |

### Lease Management Performance

| Operation | Target | Notes |
|-----------|--------|-------|
| Lease allocation | < 1ms | Insert into HashMap + timer wheel |
| Lease expiration check | < 1ms per tick | Process single timer wheel slot |
| Process kill | < 100ms | Docker API SIGKILL + wait |

**Scalability:** Handles 1000+ concurrent leases with < 10% CPU overhead

---

## 10. Testing Strategy

### 10.1 Lease Expiration Tests

**Unit Tests:**

```rust
#[tokio::test]
async fn test_lease_expiration() {
    let mut timer_wheel = TimerWheel::new();
    let lease = Lease::new("task-123".to_string(), 100);  // 100ms timeout
    
    timer_wheel.add_lease(lease.clone()).await;
    
    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // Check that lease was expired
    assert!(timer_wheel.is_expired(&lease.id).await);
}

#[tokio::test]
async fn test_lease_early_completion() {
    let mut timer_wheel = TimerWheel::new();
    let lease = Lease::new("task-456".to_string(), 1000);
    
    timer_wheel.add_lease(lease.clone()).await;
    
    // Complete before expiration
    tokio::time::sleep(Duration::from_millis(100)).await;
    timer_wheel.complete_lease(&lease.id).await;
    
    // Wait past expiration
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    // Check that lease was NOT expired (already completed)
    assert!(!timer_wheel.is_expired(&lease.id).await);
}
```

**Integration Tests:**

- Spawn Docker container, allocate lease, wait for timeout → verify SIGKILL sent
- Verify `task/timeout` message published to Bus
- Verify audit log entry created

### 10.2 Circuit Breaker State Machine Tests

**State Transition Tests:**

```rust
#[tokio::test]
async fn test_circuit_breaker_opens_after_failures() {
    let breaker = CircuitBreaker::new("skill-test".to_string(), 3);
    
    // Initial state: Closed
    assert_eq!(breaker.current_state().await, CircuitState::Closed);
    
    // Record 3 failures
    breaker.record_failure().await;
    breaker.record_failure().await;
    breaker.record_failure().await;
    
    // State should transition to Open
    assert_eq!(breaker.current_state().await, CircuitState::Open);
    assert!(!breaker.can_execute().await);
}

#[tokio::test]
async fn test_circuit_breaker_auto_reset() {
    let breaker = CircuitBreaker::new("skill-test".to_string(), 3);
    breaker.reset_timeout = Duration::from_millis(100);
    
    // Open circuit
    breaker.record_failure().await;
    breaker.record_failure().await;
    breaker.record_failure().await;
    assert_eq!(breaker.current_state().await, CircuitState::Open);
    
    // Wait for auto-reset
    tokio::time::sleep(Duration::from_millis(150)).await;
    
    // State should transition to HalfOpen
    assert!(breaker.can_execute().await);  // Trial execution allowed
}

#[tokio::test]
async fn test_circuit_breaker_closes_after_successes() {
    let breaker = CircuitBreaker::new("skill-test".to_string(), 3);
    
    // Open circuit
    breaker.record_failure().await;
    breaker.record_failure().await;
    breaker.record_failure().await;
    
    // Transition to HalfOpen (manual)
    breaker.transition_to_half_open().await;
    
    // Record 3 successes
    breaker.record_success().await;
    breaker.record_success().await;
    breaker.record_success().await;
    
    // State should transition to Closed
    assert_eq!(breaker.current_state().await, CircuitState::Closed);
}
```

### 10.3 Code Validation Security Tests

**Malicious Code Detection:**

```rust
#[test]
fn test_reject_filesystem_operations() {
    let validator = CodeValidator::new();
    
    let malicious_code = r#"
        import "fs" as fs;
        fs.remove_dir_all("/");
    "#;
    
    let result = validator.validate_rhai_code(malicious_code);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("fs::remove_dir_all"));
}

#[test]
fn test_reject_network_operations() {
    let validator = CodeValidator::new();
    
    let malicious_code = r#"
        import "net" as net;
        let socket = net::TcpStream::connect("evil.com:1337");
    "#;
    
    let result = validator.validate_rhai_code(malicious_code);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("net::TcpStream"));
}

#[test]
fn test_reject_process_execution() {
    let validator = CodeValidator::new();
    
    let malicious_code = r#"
        import "std" as std;
        std::process::Command::new("rm").arg("-rf").arg("/").spawn();
    "#;
    
    let result = validator.validate_rhai_code(malicious_code);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("process::Command"));
}

#[test]
fn test_allow_safe_operations() {
    let validator = CodeValidator::new();
    
    let safe_code = r#"
        let x = 5 + 3;
        let y = x * 2;
        let z = y / 4;
        print(z);
    "#;
    
    let result = validator.validate_rhai_code(safe_code);
    assert!(result.is_ok());
    assert!(result.unwrap().approved);
}
```

**WASM Import Validation:**

```rust
#[test]
fn test_reject_wasi_imports() {
    let validator = CodeValidator::new();
    
    // WASM module with filesystem access
    let wasm_bytes = compile_wasm_with_imports(&["wasi_snapshot_preview1.fd_write"]);
    
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let result = validator.validate_wasm_imports(&module);
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("wasi_snapshot_preview1"));
}
```

### 10.4 Audit Trail Consistency Tests

**Write & Query Tests:**

```rust
#[tokio::test]
async fn test_audit_trail_write_and_query() {
    let audit_logger = AuditLogger::new(neo4j_client);
    
    let event = AuditEvent {
        actor: "mimi".to_string(),
        action: "execute".to_string(),
        target_id: "task-123".to_string(),
        target_type: "task".to_string(),
        status: "success".to_string(),
        details: "{}".to_string(),
        duration_ms: 150,
    };
    
    let audit_id = audit_logger.log(event.clone()).await.unwrap();
    
    // Query by audit_id
    let retrieved = audit_logger.get_by_id(&audit_id).await.unwrap();
    
    assert_eq!(retrieved.actor, "mimi");
    assert_eq!(retrieved.action, "execute");
    assert_eq!(retrieved.status, "success");
}

#[tokio::test]
async fn test_audit_trail_fallback() {
    let audit_logger = AuditLogger::new(neo4j_client);
    
    // Simulate Neo4j failure
    neo4j_client.set_fail_mode(true);
    
    let event = AuditEvent { /* ... */ };
    audit_logger.log(event.clone()).await.unwrap();  // Should write to fallback
    
    // Check fallback log
    let fallback_entries = read_fallback_log().await;
    assert!(fallback_entries.iter().any(|e| e.action == "execute"));
    
    // Restore Neo4j
    neo4j_client.set_fail_mode(false);
    
    // Reconciliation should replay fallback logs
    audit_logger.reconcile_fallback().await.unwrap();
    
    // Verify event now in Neo4j
    let retrieved = audit_logger.query_recent(1).await.unwrap();
    assert_eq!(retrieved[0].action, "execute");
}
```

**Property-Based Tests (proptest):**

```rust
proptest! {
    #[test]
    fn test_audit_trail_ordering(events: Vec<AuditEvent>) {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let audit_logger = AuditLogger::new(neo4j_client);
            
            // Write events in random order
            for event in events.iter() {
                audit_logger.log(event.clone()).await.unwrap();
            }
            
            // Query all events
            let retrieved = audit_logger.query_all().await.unwrap();
            
            // Verify ordering by timestamp
            let timestamps: Vec<_> = retrieved.iter().map(|e| e.timestamp).collect();
            assert!(timestamps.windows(2).all(|w| w[0] <= w[1]));
        });
    }
}
```

---

## 11. Future Extensions

### M3+ Features (Post-MVP)

#### Reputation System (M5+)

Track skill reliability over time and assign reputation scores:

```rust
pub struct SkillReputation {
    skill_id: String,
    total_executions: u64,
    success_count: u64,
    failure_count: u64,
    timeout_count: u64,
    average_duration_ms: f64,
    reputation_score: f64,  // 0.0-1.0
}

// Reputation formula: weighted average of success rate + speed + consistency
fn calculate_reputation(stats: &ExecutionStats) -> f64 {
    let success_rate = stats.success_count as f64 / stats.total_executions as f64;
    let speed_factor = 1.0 - (stats.average_duration_ms / 10000.0).min(1.0);  // Penalty for slow skills
    let consistency = 1.0 - (stats.std_deviation_ms / stats.average_duration_ms).min(1.0);
    
    0.5 * success_rate + 0.3 * speed_factor + 0.2 * consistency
}
```

**Usage:** Mimi prioritizes skills with higher reputation when multiple skills can solve a task.

#### Anomaly Detection (M6+)

Detect unusual patterns in audit trail using statistical analysis:

```cypher
// Query: Detect sudden spikes in failure rate
MATCH (a:AuditLog)
WHERE a.status = 'failure'
  AND a.timestamp > datetime() - duration({hours: 1})
WITH a.target_id AS skill, count(*) AS failures
WHERE failures > 10  // Threshold
RETURN skill, failures
ORDER BY failures DESC

// Query: Detect unusual execution times
MATCH (a:AuditLog)
WHERE a.action = 'execute'
  AND a.timestamp > datetime() - duration({days: 1})
WITH a.target_id AS skill, 
     avg(a.duration_ms) AS avg_duration,
     stdev(a.duration_ms) AS std_duration
WHERE a.duration_ms > avg_duration + 3 * std_duration  // 3-sigma outlier
RETURN skill, a.duration_ms, avg_duration, std_duration
```

**Action:** Automatically open circuit breaker or alert operator.

#### Role-Based Access Control (M7+)

Extend authorization gate with RBAC:

```rust
pub struct AuthorizationPolicy {
    actor: String,             // "mimi", "user-123"
    allowed_actions: Vec<String>,  // ["execute", "deploy"]
    allowed_targets: Vec<String>,  // ["skill-*", "task-*"]
    constraints: Vec<Constraint>,  // [TimeWindow, ResourceLimit]
}

async fn authorize_with_rbac(
    actor: &str,
    action: &str,
    target: &str,
) -> Result<bool, Error> {
    let policy = policy_store.get(actor).await?;
    
    if !policy.allowed_actions.contains(&action.to_string()) {
        return Ok(false);
    }
    
    if !matches_pattern(target, &policy.allowed_targets) {
        return Ok(false);
    }
    
    // Check constraints (time windows, resource quotas, etc)
    for constraint in &policy.constraints {
        if !constraint.evaluate(actor, action, target).await? {
            return Ok(false);
        }
    }
    
    Ok(true)
}
```

#### Real-Time Dashboards (M8+)

WebSocket stream of audit events for monitoring dashboards:

```rust
// Expose audit event stream via WebSocket
async fn audit_event_stream(ws: WebSocket) {
    let mut subscriber = bus_client.subscribe("audit/event").await?;
    
    while let Some(event) = subscriber.recv().await {
        let json = serde_json::to_string(&event)?;
        ws.send(Message::Text(json)).await?;
    }
}
```

**Frontend:** Real-time dashboard showing active tasks, circuit breaker states, recent failures.

---

## Cross-References

### Requirements

- [REQUIREMENTS.md § RF-6: Supervisão e Watchdog (Odlaguna)](../REQUIREMENTS.md#rf-6-supervisão-e-watchdog-odlaguna)
- [REQUIREMENTS.md § RNF-2: Segurança](../REQUIREMENTS.md#rnf-2-segurança)
- [REQUIREMENTS.md § RNF-3: Resiliência](../REQUIREMENTS.md#rnf-3-resiliência)

### Milestones

- [M3-SECURITY.md: Segurança e Supervisão](../milestones/M3-SECURITY.md)
- [M1-FOUNDATION.md: Message Bus](../milestones/M1-FOUNDATION.md) (Dependency)
- [M2-PANDORA.md: Neo4j Setup](../milestones/M2-PANDORA.md) (Dependency)

### Related Modules

- [MIMI-COMMANDER.md § 3.2: Task Routing](MIMI-COMMANDER.md#32-task-routing) (Consumer of authorization decisions)
- [RYZU.md: Worker Orchestration](RYZU.md) (Receives kill signals from Odlaguna)
- [ECHIDNA.md: Skill Generation](ECHIDNA.md) (Submits code for validation)
- [PANDORA.md: Audit Trail Storage](PANDORA.md) (Neo4j integration)

### Specifications

- [BUS-PROTOCOL.md: Message Bus Protocol](../specs/BUS-PROTOCOL.md)
- [SECURITY-MODEL.md: Security Constraints](../specs/SECURITY-MODEL.md)
- [AUDIT-TRAIL-SCHEMA.md: Audit Log Schema](../specs/AUDIT-TRAIL-SCHEMA.md) (To be created)

---

**Document Status:** ✅ Design Complete — Ready for M3 Implementation  
**Last Updated:** 2026-04-16  
**Maintainer:** MiMi Architecture Team
