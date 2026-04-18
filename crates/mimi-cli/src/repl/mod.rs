//! Interactive REPL for continuous user interaction

pub mod completion;
pub mod editor;
pub mod r#loop;
pub mod special_commands;

pub use editor::ReplEditor;
pub use r#loop::run_repl;
pub use special_commands::SpecialCommand;

#[derive(Debug, Clone)]
pub struct ReplConfig {
    pub history_file: String,
    pub max_history: usize,
    pub no_history: bool,
    pub startup_script: Option<String>,
    pub completion: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            history_file: format!(
                "{}/.mimi/repl_history",
                std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
            ),
            max_history: 1000,
            no_history: false,
            startup_script: None,
            completion: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptState {
    Default,
    Following(u32),
    Debug,
    Continuation,
}

impl PromptState {
    pub fn prompt_string(&self) -> String {
        match self {
            Self::Default => "mimi> ".to_string(),
            Self::Following(task_id) => format!("mimi [task-{}]> ", task_id),
            Self::Debug => "mimi [debug]> ".to_string(),
            Self::Continuation => "... ".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ReplConfig::default();
        assert_eq!(config.max_history, 1000);
        assert!(!config.no_history);
        assert!(config.completion);
    }

    #[test]
    fn test_prompt_strings() {
        assert_eq!(PromptState::Default.prompt_string(), "mimi> ");
        assert_eq!(
            PromptState::Following(123).prompt_string(),
            "mimi [task-123]> "
        );
        assert_eq!(PromptState::Debug.prompt_string(), "mimi [debug]> ");
        assert_eq!(PromptState::Continuation.prompt_string(), "... ");
    }
}
