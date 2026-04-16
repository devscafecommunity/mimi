# M4: Evolução Autónoma (Echidna)

> **Milestone Objetivo:** Implementar geração autónoma de skills (Rhai + WASM)  
> **Status:** 🟡 Bloqueado por M1+M2+M3  
> **Duração Estimada:** 10-12 semanas  
> **Dependências:** M1 (Bus) + M2 (Pandora) + M3 (Odlaguna)  

---

## Visão Geral

Milestone 4 constrói o motor de evolução do MiMi — **Echidna** — a capacidade de:

1. **Detectar padrões** — Identificar tarefas repetidas na Pandora
2. **Gerar scripts** — Criar código Rhai para automação simples
3. **Compilar binários** — Gerar WASM para ferramentas complexas
4. **Validar código** — Submeter para Odlaguna validar
5. **Registar skills** — Armazenar na Pandora como ContextNode
6. **Executar sob supervisão** — Ryzu executa skill ao invés de ação manual

Este é o milestone que diferencia MiMi de agentes convencionais — **ela evolui as suas próprias capacidades**.

---

## Requisitos Funcionais (RF) Ativados

| RF | Descrição | Prioridade |
|----|-----------|-----------|
| **RF-4** | Criação Dinâmica de Skills (Echidna) | 🔴 Bloqueante |

---

## Tarefas por Hierarquia

### T4.0: Pattern Detection Engine (🔴 CRÍTICO)
**Bloqueado por:** M2 (Pandora)  
**Bloqueia:** T4.1, T4.2, T4.3  

**Descrição:**
- Análise de histórico de tarefas na Pandora
- Detectar padrões repetidos (sliding window)
- Agrupar tarefas similares
- Score de "automability" (quão automatizável é)
- Gerar candidatos de skill para Echidna

**Dependências Técnicas:**
- Neo4j queries complexas (Cypher)
- Clustering algoritmos (K-means, DBSCAN)
- Similarity metrics (Levenshtein, cosine)

**Artefatos:**
- `echidna-lab/src/pattern_detector.rs` — Detection engine
- `echidna-lab/tests/pattern_detection_tests.rs`

**Estrutura do Código (Rust):**
```rust
// echidna-lab/src/pattern_detector.rs
use std::collections::HashMap;

pub struct PatternDetector {
    pandora_client: PandoraClient,
    min_repetitions: usize,
    similarity_threshold: f64,
}

pub struct SkillCandidate {
    pub id: String,
    pub name: String,
    pub pattern_description: String,
    pub automability_score: f64,  // 0.0 a 1.0
    pub repetition_count: usize,
    pub estimated_time_saved: f64, // seconds
    pub suggested_language: SkillLanguage, // Rhai ou WASM
}

#[derive(Debug, Clone)]
pub enum SkillLanguage {
    Rhai,  // < 100ms, simple logic
    Wasm,  // > 100ms, complex logic
}

impl PatternDetector {
    pub async fn detect_candidates(&self) -> Result<Vec<SkillCandidate>, Error> {
        // 1. Query Pandora para últimas 1000 tarefas
        let tasks = self.pandora_client.get_recent_tasks(1000).await?;

        // 2. Group similares por fingerprint
        let groups = self.group_similar_tasks(&tasks);

        // 3. Filter grupos com N+ repetições
        let candidates: Vec<_> = groups
            .into_iter()
            .filter(|(_, group)| group.len() >= self.min_repetitions)
            .map(|(fingerprint, group)| {
                let automability = self.calculate_automability(&group);
                
                SkillCandidate {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("skill_auto_{}", &fingerprint[..8]),
                    pattern_description: self.describe_pattern(&group),
                    automability_score: automability,
                    repetition_count: group.len(),
                    estimated_time_saved: self.estimate_savings(&group),
                    suggested_language: if automability > 0.8 {
                        SkillLanguage::Rhai
                    } else {
                        SkillLanguage::Wasm
                    },
                }
            })
            .collect();

        Ok(candidates)
    }

    fn group_similar_tasks(
        &self,
        tasks: &[Task],
    ) -> HashMap<String, Vec<Task>> {
        let mut groups: HashMap<String, Vec<Task>> = HashMap::new();

        for task in tasks {
            let fingerprint = self.compute_fingerprint(task);
            groups.entry(fingerprint).or_insert_with(Vec::new).push(task.clone());
        }

        groups
    }

    fn compute_fingerprint(&self, task: &Task) -> String {
        // Remover parâmetros específicos, manter estrutura
        // E.g., "convert_jpg_to_png" vs "convert_mp4_to_mkv" → mesma categoria
        format!("{:?}", task.intent_type)  // Simplificado
    }

    fn calculate_automability(&self, tasks: &[Task]) -> f64 {
        // Score baseado em: determinismo, I/O, complexidade
        let determinism_score = self.score_determinism(tasks);
        let io_cost = self.score_io_cost(tasks);
        let complexity = self.score_complexity(tasks);

        (determinism_score * 0.5) + (io_cost * 0.3) + ((1.0 - complexity) * 0.2)
    }

    fn score_determinism(&self, tasks: &[Task]) -> f64 {
        // Tasks que sempre produzem mesmo resultado → high score
        1.0
    }

    fn score_io_cost(&self, tasks: &[Task]) -> f64 {
        // I/O-bound tasks → high score (valem a pena automatizar)
        0.8
    }

    fn score_complexity(&self, tasks: &[Task]) -> f64 {
        // Simples tarefas → high score
        0.3
    }

    fn estimate_savings(&self, tasks: &[Task]) -> f64 {
        // Tempo economizado por automatizar este padrão
        tasks.len() as f64 * 10.0  // 10 segundos por tarefa repetida
    }
}
```

**Cypher Query para buscar tarefas:**
```cypher
// Últimas 1000 tarefas executadas
MATCH (t:Task)
WHERE t.status = "completed"
RETURN t ORDER BY t.completed_at DESC LIMIT 1000
```

**DoD:**
- [ ] Pattern detector compila
- [ ] Retorna candidatos com automability > 0.5
- [ ] Grupo similares funciona
- [ ] Fingerprinting é determinístico

---

### T4.1: Rhai Code Generator (🔴 CRÍTICO)
**Bloqueado por:** T4.0 (Pattern Detection)  
**Bloqueia:** T4.3  

**Descrição:**
- Gerar scripts Rhai a partir de padrão detectado
- Template-based code generation
- Variáveis parametrizáveis
- Integração com Mimi via Message Bus
- Tests com execução real

**Dependências Técnicas:**
- `askama` ou `tera` para templates
- `rhai` para parsing/validation
- Code formatting

**Artefatos:**
- `echidna-lab/src/rhai_generator.rs` — Geração
- `echidna-lab/templates/*.rhai` — Templates
- `echidna-lab/tests/rhai_gen_tests.rs`

**Estrutura (Rust):**
```rust
// echidna-lab/src/rhai_generator.rs
use askama::Template;

#[derive(Template)]
#[template(path = "simple_task_template.rhai")]
pub struct SimpleTaskTemplate {
    pub task_description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

pub struct RhaiGenerator;

impl RhaiGenerator {
    pub fn generate_from_candidate(
        candidate: &SkillCandidate,
        sample_tasks: &[Task],
    ) -> Result<String, GenError> {
        // Analisar estrutura comum das tasks
        let common_steps = Self::extract_common_steps(sample_tasks)?;
        
        let template = SimpleTaskTemplate {
            task_description: candidate.pattern_description.clone(),
            inputs: Self::extract_inputs(&common_steps),
            outputs: Self::extract_outputs(&common_steps),
        };

        let rhai_code = template.render()?;
        
        // Validar código gerado
        rhai::Engine::new().compile_expression(&rhai_code)?;
        
        Ok(rhai_code)
    }

    fn extract_common_steps(tasks: &[Task]) -> Result<Vec<Step>, GenError> {
        // AST differencing — encontrar estrutura comum
        // Simplified: assumir primeiro task é representativo
        let first = &tasks[0];
        let steps = Self::parse_task_steps(first)?;
        Ok(steps)
    }

    fn parse_task_steps(task: &Task) -> Result<Vec<Step>, GenError> {
        // Converter Task description → lista de passos
        // Simplificado
        Ok(vec![
            Step::Input("param1"),
            Step::Process("transform"),
            Step::Output("result"),
        ])
    }

    fn extract_inputs(steps: &[Step]) -> Vec<String> {
        steps.iter()
            .filter_map(|s| if let Step::Input(name) = s {
                Some(name.to_string())
            } else {
                None
            })
            .collect()
    }

    fn extract_outputs(steps: &[Step]) -> Vec<String> {
        steps.iter()
            .filter_map(|s| if let Step::Output(name) = s {
                Some(name.to_string())
            } else {
                None
            })
            .collect()
    }
}

#[derive(Debug)]
pub enum Step {
    Input(&'static str),
    Process(&'static str),
    Output(&'static str),
}
```

**Rhai Template:**
```rhai
// templates/simple_task_template.rhai
// Auto-generated by Echidna at {{ now() }}
// Pattern: {{ task_description }}

fn execute(params) {
    let mut result = #{};
    
    {% for input in inputs %}
    let {{ input }} = params["{{ input }}"];
    {% endfor %}
    
    // Core logic (pattern-specific)
    result["output"] = execute_core({{ inputs.join(", ") }});
    
    return result;
}

fn execute_core({{ inputs.join(", ") }}) {
    // TODO: Preencher com lógica extraída de padrão
    return "implementation pending";
}

// Main entry point
let params = engine_params();
let result = execute(params);
print(result);
```

**DoD:**
- [ ] Code generator compila
- [ ] Gera Rhai válido (sem syntax errors)
- [ ] Templates funcionam com dados reais
- [ ] Código gerado passa validation Odlaguna

---

### T4.2: WASM Compiler & Optimization (🟡 Alta)
**Bloqueado por:** T4.0 (Pattern Detection)  
**Bloqueia:** T4.3  

**Descrição:**
- Compilar skills complexas para WASM
- Otimização (dead code elimination, inlining)
- Verificação de imports (whitelist)
- Binary size tuning
- Performance benchmarks

**Dependências Técnicas:**
- `wasm-pack` ou `wasmtime-cli`
- Rust target `wasm32-unknown-unknown`
- LLVM optimization flags

**Artefatos:**
- `echidna-lab/wasm-templates/*.rs` — Rust templates para WASM
- `echidna-lab/src/wasm_compiler.rs` — Compiler logic
- `build.rs` — Build script com optimization

**Estrutura (Rust):**
```rust
// echidna-lab/src/wasm_compiler.rs
use std::process::Command;

pub struct WasmCompiler;

impl WasmCompiler {
    pub fn compile_skill(
        skill_source: &str,
        skill_name: &str,
        optimization_level: u8,  // 0-3
    ) -> Result<Vec<u8>, CompileError> {
        // 1. Write temporary Rust source
        let temp_rs = format!("/tmp/skill_{}.rs", skill_name);
        std::fs::write(&temp_rs, skill_source)?;

        // 2. Compile to WASM
        let optimization_flag = match optimization_level {
            0 => "",
            1 => "-C opt-level=1",
            2 => "-C opt-level=2",
            3 => "-C opt-level=3",
            _ => "-C opt-level=2",
        };

        let output = Command::new("rustc")
            .arg("--target=wasm32-unknown-unknown")
            .arg(optimization_flag)
            .arg("-C link-arg=-s")  // Strip symbols
            .arg(&temp_rs)
            .arg("-o")
            .arg(format!("/tmp/skill_{}.wasm", skill_name))
            .output()?;

        if !output.status.success() {
            return Err(CompileError::CompilationFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // 3. Read compiled WASM
        let wasm_bytes = std::fs::read(format!("/tmp/skill_{}.wasm", skill_name))?;

        // 4. Validate imports
        Self::validate_wasm_imports(&wasm_bytes)?;

        Ok(wasm_bytes)
    }

    fn validate_wasm_imports(wasm_bytes: &[u8]) -> Result<(), CompileError> {
        // Verificar que imports são apenas de "env" (whitelist)
        // Rejeitar if imports from libc, network, etc
        Ok(())
    }
}
```

**DoD:**
- [ ] WASM compiler compila sem erros
- [ ] Gera binário válido < 1MB
- [ ] Imports validados (whitelist)
- [ ] Performance: skill executa < 500ms

---

### T4.3: Skill Validation & Registration (🟡 Alta)
**Bloqueado por:** T4.1, T4.2 (Rhai + WASM)  
**Bloqueia:** T4.4  

**Descrição:**
- Submeter skill gerada para Odlaguna validar
- AST parsing + whitelist check
- Armazenar skill em Pandora como ContextNode
- Registar em cache de skills disponíveis
- Versioning (v1.0, v1.1, etc)

**Dependências Técnicas:**
- Odlaguna code validator (de M3)
- Neo4j para armazenar skills

**Artefatos:**
- `echidna-lab/src/skill_registry.rs` — Registry
- `pandora-memory/queries/skill_registration.cypher`

**Estrutura:**
```rust
// echidna-lab/src/skill_registry.rs
pub struct SkillRegistry {
    odlaguna_client: OdlagunaClient,
    pandora_client: PandoraClient,
}

pub struct Skill {
    pub id: String,
    pub name: String,
    pub version: String,
    pub language: SkillLanguage,
    pub code: String,
    pub binary: Option<Vec<u8>>, // Para WASM
    pub metadata: SkillMetadata,
    pub validation_result: ValidationResult,
}

pub struct SkillMetadata {
    pub created_by: String,  // "echidna"
    pub created_at: DateTime,
    pub repetitions_detected: usize,
    pub automability_score: f64,
    pub estimated_time_saved: f64,
}

impl SkillRegistry {
    pub async fn validate_and_register(
        &self,
        skill: &mut Skill,
    ) -> Result<(), Error> {
        // 1. Submit para Odlaguna validar
        let validation = self.odlaguna_client
            .validate_code(&skill.code, skill.language.clone())
            .await?;

        skill.validation_result = validation;

        // 2. Se passou validação, registar em Pandora
        if skill.validation_result.is_ok() {
            self.pandora_client.register_skill(skill).await?;
        }

        Ok(())
    }
}
```

**Cypher para registar skill:**
```cypher
// Registar nova skill em Pandora
CREATE (s:Skill {
  id: $skill_id,
  name: $name,
  version: $version,
  language: $language,
  created_by: "echidna",
  created_at: datetime(),
  source_code: $code,
  validation_status: "passed",
  execution_count: 0,
  success_rate: 1.0,
  automability_score: $automability
})

// Linkar a Skills já existentes se similar
WITH s
MATCH (existing:Skill)
WHERE
  existing.language = s.language AND
  similarity(existing.name, s.name) > 0.8
CREATE (s)-[:EVOLVED_FROM]->(existing)
```

**DoD:**
- [ ] Validation passa para skill válida
- [ ] Rejeita skill com operações proibidas
- [ ] Skill registada em Pandora
- [ ] Version tracking funciona

---

### T4.4: Execution & Caching (🟡 Alta)
**Bloqueado por:** T4.3 (Validation)  
**Bloqueia:** T4.5  

**Descrição:**
- Ao encontrar padrão repetido: usar skill em vez de ação manual
- Cache skills em Ryzu
- Invoke via Ryzu (Docker isolado)
- Cache resultado para próxima execução similar
- Performance: < 100ms para Rhai, < 500ms para WASM

**Dependências Técnicas:**
- Ryzu (de M3)
- Caching strategy (LRU)

**Artefatos:**
- `ryzu-runtime/src/skill_executor.rs` — Executor
- `ryzu-runtime/src/skill_cache.rs` — Cache

**Estrutura:**
```rust
// ryzu-runtime/src/skill_executor.rs
pub struct SkillExecutor {
    docker_manager: DockerManager,
    skill_cache: SkillCache,
    odlaguna: OdlagunaClient,
}

impl SkillExecutor {
    pub async fn execute_skill(
        &self,
        skill_id: &str,
        params: &serde_json::Value,
    ) -> Result<String, Error> {
        // 1. Check cache
        if let Some(cached) = self.skill_cache.get(skill_id) {
            return Ok(cached.execute(params)?);
        }

        // 2. Load skill from Pandora
        let skill = self.odlaguna.load_skill(skill_id).await?;

        // 3. Prepare container
        let container_config = DockerConfig {
            image: "mimi-worker:latest",
            memory_mb: 256,
            timeout_ms: if skill.language == SkillLanguage::Rhai {
                100
            } else {
                500
            },
        };

        // 4. Execute in isolated container
        let result = self.docker_manager
            .run_skill(&skill.code, params, &container_config)
            .await?;

        // 5. Cache resultado
        self.skill_cache.put(skill_id, result.clone());

        Ok(result)
    }
}

pub struct SkillCache {
    cache: Arc<RwLock<LruCache<String, CachedSkill>>>,
}

#[derive(Clone)]
pub struct CachedSkill {
    pub code: String,
    pub last_execution: SystemTime,
    pub execution_count: u64,
}

impl CachedSkill {
    pub fn execute(&self, params: &serde_json::Value) -> Result<String, Error> {
        // Executar Rhai direto em processo (já validado, seguro)
        let mut engine = rhai::Engine::new();
        let result = engine.eval::<String>(&self.code)?;
        Ok(result)
    }
}
```

**DoD:**
- [ ] Skill executa via Ryzu container
- [ ] Timeout respeitado (< 100ms Rhai, < 500ms WASM)
- [ ] Cache funciona (hit rate > 80%)
- [ ] Resultado correto

---

### T4.5: End-to-End Autonomous Evolution (🟡 Alta)
**Bloqueado por:** T4.4 (Execution)  
**Bloqueia:** Nada (M4 complete)  

**Descrição:**
- Teste completo: Problema detectado → Skill criada → Executada
- Performance: détecção < 1s, geração < 5s, execução < 500ms
- Logging e telemetria de evolução
- Documentação para operador

**Dependências Técnicas:**
- Todos os anteriores

**Artefatos:**
- `tests/e2e_evolution.rs` — Integration test
- `specs/SKILL-LIFECYCLE.md` — Documentação

**Fluxo Esperado:**
```
1. User executa mesma tarefa 5x
2. Echidna detecta padrão (automability > 0.8)
3. Gera Rhai skill simples
4. Submete para Odlaguna validar ✓
5. Registra em Pandora
6. Próxima execução: Mimi usa skill direto (via Ryzu)
7. Resultado: 50% mais rápido, zero código manual
```

**DoD:**
- [ ] End-to-end flow executa sem erro
- [ ] Skill detectada em < 1 segundo
- [ ] Skill gerada em < 5 segundos
- [ ] Skill executada em < 500ms
- [ ] Logging rastreia cada passo

---

## Requisitos Não-Funcionais Aplicáveis

| RNF | Alvo | Status |
|-----|------|--------|
| **RNF-1** (Performance) | Skill Rhai < 100ms | ✅ T4.4 |
| **RNF-1** (Performance) | Skill WASM < 500ms | ✅ T4.4 |
| **RNF-4** (Escalabilidade) | 1000+ skills registadas | ✅ T4.3 |

---

## Timeline

| Semana | Tarefa | Deliverable |
|--------|--------|-------------|
| 1-2 | T4.0 (Pattern Detection) | Detector + candidates |
| 2-3 | T4.1 (Rhai Generator) | Code generation |
| 3-4 | T4.2 (WASM Compiler) | Compilation pipeline |
| 4-5 | T4.3 (Validation + Registry) | Pandora integration |
| 5-7 | T4.4 (Execution + Cache) | Executor + caching |
| 7-8 | T4.5 (E2E) | Full evolution cycle |
| 8-10 | Tests + Tuning | Benchmarks, optimizations |

---

## Critérios de Aceitação Finais (M4 DoD)

✅ **Milestone 4 Completo quando:**

- [ ] Echidna detecta padrão repetido → gera Rhai script
- [ ] Rhai script executa via Ryzu
- [ ] WASM runtime (Wasmtime) integrado
- [ ] Echidna compila skill → WASM
- [ ] Skill WASM passa validação Odlaguna → deploy
- [ ] Próxima execução usa skill em cache
- [ ] End-to-end: problema → evolução → resolução (< 10s total)
- [ ] Performance: Rhai < 100ms, WASM < 500ms
- [ ] All tests pass

---

## Bloqueadores Conhecidos

| Bloqueador | Risco | Mitigação |
|-----------|-------|-----------|
| **Pattern detection accuracy** | Falsos positivos | Threshold tuning, threshold validation |
| **Code generation quality** | Gerado código ruim | Human review + Odlaguna strict validation |
| **WASM compilation time** | Lento demais | Pre-compile templates, caching |

---

## Notas

- **M4 é climático** — MiMi finalmente evolui
- **Skills mudam o jogo** — Antes: agent. Depois: evolving agent.
- **Echidna é o coração** — Pattern detection é criativo, generation é técnico
- **Segurança paramount** — Odlaguna não deixa código malicioso passar

---

## Referências Cruzadas

- Volta a: [`REQUIREMENTS.md#RF-4`](../REQUIREMENTS.md#rf-4-criação-dinâmica-de-skills-echidna)
- Anterior: [`milestones/M3-SECURITY.md`](M3-SECURITY.md)
- Final: Próximo step → Maintenance & Evolution (M5)
