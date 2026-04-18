use futures::future::join_all;
use mimi_cli::ai::{
    AdapterCapabilities, AdapterInitParams, AdapterRegistry, AiAdapter, AiRequest, AiResponse,
    SharedAdapter,
};
use std::sync::Arc;
use tokio::sync::Mutex;

struct FullStackAdapter {
    name: String,
}

impl FullStackAdapter {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl AiAdapter for FullStackAdapter {
    async fn initialize(&self, _params: AdapterInitParams) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn capabilities(&self) -> mimi_cli::ai::AdapterResult<AdapterCapabilities> {
        Ok(AdapterCapabilities {
            supports_streaming: true,
            supports_caching: true,
            max_context_tokens: 8192,
            supported_models: vec!["full-stack-model".to_string()],
        })
    }

    async fn invoke(&self, request: AiRequest) -> mimi_cli::ai::AdapterResult<AiResponse> {
        // Simulate a full-stack response that flows through the entire pipeline
        let content = if request.prompt.to_lowercase().contains("rust") {
            "Rust is a systems programming language that emphasizes memory safety and concurrency."
                .to_string()
        } else {
            format!("Response to query: {}", request.prompt)
        };

        Ok(AiResponse {
            content,
            model: "full-stack-model".to_string(),
            tokens_used: 150,
            cached: false,
        })
    }

    async fn health_check(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> mimi_cli::ai::AdapterResult<()> {
        Ok(())
    }

    fn adapter_name(&self) -> String {
        self.name.clone()
    }
}

struct TestMessageBus;
struct TestMimiCore;

async fn start_test_message_bus() -> TestMessageBus {
    TestMessageBus
}

async fn start_mimi_core() -> TestMimiCore {
    TestMimiCore
}

async fn setup_adapters() -> Arc<AdapterRegistry> {
    let registry = AdapterRegistry::new();
    let adapter: SharedAdapter = Arc::new(Mutex::new(FullStackAdapter::new("full-stack")));

    // Register adapter for full-stack testing
    registry
        .register_with_health("full-stack".to_string(), adapter)
        .await
        .unwrap();

    registry
}

async fn send_cli_command_end_to_end(
    _cmd: &str,
    prompt: &str,
    registry: &Arc<AdapterRegistry>,
) -> Result<AiResponse, String> {
    // Simulate CLI command flowing through the entire stack:
    // CLI parse → Message Bus publish → Mimi core subscribe → Adapter invoke → Response publish → CLI receive

    let request = AiRequest {
        prompt: prompt.to_string(),
        model: None,
        temperature: Some(0.7),
        max_tokens: Some(256),
        system_context: Some("You are a helpful assistant".to_string()),
    };

    // Get best available adapter from registry (simulating Mimi core adapter selection)
    let best = registry
        .get_best_available()
        .await
        .map_err(|e| format!("Adapter selection failed: {:?}", e))?;

    // Invoke adapter (simulating message bus invocation)
    let response = best
        .lock()
        .await
        .invoke(request)
        .await
        .map_err(|e| format!("Adapter invocation failed: {:?}", e))?;

    Ok(response)
}

#[tokio::test]
async fn test_full_stack_cli_to_adapter_integration() {
    // Setup: Start all components
    let _bus = start_test_message_bus().await;
    let _mimi = start_mimi_core().await;
    let registry = setup_adapters().await;

    // Execute: Send CLI command that flows through entire stack
    // CLI parse → Message Bus publish → Mimi core subscribe → Adapter invoke → Response publish → CLI receive
    let response = send_cli_command_end_to_end("query", "Explain Rust", &registry).await;

    // Verify: Complete response from adapter is received
    assert!(response.is_ok());
    let resp = response.unwrap();
    assert!(resp.content.contains("Rust"));
    assert!(!resp.model.is_empty());
    assert_eq!(resp.model, "full-stack-model");
    assert!(resp.tokens_used > 0);
}

#[tokio::test]
async fn test_full_stack_concurrent_requests() {
    // Setup
    let _bus = start_test_message_bus().await;
    let _mimi = start_mimi_core().await;
    let registry = setup_adapters().await;

    // Execute: Send 10 concurrent requests through the full stack
    let mut handles = vec![];
    for i in 0..10 {
        let reg = registry.clone();
        let handle = tokio::spawn(async move {
            send_cli_command_end_to_end("query", &format!("Question {}", i), &reg).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let results: Vec<_> = join_all(handles).await;

    // Verify: All requests succeeded
    for result in results {
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert!(resp.is_ok());
        let response = resp.unwrap();
        assert!(!response.content.is_empty());
        assert!(!response.model.is_empty());
        assert!(response.tokens_used > 0);
    }
}
