# M2: Palácio da Memória (Pandora)

> **Milestone Objetivo:** Implementar memória em grafos com contexto térmico  
> **Status:** 🟡 Bloqueado por M1  
> **Duração Estimada:** 8-10 semanas  
> **Dependências:** M1 (Message Bus + Mimi Core)  

---

## Visão Geral

Milestone 2 constrói o "palácio da memória" do MiMi — um grafo Neo4j inteligente com algoritmo de Heatmap que mantém contexto quente e descarta ruído frio automaticamente.

Diferentemente de RAG tradicional (que recupera tudo), Pandora usa **decaimento térmico** para:
- Manter dados acessados frequentemente prontos
- Descartar dados irrelevantes automaticamente
- Evitar "context flooding" que envenena LLMs
- Economizar tokens e latência

**Optimization Layer (M2 + M2.5):** Integra-se com o **Gating System** (roteador hierárquico 3-tier) que garante:
- Tier 1 (Reflex): Liliana cache para pedidos triviais (~0 tokens)
- Tier 2 (Automated): Beatrice + Skills para tarefas estruturadas (~50-100 tokens)
- Tier 3 (Cognitive): Full pipeline para raciocínio complexo (~500+ tokens)
Resultado: 70-80% economia em tokens vs. sem gating.

---

## Requisitos Funcionais (RF) Ativados

| RF | Descrição | Prioridade |
|----|-----------|-----------|
| **RF-3** | Memória em Grafos (Pandora) | 🔴 Bloqueante |
| **RF-1.3** | Mimi persiste estado em Pandora | 🟡 Alta |

**Outros requisitos (desbloqueados em M2+):**
- RF-4 (Echidna) — bloqueado até Pandora estar pronto
- RF-6.6 (Audit Trail) — depende de Pandora para storage

---

## Tarefas por Hierarquia

### T2.0: Neo4j Setup & Bolt Driver C++ (🔴 CRÍTICO)
**Bloqueado por:** M1  
**Bloqueia:** T2.1, T2.2, T2.3  

**Descrição:**
- Setup Neo4j em container Docker
- Implementar Bolt C++ driver (conexão + autenticação)
- Definir schema de nós (ContextNode, Entity, Task, Skill, Memory)
- Criar índices de performance
- Testes de conexão e throughput

**Dependências Técnicas:**
- `neo4j-driver-cpp` (oficial ou alternativa)
- Neo4j 4.4+ (container Docker)
- `docker-compose.yml` atualizado
- Cypher query builder (Rust ou C++)

**Artefatos:**
- `pandora-memory/drivers/neo4j_bolt_client.cpp` — Bolt impl
- `pandora-memory/schema/neo4j_schema.cypher` — DDL
- `docker-compose.yml` — Neo4j container
- `pandora-memory/tests/neo4j_integration_tests.cpp` — Testes

**Estrutura da Schema (Cypher):**
```cypher
// ContextNode — fragmento de conhecimento/memória
CREATE (cn:ContextNode {
  id: STRING (UUID),
  content: STRING,
  embedding: VECTOR (1536D, float32),
  temperature: FLOAT (0.0 a 1.0),
  created_at: DATETIME,
  last_accessed: DATETIME,
  access_count: INTEGER,
  domain: STRING (e.g., "code", "knowledge", "task")
})

// Entity — entidade extraída (pessoa, conceito, etc)
CREATE (e:Entity {
  id: STRING,
  name: STRING,
  type: STRING ("person", "concept", "tool"),
  mentions: [STRING] (lista de contextos que mencionam)
})

// Skill — habilidade criada
CREATE (s:Skill {
  id: STRING,
  name: STRING,
  language: STRING ("rhai", "wasm"),
  created_at: DATETIME,
  execution_count: INTEGER,
  success_rate: FLOAT,
  last_executed: DATETIME
})

// Task — tarefa executada
CREATE (t:Task {
  id: STRING,
  description: STRING,
  status: STRING ("pending", "running", "completed", "failed"),
  created_at: DATETIME,
  completed_at: DATETIME,
  result: STRING,
  owner_skill: STRING (referência a Skill)
})

// Memory — snapshot de estado
CREATE (m:Memory {
  id: STRING,
  snapshot_type: STRING ("checkpoint", "audit_trail"),
  timestamp: DATETIME,
  data_size: INTEGER (bytes),
  compression: STRING ("none", "gzip", "snappy")
})

// Relationships
MATCH (cn1:ContextNode), (cn2:ContextNode)
CREATE (cn1)-[:REFERENCES]->(cn2)  // cn1 referencia cn2

MATCH (cn:ContextNode), (e:Entity)
CREATE (cn)-[:CONTAINS_ENTITY]->(e)  // cn menciona entidade

MATCH (cn:ContextNode), (s:Skill)
CREATE (cn)-[:CREATED_BY]->(s)  // cn foi criada por skill

MATCH (t:Task), (s:Skill)
CREATE (t)-[:EXECUTED_BY]->(s)  // task foi executada por skill

// Índices para performance
CREATE INDEX ON :ContextNode(id);
CREATE INDEX ON :ContextNode(temperature);
CREATE INDEX ON :ContextNode(last_accessed);
CREATE INDEX ON :Skill(name);
CREATE INDEX ON :Task(status);
```

**DoD:**
- [ ] Neo4j container inicia sem erros
- [ ] Bolt driver conecta com sucesso
- [ ] Schema criado (5+ tipos de nó)
- [ ] Índices criados
- [ ] Connection pool funciona (100+ concurrent)
- [ ] < 10ms latência per query

---

### T2.1: Algoritmo de Heatmap (🔴 CRÍTICO)
**Bloqueado por:** T2.0 (Neo4j)  
**Bloqueia:** T2.3, T2.4  

**Descrição:**
- Implementar fórmula de decaimento exponencial de temperatura
- Atualizar temperatura ao acessar nó
- Query Cypher para BFS limitado por calor
- Determinar threshold de descarte
- Benchmarks de performance

**Dependências Técnicas:**
- C++ math library (cmath)
- Neo4j Cypher language
- Algoritmos BFS em Rust (para Mimi invocar)

**Artefatos:**
- `pandora-memory/src/heatmap.cpp` — Implementação
- `pandora-memory/src/heatmap.h` — Header
- `specs/HEATMAP-ALGORITHM.md` — Especificação completa
- `pandora-memory/tests/heatmap_tests.cpp` — Unit tests

**Fórmula de Decaimento (Térmica):**

```
T(t) = T0 * e^(-lambda * (now - last_accessed))

Onde:
- T(t) = temperatura no tempo t
- T0 = temperatura inicial (1.0 quando criado/acessado)
- lambda = constante de decaimento (0.01 = meia-vida ~70s)
- (now - last_accessed) = segundos desde último acesso

Exemplo:
- Nó criado → T = 1.0
- 5 minutos depois → T ≈ 0.74
- 1 hora depois → T ≈ 0.54
- 10 horas depois → T ≈ 0.002 (descartável)

Threshold de BFS:
- Incluir nó na busca se T > 0.1
- Ordenar resultados por temperatura DESC
- Limitar a 500 nós máximo por query
```

**Estrutura do Código (C++):**
```cpp
// pandora-memory/src/heatmap.cpp
#include <cmath>
#include <chrono>

class HeatmapEngine {
private:
    constexpr static const float DECAY_LAMBDA = 0.01f;
    constexpr static const float DISCARD_THRESHOLD = 0.1f;
    constexpr static const int MAX_NODES_PER_QUERY = 500;

public:
    float compute_temperature(
        float initial_temp,
        int64_t last_accessed_ts,
        int64_t now_ts
    ) const {
        float age_seconds = (now_ts - last_accessed_ts) / 1000.0f;
        return initial_temp * std::exp(-DECAY_LAMBDA * age_seconds);
    }

    void update_on_access(const std::string& node_id) {
        // Update last_accessed = now em Neo4j
        cypher_query(
            "MATCH (n) WHERE n.id = $id "
            "SET n.last_accessed = datetime() "
            "SET n.temperature = 1.0 "
            "SET n.access_count = n.access_count + 1",
            {{"id", node_id}}
        );
    }
};
```

**Cypher Query para BFS Limitado:**
```cypher
// Retornar contexto relevante (quente) para uma query
MATCH (start:ContextNode {id: $context_id})
CALL apoc.path.expandConfig(
  start,
  {
    relationshipFilter: "REFERENCES|CONTAINS_ENTITY|CREATED_BY",
    minLevel: 1,
    maxLevel: 3,
    limit: 500
  }
) YIELD path
WITH nodes(path) as nodes
UNWIND nodes as n
WHERE datetime(n.last_accessed).epochSeconds - datetime.transaction().epochSeconds > -3600 * 10
ORDER BY n.temperature DESC
LIMIT 500
RETURN n {
  .id, .content, .temperature, .created_at, .access_count
}
```

**DoD:**
- [ ] Fórmula de decaimento compila e executa
- [ ] Temperatura reduz exponencialmente (verificado)
- [ ] Cypher query retorna < 500 nós
- [ ] Threshold funciona (T < 0.1 descartado)
- [ ] Benchmarks: < 50ms per query
- [ ] Unit tests cobrem 5+ cenários

---

### T2.2: LRU Cache L1 em RAM (🟡 Alta)
**Bloqueado por:** T2.0 (Neo4j)  
**Bloqueia:** T2.4  

**Descrição:**
- Implementar cache L1 in-memory (Least Recently Used)
- Cache armazena ~1000 ContextNodes mais quentes
- Invalidação automática ao timeout/update
- Bypass Neo4j para hits frequentes (< 1ms)
- Estatísticas de hit rate

**Dependências Técnicas:**
- `lru` crate (Rust)
- `tokio::sync::RwLock` para thread safety
- Metrics collection

**Artefatos:**
- `mimi-commander/src/cache/lru_cache.rs` — Implementação
- `mimi-commander/tests/cache_tests.rs` — Testes

**Estrutura (Rust):**
```rust
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct L1Cache {
    cache: Arc<RwLock<LruCache<String, ContextNode>>>,
    hit_count: Arc<AtomicU64>,
    miss_count: Arc<AtomicU64>,
}

impl L1Cache {
    pub async fn get(&self, key: &str) -> Option<ContextNode> {
        let mut cache = self.cache.write().await;
        
        if let Some(node) = cache.get_mut(key) {
            self.hit_count.fetch_add(1, Ordering::Relaxed);
            Some(node.clone())
        } else {
            self.miss_count.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    pub async fn put(&self, key: String, node: ContextNode) {
        let mut cache = self.cache.write().await;
        cache.put(key, node);
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.hit_count.load(Ordering::Relaxed) as f64;
        let misses = self.miss_count.load(Ordering::Relaxed) as f64;
        hits / (hits + misses)
    }
}
```

**DoD:**
- [ ] Cache inicializa com 1000 entry cap
- [ ] Get/Put operações < 1ms
- [ ] LRU eviction funciona
- [ ] Hit rate logs em telemetria
- [ ] Thread-safe (RwLock)

---

### T2.3: Integração Pandora ↔ Mimi (🟡 Alta)
**Bloqueado por:** T2.0, T2.1 (Neo4j + Heatmap)  
**Bloqueia:** T2.4  

**Descrição:**
- Mimi envia queries para Pandora via Message Bus
- Pandora retorna contexto relevante (top N nós quentes)
- Mimi injeta contexto em prompt para LLM
- Ciclo: Intent → Query Pandora → Augmented prompt → LLM

**Dependências Técnicas:**
- FlatBuffers message definitions (de M1)
- Pandora FFI (se necessário)
- Rust async/await

**Artefatos:**
- `mimi-commander/src/memory_client.rs` — Client para Pandora
- `proto/pandora_query.fbs` — Message definition
- `specs/INTEGRATION-MIMI-PANDORA.md` — Protocolo

**Fluxo:**
```
1. Beatrice → Mimi: Intent{user_message: "Qual é meu nome?"}
2. Mimi → Pandora: QueryRequest{query: "name", limit: 10}
3. Pandora → Mimi: QueryResponse{nodes: [ContextNode], total_temperature: 0.87}
4. Mimi → Gemini: Prompt com contexto injetado
5. Gemini → Mimi: Resposta aumentada com contexto
6. Mimi → Beatrice: Resultado final
```

**DoD:**
- [ ] Message definitions compilam
- [ ] Query latência < 100ms
- [ ] Contexto é injetado corretamente em prompt
- [ ] E2E flow funciona

---

### T2.4: Persistência de Estado Mimi em Pandora (🟡 Alta)
**Bloqueado por:** T2.3 (Integração)  
**Bloqueia:** Nada (M2 complete)  

**Descrição:**
- Mimi grava seu estado em Pandora periodicamente
- Ao iniciar, Mimi lê estado anterior
- Recuperação de falha: Mimi restaura contexto
- Não há perda de estado (durabilidade)

**Dependências Técnicas:**
- Neo4j transactions
- Serialização JSON/FlatBuffers

**Artefatos:**
- `pandora-memory/src/checkpoint.cpp` — Checkpoint logic
- `mimi-commander/src/persistence.rs` — State save/restore

**DoD:**
- [ ] Mimi salva estado a cada 5 minutos
- [ ] Ao reiniciar, estado é restaurado
- [ ] Zero perda de dados
- [ ] Latência < 100ms per checkpoint

---

## Requisitos Não-Funcionais Aplicáveis

| RNF | Alvo | Status |
|-----|------|--------|
| **RNF-1** (Performance) | Query Neo4j < 50ms | ✅ Target em T2.0 |
| **RNF-4** (Escalabilidade) | 1M+ nós no grafo | ✅ Neo4j suporta |
| **RNF-3** (Resiliência) | Zero data loss (durabilidade) | ✅ Neo4j + transactions |

---

## Timeline

| Semana | Tarefa | Deliverable |
|--------|--------|-------------|
| 1-2 | T2.0 (Neo4j + Bolt) | Neo4j container + driver C++ |
| 2-3 | T2.1 (Heatmap) | Algoritmo + Cypher queries |
| 3-4 | T2.2 (LRU Cache) | L1 cache em RAM |
| 4-5 | T2.3 (Integração) | Mimi ↔ Pandora funciona |
| 5-6 | T2.4 (Persistência) | State save/restore |
| 6-8 | Testes + Buffer | Integration tests, benchmarks |

---

## Critérios de Aceitação Finais (M2 DoD)

✅ **Milestone 2 Completo quando:**

- [ ] Neo4j rodando em container
- [ ] Pandora conectada via Bolt (C++)
- [ ] Heatmap implementado (fórmula + query Cypher)
- [ ] Mimi consulta Pandora antes de responder
- [ ] LRU Cache L1 funcionando (hit rate > 70%)
- [ ] Queries Neo4j < 50ms em média
- [ ] Tests de integração passam
- [ ] State persistence funciona
- [ ] Documentação: Schema + Cypher queries + benchmarks

---

## Bloqueadores Conhecidos

| Bloqueador | Risco | Mitigação |
|-----------|-------|-----------|
| **Neo4j Bolt C++ driver** | Pode não existir | Usar `neo4j-driver-cpp` oficial ou bindings |
| **Cypher apoc.path.expand** | Pode ser lento | Usar índices, limitar depth |
| **Temperature decay tuning** | Pode descartar contexto cedo | Fazer benchmarks e ajustar lambda |

---

## Notas

- **M2 introduz Neo4j** — Primeira integração com banco de dados
- **Sem WASM em M2** — Echidna chega em M4
- **Sem Odlaguna em M2** — Watchdog chega em M3
- **Heatmap é core** — Diferencia Pandora de RAG tradicional

---

## Referências Cruzadas

- Volta a: [`REQUIREMENTS.md#RF-3`](../REQUIREMENTS.md#rf-3-memória-em-grafos-pandora)
- Anterior: [`milestones/M1-FOUNDATION.md`](M1-FOUNDATION.md)
- Próximo: [`milestones/M3-SECURITY.md`](M3-SECURITY.md)
