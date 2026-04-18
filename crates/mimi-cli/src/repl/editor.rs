//! Editor for REPL (Phase 1: minimal, Phase 2: rustyline)

#[derive(Debug, Clone)]
pub struct ReplEditor {
    history: Vec<String>,
    max_history: usize,
}

impl ReplEditor {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
        }
    }

    pub fn add_to_history(&mut self, line: &str) {
        self.history.push(line.to_string());
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn get_history(&self, count: usize) -> Vec<String> {
        let start = if self.history.len() > count {
            self.history.len() - count
        } else {
            0
        };
        self.history[start..].to_vec()
    }
}
