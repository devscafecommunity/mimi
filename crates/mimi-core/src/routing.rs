//! Message Routing Middleware
//!
//! Provides topic-based message dispatch with MQTT-style wildcard pattern matching,
//! handler registration, and thread-safe concurrent access.

use anyhow::{anyhow, Result};
use log;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Routing error types with detailed context
#[derive(Debug, Clone)]
pub enum RoutingError {
    /// No handler found for the given topic
    NoHandlerFound { topic: String },
    /// Topic format is invalid
    InvalidTopic { topic: String, reason: String },
    /// Pattern format is invalid
    InvalidPattern { pattern: String, reason: String },
    /// Handler execution failed
    HandlerFailed { pattern: String, error: String },
}

impl fmt::Display for RoutingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHandlerFound { topic } => {
                write!(f, "No handler found for topic: {}", topic)
            },
            Self::InvalidTopic { topic, reason } => {
                write!(f, "Invalid topic '{}': {}", topic, reason)
            },
            Self::InvalidPattern { pattern, reason } => {
                write!(f, "Invalid pattern '{}': {}", pattern, reason)
            },
            Self::HandlerFailed { pattern, error } => {
                write!(f, "Handler failed for pattern '{}': {}", pattern, error)
            },
        }
    }
}

impl std::error::Error for RoutingError {}

/// Represents a validated topic string
/// Format: segments separated by '/', no empty segments, valid characters only
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Topic {
    value: String,
}

impl Topic {
    /// Create a new topic with validation
    pub fn new(topic: &str) -> Result<Self> {
        validate_topic(topic)?;
        Ok(Self {
            value: topic.to_string(),
        })
    }

    /// Get the topic as a string slice
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Get segments (split by '/')
    pub fn segments(&self) -> Vec<&str> {
        self.value.split('/').collect()
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Validate topic format
fn validate_topic(topic: &str) -> Result<()> {
    if topic.is_empty() {
        return Err(anyhow!(RoutingError::InvalidTopic {
            topic: topic.to_string(),
            reason: "topic cannot be empty".to_string(),
        }));
    }

    // Check for invalid characters
    if !topic
        .chars()
        .all(|c| c.is_alphanumeric() || c == '/' || c == '-' || c == '_')
    {
        return Err(anyhow!(RoutingError::InvalidTopic {
            topic: topic.to_string(),
            reason: "invalid characters (allowed: alphanumeric, /, -, _)".to_string(),
        }));
    }

    // Check for empty segments (e.g., "mimi//commands")
    if topic.contains("//") {
        return Err(anyhow!(RoutingError::InvalidTopic {
            topic: topic.to_string(),
            reason: "empty segments not allowed".to_string(),
        }));
    }

    // Check for leading/trailing slashes
    if topic.starts_with('/') || topic.ends_with('/') {
        return Err(anyhow!(RoutingError::InvalidTopic {
            topic: topic.to_string(),
            reason: "leading/trailing slashes not allowed".to_string(),
        }));
    }

    Ok(())
}

/// Represents a topic pattern with wildcards
/// * matches exactly one segment
/// # matches zero or more segments (only valid at end)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicPattern {
    value: String,
}

impl TopicPattern {
    /// Create a new pattern with validation
    pub fn new(pattern: &str) -> Result<Self> {
        validate_pattern(pattern)?;
        Ok(Self {
            value: pattern.to_string(),
        })
    }

    /// Check if this pattern matches a topic
    pub fn matches(&self, topic: &str) -> bool {
        match_pattern(&self.value, topic)
    }

    /// Get the pattern as a string slice
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for TopicPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

/// Validate pattern format
fn validate_pattern(pattern: &str) -> Result<()> {
    if pattern.is_empty() {
        return Err(anyhow!(RoutingError::InvalidPattern {
            pattern: pattern.to_string(),
            reason: "pattern cannot be empty".to_string(),
        }));
    }

    // Check for invalid characters (only /, -, _, *, #)
    if !pattern
        .chars()
        .all(|c| c.is_alphanumeric() || c == '/' || c == '-' || c == '_' || c == '*' || c == '#')
    {
        return Err(anyhow!(RoutingError::InvalidPattern {
            pattern: pattern.to_string(),
            reason: "invalid characters (allowed: alphanumeric, /, -, _, *, #)".to_string(),
        }));
    }

    // Check for empty segments
    if pattern.contains("//") {
        return Err(anyhow!(RoutingError::InvalidPattern {
            pattern: pattern.to_string(),
            reason: "empty segments not allowed".to_string(),
        }));
    }

    // Check for leading/trailing slashes
    if pattern.starts_with('/') || pattern.ends_with('/') {
        return Err(anyhow!(RoutingError::InvalidPattern {
            pattern: pattern.to_string(),
            reason: "leading/trailing slashes not allowed".to_string(),
        }));
    }

    // # only valid at end
    if let Some(hash_pos) = pattern.find('#') {
        if hash_pos != pattern.len() - 1 {
            return Err(anyhow!(RoutingError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "# wildcard only valid at end of pattern".to_string(),
            }));
        }
        // # must be preceded by /
        if hash_pos == 0 {
            return Err(anyhow!(RoutingError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "# must follow a segment separator (/)".to_string(),
            }));
        }
        if pattern.chars().nth(hash_pos - 1) != Some('/') {
            return Err(anyhow!(RoutingError::InvalidPattern {
                pattern: pattern.to_string(),
                reason: "# must follow a segment separator (/)".to_string(),
            }));
        }
    }

    Ok(())
}

/// Match a pattern against a topic
fn match_pattern(pattern: &str, topic: &str) -> bool {
    let pattern_segments: Vec<&str> = pattern.split('/').collect();
    let topic_segments: Vec<&str> = topic.split('/').collect();

    match_segments(&pattern_segments, &topic_segments)
}

/// Recursively match pattern segments against topic segments
fn match_segments(patterns: &[&str], topics: &[&str]) -> bool {
    // Base case: both consumed
    if patterns.is_empty() && topics.is_empty() {
        return true;
    }

    // If patterns consumed but topics remain
    if patterns.is_empty() {
        return false;
    }

    let pattern = patterns[0];

    // Handle # wildcard (multi-level, only at end)
    if pattern == "#" {
        // # matches zero or more remaining topics
        return true;
    }

    // If topics consumed but patterns remain (and pattern is not #)
    if topics.is_empty() {
        return false;
    }

    let topic = topics[0];

    // Handle * wildcard (single-level)
    if pattern == "*" {
        return match_segments(&patterns[1..], &topics[1..]);
    }

    // Exact match
    if pattern == topic {
        return match_segments(&patterns[1..], &topics[1..]);
    }

    false
}

/// Handler function type: receives topic and serialized payload, returns Result
pub type Handler = Box<dyn Fn(&str, &[u8]) -> Result<()> + Send + Sync>;

/// Message router with topic-based dispatch
/// Thread-safe via Arc<Mutex<>>
pub struct MessageRouter {
    handlers: Arc<Mutex<HashMap<String, Vec<Handler>>>>,
}

impl MessageRouter {
    /// Create a new message router
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a handler for a topic pattern
    pub fn register<F>(&self, pattern: &str, handler: F) -> Result<()>
    where
        F: Fn(&str, &[u8]) -> Result<()> + Send + Sync + 'static,
    {
        // Validate pattern
        TopicPattern::new(pattern)?;

        let mut handlers = self.handlers.lock().unwrap();
        let entry = handlers.entry(pattern.to_string()).or_default();
        entry.push(Box::new(handler));

        log::debug!("Registered handler for pattern: {}", pattern);
        Ok(())
    }

    /// Route a message to all matching handlers
    pub fn route(&self, topic: &str, payload: &[u8]) -> Result<()> {
        // Validate topic
        Topic::new(topic)?;

        let handlers = self.handlers.lock().unwrap();

        // Find all matching patterns
        let mut matching_patterns: Vec<&String> = handlers
            .keys()
            .filter(|pattern| {
                if let Ok(p) = TopicPattern::new(pattern) {
                    p.matches(topic)
                } else {
                    false
                }
            })
            .collect();

        if matching_patterns.is_empty() {
            log::warn!("No handlers found for topic: {}", topic);
            return Err(anyhow!(RoutingError::NoHandlerFound {
                topic: topic.to_string(),
            }));
        }

        // Sort: exact matches first, then by pattern specificity
        matching_patterns.sort_by(|a, b| {
            let a_is_exact = *a == topic;
            let b_is_exact = *b == topic;

            if a_is_exact && !b_is_exact {
                std::cmp::Ordering::Less
            } else if !a_is_exact && b_is_exact {
                std::cmp::Ordering::Greater
            } else {
                // Both exact or both wildcard: sort by length (longer = more specific)
                b.len().cmp(&a.len())
            }
        });

        log::debug!(
            "Routing message to topic: {} (found {} matching patterns)",
            topic,
            matching_patterns.len()
        );

        // Dispatch to all matching handlers
        let mut had_error = false;
        for pattern in matching_patterns {
            if let Some(pattern_handlers) = handlers.get(pattern) {
                for handler in pattern_handlers {
                    match handler(topic, payload) {
                        Ok(()) => {
                            log::debug!("Handler succeeded for pattern: {}", pattern);
                        },
                        Err(e) => {
                            log::error!("Handler failed for pattern: {}: {}", pattern, e);
                            had_error = true;
                        },
                    }
                }
            }
        }

        // Note: We don't fail if a handler fails (isolation), but we log it
        if had_error {
            log::warn!("One or more handlers failed for topic: {}", topic);
        }

        Ok(())
    }

    /// Unregister all handlers for a pattern
    pub fn unregister(&self, pattern: &str) -> Result<()> {
        TopicPattern::new(pattern)?;

        let mut handlers = self.handlers.lock().unwrap();
        handlers.remove(pattern);

        log::debug!("Unregistered pattern: {}", pattern);
        Ok(())
    }

    /// List all registered patterns
    pub fn list_subscriptions(&self) -> Vec<String> {
        let handlers = self.handlers.lock().unwrap();
        handlers.keys().cloned().collect()
    }
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_error_display() {
        let err = RoutingError::NoHandlerFound {
            topic: "mimi/commands/execute".to_string(),
        };
        assert!(err.to_string().contains("No handler found"));
        assert!(err.to_string().contains("mimi/commands/execute"));
    }

    #[test]
    fn test_valid_topic() {
        let topic = Topic::new("mimi/commands/execute").unwrap();
        assert_eq!(topic.as_str(), "mimi/commands/execute");
        assert_eq!(topic.segments(), vec!["mimi", "commands", "execute"]);
    }

    #[test]
    fn test_invalid_topic_empty() {
        let result = Topic::new("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_topic_empty_segment() {
        let result = Topic::new("mimi//commands");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_topic_leading_slash() {
        let result = Topic::new("/mimi/commands");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_topic_special_chars() {
        let result = Topic::new("mimi/commands@execute");
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_pattern() {
        let pattern = TopicPattern::new("mimi/commands/*").unwrap();
        assert_eq!(pattern.as_str(), "mimi/commands/*");
    }

    #[test]
    fn test_pattern_exact_match() {
        let pattern = TopicPattern::new("mimi/commands/execute").unwrap();
        assert!(pattern.matches("mimi/commands/execute"));
        assert!(!pattern.matches("mimi/commands/query"));
    }

    #[test]
    fn test_pattern_single_wildcard() {
        let pattern = TopicPattern::new("mimi/commands/*").unwrap();
        assert!(pattern.matches("mimi/commands/execute"));
        assert!(pattern.matches("mimi/commands/query"));
        assert!(!pattern.matches("mimi/commands/execute/retry"));
    }

    #[test]
    fn test_pattern_multi_wildcard() {
        let pattern = TopicPattern::new("mimi/#").unwrap();
        assert!(pattern.matches("mimi/commands/execute"));
        assert!(pattern.matches("mimi/events/state_changed"));
        assert!(pattern.matches("mimi"));
    }

    #[test]
    fn test_pattern_invalid_hash_position() {
        let result = TopicPattern::new("mimi/#/events");
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_invalid_hash_no_slash() {
        let result = TopicPattern::new("mimi#");
        assert!(result.is_err());
    }

    #[test]
    fn test_router_exact_match() {
        let router = MessageRouter::new();
        let called = Arc::new(Mutex::new(false));
        let called_clone = called.clone();

        router
            .register("mimi/commands/execute", move |_topic, _payload| {
                *called_clone.lock().unwrap() = true;
                Ok(())
            })
            .unwrap();

        router.route("mimi/commands/execute", b"test").unwrap();
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_router_no_handler() {
        let router = MessageRouter::new();
        let result = router.route("mimi/commands/execute", b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_router_list_subscriptions() {
        let router = MessageRouter::new();
        router.register("mimi/commands/*", |_, _| Ok(())).unwrap();
        router.register("mimi/events/*", |_, _| Ok(())).unwrap();

        let subs = router.list_subscriptions();
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&"mimi/commands/*".to_string()));
    }

    #[test]
    fn test_multiple_handlers_same_pattern() {
        let router = MessageRouter::new();
        let call_count = Arc::new(Mutex::new(0));

        let count1 = call_count.clone();
        router
            .register("mimi/commands/*", move |_, _| {
                *count1.lock().unwrap() += 1;
                Ok(())
            })
            .unwrap();

        let count2 = call_count.clone();
        router
            .register("mimi/commands/*", move |_, _| {
                *count2.lock().unwrap() += 1;
                Ok(())
            })
            .unwrap();

        router.route("mimi/commands/execute", b"test").unwrap();
        assert_eq!(*call_count.lock().unwrap(), 2);
    }

    #[test]
    fn test_handler_failure_isolation() {
        let router = MessageRouter::new();
        let success_called = Arc::new(Mutex::new(false));

        router
            .register("mimi/commands/*", |_, _| {
                Err(anyhow::anyhow!("Handler intentionally failed"))
            })
            .unwrap();

        let success_clone = success_called.clone();
        router
            .register("mimi/commands/*", move |_, _| {
                *success_clone.lock().unwrap() = true;
                Ok(())
            })
            .unwrap();

        let result = router.route("mimi/commands/execute", b"test");
        assert!(result.is_ok());
        assert!(*success_called.lock().unwrap());
    }

    #[test]
    fn test_invalid_topic_rejected() {
        let router = MessageRouter::new();
        router.register("mimi/commands/*", |_, _| Ok(())).unwrap();

        let result = router.route("mimi//commands/execute", b"test");
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_pattern() {
        let router = MessageRouter::new();
        router.register("mimi/commands/*", |_, _| Ok(())).unwrap();

        assert_eq!(router.list_subscriptions().len(), 1);
        router.unregister("mimi/commands/*").unwrap();
        assert_eq!(router.list_subscriptions().len(), 0);
    }
}
