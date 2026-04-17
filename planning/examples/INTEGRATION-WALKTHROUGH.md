# Integration Walkthrough: Liliana + Gating System + Persona Injection

Complete end-to-end walkthrough showing how all three components work together to handle a user request.

---

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Scenario 1: Simple Reflex Response (Tier 1)](#scenario-1-simple-reflex-response-tier-1)
3. [Scenario 2: Code Generation with Skill (Tier 2)](#scenario-2-code-generation-with-skill-tier-2)
4. [Scenario 3: Complex Architecture Discussion (Tier 3)](#scenario-3-complex-architecture-discussion-tier-3)
5. [Scenario 4: Security Alert Hardening](#scenario-4-security-alert-hardening)
6. [Scenario 5: Budget Exhaustion and Deferral](#scenario-5-budget-exhaustion-and-deferral)
7. [System Metrics and Monitoring](#system-metrics-and-monitoring)

---

## Architecture Overview

### Component Roles

| Component | Role | Responsibilities |
|-----------|------|------------------|
| **Liliana** | Interactive Presence | Mood state machine, cache management, personality computation |
| **Gating System** | Energy Router | 3-tier decision logic, cost-benefit analysis, budget management |
| **Beatrice** | NLP Interface | Parse user intent, apply personality, format responses |
| **Odlaguna** | Safety Gate | Validate responses, verify personality bounds, audit logging |
| **Pandora** | Memory Store | Persist personality snapshots, store response history, mood trajectory |

### Message Flow (High-Level)

```
User Input → Beatrice (parse)
           → Gating (route: Tier1/2/3/Defer)
           → Liliana/Echidna/Mimi (execute)
           → Liliana (personality compute)
           → Beatrice (apply personality, format)
           → Odlaguna (validate)
           → Pandora (store snapshot)
           → User Output
```

---

## Scenario 1: Simple Reflex Response (Tier 1)

**User:** "Oi! O que você faz?"

**Configuration Files Used:**
- `liliana.toml` (reflex templates, cache)
- `gating.toml` (Tier 1 settings)
- `persona.toml` (personality application)

### Step 1: Beatrice Parses Intent

```rust
// beatrice/src/cli.rs
let user_input = "Oi! O que você faz?";

// Intent Parser (regex-based in M1, NLP in M2+)
let intent = IntentParser::parse(user_input)?;
// → Intent {
//     action: "greeting_social",
//     confidence: 0.95,
//     entities: {},
//     request_id: "req-12345",
// }
```

**Configuration Impact:** `gating.toml` → `tier1.exact_match_min_confidence = 0.95`

### Step 2: Gating System Routes to Tier 1

```rust
// gating-system/src/router.rs
let decision = router.route(&intent)?;

// Gating checks:
// 1. Is this in Liliana cache?
let cache_result = liliana.query_cache("O que você faz?");
//  → CacheHit {
//      response: "Sou a Beatrice, assistente cognitivo...",
//      confidence: 0.95,
//      category: "social",
//      ttl_remaining: 2950_seconds,
//  }

// 2. Does confidence meet threshold?
if cache_result.confidence >= config.tier1.exact_match_min_confidence {
    // YES → Route to Tier 1
    decision = RoutingDecision::Tier1 { cached_response: cache_result }
}

// 3. Estimate cost
let cost_estimate = 10; // ~10 tokens (cache lookup + framing)

// 4. Check budget
if cost_estimate <= budget.remaining_tokens {
    // Proceed with Tier 1
}
```

**Configuration Impact:** 
- `liliana.toml` → `cache.exact_match` enabled, TTL = 3600s
- `gating.toml` → `tier1.estimated_token_cost = 10`

### Step 3: Liliana Provides Cached Response + Personality

```rust
// liliana/src/personality.rs
let mut personality_state = liliana.get_current_mood();
// → MoodState {
//     formality: 0.5,
//     confidence: 0.7,
//     curiosity: 0.8,
//     caution: 0.3,
//     urgency: 0.2,
// }

// Compute personality modifiers
let personality = PersonalityProfile {
    formality: 0.5 + (0.7 * 0.4) + (0.3 * 0.3),  // ≈ 0.68 (slightly formal)
    confidence: 0.7 + (0.8 * 0.5) - (0.3 * 0.3),  // ≈ 0.98 (very confident)
    // ... (compute others)
};

// Publish personality update
liliana.publish_personality_update(&personality)?;
// → Topic: liliana/personality_update
//   Version: 42
//   Checksum: sha256(...)

// Return cached response
let cached = liliana.get_from_cache("O que você faz?")?;
// → "Sou a Beatrice, assistente cognitivo..."
```

**Configuration Impact:** 
- `persona.toml` → formality formula, confidence formula
- `liliana.toml` → `personality.update_frequency_ms = 500`

### Step 4: Beatrice Applies Personality + Formats

```rust
// beatrice/src/personality_filter.rs
let personality = beatrice.current_personality.clone(); // v42

// Select vocabulary based on personality
if personality.confidence > 0.7 {
    // Use confident greetings
    let greeting = "Olá! Como posso ajudá-lo?";
} else {
    // Use tentative greetings
    let greeting = "Oi, tudo bem?";
}

// Apply confidence to response
let base_response = "Sou a Beatrice, assistente cognitivo. Posso ajudar com análise de código, design de arquitetura, e muito mais.";

// Since confidence is high (0.98), keep response as-is (no hedging)
let styled_response = base_response;

// Format for CLI output
let formatted = format!("{}\n[Status: Tier1 cache hit, latency: 45ms]", styled_response);
```

**Configuration Impact:** 
- `persona.toml` → `greetings_casual`, `confidence_formula`
- `gating.toml` → `tier1.latency_target_ms = 50`

### Step 5: Odlaguna Validates + Audits

```rust
// odlaguna/src/security_gate.rs
let validation = odlaguna.validate_response(&styled_response)?;
// → ResponseValidation {
//     valid: true,
//     personality_valid: true,
//     audit_logged: true,
// }

// Log audit event
odlaguna.log_audit_event(AuditEvent {
    event_type: "response_served",
    tier: "tier1",
    source: "liliana_cache",
    timestamp: now(),
    token_cost: 10,
    latency_ms: 45,
});
```

### Step 6: Pandora Stores Personality Snapshot

```rust
// pandora/src/personality_storage.rs
pandora.store_personality_snapshot(
    session_id: "session-123",
    personality: personality_v42,
    trigger: "social_greeting",
)?;

// Creates Neo4j node:
// (:PersonalitySnapshot {
//   version: 42,
//   timestamp: 2026-04-17T12:34:56Z,
//   mood_confidence: 0.98,
//   mood_formality: 0.68,
//   category: "social_greeting",
// })
```

### Step 7: User Sees Response

```
Olá! Como posso ajudá-lo?

Sou a Beatrice, assistente cognitivo. Posso ajudar com análise de código, 
design de arquitetura, e muito mais.

[Metrics: Tier1, 45ms, 10 tokens, confidence: 95%]
```

### Metrics Recorded

```json
{
  "request_id": "req-12345",
  "scenario": "social_greeting",
  "tier_selected": 1,
  "cache_hit": true,
  "confidence": 0.95,
  "latency_ms": 45,
  "tokens_used": 10,
  "personality_version": 42,
  "mood_state": { "confidence": 0.98, "formality": 0.68 },
  "timestamp": "2026-04-17T12:34:56Z"
}
```

---

## Scenario 2: Code Generation with Skill (Tier 2)

**User:** "Cria uma função JavaScript para somar dois números"

**Configuration Files Used:**
- `liliana.toml` (cache lookup, TTL settings)
- `gating.toml` (Tier 2 skill routing, circuit breaker)
- `persona.toml` (code style modifiers)

### Step 1: Beatrice Parses Intent

```rust
let user_input = "Cria uma função JavaScript para somar dois números";

let intent = IntentParser::parse(user_input)?;
// → Intent {
//     action: "code_generation",
//     language: "javascript",
//     confidence: 0.82,
//     entities: { "operation": "sum", "params": 2 },
//     request_id: "req-67890",
// }
```

### Step 2: Gating System Routes to Tier 2

```rust
let decision = router.route(&intent)?;

// Gating checks:

// 1. Is this in Liliana cache? (Tier 1)
let cache_result = liliana.query_cache("Cria uma função JavaScript para somar");
// → CacheMiss (not cached or confidence < threshold)

// 2. Does Echidna have a skill? (Tier 2)
let skill = echidna.find_skill("code_generator")?;
// → Skill {
//     id: "code_gen_v1",
//     language: "rhai",
//     success_rate: 0.96,
//     avg_tokens: 120,
//     availability: "available",
// }

// 3. Is skill available and reliable?
if skill.success_rate > config.tier2.skill_success_rate_min // 0.90
   && !circuit_breaker.is_open("code_gen_v1") {
    // YES → Route to Tier 2
    decision = RoutingDecision::Tier2 { skill }
}

// 4. Estimate cost
let cost_estimate = config.cost_estimation.skill_execution_base_tokens // 100
                  + (skill_complexity_factor * 1.5); // ≈ 150 tokens

// 5. Check budget
if cost_estimate <= budget.remaining_tokens {
    // Proceed with Tier 2
}
```

**Configuration Impact:**
- `gating.toml` → `tier2.skill_success_rate_min = 0.90`, `estimated_token_cost = 150`
- `liliana.toml` → `cache.ttl_code_examples_seconds = 3600`

### Step 3: Beatrice Sends to Echidna

```rust
// beatrice/src/main.rs
beatrice.publish_to_bus("skill/execute", SkillExecuteRequest {
    request_id: "req-67890",
    skill_id: "code_gen_v1",
    params: {
        "language": "javascript",
        "operation": "sum",
        "num_params": 2,
    },
    timeout_ms: 30_000,
})?;
```

### Step 4: Echidna Executes Skill in Ryzu

```rust
// echidna/src/skill_executor.rs
let result = ryzu.execute_skill_sandboxed(SkillExecution {
    skill_id: "code_gen_v1",
    language: "rhai",
    timeout_ms: 30_000,
    params: {...},
})?;

// Rhai script generates:
// function add(a, b) { return a + b; }

// Result:
// SkillResult {
//   status: "success",
//   output: "function add(a, b) { return a + b; }",
//   execution_time_ms: 145,
//   tokens_estimated: 120,
// }
```

### Step 5: Liliana Computes Personality (Code Style)

```rust
// liliana/src/personality.rs
let mut mood = liliana.get_current_mood();
// mood.curiosity = 0.8 (still curious from recent interactions)

let personality = PersonalityProfile {
    code_style: {
        base: "detailed",
        modifier_by_confidence: if mood.confidence > 0.7 { "concise" } else { "verbose" },
        // → "concise"
    },
    explanation_depth: {
        base: "medium",
        modifier_by_curiosity: if mood.curiosity > 0.7 { "high" } else { "medium" },
        // → "high"
    },
};

liliana.publish_personality_update(&personality)?;
```

**Configuration Impact:**
- `persona.toml` → `code_style_high_confidence = "concise"`, `explanation_depth_high_curiosity = "high"`

### Step 6: Beatrice Applies Personality + Formats

```rust
// beatrice/src/response_formatter.rs
let skill_result = "function add(a, b) { return a + b; }";

// Apply personality: code_style = "concise"
// Skip verbose comments, keep minimal explanation
let formatted_response = format!(r#"
Aqui está a função JavaScript:

```javascript
function add(a, b) {{
  return a + b;
}}
```

Uso: `add(5, 3)` → 8
"#);

// Since explanation_depth = "high" and curiosity is high:
// Add optional follow-up questions
let with_curiosity = format!("{}\n\n💡 Quer explorar versões alternativas? Por exemplo, usando arrow function ou validação de tipos?", formatted_response);
```

**Configuration Impact:**
- `persona.toml` → `code_style`, `explanation_depth`, `emoji_frequency`

### Step 7: Odlaguna Validates + Audits

```rust
// odlaguna/src/security_gate.rs
let validation = odlaguna.validate_response(&formatted_response)?;

// Check for code injection, unsafe patterns, etc.
// → Valid!

// Update Tier 2 metrics
odlaguna.update_skill_metrics("code_gen_v1", SkillMetrics {
    success: true,
    token_cost: 120,
    latency_ms: 145,
})?;

// Circuit breaker check: success_count++
// (success_count now = 42, if > threshold, stay CLOSED)
```

### Step 8: Pandora Stores

```rust
// pandora/src/personality_storage.rs
pandora.store_personality_snapshot(...)?;
pandora.store_response_cache(ResponseCache {
    request_id: "req-67890",
    intent: "code_generation",
    response: formatted_response,
    personality_version: 43,
    skill_used: "code_gen_v1",
})?;
```

### Step 9: User Sees Response

```
Aqui está a função JavaScript:

function add(a, b) {
  return a + b;
}

Uso: `add(5, 3)` → 8

💡 Quer explorar versões alternativas? Por exemplo, usando arrow function ou validação de tipos?

[Metrics: Tier2, 145ms, 120 tokens, skill: code_gen_v1]
```

### Metrics Recorded

```json
{
  "request_id": "req-67890",
  "scenario": "code_generation",
  "tier_selected": 2,
  "skill_id": "code_gen_v1",
  "skill_success": true,
  "latency_ms": 145,
  "tokens_used": 120,
  "personality_version": 43,
  "code_style": "concise",
  "explanation_depth": "high",
  "circuit_breaker_state": "closed",
  "timestamp": "2026-04-17T12:35:00Z"
}
```

---

## Scenario 3: Complex Architecture Discussion (Tier 3)

**User:** "Como implementar um sistema de cache distribuído com invalidação baseada em TTL e LRU eviction?"

**Configuration Files Used:**
- `liliana.toml` (no cache hit expected)
- `gating.toml` (Tier 3 routing, budget check)
- `persona.toml` (personality modifiers for complex discussion)

### Step 1-2: Beatrice & Gating Route to Tier 3

```rust
let intent = IntentParser::parse(user_input)?;
// → Intent {
//     action: "architectural_advice",
//     confidence: 0.45,  // Ambiguous question
//     entities: {...},
// }

let decision = router.route(&intent)?;

// Gating checks:
// 1. Liliana cache? NO (complex, context-dependent)
// 2. Echidna skill? NO (no skill for architecture advice)
// 3. Tier 3? YES

// Cost estimation:
let context_size = pandora.estimate_context_size(session_id)?; // 250K tokens
let cost_estimate = 
    config.cost_estimation.llm_reasoning_base_tokens       // 500
    + config.cost_estimation.context_retrieval_tokens      // 300
    + (context_size * 0.001);  // 250
// Total ≈ 1050 tokens

// Budget check:
if 1050 > budget.remaining_tokens {
    // DEFER if budget exhausted
    decision = RoutingDecision::Defer { reason: "budget_exhausted" }
} else {
    decision = RoutingDecision::Tier3
}
```

**Configuration Impact:**
- `gating.toml` → `tier3.estimated_token_cost = 800`, `max_context_tokens = 300_000`

### Step 3: Pandora Retrieves Context

```rust
// pandora/src/context_retrieval.rs
let context = pandora.retrieve_context(session_id, intent)?;

// BFS with heatmap filtering:
// 1. Find relevant nodes (distributed caching, TTL, LRU)
// 2. Filter by temperature (frequently accessed nodes stay hot)
// 3. Prune low-relevance nodes
// 4. Return ranked context

// Result: ~250K tokens of relevant architecture notes, patterns, code examples
```

### Step 4: Mimi Reasons (LLM)

```rust
// mimi-commander/src/reasoning.rs
let response = mimi.reason(&intent, &context)?;

// LLM call (expensive):
// Input tokens: 500 (prompt) + 250K (context) = 250.5K tokens
// Output tokens: ~500 (complex explanation)
// Total: ~251K tokens (but config estimates ~800 for Tier 3)

// Response generated:
let response = r#"
Há várias abordagens para isso. Uma estratégia comum é:

1. **TTL-based Invalidation:**
   - Cada entry tem timestamp
   - Background task verifica expiração
   - Tokens: O(1) lookup, O(n) background cleanup

2. **LRU Eviction:**
   - Doubly-linked list + HashMap
   - O(1) get, put, remove
   - Remove least-recently-used when capacity exceeded

3. **Combination (Hybrid):**
   - TTL para expiração automática
   - LRU para overflow
   - Implementação mais complexa, mas superior em prática

[Mimi then provides code example and trade-offs...]
"#;
```

### Step 5: Liliana Adjusts Personality for Complexity

```rust
// liliana/src/personality.rs

// Complex request detected → mood adjusts:
let mood = MoodState {
    confidence: 0.6,        // Moderate (complex topic)
    caution: 0.5,           // Increased (complex can have pitfalls)
    curiosity: 0.9,         // Very high (architectural question)
    formality: 0.7,         // More formal (technical depth)
};

// Personality computed:
let personality = PersonalityProfile {
    confidence: 0.6 + (0.9 * 0.5) - (0.5 * 0.3),  // ≈ 0.75 (confident but cautious)
    explanation_depth: "high",  // Complexity suggests deeper explanation
    ask_clarifying_questions: true,  // Due to moderate confidence
};

liliana.publish_personality_update(&personality)?;
```

**Configuration Impact:**
- `persona.toml` → `explanation_depth_high_curiosity = "high"`, confidence formula

### Step 6: Beatrice Applies Personality

```rust
// beatrice/src/response_formatter.rs

let base_response = mimi.response;

// Apply personality modifiers:
// confidence = 0.75 → Add some hedging
let with_hedging = format!(
    "Há várias abordagens para isso. Uma estratégia comum (que recomendo) é:\n\n{}",
    base_response
);

// explanation_depth = "high" → Keep detailed explanation
// ask_clarifying_questions = true → Add questions at end
let with_questions = format!(
    "{}\n\n❓ Perguntas para considerar:\n- Qual é o seu padrão de acesso (read-heavy vs write-heavy)?\n- Há restrições de memória?",
    with_hedging
);

let styled_response = with_questions;
```

### Step 7: Odlaguna Validates

```rust
// odlaguna/src/security_gate.rs

let validation = odlaguna.validate_response(&styled_response)?;
// → Valid (no code injection, safe recommendations)

// Log Tier 3 execution
odlaguna.log_audit_event(AuditEvent {
    event_type: "tier3_execution",
    tier: 3,
    source: "mimi_full_pipeline",
    token_cost: 1050,
    context_tokens: 250_000,
    reasoning_tokens: 500,
})?;

// Update budget
budget.remaining_tokens -= 1050;
```

### Step 8: User Sees Response

```
Há várias abordagens para isso. Uma estratégia comum (que recomendo) é:

1. **TTL-based Invalidation:**
   - Cada entry tem timestamp
   - Background task verifica expiração
   - Tokens: O(1) lookup, O(n) background cleanup

2. **LRU Eviction:**
   - Doubly-linked list + HashMap
   - O(1) get, put, remove
   - Remove least-recently-used quando capacidade é excedida

3. **Combination (Hybrid):**
   - TTL para expiração automática
   - LRU para overflow
   - Implementação mais complexa, mas superior em prática

[... code examples and trade-offs ...]

❓ Perguntas para considerar:
- Qual é o seu padrão de acesso (read-heavy vs write-heavy)?
- Há restrições de memória?

[Metrics: Tier3, 2.3s, 1050 tokens, budget_remaining: 150K]
```

### Metrics Recorded

```json
{
  "request_id": "req-24680",
  "scenario": "complex_architecture",
  "tier_selected": 3,
  "latency_ms": 2300,
  "tokens_used": 1050,
  "context_tokens": 250_000,
  "reasoning_tokens": 500,
  "personality_version": 44,
  "confidence": 0.75,
  "explanation_depth": "high",
  "budget_remaining": 150_000,
  "timestamp": "2026-04-17T12:36:00Z"
}
```

---

## Scenario 4: Security Alert Hardening

**Event:** Odlaguna detects suspicious token usage spike

### Step 1: Security Alert Triggered

```rust
// odlaguna/src/security_monitor.rs

let metric = fetch_current_metric("tokens_used_last_minute");
// → 50_000 tokens (unusually high, baseline is ~5_000)

let is_anomaly = metric > baseline * 10;  // 10x spike

if is_anomaly {
    odlaguna.publish_security_alert(SecurityAlert {
        event_type: "token_usage_spike",
        severity: "high",
        description: "10x increase in token usage detected",
        timestamp: now(),
    })?;
}
```

### Step 2: Liliana Hardens Personality

```rust
// liliana/src/personality.rs

liliana.on_security_alert(alert)?;

// Automatically adjust mood:
let hardened_mood = MoodState {
    caution: 0.9,        // HIGH caution
    confidence: 0.4,     // LOW confidence
    urgency: 0.7,        // Increased urgency
    formality: 0.8,      // More formal
    curiosity: 0.3,      // Reduced (focus on safety)
};

// Compute hardened personality:
let hardened_personality = PersonalityProfile {
    confidence: 0.4 + (0.3 * 0.5) - (0.9 * 0.3),  // ≈ 0.09 (very tentative)
    caution: 0.9,
    code_style: "verbose",      // Add more safety comments
    humor_allowed: false,        // No humor during alerts
    ask_clarifying_questions: true,
};

liliana.publish_personality_update(&hardened_personality)?;
// → Version: 45 (previous was 44)
```

**Configuration Impact:**
- `persona.toml` → `security_hardening_duration_seconds = 300`, `security_recovery_gradual = true`

### Step 3: Odlaguna Validates Hardened Personality

```rust
// odlaguna/src/personality_validator.rs

let validation = odlaguna.validate_personality(&hardened_personality)?;

// Checks:
// 1. Bounds check: caution=0.9 ✓ (within 0.0-0.95)
// 2. Vocabulary safety: No injection ✓
// 3. Checksum: sha256(...) ✓
// 4. Rate limit: < 1 update/second ✓

// → PersonalityValidationResult {
//     valid: true,
//     applied_constraints: ["bounds_check", "checksum_verification"],
//   }
```

### Step 4: All Responses Hardened

**Next user request (even if innocuous like "Hi"):**

```rust
// beatrice/src/response_formatter.rs

let personality = beatrice.current_personality;  // v45 (hardened)

// Apply personality modifiers:
// formality = 0.8 → More formal
// confidence = 0.09 → Very tentative

let response = format!(
    "Olá. Neste momento, há preocupações de segurança ativas. \
     Posso ajudá-lo, mas recomendo cautela.\n\n\
     [Possível continuação da conversa...]"
);

// No emoji, more formal tone, add disclaimers
```

### Step 5: Recovery After Timeout

```rust
// liliana/src/personality.rs

// After security_hardening_duration_seconds = 300 (5 minutes):

let recovery_task = async {
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Gradual recovery: 1% per second
        let recovery_rate = 0.01;  // per second
        
        mood.caution -= recovery_rate;      // 0.9 → 0.89 → ...
        mood.confidence += recovery_rate;   // 0.4 → 0.41 → ...
        
        if elapsed > hardening_duration {
            // Fully recovered
            mood = baseline_mood;
            break;
        }
    }
};
```

**Timeline:**
- T=0s: Alert triggered, personality hardens (v45)
- T=150s: Mood gradually returning to normal (caution ≈ 0.45)
- T=300s: Full recovery, back to baseline personality (v46)

---

## Scenario 5: Budget Exhaustion and Deferral

**Situation:** After several Tier 3 requests, daily token budget is exhausted

### Step 1: Budget Alert

```rust
// gating-system/src/budget_manager.rs

let budget_percent = (used_tokens / daily_limit) * 100;
// → 95% of 1_000_000 = 950_000 tokens used

if budget_percent > 90 {
    budget_manager.alert("Budget alert: 95% exhausted");
}

if budget_percent > 95 {
    budget_manager.critical_alert("Budget nearly exhausted");
}
```

**Configuration Impact:**
- `gating.toml` → `budget.budget_alert_percent = [50, 75, 90]`

### Step 2: Request Arrives (Would Exceed Budget)

```rust
let new_request = "Explica como funcionam redes neurais profundas";

let intent = IntentParser::parse(new_request)?;
// Confidence: 0.50, complexity: high

let decision = router.route(&intent)?;

// Gating analysis:
// 1. Cache? NO
// 2. Skill? NO
// 3. Tier 3? YES, but...

let cost_estimate = 800;  // Typical Tier 3
let budget_remaining = 50_000;

if cost_estimate > budget_remaining {
    // DEFER
    decision = RoutingDecision::Defer {
        reason: "budget_exhausted",
        cost_would_be: cost_estimate,
        budget_remaining: budget_remaining,
    }
}
```

### Step 3: Deferred Response to User

```rust
// beatrice/src/cli.rs

match decision {
    RoutingDecision::Defer { reason, ... } => {
        let deferral_msg = if reason == "budget_exhausted" {
            format!(
                "Desculpe, estou com orçamento de tokens limitado neste momento.\n\
                 Tokens restantes: {}/1.000.000\n\n\
                 Sugestões:\n\
                 1. Tente uma pergunta mais simples\n\
                 2. Retorne em uma nova sessão (nova cota diária)\n\
                 3. Use cache/skills quando possível\n\n\
                 [Volta em ~{}h]",
                budget_remaining,
                hours_until_reset()
            )
        } else {
            "Desculpe, não consigo processar isso no momento.".to_string()
        };
        
        println!("{}", deferral_msg);
    }
}
```

### Step 4: Deferred Request Queued

```rust
// gating-system/src/deferral_queue.rs

deferred_queue.enqueue(DeferredRequest {
    request_id: "req-36912",
    intent: intent,
    timestamp: now(),
    session_id: session_id,
    reason: "budget_exhausted",
})?;

// Queue persisted to disk
// Max 100 deferred requests per session
```

### Step 5: Monitoring Alert

```rust
// Prometheus metrics
histogram!("deferred_requests_total", 1, "reason" => "budget_exhausted");
gauge!("budget_remaining_tokens", budget_remaining);
gauge!("budget_percent_used", 95);

// Alert to DevOps:
// "Budget exhaustion detected. 95% of daily limit used. 50K tokens remaining."
```

---

## System Metrics and Monitoring

### Dashboard KPIs

```
┌─────────────────────────────────────────────────────────┐
│ GATING SYSTEM PERFORMANCE DASHBOARD                     │
├─────────────────────────────────────────────────────────┤
│                                                         │
│ Tier Distribution (current session):                    │
│   Tier 1 (Cache):     ████████░░  32% (320 req)        │
│   Tier 2 (Skills):    █████████░░ 48% (480 req)        │
│   Tier 3 (Pipeline):  ██░░░░░░░░  15% (150 req)        │
│   Deferred:           █░░░░░░░░░   5% (50 req)         │
│                                                         │
│ Token Usage:          950K / 1.0M (95%)                 │
│ Cost Savings:         77% vs baseline                   │
│ Avg Latency:          285ms                             │
│                                                         │
│ Top Skills (success rate):                              │
│   code_generator         ████████████  96%             │
│   code_reviewer          ██████████░░  92%             │
│   documentation_writer   ██████████░░  91%             │
│                                                         │
│ Personality Updates:    145 (current v45, hardened)    │
│ Security Alerts:        2 (1 active hardening)         │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Metrics (Prometheus)

```prometheus
# Routing decisions
gating_routing_decision_total{tier="1"} 320
gating_routing_decision_total{tier="2"} 480
gating_routing_decision_total{tier="3"} 150
gating_routing_deferred_total 50

# Token usage
gating_tokens_used_total 950_000
gating_tokens_remaining 50_000
gating_tokens_budget_percent 95

# Performance
gating_tier1_latency_ms bucket=50 350
gating_tier2_latency_ms bucket=300 1200
gating_tier3_latency_ms bucket=3000 12500

# Skills
skill_execution_total{skill="code_generator",status="success"} 480
skill_execution_total{skill="code_generator",status="failure"} 19
skill_execution_success_rate{skill="code_generator"} 0.96

# Personality
personality_update_count 145
personality_version_current 45
personality_validation_failures 0
personality_tampering_alerts 0

# Budget
gating_budget_reset_hours_remaining 6
gating_deferred_requests_queued 50
```

### Observability

**Logging Example:**

```json
{
  "timestamp": "2026-04-17T12:36:45Z",
  "request_id": "req-36912",
  "component": "gating_system",
  "event": "routing_decision",
  "decision": "defer",
  "reasoning": "budget_exhausted",
  "cost_would_be": 800,
  "budget_remaining": 50000,
  "daily_limit": 1000000,
  "budget_percent_used": 95,
  "tier_attempted": 3,
  "fallback_tier": null,
  "session_id": "session-123",
  "user_id": "user-456"
}
```

---

## Summary: Full Integration

| Scenario | Tier | Latency | Tokens | Personality | Budget |
|----------|------|---------|--------|-------------|--------|
| **Scenario 1** | 1 | 45ms | 10 | v42 (normal) | 999,990 remaining |
| **Scenario 2** | 2 | 145ms | 120 | v43 (code-focused) | 999,870 remaining |
| **Scenario 3** | 3 | 2,300ms | 1,050 | v44 (complex) | 998,820 remaining |
| **Scenario 4** | - | - | - | v45 (hardened) | - |
| **Scenario 5** | Defer | - | 0 | v45 (hardened) | 50,000 remaining |

### Key Takeaways

1. **Liliana** drives personality → all responses are mood-aware
2. **Gating** optimizes cost → 77% token savings vs baseline
3. **Beatrice** applies styling → responses match current personality
4. **Odlaguna** ensures safety → validates both responses and personality
5. **Pandora** preserves state → mood trajectory and response history
6. **Budget management** prevents runaway costs → deferral when needed
7. **Security hardening** adapts automatically → personality tightens under alerts
8. **Observability** enables monitoring → metrics and logging throughout

