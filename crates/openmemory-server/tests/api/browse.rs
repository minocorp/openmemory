use axum::http::{Method, StatusCode};
use serde_json::Value;
use sqlx::PgPool;

use crate::pages::{PAGE, created_id};
use crate::support::{App, Browser, TestResult};

async fn upload_text(browser: &Browser, path: &str, text: &str) -> TestResult {
    let reply = browser
        .raw(
            Method::POST,
            &format!("/api/files?path={path}"),
            &[("content-type", "text/plain")],
            text.as_bytes().to_vec(),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::CREATED);
    Ok(())
}

fn paths(items: &Value) -> Vec<&str> {
    items.as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|item| item["path"].as_str())
            .collect()
    })
}

#[sqlx::test]
async fn the_tree_lists_pages_files_and_folder_counts(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    created_id(&admin, "start", "Rules.\n").await?;
    created_id(&admin, "clients/acme", PAGE).await?;
    created_id(&admin, "clients/deep/beta", "two\n").await?;
    created_id(&admin, "notes/seoul", "three\n").await?;
    upload_text(&admin, "clients/contract.txt", "terms").await?;
    let doomed = created_id(&admin, "clients/gone", "bye\n").await?;
    admin
        .exchange(
            Method::DELETE,
            &format!("/api/pages/{doomed}"),
            &[("if-match", "\"1\"")],
            None,
        )
        .await?;

    let tree = admin.get("/api/tree").await?;
    assert_eq!(tree.status, StatusCode::OK, "{:?}", tree.body);
    assert_eq!(
        paths(&tree.body["pages"]),
        ["clients/acme", "clients/deep/beta", "notes/seoul", "start"]
    );
    assert_eq!(paths(&tree.body["files"]), ["clients/contract.txt"]);
    let folders = tree.body["folders"].as_array().ok_or("no folders")?;
    let counts: Vec<(&str, u64, u64)> = folders
        .iter()
        .filter_map(|folder| {
            Some((
                folder["path"].as_str()?,
                folder["pages"].as_u64()?,
                folder["files"].as_u64()?,
            ))
        })
        .collect();
    assert_eq!(
        counts,
        [("clients", 2, 1), ("clients/deep", 1, 0), ("notes", 1, 0)]
    );
    assert!(tree.body["pages"][0]["id"].is_i64());
    assert!(tree.body["pages"][0]["updated_at"].is_string());
    assert_eq!(tree.body["files"][0]["mime"], "text/plain");
    assert_eq!(
        app.browser().get("/api/tree").await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
async fn search_groups_matches_by_folder_with_ids_lines_and_file_pages(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let acme = created_id(&admin, "clients/acme", PAGE).await?;
    created_id(
        &admin,
        "clients/beta",
        "---\nstatus: churned\nfee: 900000\n---\nGone partner.\n",
    )
    .await?;
    created_id(&admin, "notes/seoul", "partner meeting\n").await?;
    upload_text(&admin, "clients/terms.txt", "The partner pays monthly.\n").await?;

    let found = admin.get("/api/search?q=PARTNER").await?;
    assert_eq!(found.status, StatusCode::OK, "{:?}", found.body);
    assert!(found.body["hits"].as_array().is_some_and(Vec::is_empty));
    let folders = &found.body["folders"];
    assert_eq!(paths(folders), ["clients", "notes"]);
    assert_eq!(folders[0]["matches"], 3);
    assert_eq!(folders[1]["matches"], 1);
    let clients = &folders[0]["hits"];
    assert_eq!(
        paths(clients),
        ["clients/terms.txt", "clients/beta", "clients/acme"]
    );
    assert_eq!(clients[0]["kind"], "file");
    assert!(clients[0]["id"].is_i64());
    assert_eq!(clients[0]["page"], 1);
    assert_eq!(clients[0]["line"], "The partner pays monthly.");
    assert_eq!(clients[2]["kind"], "page");
    assert_eq!(clients[2]["id"], acme);
    assert_eq!(clients[2]["line_number"], 5);
    assert_eq!(clients[2]["line"], "Partner. 500,000 KRW.");

    let inside = admin.get("/api/search?q=partner&folder=clients").await?;
    assert_eq!(
        paths(&inside.body["hits"]),
        ["clients/terms.txt", "clients/beta", "clients/acme"]
    );
    assert!(inside.body["folders"].as_array().is_some_and(Vec::is_empty));

    let conditions = admin
        .get("/api/search?where=status%3Dactive&where=fee%3C600000")
        .await?;
    assert_eq!(paths(&conditions.body["hits"]), ["clients/acme"]);

    let single = admin.get("/api/search?where=status%3Dchurned").await?;
    assert_eq!(paths(&single.body["hits"]), ["clients/beta"]);
    Ok(())
}

#[sqlx::test]
async fn search_explains_empty_results_and_pages_through_hits(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    for index in 0..3 {
        created_id(&admin, &format!("notes/n{index}"), "alpha\n").await?;
    }
    let first = admin.get("/api/search?q=alpha&limit=2").await?;
    assert_eq!(first.body["hits"].as_array().map(Vec::len), Some(2));
    let cursor = first.body["next"].as_str().ok_or("no cursor")?;
    let second = admin
        .get(&format!("/api/search?q=alpha&limit=2&cursor={cursor}"))
        .await?;
    assert_eq!(second.body["hits"].as_array().map(Vec::len), Some(1));
    assert!(second.body["next"].is_null());

    let none = admin.get("/api/search?q=zzz").await?;
    assert!(none.body["hits"].as_array().is_some_and(Vec::is_empty));
    assert!(none.body["no_hits"]["query"].is_string());
    Ok(())
}

#[sqlx::test]
async fn search_rejects_bad_input_and_anonymous_callers(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let bad_where = admin.get("/api/search?where=%3D%3D").await?;
    assert_eq!(bad_where.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        bad_where.body["type"],
        "urn:openmemory:problem:invalid_where"
    );
    for query in ["folder=a//b", "limit=many", "q=a&cursor=12"] {
        assert_eq!(
            admin.get(&format!("/api/search?{query}")).await?.status,
            StatusCode::BAD_REQUEST,
            "{query}"
        );
    }
    assert_eq!(
        app.browser().get("/api/search?q=a").await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
async fn search_matches_paths_and_text_of_pages_and_files(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let seoul = created_id(&admin, "notes/Seoul-office", "lease terms\n").await?;
    upload_text(&admin, "contracts/seoul-lease.txt", "terms\n").await?;

    let found = admin.get("/api/search?q=seoul").await?;
    assert_eq!(found.status, StatusCode::OK, "{:?}", found.body);
    let folders = &found.body["folders"];
    assert_eq!(paths(folders), ["contracts", "notes"]);
    let file = &folders[0]["hits"][0];
    assert_eq!(file["path"], "contracts/seoul-lease.txt");
    assert!(file["page"].is_null());
    let page = &folders[1]["hits"][0];
    assert_eq!(page["id"], seoul);
    assert!(page["line"].is_null());

    let text = admin.get("/api/search?q=terms").await?;
    assert_eq!(text.body["folders"][0]["hits"][0]["page"], 1);
    assert_eq!(text.body["folders"][1]["hits"][0]["line"], "lease terms");
    Ok(())
}
