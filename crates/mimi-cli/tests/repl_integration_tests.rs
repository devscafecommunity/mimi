use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_repl_command_exists() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.arg("repl").arg("--help").assert().success();
}

#[test]
fn test_repl_with_exit() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin(".exit\n").arg("repl").assert().success();
}

#[test]
fn test_repl_status_command() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin(".status\n.exit\n")
        .arg("repl")
        .assert()
        .success();
}

#[test]
fn test_repl_help_command() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin(".help\n.exit\n")
        .arg("repl")
        .assert()
        .success();
}

#[test]
fn test_repl_clear_command() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin(".clear\n.exit\n")
        .arg("repl")
        .assert()
        .success();
}

#[test]
fn test_repl_shell_command() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin("!echo test\n.exit\n")
        .arg("repl")
        .assert()
        .success();
}

#[test]
fn test_repl_multiline_input() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin("test \\\ncontinued\n.exit\n")
        .arg("repl")
        .assert()
        .success();
}

#[test]
fn test_repl_quit_command() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.write_stdin(".quit\n").arg("repl").assert().success();
}

#[test]
fn test_repl_startup_script_flag() {
    let dir = tempdir().unwrap();
    let script_path = dir.path().join("startup.mimi");
    fs::write(&script_path, ".status\n").unwrap();

    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.arg("repl")
        .arg("--startup-script")
        .arg(script_path.to_str().unwrap())
        .write_stdin(".exit\n")
        .assert()
        .success();
}

#[test]
fn test_repl_max_history_flag() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.arg("repl")
        .arg("--max-history")
        .arg("500")
        .write_stdin(".exit\n")
        .assert()
        .success();
}

#[test]
fn test_repl_no_history_flag() {
    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.arg("repl")
        .arg("--no-history")
        .write_stdin(".exit\n")
        .assert()
        .success();
}

#[test]
fn test_repl_history_file_flag() {
    let dir = tempdir().unwrap();
    let history_path = dir.path().join("history");

    let mut cmd = Command::cargo_bin("mimi").unwrap();
    cmd.arg("repl")
        .arg("--history-file")
        .arg(history_path.to_str().unwrap())
        .write_stdin(".exit\n")
        .assert()
        .success();
}
