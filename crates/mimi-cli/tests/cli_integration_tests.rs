use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    Command::cargo_bin("mimi")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Multimodal Instruction Master Interface"));
}

#[test]
fn test_version_command() {
    Command::cargo_bin("mimi")
        .unwrap()
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("v"));
}

#[test]
fn test_version_json_format() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["version", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("version"));
}

#[test]
fn test_invalid_command_fails() {
    Command::cargo_bin("mimi")
        .unwrap()
        .arg("invalid-command")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_exec_with_dry_run() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["exec", "--dry-run", "test task"])
        .assert()
        .success();
}

#[test]
fn test_query_command() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["query", "test query"])
        .assert()
        .success();
}

#[test]
fn test_config_list() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["config", "list"])
        .assert()
        .success();
}

#[test]
fn test_verbose_flag() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["-v", "version"])
        .assert()
        .success();
}

#[test]
fn test_quiet_flag() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["-q", "version"])
        .assert()
        .success();
}

#[test]
fn test_output_format_json() {
    Command::cargo_bin("mimi")
        .unwrap()
        .args(&["--output", "json", "debug", "status"])
        .assert()
        .success();
}
