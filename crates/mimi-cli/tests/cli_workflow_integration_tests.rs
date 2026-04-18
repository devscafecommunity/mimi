//! CLI Workflow Integration Tests
//!
//! Tests for end-to-end CLI command workflows including:
//! - Argument parsing and validation
//! - Config file handling
//! - Output formatting (JSON, table, raw)
//! - Logging levels

mod cli_integration_utils;
use cli_integration_utils::assertions::*;
use cli_integration_utils::*;

#[test]
fn test_version_command_success() -> Result<(), String> {
    let output = run_cli_command(&["--version"])?;
    assert_cli_success(&output)?;
    assert_output_contains(&output, "0.1.0")?;
    Ok(())
}

#[test]
fn test_version_command_long_form() -> Result<(), String> {
    let output = run_cli_command(&["version"])?;
    assert_cli_success(&output)?;
    assert_output_contains(&output, "mimi")?;
    Ok(())
}

#[test]
fn test_help_command() -> Result<(), String> {
    let output = run_cli_command(&["--help"])?;
    assert_cli_success(&output)?;
    assert_output_contains(&output, "MiMi")?;
    assert_output_contains(&output, "help")?;
    Ok(())
}

#[test]
fn test_verbose_flag() -> Result<(), String> {
    let output = run_cli_command(&["--verbose", "version"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_run_command_with_config() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let config_path = ctx.create_config_file("[server]\nhost = \"127.0.0.1\"\nport = 8080")?;

    let output = run_cli_command(&["run", "--config", &config_path])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_run_command_without_config_uses_defaults() -> Result<(), String> {
    let output = run_cli_command(&["run"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_run_command_with_invalid_config_path() -> Result<(), String> {
    let output = run_cli_command(&["run", "--config", "/nonexistent/path/config.toml"])?;
    assert_cli_success(&output)?;
    Ok(())
}

#[test]
fn test_invalid_command_argument() -> Result<(), String> {
    let output = run_cli_command(&["--invalid-flag"])?;
    assert_cli_error(&output)?;
    Ok(())
}

#[test]
fn test_subcommand_without_required_args() -> Result<(), String> {
    let output = run_cli_command(&["run", "--config"])?;
    assert_cli_error(&output)?;
    Ok(())
}

#[test]
fn test_multiple_flags_combined() -> Result<(), String> {
    let ctx = TestContext::new()?;
    let config_path = ctx.create_config_file("[server]\nhost = \"127.0.0.1\"")?;

    let output = run_cli_command(&["--verbose", "run", "--config", &config_path])?;
    assert_cli_success(&output)?;
    Ok(())
}
