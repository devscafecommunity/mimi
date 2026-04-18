use crate::cli::OutputFormat;
use serde_json::{json, Value};
use std::fmt::Write;

/// Formats command output in different styles
pub struct Formatter {
    format: OutputFormat,
    no_color: bool,
}

impl Formatter {
    pub fn new(format: OutputFormat, no_color: bool) -> Self {
        Formatter { format, no_color }
    }

    /// Format status output
    pub fn format_status(&self, state: &str, uptime_seconds: u64) -> String {
        let uptime = format_duration(uptime_seconds);
        match self.format {
            OutputFormat::Text => self.text_status(state, &uptime),
            OutputFormat::Json => self.json_status(state, uptime_seconds),
            OutputFormat::Yaml => self.yaml_status(state, uptime_seconds),
        }
    }

    /// Format success message
    pub fn format_success(&self, message: &str) -> String {
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    format!("✓ {}", message)
                } else {
                    format!("\x1b[32m✓\x1b[0m {}", message)
                }
            },
            OutputFormat::Json => json!({"status": "success", "message": message}).to_string(),
            OutputFormat::Yaml => format!("status: success\nmessage: {}", message),
        }
    }

    /// Format error message
    pub fn format_error(&self, error: &str, code: &str) -> String {
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    format!("✗ ERROR [{}]: {}", code, error)
                } else {
                    format!("\x1b[31m✗\x1b[0m ERROR [{}]: {}", code, error)
                }
            },
            OutputFormat::Json => {
                json!({"status": "error", "code": code, "message": error}).to_string()
            },
            OutputFormat::Yaml => {
                format!("status: error\ncode: {}\nmessage: {}", code, error)
            },
        }
    }

    /// Format key-value output
    pub fn format_kv(&self, pairs: &[(&str, &str)]) -> String {
        match self.format {
            OutputFormat::Text => {
                let mut output = String::new();
                let max_key_len = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
                for (key, value) in pairs {
                    writeln!(output, "{:<width$} {}", key, value, width = max_key_len + 1).ok();
                }
                output
            },
            OutputFormat::Json => {
                let mut obj = serde_json::Map::new();
                for (key, value) in pairs {
                    obj.insert(key.to_string(), Value::String(value.to_string()));
                }
                serde_json::to_string_pretty(&Value::Object(obj)).unwrap_or_default()
            },
            OutputFormat::Yaml => {
                let mut output = String::new();
                for (key, value) in pairs {
                    writeln!(output, "{}: {}", key, value).ok();
                }
                output
            },
        }
    }

    // Private formatting methods
    fn text_status(&self, state: &str, uptime: &str) -> String {
        let state_display = if self.no_color {
            state.to_string()
        } else {
            format!("\x1b[36m{}\x1b[0m", state)
        };

        format!(
            "MiMi Status\n───────────\nState:  {}\nUptime: {}",
            state_display, uptime
        )
    }

    fn json_status(&self, state: &str, uptime_seconds: u64) -> String {
        json!({
            "state": state,
            "uptime_seconds": uptime_seconds,
        })
        .to_string()
    }

    fn yaml_status(&self, state: &str, uptime_seconds: u64) -> String {
        format!("state: {}\nuptime_seconds: {}", state, uptime_seconds)
    }
}

/// Format seconds to human-readable duration
fn format_duration(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(5), "5s");
        assert_eq!(format_duration(65), "1m 5s");
        assert_eq!(format_duration(3665), "1h 1m 5s");
    }

    #[test]
    fn test_formatter_text_status() {
        let fmt = Formatter::new(OutputFormat::Text, true);
        let output = fmt.format_status("Listening", 60);
        assert!(output.contains("MiMi Status"));
        assert!(output.contains("Listening"));
        assert!(output.contains("1m 0s"));
    }
}
