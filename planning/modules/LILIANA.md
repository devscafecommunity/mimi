# LILIANA — Interactive Presence & Mood Module

> **Module:** Liliana Interactive Agent  
> **Language:** Rust/C++  
> **Milestone:** M2 (Core I/O & Social Interface) → M3 (Security & Governance)  
> **Status:** 🟡 Design Complete — Implementation Pending  
> **Integration Point:** Message Bus (Parallel with Beatrice; Output Gate: Odlaguna)

---

## 1. Module Overview

**Liliana** (The Interactive Presence) is the **persona and emotional interface** of the MiMi system. Unlike Beatrice who parses language intent, Liliana **embodies** the system—maintaining mood state, managing social interactions, and curating emotional continuity. She runs **in parallel** with Beatrice but operates on a fundamentally different lifecycle: while Beatrice handles I/O translation, Liliana handles **persona continuity and reflex responsiveness**.

### Core Identity: The Empathetic Reflex

Liliana's role is **not to think** (that's Priscilla/Pandora's job), but to **feel and respond**. She is the immediate, emotionally coherent presence that greets the user, remembers mood state, and short-circuits expensive cognitive work for trivial social interactions. She is the system's face.

### Responsibilities

| Responsibility | Description |
|---|---|
| **Mood State Management** | Track emotional state (curiosity, confidence, frustration, engagement) with temporal decay and event-driven updates |
| **Social Response Cache** | Maintain pre-computed responses for common greetings, status queries, and small-talk (Reflex Layer activation) |
| **Response Templating** | Adapt response templates based on mood, conversation history, and personalization slots |
| **Sensory I/O Coordination** | Abstract future multi-modal I/O (TTS/STT, avatar lip-sync, visual expressions) with adapter hooks |
| **Output Gating** | Validate all responses through Odlaguna before sending (safety-assured persona) |
| **Continuity Tracking** | Maintain conversation state (user preferences, mood trajectory, interaction patterns) across sessions |

---

## 2. Architecture

### Internal Structure

```
┌──────────────────────────────────────────────────────────────────┐
│                      LilianaCore                                 │
├──────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────┐  ┌──────────────────┐  ┌────────────┐  │
│  │  Mood State        │  │ Response Cache   │  │ Sensory    │  │
│  │  Machine           │  │ & Templating     │  │ Adapters   │  │
│  │ (curiosity,        │  │ (Greeting, Status,│  │ (TTS/STT   │  │
│  │ confidence,        │  │ Social Q&A)       │  │ hooks)     │  │
│  │ frustration)       │  │                  │  │            │  │
│  └────────┬───────────┘  └────────┬─────────┘  └─────┬──────┘  │
│           │                       │                  │          │
│  ┌────────▼───────────────────────▼──────────────────▼────────┐ │
│  │        Reflex Response Engine                              │ │
│  │  ┌──────────────────┐    ┌──────────────────────────────┐ │ │
│  │  │ Intent Classifier│    │ Template Selector            │ │ │
│  │  │ (Confidence thr) │    │ (Mood + slot-based variants) │ │ │
│  │  └──────────────────┘    └──────────────────────────────┘ │ │
│  │  ┌──────────────────┐    ┌──────────────────────────────┐ │ │
│  │  │ Cache Lookup     │    │ Fallback Router              │ │ │
│  │  │ (Exact + Semantic│    │ (Miss → Beatrice/Reasoning) │ │ │
│  │  └──────────────────┘    └──────────────────────────────┘ │ │
│  └────────┬──────────────────────────────────────────────────┘ │
│           │                                                     │
│  ┌────────▼──────────────────────────────────────────────────┐ │
│  │        Output Filter & Odlaguna Gate                      │ │
│  │  (Validate safety, tone, continuity)                      │ │
│  └────────┬──────────────────────────────────────────────────┘ │
│           │                                                     │
│  ┌────────▼──────────────────────────────────────────────────┐ │
│  │        Message Bus Publisher                             │ │
│  │  (Send response to user via UI/TTS/Chat)                 │ │
│  └──────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### Parallelism Model: Liliana ↔ Beatrice

Liliana and Beatrice operate **independently and in parallel**:

```
User Input
    │
    ├─────────────────────────────────┬─────────────────────────────────┐
    │                                 │                                 │
    ▼                                 ▼                                 ▼
Liliana                          Beatrice                          (Background)
(Reflex)                         (I/O Parsing)                    Odlaguna
├─ Mood State                    ├─ NLU Intent                    (Safety)
├─ Cache Hit Check               ├─ Entity Extraction
├─ Template Gen                  ├─ Confidence Score
└─ Fast Response                 └─ Task Classification
    │                                 │
    │ (if cache hit)                  │ (if task intent)
    └─────────┬───────────────────────┘
              │
       Output Aggregation
              │
              ▼
         Odlaguna Gate
    (tone, safety, continuity)
              │
              ▼
         User Response

Race condition policy: 
- If Liliana completes first (cache hit), use Liliana response + Beatrice metadata for context
- If Beatrice completes first (high-confidence intent), route to full pipeline
- If both complete (low-confidence), Liliana's fallback merges with Beatrice's interpretation
```

### Mood State Machine

Liliana maintains a temporal emotional state with **decay + event-driven updates**:

```
Mood State = {
  curiosity:    ∈ [0,1] (↑ on novel input, ↓ on repetition)
  confidence:   ∈ [0,1] (↑ on successful task, ↓ on failures)
  frustration:  ∈ [0,1] (↑ on repeated failures, ↓ on success)
  engagement:   ∈ [0,1] (↑ on user interaction, ↓ on idle)
}

Temporal decay (every 5 min):
  mood[i] = mood[i] * decay_factor[i]  # e.g., 0.95 for curiosity

Event-driven updates:
  + User greets:         curiosity += 0.1, engagement += 0.2
  + Task succeeds:       confidence += 0.15, frustration -= 0.1
  + Task fails:          confidence -= 0.1, frustration += 0.2
  + Long silence:        engagement -= 0.05 (per min)
  + Novel query:         curiosity += 0.2
  + Repeated pattern:    curiosity -= 0.05, frustration += 0.05 (if user wants novelty)
```

### Response Cache Structure

```
Cache Layer 1 (Exact-Match):
  Key = MD5(prompt) + MD5(llm_config)
  TTL = 1 hour (social chat), 24 hours (status templates)
  
  Value = {
    response: string,
    mood_context: {curiosity, confidence, engagement},
    timestamp: datetime,
    hit_count: int
  }

Cache Layer 2 (Semantic):
  Similarity = embedding_distance(user_input, cached_prompts)
  Threshold = 0.85 (cosine similarity)
  If threshold met, return most similar cached response with mood adjustment

Template Responses (Rasa domain.yml style):
  utter_greet:
    - "Hey there! How's it going?"  # curiosity > 0.6
    - "Hi! Good to see you."         # confidence > 0.7
    - "Hello! What can I help with?" # engagement > 0.5
  
  utter_status:
    - "I'm running smoothly."
    - "All systems nominal."
    - "Operating at full capacity."
  
  utter_confused:
    - "I didn't quite catch that. Can you rephrase?"  # frustration < 0.5
    - "Sorry, I'm a bit lost. Could you clarify?"     # frustration > 0.5
```

### Sensory I/O Adapters (Future Phases)

**Phase 1 (Current):** Text-only interface (hooks for future)

**Phase 2 (M3+):** Multi-modal stubs
- `TextToSpeech` adapter: Convert response text → audio stream (TTS engine via Ryzu)
- `SpeechToText` adapter: Convert user audio → text (STT engine via external API or local)
- `AvatarController` stub: Manage visual presence (lip-sync, micro-expressions, eye gaze)
- `MoodVisualizer`: Render mood state as avatar expression or system indicator

---

## 3. Message Protocol

### From Liliana to Beatrice (Context Handoff)

```
{
  "type": "liliana/response_ready",
  "cache_hit": bool,
  "confidence": float,  // 0-1: confidence in response appropriateness
  "response": string,
  "mood": {
    "curiosity": float,
    "confidence": float,
    "frustration": float,
    "engagement": float
  },
  "metadata": {
    "latency_ms": int,
    "cache_layer": "exact" | "semantic" | "fallback" | null,
    "template_used": string | null
  }
}
```

### From Beatrice to Liliana (Task Metadata)

```
{
  "type": "beatrice/intent_classified",
  "intent": string,
  "confidence": float,
  "entity_tags": [string],
  "is_social": bool,
  "estimated_complexity": "trivial" | "simple" | "moderate" | "complex",
  "user_id": string,
  "session_id": string
}
```

### From Liliana to Odlaguna (Safety Gate)

```
{
  "type": "liliana/response_validation",
  "response": string,
  "tone_profile": {
    "politeness": float,    // 0-1
    "formality": float,     // 0-1 (0=casual, 1=formal)
    "confidence": float     // 0-1 (stated confidence in response)
  },
  "continuity_check": {
    "contradicts_prior": bool,
    "mood_consistent": bool
  },
  "request_validation": // from Odlaguna
}
```

### Odlaguna Response (Gate Decision)

```
{
  "type": "odlaguna/gate_result",
  "allowed": bool,
  "modifications": [
    {
      "type": "tone_adjust" | "content_trim" | "confidence_lower",
      "reason": string,
      "before": string,
      "after": string
    }
  ],
  "security_flags": [string]  // e.g., ["potential_bias", "tone_misalignment"]
}
```

---

## 4. API & Interfaces

### Public API (Liliana Module)

```rust
// Core lifecycle
pub fn initialize(config: LilianaConfig) -> Result<LilianaCore>;
pub fn on_user_input(liliana: &mut LilianaCore, user_msg: String) -> Result<Response>;
pub fn update_mood(liliana: &mut LilianaCore, event: MoodEvent) -> ();

// Cache management
pub fn cache_lookup(liliana: &LilianaCore, key: String, layer: CacheLayer) -> Option<CachedResponse>;
pub fn cache_store(liliana: &mut LilianaCore, key: String, response: CachedResponse) -> Result<()>;
pub fn cache_evict_lru(liliana: &mut LilianaCore, threshold_age_sec: u64) -> usize;  // returns evicted count

// Template rendering
pub fn render_template(liliana: &LilianaCore, template_name: String, slots: HashMap<String, String>) -> Result<String>;

// Mood state querying
pub fn get_mood(liliana: &LilianaCore) -> MoodState;
pub fn get_mood_trajectory(liliana: &LilianaCore, window_sec: u64) -> Vec<MoodSnapshot>;
```

### Integration with Message Bus

```rust
// Subscribe to Beatrice intent classifier
message_bus.subscribe("beatrice/intent_classified", |msg: BeatriceIntent| {
    liliana.on_beatrice_intent(msg);
});

// Publish Liliana response ready
liliana.on_response_ready = |response: Response| {
    message_bus.publish("liliana/response_ready", response);
};

// Subscribe to Odlaguna gate results
message_bus.subscribe("odlaguna/gate_result", |msg: OdalgunaGate| {
    liliana.apply_gate_modifications(msg);
});
```

---

## 5. Execution Modes

### Mode 1: Reflex Response (Cache Hit, Fast Path)

```
1. User message arrives
2. Liliana checks Intent Classifier (Rasa-style thresholds: nlu_confidence >= 0.3)
3. If social intent + cache hit:
   a. Retrieve cached response + mood adjustments
   b. Render template with mood-based variant
   c. Send to Odlaguna gate
   d. Publish response; update mood + engagement
4. Total latency: < 50ms

Example:
  User: "Hi Mimi!"
  Liliana:
    - Intent: social/greet (confidence 0.95)
    - Cache: HIT (greeting template)
    - Response: "Hey there! How's it going?" [curiosity=0.7]
    - Gate: APPROVED
    - Time: 35ms
```

### Mode 2: Beatrice Negotiation (Parallel Processing)

```
1. User message arrives
2. Liliana attempts reflex; Beatrice parses intent in parallel
3. If Beatrice confidence >= 0.5 AND estimated_complexity in [trivial, simple]:
   a. Use Liliana cache if available; else route to Beatrice output
   b. Beatrice can enhance Liliana's response with entity extraction
4. If Beatrice confidence < 0.3:
   a. Use Liliana fallback (polite non-committal response)
   b. Route to higher pipeline if user pushes back
5. Total latency: < 200ms (includes Beatrice parsing)
```

### Mode 3: Full Pipeline (Cache Miss, Complex Task)

```
1. User message arrives
2. Liliana cache MISS
3. Beatrice identifies high-complexity intent
4. Route to full pipeline: Priscilla → Pandora → Echidna
5. Result flows back; Liliana wraps response in mood context
6. Update cache for future similar queries
7. Total latency: 1-5 seconds (depends on task)
```

---

## 6. Integration with Other Modules

### Pandora (Memory) Integration

```
Liliana queries Pandora for:
  - User profile (preferences, interaction history)
  - Mood trajectory (historical emotional arc)
  - Recent task results (for confidence updates)
  
Liliana writes to Pandora:
  - Mood snapshots (every 5 min or on event)
  - Cache performance metrics (hit rate, latency)
  - User engagement patterns
```

### Beatrice (I/O) Integration

- Liliana is **peer** to Beatrice's NLU, not subordinate
- Both receive user input; both produce outputs
- Liliana uses Beatrice's intent metadata to refine confidence
- Race condition resolved by: Liliana's response used if confidence > 0.7; otherwise merged with Beatrice

### Odlaguna (Security) Integration

- **Every** response from Liliana passes through Odlaguna before publication
- Odlaguna checks: tone alignment, factual consistency, no safety violations
- If Odlaguna rejects, Liliana generates fallback (e.g., "Let me think about that...")

### Echidna (Skills) Integration

- If response requires skill invocation (e.g., "What repos do I have?"), Liliana forwards to Beatrice/Echidna pipeline
- Liliana caches Echidna's responses for future similar queries

---

## 7. Roadmap & Abstraction Layers

### Phase 1 (M2 – Core I/O)
- ✅ Text-based I/O only
- ✅ Basic mood state machine (4 dimensions)
- ✅ Response cache (exact-match + semantic layers)
- ✅ Template system (Rasa domain.yml style)
- ✅ Parallel Beatrice coordination
- ✅ Odlaguna gate integration

**Deliverables:**
- `LilianaCore` struct with mood state, cache, template engine
- Message bus subscriptions for Beatrice/Odlaguna
- Unit tests for cache hit/miss, mood updates, template rendering
- Integration tests with Beatrice/Odlaguna stubs

### Phase 2 (M3 – Multi-Modal I/O)
- TTS adapter (text → audio via Ryzu)
- STT adapter (audio → text via external API)
- Avatar controller stubs (prepare visual expression hooks)
- Sensory I/O test harness

**Deliverables:**
- `SensoryAdapter` trait with Text/Audio/Visual implementations
- Integration with Ryzu for async audio processing
- Mock avatar controller for testing

### Phase 3 (M4 – Mood-Driven Behavior)
- Mood-based personality variance (more/less formal, curious/cautious)
- Long-term mood trajectories (user interaction patterns)
- Mood decay tuning (emotional half-lives per dimension)
- Dashboard for mood visualization

**Deliverables:**
- Extended mood event taxonomy
- Mood decay calibration from real user sessions
- Prometheus metrics: mood KPIs, cache efficiency, response latency

### Phase 4 (M5 – Adaptive Learning)
- Learn user preferences (prefer formal vs casual?)
- Adapt template selection based on historical effectiveness
- A/B testing framework for response variants
- Conversational analytics (user satisfaction signals)

**Deliverables:**
- Preference learning engine
- Response effectiveness tracking
- A/B testing harness

---

## 8. Configuration & Parameters

```toml
[liliana]
# Mood dynamics
mood.decay_factor = 0.95    # per 5-min interval
mood.update_weights = {
  greet = 0.1,
  success = 0.15,
  failure = -0.1,
  novel_input = 0.2,
  repetition = -0.05
}

# Cache settings
cache.exact_match_ttl_sec = 3600        # 1 hour
cache.semantic_match_ttl_sec = 86400    # 24 hours
cache.max_size_mb = 512
cache.semantic_threshold = 0.85         # cosine similarity

# Intent classification thresholds (Rasa-style)
intent.nlu_threshold = 0.3
intent.social_intent_score = 0.3        # if score > threshold, treat as social
intent.core_threshold = 0.3

# Fallback routing
fallback.use_polite_non_committal = true
fallback.max_retries_before_escalate = 2
fallback.escalate_to = "beatrice"
```

---

## 9. Behavioral Examples

### Example 1: Cache Hit (Fast Path)

```
User: "Hi Mimi!"
Timestamp: 14:32:15

Liliana Flow:
1. Intent: social/greet (nlu_confidence=0.95 > 0.3)
2. Cache lookup: exact match found (template="utter_greet")
3. Mood state: {curiosity: 0.7, confidence: 0.8, engagement: 0.6}
4. Template variant selection: curiosity > 0.6 → "Hey there! How's it going?"
5. Odlaguna gate: APPROVED (tone OK, no safety flags)
6. Response sent: "Hey there! How's it going?" [latency=32ms]
7. Update mood: engagement += 0.2 → 0.8
```

### Example 2: Cache Miss, Simple Task

```
User: "How many repos do I have?"
Timestamp: 14:33:45

Liliana Flow:
1. Intent: task/query (nlu_confidence=0.65 > 0.3, but NOT social)
2. Cache lookup: MISS (new task pattern)
3. Forward to Beatrice for entity extraction
4. Beatrice determines: estimate_complexity = "trivial"
5. Beatrice routes to Echidna (cached skill available)
6. Echidna returns: "You have 8 repos"
7. Liliana wraps response: "You have 8 repos. Nice collection!"
8. Store in cache (key=query + config)
9. Odlaguna gate: APPROVED
10. Response sent: "You have 8 repos. Nice collection!" [latency=145ms]
11. Update mood: confidence += 0.15 → 0.95, curiosity += 0.05
```

### Example 3: Cache Miss, Beatrice Escalation

```
User: "What's your opinion on AI ethics?"
Timestamp: 14:35:10

Liliana Flow:
1. Intent: task/reasoning (nlu_confidence=0.42)
2. Cache lookup: MISS (novel topic)
3. Beatrice analysis: estimate_complexity = "complex"
4. Liliana doesn't have pre-computed response
5. Full pipeline triggered: Priscilla → Pandora → LLM reasoning
6. Response: "AI ethics involves balancing... [long thoughtful response]"
7. Liliana wraps: maintains mood context, adds curiosity bump
8. Store in semantic cache
9. Odlaguna gate: APPROVED
10. Response sent: full response [latency=2.3 seconds]
11. Update mood: curiosity += 0.2 → 0.9, engagement += 0.1
```

---

## 10. Observability & Metrics

### Key Performance Indicators

```prometheus
# Cache efficiency
liliana_cache_hits_total{layer="exact"}     # counter
liliana_cache_hits_total{layer="semantic"}  # counter
liliana_cache_misses_total                  # counter
liliana_cache_hit_rate                      # gauge: hits / (hits + misses)

# Response latency
liliana_response_latency_ms{mode="reflex", path="cache"}      # histogram
liliana_response_latency_ms{mode="beatrice", path="parallel"}  # histogram
liliana_response_latency_ms{mode="full", path="pipeline"}      # histogram

# Mood state
liliana_mood_curiosity                  # gauge: 0-1
liliana_mood_confidence                 # gauge: 0-1
liliana_mood_frustration                # gauge: 0-1
liliana_mood_engagement                 # gauge: 0-1

# Interaction patterns
liliana_social_intent_rate              # gauge: ratio of social vs task intents
liliana_cache_template_selections       # counter{template_name}
liliana_responses_approved_by_odlaguna  # counter
liliana_responses_rejected_by_odalguna  # counter

# Failures & recovery
liliana_beatrice_sync_failures          # counter
liliana_cache_evictions_lru             # counter
liliana_template_render_errors          # counter
```

---

## 11. Failure Scenarios & Recovery

### Scenario 1: Cache Corruption

**Problem:** Semantic cache returns irrelevant response due to poor embedding similarity

**Recovery:**
- Fallback to exact-match layer
- If both miss, escalate to Beatrice
- Log incident for cache tuning (adjust similarity threshold)
- Recompute embeddings nightly

### Scenario 2: Mood State Drift

**Problem:** Mood state diverges from user's actual engagement (e.g., frustration stuck at 0.9)

**Recovery:**
- Implement hard reset on user explicit commands ("reset your mood")
- Implement weekly snapshot drift detection (compare to historical baseline)
- If drift detected, log alert; re-calibrate decay factors

### Scenario 3: Odlaguna Rejection

**Problem:** Liliana response rejected by Odlaguna (e.g., tone misaligned)

**Recovery:**
- Liliana generates fallback response: "Let me think about that..."
- Log rejection reason to Pandora
- Escalate to Beatrice/full pipeline for better response
- Mark template as "needs revision" if repeatedly rejected

---

## 12. Open Questions

1. **Mood persistence across sessions:** Should mood state decay continue while user is offline, or reset?
   - Current design: decay continues (implemented as lazy evaluation on next login)
   - Alternative: reset mood on session end (simpler, but less continuity)

2. **Social intent threshold tuning:** Rasa default (0.3) seems low. Should we tighten to 0.5?
   - Depends on empirical testing with real user queries
   - Consider per-user thresholds based on interaction history

3. **Semantic cache similarity threshold:** 0.85 cosine similarity may be too strict.
   - Pilot with 0.80, measure false negatives (should have hit but didn't)

4. **Avatar controller:** What animation framework/format to target?
   - Candidates: Lottie (web-friendly), Blender (3D avatar), custom micro-expressions
   - Defer to M3+

5. **Multi-language support:** How to adapt templates across languages?
   - Current: English only
   - M3+: Use i18n framework; maintain separate domain.yml per language

---

## 13. References & Evidence

- **Rasa Two-Stage Fallback:** NLU confidence thresholds (0.3 default) as production pattern. https://github.com/RasaHQ/rasa_core/blob/master/rasa/core/policies/two_stage_fallback.py
- **LangChain LLM Caching:** Exact-match + semantic cache patterns. https://python.langchain.com/docs/integrations/llms/llm_caching
- **Template-based responses:** Rasa domain.yml structure. https://raw.githubusercontent.com/RasaHQ/rasa-demo/main/domain.yml
- **Mood decay patterns:** QRRanker temporal weighting (arXiv 2602.12192). https://arxiv.org/abs/2602.12192
- **Behavioral continuity:** User modeling in conversational AI. Persona & long-term memory research.

---

## 14. Acceptance Criteria

- [ ] `LilianaCore` struct implemented with mood state, cache, template engine
- [ ] All mood state updates (decay, event-driven) tested and verified
- [ ] Cache hit/miss rates > 70% for common queries
- [ ] Response latency < 50ms for Reflex mode (cache hit)
- [ ] Beatrice parallel coordination working without race conditions
- [ ] All responses validated by Odlaguna before publication
- [ ] Mood state persisted and retrieved from Pandora
- [ ] Unit test coverage > 85%
- [ ] Integration tests with Beatrice/Odlaguna/Pandora passing
- [ ] Documentation complete (this spec + inline code comments)

---

## 15. Glossary

| Term | Definition |
|---|---|
| **Mood State** | Vector of emotional dimensions (curiosity, confidence, frustration, engagement) ranging [0, 1] |
| **Reflex Mode** | Fast path using cached responses; latency < 50ms |
| **Beatrice Coordination** | Parallel intent parsing; negotiated response selection |
| **Odlaguna Gate** | Safety validator ensuring tone, continuity, factual correctness |
| **Template Rendering** | Adaptive response generation using mood-based variant selection and slot filling |
| **Semantic Cache** | Layer 2 cache using embedding similarity (cosine distance) to match semantically similar queries |
| **Exact-Match Cache** | Layer 1 cache using MD5 hash of input + config for deterministic lookups |

