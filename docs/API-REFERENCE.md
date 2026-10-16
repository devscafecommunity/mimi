# API Reference (M1)

Consolidated reference for Beatrice CLI, auth, WebSocket messages, and AI adapters. Detailed phase docs remain in `M1.4.*` and `M1.5.*`.

## CLI

Binary: `mimi` (`crates/mimi-cli`).

| Command | Description |
|---------|-------------|
| `mimi exec <TASK>` | Execute a task (priority, timeout, async flags) |
| `mimi query <QUERY>` | Query memory/status |
| `mimi config {set,get,list,reset,validate,init}` | Configuration |
| `mimi debug {status,...}` | Diagnostics |
| `mimi repl` | Interactive shell |

Global flags: `-v`, `--quiet`, `--config`, `--log-level`, `--output`, `--no-color`.

## Authentication

Module: `crates/mimi-cli/src/auth/`.

| API | Purpose |
|-----|---------|
| `AuthManager::new(secret, ttl_secs)` | Create manager |
| `generate_token(&Identity)` | Issue JWT |
| `validate_token(&str)` | Parse and verify JWT |
| `register_default_policies()` | RBAC defaults |

Roles: `User`, `Admin`, etc. (see `auth/types.rs`).

## WebSocket

Types: `crates/mimi-cli/src/ws/types.rs`.

**Client → server (`WsMessage`)**

| Field | Description |
|-------|-------------|
| `id` | Correlation id |
| `msg_type` | `subscribe`, `unsubscribe`, `ping` |
| `channel` | `execute`, `query` |
| `payload` | JSON body |

**Server → client (`WsEvent`)**

| `event_type` | Meaning |
|--------------|---------|
| `start` | Operation started |
| `progress` | Partial progress (`percent`, `message`) |
| `result` | Final payload |
| `error` | Failure |
| `complete` | Stream finished |

## AI adapters

Trait: `AiAdapter` (`crates/mimi-cli/src/ai/adapter.rs`).

| Method | Description |
|--------|-------------|
| `initialize` | Setup from `AdapterInitParams` |
| `capabilities` | Models, streaming, token limits |
| `invoke` | `AiRequest` → `AiResponse` |
| `health_check` | Liveness for registry |
| `cleanup` | Release resources |

**Registry / factory**

- `AdapterRegistry::register_with_health(name, adapter)`
- `AdapterFactory` — config-driven construction (Gemini, Ollama)
- Health: `HealthChecker`, `AdapterPriority`, performance: `PerformanceTracker`

## Mimi core (library)

| Module | Exports |
|--------|---------|
| `state_machine` | `StateManager`, `MimiState`, `Task` |
| `serialization` | `MessageSerializer`, protocol version constants |
| `routing` | `MessageRouter`, `Topic`, `TopicPattern` |
| `zenoh_bus` | `ZenohBusAdapter`, `ZenohConfig` |
| `pandora_client` | `PandoraClient`, `Neo4jConfig` |

## Message serialization

```rust
MessageSerializer::encode_with_version(version, message_id, message_type, payload)?;
MessageSerializer::decode_with_version(bytes)?;
```

Protocol version: `PROTOCOL_VERSION` (currently `1`). See `docs/M1.2.1-FLATBUFFERS-SCHEMA.md` for schema evolution.
