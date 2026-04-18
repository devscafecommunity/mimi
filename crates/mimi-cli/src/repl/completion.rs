//! Command completion for REPL (stub for future enhancement)

#[derive(Debug, Clone)]
pub struct CompletionProvider;

impl CompletionProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn complete(&self, _partial: &str) -> Vec<String> {
        vec![]
    }
}

impl Default for CompletionProvider {
    fn default() -> Self {
        Self::new()
    }
}
