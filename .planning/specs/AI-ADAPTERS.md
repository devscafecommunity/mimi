# AI Adapters Technical Specification

## 1. Overview
This specification defines the pluggable interface for LLM interaction in Mimi. The goal is to decouple the core logic from specific provider implementations, allowing seamless transitions between cloud APIs and local models. By using the adapter pattern, Mimi remains extensible and resilient to API changes or service outages.

## 2. Trait Definition
All adapters must implement the following core interface in Rust. This ensures a consistent contract for the Mimi Commander and other internal modules.

```rust
#[async_trait]
pub trait AIAdapter: Send + Sync {
    /// Generates a complete response for the given prompt.
    async fn generate(&self, prompt: &str) -> Result<String>;

    /// Returns a stream of partial responses as they are generated.
    async fn stream(&self, prompt: &str) -> BoxStream<String>;

    /// Provides the estimated cost in USD for the current configuration.
    fn get_cost_estimate(&self) -> f64;

    /// Returns the unique identifier for this adapter instance.
    fn name(&self) -> &str;

    /// Indicates whether this adapter supports real-time streaming.
    fn supports_streaming(&self) -> bool;
}
```

## 3. Adapter Lifecycle
### Discovery
In Milestone 1, adapters are statically linked and registered during application startup. Future milestones (M2+) will introduce dynamic discovery via plugin directories.

### Initialization
Adapters initialize by loading credentials from environment variables or encrypted stores. Connection pools and HTTP clients are established at this stage.

### Health Checking
The system performs periodic liveness tests by sending minimal "ping" queries to ensure the provider is responsive.

### Shutdown
Adapters implement graceful cleanup to close active connections and flush pending audit logs before the process terminates.

## 4. GeminiAdapter (M1)
### API Details
- **Endpoint**: `generativelanguage.googleapis.com`
- **Authentication**: API key via `GEMINI_API_KEY` environment variable.
- **Rate Limiting**: Default 100 requests per minute, adjustable via configuration.
- **Retry Logic**: Exponential backoff with a maximum of 3 retries for transient failures.

### Error Handling & Costs
- **Errors**: Dedicated handling for authentication failures, quota exhaustion, and network timeouts.
- **Cost Estimate**: Approximately $0.000025 per input token (based on current pricing tiers).
- **Configuration**: Supports model selection, such as `gemini-pro` or `gemini-ultra`.

## 5. OllamaAdapter (M1+)
### Local Integration
- **Endpoint**: `http://localhost:11434` (default).
- **Models**: Compatible with any model pulled via Ollama (e.g., `llama2`, `mistral`, `codellama`).
- **Streaming**: Native support for token-by-token output.

### Performance & Safety
- **Rate Limiting**: None, limited only by local hardware performance.
- **Error Handling**: Handles connection refused errors and "model not found" scenarios.
- **Cost Estimate**: Always $0.0 (local compute).
- **Configuration**: Host, port, and specific model name.

## 6. Custom Adapters (M2+)
### Dynamic Extension
Mimi will support loading external adapters as shared libraries (`.so` or `.dll`). These must expose a C-compatible FFI boundary to interact with the Rust runtime.

### Examples & Isolation
- **Example Targets**: Claude API, GPT-4 API, or proprietary internal models.
- **Sandboxing**: Planned for M3, allowing adapters to run in isolated processes for enhanced security.

## 7. Selection & Routing
### Decision Logic
- **Cost-Aware**: Mimi prioritizes the cheapest available adapter for the requested task complexity.
- **Capability Matching**: Complex reasoning tasks route to high-parameter models, while simple formatting uses local models.

### Reliability
- **Fallback**: If the primary adapter fails, the system automatically tries the next one in the `ADAPTER_FALLBACK_ORDER`.
- **Load Balancing**: Distributes requests across multiple instances of the same adapter type if configured.

## 8. Prompt Injection & Safety
### Content Control
- **Context Injection**: Memory and state are safely embedded using delimiters to prevent context leakage.
- **Token Limits**: Requests are checked against the adapter's maximum context window before transmission.

### Audit & Filtering
- **Filter Patterns**: Rejects requests that match known dangerous or non-compliant patterns.
- **Audit Logging**: Every call, including prompt metadata and response length, is recorded for security reviews.

## 9. Performance Characteristics
| Metric | Gemini (Cloud) | Ollama (Local) |
| :--- | :--- | :--- |
| **Avg Latency** | ~500ms | ~200ms |
| **Throughput** | ~50 tokens/s | ~100+ tokens/s |
| **Concurrency** | Provider limited | Hardware limited |
| **Cost** | Itemized per call | Free |

## 10. Error Scenarios & Recovery
| Scenario | Recovery Action |
| :--- | :--- |
| **Auth Failure** | Log error, notify for credential rotation, fallback if possible. |
| **Rate Limit** | Apply exponential backoff, queue subsequent requests. |
| **Model Unavailable** | Switch to the next model in the fallback sequence. |
| **Network Partition** | Trigger circuit breaker and retry with increased delay. |
| **Context Overflow** | Apply summarization or trim memory using LRU logic. |

## 11. Configuration
Settings are managed via `.env` or `config.toml`:

```toml
GEMINI_API_KEY = "sk-..."
GEMINI_MODEL = "gemini-pro"
GEMINI_RATE_LIMIT = 100

OLLAMA_ENDPOINT = "http://localhost:11434"
OLLAMA_MODEL = "mistral"

ADAPTER_FALLBACK_ORDER = "gemini,ollama,custom"
```

## 12. Testing Strategy
### Validation Layers
- **Unit Tests**: Mocking adapter responses and simulating every entry in the error matrix.
- **Integration Tests**: End-to-end calls using development credentials.
- **Benchmarks**: Continuous tracking of latency and throughput across different versions.

### Security & Compatibility
- **Security Tests**: Active testing of prompt injection defenses and token limit enforcement.
- **Compatibility**: Verifying that new models or provider API updates still satisfy the `AIAdapter` trait.

## 13. Future Extensions (M2+)
- **Provider Additions**: Azure OpenAI, Anthropic Claude, and Cohere.
- **Optimizations**: Support for local model quantization (GGUF/EXL2).
- **Modality**: Adapters for vision, audio, and multi-modal inputs.

---
**Related Documents:**
- [Requirements](REQUIREMENTS.md#RF-8)
- [Milestone 1: Foundation](milestones/M1-FOUNDATION.md)
- [Mimi Commander Module](modules/MIMI-COMMANDER.md)
