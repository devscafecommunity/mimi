use reqwest::{Client, ClientBuilder};
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

/// Gemini API request/response types
#[derive(Debug, Clone)]
pub struct GeminiRequest {
    pub prompt: String,
    pub model: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub system_context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GeminiResponse {
    pub text: String,
    pub model: String,
    pub tokens_used: u32,
}

/// Low-level Gemini API client
#[derive(Clone)]
pub struct GeminiClient {
    client: Client,
    api_key: String,
    api_base: String,
    timeout: Duration,
}

#[derive(Error, Debug)]
pub enum GeminiClientError {
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
}

impl GeminiClient {
    /// Create a new Gemini API client
    pub fn new(api_key: String, timeout_ms: u64) -> Result<Self, GeminiClientError> {
        if api_key.is_empty() {
            return Err(GeminiClientError::InvalidRequest(
                "API key cannot be empty".to_string(),
            ));
        }

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(timeout_ms))
            .connection_verbose(false)
            .build()
            .map_err(|e| GeminiClientError::RequestFailed(e.to_string()))?;

        Ok(GeminiClient {
            client,
            api_key,
            api_base: "https://generativelanguage.googleapis.com/v1beta/models".to_string(),
            timeout: Duration::from_millis(timeout_ms),
        })
    }

    /// Send a request to Gemini API and get response
    pub async fn invoke(
        &self,
        request: GeminiRequest,
    ) -> Result<GeminiResponse, GeminiClientError> {
        let url = format!(
            "{}/{}:generateContent?key={}",
            self.api_base, request.model, self.api_key
        );

        let mut body = json!({
            "contents": [{
                "parts": [{
                    "text": request.prompt
                }]
            }]
        });

        // Add system context if provided
        if let Some(system) = request.system_context {
            body["system_instruction"] = json!({ "parts": [{ "text": system }] });
        }

        // Add generation config
        let mut gen_config = json!({});
        if let Some(temp) = request.temperature {
            gen_config["temperature"] = json!(temp);
        }
        if let Some(max_tok) = request.max_tokens {
            gen_config["maxOutputTokens"] = json!(max_tok);
        }
        if !gen_config.is_null() && gen_config.as_object().map_or(false, |m| !m.is_empty()) {
            body["generationConfig"] = gen_config;
        }

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    GeminiClientError::Timeout(self.timeout.as_millis() as u64)
                } else {
                    GeminiClientError::RequestFailed(e.to_string())
                }
            })?;

        let status = response.status();

        match status.as_u16() {
            200 => {
                let json: Value = response
                    .json()
                    .await
                    .map_err(|e| GeminiClientError::ParseError(e.to_string()))?;

                self.parse_response(&json, &request.model)
            },
            429 => {
                // Rate limited - extract retry-after if available
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(1000);

                Err(GeminiClientError::RateLimited(retry_after))
            },
            400..=499 => {
                let json: Value = response.json().await.unwrap_or_else(|_| Value::Null);

                let error_msg = json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown client error");

                Err(GeminiClientError::ApiError(error_msg.to_string()))
            },
            500..=599 => {
                let json: Value = response.json().await.unwrap_or_else(|_| Value::Null);

                let error_msg = json
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .unwrap_or("Unknown server error");

                Err(GeminiClientError::ApiError(error_msg.to_string()))
            },
            _ => Err(GeminiClientError::ApiError(format!(
                "Unexpected status code: {}",
                status
            ))),
        }
    }

    /// Parse Gemini API response
    fn parse_response(
        &self,
        json: &Value,
        model: &str,
    ) -> Result<GeminiResponse, GeminiClientError> {
        let text = json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|candidate| candidate.get("content"))
            .and_then(|content| content.get("parts"))
            .and_then(|parts| parts.get(0))
            .and_then(|part| part.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| GeminiClientError::ParseError("No text in response".to_string()))?
            .to_string();

        // Parse token usage if available
        let tokens_used = json
            .get("usageMetadata")
            .and_then(|u| u.get("totalTokenCount"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        Ok(GeminiResponse {
            text,
            model: model.to_string(),
            tokens_used,
        })
    }

    /// Check API connectivity
    pub async fn health_check(&self) -> Result<(), GeminiClientError> {
        let url = format!(
            "{}/gemini-pro:countTokens?key={}",
            self.api_base, self.api_key
        );

        let body = json!({
            "contents": [{
                "parts": [{
                    "text": "test"
                }]
            }]
        });

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| GeminiClientError::RequestFailed(e.to_string()))?;

        match response.status().as_u16() {
            200 | 400 => Ok(()), // 400 is expected for invalid input, but shows API is reachable
            401 => Err(GeminiClientError::InvalidRequest(
                "Invalid API key".to_string(),
            )),
            _ => Err(GeminiClientError::ApiError(format!(
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
    fn test_gemini_client_creation_with_valid_key() {
        let client = GeminiClient::new("test-key-123".to_string(), 30000);
        assert!(client.is_ok());
    }

    #[test]
    fn test_gemini_client_creation_with_empty_key() {
        let client = GeminiClient::new(String::new(), 30000);
        assert!(client.is_err());
        match client {
            Err(GeminiClientError::InvalidRequest(msg)) => {
                assert!(msg.contains("API key cannot be empty"));
            },
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[test]
    fn test_parse_valid_response() {
        let client = GeminiClient::new("test-key".to_string(), 30000).unwrap();
        let json = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "text": "Hello, world!"
                    }]
                }
            }],
            "usageMetadata": {
                "totalTokenCount": 42
            }
        });

        let result = client.parse_response(&json, "gemini-pro");
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.text, "Hello, world!");
        assert_eq!(response.tokens_used, 42);
    }

    #[test]
    fn test_parse_invalid_response() {
        let client = GeminiClient::new("test-key".to_string(), 30000).unwrap();
        let json = json!({"invalid": "response"});

        let result = client.parse_response(&json, "gemini-pro");
        assert!(result.is_err());
    }
}
