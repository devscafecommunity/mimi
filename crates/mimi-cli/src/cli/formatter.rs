use crate::cli::OutputFormat;

/// Formats command output
pub struct Formatter {
    format: OutputFormat,
    no_color: bool,
}

impl Formatter {
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        Formatter { format, no_color }
    }

    pub fn format_success(&self, _message: &str) -> String {
        "OK".to_string()
    }

    pub fn format_kv(&self, _pairs: &[(&str, &str)]) -> String {
        "".to_string()
    }

    pub fn format_status(&self, _state: &str, _uptime: u64) -> String {
        "".to_string()
    }
}
