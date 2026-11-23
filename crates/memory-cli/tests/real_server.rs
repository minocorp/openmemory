use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;

use memory_server::{AppState, router};
use object_store::memory::InMemory;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tokio::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const TOKEN: &str = "mpat_end-to-end-token";

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

async fn serve(pool: PgPool) -> Result<SocketAddr, Box<dyn Error>> {
    let state = AppState::new(pool, Arc::new(InMemory::new()), 1 << 20);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        let service = router(state).into_make_service_with_connect_info::<SocketAddr>();
        axum::serve(listener, service).await
    });
    Ok(address)
}

async fn personal_token(pool: &PgPool, can_write: bool) -> Result<(), Box<dyn Error>> {
    let user = sqlx::query_scalar!(
        "INSERT INTO users (username, display_name, user_handle, is_admin) VALUES ('ada', 'Ada', decode('00112233445566778899aabbccddeeff', 'hex'), true) RETURNING id"
    )
    .fetch_one(pool)
    .await?;
    let hash = Sha256::digest(TOKEN.as_bytes()).to_vec();
    sqlx::query!(
        "INSERT INTO connections (user_id, name, scopes, can_write, audience, expires_at, personal_token_hash) VALUES ($1, 'ci', ARRAY['*'], $2, 'api', now() + interval '1 day', $3)",
        user,
        can_write,
        hash,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn memory(
    address: SocketAddr,
    arguments: &[&str],
    stdin: &str,
) -> Result<Run, Box<dyn Error>> {
    let config = tempfile::tempdir()?;
    let mut child = Command::new(env!("CARGO_BIN_EXE_memory"))
        .args(arguments)
        .env("XDG_CONFIG_HOME", config.path())
        .env("MEMORY_HOST", format!("http://{address}"))
        .env("MEMORY_TOKEN", TOKEN)
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut input) = child.stdin.take() {
        tokio::io::AsyncWriteExt::write_all(&mut input, stdin.as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    Ok(Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[sqlx::test(migrations = "../memory-server/migrations")]
async fn the_cli_works_a_page_and_a_file_through_the_real_router(pool: PgPool) -> TestResult {
    personal_token(&pool, true).await?;
    let address = serve(pool).await?;

    let created = memory(
        address,
        &["page", "create", "clients/acme", "-"],
        "---\nstatus: active\n---\nHello\n",
    )
    .await?;
    assert_eq!(created.code, Some(0), "{}", created.stderr);

    let viewed = memory(address, &["--json", "page", "view", "clients/acme"], "").await?;
    assert_eq!(viewed.code, Some(0), "{}", viewed.stderr);
    assert!(
        viewed.stdout.contains("\"path\": \"clients/acme\"")
            || viewed.stdout.contains("\"path\":\"clients/acme\"")
    );

    let appended = memory(
        address,
        &[
            "page",
            "append",
            "clients/acme",
            "called back",
            "--date",
            "2026-01-02",
        ],
        "",
    )
    .await?;
    assert_eq!(appended.code, Some(0), "{}", appended.stderr);

    let found = memory(
        address,
        &["--json", "search", "called", "--where", "status=active"],
        "",
    )
    .await?;
    assert_eq!(found.code, Some(0), "{}", found.stderr);
    assert!(found.stdout.contains("clients/acme"), "{}", found.stdout);

    let history = memory(address, &["--json", "history", "clients/acme"], "").await?;
    assert_eq!(history.code, Some(0), "{}", history.stderr);

    let moved = memory(
        address,
        &["page", "move", "clients/acme", "clients/acme-inc"],
        "",
    )
    .await?;
    assert_eq!(moved.code, Some(0), "{}", moved.stderr);

    let listed = memory(address, &["--json", "page", "list", "clients"], "").await?;
    assert!(
        listed.stdout.contains("acme-inc"),
        "{} {}",
        listed.stdout,
        listed.stderr
    );

    let deleted = memory(address, &["page", "delete", "clients/acme-inc"], "").await?;
    assert_eq!(deleted.code, Some(0), "{}", deleted.stderr);

    let missing = memory(address, &["page", "view", "clients/acme-inc"], "").await?;
    assert_eq!(missing.code, Some(1));

    let local = tempfile::tempdir()?;
    let source = local.path().join("note.txt");
    tokio::fs::write(&source, "plain text file").await?;
    let source = source.to_string_lossy().into_owned();
    let uploaded = memory(address, &["file", "upload", &source, "docs/note.txt"], "").await?;
    assert_eq!(uploaded.code, Some(0), "{}", uploaded.stderr);
    let downloaded = memory(
        address,
        &["file", "download", "docs/note.txt", "-o", "-"],
        "",
    )
    .await?;
    assert_eq!(downloaded.stdout, "plain text file");
    Ok(())
}

#[sqlx::test(migrations = "../memory-server/migrations")]
async fn a_read_only_token_cannot_write_and_a_bad_token_needs_auth(pool: PgPool) -> TestResult {
    personal_token(&pool, false).await?;
    let address = serve(pool).await?;
    let denied = memory(address, &["page", "create", "a", "-"], "text").await?;
    assert_eq!(denied.code, Some(1), "{}", denied.stderr);
    let config = tempfile::tempdir()?;
    let output = Command::new(env!("CARGO_BIN_EXE_memory"))
        .args(["page", "list"])
        .env("XDG_CONFIG_HOME", config.path())
        .env("MEMORY_HOST", format!("http://{address}"))
        .env("MEMORY_TOKEN", "mpat_wrong")
        .output()
        .await?;
    assert_eq!(
        output.status.code(),
        Some(4),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
