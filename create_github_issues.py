#!/usr/bin/env python3
"""
Generate GitHub issues from TASKLIST.md
Creates issues for all tasks not yet in GitHub
"""

import subprocess
import json
import os
import sys
from typing import Dict, List, Tuple

# GitHub token from environment
GH_TOKEN = os.getenv('GH_TOKEN', '')
REPO = 'devscafecommunity/mimi'

# Existing issues (from GitHub review)
EXISTING_ISSUES = {
    'M1.1.1': 1,
    'M1.1.2': 2,
    'M1.1.3': 3,
    'M1.1.4': 4,
    'M1.1.5': 5,
    'M1.2.1': 6,
    'M1.2.2': 7,
    'M1.2.3': 8,
    'M1.2.4': 9,
    'M1.2.5': 10,
    'M1.2.6': 11,
    'M1.3.1': 12,
    'M1.3.2': 13,
    'M1.3.3': 14,
    'M1.3.4': 15,
    'M1.3.5': 16,
    'M1.3.6': 17,
}

# Milestone mapping
MILESTONE_MAP = {
    'M1.1': 'M1',
    'M1.2': 'M1.2',
    'M1.3': 'M1.3',
    'M1.4': 'M1.4',
    'M1.5': 'M1.5',
    'M1.6': 'M1.6',
    'M2.1': 'M2',
    'M2.2': 'M2',
    'M2.3': 'M2',
    'M2.4': 'M2',
    'M2.5': 'M2',
    'M2.6': 'M2',
    'M2.7': 'M2',
    'M3.1': 'M3',
    'M3.2': 'M3',
    'M3.3': 'M3',
    'M3.4': 'M3',
    'M3.5': 'M3',
    'M3.6': 'M3',
    'M4.1': 'M4',
    'M4.2': 'M4',
    'M4.3': 'M4',
    'M4.4': 'M4',
    'M4.5': 'M4',
    'M4.6': 'M4',
    'M4.7': 'M4',
    'Final': 'Final',
}

# All tasks from TASKLIST.md (structured data)
ALL_TASKS = [
    # M1.1
    ('M1.1.1', 'Set up Git repository structure', 'Initialize main repo with .gitignore (Rust, C++, Python, IDE configs). Create directory structure: src/, tests/, docs/, docker/, .planning/'),
    ('M1.1.2', 'Set up Rust project scaffold', 'Initialize Cargo workspace (mimi-core, mimi-cli). Configure Cargo.toml with dependencies (tokio, serde, protobuf/FlatBuffers, tracing).'),
    ('M1.1.3', 'Set up C++ project scaffold', 'Initialize CMake project for Neo4j drivers and utilities. Configure build system (GCC/Clang, optimization flags).'),
    ('M1.1.4', 'Set up Docker environment', 'Create docker-compose.yml for Zenoh, Neo4j, Redis. Define Dockerfile for Mimi service container.'),
    ('M1.1.5', 'Set up CI/CD pipeline', 'Configure GitHub Actions for Rust builds (test, lint, coverage). Configure C++ builds (compile, unit tests).'),
    
    # M1.2
    ('M1.2.1', 'Design FlatBuffers schema for Mimi messages', 'Define base message structure (header, payload, metadata). Define command message types (QUERY, EXECUTE, RESPONSE, ERROR).'),
    ('M1.2.2', 'Evaluate & select message bus transport', 'Benchmark Zenoh vs NATS performance (latency, throughput, memory). Test QoS requirements (at-most-once vs at-least-once).'),
    ('M1.2.3', 'Implement Zenoh client library (Rust)', 'Create connection pooling layer. Implement publish/subscribe wrapper with error handling.'),
    ('M1.2.4', 'Implement NATS client library (if selected alternative)', 'Create connection pooling layer. Implement publish/subscribe wrapper.'),
    ('M1.2.5', 'Define FlatBuffers serialization layer', 'Implement encode/decode functions for all message types. Add version compatibility checks.'),
    ('M1.2.6', 'Implement message routing middleware', 'Create topic hierarchy (e.g., mimi/commands/*, mimi/events/*). Route messages to correct handlers based on topic pattern.'),
    
    # M1.3
    ('M1.3.1', 'Design Mimi state machine', 'Define states: IDLE, LISTENING, PROCESSING, EXECUTING, ERROR, SHUTDOWN. Define state transitions and guard conditions.'),
    ('M1.3.2', 'Implement Mimi state machine in Rust', 'Use state pattern or enum-based state machine. Implement state handlers (entry, exit, internal actions).'),
    ('M1.3.3', 'Implement task queue & executor', 'Create async task queue (tokio::mpsc::channel). Implement task scheduling based on priority.'),
    ('M1.3.4', 'Implement error handling & recovery', 'Define error types (network, timeout, validation, execution, module). Implement error propagation through state machine.'),
    ('M1.3.5', 'Implement metrics & observability', 'Add structured logging with tracing crate. Add metrics collection (task count, latency, error rates).'),
    ('M1.3.6', 'Implement graceful shutdown', 'Add shutdown signal handling (SIGTERM, SIGINT). Drain task queue and wait for running tasks.'),
    
    # M1.4
    ('M1.4.1', 'Design Beatrice CLI argument parsing', 'Define command structure (mimi [command] [args] [options]). Design help system and error messages.'),
    ('M1.4.2', 'Implement Beatrice CLI core', 'Use clap or structopt for argument parsing. Implement command dispatch to handlers.'),
    ('M1.4.3', 'Implement Beatrice interactive REPL', 'Create prompt and input reading loop. Implement command history and completion.'),
    ('M1.4.4', 'Implement Beatrice HTTP server', 'Use actix-web or axum for HTTP framework. Define REST API endpoints (/query, /execute, /status).'),
    ('M1.4.5', 'Implement Beatrice WebSocket server', 'Use tokio-tungstenite or similar for WebSocket. Implement persistent client connections.'),
    ('M1.4.6', 'Connect Beatrice to Mimi core', 'Implement client-side message marshaling. Handle Mimi responses and surface to user.'),
    
    # M1.5
    ('M1.5.1', 'Design pluggable AI adapter interface', 'Define Adapter trait/protocol (initialize, invoke, cleanup). Define request/response format for LLM calls.'),
    ('M1.5.2', 'Implement Gemini adapter', 'Use Google Cloud Generative AI library (Rust or HTTP client). Implement connection pooling to Gemini API.'),
    ('M1.5.3', 'Implement Ollama adapter (local LLM)', 'Use Ollama HTTP API client. Implement model loading and caching.'),
    ('M1.5.4', 'Implement adapter registry & discovery', 'Create adapter factory pattern. Implement configuration-driven adapter selection.'),
    ('M1.5.5', 'Implement adapter performance monitoring', 'Add latency tracking per adapter. Track API call success/error rates.'),
    
    # M1.6
    ('M1.6.1', 'Write end-to-end integration test suite (M1 components)', 'Test: CLI command → Message Bus → Mimi core → AI adapter → response. Test: HTTP request → Message Bus → Mimi core → response.'),
    ('M1.6.2', 'Write performance benchmarks (M1 components)', 'Benchmark message bus latency (publish/subscribe/request-reply). Benchmark FlatBuffers serialization/deserialization.'),
    ('M1.6.3', 'Write documentation for M1', 'API documentation (FlatBuffers schema, Mimi core API, Beatrice endpoints). Architecture diagrams and sequence diagrams.'),
    ('M1.6.4', 'Prepare M1 for deployment', 'Build Docker image for Mimi core. Create docker-compose for M1 (Zenoh, Mimi, Beatrice server).'),
    
    # M2.1
    ('M2.1.1', 'Set up Neo4j database instance', 'Create Neo4j Docker container (Community or Enterprise). Configure Neo4j parameters (memory, query timeout).'),
    ('M2.1.2', 'Design Neo4j graph schema', 'Define node types (Context, Skill, Parameter, Resource, Metadata). Define relationships (HAS, EXECUTES, DEPENDS_ON, REFERENCES).'),
    ('M2.1.3', 'Implement Neo4j DDL in Cypher', 'Write CREATE CONSTRAINT statements for unique properties. Write CREATE INDEX statements for query performance.'),
    ('M2.1.4', 'Design initialization & seeding', 'Define initial context seed data (system skills, base resources). Write seeding Cypher scripts.'),
    
    # M2.2
    ('M2.2.1', 'Design heatmap temperature model', 'Define decay formula: T(t) = T₀ × e^(-λ×age). Decide on lambda (0.01 = 70s half-life), threshold (0.1), max nodes (500).'),
    ('M2.2.2', 'Implement temperature decay in C++', 'Create HeatmapManager class with update loop. Implement decay formula with high precision.'),
    ('M2.2.3', 'Implement heatmap persistence to Neo4j', 'Design schema for storing temperatures (node properties or separate relationship). Implement periodic batch updates to Neo4j.'),
    ('M2.2.4', 'Implement heatmap query interface', 'Create query methods (get_hot_nodes, get_by_temperature_range, get_recent). Implement BFS for context-aware queries (nearby high-temp nodes).'),
    ('M2.2.5', 'Implement heatmap optimization', 'Add in-memory L1 cache for hot nodes (capacity ~1000). Implement cache eviction policy (LRU).'),
    
    # M2.3
    ('M2.3.1', 'Evaluate & select C++ Neo4j driver', 'Review libneo4j-client and neo4j-cpp-driver options. Benchmark query performance and resource usage.'),
    ('M2.3.2', 'Implement Neo4j connection pool', 'Create connection pooling layer (size, timeout, validation). Implement connection health checks.'),
    ('M2.3.3', 'Implement Neo4j query execution layer', 'Create query builder for common operations. Implement result set parsing and type conversion.'),
    ('M2.3.4', 'Implement Neo4j transaction layer', 'Create transaction wrapper (begin, commit, rollback). Implement transaction timeout and automatic rollback.'),
    ('M2.3.5', 'Implement parameterized queries', 'Create parameter binding to prevent injection. Implement query templating for common patterns.'),
    
    # M2.4
    ('M2.4.1', 'Design Pandora initialization & bootstrapping', 'Define startup sequence (Neo4j connect, heatmap initialize, cache prepare). Plan warm-up period for cache population.'),
    ('M2.4.2', 'Implement Pandora context retrieval', 'Create interface for querying context by ID/name. Implement hierarchical context resolution (specific → general).'),
    ('M2.4.3', 'Implement Pandora context storage', 'Create interface for storing new contexts. Implement automatic temperature initialization (T₀).'),
    ('M2.4.4', 'Implement Pandora context update', 'Create interface for updating context properties. Implement temperature boost on access (refresh T).'),
    ('M2.4.5', 'Implement Pandora context relationships', 'Create interface for linking contexts (HAS, DEPENDS_ON, etc.). Implement relationship validation.'),
    ('M2.4.6', 'Implement Pandora skill registration', 'Create interface for registering new skills in graph. Implement automatic skill indexing and tagging.'),
    
    # M2.5
    ('M2.5.1', 'Design LRU cache architecture', 'Define cache capacity (initially ~1000 nodes). Define eviction policy (least recently used).'),
    ('M2.5.2', 'Implement LRU cache in Rust', 'Create LRU cache structure with O(1) access. Implement eviction on capacity exceeded.'),
    ('M2.5.3', 'Implement cache-aware Pandora queries', 'Route queries through cache first. Implement cache miss → Neo4j fetch → cache insert.'),
    ('M2.5.4', 'Implement cache statistics & tuning', 'Add metrics (hit rate, eviction rate, capacity utilization). Implement dynamic capacity adjustment (if needed).'),
    
    # M2.6
    ('M2.6.1', 'Connect Pandora to Mimi core', 'Implement Mimi → Pandora message types. Add context retrieval calls from Mimi task execution.'),
    ('M2.6.2', 'Implement Pandora → Neo4j persistence', 'Add periodic flush of cached contexts to Neo4j. Implement transaction batching for efficiency.'),
    ('M2.6.3', 'Implement context-aware task execution', 'Modify Mimi executor to fetch context before task. Pass context to skills during execution.'),
    
    # M2.7
    ('M2.7.1', 'Write Pandora unit tests', 'Test temperature decay calculations. Test context CRUD operations.'),
    ('M2.7.2', 'Write Pandora integration tests', 'Test Pandora ↔ Neo4j persistence. Test cache coherency with Neo4j updates.'),
    ('M2.7.3', 'Write performance benchmarks', 'Benchmark context retrieval (cache hit, cache miss, Neo4j direct). Benchmark heatmap query performance.'),
    ('M2.7.4', 'Write Pandora documentation', 'Architecture diagrams (cache layers, Neo4j integration). API documentation (all public functions).'),
    
    # M3.1
    ('M3.1.1', 'Design container security architecture', 'Define resource limits (CPU: 50%, memory: 256MB per skill container). Design network isolation (none driver by default, explicit bridges for inter-skill communication).'),
    ('M3.1.2', 'Implement Skill Container Runtime', 'Create Dockerfile for skill execution container. Define container entry point and signal handling.'),
    ('M3.1.3', 'Implement Container Orchestration', 'Create container lifecycle manager (create, start, stop, remove). Implement container pooling for performance.'),
    ('M3.1.4', 'Implement Container Networking', 'Design network configuration for skill isolation. Implement inter-skill communication if needed (explicit bridges).'),
    ('M3.1.5', 'Implement Container Logging & Monitoring', 'Capture container stdout/stderr to centralized log. Add container resource usage metrics.'),
    
    # M3.2
    ('M3.2.1', 'Design Odlaguna architecture & state machine', 'Define states: IDLE, MONITORING, ALERTING, INTERVENTION, SHUTDOWN. Define monitoring triggers (timeout, resource exceeded, crash).'),
    ('M3.2.2', 'Implement Odlaguna core in Rust', 'Create state machine using enum-based pattern. Implement task monitoring loop (health checks, metrics collection).'),
    ('M3.2.3', 'Implement timeout enforcement', 'Create timeout tracking per running task. Implement timeout detection (elapsed >= configured timeout).'),
    ('M3.2.4', 'Implement resource limit enforcement', 'Monitor container CPU and memory usage (via cgroup). Detect resource exceeded conditions.'),
    ('M3.2.5', 'Implement health check & crash recovery', 'Define health check protocol (ping, heartbeat, status endpoint). Implement health check polling (interval, retries).'),
    ('M3.2.6', 'Implement audit logging', 'Log all Odlaguna interventions (timeout, resource, restart). Log all task lifecycle events (start, complete, error).'),
    
    # M3.3
    ('M3.3.1', 'Design circuit breaker state machine', 'Define states: CLOSED (normal), OPEN (failing), HALF_OPEN (testing). Define thresholds (consecutive failures = 3 → OPEN, successes = N → CLOSED).'),
    ('M3.3.2', 'Implement circuit breaker for Mimi → Pandora calls', 'Track consecutive failures (Neo4j calls, context retrieval). Implement state transitions (CLOSED → OPEN → HALF_OPEN).'),
    ('M3.3.3', 'Implement circuit breaker for Mimi → AI adapter calls', 'Track consecutive API failures (Gemini, Ollama). Implement state transitions with adaptive thresholds.'),
    ('M3.3.4', 'Implement circuit breaker metrics', 'Track state transitions and duration in each state. Track fallback invocations.'),
    
    # M3.4
    ('M3.4.1', 'Design WASM sandbox architecture', 'Plan WASM runtime selection (Wasmtime, Wasmer). Define resource limits (memory: 256MB, CPU: execution steps limit).'),
    ('M3.4.2', 'Implement WASM skill executor', 'Create WASM module loader and validator. Implement resource limit enforcement (memory, CPU).'),
    ('M3.4.3', 'Implement WASM ↔ Rust FFI boundary', 'Create type conversion layer (WASM types ↔ Rust types). Implement secure function calls with parameter validation.'),
    
    # M3.5
    ('M3.5.1', 'Implement error recovery strategies', 'Define error classification (recoverable vs fatal). Implement retry logic with exponential backoff.'),
    ('M3.5.2', 'Implement graceful degradation', 'Plan reduced functionality mode when components unavailable (no Pandora → minimal context). Implement feature detection (check component availability).'),
    ('M3.5.3', 'Implement security-focused error logging', 'Log security events (unauthorized access attempts, resource violations). Implement log encryption for sensitive data.'),
    
    # M3.6
    ('M3.6.1', 'Write security unit tests', 'Test resource limit enforcement. Test timeout enforcement.'),
    ('M3.6.2', 'Write security integration tests', 'Test container isolation (skill cannot access another skill\'s data). Test network isolation (skill cannot connect to external networks).'),
    ('M3.6.3', 'Write security benchmarks', 'Benchmark container startup time. Benchmark container resource overhead.'),
    ('M3.6.4', 'Write security documentation', 'Security model and threat model. Container hardening guide.'),
    
    # M4.1
    ('M4.1.1', 'Design pattern detection architecture', 'Define pattern types (sequential, conditional, repetitive, data-flow). Plan pattern matching algorithm (AST-based, regex-based, or hybrid).'),
    ('M4.1.2', 'Implement pattern matcher', 'Create pattern matching engine (AST parsing for code examples). Implement pattern scoring (confidence, specificity).'),
    ('M4.1.3', 'Implement pattern storage', 'Design Neo4j schema for pattern storage (PATTERN nodes, relationships). Implement pattern CRUD operations.'),
    ('M4.1.4', 'Implement pattern discovery', 'Create algorithm for finding common patterns in codebase. Implement pattern extraction from user instructions.'),
    ('M4.1.5', 'Implement pattern learning from execution', 'Track skill execution and outcomes. Detect patterns in successful executions.'),
    
    # M4.2
    ('M4.2.1', 'Design code generation architecture', 'Plan target languages (Rust initial, C++ future, Python future). Design AST-based code generation.'),
    ('M4.2.2', 'Implement code generator core', 'Create AST node types for target language. Implement AST-to-code conversion.'),
    ('M4.2.3', 'Implement template system', 'Create templates for common structures (main function, error handling, logging). Implement template variable substitution.'),
    ('M4.2.4', 'Implement code validation', 'Create syntax validator (parse generated code). Add type checker integration.'),
    ('M4.2.5', 'Implement code optimization', 'Add dead code elimination. Implement constant folding.'),
    
    # M4.3
    ('M4.3.1', 'Design Rhai integration architecture', 'Plan Rhai script lifecycle (compile, execute, cleanup). Design API for Rhai scripts (available functions, data structures).'),
    ('M4.3.2', 'Implement Rhai script executor', 'Create Rhai engine and script compiler. Implement resource limits (execution steps, memory).'),
    ('M4.3.3', 'Implement Rhai-Rust FFI', 'Create function registry for Rust callbacks. Implement type conversion (Rhai ↔ Rust).'),
    ('M4.3.4', 'Implement Rhai debugging', 'Add script tracing (function entry/exit). Implement breakpoint support (if needed).'),
    
    # M4.4
    ('M4.4.1', 'Design WASM generation architecture', 'Plan target WASM features (linear memory, function calls). Design code generation to WASM bytecode (manual or via IR).'),
    ('M4.4.2', 'Implement WASM code generator', 'Create IR-to-WASM compiler. Implement WASM module builder.'),
    ('M4.4.3', 'Implement WASM executor', 'Create WASM module loader. Implement function invocation.'),
    ('M4.4.4', 'Implement WASM-Rust FFI', 'Create import function registry. Implement parameter marshaling.'),
    
    # M4.5
    ('M4.5.1', 'Design 7-phase skill lifecycle', 'Phase 1: Detection (find new skill pattern in user instructions). Phase 2: Extraction (extract skill definition from pattern).'),
    ('M4.5.2', 'Implement skill state machine', 'Create state machine for 7-phase lifecycle. Implement state handlers and transitions.'),
    ('M4.5.3', 'Implement skill cache', 'Design skill cache (in-memory hash map by skill ID). Implement cache invalidation (on skill update).'),
    ('M4.5.4', 'Implement skill versioning', 'Design version schema (semantic versioning). Implement version history storage in Neo4j.'),
    ('M4.5.5', 'Implement skill dependency resolution', 'Detect skill dependencies (skills that call other skills). Implement topological sort for execution order.'),
    ('M4.5.6', 'Implement skill execution orchestration', 'Create executor for generated skills. Implement parameter passing and result collection.'),
    
    # M4.6
    ('M4.6.1', 'Connect Echidna to Mimi core', 'Implement skill generation on user instruction. Integrate generated skills into Mimi executor.'),
    ('M4.6.2', 'Connect Echidna to Pandora', 'Store generated skills in Neo4j. Track skill metadata (version, dependencies, creation date).'),
    ('M4.6.3', 'Connect Echidna to Odlaguna', 'Add Odlaguna monitoring for generated skills. Implement timeout enforcement for generated code.'),
    ('M4.6.4', 'Implement skill performance monitoring', 'Track execution time per skill. Track error rates and types.'),
    
    # M4.7
    ('M4.7.1', 'Write Echidna unit tests', 'Test pattern matching and detection. Test code generation correctness.'),
    ('M4.7.2', 'Write Echidna integration tests', 'Test skill generation pipeline (detection → deployment → execution). Test generated skills with various input types.'),
    ('M4.7.3', 'Write performance benchmarks', 'Benchmark pattern matching time. Benchmark code generation time (target: < 100ms).'),
    ('M4.7.4', 'Write Echidna documentation', 'Architecture diagrams (7-phase lifecycle). Pattern specification guide (how to define patterns).'),
    
    # Final
    ('Final.1', 'End-to-end system test (all M1-M4 components)', 'Full workflow: User instruction → CLI → Mimi → Beatrice → AI adapter → skill detection → skill generation → skill execution → context update → response. Test all component interactions (Message bus, state machine, Neo4j, watchdog, WASM).'),
    ('Final.2', 'Load & stress testing', 'Run 100+ concurrent tasks through Mimi. Monitor resource usage (CPU, memory, disk).'),
    ('Final.3', 'Security validation', 'Penetration testing (attempt to escape container, access unauthorized data). Audit trail verification (all security events logged).'),
    ('Final.4', 'Documentation completion', 'API reference for all public interfaces. Architecture decision record (ADR) for major decisions.'),
    ('Final.5', 'Release preparation', 'Version all components. Create release notes.'),
]

def get_milestone_number(task_id: str) -> str:
    """Get milestone name from task ID (e.g., M1.2.3 -> M1.2)"""
    parts = task_id.split('.')
    if task_id.startswith('Final'):
        return 'Final'
    return f"{parts[0]}.{parts[1]}"

def create_issue(task_id: str, title: str, description: str) -> bool:
    """Create a GitHub issue using gh CLI"""
    if task_id in EXISTING_ISSUES:
        print(f"⏭️  SKIP {task_id}: Already exists as issue #{EXISTING_ISSUES[task_id]}")
        return True
    
    phase = get_milestone_number(task_id)
    milestone = MILESTONE_MAP.get(phase, 'M1')
    
    body = f"**Task ID:** {task_id}\n\n{description}"
    
    cmd = [
        'gh', 'issue', 'create',
        '--repo', REPO,
        '--title', f"{task_id} {title}",
        '--body', body,
        '--label', 'planned',
        '--milestone', milestone,
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        if result.returncode == 0:
            issue_num = result.stdout.strip().split('/')[-1]
            print(f"✅ CREATE {task_id}: Issue #{issue_num}")
            return True
        else:
            print(f"❌ FAIL {task_id}: {result.stderr}")
            return False
    except Exception as e:
        print(f"❌ ERROR {task_id}: {str(e)}")
        return False

def main():
    if not GH_TOKEN:
        print("ERROR: GH_TOKEN environment variable not set")
        sys.exit(1)
    
    print(f"📋 Processing {len(ALL_TASKS)} tasks...")
    print(f"📌 Already created: {len(EXISTING_ISSUES)}")
    print(f"🆕 New to create: {len(ALL_TASKS) - len(EXISTING_ISSUES)}")
    print()
    
    created = 0
    skipped = 0
    failed = 0
    
    for task_id, title, description in ALL_TASKS:
        if create_issue(task_id, title, description):
            if task_id not in EXISTING_ISSUES:
                created += 1
            else:
                skipped += 1
        else:
            failed += 1
    
    print()
    print(f"✅ Created: {created}")
    print(f"⏭️  Skipped: {skipped}")
    print(f"❌ Failed: {failed}")
    print(f"📊 Total: {created + skipped + failed}/{len(ALL_TASKS)}")

if __name__ == '__main__':
    main()
