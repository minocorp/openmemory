use openmemory_ops::{
    HitKind, Replacement, SearchInput, SearchOutput, WriteAction, format_iso_date,
};
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use crate::access::Actor;
use crate::pages::delete::delete;
use crate::pages::write::write;
use crate::search::{Match, SearchError, search};
use crate::test_support::{agent, create, path, scoped};
use crate::time_zone::TimeZone;

fn days_from_today(days: i64) -> String {
    format_iso_date(OffsetDateTime::now_utc().date() + Duration::days(days))
}

async fn seed(pool: &PgPool) -> Actor {
    let actor = agent(pool, "mino", "claude-code").await;
    let acme = format!(
        "---\nstatus: active\nfee: 450000\nrenewal: {}\nowner: kim\n---\nPartner of ours. Design work.\n\n## Timeline\n2026-03-02 | call — signed, 월 금 오십만원\n",
        days_from_today(30)
    );
    let beta = format!(
        "---\nstatus: churned\nfee: 900000\nrenewal: {}\ntags: [a, b]\n---\nChurned partner.\n",
        days_from_today(-90)
    );
    create(pool, &actor, "clients/acme", &acme).await;
    create(pool, &actor, "clients/beta", &beta).await;
    create(pool, &actor, "clients2/other", "Partner elsewhere\n").await;
    create(pool, &actor, "tax/2026", "refund 100% done, file_id 7\n").await;
    create(pool, &actor, "notes/seoul", "서울 지사 계약 검토\n").await;
    actor
}

async fn find(pool: &PgPool, actor: &Actor, input: SearchInput) -> SearchOutput {
    search(pool, &TimeZone::utc(), actor, &input, Match::Text)
        .await
        .unwrap()
}

fn text(value: &str) -> SearchInput {
    SearchInput {
        text: Some(value.to_owned()),
        ..SearchInput::default()
    }
}

fn filter(conditions: &[&str]) -> SearchInput {
    SearchInput {
        conditions: conditions.iter().map(|c| (*c).to_owned()).collect(),
        ..SearchInput::default()
    }
}

fn paths(output: &SearchOutput) -> Vec<&str> {
    let mut paths: Vec<&str> = output.hits.iter().map(|hit| hit.path.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

#[sqlx::test]
async fn text_search_returns_matching_lines_with_author_and_time(pool: PgPool) {
    let actor = seed(&pool).await;
    let output = find(&pool, &actor, text("Partner")).await;
    assert_eq!(paths(&output), ["clients/acme", "clients2/other"]);
    let hit = output
        .hits
        .iter()
        .find(|hit| hit.path == "clients/acme")
        .unwrap();
    assert_eq!(hit.line_number, Some(7));
    assert_eq!(hit.line.as_deref(), Some("Partner of ours. Design work."));
    assert_eq!(hit.author, "mino/claude-code");
    assert!(hit.at.contains('T'));
    assert!(output.no_hits.is_none());
}

#[sqlx::test]
async fn search_is_literal_and_case_sensitive(pool: PgPool) {
    let actor = seed(&pool).await;
    assert_eq!(
        paths(&find(&pool, &actor, text("100%")).await),
        ["tax/2026"]
    );
    assert_eq!(
        paths(&find(&pool, &actor, text("file_id")).await),
        ["tax/2026"]
    );
    assert_eq!(find(&pool, &actor, text("1_0")).await.hits.len(), 0);
    assert!(
        find(&pool, &actor, text("%"))
            .await
            .hits
            .iter()
            .all(|hit| hit.path == "tax/2026")
    );
    assert_eq!(find(&pool, &actor, text("PARTNER")).await.hits.len(), 0);
    assert_eq!(find(&pool, &actor, text("\\")).await.hits.len(), 0);
    assert_eq!(find(&pool, &actor, text("100%d")).await.hits.len(), 0);
}

#[sqlx::test]
async fn two_character_korean_queries_match(pool: PgPool) {
    let actor = seed(&pool).await;
    assert_eq!(
        paths(&find(&pool, &actor, text("계약")).await),
        ["notes/seoul"]
    );
    assert_eq!(
        paths(&find(&pool, &actor, text("지사")).await),
        ["notes/seoul"]
    );
    assert_eq!(
        paths(&find(&pool, &actor, text("오십")).await),
        ["clients/acme"]
    );
    assert_eq!(
        paths(&find(&pool, &actor, text("금")).await),
        ["clients/acme"]
    );
    assert_eq!(find(&pool, &actor, text("계지")).await.hits.len(), 0);
}

#[sqlx::test]
async fn field_filters_compare_by_type(pool: PgPool) {
    let actor = seed(&pool).await;
    let cases: [(&[&str], &[&str]); 11] = [
        (&["status=active"], &["clients/acme"]),
        (
            &["status!=active"],
            &["clients/beta", "clients2/other", "notes/seoul", "tax/2026"],
        ),
        (&["fee<1000000"], &["clients/acme", "clients/beta"]),
        (&["fee>450000"], &["clients/beta"]),
        (&["fee=450000", "status=active"], &["clients/acme"]),
        (&["renewal<+60d", "renewal>today"], &["clients/acme"]),
        (&["renewal<-30d"], &["clients/beta"]),
        (&["tags=b"], &["clients/beta"]),
        (&["owner~im"], &["clients/acme"]),
        (&["owner:exists"], &["clients/acme"]),
        (&["status=active", "status=churned"], &[]),
    ];
    for (conditions, expected) in cases {
        let output = find(&pool, &actor, filter(conditions)).await;
        assert_eq!(paths(&output), expected, "{conditions:?}");
    }
}

#[sqlx::test]
async fn filter_hits_show_the_page_fields(pool: PgPool) {
    let actor = seed(&pool).await;
    let output = find(&pool, &actor, filter(&["status=churned"])).await;
    let hit = output.hits.first().unwrap();
    assert_eq!(hit.path, "clients/beta");
    assert_eq!(hit.line_number, None);
    assert_eq!(
        hit.line.as_deref(),
        Some(
            format!(
                "status: churned; fee: 900000; renewal: {}; tags: a, b",
                days_from_today(-90)
            )
            .as_str()
        )
    );
}

#[sqlx::test]
async fn text_and_filters_and_folders_combine(pool: PgPool) {
    let actor = seed(&pool).await;
    let both = SearchInput {
        text: Some("Partner".to_owned()),
        conditions: vec!["status=churned".to_owned()],
        folder: Some("clients".to_owned()),
        ..SearchInput::default()
    };
    assert_eq!(find(&pool, &actor, both).await.hits.len(), 0);
    let matching = SearchInput {
        text: Some("Churned".to_owned()),
        conditions: vec!["status=churned".to_owned()],
        folder: Some("clients".to_owned()),
        ..SearchInput::default()
    };
    assert_eq!(
        paths(&find(&pool, &actor, matching).await),
        ["clients/beta"]
    );
}

#[sqlx::test]
async fn folders_match_on_segment_boundaries(pool: PgPool) {
    let actor = seed(&pool).await;
    let input = SearchInput {
        text: Some("artner".to_owned()),
        folder: Some("clients".to_owned()),
        ..SearchInput::default()
    };
    assert_eq!(
        paths(&find(&pool, &actor, input).await),
        ["clients/acme", "clients/beta"]
    );
}

#[sqlx::test]
async fn results_stay_inside_the_scope_and_zero_hits_explain_the_hidden_folders(pool: PgPool) {
    let admin = seed(&pool).await;
    let tax_only = scoped(&admin, &["tax"], true);
    assert_eq!(find(&pool, &tax_only, text("Partner")).await.hits.len(), 0);
    assert_eq!(
        find(&pool, &tax_only, filter(&["status=active"]))
            .await
            .hits
            .len(),
        0
    );
    let output = find(&pool, &tax_only, text("Partner")).await;
    let explanation = output.no_hits.unwrap();
    assert_eq!(explanation.folders_out_of_scope, 3);
    assert_eq!(explanation.fields_in_scope.len(), 0);
    assert!(explanation.query.contains("\"Partner\""));
    let visible = find(&pool, &tax_only, text("refund")).await;
    assert_eq!(paths(&visible), ["tax/2026"]);
}

#[sqlx::test]
async fn zero_hit_explanations_list_fields_and_the_reading_of_the_query(pool: PgPool) {
    let actor = seed(&pool).await;
    let input = SearchInput {
        text: Some("missing".to_owned()),
        conditions: vec!["renewal<+90d".to_owned(), "status!=active".to_owned()],
        folder: Some("clients".to_owned()),
        ..SearchInput::default()
    };
    let explanation = find(&pool, &actor, input).await.no_hits.unwrap();
    assert_eq!(
        explanation.fields_in_scope,
        ["fee", "owner", "renewal", "status", "tags"]
    );
    assert_eq!(explanation.folders_out_of_scope, 0);
    assert!(explanation.query.contains("renewal < date"));
    assert!(explanation.query.contains("status != text \"active\""));
    assert!(explanation.query.contains("folder clients"));
}

#[sqlx::test]
async fn only_current_text_of_live_pages_is_searched(pool: PgPool) {
    let actor = seed(&pool).await;
    let action = WriteAction::Edit {
        replace: Some(Replacement {
            old: "계약".to_owned(),
            new: "합의".to_owned(),
        }),
        append: None,
    };
    write(
        &pool,
        &TimeZone::utc(),
        &actor,
        &path("notes/seoul"),
        action,
        None,
    )
    .await
    .unwrap();
    assert_eq!(find(&pool, &actor, text("계약")).await.hits.len(), 0);
    assert_eq!(
        paths(&find(&pool, &actor, text("합의")).await),
        ["notes/seoul"]
    );
    delete(&pool, &actor, &path("notes/seoul"), None)
        .await
        .unwrap();
    assert_eq!(find(&pool, &actor, text("합의")).await.hits.len(), 0);
}

#[sqlx::test]
async fn limits_and_cursors_page_through_results_newest_first(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    for index in 0..5 {
        create(&pool, &actor, &format!("items/{index}"), "shared word\n").await;
    }
    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let input = SearchInput {
            text: Some("shared".to_owned()),
            limit: Some(2),
            cursor,
            ..SearchInput::default()
        };
        let output = find(&pool, &actor, input).await;
        assert!(output.hits.len() <= 2);
        seen.extend(output.hits.iter().map(|hit| hit.path.clone()));
        match output.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(
        seen,
        ["items/4", "items/3", "items/2", "items/1", "items/0"]
    );
}

#[sqlx::test]
async fn many_matching_lines_in_one_page_are_capped(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "log", &"needle\n".repeat(30)).await;
    let output = find(&pool, &actor, text("needle")).await;
    assert_eq!(output.hits.len(), 5);
}

#[sqlx::test]
async fn filters_without_text_return_page_level_hits(pool: PgPool) {
    let actor = seed(&pool).await;
    let output = find(&pool, &actor, filter(&["status=active"])).await;
    assert_eq!(output.hits.len(), 1);
    assert_eq!(output.hits[0].line_number, None);
    assert_eq!(output.hits[0].path, "clients/acme");
}

#[sqlx::test]
async fn invalid_conditions_and_folders_are_rejected(pool: PgPool) {
    let actor = seed(&pool).await;
    let bad_where = search(
        &pool,
        &TimeZone::utc(),
        &actor,
        &filter(&["status"]),
        Match::Text,
    )
    .await;
    let bad_folder = search(
        &pool,
        &TimeZone::utc(),
        &actor,
        &SearchInput {
            folder: Some("a//b".to_owned()),
            ..SearchInput::default()
        },
        Match::Text,
    )
    .await;
    assert!(matches!(bad_where, Err(SearchError::Where(_))));
    assert!(matches!(bad_folder, Err(SearchError::Folder(_))));
}

#[sqlx::test]
async fn today_in_where_follows_the_configured_zone(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let ahead: TimeZone = "Pacific/Kiritimati".parse().unwrap();
    let behind: TimeZone = "Pacific/Pago_Pago".parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let ahead_today = ahead.today(&mut conn).await.unwrap();
    drop(conn);
    let page = format!(
        "---\nrenewal: {}\n---\nrenews\n",
        format_iso_date(ahead_today)
    );
    create(&pool, &actor, "renews", &page).await;
    let input = filter(&["renewal=today"]);
    let in_ahead = search(&pool, &ahead, &actor, &input, Match::Text)
        .await
        .unwrap();
    let in_behind = search(&pool, &behind, &actor, &input, Match::Text)
        .await
        .unwrap();
    assert_eq!(paths(&in_ahead), ["renews"]);
    assert_eq!(paths(&in_behind), Vec::<&str>::new());
}

async fn add_file(pool: &PgPool, actor: &Actor, path: &str, pages: &[&str]) {
    let file_id = sqlx::query_scalar!(
        r#"INSERT INTO files (path, sha256, blob_key, mime, size, user_id)
VALUES ($1, decode(repeat('ab', 32), 'hex'), $1, 'application/pdf', 1, $2)
RETURNING id"#,
        path,
        actor.user_id.0
    )
    .fetch_one(pool)
    .await
    .unwrap();
    for (index, text) in pages.iter().enumerate() {
        sqlx::query!(
            "INSERT INTO file_pages (file_id, page, text) VALUES ($1, $2, $3)",
            file_id,
            i32::try_from(index + 1).unwrap(),
            text
        )
        .execute(pool)
        .await
        .unwrap();
    }
}

#[sqlx::test]
async fn text_search_covers_file_pages_and_names_the_pdf_page(pool: PgPool) {
    let actor = seed(&pool).await;
    add_file(
        &pool,
        &actor,
        "contracts/acme.pdf",
        &["cover page", "fee\n월 금 오십만원정"],
    )
    .await;
    let output = find(&pool, &actor, text("오십만원")).await;
    let file_hits: Vec<_> = output
        .hits
        .iter()
        .filter(|hit| hit.kind == HitKind::File)
        .collect();
    assert_eq!(file_hits.len(), 1);
    assert_eq!(file_hits[0].path, "contracts/acme.pdf");
    assert_eq!(file_hits[0].page, Some(2));
    assert_eq!(file_hits[0].line_number, Some(2));
    assert_eq!(file_hits[0].line.as_deref(), Some("월 금 오십만원정"));
    assert_eq!(file_hits[0].author, "mino");
    assert!(
        output
            .hits
            .iter()
            .any(|hit| hit.kind == HitKind::Page && hit.path == "clients/acme")
    );
}

#[sqlx::test]
async fn file_hits_respect_scope_folder_and_field_conditions(pool: PgPool) {
    let actor = seed(&pool).await;
    add_file(&pool, &actor, "contracts/acme.pdf", &["Partner terms"]).await;
    let tax_only = scoped(&actor, &["tax"], true);
    assert_eq!(find(&pool, &tax_only, text("Partner")).await.hits.len(), 0);
    let in_folder = SearchInput {
        folder: Some("clients".to_owned()),
        ..text("Partner")
    };
    assert!(
        find(&pool, &actor, in_folder)
            .await
            .hits
            .iter()
            .all(|hit| hit.kind == HitKind::Page)
    );
    let with_condition = SearchInput {
        conditions: vec!["status=active".to_owned()],
        ..text("Partner")
    };
    assert!(
        find(&pool, &actor, with_condition)
            .await
            .hits
            .iter()
            .all(|hit| hit.kind == HitKind::Page)
    );
}

#[sqlx::test]
async fn paths_match_only_when_asked_and_ignore_case(pool: PgPool) {
    let actor = seed(&pool).await;
    add_file(&pool, &actor, "contracts/Seoul-lease.pdf", &["unrelated"]).await;
    let by_text = find(&pool, &actor, text("seoul")).await;
    assert_eq!(by_text.hits, []);
    let output = search(
        &pool,
        &TimeZone::utc(),
        &actor,
        &text("seoul"),
        Match::TextAndPaths,
    )
    .await
    .unwrap();
    let found: Vec<(HitKind, &str)> = output
        .hits
        .iter()
        .map(|hit| (hit.kind, hit.path.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            (HitKind::Page, "notes/seoul"),
            (HitKind::File, "contracts/Seoul-lease.pdf")
        ]
    );
    assert_eq!(output.hits[0].line, None);
}
