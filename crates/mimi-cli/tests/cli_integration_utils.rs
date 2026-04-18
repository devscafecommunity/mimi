//! CLI Integration Test Utilities
//!
//! Provides infrastructure for spawning test servers, CLI processes, and managing test lifecycle.

use std::collections::HashSet;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

use mimi_cli::auth::{AuthManager, Identity, Role};

/// Spawned CLI subprocess with output capture
pub struct CliProcess {
    child: Child,
    stdout_buffer: Arc<Mutex<String>>,
    stderr_buffer: Arc<Mutex<String>>,
}

impl CliProcess {
    /// Execute CLI command and return output
    pub fn output(self) -> Result<CliOutput, String> {
        let output = self
            .child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait on CLI process: {}", e))?;

        Ok(CliOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// Wait for process to complete with timeout
    pub async fn wait_timeout(mut self, duration: Duration) -> Result<CliOutput, String> {
        let start = std::time::Instant::now();
        loop {
            if let Ok(Some(status)) = self.child.try_wait() {
                let output = self
                    .child
                    .wait_with_output()
                    .map_err(|e| format!("Failed to capture output: {}", e))?;
                return Ok(CliOutput {
                    exit_code: status.code().unwrap_or(-1),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                });
            }

            if start.elapsed() > duration {
                let _ = self.child.kill();
                return Err(format!("CLI process timed out after {:?}", duration));
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

/// Captured CLI output
#[derive(Debug, Clone)]
pub struct CliOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutput {
    /// Check if exit code indicates success
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Assert exit code is 0
    pub fn assert_success(&self) -> Result<(), String> {
        if self.is_success() {
            Ok(())
        } else {
            Err(format!(
                "Expected exit code 0, got {}. stderr: {}",
                self.exit_code, self.stderr
            ))
        }
    }

    /// Assert exit code matches expected
    pub fn assert_exit_code(&self, expected: i32) -> Result<(), String> {
        if self.exit_code == expected {
            Ok(())
        } else {
            Err(format!(
                "Expected exit code {}, got {}. stderr: {}",
                expected, self.exit_code, self.stderr
            ))
        }
    }

    /// Parse stdout as JSON
    pub fn json(&self) -> Result<serde_json::Value, String> {
        serde_json::from_str(&self.stdout).map_err(|e| {
            format!(
                "Failed to parse stdout as JSON: {}. stdout: {}",
                e, self.stdout
            )
        })
    }

    /// Check if output contains substring
    pub fn contains(&self, needle: &str) -> bool {
        self.stdout.contains(needle) || self.stderr.contains(needle)
    }
}

/// Test server configuration
pub struct TestServerConfig {
    pub host: String,
    pub port: u16,
    pub ws_port: u16,
}

impl TestServerConfig {
    pub fn http_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    pub fn ws_url(&self) -> String {
        format!("ws://{}:{}", self.host, self.ws_port)
    }
}

impl Default for TestServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ws_port: 8081,
        }
    }
}

/// Test configuration and context
pub struct TestContext {
    pub server_config: TestServerConfig,
    pub auth_manager: Arc<AuthManager>,
    pub temp_dir: TempDir,
    pub temp_config_path: String,
}

impl TestContext {
    /// Create new test context
    pub fn new() -> Result<Self, String> {
        let temp_dir = TempDir::new().map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let temp_config_path = temp_dir
            .path()
            .join("test-config.toml")
            .to_string_lossy()
            .to_string();

        let auth_manager = Arc::new(AuthManager::new(
            "test-secret-key-32-chars-min!!!".to_string(),
            3600,
        ));

        auth_manager.register_default_policies();

        Ok(TestContext {
            server_config: TestServerConfig::default(),
            auth_manager,
            temp_dir,
            temp_config_path,
        })
    }

    /// Generate test token for identity
    pub fn generate_token(&self, identity: Identity) -> Result<String, String> {
        self.auth_manager
            .generate_token(&identity)
            .map_err(|e| format!("Failed to generate token: {}", e))
    }

    /// Create admin identity token
    pub fn create_admin_token(&self) -> Result<String, String> {
        let identity = Identity {
            user_id: "admin-1".to_string(),
            username: "admin".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Admin);
                set
            },
        };
        self.generate_token(identity)
    }

    /// Create user identity token
    pub fn create_user_token(&self) -> Result<String, String> {
        let identity = Identity {
            user_id: "user-1".to_string(),
            username: "testuser".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::User);
                set
            },
        };
        self.generate_token(identity)
    }

    /// Create guest identity token
    pub fn create_guest_token(&self) -> Result<String, String> {
        let identity = Identity {
            user_id: "guest-1".to_string(),
            username: "guest".to_string(),
            roles: {
                let mut set = HashSet::new();
                set.insert(Role::Guest);
                set
            },
        };
        self.generate_token(identity)
    }

    /// Create temporary config file
    pub fn create_config_file(&self, content: &str) -> Result<String, String> {
        std::fs::write(&self.temp_config_path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))?;
        Ok(self.temp_config_path.clone())
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new().expect("Failed to create TestContext")
    }
}

/// Run CLI command with given arguments
pub fn run_cli_command(args: &[&str]) -> Result<CliOutput, String> {
    let output = Command::new("cargo")
        .args(&["run", "-p", "mimi-cli", "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute CLI command: {}", e))?;

    Ok(CliOutput {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Run CLI command and spawn as process
pub fn spawn_cli_command(args: &[&str]) -> Result<CliProcess, String> {
    let child = Command::new("cargo")
        .args(&["run", "-p", "mimi-cli", "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn CLI process: {}", e))?;

    Ok(CliProcess {
        child,
        stdout_buffer: Arc::new(Mutex::new(String::new())),
        stderr_buffer: Arc::new(Mutex::new(String::new())),
    })
}

/// Assertion helpers
pub mod assertions {
    use super::*;

    /// Assert CLI succeeded (exit code 0)
    pub fn assert_cli_success(output: &CliOutput) -> Result<(), String> {
        output.assert_success()
    }

    /// Assert CLI failed (exit code != 0)
    pub fn assert_cli_error(output: &CliOutput) -> Result<(), String> {
        if !output.is_success() {
            Ok(())
        } else {
            Err("Expected non-zero exit code but got 0".to_string())
        }
    }

    /// Assert exit code matches
    pub fn assert_exit_code(output: &CliOutput, expected: i32) -> Result<(), String> {
        output.assert_exit_code(expected)
    }

    /// Assert output contains text
    pub fn assert_output_contains(output: &CliOutput, text: &str) -> Result<(), String> {
        if output.contains(text) {
            Ok(())
        } else {
            Err(format!(
                "Expected output to contain '{}', but got: stdout={}, stderr={}",
                text, output.stdout, output.stderr
            ))
        }
    }

    /// Assert JSON output is valid and matches predicate
    pub fn assert_json_output<F>(output: &CliOutput, predicate: F) -> Result<(), String>
    where
        F: FnOnce(&serde_json::Value) -> bool,
    {
        let json = output.json()?;
        if predicate(&json) {
            Ok(())
        } else {
            Err(format!("JSON predicate failed for: {}", json))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_output_success() {
        let output = CliOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: "".to_string(),
        };
        assert!(output.is_success());
        assert!(output.assert_success().is_ok());
    }

    #[test]
    fn test_cli_output_error() {
        let output = CliOutput {
            exit_code: 1,
            stdout: "".to_string(),
            stderr: "error".to_string(),
        };
        assert!(!output.is_success());
        assert!(output.assert_success().is_err());
    }

    #[test]
    fn test_cli_output_contains() {
        let output = CliOutput {
            exit_code: 0,
            stdout: "hello world".to_string(),
            stderr: "".to_string(),
        };
        assert!(output.contains("hello"));
        assert!(!output.contains("foo"));
    }

    #[test]
    fn test_test_context_creation() -> Result<(), String> {
        let ctx = TestContext::new()?;
        assert!(!ctx.temp_config_path.is_empty());
        Ok(())
    }

    #[test]
    fn test_test_context_admin_token() -> Result<(), String> {
        let ctx = TestContext::new()?;
        let token = ctx.create_admin_token()?;
        assert!(!token.is_empty());

        // Validate token
        let identity = ctx
            .auth_manager
            .validate_token(&token)
            .map_err(|e| format!("Failed to validate token: {}", e))?;
        assert_eq!(identity.username, "admin");
        assert!(identity.roles.contains(&Role::Admin));
        Ok(())
    }
}
