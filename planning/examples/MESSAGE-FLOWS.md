# Message Flow Diagrams: Liliana + Gating System + Persona Injection

Complete integration flows showing how all system components interact via the Message Bus.

---

## 1. Trivial Request Flow (Tier 1 - Liliana Cache Hit)

**Scenario:** User asks "O que você faz?" (What do you do?) - a common social question.

```mermaid
sequenceDiagram
    participant User
    participant Beatrice
    participant MessageBus as Message Bus<br/>(NATS)
    participant Liliana
    participant Gating as Gating System
    participant Odlaguna
    
    User->>Beatrice: "O que você faz?"
    
    Beatrice->>Gating: Route request (classify intent)
    activate Gating
    Gating->>Liliana: Query cache (confidence check)
    activate Liliana
    Liliana-->>Gating: Cache HIT + confidence=0.95
    deactivate Liliana
    Gating->>Gating: Tier 1 match<br/>(cost ≈ 0 tokens)
    Gating-->>Beatrice: Route to Liliana cache
    deactivate Gating
    
    Beatrice->>MessageBus: Subscribe to<br/>liliana/personality_update
    Beatrice->>Liliana: Get cached response<br/>+ personality modifiers
    activate Liliana
    Liliana->>MessageBus: Publish cached response
    Liliana->>MessageBus: Publish personality state
    MessageBus-->>Beatrice: Personality injected
    deactivate Liliana
    
    Beatrice->>Beatrice: Apply personality filters<br/>(formality, confidence, etc)
    Beatrice->>Beatrice: Format response
    Beatrice-->>User: "Sou a Beatrice...<br/>[styled by Liliana mood]"
    
    Beatrice->>Odlaguna: Publish response to audit
    activate Odlaguna
    Odlaguna->>Odlaguna: Log to audit trail<br/>(Tier 1 cache hit)
    Odlaguna->>MessageBus: Publish audit event
    deactivate Odlaguna
```

**Token Cost:** ~10 tokens (cache lookup only)  
**Latency:** ~50ms (local cache + personality application)  
**Activation:** None (Tier 1 is always available)

---

## 2. Moderate Request Flow (Tier 2 - Automated Skill)

**Scenario:** User asks "Cria uma função JavaScript para somar dois números" (Create a JS function to add two numbers).

```mermaid
sequenceDiagram
    participant User
    participant Beatrice
    participant MessageBus as Message Bus
    participant Liliana
    participant Gating as Gating System
    participant Echidna
    participant Odlaguna
    participant Ryzu
    
    User->>Beatrice: "Cria uma função..."
    
    Beatrice->>Gating: Route request (code generation intent)
    activate Gating
    Gating->>Liliana: Query cache (confidence check)
    activate Liliana
    Liliana-->>Gating: Cache MISS or low confidence
    deactivate Liliana
    
    Gating->>Gating: Check Echidna skills<br/>for "code_generator"
    Gating->>Gating: Estimate costs:<br/>Tier2 ≈ 150 tokens<br/>vs Tier3 ≈ 800 tokens
    Gating-->>Beatrice: Route to Tier 2<br/>(skill exists + saves 650 tokens)
    deactivate Gating
    
    Beatrice->>MessageBus: Publish intent/<br/>code_generation
    activate Echidna
    MessageBus->>Echidna: code_generation intent
    Echidna->>Echidna: Load "code_generator" skill
    Echidna->>Ryzu: Execute in sandbox<br/>(function template + params)
    activate Ryzu
    Ryzu->>Ryzu: Generate function<br/>(Rhai or WASM)
    Ryzu-->>Echidna: Result + execution time
    deactivate Ryzu
    Echidna->>MessageBus: Publish skill/<br/>execution_result
    deactivate Echidna
    
    MessageBus->>Beatrice: execution_result received
    
    Beatrice->>MessageBus: Subscribe to<br/>liliana/personality_update
    Beatrice->>Liliana: Get current personality
    activate Liliana
    Liliana->>Liliana: Compute modifiers<br/>(mood → personality)
    Liliana->>MessageBus: Publish<br/>liliana/personality_update
    deactivate Liliana
    MessageBus-->>Beatrice: Personality state
    
    Beatrice->>Beatrice: Apply code_style from personality<br/>(e.g., "concise" → shorter comments)
    Beatrice->>Beatrice: Format code example
    Beatrice-->>User: JavaScript function<br/>[styled to personality]
    
    Beatrice->>MessageBus: Publish response
    
    Beatrice->>Odlaguna: Notify execution
    activate Odlaguna
    Odlaguna->>Odlaguna: Validate response safety
    Odlaguna->>Odlaguna: Check skill success rate
    Odlaguna->>MessageBus: Publish audit event<br/>(Tier 2, skill_id=code_gen_v1)
    Odlaguna->>MessageBus: Publish metrics<br/>(tokens used, latency, success)
    deactivate Odlaguna
    
    Beatrice->>MessageBus: Subscribe to<br/>pandora/store_request
    Beatrice->>MessageBus: Request store personality<br/>+ response history
    activate Pandora
    Pandora->>Pandora: Create PersonalitySnapshot node
    Pandora->>Pandora: Link to session
    Pandora-->>Beatrice: Stored (snapshot_id)
    deactivate Pandora
```

**Token Cost:** ~150 tokens (skill execution + personality styling)  
**Latency:** ~300ms (skill execution + message bus roundtrip)  
**Activation:** Echidna + Ryzu (sandboxed execution)  
**Savings vs Tier 3:** 650 tokens (81% reduction)

---

## 3. Complex Request Flow (Tier 3 - Full Cognitive Pipeline)

**Scenario:** User asks "Como implementar um sistema de cache distribuído seguindo padrões de design?" (How to implement a distributed caching system following design patterns?).

```mermaid
sequenceDiagram
    participant User
    participant Beatrice
    participant MessageBus as Message Bus
    participant Liliana
    participant Gating as Gating System
    participant Mimi as Mimi Commander
    participant Pandora
    participant Odlaguna
    
    User->>Beatrice: "Como implementar..."
    
    Beatrice->>Gating: Route request (architectural advice - ambiguous)
    activate Gating
    Gating->>Liliana: Query cache (confidence check)
    activate Liliana
    Liliana-->>Gating: Cache MISS<br/>(complex, context-dependent)
    deactivate Liliana
    
    Gating->>Gating: Check budget:<br/>daily_limit=1000 tokens<br/>used_today=750 tokens<br/>available=250 tokens
    Gating->>Gating: Tier3 would cost ~800 tokens<br/>exceeds available (250 tokens)
    
    Note over Gating: Cost-benefit analysis:<br/>- No cached response<br/>- No skill available<br/>- Token budget exhausted<br/>→ DEFER execution
    
    Gating-->>Beatrice: DEFER<br/>(budget exhausted)
    deactivate Gating
    
    Beatrice-->>User: "Desculpe, estou com orçamento<br/>de tokens limitado. Tente novamente<br/>em uma nova sessão."
    
    Beatrice->>Odlaguna: Log deferred request
    activate Odlaguna
    Odlaguna->>MessageBus: Publish audit event<br/>(DEFERRED, reason=budget_exhausted)
    Odlaguna->>Pandora: Store deferred request<br/>for analytics
    deactivate Odlaguna
```

**Alternative: Tier 3 with budget available:**

```mermaid
sequenceDiagram
    participant User
    participant Beatrice
    participant MessageBus as Message Bus
    participant Liliana
    participant Gating as Gating System
    participant Mimi as Mimi Commander
    participant Pandora
    participant Odlaguna
    
    User->>Beatrice: "Como implementar..."
    
    Beatrice->>Gating: Route request
    activate Gating
    Gating->>Liliana: Query cache (MISS)
    Gating->>Gating: Check budget: 250 used / 1000 = 25%<br/>Tier3 cost ~800 tokens OK
    Gating-->>Beatrice: Route to Tier 3<br/>(full pipeline)
    deactivate Gating
    
    Beatrice->>MessageBus: Publish intent/<br/>architectural_advice
    
    activate Mimi
    MessageBus->>Mimi: intent received
    
    Mimi->>Pandora: Query session history<br/>(heatmap + context)
    activate Pandora
    Pandora->>Pandora: BFS with temperature filter<br/>(retrieve relevant architecture notes)
    Pandora-->>Mimi: Context window (~300K tokens)
    deactivate Pandora
    
    Mimi->>Mimi: Reason about distributed caching<br/>(LLM inference ~500 tokens)
    
    Mimi->>MessageBus: Publish response
    deactivate Mimi
    
    MessageBus->>Beatrice: response received
    
    Beatrice->>MessageBus: Subscribe to<br/>liliana/personality_update
    Beatrice->>Liliana: Get current personality
    activate Liliana
    Liliana->>Liliana: Check mood state<br/>(confidence=0.6, curiosity=0.8)
    Liliana->>Liliana: Compute modifiers
    Liliana->>MessageBus: Publish personality_update
    deactivate Liliana
    
    Beatrice->>Beatrice: Apply personality<br/>(moderate confidence→add caveats)<br/>(high curiosity→add questions)
    Beatrice->>Beatrice: Format response with examples
    Beatrice-->>User: "Há várias abordagens...<br/>[styled to mood]"
    
    Beatrice->>Odlaguna: Notify Tier 3 execution
    activate Odlaguna
    Odlaguna->>Odlaguna: Log full pipeline execution<br/>(Tier3, mimi_tokens=500,<br/>context_tokens=300, personality_v=42)
    Odlaguna->>MessageBus: Publish audit event
    Odlaguna->>MessageBus: Update metrics<br/>(total_tokens, latency_ms)
    deactivate Odlaguna
    
    Beatrice->>MessageBus: Request personality snapshot
    Pandora->>Pandora: Store mood state +<br/>response trajectory
```

**Token Cost:** ~800 tokens (full LLM reasoning + context)  
**Latency:** ~2-3 seconds (LLM inference + context retrieval)  
**Activation:** Mimi + Pandora (full cognitive pipeline)  
**Budget Management:** May defer if budget exhausted

---

## 4. Personality State Update Flow (Mood Change)

**Scenario:** Security alert detected → Liliana shifts mood to "cautious" → All system responses harden.

```mermaid
sequenceDiagram
    participant Odlaguna
    participant MessageBus as Message Bus
    participant Liliana
    participant Beatrice
    participant Pandora
    
    Odlaguna->>MessageBus: Publish odlaguna/<br/>security_alert
    Note over Odlaguna: Suspicious pattern detected:<br/>token usage spike
    
    MessageBus->>Liliana: security_alert received
    activate Liliana
    
    Liliana->>Liliana: Update mood state:<br/>caution: 0.3 → 0.9<br/>confidence: 0.8 → 0.4
    
    Liliana->>Liliana: Recompute personality modifiers
    Note over Liliana: mood_modifiers affecting<br/>formality, confidence, urgency
    
    Liliana->>Liliana: Generate new<br/>PersonalityProfile v43
    
    Liliana->>Liliana: Compute checksum<br/>(SHA256 signature)
    
    Liliana->>MessageBus: Publish liliana/<br/>personality_update (v43)
    deactivate Liliana
    
    MessageBus->>Odlaguna: personality_update received
    activate Odlaguna
    Odlaguna->>Odlaguna: Validate personality<br/>(bounds, vocabulary, checksum)
    
    alt validation passes
        Odlaguna->>MessageBus: Publish odlaguna/<br/>personality_validation<br/>(approved)
    else validation fails
        Odlaguna->>MessageBus: Publish odlaguna/<br/>personality_validation<br/>(rejected)
        Liliana--xMessageBus: Personality NOT applied
    end
    deactivate Odlaguna
    
    MessageBus->>Beatrice: personality_update (v43) received
    activate Beatrice
    
    Beatrice->>Beatrice: Update internal personality state
    Note over Beatrice: formality ↑ (0.6→0.8)<br/>confidence ↓ (0.8→0.5)<br/>caution ↑ (0.3→0.9)
    
    Beatrice->>Beatrice: All subsequent responses<br/>will use v43 modifiers
    deactivate Beatrice
    
    MessageBus->>Pandora: personality_update snapshot
    activate Pandora
    Pandora->>Pandora: Create PersonalitySnapshot node (v43)
    Pandora->>Pandora: Link mood transition<br/>(v42)→[hardened](v43)
    Pandora->>Pandora: Record timestamp<br/>+ trigger (security_alert)
    deactivate Pandora
    
    Note over MessageBus: All subsequent user responses<br/>will be styled with v43 personality<br/>(cautious, formal, lower confidence)
```

**Trigger:** Odlaguna security alert  
**Propagation Time:** ~50ms (message bus latency)  
**Duration:** Until next mood reset or user override

---

## 5. Gating Decision Tree (Token Budget Management)

**Scenario:** Sequential requests with decreasing token budget.

```mermaid
graph TD
    A["Request arrives"] --> B["Classify intent<br/>(NLU confidence)"]
    
    B --> C{"Cache hit?<br/>(exact-match or semantic)"}
    
    C -->|YES| D["Tier 1<br/>Return cached response<br/>~0 tokens"]
    C -->|NO| E{"Skill exists?<br/>(Echidna check)"}
    
    E -->|YES| F{"Token budget<br/>check"}
    E -->|NO| G{"Token budget<br/>check"}
    
    F --> F1{"Skill tokens<br/>< budget?"}
    G --> G1{"Tier3 tokens<br/>< budget?"}
    
    F1 -->|YES| H["Tier 2<br/>Execute skill<br/>~100-300 tokens"]
    F1 -->|NO| I["DEFER<br/>Budget exhausted"]
    
    G1 -->|YES| J["Tier 3<br/>Full pipeline<br/>~500-1000 tokens"]
    G1 -->|NO| K["DEFER<br/>Budget exhausted"]
    
    D --> L["Apply personality<br/>modifiers"]
    H --> L
    J --> L
    
    L --> M["Beatrice formats<br/>response"]
    M --> N["Odlaguna<br/>validates + audits"]
    N --> O["Pandora stores<br/>personality snapshot"]
    
    I --> P["Return deferral message<br/>Suggest: New session"]
    K --> P
    
    style D fill:#90EE90
    style H fill:#87CEEB
    style J fill:#FFB6C1
    style I fill:#FFD700
    style K fill:#FFD700
    style L fill:#DDA0DD
    style N fill:#F0E68C
    style O fill:#B0C4DE
```

---

## 6. Circuit Breaker Integration (Skill Reliability)

**Scenario:** A skill fails repeatedly → Circuit breaker opens → Requests route to Tier 3 instead.

```mermaid
stateDiagram-v2
    [*] --> Closed: Skill healthy<br/>(success_rate > 95%)
    
    Closed --> Closed: Success<br/>→ Continue<br/>using skill
    
    Closed --> Open: Failure threshold<br/>reached<br/>(N consecutive failures)
    
    Open --> Open: Request arrives<br/>→ FAST FAIL<br/>→ Reroute to Tier 3
    
    Open --> HalfOpen: Timeout<br/>(circuit_break_timeout = 60s)
    
    HalfOpen --> Closed: Test succeeds<br/>→ Skill recovered<br/>→ Resume normal flow
    
    HalfOpen --> Open: Test fails<br/>→ Still failing<br/>→ Stay open
    
    note right of Closed
        Tier 2 active
        Skills executing
        Fast responses
    end note
    
    note right of Open
        Tier 2 disabled
        All requests rerouted to Tier 3
        Waiting for skill recovery
    end note
    
    note right of HalfOpen
        Limited test traffic
        Probing skill recovery
        If recovered → resume Tier 2
    end note
```

**Impact on Gating:**
- When circuit is **OPEN**: Requests skip Tier 2, go directly to Tier 3
- When circuit is **HALF-OPEN**: Limited traffic to skill for testing
- When circuit is **CLOSED**: Normal Tier 2 routing

---

## 7. Full System Integration (Request to Response)

**Overview of all components interacting:**

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER INPUT                                  │
│                    "Cria uma API REST"                             │
└────────────────────────────┬────────────────────────────────────────┘
                             │
                             ▼
        ┌────────────────────────────────────────┐
        │         BEATRICE (NLP Interface)       │
        │                                        │
        │  Parse intent (Intent Extractor)      │
        │  Confidence scoring                    │
        └────────────────┬───────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │      GATING SYSTEM (3-Tier Router)    │
        │                                        │
        │  1. Query Liliana cache?               │
        │     → Tier 1 (~0 tokens)               │
        │                                        │
        │  2. Echidna skill exists?              │
        │     → Tier 2 (~150 tokens)             │
        │                                        │
        │  3. Full pipeline?                     │
        │     → Tier 3 (~800 tokens)             │
        │                                        │
        │  4. Budget check                       │
        │     → DEFER if exhausted               │
        └────────────────┬───────────────────────┘
                         │
              ┌──────────┼──────────┐
              │          │          │
              ▼          ▼          ▼
         ┌────────┐ ┌───────────┐ ┌─────────┐
         │Liliana │ │  Echidna  │ │  Mimi   │
         │(Cache) │ │ (Skill)   │ │(Pipeline)
         └────────┘ └───────────┘ └─────────┘
              │          │          │
              └──────────┼──────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │       LILIANA (Mood + Personality)    │
        │                                        │
        │  Current mood state                    │
        │  Compute personality modifiers         │
        │  Publish PersonalityProfile            │
        └────────────────┬───────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │    BEATRICE (Response Styling)         │
        │                                        │
        │  Apply personality filters             │
        │  Format response                       │
        │  Render for user                       │
        └────────────────┬───────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │      ODLAGUNA (Safety Gating)         │
        │                                        │
        │  Validate response safety              │
        │  Check personality bounds              │
        │  Log audit trail                       │
        └────────────────┬───────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │    PANDORA (Memory + Persistence)     │
        │                                        │
        │  Store response history                │
        │  Store personality snapshot            │
        │  Update heatmap                        │
        └────────────────┬───────────────────────┘
                         │
                         ▼
        ┌────────────────────────────────────────┐
        │           USER OUTPUT                  │
        │   "Aqui está uma API REST seguindo..." │
        │        [Styled by Liliana mood]       │
        └────────────────────────────────────────┘
```

---

## 8. Error Recovery Flows

### 8.1 Skill Execution Failure

```mermaid
sequenceDiagram
    participant Beatrice
    participant Echidna
    participant Ryzu
    participant Odlaguna
    
    Beatrice->>Echidna: Execute skill
    activate Echidna
    Echidna->>Ryzu: Run sandboxed code
    activate Ryzu
    Ryzu--xEchidna: Error (timeout/crash)
    deactivate Ryzu
    Echidna--xBeatrice: Skill failed
    deactivate Echidna
    
    Beatrice->>Odlaguna: Skill execution failed
    activate Odlaguna
    Odlaguna->>Odlaguna: Update circuit breaker<br/>(failure_count++)
    
    alt failure_count < threshold
        Odlaguna-->>Beatrice: Retry? Fallback to Tier 3?
    else threshold reached
        Odlaguna->>Odlaguna: Circuit breaker → OPEN
        Odlaguna-->>Beatrice: Circuit open<br/>Use Tier 3 for future requests
    end
    deactivate Odlaguna
    
    Beatrice->>Beatrice: Either retry or reroute to Tier 3
    Beatrice-->>User: Response (via fallback)
```

### 8.2 Personality Validation Failure

```mermaid
sequenceDiagram
    participant Liliana
    participant Odlaguna
    participant Beatrice
    
    Liliana->>Liliana: Compute mood-based personality
    Liliana->>Liliana: Create PersonalityProfile v44
    Liliana->>Odlaguna: Validate personality
    
    activate Odlaguna
    Odlaguna->>Odlaguna: Check bounds<br/>Check vocabulary<br/>Check checksum
    
    alt validation fails
        Odlaguna-->>Liliana: Personality REJECTED<br/>(reason: vocabulary injection detected)
        Liliana->>Liliana: Revert to previous valid<br/>personality (v43)
        Liliana->>Liliana: Log validation failure
    else validation passes
        Odlaguna-->>Liliana: Personality APPROVED
        Liliana->>Beatrice: Apply new personality
    end
    deactivate Odlaguna
```

---

## 9. Message Bus Topic Reference

| Topic | Direction | Source | Consumer | Payload |
|-------|-----------|--------|----------|---------|
| `intent/raw` | → | Beatrice | Gating/Mimi | Intent struct |
| `liliana/personality_update` | → | Liliana | Beatrice, Odlaguna, Pandora | PersonalityInjection |
| `liliana/mood_event` | → | Liliana | Monitoring | MoodChangeEvent |
| `liliana/response_ready` | → | Liliana | Beatrice | CachedResponse |
| `gating/routing_decision` | → | Gating | Beatrice, Mimi | RoutingDecision (Tier1/2/3/Defer) |
| `skill/execute` | → | Beatrice | Echidna | SkillExecuteRequest |
| `skill/execution_result` | → | Echidna | Beatrice, Odlaguna | SkillResult |
| `task/execute` | → | Beatrice | Mimi | TaskExecuteRequest |
| `intent/response/{request_id}` | → | Mimi | Beatrice | Response |
| `odlaguna/personality_validation` | → | Odlaguna | Liliana | ValidationResult |
| `odlaguna/security_alert` | → | Odlaguna | Liliana, Mimi | SecurityAlert |
| `audit/event` | → | Odlaguna | Pandora, Monitoring | AuditEvent |
| `pandora/store_request` | → | Beatrice | Pandora | StoreRequest |
| `pandora/personality_snapshot` | → | Pandora | Monitoring | PersonalitySnapshot |

