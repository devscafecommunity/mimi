use super::{adapter::*, error::AdapterResult};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global adapter registry
pub struct AdapterRegistry {
    adapters: RwLock<HashMap<String, SharedAdapter>>,
}

impl AdapterRegistry {
    /// Create new registry
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            adapters: RwLock::new(HashMap::new()),
        })
    }

    /// Register adapter by name
    pub async fn register(&self, name: String, adapter: SharedAdapter) -> AdapterResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.insert(name, adapter);
        Ok(())
    }

    /// Get adapter by name
    pub async fn get(&self, name: &str) -> AdapterResult<SharedAdapter> {
        let adapters = self.adapters.read().await;
        adapters.get(name).cloned().ok_or_else(|| {
            super::error::AdapterError::AdapterNotFound(format!("adapter not found: {}", name))
        })
    }

    /// List all registered adapters
    pub async fn list(&self) -> Vec<String> {
        let adapters = self.adapters.read().await;
        adapters.keys().cloned().collect()
    }

    /// Remove adapter by name
    pub async fn remove(&self, name: &str) -> AdapterResult<()> {
        let mut adapters = self.adapters.write().await;
        adapters.remove(name);
        Ok(())
    }
}
