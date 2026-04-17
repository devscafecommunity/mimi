# Beatrice — NLP Interface Module

> **Module Type:** Input/Output Gateway  
> **Primary Language:** Rust (CLI in M1, HTTP/WebSocket in M2+)  
> **Status:** 🟡 Pre-Development  
> **Requirement Coverage:** [RF-2](../REQUIREMENTS.md#rf-2-interface-nlp-beatrice)

---

## Module Overview

**Beatrice** is the human-machine interface layer of the MiMi system. It serves as the primary entry point for user interaction, converting natural language inputs into structured `Intent` messages that the Mimi Commander can process.

### Core Responsibilities

1. **Natural Language Processing** — Parse user input (text initially, multimodal future)
2. **Intent Extraction** — Identify user goals and extract relevant entities
3. **Validation** — Ensure Intent structure is complete and well-formed
4. **Transport Management** — Handle CLI, HTTP API, and WebSocket connections
5. **Result Formatting** — Present Mimi responses in user-friendly formats

### Role in System

Beatrice acts as the **boundary translator** between unstructured human communication and the structured message protocols used by the internal MiMi architecture. It is the only module directly exposed to end users in M1, and will serve as the API gateway in later milestones.

**Key Principles:**
- **Stateless by design** — All conversation state lives in Pandora (M2+)
- **Low latency** — Target < 500ms total (parsing + bus roundtrip)
- **Graceful degradation** — Fall back to keyword matching if NLP fails

---

## Architecture

### Internal Components

```
┌─────────────────────────────────────────────────────────┐
│                        BEATRICE                         │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────┐      ┌──────────────────┐        │
│  │   CLI Handler   │      │   HTTP Server    │ (M2+) │
│  │  (clap/reedline)│      │   (axum/actix)   │        │
│  └────────┬────────┘      └────────┬─────────┘        │
│           │                        │                   │
│           └───────────┬────────────┘                   │
│                       │                                │
│           ┌───────────▼───────────┐                    │
│           │   Intent Parser       │                    │
│           │ ┌──────────────────┐  │                    │
│           │ │ Regex Matcher    │  │ (M1)              │
│           │ │ (Temporary)      │  │                    │
│           │ └──────────────────┘  │                    │
│           │ ┌──────────────────┐  │                    │
│           │ │ NLP Model Adapter│  │ (M2+)             │
│           │ │ (Ollama/Cloud)   │  │                    │
│           │ └──────────────────┘  │                    │
│           └───────────┬───────────┘                    │
│                       │                                │
│           ┌───────────▼───────────┐                    │
│           │   Entity Extractor    │                    │
│           └───────────┬───────────┘                    │
│                       │                                │
│           ┌───────────▼───────────┐                    │
│           │  Confidence Scorer    │                    │
│           └───────────┬───────────┘                    │
│                       │                                │
│           ┌───────────▼───────────┐                    │
│           │    Intent Validator   │                    │
│           └───────────┬───────────┘                    │
│                       │                                │
│           ┌───────────▼───────────┐                    │
│           │    Bus Client         │                    │
│           │  (Zenoh/NATS)         │                    │
│           └───────────────────────┘                    │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Processing Pipeline

1. **Input Reception** → CLI/HTTP receives raw user message
2. **Intent Parsing** → Extract semantic structure from text
3. **Entity Extraction** → Identify key entities (names, dates, numbers, etc.)
4. **Confidence Scoring** → Assign probability to parsed intent
5. **Validation** → Check schema compliance (required fields present)
6. **Bus Transmission** → Serialize to FlatBuffers and publish to `intent/raw` topic
7. **Response Reception** → Subscribe to `intent/response/{request_id}`
8. **Formatting** → Render response for user consumption

---

## API/Interfaces

### Input Methods

#### 1. CLI Interface (M1)

**Command Syntax:**
```bash
# Interactive mode
beatrice

# Single-shot mode
beatrice "What is the weather today?"

# Pipe mode
echo "Summarize file.txt" | beatrice
```

**CLI Dependencies:**
- `clap` — Argument parsing
- `reedline` — Interactive REPL with history
- `crossterm` — Terminal styling

#### 2. HTTP API (M2+)

**Endpoints:**

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/v1/intent` | Submit natural language query |
| `GET` | `/api/v1/health` | Health check |
| `GET` | `/api/v1/version` | Version info |

**Example Request:**
```json
POST /api/v1/intent
Content-Type: application/json

{
  "message": "Create a reminder for tomorrow at 3pm",
  "user_id": "user_12345",
  "context_id": "session_abc"
}
```

**Example Response:**
```json
{
  "request_id": "req_78910",
  "intent": {
    "intent_type": "create_reminder",
    "confidence": 0.92,
    "entities": [
      {"type": "datetime", "value": "2026-04-17T15:00:00Z"},
      {"type": "action", "value": "create"}
    ]
  },
  "result": {
    "status": "success",
    "message": "Reminder created for tomorrow at 3pm"
  }
}
```

#### 3. WebSocket (M2+)

**Connection:** `ws://localhost:8080/ws`

**Message Format (JSON):**
```json
// Client → Server
{
  "type": "intent",
  "payload": {
    "message": "Tell me a joke",
    "stream": true
  }
}

// Server → Client (streaming)
{
  "type": "response_chunk",
  "chunk": "Why did the ",
  "is_final": false
}
```

### Intent Structure

**Core Schema (Rust):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub request_id: Uuid,
    pub user_message: String,
    pub entities: Vec<Entity>,
    pub intent_type: IntentType,
    pub confidence: ConfidenceScore,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}
```

**FlatBuffers Schema (for Bus):**
```fbs
table Intent {
  request_id: string;
  user_message: string;
  entities: [Entity];
  intent_type: IntentType;
  confidence: float;
  timestamp: int64;
  metadata: [KeyValue];
}
```

---

## Key Algorithms

### 1. Intent Parsing (M1 — Regex-Based)

**Strategy:** Pattern matching on common phrases (temporary solution).

```rust
pub fn parse_intent_regex(message: &str) -> Result<IntentType, ParseError> {
    let message_lower = message.to_lowercase();
    
    // Priority order matters
    if REMIND_PATTERN.is_match(&message_lower) {
        return Ok(IntentType::CreateReminder);
    }
    if SEARCH_PATTERN.is_match(&message_lower) {
        return Ok(IntentType::Search);
    }
    if CREATE_SKILL_PATTERN.is_match(&message_lower) {
        return Ok(IntentType::CreateSkill);
    }
    
    // Default fallback
    Ok(IntentType::GeneralQuery)
}
```

**Patterns (examples):**
```rust
lazy_static! {
    static ref REMIND_PATTERN: Regex = 
        Regex::new(r"(?:remind|reminder|alert|notify)").unwrap();
    static ref SEARCH_PATTERN: Regex = 
        Regex::new(r"(?:search|find|look for|show me)").unwrap();
    static ref CREATE_SKILL_PATTERN: Regex = 
        Regex::new(r"(?:create|make|build|generate).*(?:skill|tool)").unwrap();
}
```

**Limitations:**
- No semantic understanding
- Brittle to phrasing variations
- Cannot handle ambiguity

### 2. Intent Parsing (M2+ — NLP Model)

**Strategy:** Use lightweight NLP model (Ollama local or cloud API).

```rust
pub async fn parse_intent_nlp(
    message: &str, 
    adapter: &dyn NLPAdapter
) -> Result<IntentType, ParseError> {
    let prompt = format!(
        "Classify this user message into one of these intents: \
        [search, create_reminder, create_skill, general_query]. \
        Message: '{}'\nIntent:",
        message
    );
    
    let response = adapter.generate(&prompt).await?;
    IntentType::from_str(response.trim())
}
```

**Future Enhancements (M3+):**
- Fine-tuned classification model
- Multi-label intent detection
- Hierarchical intent taxonomy

### 3. Entity Extraction

**M1 Strategy:** Regex + heuristics

```rust
pub fn extract_entities(message: &str) -> Vec<Entity> {
    let mut entities = Vec::new();
    
    // Datetime extraction
    if let Some(dt) = extract_datetime(message) {
        entities.push(Entity {
            entity_type: EntityType::Datetime,
            value: dt.to_rfc3339(),
            confidence: 0.8,
        });
    }
    
    // Number extraction
    for num in extract_numbers(message) {
        entities.push(Entity {
            entity_type: EntityType::Number,
            value: num.to_string(),
            confidence: 0.9,
        });
    }
    
    entities
}
```

**M2+ Strategy:** Use NER (Named Entity Recognition) model

Libraries considered:
- `rust-bert` — Pre-trained NER models
- Remote API — Spacy/HuggingFace via HTTP

### 4. Confidence Scoring

**Formula:**
```rust
pub fn calculate_confidence(
    intent_match: MatchQuality,
    entity_count: usize,
    message_length: usize,
) -> ConfidenceScore {
    let base_score = match intent_match {
        MatchQuality::Exact => 0.95,
        MatchQuality::Strong => 0.80,
        MatchQuality::Weak => 0.60,
        MatchQuality::None => 0.40,
    };
    
    // Boost for entities found
    let entity_boost = (entity_count as f32 * 0.05).min(0.15);
    
    // Penalize very short messages (likely ambiguous)
    let length_penalty = if message_length < 5 { -0.10 } else { 0.0 };
    
    ConfidenceScore::new(
        (base_score + entity_boost + length_penalty).clamp(0.0, 1.0)
    )
}
```

**Confidence Thresholds:**
- `>= 0.8` — High confidence, proceed
- `0.5 - 0.8` — Medium, may require clarification
- `< 0.5` — Low, request user confirmation

---

## Dependencies

### Module Dependencies

```
Beatrice depends on:
  ├─ Mimi Commander (receives Intent, returns Result)
  ├─ Message Bus (Zenoh/NATS for transport)
  └─ AI Adapters (M2+ for NLP, optional)

Depended on by:
  └─ End Users (CLI/HTTP/WebSocket)
```

### External Crates (Rust)

**M1 Core:**
```toml
[dependencies]
clap = "4.0"              # CLI parsing
reedline = "0.20"         # Interactive shell
crossterm = "0.27"        # Terminal UI
regex = "1.7"             # Pattern matching
lazy_static = "1.4"       # Static regex compilation
uuid = { version = "1.3", features = ["v4"] }
chrono = "0.4"            # Timestamp handling
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.28", features = ["full"] }
flatbuffers = "23.5"      # Bus serialization
zenoh = "0.10"            # Message bus client (OR nats = "0.24")
tracing = "0.1"           # Structured logging
```

**M2+ Extensions:**
```toml
axum = "0.6"              # HTTP server
tokio-tungstenite = "0.20" # WebSocket
rust-bert = "0.21"        # NLP models (optional)
```

---

## Data Structures

### Intent Type

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentType {
    // Query intents
    GeneralQuery,
    Search,
    Summarize,
    Translate,
    
    // Action intents
    CreateReminder,
    CreateSkill,
    ExecuteSkill,
    ModifyMemory,
    
    // System intents
    HealthCheck,
    ListSkills,
    ShowMemory,
    
    // Error states
    Ambiguous,
    Unknown,
}

impl IntentType {
    pub fn requires_pandora(&self) -> bool {
        matches!(self, 
            Self::ShowMemory | Self::Summarize | Self::ListSkills
        )
    }
    
    pub fn is_actionable(&self) -> bool {
        matches!(self,
            Self::CreateReminder | Self::CreateSkill | Self::ExecuteSkill
        )
    }
}
```

### Entity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub confidence: f32,
    pub span: Option<(usize, usize)>, // Character positions in original text
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Datetime,
    Duration,
    Number,
    Person,
    Location,
    Organization,
    Skill,
    File,
    Url,
    Custom(u16), // Extensible via ID
}
```

### Confidence Score

```rust
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConfidenceScore(f32);

impl ConfidenceScore {
    pub fn new(score: f32) -> Self {
        assert!(score >= 0.0 && score <= 1.0, "Score must be in [0,1]");
        Self(score)
    }
    
    pub fn as_f32(&self) -> f32 {
        self.0
    }
    
    pub fn is_high(&self) -> bool {
        self.0 >= 0.8
    }
    
    pub fn is_medium(&self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }
    
    pub fn is_low(&self) -> bool {
        self.0 < 0.5
    }
}
```

### Complete Intent Structure

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Unique identifier for tracking request-response pairs
    pub request_id: Uuid,
    
    /// Original user message (unmodified)
    pub user_message: String,
    
    /// Extracted entities from the message
    pub entities: Vec<Entity>,
    
    /// Classified intent type
    pub intent_type: IntentType,
    
    /// Confidence in the classification
    pub confidence: ConfidenceScore,
    
    /// When the intent was created
    pub timestamp: DateTime<Utc>,
    
    /// Extensible metadata (user_id, session_id, etc.)
    pub metadata: HashMap<String, String>,
}

impl Intent {
    pub fn validate(&self) -> Result<(), ValidationError> {
        if self.user_message.is_empty() {
            return Err(ValidationError::EmptyMessage);
        }
        
        if self.user_message.len() > 10_000 {
            return Err(ValidationError::MessageTooLong);
        }
        
        if self.confidence.as_f32() < 0.0 || self.confidence.as_f32() > 1.0 {
            return Err(ValidationError::InvalidConfidence);
        }
        
        Ok(())
    }
}
```

---

## Integration Points

### 1. Receiving User Input

**Entry Points:**

```
User Input Sources:
  ├─ CLI (stdin)
  │   └─ Interactive REPL (reedline)
  │   └─ Single-shot command
  │   └─ Piped input
  │
  ├─ HTTP API (M2+)
  │   └─ REST endpoint: POST /api/v1/intent
  │
  └─ WebSocket (M2+)
      └─ Bidirectional streaming
```

**Example CLI Flow:**
```rust
// beatrice/src/cli.rs
pub async fn run_interactive() -> Result<()> {
    let mut rl = Reedline::create();
    let bus_client = BusClient::connect("tcp://localhost:7447").await?;
    
    loop {
        let sig = rl.read_line(&prompt)?;
        
        match sig {
            Signal::Success(line) => {
                let intent = parse_user_input(&line)?;
                let response = send_intent_and_wait(&bus_client, intent).await?;
                println!("{}", format_response(response));
            }
            Signal::CtrlC | Signal::CtrlD => break,
        }
    }
    
    Ok(())
}
```

### 2. Sending to Mimi

**Message Bus Topic:** `intent/raw`

**Protocol:**
```rust
pub async fn send_intent(
    bus_client: &BusClient,
    intent: Intent,
) -> Result<Uuid> {
    // Validate before sending
    intent.validate()?;
    
    // Serialize to FlatBuffers
    let payload = serialize_intent_fb(&intent)?;
    
    // Publish to bus
    bus_client
        .publish("intent/raw", payload)
        .await?;
    
    Ok(intent.request_id)
}
```

**Request-Response Pattern:**
```rust
pub async fn send_intent_and_wait(
    bus_client: &BusClient,
    intent: Intent,
    timeout: Duration,
) -> Result<Response> {
    let request_id = intent.request_id;
    
    // Subscribe to response topic BEFORE sending
    let response_topic = format!("intent/response/{}", request_id);
    let mut subscriber = bus_client.subscribe(&response_topic).await?;
    
    // Send intent
    send_intent(bus_client, intent).await?;
    
    // Wait for response with timeout
    tokio::select! {
        result = subscriber.recv() => {
            let payload = result?;
            deserialize_response_fb(&payload)
        }
        _ = tokio::time::sleep(timeout) => {
            Err(Error::Timeout)
        }
    }
}
```

### 3. Receiving Results from Mimi

**Message Bus Topic:** `intent/response/{request_id}`

**Response Structure:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub request_id: Uuid,
    pub status: ResponseStatus,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ResponseStatus {
    Success,
    PartialSuccess,
    Failed,
    Timeout,
    RequiresClarification,
}
```

**Response Formatting:**
```rust
pub fn format_response(response: Response) -> String {
    match response.status {
        ResponseStatus::Success => {
            format!("✓ {}", response.content)
        }
        ResponseStatus::RequiresClarification => {
            format!("⚠ Clarification needed: {}", response.content)
        }
        ResponseStatus::Failed => {
            format!("✗ Error: {}", response.content)
        }
        _ => response.content,
    }
}
```

## Personality Injection (via Liliana)

**Overview:**

Beatrice does not generate its own personality or tone. Instead, it subscribes to personality updates published by the **Liliana** module (the interactive presence layer). All system responses are styled according to Liliana's current personality state, which is mood-responsive and configurable.

**Key Points:**
- Liliana maintains a `PersonalityProfile` struct that represents Beatrice's communication style
- Beatrice subscribes to `liliana/personality_update` messages on the Message Bus
- Before formatting any response, Beatrice applies the current personality modifiers
- Personality changes are atomic and versioned for consistency

**Architecture:**

```
┌──────────────┐
│   Liliana    │  (Interactive Presence)
│  ┌─────────┐ │
│  │Mood     │ │  → Computes PersonalityProfile
│  │State    │ │     (formality, confidence, urgency, curiosity, caution)
│  └─────────┘ │
└────────┬─────┘
         │ liliana/personality_update
         ▼
    ┌─────────────────────────┐
    │  Message Bus (NATS)     │
    └─────────────────────────┘
         │
         │ liliana/personality_update (subscribed)
         ▼
    ┌──────────────┐
    │  Beatrice    │
    │  ┌────────┐  │
    │  │Persona │  │  Apply personality modifiers
    │  │Filter  │  │  to all responses before output
    │  └────────┘  │
    └──────────────┘
```

**Personality Application Example:**

```rust
// beatrice/src/personality_filter.rs
pub struct PersonalityFilter {
    current_personality: PersonalityProfile,
    version: u64,
}

impl PersonalityFilter {
    /// Subscribe to personality updates from Liliana
    pub async fn subscribe(bus_client: &BusClient) -> Result<Self> {
        let mut subscriber = bus_client.subscribe("liliana/personality_update").await?;
        
        // Receive initial personality state
        let msg = subscriber.recv().await?;
        let injection: PersonalityInjection = deserialize_pb(&msg)?;
        
        Ok(PersonalityFilter {
            current_personality: injection.personality_state,
            version: injection.version,
        })
    }
    
    /// Apply personality modifiers to response content
    pub fn apply_personality(&self, response: &str) -> String {
        let profile = &self.current_personality;
        let mut styled = response.to_string();
        
        // Example: Increase formality if confidence is high
        if profile.mood_modifiers.confidence > 0.8 {
            styled = self.elevate_formality(&styled, profile.style_vocabulary);
        }
        
        // Example: Add hedging language if caution is high
        if profile.mood_modifiers.caution > 0.7 {
            styled = self.add_caveats(&styled, profile.style_vocabulary);
        }
        
        // Example: Adjust code style if behavior.code_style is "concise"
        if profile.behavior.code_style == "concise" {
            styled = self.simplify_code_examples(&styled);
        }
        
        styled
    }
    
    /// Listen for personality updates and refresh state
    pub async fn listen_for_updates(&mut self, bus_client: &BusClient) -> Result<()> {
        let mut subscriber = bus_client.subscribe("liliana/personality_update").await?;
        
        loop {
            let msg = subscriber.recv().await?;
            let injection: PersonalityInjection = deserialize_pb(&msg)?;
            
            // Only update if version is newer
            if injection.version > self.version {
                self.current_personality = injection.personality_state;
                self.version = injection.version;
                log::info!("Personality updated to v{}", self.version);
            }
        }
    }
}
```

**Integration with Response Formatting:**

```rust
// beatrice/src/cli.rs (updated)
pub async fn run_interactive(
    mut personality_filter: PersonalityFilter,
) -> Result<()> {
    let mut rl = Reedline::create();
    let bus_client = BusClient::connect("tcp://localhost:7447").await?;
    
    // Spawn task to listen for personality updates
    let pf_clone = personality_filter.clone();
    let bc_clone = bus_client.clone();
    tokio::spawn(async move {
        let _ = pf_clone.listen_for_updates(&bc_clone).await;
    });
    
    loop {
        let sig = rl.read_line(&prompt)?;
        
        match sig {
            Signal::Success(line) => {
                let intent = parse_user_input(&line)?;
                let response = send_intent_and_wait(&bus_client, intent).await?;
                
                // Apply personality BEFORE formatting
                let styled = personality_filter.apply_personality(&response.content);
                let formatted = format_response(Response {
                    content: styled,
                    ..response
                });
                
                println!("{}", formatted);
            }
            Signal::CtrlC | Signal::CtrlD => break,
        }
    }
    
    Ok(())
}
```

**Personality State Availability:**

The `PersonalityProfile` is always available to Beatrice for reference:

```rust
pub struct PersonalityProfile {
    pub identity: PersonalityIdentity,           // Archetype & values
    pub mood_modifiers: PersonalityModifiers,    // Formality, confidence, urgency, curiosity, caution
    pub style_vocabulary: StyleVocabulary,       // Greetings, confirmations, errors, etc.
    pub behavior: BehaviorConfig,                // Emoji usage, code style, explanation depth
}
```

See **[PERSONA-INJECTION.md](../specs/PERSONA-INJECTION.md)** for the complete personality injection architecture, mood system, and real-world examples.

---

## Error Handling

### Invalid Input Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum BeatriceError {
    #[error("Empty message provided")]
    EmptyMessage,
    
    #[error("Message exceeds maximum length of {0} characters")]
    MessageTooLong(usize),
    
    #[error("Failed to parse intent: {0}")]
    ParseError(String),
    
    #[error("Failed to extract entities: {0}")]
    EntityExtractionError(String),
    
    #[error("Validation failed: {0}")]
    ValidationError(String),
    
    #[error("Bus communication error: {0}")]
    BusError(#[from] zenoh::Error),
    
    #[error("Timeout waiting for response after {0:?}")]
    Timeout(Duration),
    
    #[error("Invalid UTF-8 in message")]
    Utf8Error(#[from] std::string::FromUtf8Error),
}
```

**Error Recovery Strategies:**

| Error Type | Strategy |
|------------|----------|
| Empty message | Prompt user for input |
| Message too long | Truncate or reject with message |
| Parse failure | Fall back to `IntentType::GeneralQuery` |
| Entity extraction failure | Continue with empty entity list |
| Bus timeout | Retry once, then return timeout error |
| Invalid confidence | Clamp to [0,1] range with warning |

### Timeout Recovery

```rust
pub async fn send_with_retry(
    bus_client: &BusClient,
    intent: Intent,
    max_retries: usize,
) -> Result<Response> {
    let mut attempts = 0;
    let base_timeout = Duration::from_secs(5);
    
    loop {
        attempts += 1;
        
        let timeout = base_timeout * attempts as u32; // Exponential backoff
        
        match send_intent_and_wait(bus_client, intent.clone(), timeout).await {
            Ok(response) => return Ok(response),
            Err(Error::Timeout) if attempts < max_retries => {
                tracing::warn!("Timeout on attempt {}, retrying...", attempts);
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}
```

### Graceful Degradation

**Fallback Chain:**
1. **Primary:** NLP model intent classification (M2+)
2. **Fallback 1:** Regex pattern matching
3. **Fallback 2:** Mark as `IntentType::Unknown` and forward to Mimi for general handling

```rust
pub async fn parse_intent_robust(message: &str) -> Intent {
    // Try NLP first (if available)
    if let Ok(intent) = parse_intent_nlp(message).await {
        return intent;
    }
    
    // Fallback to regex
    if let Ok(intent) = parse_intent_regex(message) {
        return intent;
    }
    
    // Ultimate fallback
    Intent {
        request_id: Uuid::new_v4(),
        user_message: message.to_string(),
        entities: vec![],
        intent_type: IntentType::Unknown,
        confidence: ConfidenceScore::new(0.3),
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    }
}
```

---

## Performance Characteristics

### Latency Targets

| Operation | Target | M1 Actual | Notes |
|-----------|--------|-----------|-------|
| Intent parsing (regex) | < 5ms | TBD | Single-threaded Rust |
| Intent parsing (NLP) | < 100ms | N/A | M2+, depends on model |
| Entity extraction | < 10ms | TBD | Regex-based |
| Bus publish | < 1ms | TBD | Zenoh/NATS |
| Bus roundtrip | < 50ms | TBD | Includes Mimi processing |
| **Total end-to-end** | **< 500ms** | **TBD** | **User input → formatted response** |

### Throughput

**M1 CLI:**
- Sequential processing (single user)
- Target: 1 request/second (interactive human pace)

**M2+ HTTP API:**
- Concurrent processing (Tokio async)
- Target: 100 requests/second (single instance)
- Horizontal scaling: Load balancer → multiple Beatrice instances

### Memory Usage

**M1 Baseline:**
- Idle: ~10 MB (CLI process)
- Per-request: ~100 KB (Intent struct + metadata)

**M2+ with NLP model:**
- Idle: ~500 MB (model in memory)
- Per-request: ~200 KB (model inference overhead)

---

## Testing Strategy

### CLI Behavior Tests

**Test Categories:**
1. **Input parsing** — Verify correct handling of stdin, pipes, arguments
2. **Error display** — Ensure user-friendly error messages
3. **Exit codes** — Proper POSIX exit codes (0=success, 1=error, 130=SIGINT)

**Example Test:**
```rust
#[tokio::test]
async fn test_cli_single_shot() {
    let output = Command::new("beatrice")
        .arg("What is 2+2?")
        .output()
        .unwrap();
    
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("4"));
}
```

### Intent Parsing Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_reminder_intent() {
        let message = "Remind me to call John tomorrow at 3pm";
        let intent = parse_intent_regex(message).unwrap();
        
        assert_eq!(intent.intent_type, IntentType::CreateReminder);
        assert!(intent.confidence.is_high());
        assert_eq!(intent.entities.len(), 2); // person + datetime
    }
    
    #[test]
    fn test_parse_ambiguous_intent() {
        let message = "it";
        let intent = parse_intent_regex(message).unwrap();
        
        assert_eq!(intent.intent_type, IntentType::Unknown);
        assert!(intent.confidence.is_low());
    }
    
    #[test]
    fn test_entity_extraction_datetime() {
        let message = "Meet me on April 17th at 2pm";
        let entities = extract_entities(message);
        
        let datetime_entities: Vec<_> = entities.iter()
            .filter(|e| e.entity_type == EntityType::Datetime)
            .collect();
        
        assert_eq!(datetime_entities.len(), 1);
    }
}
```

### Integration Tests (End-to-End)

**Scenario:** User input → Bus → Mimi → Response

```rust
#[tokio::test]
async fn test_e2e_intent_flow() {
    // Setup test bus
    let bus = TestBusServer::start().await;
    let client = BusClient::connect(bus.url()).await.unwrap();
    
    // Mock Mimi responder
    let mimi_mock = spawn_mimi_mock(&bus).await;
    
    // Send intent
    let intent = Intent {
        request_id: Uuid::new_v4(),
        user_message: "What is the weather?".into(),
        intent_type: IntentType::GeneralQuery,
        confidence: ConfidenceScore::new(0.9),
        entities: vec![],
        timestamp: Utc::now(),
        metadata: HashMap::new(),
    };
    
    let response = send_intent_and_wait(&client, intent, Duration::from_secs(5))
        .await
        .unwrap();
    
    assert_eq!(response.status, ResponseStatus::Success);
    assert!(response.content.contains("weather"));
    
    // Cleanup
    mimi_mock.shutdown().await;
    bus.shutdown().await;
}
```

### Performance Tests

```rust
#[tokio::test]
async fn test_intent_parsing_latency() {
    let message = "Create a reminder for tomorrow at 3pm to call mom";
    
    let start = Instant::now();
    let _ = parse_intent_regex(message).unwrap();
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(5), 
        "Parsing took {:?}, expected < 5ms", duration);
}

#[tokio::test]
async fn test_bus_roundtrip_latency() {
    let bus = TestBusServer::start().await;
    let client = BusClient::connect(bus.url()).await.unwrap();
    
    // Mock instant responder
    spawn_instant_responder(&bus).await;
    
    let intent = create_test_intent();
    
    let start = Instant::now();
    let _ = send_intent_and_wait(&client, intent, Duration::from_secs(1))
        .await
        .unwrap();
    let duration = start.elapsed();
    
    assert!(duration < Duration::from_millis(50),
        "Roundtrip took {:?}, expected < 50ms", duration);
}
```

### Test Coverage Targets

| Component | Target Coverage | Priority |
|-----------|----------------|----------|
| Intent parsing | ≥ 90% | High |
| Entity extraction | ≥ 85% | High |
| CLI handlers | ≥ 80% | Medium |
| Bus communication | ≥ 95% | Critical |
| Error handling | ≥ 90% | High |

---

## Future Extensions

### M2+ Enhancements

1. **HTTP/WebSocket API**
   - REST endpoints for intent submission
   - WebSocket for streaming responses
   - API key authentication
   - Rate limiting per user

2. **NLP Model Integration**
   - Replace regex with actual NLP models
   - Support for Ollama (local) or cloud APIs
   - Model switching based on task complexity
   - Fine-tuning on domain-specific intents

3. **Advanced Entity Recognition**
   - NER (Named Entity Recognition) models
   - Custom entity types per user
   - Entity linking to Pandora knowledge graph
   - Coreference resolution

### M3+ Advanced Features

1. **Multilingual Support**
   - Auto-detect language
   - Translate intent to English internally
   - Respond in user's language
   - Language-specific entity rules

2. **Multimodal Input**
   - Image input (OCR + vision models)
   - Audio input (speech-to-text)
   - Document parsing (PDF, DOCX)
   - Screen sharing for troubleshooting

3. **Context-Aware Parsing**
   - Integration with Pandora for conversation history
   - Anaphora resolution ("it", "that", "this")
   - Multi-turn intent clarification
   - Session-based context retention

4. **Intent Chaining**
   - Parse compound intents ("Do X then Y")
   - Conditional intents ("If X then Y")
   - Parallel intent execution
   - Dependency resolution between intents

### Performance Optimizations

1. **Caching**
   - LRU cache for common phrases
   - Pre-compiled regex patterns
   - Entity extraction cache

2. **Batching**
   - Batch multiple intents to NLP model
   - Reduce model invocation overhead

3. **Connection Pooling**
   - Reuse Bus connections
   - HTTP/2 connection multiplexing

---

## Cross-References

### Related Documents

- **Requirements:** [RF-2 — NLP Interface](../REQUIREMENTS.md#rf-2-interface-nlp-beatrice)
- **Architecture:** [ARCHITECTURE.md](../ARCHITECTURE.md)
- **Module:** [MIMI-COMMANDER.md](./MIMI-COMMANDER.md) — Intent consumer
- **Spec:** [BUS-PROTOCOL.md](../specs/BUS-PROTOCOL.md) — Message bus topics
- **Milestone:** [M1-FOUNDATION.md](../milestones/M1-FOUNDATION.md) — Implementation plan

### Intent Flow Diagram

```
User Input (CLI/HTTP/WS)
    │
    ▼
Beatrice (parse + validate)
    │
    ▼
Message Bus: intent/raw
    │
    ▼
Mimi Commander (route to appropriate module)
    │
    ├─▶ Pandora (memory query)
    ├─▶ Echidna (skill creation)
    ├─▶ Ryzu (skill execution)
    └─▶ AI Adapter (generation)
    │
    ▼
Message Bus: intent/response/{request_id}
    │
    ▼
Beatrice (format response)
    │
    ▼
User Output (CLI/HTTP/WS)
```

---

**Document Version:** 1.0  
**Last Updated:** 2026-04-16  
**Maintainer:** MiMi Development Team
