# M3: Segurança e Supervisão (Odlaguna & Priscilla)

> **Milestone Objetivo:** Implementar watchdog, timeouts, isolamento de segurança e camada crítica de racionalidade  
> **Status:** 🟡 Bloqueado por M1+M2  
> **Duração Estimada:** 12 semanas (8 Odlaguna + 4 Priscilla)  
> **Dependências:** M1 (Message Bus) + M2 (Pandora)  

---

## Visão Geral

Milestone 3 constrói o sistema de supervisão, segurança e governança do MiMi. Duas guardiãs trabalham em conselho:

### Odlaguna: O Executor de Segurança
**"É seguro? Pode executar?"** — Autoridade executiva com poder de veto absoluto.

- **Supervisiona** todas as mensagens no Bus (não-bloqueante)
- **Aplica timeouts** (Lease/Deadline) a tarefas para evitar hanging
- **Envia SIGKILL** a processos que excedem deadline
- **Circuit Breaker** bloqueia skills com falhas repetidas
- **Valida código** gerado (AST parsing) antes de deploy
- **Audit Trail** imutável de todas as operações

### Priscilla: A Advocada do Diabo
**"É inteligente? Existe um jeito melhor?"** — Consultora estratégica sem poder de veto, força Mimi a pensar.

- **Questiona a necessidade** de cada tarefa (detecção de loops, redundância)
- **Analisa custo-benefício** (tokens, CPU, memória vs. resultado esperado)
- **Detecta viés e alucinação** da Beatrice (clarificação de intent antes de planejar)
- **Refina planos** (sugere caminhos mais curtos, parallelização, reutilização de skills)
- **Aprende de falhas** (integração com Pandora para padrões históricos)
- **Metacognição**: Força o sistema a ser reflexivo, não impulsivo

### Hierarquia de Decisão
```
Beatrice (Captura Desejo) 
  ↓
Mimi (Elabora Estratégia)
  ↓
Priscilla (Questiona Lógica — Advisoria)
  ↓
Odlaguna (Verifica Segurança — VETO)
  ↓
Ryzu/Echidna (Executa)
```

**Diferença crítica:** Priscilla **nunca bloqueia**, apenas **questiona**. Odlaguna bloqueia se necessário.

## Tarefas por Hierarquia

### T3.0: Docker + Ryzu Worker Isolation (🔴 CRÍTICO)
**Bloqueado por:** M1  
**Bloqueia:** T3.1, T3.2, T3.3  

**Descrição:**
- Setup Docker para isolamento de skills
- Implementar Ryzu (C++/Rust) como worker orchestrator
- Resource limits (CPU, RAM, network)
- Process management (spawn, monitor, kill)
- Stdout/stderr capture

**Dependências Técnicas:**
- `docker` API (via `docker` crate ou `tonic`)
- `libc` para process management
- `tokio` para async process handling

**Artefatos:**
- `ryzu-runtime/src/docker_manager.rs` — Docker orchestration
- `ryzu-runtime/src/worker.rs` — Worker lifecycle
- `docker/Dockerfile.worker` — Worker base image
- `docker-compose.test.yml` — Test environment

**Estrutura do Código (Rust):**
```rust
// ryzu-runtime/src/docker_manager.rs
use docker_api::Docker;
use std::collections::HashMap;

pub struct DockerWorker {
    docker: Docker,
    container_id: String,
}

impl DockerWorker {
    pub async fn spawn(
        &self,
        skill_code: &str,
        timeout_ms: u64,
        memory_limit_mb: u32,
    ) -> Result<WorkerHandle, Error> {
        let config = bollard::container::Config {
            image: Some("mimi-worker:latest"),
            hostname: Some("worker-isolated"),
            host_config: Some(bollard::models::HostConfig {
                memory: Some((memory_limit_mb as i64) * 1024 * 1024),
                cpu_quota: Some(50000), // 50% CPU
                network_mode: Some("none"), // No network por padrão
                ..Default::default()
            }),
            ..Default::default()
        };

        let container = self.docker.create_container(
            Some(bollard::container::CreateContainerOptions {
                name: format!("mimi-worker-{}", uuid()),
                ..Default::default()
            }),
            config,
        ).await?;

        self.docker.start_container(&container.id, None).await?;
        
        Ok(WorkerHandle {
            container_id: container.id,
            timeout_ms,
        })
    }

    pub async fn wait_with_timeout(
        handle: &WorkerHandle,
    ) -> Result<ExitCode, Error> {
        tokio::time::timeout(
            Duration::from_millis(handle.timeout_ms),
            self.docker.wait_container(&handle.container_id, None),
        )
        .await
        .map_err(|_| Error::Timeout)?
        .map(|res| res.status_code.unwrap_or(-1) as i32)
    }

    pub async fn kill_container(&self, container_id: &str) -> Result<(), Error> {
        self.docker.kill_container(container_id, None).await?;
        self.docker.remove_container(container_id, None).await?;
        Ok(())
    }
}
```

**Worker Base Image (Dockerfile):**
```dockerfile
FROM rust:1.75-slim as builder
RUN apt-get update && apt-get install -y \
    build-essential \
    clang \
    && rm -rf /var/lib/apt/lists/*

COPY runtime/worker-runtime /app
WORKDIR /app
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mimi-worker-runtime /usr/local/bin/

# Non-root user
RUN useradd -m -u 1000 worker
USER worker

ENTRYPOINT ["mimi-worker-runtime"]
```

**DoD:**
- [ ] Docker container inicia sem erros
- [ ] Resource limits aplicados (CPU, RAM testados)
- [ ] Network isolada (ping localhost falha fora do container)
- [ ] Stdout/stderr capturados
- [ ] Process kill funciona < 100ms

---

### T3.1: Fueling & Instruction Limiting (🔴 CRÍTICO)
**Bloqueado por:** T3.0 (Docker)  
**Bloqueia:** T3.2, T3.3  

**Descrição:**
- Implementar "fueling" (limite de instruções) para WASM
- Contador de instruções em Wasmtime
- Abort execução se fuel esgota
- Tune fuel limits por tipo de skill

**Dependências Técnicas:**
- `wasmtime` crate com suporte de fuel
- Instrumentação de WASM para contar instruções

**Artefatos:**
- `ryzu-runtime/src/fuel_manager.rs` — Fuel impl
- `ryzu-runtime/tests/fuel_limits_tests.rs` — Testes

**Estrutura:**
```rust
// ryzu-runtime/src/fuel_manager.rs
use wasmtime::{Engine, Linker, Module, Instance, Store};

pub struct FueledExecutor {
    engine: Engine,
    fuel_per_skill_type: HashMap<String, u64>,
}

impl FueledExecutor {
    pub fn new() -> Self {
        let mut config = wasmtime::Config::new();
        config.wasm_multi_value(true);
        config.wasm_bulk_memory(true);
        config.consume_fuel(true); // Ativar fuel tracking
        
        Self {
            engine: Engine::new(&config).unwrap(),
            fuel_per_skill_type: Self::default_fuel_limits(),
        }
    }

    pub async fn execute_with_fuel(
        &self,
        wasm_bytes: &[u8],
        skill_type: &str,
    ) -> Result<String, FuelError> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        let mut store = Store::new(&self.engine, ());
        
        // Allocate fuel
        let fuel_limit = self.fuel_per_skill_type
            .get(skill_type)
            .copied()
            .unwrap_or(1_000_000);
        store.add_fuel(fuel_limit)?;
        
        let mut linker = Linker::new(&self.engine);
        linker.allow_shadowing(true);
        
        // Add host functions
        linker.func_wrap("env", "log", |mut caller: Caller<_>, val: i32| {
            println!("WASM log: {}", val);
            Ok(())
        })?;
        
        let instance = linker.instantiate(&mut store, &module)?;
        let main = instance.get_typed_func::<(), i32>(&mut store, "main")?;
        
        match main.call(&mut store, ()) {
            Ok(result) => Ok(format!("{}", result)),
            Err(e) if e.to_string().contains("out of fuel") => {
                Err(FuelError::FuelExhausted)
            },
            Err(e) => Err(FuelError::ExecutionError(e)),
        }
    }

    fn default_fuel_limits() -> HashMap<String, u64> {
        [
            ("simple".to_string(), 100_000),
            ("medium".to_string(), 1_000_000),
            ("complex".to_string(), 10_000_000),
        ].iter().cloned().collect()
    }
}
```

**DoD:**
- [ ] Fuel tracking compila
- [ ] WASM execution respeita fuel limit
- [ ] Execução aborta gracefully ao esgotar fuel
- [ ] Fuel limits são configuráveis por skill type

---

### T3.2: Timeout & Lease Management (🟡 Alta)
**Bloqueado por:** T3.0 (Docker)  
**Bloqueia:** T3.3  

**Descrição:**
- Cada tarefa recebe Lease com deadline
- Odlaguna monitora timeout
- Ao expirar: SIGKILL → cleanup
- Callback para notificar Mimi

**Dependências Técnicas:**
- `tokio::time::sleep`
- Signal handling (POSIX)
- Timer wheel data structure

**Artefatos:**
- `odlaguna-guard/src/lease.rs` — Lease management
- `odlaguna-guard/src/timer_wheel.rs` — Timer wheel

**Estrutura:**
```rust
// odlaguna-guard/src/lease.rs
use std::time::{Duration, SystemTime};
use uuid::Uuid;

pub struct Lease {
    pub id: String,
    pub created_at: SystemTime,
    pub deadline: SystemTime,
    pub container_id: Option<String>,
    pub callback_topic: String,
}

impl Lease {
    pub fn new(timeout_ms: u64, container_id: Option<String>) -> Self {
        let now = SystemTime::now();
        let deadline = now + Duration::from_millis(timeout_ms);
        
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            deadline,
            container_id,
            callback_topic: "task/timeout".to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.deadline
    }

    pub fn time_remaining(&self) -> Option<Duration> {
        self.deadline.duration_since(SystemTime::now()).ok()
    }
}

// odlaguna-guard/src/timer_wheel.rs
pub struct TimerWheel {
    leases: Arc<RwLock<HashMap<String, Lease>>>,
    bus: BusClient,
}

impl TimerWheel {
    pub async fn monitor_leases(&self) {
        loop {
            let now = SystemTime::now();
            let expired: Vec<_> = {
                let leases = self.leases.read().await;
                leases.values()
                    .filter(|l| l.is_expired())
                    .map(|l| l.clone())
                    .collect()
            };
            
            for lease in expired {
                // Kill container
                if let Some(container_id) = &lease.container_id {
                    kill_container(container_id).await;
                }
                
                // Notify via Bus
                self.bus.publish(
                    &lease.callback_topic,
                    &TaskTimeoutMessage { lease_id: lease.id },
                ).await;
                
                // Remove from tracking
                self.leases.write().await.remove(&lease.id);
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
```

**DoD:**
- [ ] Lease é criado com deadline
- [ ] Timer wheel monitora leases
- [ ] Expiração dispara SIGKILL
- [ ] Callback publicado no Bus

---

### T3.3: Circuit Breaker & Reliability Tracking (🟡 Alta)
**Bloqueado por:** T3.0 (Docker)  
**Bloqueia:** T3.4  

**Descrição:**
- Rastrear falhas de skill (execution count, success rate)
- Após 3 falhas consecutivas: circuit breaker ABERTO
- Skill bloqueada até reset manual
- Notificação em Audit Trail

**Dependências Técnicas:**
- Neo4j para armazenar state
- Rust state machine

**Artefatos:**
- `odlaguna-guard/src/circuit_breaker.rs` — Circuit breaker logic
- `odlaguna-guard/tests/circuit_breaker_tests.rs`

**Estrutura:**
```rust
// odlaguna-guard/src/circuit_breaker.rs
#[derive(Debug, Clone)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    skill_id: String,
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<AtomicU32>,
    success_count: Arc<AtomicU32>,
    failure_threshold: u32,
    success_threshold_for_reset: u32,
}

impl CircuitBreaker {
    pub fn new(skill_id: String, failure_threshold: u32) -> Self {
        Self {
            skill_id,
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            failure_threshold,
            success_threshold_for_reset: 3,
        }
    }

    pub async fn can_execute(&self) -> bool {
        let state = self.state.lock().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => false,
            CircuitState::HalfOpen => true,
        }
    }

    pub async fn record_failure(&self) {
        let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        if count >= self.failure_threshold {
            let mut state = self.state.lock().await;
            *state = CircuitState::Open;
            
            // Log para Audit Trail
            log_to_audit_trail(&format!(
                "Circuit breaker OPENED for skill {}",
                self.skill_id
            ));
        }
    }

    pub async fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        
        let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
        
        if count >= self.success_threshold_for_reset {
            let mut state = self.state.lock().await;
            *state = CircuitState::Closed;
            self.success_count.store(0, Ordering::Relaxed);
            
            log_to_audit_trail(&format!(
                "Circuit breaker CLOSED for skill {}",
                self.skill_id
            ));
        }
    }
}
```

**DoD:**
- [ ] Circuit breaker state machine funciona
- [ ] Falhas são rastreadas
- [ ] Circuit abre após 3 falhas
- [ ] Reset funciona após sucessos

---

### T3.4: Code Validation & AST Parsing (🟡 Alta)
**Bloqueado por:** T3.3 (Circuit Breaker)  
**Bloqueia:** T3.5  

**Descrição:**
- Parser de código gerado (Rhai ou WASM)
- Validar AST para operações proibidas (rm -rf, network, etc)
- Whitelist de operações permitidas
- Rejeitar código malicioso

**Dependências Técnicas:**
- `tree-sitter` ou `syn` para parsing
- AST traversal e validation

**Artefatos:**
- `odlaguna-guard/src/code_validator.rs` — Validator
- `odlaguna-guard/tests/code_validation_tests.rs`

**Estrutura:**
```rust
// odlaguna-guard/src/code_validator.rs
pub struct CodeValidator;

impl CodeValidator {
    pub fn validate_rhai_code(code: &str) -> Result<(), ValidationError> {
        // Parse Rhai AST
        let ast = rhai::Engine::new()
            .compile_expression(code)
            .map_err(|e| ValidationError::ParseError(e.to_string()))?;
        
        // Traverse e validar
        Self::check_forbidden_operations(&ast)?;
        Ok(())
    }

    fn check_forbidden_operations(ast: &rhai::AST) -> Result<(), ValidationError> {
        // Forbidden patterns
        let forbidden = [
            "fs::remove_dir_all",
            "std::process::Command::new(\"rm\")",
            "std::process::Command::new(\"shutdown\")",
            "/etc/passwd",
            "net::TcpStream",
        ];

        let code_str = format!("{:?}", ast);
        
        for pattern in &forbidden {
            if code_str.contains(pattern) {
                return Err(ValidationError::ForbiddenOperation(
                    pattern.to_string()
                ));
            }
        }
        
        Ok(())
    }

    pub fn validate_wasm_imports(wasm_module: &wasmtime::Module) -> Result<(), ValidationError> {
        // Validar imports do WASM
        // Rejeitar imports de libc, network functions
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reject_rm_rf() {
        let code = r#"
            import "fs" as fs;
            fs.remove_dir_all("/");
        "#;
        
        assert!(CodeValidator::validate_rhai_code(code).is_err());
    }

    #[test]
    fn test_allow_math_operations() {
        let code = r#"
            let x = 5 + 3;
            let y = x * 2;
        "#;
        
        assert!(CodeValidator::validate_rhai_code(code).is_ok());
    }
}
```

**DoD:**
- [ ] AST parser funciona
- [ ] Operações proibidas são detectadas
- [ ] Operações permitidas passam
- [ ] Tests cobrem 5+ cenários maliciosos

---

### T3.5: Audit Trail Implementation (🟡 Alta)
**Bloqueado por:** T3.0 (Neo4j via M2)  
**Bloqueia:** Nada (M3 complete)  

**Descrição:**
- Log imutável de todas as operações
- Armazenar em Neo4j com timestamp
- Trace: quem executou, quando, resultado
- Query audit para compliance

**Dependências Técnicas:**
- Neo4j (de M2)
- Cypher para append-only log

**Artefatos:**
- `pandora-memory/src/audit_trail.cpp` — Audit impl
- `specs/AUDIT-TRAIL-SCHEMA.md` — Schema

**Estrutura (Cypher):**
```cypher
// Nó AuditLog
CREATE (a:AuditLog {
  id: STRING (UUID),
  timestamp: DATETIME,
  actor: STRING (mimi, echidna, odlaguna, etc),
  action: STRING (execute, create, delete, validate),
  target_id: STRING (skill_id, task_id, etc),
  target_type: STRING (skill, task, memory),
  status: STRING (success, failure, timeout),
  details: STRING (JSON serialized),
  duration_ms: INTEGER
})

// Índice para queries rápidas
CREATE INDEX ON :AuditLog(timestamp);
CREATE INDEX ON :AuditLog(actor);
CREATE INDEX ON :AuditLog(status);

// Query: todos os eventos em últimas 24h
MATCH (a:AuditLog)
WHERE a.timestamp > datetime() - duration({days: 1})
RETURN a ORDER BY a.timestamp DESC
LIMIT 1000
```

**DoD:**
- [ ] Audit trail salva operações
- [ ] Timestamps são imutáveis
- [ ] Queries audit funcionam < 100ms
- [ ] Zero perda de logs

---

## Requisitos Não-Funcionais Aplicáveis

| RNF | Alvo | Status |
|-----|------|--------|
| **RNF-2** (Segurança) | Docker isolation | ✅ T3.0 |
| **RNF-2** (Segurança) | Sem acesso /etc | ✅ T3.4 |
| **RNF-3** (Resiliência) | Zombie cleanup < 5s | ✅ T3.2 |
| **RNF-3** (Resiliência) | Zero data loss | ✅ T3.5 |

---

## Timeline

| Semana | Tarefa | Deliverable |
|--------|--------|-------------|
| 1-2 | T3.0 (Docker + Ryzu) | Worker orchestration |
| 2-3 | T3.1 (Fueling) | WASM fuel limits |
| 3-4 | T3.2 (Timeouts) | Lease + Timer wheel |
| 4-5 | T3.3 (Circuit Breaker) | Reliability tracking |
| 5-6 | T3.4 (Code Validation) | AST parsing + whitelist |
| 6-7 | T3.5 (Audit Trail) | Imutable log |
| 7-8 | Tests + Buffer | Integration, benchmarks |

---

## Critérios de Aceitação Finais (M3 DoD)

✅ **Milestone 3 Completo quando:**

- [ ] Docker container isolado + Ryzu rodando
- [ ] Odlaguna monitorando Bus
- [ ] Timeouts funcionam (tarefa+5s executa → kill)
- [ ] Skill simples (Bash) executa under Odlaguna
- [ ] Audit Trail completo no Neo4j
- [ ] Circuit Breaker ativa após 3 falhas
- [ ] Code validation rejeita operações maliciosas
- [ ] Zero kernel panics
- [ ] All tests pass

---

## Bloqueadores Conhecidos

| Bloqueador | Risco | Mitigação |
|-----------|-------|-----------|
| **Docker API** | Complexo | Usar `docker` crate oficial |
| **Wasmtime fuel** | Pode ser granular demais | Tune com benchmarks |
| **Signal handling POSIX** | Windows incompatível | WSL2 em CI |

---

## Notas

- **M3 é sobre segurança** — Antes de M4 evoluir
- **Docker obrigatório** — Sem isolamento, sem deploy
- **Circuit breaker é resilience** — Não deixa skill quebrada looping

---

## Referências Cruzadas

- Volta a: [`REQUIREMENTS.md#RF-6`](../REQUIREMENTS.md#rf-6-supervisão-e-watchdog-odlaguna)
- Anterior: [`milestones/M2-PANDORA.md`](M2-PANDORA.md)
- Próximo: [`milestones/M4-ECHIDNA.md`](M4-ECHIDNA.md)

---

## Tasks para Priscilla (T3.5+)

### T3.5: Priscilla - Message Monitor & Critique Framework (🟡 ALTA)
**Bloqueado por:** T3.0 (Docker setup), M2 (Pandora integração)  
**Bloqueia:** T3.6, T3.7  

**Descrição:**
- Implementar async subscriber para task/draft topic
- Critique analysis engine com 5 analyzers:
  1. Necessity checker (loop/redundancy detection)
  2. Cost-benefit analyzer (token/CPU budget)
  3. Intent validator (bias detection from Beatrice)
  4. Failure pattern analyzer (Pandora link)
  5. Plan optimization suggester (skill reuse, parallelization)
- Message protocol: TaskDraftMessage → CritiqueCommentaryMessage
- Configuration system (priscilla-rules.toml with cynicism levels)

**Dependências Técnicas:**
- Zenoh/NATS subscriber (via bus crate)
- Neo4j query interface (read-only access to Pandora)
- Tokio async runtime
- Serde for JSON serialization

**Artefatos:**
- `priscilla/src/lib.rs` — Core module
- `priscilla/src/analyzers/mod.rs` — All 5 analyzers
- `priscilla/src/message_protocol.rs` — Message types
- `priscilla-rules.toml` — Configuration template
- `priscilla/tests/` — Comprehensive test suite (50+ scenarios)

### T3.6: Priscilla - Failure Pattern Integration (🟡 ALTA)
**Bloqueado por:** T3.5, M2 finalized  

**Descrição:**
- Integrate with Pandora's read-only failure indices
- Query builder for similar task matching (semantic similarity)
- Temporal weighting of failures (recent failures > old failures)
- Commentary generation from pattern analysis
- Performance optimization (< 20ms Pandora queries)

**Dependências:**
- Pandora Neo4j schema finalized
- Vector similarity implemented in Pandora
- Cached query results (avoid latency)

**Artefatos:**
- `priscilla/src/pandora_client.rs` — Read-only Pandora queries
- `priscilla/src/failure_patterns.rs` — Pattern matching logic
- Neo4j query templates (in documentation)

### T3.7: Priscilla - Cynicism Controller & Configuration (🟢 MÉDIA)
**Bloqueado por:** T3.5  

**Descrição:**
- Implement risk level parametrization (CREATIVE, OPERATIONAL, CRITICAL)
- Dynamic scrutiny adjustment based on task risk
- Configuration hot-reloading (no restart needed)
- Metrics/observability (override rates, improvement estimates)

**Dependências:**
- T3.5 analyzers implemented
- Prometheus integration (metrics export)

**Artefatos:**
- `priscilla/src/cynicism_controller.rs` — Risk level logic
- `priscilla/src/metrics.rs` — Prometheus metrics
- Config parsing and hot-reload logic

### T3.8: Priscilla - Integration Testing & Documentation (🟢 MÉDIA)
**Bloqueado by:** T3.5, T3.6, T3.7  

**Descrição:**
- Full integration tests (Beatrice → Mimi → Priscilla → Odlaguna → Ryzu pipeline)
- Comprehensive operator guide (how to tune cynicism levels)
- API documentation (message protocol, configuration options)
- Behavioral examples (creative, operational, critical tasks)

**Artefatos:**
- `integration-tests/priscilla_full_flow.rs`
- `docs/priscilla-operator-guide.md`
- `docs/priscilla-api.md`
- Example critique logs (real scenarios)

