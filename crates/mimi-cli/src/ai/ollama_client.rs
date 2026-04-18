use reqwest::{Client, ClientBuilder};
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

/// Ollama API request/response types
#[derive(Debug, Clone)]
pub struct OllamaRequest {
    pub prompt: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct OllamaResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
}

/// Ollama API connection mode
#[derive(Debug, Clone, PartialEq)]
pub enum OllamaMode {
    /// Local Ollama instance (http://localhost:11434)
    Local,
    /// Cloud Ollama API (https://ollama.com)
    Cloud,
}

/// Low-level Ollama API client supporting both local and cloud modes
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    mode: OllamaMode,
    timeout: Duration,
}

#[derive(Error, Debug)]
pub enum OllamaClientError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(String),

    #[error("API returned error: {0}")]
    ApiError(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Rate limited, retry after {0}ms")]
    RateLimited(u64),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
}

impl OllamaClient {
    /// Create a new Ollama API client for local mode
    pub fn new_local(endpoint: String, timeout_ms: u64) -> Result<Self, OllamaClientError> {
        if endpoint.is_empty() {
            return Err(OllamaClientError::InvalidRequest(
                "Endpoint cannot be empty".to_string(),
            ));
        }

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .connection_verbose(false)
            .build()
            .map_err(|e| OllamaClientError::RequestFailed(e.to_string()))?;

        Ok(OllamaClient {
            client,
            endpoint,
            api_key: None,
            mode: OllamaMode::Local,
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    /// Create a new Ollama API client for cloud mode
    pub fn new_cloud(
        api_key: String,
        endpoint: String,
        timeout_ms: u64,
    ) -> Result<Self, OllamaClientError> {
        if api_key.is_empty() {
            return Err(OllamaClientError::AuthenticationFailed(
                "API key cannot be empty for cloud mode".to_string(),
            ));
        }

        if endpoint.is_empty() {
            return Err(OllamaClientError::InvalidRequest(
                "Endpoint cannot be empty".to_string(),
            ));
        }

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .connection_verbose(false)
            .build()
            .map_err(|e| OllamaClientError::RequestFailed(e.to_string()))?;

        Ok(OllamaClient {
            client,
            endpoint,
            api_key: Some(api_key),
            mode: OllamaMode::Cloud,
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    /// Send a request to Ollama API and get response
    pub async fn invoke(
        &self,
        request: OllamaRequest,
    ) -> Result<OllamaResponse, OllamaClientError> {
        let url = format!("{}/api/chat", self.endpoint);

        let body = json!({
            "model": request.model,
            "messages": [{
                "role": "user",
                "content": request.prompt
            }],
            "stream": false,
            "temperature": request.temperature.unwrap_or(0.7),
        });

        let mut req_builder = self
            .client
            .post(&url)
            .header("Content-Type", "application/json");

        // Add authentication for cloud mode
        if self.mode == OllamaMode::Cloud {
            if let Some(api_key) = &self.api_key {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
            }
        }

        let response = req_builder.json(&body).send().await.map_err(|e| {
            if e.is_timeout() {
                OllamaClientError::Timeout(self.timeout.as_millis() as u64)
            } else {
                OllamaClientError::RequestFailed(e.to_string())
            }
        })?;

        let status = response.status();

        match status.as_u16() {
            200 => {
                let json: Value = response
                    .json()
                    .await
                    .map_err(|e| OllamaClientError::ParseError(e.to_string()))?;

                self.parse_response(&json, &request.model)
            },
            401 | 403 => {
                let json: Value = response.json().await.unwrap_or_else(|_| Value::Null);
                let error_msg = json
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unauthorized");

                Err(OllamaClientError::AuthenticationFailed(
                    error_msg.to_string(),
                ))
            },
            429 => {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1000);

                Err(OllamaClientError::RateLimited(retry_after))
            },
            400..=499 => {
                let json: Value = response.json().await.unwrap_or_else(|_| Value::Null);

                let error_msg = json
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown client error");

                Err(OllamaClientError::ApiError(error_msg.to_string()))
            },
            500..=599 => {
                let json: Value = response.json().await.unwrap_or_else(|_| Value::Null);

                let error_msg = json
                    .get("error")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown server error");

                Err(OllamaClientError::ApiError(error_msg.to_string()))
            },
            _ => Err(OllamaClientError::ApiError(format!(
                "Unexpected status code: {}",
                status
            ))),
        }
    }

    /// Parse Ollama API response
    fn parse_response(
        &self,
        json: &Value,
        model: &str,
    ) -> Result<OllamaResponse, OllamaClientError> {
        let text = json
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| OllamaClientError::ParseError("No content in response".to_string()))?
            .to_string();

        // Parse token usage if available (approximated from response)
        let tokens_used = 0u32; // Ollama doesn't always return token count in /api/chat

        Ok(OllamaResponse {
            text,
            model: model.to_string(),
            tokens_used,
        })
    }

    /// Check API connectivity
    pub async fn health_check(&self) -> Result<(), OllamaClientError> {
        let url = format!("{}/api/tags", self.endpoint);

        let mut req_builder = self.client.get(&url);

        // Add authentication for cloud mode
        if self.mode == OllamaMode::Cloud {
            if let Some(api_key) = &self.api_key {
                req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
            }
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| OllamaClientError::RequestFailed(e.to_string()))?;

        match response.status().as_u16() {
            200 => Ok(()),
            401 | 403 => Err(OllamaClientError::AuthenticationFailed(
                "Invalid API key or insufficient permissions".to_string(),
            )),
            _ => Err(OllamaClientError::ApiError(format!(
                "Health check failed with status: {}",
                response.status()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_creation_local_with_valid_endpoint() {
        let client = OllamaClient::new_local("http://localhost:11434".to_string(), 30000);
        assert!(client.is_ok());
        let c = client.unwrap();
        assert_eq!(c.mode, OllamaMode::Local);
        assert_eq!(c.api_key, None);
    }

    #[test]
    fn test_ollama_client_creation_local_with_empty_endpoint() {
        let client = OllamaClient::new_local(String::new(), 30000);
        assert!(client.is_err());
        match client {
            Err(OllamaClientError::InvalidRequest(msg)) => {
                assert!(msg.contains("Endpoint cannot be empty"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_ollama_client_creation_cloud_with_valid_key() {
        let client = OllamaClient::new_cloud(
            "test-api-key".to_string(),
            "https://ollama.com".to_string(),
            30000,
        );
        assert!(client.is_ok());
        let c = client.unwrap();
        assert_eq!(c.mode, OllamaMode::Cloud);
        assert_eq!(c.api_key, Some("test-api-key".to_string()));
    }

    #[test]
    fn test_ollama_client_creation_cloud_with_empty_key() {
        let client =
            OllamaClient::new_cloud(String::new(), "https://ollama.com".to_string(), 30000);
        assert!(client.is_err());
        match client {
            Err(OllamaClientError::AuthenticationFailed(msg)) => {
                assert!(msg.contains("API key cannot be empty"));
            },
            _ => panic!("Expected AuthenticationFailed error"),
        }
    }

    #[test]
    fn test_parse_valid_response() {
        let client = OllamaClient::new_local("http://localhost:11434".to_string(), 30000).unwrap();
        let json = json!({
            "message": {
                "content": "Hello, world!"
            }
        });

        let result = client.parse_response(&json, "llama2");
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Hello, world!");
        assert_eq!(response.model, "llama2");
    }

    #[test]
    fn test_parse_invalid_response() {
        let client = OllamaClient::new_local("http://localhost:11434".to_string(), 30000).unwrap();
        let json = json!({"invalid": "response"});

        let result = client.parse_response(&json, "llama2");
        assert!(result.is_err());
    }
}
