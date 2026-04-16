# RYZU — Isolated Worker Execution Module

> **Primary Language:** C++/Rust Hybrid  
> **Role:** Docker-based worker orchestration for skill execution with strong resource isolation  
> **Status:** 🟡 Planned (Milestone M3)  
> **Owner:** Security & Execution Layer  

---

## 1. Module Overview

**Ryzu** (Nameless Processors) is MiMi's execution sandbox layer. It orchestrates Docker containers that execute user-generated skills (Rhai scripts, WASM binaries, or native executables) in complete isolation from the host system.

**Core Responsibilities:**
- **Container Lifecycle Management:** Spawn, monitor, and teardown Docker containers per skill execution
- **Resource Isolation:** Enforce CPU quotas, memory limits, and network isolation
- **Process Monitoring:** Capture stdout/stderr, track execution time, detect zombie processes
- **Output Streaming:** Real-time log capture and forwarding to Odlaguna/Mimi
- **Cleanup Automation:** Guarantee container removal within 100ms post-execution

**Key Constraints:**
- **Zero Persistence:** Containers are ephemeral; no writable volumes mounted
- **Zero Root Access:** All processes run as non-privileged user (UID 1000)
- **Network Isolation:** Default network mode is `none` (no external connectivity)
- **Timeout Enforcement:** Delegated to Odlaguna; Ryzu only provides process handle

**Security Guarantees:**
- Skills cannot access host filesystem beyond read-only mounted code
- Skills cannot spawn persistent daemons or background processes
- Skills cannot escalate privileges or access kernel interfaces
- Skills cannot consume unbounded resources (enforced via cgroups)

---

## 2. Architecture

### 2.1 Internal Components

```
┌─────────────────────────────────────────────────────────────┐
│                      Ryzu Runtime                           │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────────┐       ┌──────────────────┐           │
│  │ Docker Manager   │◄──────┤ Message Bus      │           │
│  │ (Rust)           │       │ Subscriber       │           │
│  │                  │       │ (task/execute)   │           │
│  │ - spawn()        │       └──────────────────┘           │
│  │ - kill()         │                                       │
│  │ - cleanup()      │       ┌──────────────────┐           │
│  └────────┬─────────┘       │ Worker Pool      │           │
│           │                 │ (C++)            │           │
│           │                 │                  │           │
│           │                 │ - WorkerHandle[] │           │
│           │                 │ - LRU eviction   │           │
│           └────────────────►│ - Health checks  │           │
│                             └──────────────────┘           │
│                                                             │
│  ┌──────────────────┐       ┌──────────────────┐           │
│  │ Output Capture   │       │ Resource Monitor │           │
│  │ (Rust)           │       │ (C++)            │           │
│  │                  │       │                  │           │
│  │ - stdout/stderr  │       │ - CPU usage      │           │
│  │ - streaming      │       │ - Memory RSS     │           │
│  │ - log rotation   │       │ - cgroups stats  │           │
│  └──────────────────┘       └──────────────────┘           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                             │
                             │ Docker API (Unix socket)
                             ▼
                   ┌──────────────────────┐
                   │ Docker Engine        │
                   │                      │
                   │ [worker-container-1] │
                   │ [worker-container-2] │
                   │ [worker-container-N] │
                   └──────────────────────┘
```

### 2.2 Data Flow

1. **Execution Request:** Odlaguna/Mimi publishes to `task/execute` topic
2. **Container Spawn:** Docker Manager creates container with config:
   - Base image: `mimi-worker:latest`
   - CPU quota: 50% (50000 µs / 100000 µs period)
   - Memory limit: 256MB (configurable)
   - Network: `none`
   - User: `worker:worker` (UID:GID 1000:1000)
3. **Code Injection:** Skill code mounted as read-only volume at `/app/skill`
4. **Process Start:** Container entrypoint executes skill runtime (Rhai engine, WASM runtime, or bash)
5. **Output Capture:** Ryzu attaches to stdout/stderr streams, forwards to Message Bus (`task/logs`)
6. **Completion:** Process exits → Ryzu captures exit code → publishes result to `task/result`
7. **Cleanup:** Container removed (forced if necessary), resources released

### 2.3 Container Pooling Strategy

**Cold Start (M3):**
- Each execution spawns fresh container
- Spawn time: ~100ms (acceptable for M3 security focus)

**Warm Pool (M4+):**
- Maintain 3-5 pre-warmed containers in idle state
- Reuse containers for sequential skill executions
- Eviction policy: LRU when pool exceeds limit
- Reset between executions: clear `/tmp`, reset environment variables

---

## 3. API/Interfaces

### 3.1 Message Bus Topics (Subscriber)

#### `task/execute` (Request)
**Schema (FlatBuffers):**
```flatbuffers
table ExecuteTaskRequest {
  task_id: string;
  skill_id: string;
  skill_code: string;          // Code to execute (Rhai/WASM/Bash)
  skill_type: SkillType;        // enum: Rhai, WASM, Bash
  timeout_ms: uint64 = 5000;    // Timeout (enforced by Odlaguna)
  memory_limit_mb: uint32 = 256;
  cpu_quota_percent: uint8 = 50;
  network_enabled: bool = false;
  environment: [KeyValue];      // Environment variables
}
```

**Behavior:**
- Ryzu validates resource limits (reject if > 512MB or > 100% CPU)
- Spawns container with specified constraints
- Returns `WorkerHandle` to caller (internal state)

---

#### `task/kill` (Request)
**Schema:**
```flatbuffers
table KillTaskRequest {
  task_id: string;
  container_id: string;
  force: bool = true;           // true = SIGKILL, false = SIGTERM
}
```

**Behavior:**
- Sends SIGKILL/SIGTERM to container
- Waits up to 5s for graceful shutdown
- Force-removes container if still alive

---

### 3.2 Message Bus Topics (Publisher)

#### `task/result` (Response)
**Schema:**
```flatbuffers
table TaskResult {
  task_id: string;
  container_id: string;
  exit_code: int32;
  stdout: string;               // Truncated if > 10KB
  stderr: string;
  execution_time_ms: uint64;
  peak_memory_mb: uint32;
  cpu_time_ms: uint64;
  status: ExecutionStatus;      // enum: Success, Timeout, OOM, Error
}

enum ExecutionStatus: byte {
  Success = 0,
  Timeout = 1,
  OutOfMemory = 2,
  Crashed = 3,
  Killed = 4,
  ValidationFailed = 5
}
```

---

#### `task/logs` (Streaming)
**Schema:**
```flatbuffers
table TaskLogChunk {
  task_id: string;
  timestamp: uint64;            // Unix timestamp (ms)
  stream: LogStream;            // enum: Stdout, Stderr
  chunk: string;                // Up to 4KB per chunk
}

enum LogStream: byte {
  Stdout = 0,
  Stderr = 1
}
```

---

### 3.3 Docker API Integration

**Crate Used:** `bollard` (official Docker Rust client)

**Key Operations:**
```rust
// Container creation
docker.create_container(
  Some(CreateContainerOptions { name: "mimi-worker-uuid", .. }),
  Config {
    image: Some("mimi-worker:latest"),
    host_config: Some(HostConfig {
      memory: 268_435_456,        // 256MB
      cpu_quota: 50_000,          // 50% CPU
      network_mode: Some("none"),
      security_opt: vec!["no-new-privileges".to_string()],
      ..Default::default()
    }),
    ..Default::default()
  }
).await?;

// Container start
docker.start_container(&container_id, None).await?;

// Log streaming
let stream = docker.logs(&container_id, Some(LogsOptions {
  follow: true,
  stdout: true,
  stderr: true,
  ..Default::default()
}));

// Container removal
docker.remove_container(&container_id, Some(RemoveContainerOptions {
  force: true,
  ..Default::default()
})).await?;
```

---

## 4. Key Algorithms

### 4.1 Container Spawning Algorithm

```rust
async fn spawn_worker(config: ExecuteTaskRequest) -> Result<WorkerHandle, RyzuError> {
  // 1. Validate resource limits
  if config.memory_limit_mb > 512 {
    return Err(RyzuError::InvalidConfig("Memory limit exceeds 512MB"));
  }

  // 2. Create container with security config
  let container_id = create_container(&config).await?;

  // 3. Inject skill code as read-only volume
  copy_code_to_volume(&container_id, &config.skill_code).await?;

  // 4. Start container
  docker.start_container(&container_id, None).await?;

  // 5. Attach to stdout/stderr streams
  let log_stream = docker.logs(&container_id, log_options()).await?;

  // 6. Return handle
  Ok(WorkerHandle {
    task_id: config.task_id,
    container_id,
    started_at: SystemTime::now(),
    log_stream,
  })
}
```

**Performance Target:** Spawn + start < 100ms (cold start)

---

### 4.2 Resource Limiting

**CPU Quota (cgroups v2):**
```dockerfile
# In container creation
cpu_quota: 50_000,               // 50ms per 100ms period
cpu_period: 100_000,             // 100ms period
```

**Memory Limit:**
```dockerfile
memory: 268_435_456,             // 256MB hard limit
memory_swap: 268_435_456,        // No swap
oom_kill_disable: false,         // Allow OOM killer
```

**Network Isolation:**
```dockerfile
network_mode: "none"             // No network interfaces except loopback
```

**Validation Logic:**
```rust
fn validate_limits(config: &ExecuteTaskRequest) -> Result<(), ValidationError> {
  // Max memory: 512MB
  if config.memory_limit_mb > 512 {
    return Err(ValidationError::MemoryExceeded);
  }

  // Max CPU: 100% of 1 core
  if config.cpu_quota_percent > 100 {
    return Err(ValidationError::CpuExceeded);
  }

  // Network disabled by default
  if config.network_enabled && !is_authorized_for_network(&config.skill_id) {
    return Err(ValidationError::NetworkForbidden);
  }

  Ok(())
}
```

---

### 4.3 Process Monitoring

**Zombie Process Detection:**
```rust
async fn monitor_worker(handle: &WorkerHandle) -> Result<(), RyzuError> {
  loop {
    let stats = docker.stats(&handle.container_id, Some(StatsOptions {
      stream: false,
      ..Default::default()
    })).await?;

    // Check if container is still alive
    let inspect = docker.inspect_container(&handle.container_id, None).await?;
    
    if !inspect.state.running.unwrap_or(false) {
      // Process exited, cleanup
      return Ok(());
    }

    // Check for zombie (CPU time not increasing)
    if is_zombie(&stats) {
      warn!("Zombie process detected: {}", handle.container_id);
      kill_container(&handle.container_id, true).await?;
      return Err(RyzuError::ZombieProcess);
    }

    tokio::time::sleep(Duration::from_millis(500)).await;
  }
}

fn is_zombie(stats: &Stats) -> bool {
  // Heuristic: CPU usage < 0.01% for > 5s
  stats.cpu_stats.cpu_usage.total_usage < 1_000_000 // < 1ms
}
```

---

### 4.4 Output Streaming

**Real-time Log Forwarding:**
```rust
async fn stream_logs(handle: WorkerHandle, bus: BusClient) -> Result<(), RyzuError> {
  let mut stream = handle.log_stream;

  while let Some(chunk) = stream.next().await {
    match chunk {
      Ok(LogOutput::StdOut { message }) => {
        bus.publish("task/logs", &TaskLogChunk {
          task_id: handle.task_id.clone(),
          timestamp: now_ms(),
          stream: LogStream::Stdout,
          chunk: String::from_utf8_lossy(&message).to_string(),
        }).await?;
      },
      Ok(LogOutput::StdErr { message }) => {
        bus.publish("task/logs", &TaskLogChunk {
          task_id: handle.task_id.clone(),
          timestamp: now_ms(),
          stream: LogStream::Stderr,
          chunk: String::from_utf8_lossy(&message).to_string(),
        }).await?;
      },
      Err(e) => {
        error!("Log stream error: {}", e);
        break;
      }
    }
  }

  Ok(())
}
```

**Buffer Management:**
- Chunk size: 4KB (balance latency vs overhead)
- Max total stdout/stderr: 10MB (truncate older logs if exceeded)
- Log rotation: per-container log file with max 3 rotations

---

## 5. Dependencies

### 5.1 Runtime Dependencies

| Component | Version | Purpose |
|-----------|---------|---------|
| **Docker Engine** | ≥ 20.10 | Container runtime |
| **bollard** | ≥ 0.14 | Docker Rust client |
| **tokio** | ≥ 1.28 | Async runtime |
| **serde** + **serde_json** | ≥ 1.0 | JSON serialization |
| **flatbuffers** | ≥ 23.5 | Message Bus serialization |

### 5.2 Module Dependencies

| Module | Interaction | Data Flow |
|--------|-------------|-----------|
| **Odlaguna** | Authorization + Timeouts | `task/execute` → Ryzu, Ryzu → `task/result` |
| **Mimi** | Task Orchestration | Mimi → `task/execute`, Ryzu → `task/result` |
| **Message Bus** | Communication Layer | Pub/Sub for all topics |
| **Pandora** | Audit Logging | Ryzu → `audit/execution_log` |

### 5.3 External Dependencies

- **cgroups v2:** Required for CPU/memory limiting (Linux kernel ≥ 4.5)
- **AppArmor/SELinux:** Optional enhanced security profiles
- **Unix sockets:** Docker daemon communication (`/var/run/docker.sock`)

---

## 6. Data Structures

### 6.1 WorkerHandle

```rust
pub struct WorkerHandle {
  pub task_id: String,
  pub container_id: String,
  pub started_at: SystemTime,
  pub log_stream: Option<LogStream>,
  pub resource_monitor: ResourceMonitor,
}

impl WorkerHandle {
  pub async fn wait_for_exit(&self) -> Result<ExitCode, RyzuError> {
    let result = docker.wait_container(&self.container_id, None).await?;
    Ok(ExitCode(result.status_code))
  }

  pub async fn get_stats(&self) -> Result<ContainerStats, RyzuError> {
    docker.stats(&self.container_id, None).await
  }
}
```

---

### 6.2 ExitCode

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitCode(pub i32);

impl ExitCode {
  pub fn is_success(&self) -> bool {
    self.0 == 0
  }

  pub fn is_timeout(&self) -> bool {
    self.0 == 124 // Standard timeout exit code
  }

  pub fn is_signal(&self) -> bool {
    self.0 > 128 // Killed by signal (e.g., SIGKILL = 137)
  }
}
```

---

### 6.3 ProcessOutput

```rust
pub struct ProcessOutput {
  pub stdout: Vec<u8>,
  pub stderr: Vec<u8>,
  pub exit_code: ExitCode,
  pub execution_time: Duration,
  pub peak_memory_bytes: u64,
  pub cpu_time: Duration,
}

impl ProcessOutput {
  pub fn truncate_logs(&mut self, max_bytes: usize) {
    if self.stdout.len() > max_bytes {
      self.stdout.truncate(max_bytes);
      self.stdout.extend_from_slice(b"\n[TRUNCATED]");
    }
    if self.stderr.len() > max_bytes {
      self.stderr.truncate(max_bytes);
      self.stderr.extend_from_slice(b"\n[TRUNCATED]");
    }
  }
}
```

---

### 6.4 ResourceLimits

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
  pub memory_mb: u32,           // Default: 256MB
  pub cpu_quota_percent: u8,    // Default: 50%
  pub disk_write_mb: u32,       // Default: 0 (read-only)
  pub network_enabled: bool,    // Default: false
  pub max_pids: u32,            // Default: 64
}

impl Default for ResourceLimits {
  fn default() -> Self {
    Self {
      memory_mb: 256,
      cpu_quota_percent: 50,
      disk_write_mb: 0,
      network_enabled: false,
      max_pids: 64,
    }
  }
}
```

---

### 6.5 DockerConfig

```rust
pub struct DockerConfig {
  pub base_image: String,       // Default: "mimi-worker:latest"
  pub socket_path: PathBuf,     // Default: "/var/run/docker.sock"
  pub container_prefix: String, // Default: "mimi-worker-"
  pub cleanup_timeout_ms: u64,  // Default: 100ms
  pub max_concurrent_workers: usize, // Default: 10
}
```

---

## 7. Integration Points

### 7.1 Receives Execute Commands

**From:** Mimi (task orchestrator) or Odlaguna (direct authorization)

**Topic:** `task/execute`

**Trigger:** User skill execution request after Odlaguna validation

**Contract:**
- Ryzu MUST validate resource limits before spawning container
- Ryzu MUST reject requests with invalid `skill_type` or missing `skill_code`
- Ryzu MUST publish `task/result` on completion or failure

---

### 7.2 Sends Results Back

**To:** Mimi (for aggregation) and Pandora (for audit trail)

**Topics:**
- `task/result` → Final execution result
- `task/logs` → Real-time log streaming

**Contract:**
- Results MUST include exit code, stdout/stderr (truncated if necessary)
- Results MUST include resource usage (peak memory, CPU time)
- Logs MUST be timestamped with millisecond precision

---

### 7.3 Respects Fueling Limits

**Enforced by:** Odlaguna (timeout mechanism)

**Ryzu Responsibility:**
- Provide `WorkerHandle` to Odlaguna for timeout tracking
- Respond to `task/kill` requests within 100ms
- Guarantee container cleanup even if kill fails (force removal)

**Interaction Flow:**
```
1. Ryzu spawns container → returns WorkerHandle
2. Odlaguna starts timeout timer (e.g., 5s)
3. If timeout expires → Odlaguna publishes task/kill
4. Ryzu receives kill request → SIGKILL container → cleanup
5. Ryzu publishes task/result with status=Timeout
```

---

## 8. Error Handling

### 8.1 Container Startup Failures

**Scenario:** Docker daemon unreachable, image not found, invalid config

**Handling:**
```rust
async fn spawn_worker(config: ExecuteTaskRequest) -> Result<WorkerHandle, RyzuError> {
  match docker.create_container(...).await {
    Err(bollard::errors::Error::DockerResponseServerError { status_code, message })
      if status_code == 404 => {
        // Image not found → Pull image and retry
        docker.pull_image("mimi-worker:latest", None).await?;
        return spawn_worker(config).await; // Retry once
      },
    Err(e) => {
      error!("Container creation failed: {}", e);
      publish_error_result(&config.task_id, ExecutionStatus::ValidationFailed).await;
      return Err(RyzuError::SpawnFailed(e.to_string()));
    },
    Ok(container) => { /* Continue */ }
  }
}
```

**Mitigation:**
- Pre-warm: Pull `mimi-worker:latest` on Ryzu startup
- Validate Docker daemon health before accepting tasks (heartbeat check)

---

### 8.2 Timeout/SIGKILL Handling

**Scenario:** Process exceeds Odlaguna deadline

**Handling:**
```rust
async fn kill_worker(container_id: &str, force: bool) -> Result<(), RyzuError> {
  if !force {
    // Try graceful shutdown first (SIGTERM)
    docker.kill_container(container_id, Some(KillContainerOptions {
      signal: "SIGTERM",
    })).await?;

    // Wait up to 5s for exit
    match tokio::time::timeout(
      Duration::from_secs(5),
      docker.wait_container(container_id, None)
    ).await {
      Ok(_) => return Ok(()), // Graceful exit
      Err(_) => {
        warn!("Container {} did not exit gracefully, forcing...", container_id);
      }
    }
  }

  // Force kill (SIGKILL)
  docker.kill_container(container_id, Some(KillContainerOptions {
    signal: "SIGKILL",
  })).await?;

  // Force remove even if kill fails
  docker.remove_container(container_id, Some(RemoveContainerOptions {
    force: true,
    ..Default::default()
  })).await?;

  Ok(())
}
```

**Guarantee:** Container removed within 100ms of SIGKILL

---

### 8.3 Zombie Process Cleanup

**Scenario:** Container enters zombie state (process stuck in `D` state, unkillable)

**Detection:**
```rust
fn is_zombie(stats: &ContainerStats) -> bool {
  // No CPU time increase in last 5 seconds
  stats.cpu_stats.cpu_usage.total_usage < 1_000_000
}
```

**Handling:**
1. Log zombie detection to Pandora audit trail
2. Force-remove container (`force: true`)
3. If removal fails → escalate to Docker daemon restart (last resort)
4. Publish `task/result` with `status=Crashed`

**Prevention:**
- Set `init: true` in container config (PID 1 reaping orphaned processes)
- Use `stop_timeout: 5` to guarantee cleanup

---

### 8.4 Output Capture Errors

**Scenario:** Log stream disconnected, buffer overflow

**Handling:**
```rust
async fn stream_logs(handle: WorkerHandle, bus: BusClient) -> Result<(), RyzuError> {
  let mut stream = handle.log_stream;
  let mut buffer = Vec::new();

  while let Some(chunk) = stream.next().await {
    match chunk {
      Ok(log) => {
        buffer.extend_from_slice(&log);

        // Prevent buffer overflow (max 10MB)
        if buffer.len() > 10_485_760 {
          warn!("Log buffer exceeded 10MB, truncating...");
          buffer.truncate(10_485_760);
          buffer.extend_from_slice(b"\n[LOGS TRUNCATED - LIMIT EXCEEDED]");
          break;
        }

        // Forward to Bus
        bus.publish("task/logs", &log).await?;
      },
      Err(e) => {
        error!("Log stream error: {}", e);
        // Attempt reconnection once
        if let Ok(new_stream) = docker.logs(&handle.container_id, log_options()).await {
          stream = new_stream;
          continue;
        }
        break;
      }
    }
  }

  Ok(())
}
```

**Fallback:** If streaming fails, capture logs via `docker logs` API after process exits

---

## 9. Performance Characteristics

### 9.1 Latency Targets

| Operation | Target | Rationale |
|-----------|--------|-----------|
| **Container Spawn** | < 100ms | Cold start acceptable for M3 security focus |
| **Container Kill** | < 50ms | SIGKILL is instant, removal overhead minimal |
| **Container Cleanup** | < 100ms | Guarantee no resource leaks |
| **Log Streaming** | < 10ms | Real-time feedback to user |
| **Stats Collection** | < 5ms | Non-blocking monitoring |

### 9.2 Throughput

| Metric | Target | Notes |
|--------|--------|-------|
| **Concurrent Workers** | 10-20 | Limited by Docker daemon and host resources |
| **Executions/second** | 5-10 | Bottleneck: container spawn time |
| **Max Container Lifetime** | 60s | Odlaguna timeout enforcement |

### 9.3 Resource Usage (Ryzu Itself)

| Resource | Baseline | Peak |
|----------|----------|------|
| **CPU** | 2-5% | 20% (during concurrent spawns) |
| **Memory** | 50MB | 200MB (with 10 active workers) |
| **Disk I/O** | Minimal | Spike during image pulls |

### 9.4 Network Isolation

**Default:** Network mode = `none`

**Performance Impact:** None (no network stack initialization)

**When Enabled (future):**
- Bridge network with outbound-only firewall rules
- Latency: +5ms (iptables overhead)

---

## 10. Testing Strategy

### 10.1 Container Lifecycle Tests

**Unit Tests (Rust):**
```rust
#[tokio::test]
async fn test_container_spawn_success() {
  let config = ExecuteTaskRequest {
    task_id: "test-task".to_string(),
    skill_code: "echo 'Hello, Ryzu'".to_string(),
    skill_type: SkillType::Bash,
    ..Default::default()
  };

  let handle = spawn_worker(config).await.unwrap();
  assert!(!handle.container_id.is_empty());
}

#[tokio::test]
async fn test_container_cleanup_after_exit() {
  let handle = spawn_test_worker().await;
  wait_for_exit(&handle).await.unwrap();
  
  // Container should be auto-removed
  assert!(docker.inspect_container(&handle.container_id, None).await.is_err());
}
```

**Integration Tests (Docker):**
```bash
# Test script: tests/integration/test_worker_isolation.sh
#!/bin/bash

# 1. Spawn container
docker run -d --name test-worker \
  --memory 256m \
  --cpus 0.5 \
  --network none \
  mimi-worker:latest

# 2. Verify resource limits
MEMORY=$(docker stats --no-stream --format "{{.MemLimit}}" test-worker)
test "$MEMORY" = "256MiB" || exit 1

# 3. Verify network isolation
docker exec test-worker ping -c 1 8.8.8.8 2>&1 | grep "Network is unreachable" || exit 1

# 4. Cleanup
docker rm -f test-worker
```

---

### 10.2 Resource Limit Enforcement Tests

**CPU Quota Test:**
```rust
#[tokio::test]
async fn test_cpu_quota_enforcement() {
  let config = ExecuteTaskRequest {
    skill_code: r#"
      // Infinite loop to saturate CPU
      while true; do :; done
    "#.to_string(),
    cpu_quota_percent: 50,
    ..Default::default()
  };

  let handle = spawn_worker(config).await.unwrap();
  tokio::time::sleep(Duration::from_secs(5)).await;

  let stats = docker.stats(&handle.container_id, None).await.unwrap();
  let cpu_usage = calculate_cpu_percent(&stats);

  // Should be ~50% ± 5%
  assert!(cpu_usage > 45.0 && cpu_usage < 55.0);
}
```

**Memory Limit Test:**
```rust
#[tokio::test]
async fn test_memory_oom_kill() {
  let config = ExecuteTaskRequest {
    skill_code: r#"
      // Allocate 512MB (exceeds 256MB limit)
      my_vec = vec![0u8; 536_870_912];
    "#.to_string(),
    memory_limit_mb: 256,
    ..Default::default()
  };

  let handle = spawn_worker(config).await.unwrap();
  let result = wait_for_exit(&handle).await.unwrap();

  // Should be killed by OOM killer (exit code 137)
  assert_eq!(result.0, 137);
}
```

---

### 10.3 Stdout/Stderr Capture Tests

**Output Correctness Test:**
```rust
#[tokio::test]
async fn test_stdout_stderr_capture() {
  let config = ExecuteTaskRequest {
    skill_code: r#"
      echo "STDOUT message"
      >&2 echo "STDERR message"
    "#.to_string(),
    skill_type: SkillType::Bash,
    ..Default::default()
  };

  let handle = spawn_worker(config).await.unwrap();
  let output = collect_output(&handle).await.unwrap();

  assert_eq!(output.stdout.trim(), "STDOUT message");
  assert_eq!(output.stderr.trim(), "STDERR message");
}
```

**Log Truncation Test:**
```rust
#[tokio::test]
async fn test_log_truncation() {
  let config = ExecuteTaskRequest {
    skill_code: r#"
      // Generate 20MB of output
      for i in 1..20000000; do echo "Line $i"; done
    "#.to_string(),
    ..Default::default()
  };

  let handle = spawn_worker(config).await.unwrap();
  let output = collect_output(&handle).await.unwrap();

  // Should be truncated to 10MB
  assert!(output.stdout.len() <= 10_485_760);
  assert!(output.stdout.ends_with(b"[TRUNCATED]"));
}
```

---

### 10.4 Benchmark Suite

**Spawn Latency Benchmark:**
```rust
#[bench]
fn bench_container_spawn(b: &mut Bencher) {
  let rt = Runtime::new().unwrap();
  b.iter(|| {
    rt.block_on(async {
      let handle = spawn_worker(default_config()).await.unwrap();
      cleanup_worker(&handle).await.unwrap();
    });
  });
}
// Target: < 100ms per iteration
```

**Concurrent Execution Benchmark:**
```rust
#[bench]
fn bench_concurrent_workers(b: &mut Bencher) {
  let rt = Runtime::new().unwrap();
  b.iter(|| {
    rt.block_on(async {
      let handles = futures::future::join_all(
        (0..10).map(|_| spawn_worker(default_config()))
      ).await;
      
      for handle in handles {
        cleanup_worker(&handle.unwrap()).await.unwrap();
      }
    });
  });
}
// Target: 10 workers spawned in < 500ms
```

---

## 11. Future Extensions (M3+ Notes)

### 11.1 Kata Containers for Stronger Isolation (M4+)

**Motivation:** Docker containers share the host kernel; Kata provides VM-level isolation

**Implementation:**
- Replace `mimi-worker:latest` with Kata runtime
- Configure Docker to use `io.containerd.kata.v2` runtime
- Trade-off: Spawn latency increases to ~500ms (acceptable for high-security skills)

**Config:**
```json
{
  "runtimes": {
    "kata": {
      "path": "/usr/bin/kata-runtime"
    }
  }
}
```

---

### 11.2 GPU Support (M5+)

**Use Case:** Skills requiring ML inference (WASM with ONNX runtime)

**Implementation:**
- Add `--gpus all` to Docker container config
- Limit GPU memory via `device_requests`
- Require NVIDIA Container Toolkit

**Config:**
```rust
host_config: Some(HostConfig {
  device_requests: Some(vec![DeviceRequest {
    driver: Some("nvidia".to_string()),
    count: Some(1),
    capabilities: Some(vec![vec!["gpu".to_string()]]),
    ..Default::default()
  }]),
  ..Default::default()
}),
```

---

### 11.3 Warm Container Pool (M4+)

**Optimization:** Pre-spawn 3-5 idle containers to eliminate cold start latency

**Implementation:**
```rust
struct WarmPool {
  idle_containers: VecDeque<String>,
  max_size: usize,
}

impl WarmPool {
  async fn get_or_spawn(&mut self) -> Result<String, RyzuError> {
    if let Some(container_id) = self.idle_containers.pop_front() {
      Ok(container_id)
    } else {
      spawn_idle_container().await
    }
  }

  async fn return_container(&mut self, container_id: String) {
    if self.idle_containers.len() < self.max_size {
      reset_container(&container_id).await;
      self.idle_containers.push_back(container_id);
    } else {
      cleanup_container(&container_id).await;
    }
  }
}
```

---

### 11.4 Persistent Skill Cache (M4+)

**Optimization:** Cache compiled WASM modules to avoid recompilation

**Implementation:**
- Store compiled WASM in `/var/mimi/skill-cache/<skill_id>.wasm`
- Mount as read-only volume in container
- Invalidate cache on skill code change (hash-based)

---

### 11.5 Network Namespace Management (M5+)

**Advanced Isolation:** Per-skill network policies (whitelist/blacklist IPs)

**Implementation:**
- Use `iptables` rules in container namespace
- Allow outbound to specific domains only (e.g., `api.openai.com`)
- Block all inbound connections

**Config:**
```rust
network_policy: NetworkPolicy {
  mode: NetworkMode::Restricted,
  allowed_domains: vec!["api.openai.com".to_string()],
  allowed_ports: vec![443],
}
```

---

## Docker Configuration Examples

### Worker Base Image Dockerfile

```dockerfile
# File: docker/Dockerfile.worker
FROM rust:1.75-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Build worker runtime
COPY worker-runtime /app
WORKDIR /app
RUN cargo build --release --bin mimi-worker-runtime

# --- Production Image ---
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy worker binary
COPY --from=builder /app/target/release/mimi-worker-runtime /usr/local/bin/

# Create non-root user
RUN useradd -m -u 1000 -s /bin/bash worker

# Set working directory
WORKDIR /app
RUN chown worker:worker /app

# Switch to non-root user
USER worker

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD pgrep -u worker mimi-worker-runtime || exit 1

# Entrypoint
ENTRYPOINT ["mimi-worker-runtime"]
CMD ["--mode", "isolated"]
```

---

### Container Configuration Template

```rust
// File: ryzu-runtime/src/templates/default_config.rs
pub fn default_container_config() -> bollard::container::Config<String> {
  Config {
    image: Some("mimi-worker:latest".to_string()),
    hostname: Some("worker-isolated".to_string()),
    user: Some("worker".to_string()),
    working_dir: Some("/app".to_string()),
    
    host_config: Some(HostConfig {
      // Resource limits
      memory: Some(268_435_456),              // 256MB
      memory_swap: Some(268_435_456),         // No swap
      cpu_quota: Some(50_000),                // 50% CPU
      cpu_period: Some(100_000),
      pids_limit: Some(64),                   // Max 64 processes

      // Security
      network_mode: Some("none".to_string()),
      cap_drop: Some(vec!["ALL".to_string()]),
      security_opt: Some(vec![
        "no-new-privileges".to_string(),
        "apparmor=mimi-worker-profile".to_string(),
      ]),
      read_only_root_fs: Some(true),

      // Volumes
      binds: Some(vec![
        "/tmp:/tmp:rw".to_string(),           // Writable temp
        "/app/skill:/app/skill:ro".to_string(), // Read-only code
      ]),

      // Auto-remove on exit
      auto_remove: Some(true),

      // Init process (PID 1 reaper)
      init: Some(true),

      ..Default::default()
    }),

    ..Default::default()
  }
}
```

---

### Resource Limit Tuning Guide

#### Memory Limit Selection

| Skill Type | Recommended Limit | Rationale |
|------------|-------------------|-----------|
| **Rhai Script** | 128MB | Minimal overhead, no compilation |
| **WASM (Simple)** | 256MB | WASM runtime + linear memory |
| **WASM (Complex)** | 512MB | ML models, large data structures |
| **Native Binary** | 512MB | Full system libraries |

**Tuning Command:**
```bash
# Benchmark skill memory usage
docker stats --no-stream --format "{{.Name}}\t{{.MemUsage}}" mimi-worker-*

# Adjust limit in config
# ryzu-runtime/config.toml
[resource_limits]
rhai_memory_mb = 128
wasm_simple_memory_mb = 256
wasm_complex_memory_mb = 512
```

---

#### CPU Quota Tuning

| Quota | Use Case | Expected Perf |
|-------|----------|---------------|
| **25%** | I/O-bound (file processing) | Minimal impact |
| **50%** | Balanced (default) | Slight slowdown on CPU-heavy tasks |
| **100%** | CPU-intensive (crypto, compression) | Near-native performance |

**Calculation:**
```
cpu_quota = (desired_percent / 100) * cpu_period
Example: 50% CPU → 50_000 / 100_000
```

**Dynamic Tuning (Future):**
```rust
// Adjust quota based on skill profiling
fn auto_tune_cpu_quota(skill_id: &str) -> u64 {
  let profile = load_skill_profile(skill_id);
  match profile.cpu_intensity {
    CpuIntensity::Low => 25_000,
    CpuIntensity::Medium => 50_000,
    CpuIntensity::High => 100_000,
  }
}
```

---

## Cross-References

- **Requirements:** [REQUIREMENTS.md#RF-5](../REQUIREMENTS.md) — Execution Segura (Ryzu + Docker)
- **Milestone:** [milestones/M3-SECURITY.md](../milestones/M3-SECURITY.md) — Implementation roadmap
- **Related Modules:**
  - [ODLAGUNA.md](./ODLAGUNA.md) — Timeout enforcement and authorization
  - [ECHIDNA.md](./ECHIDNA.md) — Skill code generation (consumer of Ryzu)
  - [MIMI-COMMANDER.md](./MIMI-COMMANDER.md) — Task orchestration
- **Specs:**
  - [BUS-PROTOCOL.md](../specs/BUS-PROTOCOL.md) — Message Bus topics
  - [SECURITY-MODEL.md](../specs/SECURITY-MODEL.md) — Isolation guarantees

---

**End of Ryzu Module Design Document**
