# MiMi — Requisitos Funcionais e Não-Funcionais

## Requisitos Funcionais (RF)

### RF-1: Orquestração Central (Mimi)
- **RF-1.1:** Sistema deve receber intenções estruturadas via Message Bus
- **RF-1.2:** Mimi deve decidir qual módulo (Pandora, Echidna, Ryzu) executa cada tarefa
- **RF-1.3:** Mimi deve manter estado persistente em Pandora entre sessões
- **RF-1.4:** Mimi deve respeitar prioridades de tarefa (HIGH, MEDIUM, LOW)

**Bloqueia:** Beatrice (precisa de orquestrador), Pandora (recebe queries), Echidna (recebe tarefas)

---

### RF-2: Interface NLP (Beatrice)
- **RF-2.1:** Beatrice deve converter linguagem natural em Intent estruturado
- **RF-2.2:** Intent deve conter: `{user_message, entities[], intent_type, confidence, timestamp}`
- **RF-2.3:** Suportar entrada via CLI, API HTTP, e future WebSocket
- **RF-2.4:** Validar Intent antes de enviar para Mimi

**Bloqueada por:** Mimi  
**Bloqueia:** Interação final com utilizador

---

### RF-3: Memória em Grafos (Pandora)
- **RF-3.1:** Conectar a Neo4j via Bolt Driver (C++)
- **RF-3.2:** Criar nós de tipo: `ContextNode, Entity, Skill, Task, Memory`
- **RF-3.3:** Implementar algoritmo de Heatmap com decaimento exponencial
- **RF-3.4:** BFS limitado por calor — retornar subgrafo relevante
- **RF-3.5:** Manter LRU Cache (L1) para contexto imediato em RAM

**Bloqueada por:** Mimi  
**Bloqueia:** Contexto para Mimi, Validação para Odlaguna

---

### RF-4: Criação Dinâmica de Skills (Echidna)
- **RF-4.1:** Detectar padrões de tarefas repetitivas na Pandora
- **RF-4.2:** Gerar scripts Rhai para automações simples
- **RF-4.3:** Gerar/compilar WASM para ferramentas complexas
- **RF-4.4:** Registar skill criada na Pandora como ContextNode
- **RF-4.5:** Submeter skill para validação da Odlaguna antes de deploy

**Bloqueada por:** Pandora (saber o que já existe), Odlaguna (validação)  
**Bloqueia:** Capacidade autónoma de evolução

---

### RF-5: Execução Segura (Ryzu + Docker)
- **RF-5.1:** Executar skills em container Docker isolado
- **RF-5.2:** Aplicar fueling (limite de instruções) a execução WASM
- **RF-5.3:** Aplicar limites de CPU/RAM ao container
- **RF-5.4:** Isolar rede do container por padrão
- **RF-5.5:** Capturar stdout/stderr e tempo de execução

**Bloqueada por:** Odlaguna (decisão de executar)  
**Bloqueia:** Nada — é "leaf" node na execução

---

### RF-6: Supervisão e Watchdog (Odlaguna)
- **RF-6.1:** Monitorar todas as mensagens no Message Bus
- **RF-6.2:** Aplicar timeout para tarefas (Lease/Deadline)
- **RF-6.3:** Enviar SIGKILL a processos que excedem deadline
- **RF-6.4:** Implementar Circuit Breaker para skills com falhas repetidas
- **RF-6.5:** Validar código gerado (AST parsing) antes de deploy
- **RF-6.6:** Gerar Audit Trail de todas as operações

**Bloqueada por:** Message Bus (precisa ouvir)  
**Bloqueia:** Echidna (validação), Ryzu (autorização para executar)

---

### RF-7: Message Bus
- **RF-7.1:** Implementar broker Zenoh/NATS com suporte Pub/Sub e Request-Response
- **RF-7.2:** Serializar mensagens com FlatBuffers (zero-copy)
- **RF-7.3:** Topics padrão: `intent/raw`, `task/create_skill`, `skill/review`, `skill/deploy`, `task/execute`, `task/result`, `memory/update`
- **RF-7.4:** Garantir entrega de mensagens críticas (at-least-once)

**Bloqueia:** TUDO — é a espinha dorsal de comunicação

---

### RF-8: Adaptadores de IA
- **RF-8.1:** Trait `AIAdapter` com métodos: `generate()`, `stream()`, `get_cost_estimate()`
- **RF-8.2:** Implementar GeminiAdapter (Cloud API)
- **RF-8.3:** Implementar OllamaAdapter (Local HTTP)
- **RF-8.4:** Suportar dynamic loading de adaptadores (.so / .dll)

**Bloqueada por:** Mimi (chamar adaptador)  
**Bloqueia:** Nada — é "leaf" node

---

## Requisitos Não-Funcionais (RNF)

### RNF-1: Performance
| Métrica | Alvo | Justificativa |
|---------|------|--------------|
| Latência Mimi ↔ Pandora | < 5ms | Comunicação intra-módulo via FFI/Unix Socket |
| Latência Bus | < 1ms | Mensagens pequenas em FlatBuffers |
| Query Neo4j | < 50ms | Máximo para subgrafo relevante (100-500 nós) |
| Skill Rhai | < 100ms | Automação simples, sem I/O |
| Skill WASM | < 500ms | Ferramentas complexas mas com isolamento |
| Overhead Odlaguna | < 10% CPU | Monitoramento não-bloqueante |

---

### RNF-2: Segurança
- **RNF-2.1:** Toda execução isolada em container Docker ou WASM
- **RNF-2.2:** Skill gerada não pode:
  - Aceder a `/etc/passwd`, `/root/`, `/sys`, `/proc`
  - Fazer `rm -rf /`, `shutdown`, `reboot`
  - Abrir conexões de rede (por padrão)
- **RNF-2.3:** Código Rust deve usar `cargo clippy` e `cargo audit`
- **RNF-2.4:** Buffer overflows impossíveis (Rust) ou detectados (C++ com AddressSanitizer)

---

### RNF-3: Resiliência
- **RNF-3.1:** Se um módulo cai, outros continuam operacionais (Message Bus isola)
- **RNF-3.2:** Se Mimi cai, Pandora preserva estado em Neo4j
- **RNF-3.3:** Se Ryzu/Docker fica zombie, Odlaguna detecta em < 5s e limpa
- **RNF-3.4:** Máximo 0 data loss — Audit Trail é imutável

---

### RNF-4: Escalabilidade
- **RNF-4.1:** Suportar 1000+ skills registadas na Pandora
- **RNF-4.2:** Suportar 100+ tasks concorrentes no Bus
- **RNF-4.3:** Grafo Neo4j pode crescer a 1M+ nós sem degradação substancial

---

### RNF-5: Manutenibilidade
- **RNF-5.1:** Documentação inline em código (rustdoc, doxygen)
- **RNF-5.2:** Testes unitários ≥ 80% coverage (Rust)
- **RNF-5.3:** Cada módulo tem README.md próprio
- **RNF-5.4:** CI/CD via GitHub Actions (build, test, lint)

---

### RNF-6: Compatibilidade
- **RNF-6.1:** Suportar Linux (Ubuntu 20.04+), macOS (12+), Windows (WSL2)
- **RNF-6.2:** Suportar Rust 1.70+, C++17+
- **RNF-6.3:** Neo4j 4.4+ ou 5.x

---

## Critérios de Aceitação por Milestone

### Milestone 1: Espinha Dorsal
✅ **DoD (Definition of Done):**
- [ ] Message Bus funcional (Zenoh ou NATS rodando)
- [ ] Mimi Core consegue receber e rotear mensagens
- [ ] Adaptador Gemini integrado
- [ ] Beatrice CLI envia Intent via Bus
- [ ] Fluxo end-to-end: `User → Beatrice → Mimi → Gemini → Bus → Beatrice`
- [ ] Tests unitários passam
- [ ] Zero lint errors (clippy, cargo check)

---

### Milestone 2: Palácio da Memória
✅ **DoD:**
- [ ] Neo4j rodando em container
- [ ] Pandora conectada via Bolt (C++)
- [ ] Heatmap implementado (fórmula + query Cypher)
- [ ] Mimi consulta Pandora antes de responder
- [ ] LRU Cache L1 funcionando
- [ ] Queries Neo4j < 50ms em média
- [ ] Tests de integração passam

---

### Milestone 3: Segurança e Supervisão
✅ **DoD:**
- [ ] Docker container isolado + Ryzu rodando
- [ ] Odlaguna monitorando Bus
- [ ] Timeouts funcionam (tarefa+5s executa → kill)
- [ ] Skill simples (Bash) executa under Odlaguna
- [ ] Audit Trail completo no Neo4j
- [ ] Circuit Breaker ativa após 3 falhas
- [ ] Zero kernel panics

---

### Milestone 4: Evolução Autónoma
✅ **DoD:**
- [ ] Echidna detecta padrão repetido → gera Rhai script
- [ ] Rhai script executa via Ryzu
- [ ] WASM runtime (Wasmtime) integrado
- [ ] Echidna compila skill → WASM
- [ ] Skill WASM passa validação Odlaguna → deploy
- [ ] Próxima execução usa skill em cache
- [ ] End-to-end: problema → evolução → resolução

---

## Matriz de Rastreabilidade

| RF/RNF | Milestone | Módulo | Documento |
|--------|-----------|--------|-----------|
| RF-1 | M1 | Mimi | `modules/MIMI-COMMANDER.md` |
| RF-2 | M1 | Beatrice | `modules/BEATRICE.md` |
| RF-3 | M2 | Pandora | `modules/PANDORA.md` |
| RF-4 | M4 | Echidna | `modules/ECHIDNA.md` |
| RF-5 | M3 | Ryzu | `modules/RYZU.md` |
| RF-6 | M3 | Odlaguna | `modules/ODLAGUNA.md` |
| RF-7 | M1 | Bus | `specs/BUS-PROTOCOL.md` |
| RF-8 | M1 | Adaptadores | `specs/AI-ADAPTERS.md` |
| RNF-1 | All | Architecture | `ARCHITECTURE.md` |
| RNF-2 | M3 | Security | `specs/SECURITY-MODEL.md` |
| RNF-3 | All | Message Bus | `specs/BUS-PROTOCOL.md` |
| RNF-4 | All | Architecture | `ARCHITECTURE.md` |
| RNF-5 | All | DevOps | `.github/workflows/` |
| RNF-6 | All | Build | `Cargo.toml`, `CMakeLists.txt` |
