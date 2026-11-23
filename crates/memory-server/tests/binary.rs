use std::process::Command;

const BINARY: &str = env!("CARGO_BIN_EXE_memory-server");
const UNREACHABLE_DATABASE: &str = "postgres://memory:memory@127.0.0.1:1/memory";

#[test]
fn the_server_reads_its_configuration_from_the_environment_alone() {
    let output = Command::new(BINARY)
        .env("DATABASE_URL", UNREACHABLE_DATABASE)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(!stderr.contains("configuration is missing"), "{stderr}");
    assert!(!stderr.contains("DATABASE_URL"), "{stderr}");
}

#[test]
fn the_openapi_command_prints_the_document_without_configuration() {
    let output = Command::new(BINARY)
        .arg("openapi")
        .env_remove("DATABASE_URL")
        .output()
        .unwrap();
    assert!(output.status.success());
    let document: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(document["openapi"].is_string());
}
