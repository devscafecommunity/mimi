use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterPriority {
    pub adapters: Vec<String>,
    pub check_interval_secs: u64,
}

impl AdapterPriority {
    pub fn new(adapters: Vec<String>, check_interval_secs: u64) -> Self {
        Self {
            adapters,
            check_interval_secs,
        }
    }

    pub async fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: AdapterPriority = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn first_adapter(&self) -> Option<String> {
        self.adapters.first().cloned()
    }

    pub fn fallback_chain(&self) -> Vec<String> {
        self.adapters.clone()
    }

    pub fn fallback_adapters(&self) -> Vec<String> {
        if self.adapters.is_empty() {
            vec![]
        } else {
            self.adapters[1..].to_vec()
        }
    }
}

impl Default for AdapterPriority {
    fn default() -> Self {
        Self {
            adapters: vec!["gemini".to_string(), "ollama".to_string()],
            check_interval_secs: 30,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_default() {
        let priority = AdapterPriority::default();
        assert!(!priority.adapters.is_empty());
        assert_eq!(priority.check_interval_secs, 30);
        assert_eq!(priority.adapters[0], "gemini");
    }

    #[test]
    fn test_priority_new_with_adapters() {
        let adapters = vec!["gemini".to_string(), "ollama".to_string()];
        let priority = AdapterPriority::new(adapters.clone(), 60);
        assert_eq!(priority.adapters, adapters);
        assert_eq!(priority.check_interval_secs, 60);
    }

    #[test]
    fn test_priority_order() {
        let adapters = vec![
            "gemini".to_string(),
            "ollama".to_string(),
            "mock".to_string(),
        ];
        let priority = AdapterPriority::new(adapters.clone(), 30);
        assert_eq!(priority.adapters, adapters);
    }

    #[test]
    fn test_priority_first_adapter() {
        let adapters = vec!["gemini".to_string(), "ollama".to_string()];
        let priority = AdapterPriority::new(adapters, 30);
        assert_eq!(priority.first_adapter(), Some("gemini".to_string()));
    }

    #[test]
    fn test_priority_first_adapter_empty() {
        let priority = AdapterPriority::new(vec![], 30);
        assert_eq!(priority.first_adapter(), None);
    }

    #[test]
    fn test_priority_fallback_chain() {
        let adapters = vec![
            "gemini".to_string(),
            "ollama".to_string(),
            "mock".to_string(),
        ];
        let priority = AdapterPriority::new(adapters, 30);
        let chain = priority.fallback_chain();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0], "gemini");
    }

    #[test]
    fn test_priority_fallback_adapters() {
        let adapters = vec![
            "gemini".to_string(),
            "ollama".to_string(),
            "mock".to_string(),
        ];
        let priority = AdapterPriority::new(adapters, 30);
        let fallbacks = priority.fallback_adapters();
        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[0], "ollama");
    }
}
