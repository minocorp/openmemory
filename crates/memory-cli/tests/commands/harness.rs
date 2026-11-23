use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::process::{Child, Command};

use crate::mock::TestResult;

const RUN_TIMEOUT: Duration = Duration::from_secs(60);

pub(crate) struct Home {
    directory: TempDir,
}

pub(crate) struct Run {
    pub(crate) success: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

impl Home {
    pub(crate) fn new() -> TestResult<Self> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    pub(crate) fn credentials_path(&self) -> PathBuf {
        self.directory
            .path()
            .join("memory")
            .join("credentials.json")
    }

    pub(crate) fn saved(&self) -> TestResult<Value> {
        Ok(serde_json::from_slice(&std::fs::read(
            self.credentials_path(),
        )?)?)
    }

    pub(crate) fn sign_in_with_expired_token(&self, origin: &str) -> TestResult {
        let saved = json!({
            "server": origin,
            "credentials": {
                "client_id": "mock-client",
                "token_response": {
                    "access_token": "access-1",
                    "token_type": "bearer",
                    "expires_in": 3600,
                    "refresh_token": "refresh-1",
                },
                "granted_scopes": [],
                "token_received_at": 1,
                "issuer": origin,
            },
        });
        let path = self.credentials_path();
        std::fs::create_dir_all(path.parent().ok_or("no parent")?)?;
        std::fs::write(path, serde_json::to_vec(&saved)?)?;
        Ok(())
    }

    pub(crate) fn command(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_memory"));
        command
            .args(arguments)
            .env("XDG_CONFIG_HOME", self.directory.path())
            .env("HOME", self.directory.path())
            .stdin(Stdio::null())
            .kill_on_drop(true);
        command
    }

    pub(crate) async fn run(&self, arguments: &[&str]) -> TestResult<Run> {
        let output = tokio::time::timeout(RUN_TIMEOUT, self.command(arguments).output()).await??;
        Ok(Run {
            success: output.status.success(),
            stdout: String::from_utf8(output.stdout)?,
            stderr: String::from_utf8(output.stderr)?,
        })
    }

    pub(crate) fn spawn(&self, arguments: &[&str]) -> TestResult<Child> {
        Ok(self
            .command(arguments)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?)
    }
}
