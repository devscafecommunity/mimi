# MiMi — Project Master Document

> **Repositório:** https://github.com/devscafecommunity/mimi  
> **Licença:** MIT  
> **Stack:** Rust · C++ · Neo4j · Docker · Zenoh · FlatBuffers · Rhai · WASM  
> **Status:** 🟡 Pre-Development — Fase 1 não iniciada

---

## Índice de Documentação

| Documento | Descrição |
|-----------|-----------|
| [`PROJECT.md`](PROJECT.md) | Este ficheiro — visão geral, glossário, módulos |
| [`REQUIREMENTS.md`](REQUIREMENTS.md) | Todos os Requisitos Funcionais (RF) e Não-Funcionais (RNF) |
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Arquitetura técnica, fluxos, protocolos |
| [`DEPENDENCY_GRAPH.md`](DEPENDENCY_GRAPH.md) | Mapa de dependências entre tarefas |
| [`milestones/M1-FOUNDATION.md`](milestones/M1-FOUNDATION.md) | Milestone 1 — Espinha Dorsal |
| [`milestones/M2-PANDORA.md`](milestones/M2-PANDORA.md) | Milestone 2 — Palácio da Memória |
| [`milestones/M3-SECURITY.md`](milestones/M3-SECURITY.md) | Milestone 3 — Segurança e Supervisão |
| [`milestones/M4-ECHIDNA.md`](milestones/M4-ECHIDNA.md) | Milestone 4 — Evolução Autónoma |
| [`modules/MIMI-COMMANDER.md`](modules/MIMI-COMMANDER.md) | Módulo Mimi — Orquestrador |
| [`modules/BEATRICE.md`](modules/BEATRICE.md) | Módulo Beatrice — Interface NLP |
| [`modules/PANDORA.md`](modules/PANDORA.md) | Módulo Pandora — Memória em Grafos |
| [`modules/ECHIDNA.md`](modules/ECHIDNA.md) | Módulo Echidna — Skills Planner |
| [`modules/RYZU.md`](modules/RYZU.md) | Módulo Ryzu — Processadores Modulares |
| [`modules/ODLAGUNA.md`](modules/ODLAGUNA.md) | Módulo Odlaguna — Moderador/Watchdog |
| [`specs/BUS-PROTOCOL.md`](specs/BUS-PROTOCOL.md) | Protocolo do Message Bus |
| [`specs/HEATMAP-ALGORITHM.md`](specs/HEATMAP-ALGORITHM.md) | Algoritmo de Heatmap da Pandora |
| [`specs/SKILL-LIFECYCLE.md`](specs/SKILL-LIFECYCLE.md) | Ciclo de vida de uma Skill |

---

## Visão do Produto

**MiMi** (*Multimodal Instruction Master Interface* / *Modular Integrated Memory Instance*) é um **sistema operacional cognitivo** de alto desempenho. Diferencia-se de agentes convencionais por:

1. **Contexto explosivo** — Memória em grafos térmicos via Neo4j, sem flooding de tokens
2. **Evolução autónoma** — Cria as suas próprias ferramentas (Skills) sob demanda
3. **Multimodal modular** — Delega para diferentes modelos e agentes via adaptadores
4. **Segurança por design** — Execução isolada em Docker/WASM com supervisão ativa
5. **Baixa latência** — Core em Rust/C++ com comunicação via Message Bus (Zenoh + FlatBuffers)

---

## Módulos do Sistema

```
┌─────────────────────────────────────────────────────────┐
│                        USUÁRIO                          │
└────────────────────────┬────────────────────────────────┘
                         │ Linguagem Natural
                    ┌────▼────┐
                    │BEATRICE │  NLP Interface (Rust/C++)
                    └────┬────┘  Converte intenção → Intent estruturado
                         │
                    ┌────▼────┐
                    │  MIMI   │  Agentic Commander (Rust)
                    │ CORE    │  Orquestrador central — roteamento e estado
                    └─┬──┬──┬─┘
          ┌───────────┘  │  └──────────────┐
     ┌────▼────┐    ┌────▼────┐      ┌─────▼────┐
     │ PANDORA │    │ODLAGUNA │      │  ECHIDNA  │
     │(C++)    │    │(Rust)   │      │  (Rust)   │
     │Memória  │    │Watchdog │      │Skills Lab │
     │Neo4j    │    │Moderador│      │Rhai + WASM│
     └────┬────┘    └────┬────┘      └─────┬─────┘
          │              │                  │
          └──────────────┴──────────┬───────┘
                                    │
                              ┌─────▼─────┐
                              │   RYZU    │
                              │ (C++/ASM) │
                              │ Workers   │
                              │  Docker   │
                              └───────────┘
                         ┌────────────────────┐
                         │    MESSAGE BUS      │
                         │  Zenoh + FlatBuffers│
                         └────────────────────┘
```

### Tabela de Módulos

| Módulo | Linguagem | Responsabilidade | Doc |
|--------|-----------|-----------------|-----|
| **Mimi** | Rust | Orquestração async, estado, priorização | [`modules/MIMI-COMMANDER.md`](modules/MIMI-COMMANDER.md) |
| **Beatrice** | Rust/C++ | Parsing NLP, tradução de intenção, I/O | [`modules/BEATRICE.md`](modules/BEATRICE.md) |
| **Pandora** | C++ | Grafos Neo4j, Heatmap, contexto ST/LT | [`modules/PANDORA.md`](modules/PANDORA.md) |
| **Echidna** | Rust | Criação de Skills (Rhai + WASM) | [`modules/ECHIDNA.md`](modules/ECHIDNA.md) |
| **Ryzu** | C++/ASM | Execução isolada Docker, workers | [`modules/RYZU.md`](modules/RYZU.md) |
| **Odlaguna** | Rust | QA, Watchdog, Circuit Breaker, auditoria | [`modules/ODLAGUNA.md`](modules/ODLAGUNA.md) |

---

## Glossário Técnico

| Termo | Definição |
|-------|-----------|
| **Message Bus** | Backbone de comunicação assíncrona entre módulos (Zenoh/NATS) |
| **Heatmap** | Sistema de peso térmico nos nós do grafo Neo4j que determina relevância contextual |
| **Skill** | Ferramenta criada dinamicamente pela Echidna (`.rhai` ou `.wasm`) |
| **Intent** | Estrutura de dados gerada pela Beatrice a partir de linguagem natural |
| **ContextNode** | Nó no grafo Neo4j representando um fragmento de memória/conhecimento |
| **Bolt Driver** | Protocolo de conexão C++ ↔ Neo4j |
| **Fueling** | Limite de instruções CPU aplicado a execuções WASM pela Odlaguna |
| **Lease / Deadline** | Contrato de tempo atribuído a cada tarefa para evitar hanging |
| **Circuit Breaker** | Mecanismo que bloqueia skills com falhas repetidas |
| **Checkpoint** | Snapshot do estado do grafo criado pela Pandora após cada ação crítica |
| **FlatBuffers** | Formato de serialização zero-copy para comunicação interna |
| **Rhai** | Motor de scripting embutido nativo em Rust (scripts simples/rápidos) |
| **WASM** | WebAssembly — binários sandboxed para skills complexas |
| **Wasmtime** | Runtime WASM usado pelo Ryzu para execução de skills |
| **LRU Cache** | Cache L1 mantido pela Pandora para contexto imediato (em RAM) |
| **BFS** | Breadth-First Search — estratégia de busca no grafo pela Pandora |
| **Audit Trail** | Log histórico de todas as mensagens do Bus, gerido pela Pandora/Odlaguna |

---

## Adaptadores de IA

O sistema não corre modelos localmente. Usa um **Model Gateway** com adaptadores plugáveis:

```rust
pub trait AIAdapter {
    fn generate(&self, prompt: ContextNode) -> Result<String, Error>;
    fn stream(&self, prompt: ContextNode) -> BoxStream<String>;
    fn get_cost_estimate(&self) -> f64;  // Para roteamento pela Odlaguna
}
```

| Adaptador | Tipo | Estado |
|-----------|------|--------|
| Gemini API | Cloud | Planeado — M1 |
| Ollama | Local via HTTP | Planeado — M1 |
| Custom (`.so`/`.dll`) | Dynamic Library | Planeado — M2+ |

---

## Referências Externas

- [Neo4j Bolt Protocol](https://neo4j.com/docs/bolt/current/)
- [Zenoh (Rust)](https://zenoh.io/)
- [FlatBuffers](https://flatbuffers.dev/)
- [Rhai Scripting](https://rhai.rs/)
- [Wasmtime](https://wasmtime.dev/)
- [Tokio Async Runtime](https://tokio.rs/)
- [gVisor / Kata Containers](https://gvisor.dev/)
