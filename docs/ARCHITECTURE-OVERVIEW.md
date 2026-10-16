# MiMi Architecture Overview

MiMi is a cognitive orchestration stack: **Beatrice** (I/O), **Mimi core** (state + tasks + routing), **message bus** (Zenoh), **AI adapters** (Gemini/Ollama), and **Pandora** (Neo4j memory, C++).

## System diagram

```text
                    ┌─────────────────────────────────────────┐
                    │              Beatrice (mimi-cli)         │
                    │  CLI · REPL · Auth · WS types · Adapters │
                    └───────────────┬─────────────────────────┘
                                    │
                         FlatBuffers / JSON payloads
                                    │
                    ┌───────────────▼─────────────────────────┐
                    │           Mimi Core (mimi-core)          │
                    │  StateManager · Task queue · Routing     │
                    │  Serialization · Health · Zenoh adapter  │
                    └───────┬─────────────────────┬───────────┘
                            │                     │
                     Zenoh bus              Pandora client
                            │                     │
              ┌─────────────▼─────────┐   ┌───────▼──────────┐
              │  Zenoh (docker)       │   │ Neo4j + Redis    │
              │  pub/sub · req/reply  │   │ graph + L1 cache │
              └───────────────────────┘   └──────────────────┘
                            │
              ┌─────────────▼─────────┐
              │  External LLMs        │
              │  Gemini · Ollama      │
              └───────────────────────┘
```

## Crates

| Crate | Role |
|-------|------|
| `crates/mimi-core` | FSM, task executor, message serialization, routing middleware, Zenoh integration surface |
| `crates/mimi-cli` | Beatrice: CLI/REPL, JWT auth, AI adapter registry/factory, WebSocket message types |
| `crates/pandora-memory` | C++ LRU + thermal decay; Neo4j Bolt integration (M2) |

## Message flow (query)

1. User invokes `mimi query "..."` or a WebSocket `query` channel message.
2. Beatrice validates input and resolves auth (when enabled).
3. Request is encoded (`MessageSerializer`) and routed (`MessageRouter` topics under `mimi/`).
4. `StateManager` moves `Idle → Listening → Processing → …` and schedules work on the task queue.
5. `AdapterRegistry` selects a healthy adapter (Gemini/Ollama) with fallback and performance tracking.
6. Response returns via CLI output, HTTP (when server is bound), or WebSocket event stream.

## State machine

States: `Idle`, `Listening`, `Processing`, `Executing`, `Responding`, `Degraded`, `Recovering`, `FailedComponent`, `CriticalError`, `Shutdown`.

Invalid transitions are rejected; health guards can force `Degraded` or `Recovering`. See `crates/mimi-core/src/state_machine.rs`.

## Configuration

- Workspace: root `Cargo.toml`
- CLI defaults: `crates/mimi-cli/src/config/`
- Docker stack: `docker-compose.yml` + `.env.development.example`

## Related docs

- [SEQUENCE-DIAGRAMS.md](./SEQUENCE-DIAGRAMS.md)
- [API-REFERENCE.md](./API-REFERENCE.md)
- [QUICKSTART.md](./QUICKSTART.md)
- Phase docs: `M1.2.*` through `M1.5.*` in `/docs`
