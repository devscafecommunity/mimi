# BUS-PROTOCOL: Technical Specification for Message Bus

Technical specification for the MiMi Message Bus protocol, defining communication patterns, schemas, and performance targets.

## 1. Overview
The Message Bus is the backbone of MiMi, facilitating low-latency communication between Rust and C++ modules. It enables a decoupled architecture where agents (Mimi, Beatrice, Pandora, etc.) interact via structured messages.

### Decision Matrix: Zenoh vs NATS
| Feature | Zenoh | NATS | Decision |
|---------|-------|------|----------|
| **Latency** | < 1ms (Native Rust) | ~1-2ms (Go-based broker) | **Zenoh** (Superior for intra-node) |
| **Footprint** | Extremely small (< 10MB) | Medium (~30MB) | **Zenoh** |
| **Patterns** | Pub/Sub, Query/Reply, Storage | Pub/Sub, Request/Reply, JetStream | Tie |
| **Zero-Copy** | Native support | Buffer copying in most clients | **Zenoh** |
| **Inter-node** | Native bridging/routing | Requires leaf nodes/clusters | **Zenoh** |

**Final Decision:** **Zenoh** is selected for M1 due to its native Rust implementation, lower latency floor, and alignment with zero-copy requirements via FlatBuffers.

## 2. Architecture
The bus follows a hybrid broker/brokerless model.

- **Broker Model:** Zenoh router handles message distribution and discovery.
- **Pub/Sub Pattern:** Used for asynchronous updates (e.g., `memory/update`, `intent/raw`).
- **Request-Response Pattern:** Used for synchronous task execution (e.g., `task/execute` → `task/result`).
- **QoS Levels:**
  - **Best Effort:** For high-frequency telemetry or non-critical updates.
  - **Reliable:** For task execution and memory state updates (at-least-once).

## 3. Topics Schema
Standard topics for inter-module communication:

| Topic | Producer | Consumer | Message Type | Description |
|-------|----------|----------|--------------|-------------|
| `intent/raw` | Beatrice | Mimi | `IntentMessage` | Raw user intent from NLP interface. |
| `task/create_skill` | Mimi | Echidna | `TaskMessage` | Request to generate a new skill. |
| `skill/review` | Echidna | Odlaguna | `SkillMessage` | New skill pending safety validation. |
| `skill/deploy` | Odlaguna | Pandora | `SkillMessage` | Approved skill ready for storage. |
| `task/execute` | Mimi | Ryzu | `TaskMessage` | Execution request for a specific skill. |
| `task/result` | Ryzu | Mimi | `ExecutionResult` | Outcome of a skill execution. |
| `memory/update` | Mimi | Pandora | `MemoryMessage` | Update to the long-term graph memory. |

## 4. Message Format (FlatBuffers)
All messages use FlatBuffers (.fbs) for zero-copy deserialization.

### `schema.fbs`
```fbs
namespace MiMi.Protocol;

enum IntentType : byte { Query, Action, SkillCreation }
enum Priority : byte { Low, Medium, High }

table IntentMessage {
  id: string (required);
  user_message: string (required);
  intent_type: IntentType;
  confidence: float;
  timestamp: long;
}

table TaskMessage {
  id: string (required);
  skill_id: string (required);
  priority: Priority;
  params: [ubyte]; // Serialized skill-specific data
  timeout_ms: uint;
}

table ExecutionResult {
  task_id: string (required);
  success: bool;
  output: string;
  error: string;
  execution_time_ms: uint;
}

table SkillMessage {
  id: string (required);
  name: string (required);
  code: [ubyte] (required); // Rhai script or WASM binary
  language: string; // "rhai", "wasm"
  metadata: string; // JSON metadata
}

union MessageBody { IntentMessage, TaskMessage, ExecutionResult, SkillMessage }

table Envelope {
  version: uint;
  body: MessageBody;
}

root_type Envelope;
```

## 5. Serialization
MiMi uses **FlatBuffers** to achieve zero-copy access.

- **Approach:** Data is accessed directly in the receive buffer without unpacking into intermediate heap objects.
- **Wire Format:** Little-endian binary representation defined by FlatBuffers.
- **Size Estimates:**
  - Small Intent: ~128 - 256 bytes
  - Skill (WASM): 10KB - 2MB (depending on binary size)
  - Result: ~512 bytes

## 6. QoS & Reliability
- **At-Least-Once:** Applied to `task/execute` and `memory/update`. Uses Zenoh's reliability layer with acknowledgments.
- **At-Most-Once:** Applied to high-frequency telemetry or non-critical logs.
- **Retry Logic:** Producers implement exponential backoff (starting at 10ms, max 3 retries) for `Reliable` topics.
- **Deadletter Handling:** Messages failing after 3 retries are published to `sys/deadletter` for Odlaguna to audit.

## 7. Latency Budget
Target end-to-end latency for a local message hop:

| Component | Budget |
|-----------|--------|
| Broker (Zenoh) Latency | < 1ms |
| Serialization (FlatBuffers) | < 50μs |
| Deserialization (FlatBuffers) | < 50μs |
| **Total End-to-End** | **< 5ms** |

## 8. Authentication & Authorization
- **M1 (Foundation):** No authentication. Localhost only.
- **Production (Future):**
  - **TLS:** All traffic encrypted.
  - **mTLS:** Mutual TLS for module authentication.
  - **ACLs:** Topic-based access control (e.g., Beatrice cannot write to `skill/deploy`).

## 9. Monitoring & Observability
- **Metrics:** Exported via Prometheus (messages/sec, byte rate, latency percentiles).
- **Message Tracing:** Every message carries a unique correlation `id` for distributed tracing.
- **Lag Monitoring:** Odlaguna monitors consumer group lag to detect stalled modules.

## 10. Scalability
- **Throughput:** Optimized for 10,000+ small messages/sec on a single node.
- **Burst Handling:** Modules use internal task queues (Tokio channels) to buffer bursts without blocking the bus.
- **Horizontal Scaling:** Zenoh's mesh routing allows bridging to remote workers if local CPU is saturated.

## 11. Error Handling
- **Network Partition:** Zenoh automatically reconnects and synchronizes state when the link is restored.
- **Broker Failure:** If the Zenoh router dies, modules enter "Safety Mode," queuing critical tasks in local storage until the broker recovers.
- **Message Loss:** Reliable topics use sequence numbers to detect gaps; missing messages trigger re-request from producer.

## 12. Testing Strategy
- **Unit Tests:** Verify `.fbs` schema integrity and serialization performance.
- **Integration Tests:** End-to-end flow validation (Beatrice → Mimi → Gemini → Beatrice).
- **Chaos Tests:** Use `tc` (traffic control) to simulate packet loss and latency on the bus loopback.

---
**References:**
- Cross-link: [REQUIREMENTS.md#RF-7](../REQUIREMENTS.md#rf-7)
- Milestone: [M1-FOUNDATION.md](../milestones/M1-FOUNDATION.md)
