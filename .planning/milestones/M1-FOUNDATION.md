# M1: Espinha Dorsal (Foundation)

> **Milestone Objetivo:** Estabelecer a infraestrutura de comunicação base do MiMi  
> **Status:** 🟡 Não iniciado  
> **Duração Estimada:** 6-8 semanas  
> **Dependências:** Nenhuma (milestone de fundação)  

---

## Visão Geral

O Milestone 1 constrói os pilares sobre os quais toda a comunicação e orquestração do MiMi assentam:

1. **Message Bus funcional** — Zenoh/NATS como espinha dorsal
2. **Mimi Core** — Orquestrador central (Rust) que roteia mensagens
3. **Beatrice CLI** — Interface de entrada que traduz intenção natural
4. **AI Adapter (Gemini)** — Primeira integração com LLM externo
5. **Fluxo end-to-end** — Ciclo completo: Utilizador → Intent → Mimi → LLM → Resposta

---

## Requisitos Funcionais (RF) Ativados

| RF | Descrição | Prioridade |
|----|-----------|-----------|
| **RF-7** | Message Bus (Zenoh/NATS + FlatBuffers) | 🔴 Bloqueante |
| **RF-1** | Orquestração Central (Mimi Core) | 🔴 Bloqueante |
| **RF-2** | Interface NLP (Beatrice) | 🟡 Alta |
| **RF-8** | Adaptadores de IA (Gemini) | 🟡 Alta |

**Outros requisitos (dependem de M1):**
- RF-3 (Pandora) — bloqueado até Message Bus estar pronto
- RF-4 (Echidna) — bloqueado até Pandora estar pronto
- RF-5 (Ryzu) — bloqueado até Odlaguna estar pronto
- RF-6 (Odlaguna) — bloqueado até Message Bus estar pronto

---

## Tarefas por Hierarquia

### T1.0: Message Bus Setup (🔴 CRÍTICO)
**Bloqueado por:** Nada  
**Bloqueia:** T1.1, T1.2, T1.3, T1.4  

**Descrição:**
- Escolher entre Zenoh vs NATS (análise comparativa)
- Setup inicial do broker em container Docker
- Definir schema de Topics (Pub/Sub)
- Implementar FlatBuffers serialização
- Testes de latência (target: < 1ms)

**Dependências Técnicas:**
- `zenoh-rs` crate ou `nats` crate
- `flatbuffers` crate para definições .fbs
- Docker Compose para broker
- `tokio` runtime (já que Rust)

**Artefatos:**
- `proto/*.fbs` — Definições FlatBuffers
- `mimi-bus/` — Módulo de comunicação Rust
- `docker-compose.yml` — Setup broker
- `.github/workflows/bus-latency-tests.yml` — CI/CD

**DoD:**
- [ ] Broker rodando sem erros
- [ ] Pub/Sub funciona (testes passam)
- [ ] Request-Response pattern implementado
- [ ] Latência < 1ms comprovada (benchmark)
- [ ] 0 clippy warnings

---

### T1.1: Mimi Core Orquestrador (🔴 CRÍTICO)
**Bloqueado por:** T1.0 (Message Bus)  
**Bloqueia:** T1.2, T1.3, T1.4  

**Descrição:**
- Implementar state machine de Mimi
- Router de mensagens (match em topic, dispatch para handler)
- Priority queue para tarefas (HIGH, MEDIUM, LOW)
- Connection pool ao Message Bus
- Logging e telemetria estruturada

**Dependências Técnicas:**
- `tokio` para async
- `serde` para serialização
- `tracing` ou `log` para logging
- FlatBuffers bindings (de T1.0)

**Artefatos:**
- `mimi-commander/src/core/state.rs` — State machine
- `mimi-commander/src/router.rs` — Message routing
- `mimi-commander/src/priority_queue.rs` — Task scheduling
- `mimi-commander/tests/integration_tests.rs` — Testes end-to-end

**Estrutura do Código (Rust):**
```rust
// mimi-commander/src/core.rs
pub struct MimiCore {
    bus_client: Arc<BusClient>,
    state: Arc<Mutex<MimiState>>,
    task_queue: Arc<PriorityQueue<Task>>,
    handlers: HashMap<String, Box<dyn MessageHandler>>,
}

impl MimiCore {
    pub async fn start(&self) {
        // Subscribe aos topics críticos
        self.bus_client.subscribe("intent/raw", |msg| {
            self.route_intent(msg)
        }).await;
    }
    
    pub async fn route_intent(&self, intent: Intent) {
        match intent.intent_type {
            IntentType::Query => self.handle_query(intent).await,
            IntentType::Action => self.handle_action(intent).await,
            IntentType::SkillCreation => self.handle_skill_request(intent).await,
        }
    }
}
```

**DoD:**
- [ ] State machine passa testes (5+ cenários)
- [ ] Router roteia 100 msgs/sec sem erro
- [ ] Priority queue funciona (HIGH tasks executam primeiro)
- [ ] Logging estruturado para toda operação
- [ ] 0 clippy warnings

---

### T1.2: Beatrice CLI Interface (🟡 Alta)
**Bloqueado por:** T1.0, T1.1 (Bus + Mimi)  
**Bloqueia:** T1.4  

**Descrição:**
- Implementar CLI que aceita linguagem natural
- Converter texto → Intent estruturado (JSON)
- Enviar Intent via Bus para `intent/raw` topic
- Ouvir resposta no topic `task/result`
- Validar Intent antes de enviar

**Dependências Técnicas:**
- `clap` para CLI parsing
- `reqwest` se REPL HTTP (opcional em M1)
- FlatBuffers bindings
- Modelo simples de NLP (regex-based ou Ollama local)

**Artefatos:**
- `beatrice-ui/src/cli.rs` — CLI main loop
- `beatrice-ui/src/intent_parser.rs` — NLP logic
- `beatrice-ui/tests/cli_tests.rs` — CLI behavior tests

**Estrutura do Código (Rust):**
```rust
// beatrice-ui/src/intent_parser.rs
pub struct IntentParser {
    patterns: Vec<(Regex, IntentType)>,
}

impl IntentParser {
    pub fn parse(&self, user_input: &str) -> Intent {
        // Simples regex-based em M1
        // Em M2+ integrar NLTK ou modelo real
        Intent {
            user_message: user_input.to_string(),
            intent_type: self.detect_intent_type(user_input),
            entities: self.extract_entities(user_input),
            confidence: 0.95,
            timestamp: SystemTime::now(),
        }
    }
}

// beatrice-ui/src/cli.rs
#[tokio::main]
async fn main() {
    let bus = BusClient::connect("127.0.0.1:7447").await;
    
    loop {
        let input = read_line();
        let intent = IntentParser::parse(&input);
        
        // Enviar Intent via Bus
        bus.publish("intent/raw", &intent).await;
        
        // Ouvir resposta
        let response = bus.wait_for_topic("task/result", intent.id).await;
        println!("{}", response);
    }
}
```

**DoD:**
- [ ] CLI inicia sem crash
- [ ] Aceita input do utilizador
- [ ] Intent é validado antes de enviar
- [ ] Resposta é recebida e exibida
- [ ] End-to-end latência < 500ms

---

### T1.3: Adaptador Gemini (🟡 Alta)
**Bloqueado por:** T1.0, T1.1 (Bus + Mimi)  
**Bloqueia:** T1.4  

**Descrição:**
- Implementar trait `AIAdapter` (interface)
- Implementar `GeminiAdapter` específico
- Integração com Gemini API (autenticação + rate limiting)
- Retry logic com exponential backoff
- Cost estimation para routing

**Dependências Técnicas:**
- `reqwest` para HTTP calls
- `tokio` para async
- Gemini API key (via env var)
- `futures` para streaming (opcional em M1)

**Artefatos:**
- `mimi-core/src/adapters/trait.rs` — AIAdapter trait
- `mimi-core/src/adapters/gemini.rs` — GeminiAdapter impl
- `.env.example` — Configuração de exemplo

**Estrutura do Código (Rust):**
```rust
// mimi-core/src/adapters/trait.rs
#[async_trait]
pub trait AIAdapter: Send + Sync {
    async fn generate(&self, prompt: &str) -> Result<String, AdapterError>;
    async fn stream(&self, prompt: &str) -> Result<BoxStream<String>, AdapterError>;
    fn get_cost_estimate(&self) -> f64;
    fn name(&self) -> &str;
}

// mimi-core/src/adapters/gemini.rs
pub struct GeminiAdapter {
    api_key: String,
    client: reqwest::Client,
    model: String,
}

#[async_trait]
impl AIAdapter for GeminiAdapter {
    async fn generate(&self, prompt: &str) -> Result<String, AdapterError> {
        let response = self.client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent")
            .bearer_auth(&self.api_key)
            .json(&json!({
                "contents": [{
                    "parts": [{"text": prompt}]
                }]
            }))
            .send()
            .await?;
        
        let data = response.json::<GeminiResponse>().await?;
        Ok(data.candidates[0].content.parts[0].text.clone())
    }
}
```

**DoD:**
- [ ] Trait compila sem erros
- [ ] GeminiAdapter consegue autenticar
- [ ] Gera resposta válida para prompt simples
- [ ] Rate limiting funciona (max 100 req/min)
- [ ] Retry logic testa com 3 falhas simuladas

---

### T1.4: Fluxo End-to-End (🟡 Alta)
**Bloqueado por:** T1.2, T1.3 (Beatrice + Gemini)  
**Bloqueia:** Nada (M1 complete)  

**Descrição:**
- Integrar todos os componentes T1.0-T1.3
- Teste: Utilizador digita intent → resposta gerada e exibida
- Performance: < 1 segundo latência total
- Logging: rastrear cada step no fluxo

**Dependências Técnicas:**
- Todos os anteriores
- Docker Compose (Bus + Mimi + Beatrice running)
- Integration test framework

**Artefatos:**
- `tests/e2e_flow.rs` — Test end-to-end
- `docker-compose.test.yml` — Setup para testes
- `.github/workflows/e2e-tests.yml` — CI/CD pipeline

**Fluxo Esperado:**
```
1. User: "Qual é a capital de Portugal?"
2. Beatrice CLI: Envia Intent{user_message: "...", intent_type: Query, ...}
3. Mimi Core: Recebe, valida, roteia para GeminiAdapter
4. GeminiAdapter: Chama Gemini API
5. Mimi Core: Recebe resposta, publica em `task/result`
6. Beatrice CLI: Mostra resposta ao user
7. Total latência: < 1s
```

**DoD:**
- [ ] End-to-end flow executa sem erro
- [ ] Latência total < 1 segundo (medido)
- [ ] Logging mostra cada passo
- [ ] Teste passa 10x consecutivas (estabilidade)

---

## Requisitos Não-Funcionais Aplicáveis

| RNF | Alvo | Status |
|-----|------|--------|
| **RNF-1** (Performance) | Latência Bus < 1ms | ✅ Testado em T1.0 |
| **RNF-1** (Performance) | Latência Gemini < 500ms | ✅ Aceitável (cloud) |
| **RNF-2** (Segurança) | API key não em hardcode | ✅ Env vars apenas |
| **RNF-5** (Manutenibilidade) | Tests ≥ 80% coverage | ⏳ Target em M1 |
| **RNF-5** (Manutenibilidade) | 0 clippy warnings | ✅ Enforced CI |
| **RNF-6** (Compatibilidade) | Rust 1.70+, Linux/macOS/Windows | ✅ Verificado |

---

## Timeline

| Semana | Tarefa | Deliverable |
|--------|--------|-------------|
| 1-2 | T1.0 (Bus) | Bus rodando, latência < 1ms |
| 2-3 | T1.1 (Mimi Core) | State machine + router |
| 3-4 | T1.2 (Beatrice CLI) | CLI aceita input, envia Intent |
| 4-5 | T1.3 (Gemini Adapter) | GeminiAdapter gera resposta |
| 5-6 | T1.4 (E2E) + Buffer | Flow completo, testes passam |
| 6-8 | Documentação + Buffer | README, API docs, troubleshooting |

---

## Critérios de Aceitação Finais (M1 DoD)

✅ **Milestone 1 Completo quando:**

- [ ] Message Bus funcional (Zenoh ou NATS rodando)
- [ ] Mimi Core consegue receber e rotear mensagens (100+ msgs/sec)
- [ ] Adaptador Gemini integrado e autenticado
- [ ] Beatrice CLI envia Intent via Bus
- [ ] Fluxo end-to-end: `User → Beatrice → Mimi → Gemini → Bus → Beatrice` (< 1s)
- [ ] Testes unitários ≥ 80% coverage
- [ ] 0 clippy warnings
- [ ] 0 compiler errors
- [ ] Documentação: README.md + API docs
- [ ] CI/CD pipeline funcional (GitHub Actions)

---

## Bloqueadores Conhecidos

| Bloqueador | Risco | Mitigação |
|-----------|-------|-----------|
| **Zenoh vs NATS decision** | Pode atrasar T1.0 | Decidir em semana 1 (benchmarks) |
| **Gemini API key access** | Depende de aprovação externa | Preparar pedido logo |
| **Rust/C++ interop** | Não aplicável em M1 (apenas Rust) | Adiar para M2 |
| **Neo4j setup** | Não aplicável em M1 | Adiar para M2 |

---

## Notas

- **M1 é 100% Rust** — Sem C++ nesta fase (Message Bus é puro Rust)
- **Neo4j não é usado em M1** — Pandora chega em M2
- **LLM é apenas Gemini** — Ollama adapter chega em M2
- **Sem Docker em T1** — Ryzu chega em M3
- **Sem Skills** — Echidna chega em M4

---

## Referências Cruzadas

- Volta a: [`REQUIREMENTS.md#RF-7`](../REQUIREMENTS.md#rf-7-message-bus)
- Volta a: [`PROJECT.md#Módulos`](../PROJECT.md#tabela-de-módulos)
- Próximo: [`milestones/M2-PANDORA.md`](M2-PANDORA.md)
