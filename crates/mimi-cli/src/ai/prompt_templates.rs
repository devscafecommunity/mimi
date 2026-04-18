pub struct PromptTemplate {
    system_context: Option<String>,
    user_prompt: String,
}

impl PromptTemplate {
    pub fn new(user_prompt: impl Into<String>) -> Self {
        PromptTemplate {
            system_context: None,
            user_prompt: user_prompt.into(),
        }
    }

    pub fn with_system_context(mut self, context: impl Into<String>) -> Self {
        self.system_context = Some(context.into());
        self
    }

    pub fn get_system_context(&self) -> Option<&str> {
        self.system_context.as_deref()
    }

    pub fn get_user_prompt(&self) -> &str {
        &self.user_prompt
    }

    pub fn build(&self) -> String {
        if let Some(ref system) = self.system_context {
            format!("{}\n\n{}", system, self.user_prompt)
        } else {
            self.user_prompt.clone()
        }
    }
}

pub mod system_contexts {
    pub const GENERAL_ASSISTANT: &str =
        "You are a helpful AI assistant. Answer questions accurately and concisely.";

    pub const CODE_HELPER: &str = 
        "You are an expert programmer. Help with code questions and provide well-structured solutions.";

    pub const RESEARCH_ASSISTANT: &str =
        "You are a research assistant. Provide factual, well-sourced information.";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_basic() {
        let template = PromptTemplate::new("What is Rust?");
        assert_eq!(template.get_user_prompt(), "What is Rust?");
        assert_eq!(template.get_system_context(), None);
    }

    #[test]
    fn test_prompt_template_with_context() {
        let template = PromptTemplate::new("What is Rust?")
            .with_system_context("You are a programming expert.");

        assert_eq!(template.get_user_prompt(), "What is Rust?");
        assert_eq!(
            template.get_system_context(),
            Some("You are a programming expert.")
        );
    }

    #[test]
    fn test_prompt_template_build_without_context() {
        let template = PromptTemplate::new("What is Rust?");
        assert_eq!(template.build(), "What is Rust?");
    }

    #[test]
    fn test_prompt_template_build_with_context() {
        let template = PromptTemplate::new("What is Rust?")
            .with_system_context("You are a programming expert.");

        let built = template.build();
        assert!(built.contains("You are a programming expert."));
        assert!(built.contains("What is Rust?"));
    }

    #[test]
    fn test_system_contexts_constants() {
        assert!(!system_contexts::GENERAL_ASSISTANT.is_empty());
        assert!(!system_contexts::CODE_HELPER.is_empty());
        assert!(!system_contexts::RESEARCH_ASSISTANT.is_empty());
    }
}
