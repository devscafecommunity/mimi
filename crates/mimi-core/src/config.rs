use serde::{Deserialize, Serialize};

/// System configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub log_level: String,
    pub workers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            workers: num_cpus::get(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.workers > 0);
    }
}
