# MiMi Project Task List

**Complete hierarchical breakdown of all project work from start to final delivery.**

Structure: **Milestone → Phase → Tasks**

---

## M1: FOUNDATION — Message Bus & Core System

### M1.1: Project Infrastructure & Setup

**M1.1.1** Set up Git repository structure
- Initialize main repo with .gitignore (Rust, C++, Python, IDE configs)
- Create directory structure: `src/`, `tests/`, `docs/`, `docker/`, `.planning/`
- Configure GitHub org and CI/CD placeholders

**M1.1.2** Set up Rust project scaffold
- Initialize Cargo workspace (mimi-core, mimi-cli)
- Configure Cargo.toml with dependencies (tokio, serde, protobuf/FlatBuffers, tracing)
- Set up rustfmt, clippy, and test infrastructure

**M1.1.3** Set up C++ project scaffold
- Initialize CMake project for Neo4j drivers and utilities
- Configure build system (GCC/Clang, optimization flags)
- Set up unit test framework (GTest or Catch2)

**M1.1.4** Set up Docker environment
- Create docker-compose.yml for Zenoh, Neo4j, Redis
- Define Dockerfile for Mimi service container
- Create docker-compose override for development

**M1.1.5** Set up CI/CD pipeline
- Configure GitHub Actions for Rust builds (test, lint, coverage)
- Configure C++ builds (compile, unit tests)
- Set up automated Docker image builds

---

### M1.2: Message Bus Protocol & Serialization

**M1.2.1** Design FlatBuffers schema for Mimi messages
- Define base message structure (header, payload, metadata)
- Define command message types (QUERY, EXECUTE, RESPONSE, ERROR)
- Define event message types (SKILL_LOADED, CONTEXT_UPDATED, etc.)
- Generate Rust/C++ code from .fbs schema

**M1.2.2** Evaluate & select message bus transport
- Benchmark Zenoh vs NATS performance (latency, throughput, memory)
- Test QoS requirements (at-most-once vs at-least-once)
- Document trade-offs and selection rationale

**M1.2.3** Implement Zenoh client library (Rust)
- Create connection pooling layer
- Implement publish/subscribe wrapper with error handling
- Implement request/reply pattern for synchronous operations
- Write integration tests with local Zenoh instance

**M1.2.4** Implement NATS client library (if selected alternative)
- Create connection pooling layer
- Implement publish/subscribe wrapper
- Implement request/reply pattern
- Write integration tests

**M1.2.5** Define FlatBuffers serialization layer
- Implement encode/decode functions for all message types
- Add version compatibility checks
- Write round-trip tests (serialize → deserialize → verify)

**M1.2.6** Implement message routing middleware
- Create topic hierarchy (e.g., `mimi/commands/*`, `mimi/events/*`)
- Route messages to correct handlers based on topic pattern
- Log message flow for debugging

---

### M1.3: Mimi Core Engine (State Machine & Orchestration)

**M1.3.1** ✅ Design Mimi state machine
- Define states: IDLE, LISTENING, PROCESSING, EXECUTING, ERROR, SHUTDOWN
- Define state transitions and guard conditions
- Document state-specific behavior and side effects

**M1.3.2** ✅ Implement Mimi state machine in Rust
- Use state pattern or enum-based state machine
- Implement state handlers (entry, exit, internal actions)
- Add logging at each state transition
- Write unit tests for each state and transition

**M1.3.3** ✅ Implement task queue & executor
- Create async task queue (tokio::mpsc::channel)
- Implement task scheduling based on priority
- Add timeout enforcement per task
- Write tests for queue ordering and timeout handling

**M1.3.4** ✅ Implement error handling & recovery
- Define error types (network, timeout, validation, execution, module)
- Implement error propagation through state machine
- Add automatic recovery strategies (retry with backoff, circuit breaker)
- Write tests for error scenarios

**M1.3.5** ✅ Implement metrics & observability
- Add structured logging with tracing crate
- Add metrics collection (task count, latency, error rates)
- Integrate with Prometheus exporter (if monitoring required)
- Write tests for log output and metrics

**M1.3.6** ✅ Implement graceful shutdown
- Add shutdown signal handling (SIGTERM, SIGINT)
- Drain task queue and wait for running tasks
- Close all connections cleanly
- Write integration tests for shutdown sequence

**M1.3 STATUS**: ✅ COMPLETE
- 15 commits implementing full state machine pipeline
- 61 total tests (47 unit + 9 integration + 5 acceptance)
- Task priorities, concurrent execution, timeout handling, circuit breaker, health monitoring
- Full Zenoh message bus integration with Neo4j context persistence
- Documentation: state machine design, API docs, architecture diagrams

---

### M1.4: Beatrice CLI Interface

**M1.4.1** Design Beatrice CLI argument parsing (#212)
- Define command structure (mimi [command] [args] [options])
- Design help system and error messages
- Plan command hierarchy (exec, query, config, debug)

**M1.4.2** Implement Beatrice CLI core (#213)
- Use clap or structopt for argument parsing
- Implement command dispatch to handlers
- Add colored output for readability
- Write tests for all command parsing scenarios

**M1.4.3** Implement Beatrice interactive REPL (#214)
- Create prompt and input reading loop
- Implement command history and completion
- Add exit handling and session cleanup
- Write tests for REPL state machine

**M1.4.4** Implement Beatrice HTTP server (#215)
- Use actix-web or axum for HTTP framework
- Define REST API endpoints (/query, /execute, /status)
- Implement request validation and error responses
- Write tests for each endpoint with mock Mimi backend

**M1.4.5** ✅ Implement Beatrice WebSocket server (#216)
- Use tokio-tungstenite or similar for WebSocket
- Implement persistent client connections
- Add subscription model for real-time updates
- Write tests for WebSocket communication
- **Status**: COMPLETE (14 integration tests, comprehensive API docs)
- **Commit**: ff34233 feat(ws): add 14 integration tests and comprehensive API documentation

**M1.4.6** ✅ Connect Beatrice to Mimi core + Auth Integration (#217)
- Implement client-side message marshaling
- Handle Mimi responses and surface to user
- Implement streaming responses for long-running operations
- Write integration tests (Beatrice → Message Bus → Mimi)
- **Auth Module Implementation**: Complete auth/types, auth/manager, auth/token
- **Status**: COMPLETE (7 auth integration tests, comprehensive API docs)
- **Commit**: e8d2e25 feat(auth): complete integration tests (7 tests) and comprehensive API documentation

**M1.4.7** ✅ CLI Integration Tests & Backend Auth Integration (#218)
- CLI workflow integration tests (10 tests)
- Backend + auth integration tests (10 tests)
- Error handling integration tests (10 tests)
- Test utilities and helpers (cli_integration_utils.rs - 350 lines)
- **Status**: COMPLETE (30+ integration tests)
- **Commits**:
  - a7b7ba3 feat(cli-tests): add integration test utilities and helpers
  - 7fe20c4 feat(cli-tests): add 10 CLI workflow integration tests
  - d3f8200 feat(cli-tests): add 10 backend + auth integration tests
  - ed8ebcb feat(cli-tests): add 10 error handling tests and comprehensive CLI documentation

**M1.4.8** ✅ Performance & Load Testing (M1.4.8 Tasks 1-3)
- **Task 1**: Benchmark Infrastructure & Utilities
  - 4 CLI benchmark suites (token gen, validation, permission checks, manager creation)
  - 8 auth benchmark suites (identity lifecycle, role hierarchy, multi-role, registry, scaling, concurrent)
  - 20+ benchmarks with criterion framework, HTML reports
  - **Status**: COMPLETE
  - **Commit**: 4c98da8 perf(benchmarks): add benchmark infrastructure and utilities

- **Task 2**: Load Testing & Stress Tests
  - 10 load tests: throughput, latency, concurrency, scaling, CLI startup
  - LoadMetrics struct with percentile analysis (p50, p95, p99)
  - All performance targets verified (529k ops/sec token gen, 312k validation, 4.1M permission checks)
  - **Status**: COMPLETE (15/15 tests passing)
  - **Commit**: 5c962e3 perf(benchmarks): add 10 load tests and stress testing

- **Task 3**: Memory Profiling & Optimization
  - 10 memory profiling tests (token footprint, validation, manager baseline, policy storage, identity scaling, concurrent stability, CLI startup, leak detection, peak under load, cleanup)
  - MemoryMetrics struct with allocation tracking and leak detection
  - All memory targets met (0.19 MB peak for 1000 identities vs 10 MB target = 52x)
  - No memory leaks detected (1.1% growth <5% threshold)
  - Linear O(n) scaling verified, 200 bytes per identity
  - **Status**: COMPLETE (15/15 tests passing)
  - **Commits**:
    - 6852d59 perf(benchmarks): add memory profiling and optimization analysis
    - 29788a3 docs(performance): update M1.4.8 report with Task 3 memory profiling results

- **Task 4** (OPTIONAL): Final Documentation Enhancement
  - Comprehensive performance report already created (620 lines)
  - Could add: production readiness checklist, optimization recommendations, operations guide
  - **Status**: OPTIONAL - Not blocking, main deliverables complete

---

### M1.5: Gemini AI Adapter

**M1.5.1** Design pluggable AI adapter interface (#218)
- Define Adapter trait/protocol (initialize, invoke, cleanup)
- Define request/response format for LLM calls
- Plan configuration system for adapter parameters
- Document extensibility points for future adapters

**M1.5.2** Implement Gemini adapter (#219)
- Use Google Cloud Generative AI library (Rust or HTTP client)
- Implement connection pooling to Gemini API
- Implement prompt templates and response parsing
- Add API key management and error handling
- Write tests with mock Gemini responses

**M1.5.3** Implement Ollama adapter (local LLM) (#220)
- Use Ollama HTTP API client
- Implement model loading and caching
- Implement streaming response handling
- Add fallback to Gemini if Ollama unavailable
- Write tests with local Ollama instance

**M1.5.4** Implement adapter registry & discovery (#221)
- Create adapter factory pattern
- Implement configuration-driven adapter selection
- Add adapter health checks and fallback logic
- Write tests for adapter switching

**M1.5.5** Implement adapter performance monitoring (#222)
- Add latency tracking per adapter
- Track API call success/error rates
- Implement adaptive timeout adjustment
- Write tests for metrics collection

---

### M1.6: Integration & Testing

**M1.6.1** ✅ Write end-to-end integration test suite (M1 components) (#223)
- 39 e2e tests under `crates/mimi-cli/tests/e2e_*.rs`
- **Status**: COMPLETE — plan in `docs/plans/M1.6.1-e2e-integration-tests.md`

**M1.6.2** ✅ Write performance benchmarks (M1 components) (#224)
- Criterion suites: `mimi-core` (serialization, state machine, Zenoh adapter) + `mimi-cli` startup parsing
- **Status**: COMPLETE — see `docs/M1.6.2-BENCHMARKS.md`

**M1.6.3** ✅ Write documentation for M1 (#225)
- `ARCHITECTURE-OVERVIEW.md`, `SEQUENCE-DIAGRAMS.md`, `QUICKSTART.md`, `API-REFERENCE.md`, `TROUBLESHOOTING.md`
- **Status**: COMPLETE — indexed in `docs/README.md`

**M1.6.4** ✅ Prepare M1 for deployment (#226)
- Dockerfile builds `mimi` CLI; env templates; `DEPLOYMENT.md`, `MONITORING.md`, `SCALING.md`
- **Status**: COMPLETE (Grafana dashboard optional / not blocking)

---

## M2: PANDORA — Memory Engine with Neo4j & Heatmap

### M2.1: Neo4j Environment & Schema Setup

**M2.1.1** ✅ Set up Neo4j database instance
- Neo4j 5.15 in `docker-compose.yml` (`mimi-neo4j`)
- **Status**: READY — see `docs/M2.1-NEO4J-SCHEMA.md`
- Set up authentication (username/password)
- Create backup/restore procedures

**M2.1.2** ✅ Design Neo4j graph schema
- `docker/neo4j/schema.cypher` + `docs/M2.1-NEO4J-SCHEMA.md`
- **Status**: COMPLETE

**M2.1.3** ✅ Implement Neo4j DDL in Cypher
- Constraints/indexes with `IF NOT EXISTS`; apply via `scripts/neo4j-apply-schema.sh`
- **Status**: COMPLETE

**M2.1.4** ✅ Design initialization & seeding
- `docker/neo4j/seed.cypher`; migration notes in M2.1 doc
- **Status**: COMPLETE

---

### M2.2: Heatmap Algorithm & Temperature Decay

**M2.2.1** Design heatmap temperature model
- Define decay formula: T(t) = T₀ × e^(-λ×age)
- Decide on lambda (0.01 = 70s half-life), threshold (0.1), max nodes (500)
- Define temperature scale (0.0 = cold/forgotten, 1.0 = hot/frequent)
- Plan temperature update frequency and batch size

**M2.2.2** ✅ Implement temperature decay (Rust + C++)
- Rust: `crates/mimi-core/src/heatmap.rs`
- C++: `crates/pandora-memory/include/heatmap.hpp`
- **Status**: COMPLETE — unit tests in both crates

**M2.2.3** ✅ Implement heatmap persistence to Neo4j
- `PandoraClient`: `touch_context`, `flush_heatmap`, `hot_contexts` via `neo4rs`
- Doc: `docs/M2.2-HEATMAP-NEO4J.md`; integration test `pandora_neo4j_integration` (`--ignored`)
- **Status**: COMPLETE

**M2.2.4** ✅ Implement heatmap query interface
- `PandoraClient` query API + `resolve_context_for_query`
- CLI: `mimi query --include-context`
- **Status**: COMPLETE — see `docs/M2.2-HEATMAP-NEO4J.md`

**M2.2.5** ✅ Implement heatmap optimization
- `L1ContextCache` in `crates/mimi-core/src/l1_cache.rs` (LRU, capacity 1000)
- Wired into `PandoraClient` (`get_context`, touch/hot paths); `l1_cache_stats()`
- CLI: `mimi debug memory-stats`
- **Status**: COMPLETE — unit tests in `l1_cache.rs`

---

### M2.3: Neo4j Bolt Driver (C++)

**M2.3.1** ✅ Evaluate & select C++ Neo4j driver
- **Decision:** official `neo4j-cpp-driver` behind `pandora::bolt::ConnectionPool` (mock backend for CI)
- Doc: `docs/M2.3-NEO4J-CPP-DRIVER.md`

**M2.3.2** ✅ Implement Neo4j connection pool (mock)
- `crates/pandora-memory/include/bolt_client.hpp` — connect, health, stats

**M2.3.3** ✅ Implement Neo4j query execution layer (mock)
- `ConnectionPool::run`, `QueryResult`, context MERGE/MATCH stubs

**M2.3.4** ✅ Implement Neo4j transaction layer (mock)
- `begin_transaction`, commit, rollback — tests in `test_bolt_client.cpp`

**M2.3.5** ✅ Implement parameterized queries
- `validate_parameters` — required `$placeholders`

**Next:** optional `PANDORA_ENABLE_NEO4J_CPP` + link real driver; reconnect/backoff

---

### M2.4: Pandora Memory Engine Core

**M2.4.1** ✅ Design Pandora initialization & bootstrapping (C++)
- `MemoryManager::initialize` — Bolt ping → LRU + `HeatmapManager`
- Tests: `test_memory_manager.cpp`

**M2.4.2** ✅ Implement Pandora context retrieval (Rust)
- `PandoraClient::get_context`, `search_contexts`, `resolve_context_for_query`

**M2.4.3** ✅ Implement Pandora context storage (Rust)
- `PandoraClient::store_context`

**M2.4.4** ✅ Implement Pandora context update (Rust)
- `touch_context` + heatmap boost; `flush_heatmap`

**M2.4.5** ✅ Implement Pandora context relationships (Rust)
- `link_contexts` + `ContextRelation` enum

**M2.4.6** ✅ Implement Pandora skill registration (Rust, baseline)
- `register_skill(id, name, status)`

**Next:** C++ parity APIs, config files, skill parameters

---

### M2.5: LRU Cache Layer (L1 Cache)

**M2.5.1–M2.5.4** ✅ (Rust + C++)
- Rust: `l1_cache.rs`, `PandoraClient` integration, `mimi debug memory-stats`
- C++: `lru_cache.hpp` + unit tests
- **Next:** cache invalidation on `store_context`, startup warm-up

---

### M2.6: Pandora Integration with M1

**M2.6.1** ✅ Connect Pandora to Mimi core
- `pandora_integration.rs`: `PandoraBusMessage`, `fetch_task_context`, `record_task_outcome`
- Tests: `pandora_mimi_integration`, `pandora_integration` unit tests

**M2.6.2** ✅ Implement Pandora → Neo4j persistence
- `flush_with_retry`, `PandoraFlushLoop` + `PANDORA_FLUSH_INTERVAL_SECS`
- CLI flush after `exec` / `query --include-context`

**M2.6.3** ✅ Implement context-aware task execution
- `execute_task_with_context`, `execute_next_task_with_pandora`
- `mimi exec` injects Pandora `system_context`
- Doc: `docs/M2.6-PANDORA-MIMI.md`

**Next:** Zenoh publish of `PandoraBusMessage`; long-running daemon with flush loop

---

### M2.7: Testing & Benchmarking

**M2.7.0** ✅ M2 acceptance checklist
- `docs/M2.7-ACCEPTANCE.md`; `tests/m2_acceptance.rs`; critérios em `M2-PANDORA.md`

**M2.7.1** Write Pandora unit tests
- Test temperature decay calculations
- Test context CRUD operations
- Test heatmap queries
- Test LRU cache eviction

**M2.7.2** Write Pandora integration tests
- Test Pandora ↔ Neo4j persistence
- Test cache coherency with Neo4j updates
- Test Mimi ↔ Pandora ↔ Neo4j flow
- Test error scenarios (Neo4j down, cache overflow)

**M2.7.3** Write performance benchmarks
- Benchmark context retrieval (cache hit, cache miss, Neo4j direct)
- Benchmark heatmap query performance
- Benchmark Neo4j query latency (100, 1000, 10000 nodes)
- Target: < 5ms for cache hits, < 50ms for Neo4j queries

**M2.7.4** Write Pandora documentation
- Architecture diagrams (cache layers, Neo4j integration)
- API documentation (all public functions)
- Configuration reference (capacity, decay, timeout)
- Troubleshooting guide (query performance, memory issues)

---

## M3: SECURITY — Isolation, Watchdog, Sandboxing

> **Próximo foco:** Priscilla `task/draft` subscriber, analyzers 3–5, `priscilla-rules.toml`. Ver `planning/milestones/M3-SECURITY.md`.

### M3.1: Docker Isolation & Resource Limits

**M3.1.1** ✅ Design container security architecture
- Doc: `docs/M3.1-DOCKER-ISOLATION.md`

**M3.1.2** ✅ Implement Skill Container Runtime (baseline)
- `docker/Dockerfile.worker`, `docker/worker-entry.sh`
- `crates/ryzu-runtime` (`MockSkillRuntime`, optional `docker` + Bollard)

**M3.1.3** 🟡 Container orchestration (baseline)
- ✅ `BollardSkillRuntime`: create/start, wait/finish, `kill_container` (Odlaguna)
- ✅ `run_bollard_skill_supervised` — lease + `intervene_expired_containers` → kill (`docker-kill`)
- ✅ `run_isolated_skill_supervised` + `execute_task_with_context` payload + `mimi debug skill-run`
- ✅ `SkillContainerPool` (concurrency cap) + `run_with_restart` / `run_managed`
- Create container lifecycle manager (create, start, stop, remove) — partial via `run_once` / `kill_container`
- ✅ Warm pool baseline — `ryzu-runtime/warm_pool.rs` (`prewarm_image` + `SkillContainerPool` in `BollardSkillRuntime`)
- ✅ Add container restart on failure (with backoff) — `RestartPolicy`
- Write tests for container management

**M3.1.4** 🟡 Container networking (baseline test)
- ✅ `docker_integration` test `network_none_blocks_egress` (`DOCKER_INTEGRATION=1`)
- Design network configuration for skill isolation
- Implement inter-skill communication if needed (explicit bridges)
- Add network policy enforcement
- Write tests for network isolation

**M3.1.5** Implement Container Logging & Monitoring
- Capture container stdout/stderr to centralized log
- Add container resource usage metrics
- Implement log rotation and retention
- Write tests for log capture

---

### M3.2: Odlaguna Watchdog System

**M3.2.1** ✅ Design Odlaguna architecture & state machine
- Doc: `docs/M3.2-ODLAGUNA.md`

**M3.2.2** ✅ Implement Odlaguna core in Rust (baseline)
- `mimi-core/src/odlaguna.rs` — register/complete/poll_expired + audit trail

**M3.2.3** 🟡 Timeout enforcement (baseline)
- ✅ `odlaguna_deadline` + `mimi exec` + `execute_task_with_context`
- Create timeout tracking per running task
- Implement timeout detection (elapsed >= configured timeout)
- Implement timeout action (terminate container gracefully, then forcefully)
- Write tests for timeout scenarios

**M3.2.4** 🟡 Resource limit enforcement (baseline)
- ✅ Docker stats polling (`cgroup.rs`) during `finish_skill_container` → `ResourceExceeded` + kill
- Monitor container CPU and memory usage (via cgroup)
- Detect resource exceeded conditions
- Implement intervention (pause, kill)
- Write tests for resource detection

**M3.2.5** 🟡 Health check & crash recovery (baseline)
- ✅ Worker `mimi-heartbeat` + `heartbeat_interval_secs` + `HeartbeatLost`
- ✅ `ryzu-runtime/src/heartbeat.rs` log polling
- Define health check protocol (ping, heartbeat, status endpoint)
- Implement health check polling (interval, retries)
- Detect crash or hang (no response)
- Implement automatic restart or alert
- Write tests for health check scenarios

**M3.2.6** 🟡 Implement audit logging (baseline)
- ✅ `OdlagunaAudit` Neo4j + in-memory (`record_odlaguna_intervention`, `emit_intervention_side_effects`)
- ✅ Zenoh `mimi/task/timeout` on expired leases (`MIMI_ZENOH=1` on `mimi exec`)
- Log all task lifecycle events (start, complete, error) — partial (`odlaguna.audit` tracing)
- Write tests for audit log — ✅ `odlaguna_intervention::tests::audit_and_zenoh_hooks_run`

---

### M3.3: Circuit Breaker Pattern

**M3.3.1** ✅ Circuit breaker state machine (existing `state_machine::CircuitBreaker`)

**M3.3.x baseline** 🟡
- ✅ `with_circuit_breaker` + Pandora/AI on `mimi exec` and `mimi query`
- ✅ `CircuitBreakerSnapshot` metrics (`circuit_metrics.rs`, `breaker.snapshot()`)
- Alternate adapter fallback + daemon-wide breakers — pending

**M3.3.1 (detail)** Design circuit breaker state machine
- Define states: CLOSED (normal), OPEN (failing), HALF_OPEN (testing)
- Define thresholds (consecutive failures = 3 → OPEN, successes = N → CLOSED)
- Plan timeout before HALF_OPEN (exponential backoff)
- Document fallback behavior in OPEN state

**M3.3.2** 🟡 Circuit breaker Mimi → Pandora (baseline)
- ✅ `circuit_breakers()` + `pandora_context_with_fallback` + L1 fallback
- ✅ Test `pandora_open_uses_l1_when_populated`
- Neo4j-persisted breaker state — pending

**M3.3.3** 🟡 Circuit breaker Mimi → AI (baseline)
- ✅ `invoke_ai_with_fallback` + registry alternate adapter
- Adaptive thresholds — pending

**M3.3.4** 🟡 Circuit breaker metrics + alerts (baseline)
- ✅ `CircuitBreakerSnapshot`, `sustained_open_alerts`, `circuit.alert` log (`SUSTAINED_OPEN_ALERT_MS`)
- ✅ `docker_skill_integration` + `ryzu-runtime` `docker_integration` (`#[ignore]`)
- Prometheus/Grafana export — pending

---

### T3.5: Priscilla (baseline)

- ✅ `mimi-core/priscilla.rs` — `TaskDraftMessage`, `CritiqueCommentaryMessage`, necessity/cost heuristics
- ✅ `mimi debug priscilla-critique --text "..."` (JSON with `--output json`)
- Subscriber `task/draft`, analyzers 3–5, `priscilla-rules.toml` — pending

### T3.6: Priscilla failure patterns (baseline)

- ✅ `priscilla_failure.rs` — temporal weighting + intent similarity
- ✅ `PandoraClient::query_failure_patterns` (in-memory audit + Neo4j)
- ✅ `critique_with_pandora` + `--with-patterns` on CLI

### M3 daemon / bus

- ✅ `mimi daemon` — `PandoraFlushLoop` + `OdlagunaSupervisorLoop`
- ✅ `InterventionContext` on `run_isolated_skill_supervised` / `execute_task_with_context`
- ✅ Zenoh native behind `zenoh-bus` feature (`ZENOH_*` env)

---

### M3.4: WASM Sandboxing (Future Enhancement)

**M3.4.1** Design WASM sandbox architecture
- Plan WASM runtime selection (Wasmtime, Wasmer)
- Define resource limits (memory: 256MB, CPU: execution steps limit)
- Plan system call restrictions (whitelist allowed functions)
- Document security model for skill execution

**M3.4.2** Implement WASM skill executor
- Create WASM module loader and validator
- Implement resource limit enforcement (memory, CPU)
- Add system function whitelist (file I/O, network, time)
- Write tests for WASM execution

**M3.4.3** Implement WASM ↔ Rust FFI boundary
- Create type conversion layer (WASM types ↔ Rust types)
- Implement secure function calls with parameter validation
- Add return value serialization
- Write tests for FFI boundary

---

### M3.5: Error Handling & Graceful Degradation

**M3.5.1** Implement error recovery strategies
- Define error classification (recoverable vs fatal)
- Implement retry logic with exponential backoff
- Implement cascading fallbacks (e.g., Gemini → Ollama → cached response)
- Write tests for recovery logic

**M3.5.2** Implement graceful degradation
- Plan reduced functionality mode when components unavailable (no Pandora → minimal context)
- Implement feature detection (check component availability)
- Add user notifications for degraded mode
- Write tests for degradation scenarios

**M3.5.3** Implement security-focused error logging
- Log security events (unauthorized access attempts, resource violations)
- Implement log encryption for sensitive data
- Add structured logging for security analysis
- Write tests for security logging

---

### M3.6: Security Testing & Validation

**M3.6.1** Write security unit tests
- Test resource limit enforcement
- Test timeout enforcement
- Test circuit breaker state transitions
- Test error scenarios and fallbacks

**M3.6.2** Write security integration tests
- Test container isolation (skill cannot access another skill's data)
- Test network isolation (skill cannot connect to external networks)
- Test resource limit enforcement under load
- Test timeout enforcement with long-running operations

**M3.6.3** Write security benchmarks
- Benchmark container startup time
- Benchmark container resource overhead
- Benchmark Odlaguna monitoring overhead
- Target: < 100ms startup, < 10% monitoring overhead

**M3.6.4** Write security documentation
- Security model and threat model
- Container hardening guide
- Configuration guide (resource limits, timeouts)
- Incident response procedures

---

## M4: ECHIDNA — Skills Generator & Pattern Detection

### M4.1: Pattern Detection System

**M4.1.1** Design pattern detection architecture
- Define pattern types (sequential, conditional, repetitive, data-flow)
- Plan pattern matching algorithm (AST-based, regex-based, or hybrid)
- Design pattern repository storage (database schema)
- Document extensibility for new pattern types

**M4.1.2** Implement pattern matcher
- Create pattern matching engine (AST parsing for code examples)
- Implement pattern scoring (confidence, specificity)
- Add pattern deduplication (avoid registering duplicate patterns)
- Write tests for pattern matching

**M4.1.3** Implement pattern storage
- Design Neo4j schema for pattern storage (PATTERN nodes, relationships)
- Implement pattern CRUD operations
- Add pattern metadata (author, creation date, usage count)
- Write tests for pattern storage

**M4.1.4** Implement pattern discovery
- Create algorithm for finding common patterns in codebase
- Implement pattern extraction from user instructions
- Add pattern ranking by usefulness
- Write tests for pattern discovery

**M4.1.5** Implement pattern learning from execution
- Track skill execution and outcomes
- Detect patterns in successful executions
- Add patterns to repository
- Write tests for learning pipeline

---

### M4.2: Code Generation Engine

**M4.2.1** Design code generation architecture
- Plan target languages (Rust initial, C++ future, Python future)
- Design AST-based code generation
- Plan template system for code boilerplate
- Document extensibility for new languages

**M4.2.2** Implement code generator core
- Create AST node types for target language
- Implement AST-to-code conversion
- Add formatting and style enforcement
- Write tests for code generation

**M4.2.3** Implement template system
- Create templates for common structures (main function, error handling, logging)
- Implement template variable substitution
- Add template composition (nested templates)
- Write tests for template rendering

**M4.2.4** Implement code validation
- Create syntax validator (parse generated code)
- Add type checker integration
- Implement lint checks on generated code
- Write tests for validation

**M4.2.5** Implement code optimization
- Add dead code elimination
- Implement constant folding
- Add variable inlining
- Write tests for optimization

---

### M4.3: Rhai Script Integration

**M4.3.1** Design Rhai integration architecture
- Plan Rhai script lifecycle (compile, execute, cleanup)
- Design API for Rhai scripts (available functions, data structures)
- Plan security model (no file access, no network, CPU limits)
- Document script examples for common use cases

**M4.3.2** Implement Rhai script executor
- Create Rhai engine and script compiler
- Implement resource limits (execution steps, memory)
- Add error handling and recovery
- Write tests for script execution

**M4.3.3** Implement Rhai-Rust FFI
- Create function registry for Rust callbacks
- Implement type conversion (Rhai ↔ Rust)
- Add return value handling
- Write tests for FFI communication

**M4.3.4** Implement Rhai debugging
- Add script tracing (function entry/exit)
- Implement breakpoint support (if needed)
- Add variable inspection during execution
- Write tests for debugging

---

### M4.4: WASM Code Generation & Execution

**M4.4.1** Design WASM generation architecture
- Plan target WASM features (linear memory, function calls)
- Design code generation to WASM bytecode (manual or via IR)
- Plan linking strategy (imports for Rust functions)
- Document performance expectations

**M4.4.2** Implement WASM code generator
- Create IR-to-WASM compiler
- Implement WASM module builder
- Add function generation and linking
- Write tests for WASM generation

**M4.4.3** Implement WASM executor
- Create WASM module loader
- Implement function invocation
- Add resource limit enforcement
- Write tests for execution

**M4.4.4** Implement WASM-Rust FFI
- Create import function registry
- Implement parameter marshaling
- Add return value unmarshaling
- Write tests for FFI

---

### M4.5: Skill Lifecycle Management

**M4.5.1** Design 7-phase skill lifecycle
- Phase 1: Detection (find new skill pattern in user instructions)
- Phase 2: Extraction (extract skill definition from pattern)
- Phase 3: Generation (generate skill code from definition)
- Phase 4: Validation (verify generated code is correct)
- Phase 5: Deployment (register skill in system and Neo4j)
- Phase 6: Execution (run skill with caching)
- Phase 7: Learning (collect execution results, update patterns)

**M4.5.2** Implement skill state machine
- Create state machine for 7-phase lifecycle
- Implement state handlers and transitions
- Add error handling for failed phases
- Write tests for state machine

**M4.5.3** Implement skill cache
- Design skill cache (in-memory hash map by skill ID)
- Implement cache invalidation (on skill update)
- Add cache hit tracking
- Write tests for caching

**M4.5.4** Implement skill versioning
- Design version schema (semantic versioning)
- Implement version history storage in Neo4j
- Add rollback capability (revert to previous version)
- Write tests for versioning

**M4.5.5** Implement skill dependency resolution
- Detect skill dependencies (skills that call other skills)
- Implement topological sort for execution order
- Add circular dependency detection
- Write tests for dependency resolution

**M4.5.6** Implement skill execution orchestration
- Create executor for generated skills
- Implement parameter passing and result collection
- Add context propagation to child skills
- Write tests for orchestration

---

### M4.6: Integration with M1-M3 Components

**M4.6.1** Connect Echidna to Mimi core
- Implement skill generation on user instruction
- Integrate generated skills into Mimi executor
- Add skill caching to Mimi
- Write integration tests (Mimi → Echidna → generated skill execution)

**M4.6.2** Connect Echidna to Pandora
- Store generated skills in Neo4j
- Track skill metadata (version, dependencies, creation date)
- Link skills to patterns and contexts
- Write tests for Pandora ↔ Echidna integration

**M4.6.3** Connect Echidna to Odlaguna
- Add Odlaguna monitoring for generated skills
- Implement timeout enforcement for generated code
- Add resource limits for generated WASM
- Write tests for security monitoring

**M4.6.4** Implement skill performance monitoring
- Track execution time per skill
- Track error rates and types
- Implement adaptive optimization (mark slow skills for optimization)
- Write tests for monitoring

---

### M4.7: Testing & Validation

**M4.7.1** Write Echidna unit tests
- Test pattern matching and detection
- Test code generation correctness
- Test Rhai script execution
- Test WASM generation and execution
- Test skill lifecycle state machine

**M4.7.2** Write Echidna integration tests
- Test skill generation pipeline (detection → deployment → execution)
- Test generated skills with various input types
- Test skill dependency resolution
- Test error scenarios (generation failure, execution failure)

**M4.7.3** Write performance benchmarks
- Benchmark pattern matching time
- Benchmark code generation time (target: < 100ms)
- Benchmark generated code execution time (Rhai vs WASM)
- Benchmark skill caching effectiveness

**M4.7.4** Write Echidna documentation
- Architecture diagrams (7-phase lifecycle)
- Pattern specification guide (how to define patterns)
- Generated skill testing guide
- Performance tuning guide

---

## Project Completion

### Final Integration & System Tests

**Final.1** End-to-end system test (all M1-M4 components)
- Full workflow: User instruction → CLI → Mimi → Beatrice → AI adapter → skill detection → skill generation → skill execution → context update → response
- Test all component interactions (Message bus, state machine, Neo4j, watchdog, WASM)
- Test error scenarios across entire system

**Final.2** Load & stress testing
- Run 100+ concurrent tasks through Mimi
- Monitor resource usage (CPU, memory, disk)
- Verify no data corruption or lost messages
- Target: 90th percentile latency < 10 seconds per task

**Final.3** Security validation
- Penetration testing (attempt to escape container, access unauthorized data)
- Audit trail verification (all security events logged)
- Resource limit enforcement validation
- Compliance check against security model

**Final.4** Documentation completion
- API reference for all public interfaces
- Architecture decision record (ADR) for major decisions
- Operations manual (deployment, monitoring, troubleshooting)
- Performance tuning guide

**Final.5** Release preparation
- Version all components
- Create release notes
- Tag Git commit for release
- Generate release artifacts (Docker images, compiled binaries)

---

## Summary by Phase

| Milestone | Phases | Focus | Status | Duration |
|-----------|--------|-------|--------|----------|
| **M1.1** | 5 phases | Project Infrastructure | ✅ COMPLETE | Setup |
| **M1.2** | 6 phases | Message Bus Protocol | ✅ COMPLETE | Communication |
| **M1.3** | 6 phases | Mimi Core State Machine | ✅ COMPLETE | Core Engine (61 tests) |
| **M1.4** | 8+ phases | Beatrice CLI Interface | 🚀 IN PROGRESS | CLI + Performance |
| **M1.5** | 5 phases | Gemini AI Adapter | ⏳ PENDING | AI Integration |
| **M1.6** | 4 phases | Integration & Testing | ⏳ PENDING | System Integration |
| **M2: Pandora** | 7 phases | Memory Engine | ⏳ PENDING | Neo4j + Heatmap |
| **M3: Security** | 6 phases | Isolation & Watchdog | ⏳ PENDING | Docker + Monitoring |
| **M4: Echidna** | 7 phases | Skills Generator | ⏳ PENDING | Code Generation |
| **Final** | 5 phases | Integration & Deployment | ⏳ PENDING | Release |

**M1.4 Progress**:
- M1.4.1-M1.4.4: Design + partial implementation
- ✅ M1.4.5: WebSocket server complete (14 integration tests)
- ✅ M1.4.6: Auth module + Beatrice integration complete (7 integration tests)
- ✅ M1.4.7: CLI integration tests complete (30+ tests: workflow, backend, error handling)
- ✅ M1.4.8: Performance & Load Testing complete (Tasks 1-3: benchmarks, load tests, memory profiling)
  - Task 1: 20+ benchmark suites with criterion
  - Task 2: 10 load tests, all throughput targets exceeded (529k token gen, 312k validation, 4.1M permission checks)
  - Task 3: 10 memory tests, all targets met (0.19 MB peak vs 10 MB target), zero memory leaks
  - Task 4: Optional documentation enhancement

**Total M1.4 Deliverables**: 80+ tests across WebSocket, auth, CLI integration, and performance testing

**Total Tests in Project**: 100+ (47 unit + 40+ integration + 15 load + 15 memory profiling)

**Total Commits**: 97 commits across all milestones

**Key Deliverables by Milestone:**
- **M1 END:** Beatrice CLI/HTTP/WebSocket fully functional ✅, Mimi core running ✅, message bus stable ✅, auth system complete ✅, performance baseline established ✅
- **M1.4 END (CURRENT)**: Performance & Load Testing complete (Tasks 1-3), comprehensive documentation, all targets met
- **M2 END:** Pandora context engine functional, Neo4j integrated, caching working
- **M3 END:** All containers isolated, Odlaguna monitoring active, security model enforced
- **M4 END:** Skill generation pipeline working, patterns learned, autonomous execution verified
- **FINAL END:** Production-ready MiMi system with full documentation and deployment artifacts
