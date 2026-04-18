//! Special commands for REPL (prefixed with .)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecialCommand {
    Help(Option<String>),
    Status,
    Clear,
    ConfigGet(String),
    ConfigSet(String, String),
    History(Option<usize>),
    Exit,
    Shell(String),
}

impl SpecialCommand {
    pub fn parse(line: &str) -> Option<Self> {
        if line.is_empty() {
            return None;
        }

        let trimmed = line.trim();

        if trimmed.starts_with('!') {
            let cmd = trimmed[1..].to_string();
            return Some(Self::Shell(cmd));
        }

        if !trimmed.starts_with('.') {
            return None;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();

        match parts.as_slice() {
            [".help"] => Some(Self::Help(None)),
            [".help", cmd] => Some(Self::Help(Some(cmd.to_string()))),

            [".status"] => Some(Self::Status),

            [".clear"] => Some(Self::Clear),

            [".config", "get", key] => Some(Self::ConfigGet(key.to_string())),
            [".config", "set", key, value] => {
                Some(Self::ConfigSet(key.to_string(), value.to_string()))
            },

            [".history"] => Some(Self::History(None)),
            [".history", n_str] => n_str.parse::<usize>().ok().map(|n| Self::History(Some(n))),

            [".exit"] | [".quit"] => Some(Self::Exit),

            _ => None,
        }
    }

    pub fn execute(&self) -> String {
        match self {
            Self::Help(cmd) => {
                if let Some(cmd_name) = cmd {
                    format!("Help for {}: (detailed help to be implemented)", cmd_name)
                } else {
                    "Special REPL commands:\n  .help [COMMAND]     - Show help\n  .status             - Show system status\n  .clear              - Clear screen\n  .config get <KEY>   - Get config value\n  .config set <KEY> <VAL> - Set config value\n  .history [N]        - Show last N commands\n  .exit / .quit       - Exit REPL\n  !<CMD>              - Run shell command".to_string()
                }
            },

            Self::Status => "System status: OK".to_string(),

            Self::Clear => "\x1B[2J\x1B[H".to_string(),

            Self::ConfigGet(key) => {
                format!("{} = (config not available in Phase 1)", key)
            },

            Self::ConfigSet(key, value) => format!("Set {} = {}", key, value),

            Self::History(_n) => "History (last entries):\n(to be populated by REPL)".to_string(),

            Self::Exit => "exit".to_string(),

            Self::Shell(cmd) => {
                match std::process::Command::new("sh").arg("-c").arg(cmd).output() {
                    Ok(output) => String::from_utf8_lossy(&output.stdout).to_string(),
                    Err(e) => format!("Shell error: {}", e),
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(
            SpecialCommand::parse(".help"),
            Some(SpecialCommand::Help(None))
        );
        assert_eq!(
            SpecialCommand::parse(".help exec"),
            Some(SpecialCommand::Help(Some("exec".to_string())))
        );
    }

    #[test]
    fn test_parse_status() {
        assert_eq!(
            SpecialCommand::parse(".status"),
            Some(SpecialCommand::Status)
        );
    }

    #[test]
    fn test_parse_clear() {
        assert_eq!(SpecialCommand::parse(".clear"), Some(SpecialCommand::Clear));
    }

    #[test]
    fn test_parse_config() {
        assert_eq!(
            SpecialCommand::parse(".config get key"),
            Some(SpecialCommand::ConfigGet("key".to_string()))
        );
        assert_eq!(
            SpecialCommand::parse(".config set key value"),
            Some(SpecialCommand::ConfigSet(
                "key".to_string(),
                "value".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_history() {
        assert_eq!(
            SpecialCommand::parse(".history"),
            Some(SpecialCommand::History(None))
        );
        assert_eq!(
            SpecialCommand::parse(".history 30"),
            Some(SpecialCommand::History(Some(30)))
        );
    }

    #[test]
    fn test_parse_exit() {
        assert_eq!(SpecialCommand::parse(".exit"), Some(SpecialCommand::Exit));
        assert_eq!(SpecialCommand::parse(".quit"), Some(SpecialCommand::Exit));
    }

    #[test]
    fn test_parse_shell() {
        assert_eq!(
            SpecialCommand::parse("!ls -la"),
            Some(SpecialCommand::Shell("ls -la".to_string()))
        );
    }

    #[test]
    fn test_parse_invalid() {
        assert_eq!(SpecialCommand::parse("invalid"), None);
        assert_eq!(SpecialCommand::parse(".unknown"), None);
    }
}
