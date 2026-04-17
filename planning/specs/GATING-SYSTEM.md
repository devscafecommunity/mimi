# GATING-SYSTEM.md — Energy-Aware Task Activation Architecture

> **Specification:** Energy/Cost-aware hierarchical task routing  
> **Milestone:** M2 (Optimization) — M3+ (Refinement)  
> **Status:** 🟡 Design Complete — Implementation Pending  
> **References:** FrugalGPT, RouteLLM, OmniRouter, MasRouter  
> **Principle:** "Don't spin up the whole system for trivial requests"

---

## 1. Overview

The **Gating System** is a hierarchical, cost-aware task router that prevents MiMi from activating expensive cognitive modules (Priscilla, Pandora, Echidna) for requests that can be handled by cheaper, faster alternatives. It operates as a **middleware proxy** between user input (Beatrice) and the full pipeline, implementing a **3-tier cascade** with explicit cost-benefit decision logic.

### Core Principle

**Energy Minimization with Quality Guarantees:** Route every request to the tier that minimizes token/CPU cost while meeting quality thresholds for that request type.

### Budget Allocation (Example)

```
Daily Token Budget: 1,000,000 tokens

Tier 1 (Reflex):      100,000 tokens  (~10%) - Liliana cache, small-talk, status
Tier 2 (Automated):   300,000 tokens  (~30%) - Beatrice + Skill invocation
Tier 3 (Cognitive):   600,000 tokens  (~60%) - Full pipeline: Priscilla + Pandora + Echidna

Smart allocation: If Tier 1/2 saturate, deprioritize non-critical tasks (e.g., "tell me a joke") and reserve budget for critical tasks (e.g., "help me debug this code").
```

---

## 2. Architecture: 3-Tier Cascade Model

```
User Input (Beatrice)
    │
    ▼
┌─────────────────────────────────────────────────────────────┐
│              Gating System Decision Engine                  │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Intent Classification + Complexity Estimation             │
│  ├─ Is it trivial/social? (confidence > 0.7)              │
│  ├─ Does a skill exist? (cost estimate < reasoning cost?) │
│  └─ Is it novel/complex? (requires full reasoning)        │
│                                                             │
│  Budget Check:                                              │
│  ├─ Tier1_budget_remaining > token_estimate[tier1]?       │
│  ├─ Tier2_budget_remaining > token_estimate[tier2]?       │
│  └─ Tier3_budget_remaining > token_estimate[tier3]?       │
│                                                             │
│  Route Decision:                                            │
│  └─ Pick tier with best cost/quality ratio within budget   │
│                                                             │
└─────────────────────────────────────────────────────────────┘
    │         │               │
    ▼         ▼               ▼
  ┌──────┐ ┌──────────────┐ ┌─────────────────┐
  │Tier1 │ │  Tier 2      │ │   Tier 3        │
  │Reflex│ │ Automated    │ │  Cognitive      │
  │ Mode │ │ Skill + LLM  │ │  Full Pipeline  │
  │      │ │              │ │                 │
  │Cost: │ │  Cost:       │ │  Cost:          │
  │~0    │ │  ~50 tokens  │ │  ~500+ tokens   │
  │      │ │              │ │                 │
  │Time: │ │  Time:       │ │  Time:          │
  │<50ms │ │  <200ms      │ │  1-5 sec        │
  └──────┘ └──────────────┘ └─────────────────┘
    │         │               │
    └─────────┴───────────────┘
              │
              ▼
    ┌──────────────────────┐
    │ Odlaguna Gate        │
    │ (Safety & Continuity)│
    └──────────────────────┘
              │
              ▼
         User Response
```

---

## 3. Tier Definitions

### Tier 1: Reflex (Fast Path, Cache-First)

**Activation:** Liliana mode for trivial/social requests

| Property | Value |
|---|---|
| **Modules Active** | Liliana (mood + cache) |
| **Token Cost** | ~0–5 tokens (cache lookup only) |
| **Latency** | < 50ms |
| **Complexity Threshold** | `trivial` \| `social` |
| **Examples** | "Hi Mimi", "How are you?", "What time is it?" |
| **Success Rate Target** | > 90% (confidence in cached responses) |
| **Daily Budget Allocation** | ~10% of token budget |

**Gating Logic:**

```
IF request.intent in [social/greet, social/status, social/small_talk]
   AND liliana.cache_hit(request) == TRUE
   AND liliana.cache_confidence >= 0.7
THEN route to Tier 1
ELSE try Tier 2
```

**Cost Formula:**

```
Cost_Tier1 = 0  (cache hit)
           = 50  (template rendering + Odlaguna gate)
Cost_Tier1_MISS = 100  (fallback to Tier 2)
```

### Tier 2: Automated (Medium Path, Skill-Based)

**Activation:** Beatrice + Echidna skill invocation for structured, repetitive tasks

| Property | Value |
|---|---|
| **Modules Active** | Beatrice + Echidna (skill lookup & execution) |
| **Token Cost** | ~30–100 tokens (skill I/O + Beatrice parsing) |
| **Latency** | < 200ms |
| **Complexity Threshold** | `simple` \| `moderate` (NOT complex) |
| **Examples** | "How many repos do I have?", "List my TODOs", "What's the time in UTC?" |
| **Success Rate Target** | > 85% (skill exists and returns valid result) |
| **Daily Budget Allocation** | ~30% of token budget |

**Gating Logic:**

```
IF request.intent in [task/query, task/extraction, task/formatting]
   AND echidna.skill_exists(request.task_type) == TRUE
   AND cost_model.estimate_skill_cost(skill) < cost_model.estimate_reasoning_cost(request)
   AND Tier2_budget_remaining >= estimated_cost
THEN route to Tier 2
ELSE try Tier 3
```

**Cost Formula:**

```
Cost_Tier2 = Beatrice_NLU_tokens (usually 10–20)
           + Echidna_skill_tokens (depends on skill; 20–80)
           
Estimate: cost_model.skill_cost(task, args) = default_skill_cost + len(args) * arg_multiplier
          e.g., if skill.default_cost = 40, and args have 2 entities, cost ≈ 50 tokens
```

**Skill Availability Check:**

```rust
// In Echidna module
pub struct SkillRegistry {
    skills: HashMap<String, SkillMetadata>,
}

impl SkillRegistry {
    pub fn find_skill(&self, task_type: &str) -> Option<&SkillMetadata> {
        // Return skill if: 
        //   1. Exists in registry
        //   2. Passes security validation (Odlaguna approved)
        //   3. Is not in "failing" state (circuit breaker)
        self.skills.get(task_type).filter(|s| s.is_available())
    }
    
    pub fn estimate_cost(&self, skill: &SkillMetadata, args: &HashMap<String, String>) -> usize {
        skill.default_cost + args.len() * 5  // 5 tokens per argument
    }
}
```

### Tier 3: Cognitive (Full Path, Reasoning)

**Activation:** Full pipeline for novel, complex, multi-step reasoning tasks

| Property | Value |
|---|---|
| **Modules Active** | Priscilla (critique) + Pandora (memory) + Echidna (planning) + LLM reasoning |
| **Token Cost** | ~500–5000+ tokens (full chain-of-thought) |
| **Latency** | 1–10 seconds |
| **Complexity Threshold** | `complex` \| `novel` |
| **Examples** | "Write me a blog post about AI safety", "Help me architect a microservices system", "Explain quantum entanglement" |
| **Success Rate Target** | > 70% (reasoning quality; not all tasks solvable) |
| **Daily Budget Allocation** | ~60% of token budget |

**Gating Logic:**

```
IF request.complexity in [complex, novel]
   OR (Tier2_budget_exhausted AND Tier3_budget_remaining > 0)
   OR beatrice.confidence < 0.3  (ambiguous intent)
THEN route to Tier 3
ELSE defer or queue request
```

**Cost Formula:**

```
Cost_Tier3 = Priscilla_analysis_tokens (50–100)
           + Pandora_retrieval_tokens (100–300, context-dependent)
           + Echidna_planning_tokens (50–200)
           + LLM_reasoning_tokens (300–4500, depends on chain-of-thought depth)
           
Estimate: cost_model.reasoning_cost(prompt) = base_cost + len(prompt) * token_multiplier + retrieval_tokens
          e.g., 50 + len("Write me a blog post") * 0.15 + 150 ≈ 300 tokens baseline
```

---

## 4. Decision Logic & Routing Algorithm

### Primary Routing Function

```rust
pub fn route_request(
    request: &UserRequest,
    liliana: &LilianaCache,
    echidna: &SkillRegistry,
    budget: &BudgetManager,
    cost_model: &CostModel,
) -> RoutingDecision {
    
    // Step 1: Classify intent + complexity
    let intent = beatrice.classify_intent(request);  // Returns (intent_type, confidence)
    let complexity = beatrice.estimate_complexity(request);  // Returns "trivial" | "simple" | "moderate" | "complex"
    
    // Step 2: Check Tier 1 (Reflex)
    if intent.is_social() && complexity == "trivial" {
        if let Some(cached) = liliana.lookup(request) {
            if cached.confidence >= 0.7 && budget.tier1_remaining() > 50 {
                return RoutingDecision::Tier1(cached);
            }
        }
    }
    
    // Step 3: Check Tier 2 (Automated)
    if complexity in ["simple", "moderate"] {
        if let Some(skill) = echidna.find_skill(&intent.task_type) {
            let skill_cost = cost_model.estimate_skill_cost(skill, request);
            let reasoning_cost = cost_model.estimate_reasoning_cost(request);
            
            if skill_cost < reasoning_cost && budget.tier2_remaining() >= skill_cost {
                // Cost-benefit: skill is cheaper than reasoning
                return RoutingDecision::Tier2(skill);
            }
        }
    }
    
    // Step 4: Check Tier 3 (Cognitive) with budget constraints
    if budget.tier3_remaining() > cost_model.estimate_reasoning_cost(request) {
        return RoutingDecision::Tier3;
    }
    
    // Step 5: Fallback (budget exhausted or unclassifiable)
    return RoutingDecision::Defer(reason);
}
```

### Cost-Benefit Comparison Example

```
Request: "How many repos do I have?"

Step 1: Classify intent
  Intent: task/query (confidence=0.8)
  Complexity: simple
  
Step 2: Check Tier 1
  Liliana cache hit? No (new query variant)
  Skip Tier 1
  
Step 3: Check Tier 2
  Skill exists? Yes (repo_list_skill)
  Skill cost estimate: base_cost(40) + args(2)*5 = 50 tokens
  Reasoning cost estimate: base_cost(50) + prompt_len(25)*0.1 + retrieval(100) = 160 tokens
  Cost-benefit: 50 < 160? YES ✓
  Budget remaining (Tier 2): 250,000 tokens
  50 < 250,000? YES ✓
  
Decision: Route to Tier 2
  Execution: Beatrice parses → Echidna runs repo_list_skill → 45 tokens used
  Response: "You have 8 repos" [latency: 120ms]
  
Tokens saved: 160 - 45 = 115 tokens vs. Tier 3 reasoning
```

---

## 5. Token Budget Management

### Budget Allocation & Tracking

```rust
pub struct BudgetManager {
    tier1_daily_limit: usize,      // e.g., 100,000 tokens
    tier2_daily_limit: usize,      // e.g., 300,000 tokens
    tier3_daily_limit: usize,      // e.g., 600,000 tokens
    
    tier1_used_today: Arc<Mutex<usize>>,
    tier2_used_today: Arc<Mutex<usize>>,
    tier3_used_today: Arc<Mutex<usize>>,
    
    tier1_priority_queue: PriorityQueue<Task>,  // High-priority tasks use budget first
    tier2_priority_queue: PriorityQueue<Task>,
    tier3_priority_queue: PriorityQueue<Task>,
}

impl BudgetManager {
    pub fn reserve_budget(&self, tier: u8, estimated_tokens: usize) -> Result<Token> {
        match tier {
            1 => {
                let mut used = self.tier1_used_today.lock();
                if *used + estimated_tokens <= self.tier1_daily_limit {
                    *used += estimated_tokens;
                    Ok(Token::new(tier, estimated_tokens))
                } else {
                    Err("Tier 1 budget exhausted")
                }
            },
            2 => { /* similar */ },
            3 => { /* similar */ },
            _ => Err("Invalid tier"),
        }
    }
    
    pub fn release_budget(&self, token: Token, actual_tokens: usize) {
        // Adjust actual token usage (refund if overestimated)
        let mut used = match token.tier {
            1 => self.tier1_used_today.lock(),
            // ...
        };
        *used = (*used as isize - (token.estimated as isize - actual_tokens as isize)).max(0) as usize;
    }
    
    pub fn remaining(&self, tier: u8) -> usize {
        let used = match tier {
            1 => *self.tier1_used_today.lock(),
            2 => *self.tier2_used_today.lock(),
            3 => *self.tier3_used_today.lock(),
            _ => 0,
        };
        match tier {
            1 => self.tier1_daily_limit.saturating_sub(used),
            2 => self.tier2_daily_limit.saturating_sub(used),
            3 => self.tier3_daily_limit.saturating_sub(used),
            _ => 0,
        }
    }
}
```

### Budget Reset & Alerts

```
Daily Reset (UTC midnight):
  - tier1_used_today = 0
  - tier2_used_today = 0
  - tier3_used_today = 0

Alert Thresholds:
  - Tier 1 @ 90% used: WARN (non-critical requests queued)
  - Tier 2 @ 90% used: WARN (defer non-urgent tasks to next day)
  - Tier 3 @ 90% used: CRITICAL (activate hard limits; only critical reasoning allowed)
  
Graceful Degradation:
  - If Tier 3 budget exhausted: all non-critical requests DEFER ("Please try again later")
  - If all tiers exhausted: return cached responses or generic fallback
```

---

## 6. Cost Estimation Model

### Token Price Reference (Example: Claude Opus 4.6)

```
Input:  $5 / 1M tokens  (0.000005 per token)
Output: $25 / 1M tokens (0.000025 per token)

Batch discount: 50% off (use for non-real-time tasks)
```

### Cost Estimation Formulas

```
Cost per request = (input_tokens × input_price) + (output_tokens × output_price)

Example calculations:

Tier 1 (Liliana cache):
  Input: 50 tokens (lookup + template rendering)
  Output: 20 tokens (response)
  Cost = (50 × 0.000005) + (20 × 0.000025) = $0.0005 (essentially free)

Tier 2 (Beatrice + Skill):
  Input: 40 tokens (Beatrice NLU)
  Output: 10 tokens (skill result)
  Cost = (40 × 0.000005) + (10 × 0.000025) = $0.00045

Tier 3 (Full reasoning):
  Input: 200 tokens (prompt + context + history)
  Output: 500 tokens (chain-of-thought + answer)
  Cost = (200 × 0.000005) + (500 × 0.000025) = $0.0135

Savings example:
  Routing 1000 "How many repos?" queries:
  - Via Tier 3: 1000 × $0.0135 = $13.50
  - Via Tier 2: 1000 × $0.00045 = $0.45
  - Savings: $13.05 (97% cost reduction!)
```

### Predictive Cost Model

```rust
pub struct CostModel {
    base_costs: HashMap<String, usize>,  // base token cost per tier
    confidence_scores: HashMap<String, f32>,  // confidence in cost estimate
}

impl CostModel {
    pub fn estimate_skill_cost(&self, skill: &SkillMetadata, args: &HashMap<String, String>) -> usize {
        self.base_costs.get("skill").copied().unwrap_or(40)
            + args.len() * 5
    }
    
    pub fn estimate_reasoning_cost(&self, prompt: &str) -> usize {
        let base = self.base_costs.get("reasoning").copied().unwrap_or(50);
        let prompt_tokens = (prompt.len() / 4) as usize;  // rough estimate: 1 token ≈ 4 chars
        base + (prompt_tokens / 2)  // system prompts cost extra; divide by 2 to balance
    }
    
    pub fn confidence_in_estimate(&self, tier: &str) -> f32 {
        // Tier 1: high confidence (cache is deterministic)
        // Tier 2: medium confidence (skill cost varies)
        // Tier 3: low confidence (LLM output length unpredictable)
        self.confidence_scores.get(tier).copied().unwrap_or(0.5)
    }
}
```

---

## 7. Early Exit & Circuit Breaker Patterns

### Circuit Breaker for Failing Skills

```rust
pub enum CircuitState {
    Closed,    // Skill is healthy, requests flow normally
    Open,      // Skill is failing, requests redirected to Tier 3
    HalfOpen,  // Skill is recovering, allow trial requests
}

pub struct SkillCircuitBreaker {
    state: Arc<Mutex<CircuitState>>,
    failure_count: Arc<Mutex<usize>>,
    success_count: Arc<Mutex<usize>>,
    failure_threshold: usize,  // e.g., 3 consecutive failures
    recovery_timeout: Duration,
    last_failure_time: Arc<Mutex<Instant>>,
}

impl SkillCircuitBreaker {
    pub fn can_invoke(&self) -> bool {
        let state = *self.state.lock();
        match state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if recovery timeout expired
                if self.last_failure_time.lock().elapsed() > self.recovery_timeout {
                    *self.state.lock() = CircuitState::HalfOpen;
                    true  // Allow trial request
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => true,  // Allow trial request
        }
    }
    
    pub fn record_success(&self) {
        let mut count = self.success_count.lock();
        *count += 1;
        
        if *count >= 2 {
            *self.state.lock() = CircuitState::Closed;
            *self.failure_count.lock() = 0;
            *count = 0;
        }
    }
    
    pub fn record_failure(&self) {
        let mut count = self.failure_count.lock();
        *count += 1;
        *self.last_failure_time.lock() = Instant::now();
        
        if *count >= self.failure_threshold {
            *self.state.lock() = CircuitState::Open;
            *self.success_count.lock() = 0;
        }
    }
}
```

### Early Exit for Uncertain Intents

```
If Beatrice confidence < 0.3 (intent ambiguous):
  - Try Tier 1 (may have cached similar query)
  - If Tier 1 miss, ask user for clarification
  - Collect user's clarifying response as training signal
  - Retry with clarified intent
```

---

## 8. Observability & Metrics

### Key Performance Indicators

```prometheus
# Tier routing distribution
gating_requests_routed_to_tier{tier="1"}  # counter
gating_requests_routed_to_tier{tier="2"}  # counter
gating_requests_routed_to_tier{tier="3"}  # counter

# Cost tracking
gating_tokens_used_daily{tier="1"}        # gauge
gating_tokens_used_daily{tier="2"}        # gauge
gating_tokens_used_daily{tier="3"}        # gauge
gating_cost_usd_daily{tier="1"}           # gauge (cumulative cost)
gating_cost_usd_daily{tier="2"}           # gauge
gating_cost_usd_daily{tier="3"}           # gauge

# Budget utilization
gating_tier_budget_utilization{tier="1"}  # gauge: 0-1 (percent of daily limit used)
gating_tier_budget_utilization{tier="2"}  # gauge
gating_tier_budget_utilization{tier="3"}  # gauge

# Quality metrics
gating_tier_success_rate{tier="1"}        # gauge: 0-1 (% of requests completed successfully)
gating_tier_success_rate{tier="2"}        # gauge
gating_tier_success_rate{tier="3"}        # gauge

# Latency
gating_tier_latency_ms{tier="1", p="50"}  # histogram: p50, p95, p99
gating_tier_latency_ms{tier="2", p="50"}  # histogram
gating_tier_latency_ms{tier="3", p="50"}  # histogram

# Skill-specific metrics
gating_skill_invocations{skill="repo_list"}     # counter
gating_skill_failures{skill="repo_list"}        # counter
gating_skill_circuit_breaker_trips{skill}       # counter (Open state)

# Tier 3 quality (only for expensive reasoning)
gating_tier3_chain_of_thought_length{percentile="p95"}  # histogram: tokens in reasoning
gating_tier3_hallucination_rate                         # gauge: % of tier3 responses with detected hallucinations
```

### Dashboard Layout

```
┌──────────────────────────────────────────────────────────────────┐
│ MiMi Gating System Dashboard                                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Budget Status (Today):                                          │
│   Tier 1: 87% used (87K / 100K tokens) [████████░]  WARN       │
│   Tier 2: 65% used (195K / 300K tokens) [██████░░░]            │
│   Tier 3: 42% used (252K / 600K tokens) [████░░░░░]            │
│                                                                  │
│ Routing Distribution (Last 1 hour):                             │
│   Tier 1: 350 requests (58%) @ avg 1.2ms                        │
│   Tier 2: 180 requests (30%) @ avg 95ms                         │
│   Tier 3: 70 requests (12%) @ avg 2.3s                          │
│                                                                  │
│ Cost Breakdown (Today):                                         │
│   Tier 1: $0.001  (cache hits, essentially free)                │
│   Tier 2: $0.15   (skill invocations)                           │
│   Tier 3: $3.24   (full reasoning)                              │
│   Total: $3.40                                                  │
│                                                                  │
│ Quality Metrics:                                                │
│   Tier 1 Success: 93% (cached responses appropriate)            │
│   Tier 2 Success: 82% (skills executed correctly)               │
│   Tier 3 Success: 71% (reasoning generated valid output)        │
│                                                                  │
│ Circuit Breaker Status:                                         │
│   repo_list_skill:      [OK]                                    │
│   summarize_docs_skill: [⚠ HALF_OPEN] (recovering)             │
│   code_review_skill:    [ERROR] (will retry at 14:45)           │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 9. Integration with MiMi Modules

### Gating ↔ Beatrice

- Beatrice classifies intent; Gating decides routing
- Beatrice provides confidence score; Gating uses it to break ties
- If Beatrice confidence low, Gating may ask for clarification or defer

### Gating ↔ Liliana

- Liliana caches Tier 1 responses; Gating routes to Liliana for social queries
- Liliana mood state may influence confidence threshold (e.g., if frustrated, tighten threshold)

### Gating ↔ Echidna

- Gating queries Echidna's skill registry for availability
- Echidna provides cost estimates; Gating uses for routing decisions
- Gating monitors Echidna's circuit breaker state

### Gating ↔ Priscilla/Pandora

- Gating only activates Priscilla/Pandora for Tier 3 (complex reasoning)
- Priscilla critiques Tier 3 routing decisions (e.g., "This should have been Tier 2")
- Pandora stores routing decisions + outcomes for future optimization

---

## 10. Configuration

```toml
[gating]
# Tier definitions
tier1_daily_limit = 100000  # tokens
tier2_daily_limit = 300000
tier3_daily_limit = 600000

# Cost model parameters
cost.skill_default = 40  # base tokens
cost.skill_per_arg = 5
cost.reasoning_base = 50
cost.reasoning_per_prompt_char = 0.1

# Intent confidence thresholds
intent.social_threshold = 0.3      # Rasa-style: if confidence >= 0.3, treat as social
intent.high_confidence_threshold = 0.7   # Use Tier 1/2 only if confidence >= 0.7
intent.ambiguous_threshold = 0.3   # If confidence < 0.3, ask clarification

# Skill cost-benefit
skill.cost_multiplier = 1.0  # adjust estimated skill costs
skill.reasoning_cost_multiplier = 1.2  # add buffer to reasoning estimates for conservatism

# Circuit breaker
circuit_breaker.failure_threshold = 3    # consecutive failures before opening
circuit_breaker.recovery_timeout_sec = 300  # 5 minutes before HalfOpen trial
circuit_breaker.success_threshold = 2    # consecutive successes to close

# Logging & alerts
alert.tier_utilization_threshold = 0.9   # warn when tier > 90% used
alert.cost_daily_limit_usd = 10.0        # warn if daily cost exceeds $10
alert.skill_failure_rate = 0.15          # warn if skill fails > 15% of times
```

---

## 11. Real-World Examples

### Example 1: Trivial Request (Tier 1)

```
User: "What's the weather today?"
System Time: 2026-04-17 14:32:15

Gating Flow:
1. Classify: intent=task/weather, confidence=0.65
2. Check complexity: Not social, but trivial query
3. Route attempt Tier 1? No (weather needs current data; not pure cache)
4. Route attempt Tier 2? Yes (weather_skill exists)
   - Skill cost: 40 + 0 args = 40 tokens
   - Reasoning cost: 50 + len("What's the weather today?")/40 * 10 ≈ 56 tokens
   - Decision: 40 < 56? YES → use Tier 2
5. Execute: Beatrice NLU (15 tokens) + weather_skill (35 tokens) = 50 tokens
6. Response: "Sunny, 72°F, 65% humidity" [latency: 120ms]
7. Update metrics: tier2_requests++, tier2_tokens_used += 50
```

### Example 2: Complex Request (Tier 3)

```
User: "Write me a blog post about the future of AI in healthcare"
System Time: 2026-04-17 15:45:30

Gating Flow:
1. Classify: intent=task/generation, confidence=0.72
2. Check complexity: estimated_complexity = "complex" (length + novelty)
3. Route attempt Tier 1? No (not social/trivial)
4. Route attempt Tier 2? Skill exists? No (blog generation not a pre-built skill)
5. Route to Tier 3:
   - Estimated cost: 50 (base) + len(prompt)/4 * 0.1 + 200 (retrieval) ≈ 300 tokens
   - Tier 3 remaining: 450,000 tokens
   - 300 < 450,000? YES → route to Tier 3
6. Execute full pipeline:
   - Priscilla critique (75 tokens)
   - Pandora retrieval (180 tokens)
   - LLM reasoning + generation (1,200 tokens)
   - Total: 1,455 tokens
7. Response: 3-section blog post [latency: 3.2 seconds]
8. Update metrics: tier3_requests++, tier3_tokens_used += 1455, tier3_cost_usd += $0.045
```

### Example 3: Ambiguous Request (Ask for Clarification)

```
User: "Can you help?"
System Time: 2026-04-17 16:10:00

Gating Flow:
1. Classify: intent=unknown, confidence=0.2 (too low!)
2. Intent below threshold (0.3) → ambiguous
3. Early exit: Ask user for clarification instead of burning tokens
4. Response: "I'd love to help! Could you be more specific about what you need?"
5. No tokens consumed (fallback to Tier 1 template)
6. Update metrics: ambiguous_requests++
```

---

## 12. Expected Outcomes & Metrics

### Cost Savings (Benchmark)

Based on FrugalGPT + RouteLLM patterns:

```
Distribution of requests (typical):
- 50% social/trivial (Tier 1): ~0 tokens average
- 30% structured tasks (Tier 2): ~60 tokens average
- 20% complex reasoning (Tier 3): ~800 tokens average

Without Gating (all Tier 3):
  Average cost = 800 tokens/request
  Daily cost (1000 requests): 800,000 tokens = $20

With Gating:
  Average cost = (0.5 × 0) + (0.3 × 60) + (0.2 × 800) = 178 tokens/request
  Daily cost (1000 requests): 178,000 tokens = $4.45
  
Savings: 77% reduction in token usage, 78% reduction in cost
```

### Quality Impact

```
Tier 1 (Cache):      Success rate 93%, Latency 35ms (fast, reliable)
Tier 2 (Skill):      Success rate 82%, Latency 120ms (medium, skill-dependent)
Tier 3 (Reasoning):  Success rate 71%, Latency 2500ms (slow, complex tasks)

Overall quality maintained because:
  - Simple requests routed to fast/cheap paths (Tier 1/2)
  - Complex requests routed to expensive/capable paths (Tier 3)
  - Success rates matched to request complexity
```

---

## 13. References

- **FrugalGPT:** Cascade routing across multiple models for 98% cost reduction. https://arxiv.org/abs/2305.05176
- **RouteLLM:** Threshold-based routing between strong/weak models. https://github.com/lm-sys/RouteLLM
- **OmniRouter:** Constrained optimization for cost-performance routing. https://arxiv.org/abs/2502.20576
- **MasRouter:** Multi-agent system routing with collaboration modes. https://aclanthology.org/2025.acl-long.757/
- **BranchyNet:** Early exit strategies for neural networks. https://arxiv.org/abs/1709.01686
- **Token Budget Strategies:** Production patterns for token budgeting middleware. https://tianpan.co/blog/2025-10-20-token-budget-strategies-llm-production

---

## 14. Acceptance Criteria

- [ ] Gating router implemented with 3-tier decision logic
- [ ] Cost estimation model matches real Claude pricing (< 10% error)
- [ ] Skill registry integration with Echidna working
- [ ] Budget tracking accurate (token usage within 5% of actual)
- [ ] Circuit breaker for skills functioning (failures trigger Open state)
- [ ] Tier 1 success rate > 90%
- [ ] Tier 2 success rate > 85%
- [ ] Tier 3 success rate > 70%
- [ ] Overall cost reduction > 70% vs. all-Tier-3 baseline
- [ ] Dashboard rendering all KPIs correctly
- [ ] Integration with Beatrice/Liliana/Echidna/Odlaguna passing tests
- [ ] Documentation complete (this spec + inline comments)

