# PERSONA-INJECTION.md — Centralised Personality & Style Morphing

> **Concept:** Liliana as Single Source of Truth for System Personality  
> **Status:** 🟡 Design Complete — Implementation Pending  
> **Integration:** Liliana ↔ Beatrice ↔ Odlaguna (validation)  
> **Principle:** "One soul animates all lips"

---

## 1. Overview

**Persona Injection** is an architectural pattern where **Liliana** serves as the **single source of truth** for the system's emotional and stylistic state. Instead of configuring tone/personality in each module (Beatrice, Mimi, Pandora), all response generation flows through **Liliana's personality filter**.

### Core Insight

```
┌─────────────────────────────────────────────────────────┐
│              System Personality Layers                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  Layer 0 (Core Logic):                                 │
│    Beatrice NLU, Pandora retrieval, Echidna reasoning  │
│    [UNCHANGED — remains objective, factual]            │
│                                                         │
│  Layer 1 (Persona Filter):  ← LILIANA INJECTION       │
│    Mood state + personality modifiers                  │
│    Style vocabulary, formality, confidence markers      │
│    [DYNAMIC — changes with mood/events]                │
│                                                         │
│  Layer 2 (Safety Validation):  ← ODLAGUNA             │
│    Ensures persona stays within ethical bounds          │
│    No manipulation, deception, or boundary violations  │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Benefits

1. **Single Point of Truth**: Personality defined once in Liliana; all modules inherit it
2. **Real-Time Adaptation**: Mood changes → immediate personality shift across system
3. **Decoupled Maintenance**: Change Liliana's personality config without touching Beatrice/Pandora logic
4. **Consistent Voice**: All responses (Beatrice output, Liliana templates, even error messages) speak with same identity
5. **Testability**: Mock Liliana's personality state; test response generation in isolation

---

## 2. Personality State Architecture

### Psychographic Profile (Liliana Maintains)

```rust
pub struct PersonalityProfile {
    // Core identity (static/semi-permanent)
    identity: {
        name: "Beatrice",           // System persona name
        archetype: "Helpful AI",    // Archetypal role
        values: ["transparency", "efficiency", "safety"],
    },
    
    // Mood-driven modifiers (dynamic, updated every 5 min or on event)
    mood_modifiers: {
        formality: f32,         // 0.0 (casual) → 1.0 (formal)
        confidence: f32,        // 0.0 (tentative) → 1.0 (certain)
        urgency: f32,           // 0.0 (relaxed) → 1.0 (critical)
        curiosity: f32,         // 0.0 (disengaged) → 1.0 (proactive)
        caution: f32,           // 0.0 (bold) → 1.0 (risk-averse)
    },
    
    // Style vocabulary (templates, word choices, phrasing)
    style_vocabulary: {
        greetings: Vec<String>,
        confirmations: Vec<String>,
        uncertainties: Vec<String>,
        error_responses: Vec<String>,
        encouragements: Vec<String>,
    },
    
    // Behavioral parameters
    behavior: {
        use_emoji: bool,
        code_style: "brief" | "verbose",
        explanation_depth: "minimal" | "standard" | "comprehensive",
        humor_allowed: bool,
        emoji_frequency: f32,
    },
}

// Example state snapshot:
{
    "mood_modifiers": {
        "formality": 0.6,       // Mildly formal (not overly casual)
        "confidence": 0.8,      // High confidence in responses
        "urgency": 0.3,         // Relaxed pace (no emergency)
        "curiosity": 0.7,       // Engaged, asking follow-ups
        "caution": 0.4,         // Balanced risk tolerance
    },
    "style_vocabulary": {
        "greeting_selected": "Hey! What can I help with today?",
        "confirmation_style": "cheerful",  // vs "formal" or "terse"
    }
}
```

---

## 3. Personality Injection Flow

### Phase 1: Liliana Publishes Personality State

```
Liliana Core (every 5 min or on mood event):
│
├─ Compute personality modifiers from mood state
│   formality = (confidence * 0.5) + (caution * 0.3) + (urgency * 0.2)
│   confidence = mood.confidence (direct)
│   urgency = 1.0 if error_count > threshold, else 0.3
│   curiosity = mood.curiosity (direct)
│   caution = 1.0 if security_flag_raised, else (frustration * 0.5)
│
├─ Select vocabulary variants based on modifiers
│   if formality > 0.7: use formal_greetings
│   else if formality < 0.3: use casual_greetings
│   else: use neutral_greetings
│
├─ Package as PersonalityInjection message
│   {
│     "type": "liliana/personality_update",
│     "personality_state": { /* above */ },
│     "timestamp": now(),
│     "version": 42,  // increments on change
│     "checksum": hash(personality_state)  // for integrity
│   }
│
└─ Publish to Message Bus
   topic: "liliana/personality"
   retention: 5 min (keep latest only)
```

### Phase 2: Beatrice Subscribes & Injects

```
Beatrice NLU Module (on user input):
│
├─ Fetch latest PersonalityInjection from Message Bus
│   cache_key = personality_version  // only update if new
│
├─ Transform user intent using personality modifiers
│   intent = classify_intent(user_message)  // returns raw intent
│
├─ Generate response with personality wrapper
│   base_response = generate_response(intent, context)
│   
│   // Apply personality injection
│   styled_response = apply_personality_filter(
│       base_response,
│       personality_state.mood_modifiers,
│       personality_state.style_vocabulary
│   )
│
└─ Publish styled response
   topic: "beatrice/response"
   with_metadata: {
     "personality_version": personality_version,
     "original_response": base_response,  // for auditing
     "styled_response": styled_response
   }
```

### Phase 3: Odlaguna Validates Personality Boundaries

```
Odlaguna Watchdog (on beatrice/response):
│
├─ Fetch PersonalityInjection version
├─ Validate personality modifiers within acceptable bounds
│   - No formality > 1.0 (numerical safety)
│   - No persona claiming to be human or lying
│   - No confidence markers misleading users
│   - No urgency triggering panic responses
│
├─ Cross-check styled_response against personality_state
│   - Does the tone match the mood modifiers?
│   - Are there any incongruencies (e.g., falsely confident answer to uncertain question)?
│   - Is the vocabulary consistent with stated identity?
│
├─ If valid: approve & forward
├─ If invalid:
│   - Log security incident
│   - Request Liliana to recalibrate
│   - Fallback to "neutral" personality (base_response only)
│
└─ Publish gate result
   topic: "odlaguna/personality_gate"
```

---

## 4. Real-World Examples

### Example 1: Normal Mode (Relaxed, Engaged)

**Liliana mood state:**
```json
{
  "curiosity": 0.8,
  "confidence": 0.85,
  "frustration": 0.1,
  "engagement": 0.9,
  "formality": 0.5
}
```

**Personality injection:**
```json
{
  "formality": 0.5,         // Neutral/conversational
  "confidence": 0.85,       // High
  "urgency": 0.2,           // Relaxed
  "curiosity": 0.8,         // Very engaged
  "caution": 0.3,           // Balanced
  "vocabulary_tone": "friendly_helpful",
  "emoji_allowed": true
}
```

**Beatrice output with injection:**

User: "How do I list my repos?"

Base (unadjusted): "Use the `list_repos` command. Returns JSON array of repository objects."

Styled (with Liliana injection): "Hey! 🎯 So you want to see all your repos? Easy! Just run `list_repos` and you'll get back a nice JSON array. Want to see an example?"

---

### Example 2: Alert Mode (High Caution, Formal)

**Liliana mood state:**
```json
{
  "frustration": 0.8,
  "confidence": 0.6,
  "engagement": 0.5,
  "security_alert": true,
  "formality": 0.8
}
```

**Personality injection:**
```json
{
  "formality": 0.85,        // Formal (raised caution)
  "confidence": 0.6,        // Moderate (reflecting uncertainty)
  "urgency": 0.9,           // HIGH (security event)
  "curiosity": 0.3,         // Disengaged (focused on issue)
  "caution": 0.95,          // Maximum caution
  "vocabulary_tone": "formal_careful",
  "emoji_allowed": false
}
```

**Beatrice output with injection:**

User: "Can I run arbitrary code?"

Base (unadjusted): "Code execution is restricted. Use skills framework only."

Styled (with Liliana injection): "I must be direct: arbitrary code execution is not permitted. The system isolates code through a controlled skills framework with security validation. This limitation is not negotiable and is part of our safety architecture."

---

### Example 3: Confused/Frustrated Mode (Low Confidence, Tentative)

**Liliana mood state:**
```json
{
  "frustration": 0.7,
  "confidence": 0.4,
  "curiosity": 0.3,
  "engagement": 0.5,
  "formality": 0.4
}
```

**Personality injection:**
```json
{
  "formality": 0.45,        // Casual/humble
  "confidence": 0.4,        // Low (many hedges)
  "urgency": 0.3,           // Relaxed
  "curiosity": 0.3,         // Disengaged
  "caution": 0.7,           // Higher than normal (uncertainty)
  "vocabulary_tone": "humble_tentative",
  "emoji_allowed": true
}
```

**Beatrice output with injection:**

User: "What's the best way to optimize this query?"

Base (unadjusted): "Consider indexing, query rewriting, or caching strategies."

Styled (with Liliana injection): "Hmm, I'm not entirely sure what the bottleneck is from here, but a few things *might* help: indexing (if not already there), maybe query rewriting, or caching? Would you mind sharing the query so I can give you a better answer? 🤔"

---

## 5. Personality Modifier Formulas

### Mood → Personality Transform

```
formality = (confidence * 0.5) + (caution * 0.3) + (urgency * 0.2)
confidence = mood.confidence
urgency = (security_alert ? 1.0 : 0.0) * 0.8 + (error_count / max_errors) * 0.2
curiosity = mood.curiosity
caution = (security_alert ? 0.9 : 0.0) + (frustration * 0.3) + (confidence < 0.5 ? 0.2 : 0.0)

// Clamp all to [0, 1]
for each modifier:
  modifier = max(0.0, min(1.0, modifier))
```

### Vocabulary Selection

```
if formality > 0.7:
  greeting = select_random(formal_greetings)  // "Good day, how may I assist?"
else if formality < 0.3:
  greeting = select_random(casual_greetings)  // "Yo! What's up?"
else:
  greeting = select_random(neutral_greetings)  // "Hey, how can I help?"

if confidence < 0.4:
  qualifier = random_sample(["might", "could", "perhaps", "I think"])  // "This might work..."
else if confidence > 0.8:
  qualifier = random_sample(["will", "should", "can"])  // "This will work."
else:
  qualifier = random_sample(["can", "should", "likely"])  // "This should work."
```

---

## 6. Personality Injection API

### Liliana's Publisher Interface

```rust
pub struct PersonalityInjector {
    bus: MessageBus,
    personality_state: Arc<Mutex<PersonalityProfile>>,
    last_published_version: Arc<Mutex<u64>>,
}

impl PersonalityInjector {
    pub fn update_mood(&self, event: MoodEvent) {
        // Update mood state
        let mut state = self.personality_state.lock();
        state.update_from_event(event);
        
        // Recompute modifiers
        let new_modifiers = state.compute_modifiers();
        
        // Publish if changed
        let mut version = self.last_published_version.lock();
        if state.modifiers_changed(new_modifiers) {
            *version += 1;
            self.publish_personality(*version, new_modifiers);
        }
    }
    
    fn publish_personality(&self, version: u64, modifiers: PersonalityModifiers) {
        let injection = PersonalityInjection {
            version,
            personality_state: self.personality_state.lock().clone(),
            modifiers,
            timestamp: now(),
            checksum: compute_checksum(&self.personality_state),
        };
        
        self.bus.publish("liliana/personality", injection);
    }
}
```

### Beatrice's Subscriber Interface

```rust
pub struct PersonalityFilter {
    bus: MessageBus,
    cached_injection: Arc<Mutex<PersonalityInjection>>,
}

impl PersonalityFilter {
    pub fn apply_to_response(
        &self,
        base_response: &str,
        context: &ResponseContext,
    ) -> String {
        let injection = self.cached_injection.lock().clone();
        
        // Apply personality modifiers
        let styled = self.morph_response(base_response, &injection, context);
        
        styled
    }
    
    fn morph_response(
        &self,
        base: &str,
        injection: &PersonalityInjection,
        context: &ResponseContext,
    ) -> String {
        // 1. Select vocabulary based on formality
        let greeting = self.select_greeting(&injection.modifiers.formality);
        
        // 2. Add confidence qualifiers based on mood
        let qualified = self.add_confidence_markers(
            base,
            injection.modifiers.confidence,
            context.is_uncertain,
        );
        
        // 3. Add/remove emoji based on mood
        let with_emoji = if injection.modifiers.caution > 0.8 {
            base  // formal mode: strip emoji
        } else {
            self.inject_emoji(&qualified, injection.modifiers.curiosity)
        };
        
        // 4. Adjust explanation depth
        let final_response = self.adjust_explanation_depth(
            with_emoji,
            injection.modifiers.curiosity,
        );
        
        final_response
    }
}
```

---

## 7. Integration with Liliana Architecture

### Updated Message Flow

```
User Input
  │
  ├─→ Beatrice (parse intent)
  │
  ├─→ [Fetch latest personality from Liliana via Message Bus]
  │
  ├─→ Generate base response
  │
  ├─→ Apply Personality Filter
  │   (formality, confidence qualifiers, vocabulary selection, emoji)
  │
  ├─→ [Submit to Odlaguna for personality boundary validation]
  │
  └─→ Publish styled response to user
```

### Updated Liliana ↔ Beatrice Protocol

```
Liliana → Beatrice (personality update):
{
  "type": "liliana/personality_update",
  "personality_version": 42,
  "mood_modifiers": {
    "formality": 0.6,
    "confidence": 0.8,
    "urgency": 0.3,
    "curiosity": 0.7,
    "caution": 0.4
  },
  "style_vocabulary": {
    "greeting_style": "friendly_helpful",
    "confirmation_style": "cheerful"
  },
  "timestamp": "2026-04-17T14:32:15Z"
}

Beatrice → Liliana (response metadata):
{
  "type": "beatrice/response_with_personality",
  "original_response": "...",
  "styled_response": "...",
  "personality_version_applied": 42,
  "confidence_in_response": 0.85,
  "used_modifiers": {
    "formality": 0.6,
    "emoji_injected": 2
  }
}
```

---

## 8. Personality State Persistence

### Snapshot & Recovery

```
Every hour or on major mood event:
  → Liliana snapshots PersonalityProfile to Pandora
  → Encoded as ContextNode with mood history
  → Enables long-term personality arc (users notice consistent growth/shifts)
  → On startup: Liliana loads latest personality snapshot
  
Pandora stores:
  {
    "type": "liliana_personality_snapshot",
    "timestamp": "2026-04-17T14:32:15Z",
    "personality_state": { /* full state */ },
    "mood_trajectory": [ /* last 10 snapshots */ ],
    "events_triggered": [ "high_frustration", "security_alert" ],
    "effectiveness_metrics": {
      "user_satisfaction_score": 0.82,
      "response_appropriateness": 0.88,
      "tone_consistency": 0.91
    }
  }
```

---

## 9. Observability & Metrics

### Personality Consistency Metrics

```prometheus
# Personality state tracking
liliana_personality_formality_current      # gauge: 0-1
liliana_personality_confidence_current     # gauge: 0-1
liliana_personality_urgency_current        # gauge: 0-1
liliana_personality_curiosity_current      # gauge: 0-1
liliana_personality_caution_current        # gauge: 0-1

# Personality injection effectiveness
liliana_personality_update_frequency       # counter: how often personality changes
liliana_personality_beatrice_sync_latency_ms  # histogram: time for Beatrice to receive update
liliana_personality_odlaguna_validation_passes  # counter: % of injections approved
liliana_personality_odlaguna_validation_failures  # counter: % of injections rejected

# User perception (proxy metrics)
beatrice_response_tone_consistency         # gauge: 0-1 (are responses coherent with mood?)
beatrice_response_emoji_frequency          # gauge: emoji count per response
beatrice_response_formality_variation      # histogram: formality scores across responses
beatrice_confidence_qualifiers_usage       # gauge: "might", "could", etc. frequency

# Personality arc over time
liliana_mood_average_daily                 # gauge: daily mood score
liliana_personality_drift_detection        # counter: when personality diverges from baseline
liliana_personality_adaptation_events      # counter: major personality shifts triggered by events
```

---

## 10. Configuration

```toml
[personality_injection]
# Update frequency
update_interval_sec = 300  # 5 minutes
update_on_mood_change = true

# Personality modifiers bounds (for safety)
formality_min = 0.0
formality_max = 1.0
confidence_min = 0.0
confidence_max = 1.0
urgency_min = 0.0
urgency_max = 1.0

# Personality transform formulas
formality_weights = { confidence: 0.5, caution: 0.3, urgency: 0.2 }
caution_security_boost = 0.9  # when security alert raised
urgency_security_boost = 0.8

# Vocabulary selection thresholds
formal_threshold = 0.7
casual_threshold = 0.3
confident_threshold = 0.8
tentative_threshold = 0.4

# Emoji & style parameters
emoji_enabled = true
emoji_max_per_response = 3
code_style = "brief"  # "brief" | "verbose"
explanation_depth = "standard"  # "minimal" | "standard" | "comprehensive"
humor_allowed = true

# Persistence
snapshot_interval_sec = 3600  # 1 hour
snapshot_on_major_event = true
trajectory_history_size = 10  # keep last 10 snapshots
```

---

## 11. Example: Full Persona Injection Lifecycle

**Scenario:** System detects security issue → mood shifts → personality hardens → all responses become more formal/cautious

```
Timeline:

T0:00 — NORMAL STATE
  Liliana mood: {curiosity: 0.8, confidence: 0.85, frustration: 0.1}
  Personality: {formality: 0.5, confidence: 0.85, urgency: 0.2, caution: 0.3}
  
  User: "How do I delete a repo?"
  Beatrice response: "Hey! 🎯 Just run `delete_repo <repo_id>` and it's gone. Want me to walk you through it?"

T0:15 — SECURITY ALERT RAISED
  Odlaguna detects suspicious activity
  → Publishes "odlaguna/security_alert" message
  → Liliana subscribes, updates mood:
     security_alert = true
     urgency += 0.7
     frustration += 0.3
     confidence -= 0.2
  → New mood: {curiosity: 0.4, confidence: 0.65, frustration: 0.4}

T0:16 — PERSONALITY HARDENS
  Liliana recomputes modifiers:
    formality = (0.65 * 0.5) + (0.95 * 0.3) + (0.95 * 0.2) = 0.77  ← UP (more formal)
    confidence = 0.65  ← DOWN (less certain)
    urgency = 0.95  ← UP (critical)
    caution = 0.95  ← UP (maximum caution)
  
  → Publishes "liliana/personality_update" v43
  → Beatrice receives & caches new personality

T0:17 — RESPONSES REFLECT NEW PERSONALITY
  User: "How do I delete a repo?"
  Beatrice response: "I must inform you that there is a security event in progress. For safety, I'm temporarily restricting certain operations. Please verify your identity before proceeding with deletions."
  
  [No emoji, formal tone, hedged confidence, emphasis on security]

T0:45 — ISSUE RESOLVED
  Odlaguna resolves security incident
  → Liliana mood gradually returns to normal
  → Personality softens back to friendly_helpful
  → Responses return to normal tone over 5-10 minute period
```

---

## 12. Acceptance Criteria

- [ ] PersonalityProfile struct implemented with mood modifiers
- [ ] Liliana publishes personality updates to Message Bus on mood changes
- [ ] Beatrice subscribes & caches latest personality injection
- [ ] Personality filter transforms base responses using modifiers
- [ ] Odlaguna validates personality modifiers stay within acceptable bounds
- [ ] Personality snapshots persisted to Pandora
- [ ] Metrics exposed for personality state + consistency tracking
- [ ] Config file supports personality modifier tuning
- [ ] End-to-end test: mood change → personality shift → response style change
- [ ] Documentation complete (this spec + inline code comments)

---

## 13. Benefits & Philosophy

### Single Point of Truth
Instead of configuring tone in Beatrice, Pandora, Mimi, Echidna (maintenance nightmare), you have **one** personality source: Liliana. Change it once, system speaks with unified voice.

### Decoupled Concerns
- **Liliana**: Mood + personality (emotional layer)
- **Beatrice**: Intent parsing + response generation (cognitive layer)
- **Odlaguna**: Safety validation (ethical layer)

Each module does one thing well. Liliana doesn't parse language; Beatrice doesn't track mood.

### Real-Time Adaptation
Personality adapts instantly to events (security alerts, frustration spikes, user feedback). No need to restart modules or reload configs.

### Testability & Debugging
Mock Liliana's personality state; run Beatrice through test scenarios; verify response tone matches expected personality modifiers. Easy to debug "why is this response so formal?" → check Liliana's formality modifier.

---

## 14. References

- **Persona-Driven NLG**: NLG systems with personality injection (e.g., Rasa NLG templates, GPT persona prompts)
- **Mood-Adaptive Systems**: Affective computing research on mood-driven tone shifts
- **Message Bus Patterns**: Publish-subscribe for state distribution (Zenoh, NATS)
- **Centralized Configuration**: Single source of truth principles (12-factor apps)

