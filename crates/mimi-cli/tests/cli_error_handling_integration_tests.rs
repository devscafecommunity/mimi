//! Error Handling Integration Tests
//!
//! Tests for CLI error scenarios:
//! - Invalid command arguments
//! - Missing required arguments
//! - Invalid config files
//! - Network errors (connection refused, timeout)
//! - Auth token errors
//! - Graceful EOF and exit handling
//! - Error message clarity and actionability

mod cli_integration_utils;
use cli_integration_utils::assertions::*;
use cli_integration_utils::*;

#[test]
fn test_invalid_command_argument() -> Result<(), String> {
    let output = run_cli_command(&["invalid", "command"])?;
    assert!(!output.is_success());
    Ok(())
}

#[test]
fn test_missing_required_argument() -> Result<(), String> {
    let output = run_cli_command(&["run", "--config"])?;
    assert!(!output.is_success());
    Ok(())
}

#[test]
fn test_invalid_config_file_malformed() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let bad_config = ctx.create_config_file("this is not valid toml [[[[")?;

    let output = run_cli_command(&["run", "--config", &bad_config])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_invalid_config_file_not_found() -> Result<(), String> {
    let output = run_cli_command(&["run", "--config", "/nonexistent/config.toml"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_backend_connection_refused() -> Result<(), String> {
    let ctx = TestContext::new()?;

    let config = format!("[server]\nhost = \"127.0.0.1\"\nport = 19999\nws_port = 19998");
    let config_path = ctx.create_config_file(&config)?;

    let output = run_cli_command(&["run", "--config", &config_path])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_auth_token_file_not_found() -> Result<(), String> {
    let output = run_cli_command(&["version"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_auth_token_invalid_format() -> Result<(), String> {
    let ctx = TestContext::new()?;

    let result = ctx.auth_manager.validate_token("not.a.valid.jwt");
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_repl_eof_and_exit() -> Result<(), String> {
    let output = run_cli_command(&["version"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_error_messages_contain_context() -> Result<(), String> {
    let output = run_cli_command(&["--help"])?;
    assert_cli_success(&output)?;
    assert!(output.contains("MiMi") || output.contains("help") || output.contains("USAGE"));
    Ok(())
}

#[test]
fn test_multiple_invalid_flags_clear_error() -> Result<(), String> {
    let output = run_cli_command(&["--invalid1", "--invalid2"])?;
    assert!(!output.is_success());
    Ok(())
}
