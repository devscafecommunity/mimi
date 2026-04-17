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

**M1.4.5** Implement Beatrice WebSocket server (#216)
- Use tokio-tungstenite or similar for WebSocket
- Implement persistent client connections
- Add subscription model for real-time updates
- Write tests for WebSocket communication

**M1.4.6** Connect Beatrice to Mimi core (#217)
- Implement client-side message marshaling
- Handle Mimi responses and surface to user
- Implement streaming responses for long-running operations
- Write integration tests (Beatrice → Message Bus → Mimi)

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

**M1.6.1** Write end-to-end integration test suite (M1 components) (#223)
- Test: CLI command → Message Bus → Mimi core → AI adapter → response
- Test: HTTP request → Message Bus → Mimi core → response
- Test: WebSocket connection → Message Bus → streaming responses
- Test: Error scenarios (network failure, timeout, invalid input)

**M1.6.2** Write performance benchmarks (M1 components) (#224)
- Benchmark message bus latency (publish/subscribe/request-reply)
- Benchmark FlatBuffers serialization/deserialization
- Benchmark Mimi state machine throughput
- Benchmark Beatrice CLI startup time

**M1.6.3** Write documentation for M1 (#225)
- API documentation (FlatBuffers schema, Mimi core API, Beatrice endpoints)
- Architecture diagrams and sequence diagrams
- Installation and quickstart guide
- Troubleshooting guide for common issues

**M1.6.4** Prepare M1 for deployment (#226)
- Build Docker image for Mimi core
- Create docker-compose for M1 (Zenoh, Mimi, Beatrice server)
- Write deployment checklist and run procedures
- Create monitoring dashboard (if applicable)

---

## M2: PANDORA — Memory Engine with Neo4j & Heatmap

### M2.1: Neo4j Environment & Schema Setup

**M2.1.1** Set up Neo4j database instance
- Create Neo4j Docker container (Community or Enterprise)
- Configure Neo4j parameters (memory, query timeout)
- Set up authentication (username/password)
- Create backup/restore procedures

**M2.1.2** Design Neo4j graph schema
- Define node types (Context, Skill, Parameter, Resource, Metadata)
- Define relationships (HAS, EXECUTES, DEPENDS_ON, REFERENCES)
- Design properties for each node/relationship
- Plan indexing strategy (nodes and relationships to index)

**M2.1.3** Implement Neo4j DDL in Cypher
- Write CREATE CONSTRAINT statements for unique properties
- Write CREATE INDEX statements for query performance
- Write stored procedures for common operations
- Test schema creation and idempotency

**M2.1.4** Design initialization & seeding
- Define initial context seed data (system skills, base resources)
- Write seeding Cypher scripts
- Plan data migration strategy if upgrading schema
- Test seeding and data consistency

---

### M2.2: Heatmap Algorithm & Temperature Decay

**M2.2.1** Design heatmap temperature model
- Define decay formula: T(t) = T₀ × e^(-λ×age)
- Decide on lambda (0.01 = 70s half-life), threshold (0.1), max nodes (500)
- Define temperature scale (0.0 = cold/forgotten, 1.0 = hot/frequent)
- Plan temperature update frequency and batch size

**M2.2.2** Implement temperature decay in C++
- Create HeatmapManager class with update loop
- Implement decay formula with high precision
- Add threshold filtering (remove nodes < threshold)
- Write unit tests for decay calculations

**M2.2.3** Implement heatmap persistence to Neo4j
- Design schema for storing temperatures (node properties or separate relationship)
- Implement periodic batch updates to Neo4j
- Add transaction handling and error recovery
- Write tests for persistence layer

**M2.2.4** Implement heatmap query interface
- Create query methods (get_hot_nodes, get_by_temperature_range, get_recent)
- Implement BFS for context-aware queries (nearby high-temp nodes)
- Add filtering by node type, relationship type
- Write tests for all query types

**M2.2.5** Implement heatmap optimization
- Add in-memory L1 cache for hot nodes (capacity ~1000)
- Implement cache eviction policy (LRU)
- Add cache hit/miss metrics
- Write tests for cache behavior

---

### M2.3: Neo4j Bolt Driver (C++)

**M2.3.1** Evaluate & select C++ Neo4j driver
- Review libneo4j-client and neo4j-cpp-driver options
- Benchmark query performance and resource usage
- Decide on version and update strategy

**M2.3.2** Implement Neo4j connection pool
- Create connection pooling layer (size, timeout, validation)
- Implement connection health checks
- Add automatic reconnection on failure
- Write tests for pool behavior

**M2.3.3** Implement Neo4j query execution layer
- Create query builder for common operations
- Implement result set parsing and type conversion
- Add error handling and logging
- Write tests for query execution

**M2.3.4** Implement Neo4j transaction layer
- Create transaction wrapper (begin, commit, rollback)
- Implement transaction timeout and automatic rollback
- Add nested transaction support (savepoints)
- Write tests for transaction scenarios

**M2.3.5** Implement parameterized queries
- Create parameter binding to prevent injection
- Implement query templating for common patterns
- Add query validation before execution
- Write tests for parameter handling

---

### M2.4: Pandora Memory Engine Core

**M2.4.1** Design Pandora initialization & bootstrapping
- Define startup sequence (Neo4j connect, heatmap initialize, cache prepare)
- Plan warm-up period for cache population
- Design configuration loading from files
- Write tests for initialization

**M2.4.2** Implement Pandora context retrieval
- Create interface for querying context by ID/name
- Implement hierarchical context resolution (specific → general)
- Add filtering by context type, metadata
- Write tests for retrieval performance

**M2.4.3** Implement Pandora context storage
- Create interface for storing new contexts
- Implement automatic temperature initialization (T₀)
- Add validation for context structure
- Write tests for storage operations

**M2.4.4** Implement Pandora context update
- Create interface for updating context properties
- Implement temperature boost on access (refresh T)
- Add validation for update consistency
- Write tests for update scenarios

**M2.4.5** Implement Pandora context relationships
- Create interface for linking contexts (HAS, DEPENDS_ON, etc.)
- Implement relationship validation
- Add relationship queries and traversals
- Write tests for relationship operations

**M2.4.6** Implement Pandora skill registration
- Create interface for registering new skills in graph
- Implement automatic skill indexing and tagging
- Add skill parameter tracking
- Write tests for skill registration

---

### M2.5: LRU Cache Layer (L1 Cache)

**M2.5.1** Design LRU cache architecture
- Define cache capacity (initially ~1000 nodes)
- Define eviction policy (least recently used)
- Plan metrics collection (hit rate, eviction count)
- Design cache coherency (invalidation triggers)

**M2.5.2** Implement LRU cache in Rust
- Create LRU cache structure with O(1) access
- Implement eviction on capacity exceeded
- Add expiration support (TTL)
- Write unit tests for cache operations

**M2.5.3** Implement cache-aware Pandora queries
- Route queries through cache first
- Implement cache miss → Neo4j fetch → cache insert
- Add cache invalidation on context updates
- Write tests for cache-miss and cache-hit paths

**M2.5.4** Implement cache statistics & tuning
- Add metrics (hit rate, eviction rate, capacity utilization)
- Implement dynamic capacity adjustment (if needed)
- Add cache warming on startup
- Write tests for metrics collection

---

### M2.6: Pandora Integration with M1

**M2.6.1** Connect Pandora to Mimi core
- Implement Mimi → Pandora message types
- Add context retrieval calls from Mimi task execution
- Implement context updates after skill execution
- Write integration tests (Mimi → Pandora → Neo4j)

**M2.6.2** Implement Pandora → Neo4j persistence
- Add periodic flush of cached contexts to Neo4j
- Implement transaction batching for efficiency
- Add error handling for persistence failures
- Write integration tests with real Neo4j

**M2.6.3** Implement context-aware task execution
- Modify Mimi executor to fetch context before task
- Pass context to skills during execution
- Update context with execution results
- Write tests for context flow

---

### M2.7: Testing & Benchmarking

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

### M3.1: Docker Isolation & Resource Limits

**M3.1.1** Design container security architecture
- Define resource limits (CPU: 50%, memory: 256MB per skill container)
- Design network isolation (none driver by default, explicit bridges for inter-skill communication)
- Plan logging and audit trail (all container lifecycle events)
- Document security philosophy (defense-in-depth)

**M3.1.2** Implement Skill Container Runtime
- Create Dockerfile for skill execution container
- Define container entry point and signal handling
- Implement resource limits (cpuset, memory cgroup)
- Write tests for container startup/shutdown

**M3.1.3** Implement Container Orchestration
- Create container lifecycle manager (create, start, stop, remove)
- Implement container pooling for performance
- Add container restart on failure (with backoff)
- Write tests for container management

**M3.1.4** Implement Container Networking
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

**M3.2.1** Design Odlaguna architecture & state machine
- Define states: IDLE, MONITORING, ALERTING, INTERVENTION, SHUTDOWN
- Define monitoring triggers (timeout, resource exceeded, crash)
- Plan intervention strategies (terminate, suspend, alert)
- Document state transitions and guard conditions

**M3.2.2** Implement Odlaguna core in Rust
- Create state machine using enum-based pattern
- Implement task monitoring loop (health checks, metrics collection)
- Add signal handling for alerts
- Write unit tests for state machine

**M3.2.3** Implement timeout enforcement
- Create timeout tracking per running task
- Implement timeout detection (elapsed >= configured timeout)
- Implement timeout action (terminate container gracefully, then forcefully)
- Write tests for timeout scenarios

**M3.2.4** Implement resource limit enforcement
- Monitor container CPU and memory usage (via cgroup)
- Detect resource exceeded conditions
- Implement intervention (pause, kill)
- Write tests for resource detection

**M3.2.5** Implement health check & crash recovery
- Define health check protocol (ping, heartbeat, status endpoint)
- Implement health check polling (interval, retries)
- Detect crash or hang (no response)
- Implement automatic restart or alert
- Write tests for health check scenarios

**M3.2.6** Implement audit logging
- Log all Odlaguna interventions (timeout, resource, restart)
- Log all task lifecycle events (start, complete, error)
- Add structured logging for analysis
- Write tests for audit log

---

### M3.3: Circuit Breaker Pattern

**M3.3.1** Design circuit breaker state machine
- Define states: CLOSED (normal), OPEN (failing), HALF_OPEN (testing)
- Define thresholds (consecutive failures = 3 → OPEN, successes = N → CLOSED)
- Plan timeout before HALF_OPEN (exponential backoff)
- Document fallback behavior in OPEN state

**M3.3.2** Implement circuit breaker for Mimi → Pandora calls
- Track consecutive failures (Neo4j calls, context retrieval)
- Implement state transitions (CLOSED → OPEN → HALF_OPEN)
- Add fallback for OPEN state (cached data, empty context)
- Write unit tests for state transitions

**M3.3.3** Implement circuit breaker for Mimi → AI adapter calls
- Track consecutive API failures (Gemini, Ollama)
- Implement state transitions with adaptive thresholds
- Add fallback (retry with alternate adapter)
- Write tests for failure scenarios

**M3.3.4** Implement circuit breaker metrics
- Track state transitions and duration in each state
- Track fallback invocations
- Add alerting for sustained OPEN state
- Write tests for metrics

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

| Milestone | Phases | Focus | Duration |
|-----------|--------|-------|----------|
| **M1: Foundation** | 6 phases | Message bus, Mimi core, Beatrice, Gemini | Infrastructure + first system integration |
| **M2: Pandora** | 7 phases | Neo4j, heatmap, LRU cache, Pandora core | Memory engine + context management |
| **M3: Security** | 6 phases | Docker, Odlaguna, circuit breaker, sandbox | Isolation + monitoring + fault tolerance |
| **M4: Echidna** | 7 phases | Pattern detection, code generation, lifecycle | Autonomous skill generation + learning |
| **Final** | 5 phases | Integration, testing, security validation, docs | System completion + release |

**Total Tasks: ~300+** (across milestones and phases)

**Key Deliverables by Milestone:**
- **M1 END:** Beatrice CLI/HTTP/WebSocket fully functional, Mimi core running, message bus stable
- **M2 END:** Pandora context engine functional, Neo4j integrated, caching working
- **M3 END:** All containers isolated, Odlaguna monitoring active, security model enforced
- **M4 END:** Skill generation pipeline working, patterns learned, autonomous execution verified
- **FINAL END:** Production-ready MiMi system with full documentation and deployment artifacts
