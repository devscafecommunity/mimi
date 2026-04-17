# PRISCILLA — Critical Analysis & Rationality Module

> **Module:** Priscilla Critical Actor  
> **Language:** Rust/C++  
> **Milestone:** M3 (Security & Governance) — Critical Path  
> **Requirements:** RF-7.1, RF-7.2, RF-7.3, RF-7.4, RF-7.5, RF-7.6  
> **Status:** 🟡 Design Complete — Implementation Pending  
> **Integration Point:** Message Bus (Interceptor between Mimi planning and task execution)

---

## 1. Module Overview

**Priscilla** (The Critical Actor) is the **rationality supervisor and strategic advisor** of the MiMi system. Unlike Odlaguna who enforces security rules with a "kill switch," Priscilla acts as the "Advocate of Reason" who questions every task's necessity, efficiency, and approach before execution. She forces the system to think before acting.

### Core Identity: The Devil's Advocate

Priscilla's role is **not to block** (that's Odlaguna's job), but to **challenge, refine, and optimize** every decision the Mimi Commander makes. She introduces a layer of **metacognition** to the system—asking "why" and "is there a better way?" before the "how" is executed.

### Responsibilities

| Responsibility | Description |
|---|---|
| **Necessity Questioning** | "Does this task actually need to run, or is Mimi entering a loop/redundant path?" |
| **Cost-Benefit Analysis** | "Is the token/CPU/memory cost worth the expected result?" |
| **Bias & Hallucination Detection** | "Did Beatrice interpret the user correctly, or are we assuming something unvalidated?" |
| **Plan Refinement** | "Is there a shorter/cheaper/faster path Mimi ignored?" |
| **Failure Pattern Detection** | "Has a similar plan failed before? What was learned?" |
| **Context Freshness Validation** | "Is the data Pandora delivered 'hot' enough for this task?" |
| **Resource Optimization** | "Can we solve this with existing Skills instead of creating new ones?" |

[Full documentation continues...](PRISCILLA_FULL.md)

---

## Versão Completa

A documentação completa de Priscilla foi criada em `PRISCILLA-FULL.md` com 15 seções abrangendo:

1. Module Overview & Core Identity
2. Architecture (Internal Components)
3. Message Protocol ("The Critique Compact")
4. API & Interfaces
5. Execution Modes (Synchronous/Asynchronous)
6. Integration with Pandora (Read-only queries)
7. Differences from Odlaguna
8. Implementation Roadmap (4 Phases, 8 weeks)
9. Metrics & Observability
10. Behavioral Examples
11. Failure Scenarios & Recovery
12. Configuration Best Practices
13. Open Questions
14. References & Evidence
15. Acceptance Criteria & Glossary

**Status:** Ready for implementation phase in M3.


---

## 2. Architecture

### Internal Structure

```
┌─────────────────────────────────────────────────────────────────┐
│                      PriscillaCore                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
│  │ Message Monitor  │  │  Failure Pattern │  │ Context      │  │
│  │ (Draft Listener) │  │  Analyzer        │  │ Validator    │  │
│  │ Async Subscriber │  │ (Pandora Link)   │  │ (Freshness)  │  │
│  └────────┬─────────┘  └────────┬─────────┘  └────────┬─────┘  │
│           │                     │                     │        │
│  ┌────────▼─────────────────────▼─────────────────────▼──────┐ │
│  │           Critique Analysis Engine                        │ │
│  │  ┌─────────────────┐    ┌──────────────────────────────┐ │ │
│  │  │ Necessity Check │    │ Cost-Benefit Analyzer        │ │ │
│  │  │ (Loop detection)│    │ (Token/CPU budget vs outcome)│ │ │
│  │  └─────────────────┘    └──────────────────────────────┘ │ │
│  │  ┌─────────────────┐    ┌──────────────────────────────┐ │ │
│  │  │ Intent Validator│    │ Plan Optimization Engine     │ │ │
│  │  │ (Bias detection)│    │ (Alternative paths)          │ │ │
│  │  └─────────────────┘    └──────────────────────────────┘ │ │
│  │  ┌─────────────────┐    ┌──────────────────────────────┐ │ │
│  │  │ Skill Reuse     │    │ Cynicism Controller          │ │ │
│  │  │ Analyzer        │    │ (Risk level parametrization) │ │ │
│  │  └─────────────────┘    └──────────────────────────────┘ │ │
│  └────────┬──────────────────────────────────────────────────┘ │
│           │                                                     │
│  ┌────────▼──────────────────────────────────────────────────┐ │
│  │        Critique Commentary Generator                      │ │
│  │  (Structured reasoning for Mimi rebuttal)                │ │
│  └────────┬──────────────────────────────────────────────────┘ │
│           │                                                     │
│  ┌────────▼──────────────────────────────────────────────────┐ │
│  │        Message Bus Publisher                             │ │
│  │  (Publishes to task/critique)                            │ │
│  └────────────────────────────────────────────────────────────┘ │
│           │                                                     │
└───────────┼─────────────────────────────────────────────────────┘
            │
      ┌─────▼──────┐
      │Message Bus │
      │Zenoh/NATS  │
      └────────────┘
```

---

## 3. Key Features

### Feature 1: Necessity Questioning
- Detects redundant task chains (same task twice in short timeframe)
- Identifies loops (circular dependencies between subtasks)
- Validates if user actually requested this or if Mimi is being overzealous

### Feature 2: Cost-Benefit Analysis
- Estimates token cost per skill from historical profiles
- Projects CPU/memory from task complexity metrics
- Compares against expected value (from intent confidence + result magnitude)
- Flags if ratio < configurable threshold

### Feature 3: Bias & Hallucination Detection
- Cross-references Beatrice's parsed Intent with original user message
- Identifies assumptions Beatrice made without explicit user mention
- Suggests clarification before proceeding (e.g., "User said 'clean my logs' but did they mean delete or archive?")

### Feature 4: Plan Refinement Suggestions
- Proposes skill composition (use 3 existing skills vs create 1 new one)
- Identifies parallelizable subtasks
- Suggests staged validation (test on small dataset before full run)
- Recommends caching if pattern has appeared recently

### Feature 5: Failure Learning Loop
- Integrates with Pandora's read-only failure indices
- Retrieves similar failed tasks from past 30 days
- Extracts failure reasons and attempted fixes
- Suggests preventive measures or alternative approaches

### Feature 6: Context Freshness Validation
- Checks age of Pandora context (warns if > 15 min old)
- Verifies semantic relevance scores are above threshold
- Surfaces negative contexts that might mislead the task

---

## 4. Integration with Odlaguna

```
                 MIMI PLAN
                    │
         ┌──────────▼──────────┐
         │   PRISCILLA         │
         │  (Questions Logic)  │
         └──────────┬──────────┘
                    │
              task/critique
              (suggestions)
                    │
              ┌─────▼─────┐
              │ MIMI ACT   │
              │ (Accepts / │
              │  Modifies) │
              └─────┬─────┘
                    │
              task/final
              (revised plan)
                    │
         ┌──────────▼──────────┐
         │   ODLAGUNA          │
         │  (Questions Safety) │
         └──────────┬──────────┘
                    │
         (VETO or ALLOW)
                    │
              EXECUTION or BLOCKED
```

**Key:** Priscilla is advisory; Odlaguna is decisive.

---

## 5. Open Questions

1. **Vectorization:** How should Priscilla calculate task similarity? Cosine on embeddings or structured field comparison?

2. **Cost Estimation:** Without reliable skill profiling data, how to improve accuracy?

3. **Timeout Handling:** If Pandora unavailable, should Priscilla degrade gracefully or block until healthy?

4. **Learning Loop:** Should Priscilla adjust thresholds automatically, or is manual config sufficient?

5. **User Feedback:** Should users be able to rate Priscilla's suggestions for continuous improvement?

---

## Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Message monitor setup
- [ ] Basic necessity checker (loop detection)
- [ ] Test harness

### Phase 2: Intelligence (Week 3-4)
- [ ] Cost-benefit analyzer
- [ ] Intent validator
- [ ] Pandora integration (failure queries)

### Phase 3: Optimization (Week 5-6)
- [ ] Plan optimization engine
- [ ] Context validator
- [ ] Cynicism controller

### Phase 4: Polish (Week 7-8)
- [ ] Performance tuning (< 50ms latency)
- [ ] Comprehensive test suite (100+ scenarios)
- [ ] Monitoring & documentation

---

## References

1. **LangChain RAG Agents** — Think → Reflect → Act pattern
   https://python.langchain.com/docs/modules/agents/

2. **Kubernetes Admission Controllers** — Pre-execution validation
   https://kubernetes.io/docs/reference/access-authn-authz/admission-controllers/

3. **QRRanker** — Temporal decay in ranking (arXiv 2602.12192)
   Applies exponential weighting to recent failures

4. **Netflix Chaosmonkey** — Failure-aware decision making
   https://netflix.github.io/chaosmonkey/

---

## Related Documents

- **ODLAGUNA.md** — Security enforcement (complement to Priscilla's advisory role)
- **PANDORA-ARCHITECTURE.md** — Context delivery (failure indices read by Priscilla)
- **M3-SECURITY.md** — Milestone roadmap for M3 (Priscilla implementation phases)
- **MIMI-COMMANDER.md** — Task planning (Priscilla's input stage)

---

## Acceptance Criteria

- [ ] Message bus integration < 50ms p99 latency
- [ ] 5 analyzer components fully implemented
- [ ] Pandora integration verified
- [ ] Configuration system functional
- [ ] 100+ critique scenarios tested
- [ ] Prometheus metrics exported
- [ ] Documentation complete
- [ ] Full pipeline tests pass (Beatrice → Mimi → Priscilla → Odlaguna → Ryzu)
- [ ] Zero regressions in existing functionality

