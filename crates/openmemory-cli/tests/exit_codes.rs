use std::error::Error;
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn Error>>;

fn run_openmemory(arguments: &[&str]) -> Result<Output, Box<dyn Error>> {
    let config = tempfile::tempdir()?;
    Ok(Command::new(env!("CARGO_BIN_EXE_openmemory"))
        .args(arguments)
        .env("XDG_CONFIG_HOME", config.path())
        .env_remove("OPENMEMORY_TOKEN")
        .env_remove("OPENMEMORY_HOST")
        .output()?)
}

#[test]
fn help_exits_zero() -> TestResult {
    let output = run_openmemory(&["--help"])?;
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("auth"));
    Ok(())
}

#[test]
fn a_usage_mistake_exits_two_and_writes_to_stderr() -> TestResult {
    let output = run_openmemory(&["page"])?;
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout.len(), 0);
    assert_ne!(output.stderr.len(), 0);
    Ok(())
}

#[test]
fn a_command_without_credentials_exits_four_and_names_the_fix() -> TestResult {
    let output = run_openmemory(&["--host", "http://127.0.0.1:9", "page", "list"])?;
    assert_eq!(output.status.code(), Some(4));
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("openmemory auth login"), "{message}");
    assert!(message.contains("OPENMEMORY_TOKEN"), "{message}");
    Ok(())
}

#[test]
fn a_command_without_a_server_exits_one_and_names_the_fix() -> TestResult {
    let output = run_openmemory(&["page", "list"])?;
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--host"));
    Ok(())
}

#[test]
fn completion_scripts_go_to_stdout() -> TestResult {
    let output = run_openmemory(&["completion", "bash"])?;
    assert_eq!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stdout).contains("openmemory"));
    Ok(())
}
