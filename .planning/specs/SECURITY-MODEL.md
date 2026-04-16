# Security Model & Sandboxing Strategy

> **Document:** `.planning/specs/SECURITY-MODEL.md`  
> **Status:** Pending peer review  
> **Scope:** Odlaguna validation, Docker isolation, WASM sandboxing, audit trail  

---

## Overview

MiMi's security model is **defense-in-depth** with multiple layers:

1. **Static Analysis** — Odlaguna validates code before execution (AST parsing, whitelist)
2. **Runtime Isolation** — Docker containers + WASM sandboxing (memory/capability isolation)
3. **Resource Limits** — CPU, RAM, network constraints enforced by Odlaguna
4. **Audit Trail** — All operations logged immutably in Neo4j
5. **Circuit Breaker** — Skills failing repeatedly are automatically disabled

---

## Capability Model

### Forbidden Operations (Hard Blocks)

Skills **cannot** do:

| Operation | Reason | Example |
|-----------|--------|---------|
| File I/O outside `/tmp` | Protect host filesystem | Reading `/etc/passwd` |
| Network access | Isolate execution environment | TCP connections |
| Process spawning | Prevent privilege escalation | `execve()` calls |
| Signals/signals | Prevent denial-of-service | Sending SIGKILL to Mimi |
| Memory-mapped files | Prevent side-channel attacks | Direct memory access |
| System calls | Direct access to kernel | `syscall()` |

### Whitelist of Allowed Operations

Skills **can** do:

| Operation | Scope | Example |
|-----------|-------|---------|
| Math operations | Arbitrary | `5 + 3 * 2` |
| String manipulation | Arbitrary | `"hello".upper()` |
| Data transformation | JSON, CSV, structured | Parse/transform objects |
| Temp file I/O | `/tmp` only | Write intermediate results |
| Memory allocation | < resource limit | Allocate arrays |
| Logging | Stderr/stdout | Print debug info |

---

## Validation Rules (Odlaguna)

### Rhai Code Validation

```rust
// Pattern matching for forbidden operations
let forbidden_patterns = [
    "fs::remove_dir_all",
    "std::process::Command",
    "/etc/",
    "/root/",
    "/sys/",
    "TcpStream::connect",
    "unsafe {",
    "libc::",
];

// Whitelist patterns (allowed)
let whitelist_patterns = [
    "math::",
    "string::",
    "array::",
    "json::",
];
```

### WASM Import Validation

WASM modules can only import from:
- `env` — Whitelist of safe host functions
- No imports from `libc`, `std::net`, `std::fs` (outside /tmp)

### Size Limits

| Artifact | Max Size | Justification |
|----------|----------|---------------|
| Rhai script | 100 KB | Prevent memory bombs |
| WASM binary | 10 MB | Prevent bloat |
| Input data | 100 MB | Prevent DoS |
| Output data | 100 MB | Prevent flooding |

---

## Docker Isolation (Ryzu)

### Container Security Configuration

```yaml
# docker-compose.yml for worker
services:
  mimi-worker:
    image: mimi-worker:latest
    cap_drop:
      - ALL                    # Drop all capabilities
    cap_add:
      - NET_BIND_SERVICE       # Only bind service (for IPC)
    read_only: true            # Read-only filesystem
    security_opt:
      - no-new-privileges:true # Prevent privilege escalation
    user: "1000:1000"          # Non-root user
    environment:
      - MAX_MEMORY=256M
      - MAX_CPU=1
    networks:
      - isolated               # Custom network (no access to host)
    tmpfs:
      - /tmp:size=100M,noexec  # Temporary filesystem (no-exec)
    volumes: []                # No volume mounts allowed
```

### Network Isolation

- Container `network_mode: none` by default
- No outbound connections to host or internet
- IPC via Unix socket to Mimi only

### Filesystem Isolation

- Root filesystem read-only
- `/tmp` writable but no-exec (cannot run binaries)
- No access to `/etc`, `/proc`, `/sys`

### Process Isolation

- Container PID namespace (isolated PID 1)
- Cannot see host processes
- Cannot send signals outside container

---

## WASM Sandboxing (Wasmtime)

### Memory Isolation

WASM module gets:
- **Linear memory:** isolated heap (default 1 MB, configurable)
- **Stack:** separate from host
- **No access** to host memory

### Instruction Counting (Fueling)

```rust
// Limit CPU cycles per skill
let fuel_per_type = {
    "simple": 100_000,      // ~10ms at 10MHz
    "medium": 1_000_000,    // ~100ms
    "complex": 10_000_000,  // ~1 second
};

// Execution aborts if fuel runs out
if store.fuel_consumed() > limit {
    return Err(FuelError::Exhausted);
}
```

### Host Function Whitelist

WASM can call:

```rust
pub fn add_safe_host_functions(linker: &mut Linker) {
    // Safe operations only
    linker.func_wrap("env", "log", |val: i32| {
        println!("WASM: {}", val);
        Ok(())
    })?;
    
    linker.func_wrap("env", "sqrt", |x: f64| -> f64 {
        x.sqrt()
    })?;
    
    // NO network, file I/O, or process calls
}
```

---

## Audit Trail (Neo4j)

Every operation logged immutably:

```cypher
CREATE (a:AuditLog {
  id: STRING (UUID),
  timestamp: DATETIME,
  actor: STRING ("mimi", "echidna", "odlaguna"),
  action: STRING ("execute", "validate", "reject", "timeout"),
  target_id: STRING (skill_id),
  target_type: STRING ("skill", "task"),
  status: STRING ("success", "failure", "timeout"),
  details: STRING (JSON),
  result_hash: STRING (SHA256)
})
```

### Query Audit Trail

```cypher
// Find all failed skill executions in last 24h
MATCH (a:AuditLog)
WHERE 
  a.action = "execute" AND
  a.status = "failure" AND
  a.timestamp > datetime() - duration({days: 1})
RETURN a ORDER BY a.timestamp DESC
```

### Compliance Requirements

- **Immutable:** Once written, cannot be deleted or modified
- **Ordered:** Transactions ensure FIFO ordering
- **Complete:** Every action must be logged
- **Auditable:** Query any aspect (who, what, when, result)

---

## Error Scenarios & Recovery

| Scenario | Detection | Response |
|----------|-----------|----------|
| **Skill timeout** | Odlaguna timer expires | SIGKILL container, log failure |
| **Skill crash** | Non-zero exit code | Circuit breaker increment, retry policy |
| **Forbidden operation** | AST validation | Reject before execution, log attempt |
| **Resource exhaustion** | Memory/CPU limit hit | Kill container, alert operator |
| **Network access attempt** | Docker network isolation | Fail silently (EPERM) |
| **Privilege escalation** | `no-new-privileges` flag | Prevent kernel execution |

---

## Circuit Breaker Strategy

```
State Machine:

CLOSED (normal)
  ↓ (3 failures)
OPEN (skill blocked)
  ↓ (5 minutes elapsed)
HALF_OPEN (trial execution)
  ↓ (success)
CLOSED (back to normal)
  ↓ (failure)
OPEN (reset timer)
```

---

## Testing Security

### Unit Tests

- Whitelist pattern matching (positive/negative cases)
- Fuel consumption calculation
- Circuit breaker state transitions

### Integration Tests

- Forbidden operations are caught by validator
- WASM cannot access host memory
- Docker container enforces resource limits
- Audit trail is complete and immutable

### Security Audit Tests

```bash
# Attempt to escape container
docker exec mimi-worker cat /etc/passwd  # Should fail

# Attempt to create network connection
docker exec mimi-worker curl http://example.com  # Should fail

# Attempt to fork process
docker exec mimi-worker bash  # Should fail (no shell)

# Check audit trail
curl http://localhost:3000/audit?action=execute&status=failure
```

---

## Future Enhancements (M3+)

- **gVisor/Kata Containers:** Stronger isolation than Docker
- **SELinux/AppArmor:** Mandatory access control
- **Rate limiting per skill:** Prevent resource exhaustion
- **Reputation system:** Skills with high success rate get higher fuel limits
- **Behavioral anomaly detection:** ML-based on audit trail

---

## Compliance & Standards

- **OWASP Top 10:** Addressed via sandboxing, validation, audit trail
- **CWE-94 (Code Injection):** Mitigated by AST validation + sandboxing
- **CWE-120 (Buffer Overflow):** Mitigated by Rust memory safety + WASM linear memory
- **CWE-243 (No Check for Zero):** Checked in validation rules

---

## References

- Volta a: [`REQUIREMENTS.md#RNF-2`](../REQUIREMENTS.md#rnf-2-segurança)
- Volta a: [`milestones/M3-SECURITY.md`](../milestones/M3-SECURITY.md)
- Volta a: [`modules/ODLAGUNA.md`](../modules/ODLAGUNA.md)
- Volta a: [`modules/RYZU.md`](../modules/RYZU.md)
