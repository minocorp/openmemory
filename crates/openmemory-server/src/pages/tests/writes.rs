use openmemory_ops::{Append, CredentialPattern, WriteAction};
use sqlx::PgPool;

use super::{PAGE, edit, overwrite};
use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::read::read;
use crate::pages::write::write;
use crate::test_support::{agent, create, path};
use crate::time_zone::TimeZone;

#[sqlx::test]
async fn created_pages_read_back_with_fields_body_and_timeline(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let written = create(&pool, &actor, "clients/acme", PAGE).await;
    assert_eq!(written.version, 1);
    let page = read(&pool, &actor, &path("clients/acme"), None)
        .await
        .unwrap();
    assert_eq!(page.version, 1);
    assert_eq!(page.author, "mino/claude-code");
    assert_eq!(page.content, PAGE);
    assert_eq!(page.fields[0].key, "status");
    assert_eq!(page.fields[0].values, ["active"]);
    assert_eq!(page.body, "Partner. 500,000 KRW.\n\n");
    assert_eq!(page.timeline[0].date, "2026-03-02");
    assert_eq!(page.timeline[0].text, "signed");
}

#[sqlx::test]
async fn creating_an_existing_path_is_rejected(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one").await;
    let action = WriteAction::Create {
        content: "two".to_owned(),
    };
    let result = write(&pool, &TimeZone::utc(), &actor, &path("a"), action, None).await;
    assert!(matches!(result, Err(PagesError::PathTaken)));
}

#[sqlx::test]
async fn creating_over_a_live_file_path_is_rejected(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    sqlx::query!(
        "INSERT INTO files (path, sha256, blob_key, mime, size, user_id) VALUES ('docs/a.pdf', decode(repeat('00', 32), 'hex'), 'k', 'application/pdf', 1, $1)",
        actor.user_id as crate::access::UserId
    )
    .execute(&pool)
    .await
    .unwrap();
    let action = WriteAction::Create {
        content: "x".to_owned(),
    };
    let result = write(
        &pool,
        &TimeZone::utc(),
        &actor,
        &path("docs/a.pdf"),
        action,
        None,
    )
    .await;
    assert!(matches!(result, Err(PagesError::PathTaken)));
}

#[sqlx::test]
async fn grammar_breaks_are_rejected_with_the_line_number(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let action = WriteAction::Create {
        content: "body\n\n## Timeline\n2026-03-02 | call — ok\nnot a line\n".to_owned(),
    };
    let result = write(&pool, &TimeZone::utc(), &actor, &path("a"), action, None).await;
    match result {
        Err(PagesError::Grammar(error)) => assert_eq!(error.line(), 5),
        other => panic!("unexpected {other:?}"),
    }
}

#[sqlx::test]
async fn credentials_are_rejected_by_pattern_name_on_every_write_path(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let secret = "card 4111 1111 1111 1111";
    let create_result = write(
        &pool,
        &TimeZone::utc(),
        &actor,
        &path("b"),
        WriteAction::Create {
            content: secret.to_owned(),
        },
        None,
    )
    .await;
    let replace_result = edit(&pool, &actor, "a", Some(("500,000 KRW", secret)), None).await;
    let append_result = edit(&pool, &actor, "a", None, Some(secret)).await;
    let overwrite_result = overwrite(&pool, &actor, "a", secret, 1).await;
    for result in [
        create_result,
        replace_result,
        append_result,
        overwrite_result,
    ] {
        match result {
            Err(PagesError::Credential(pattern)) => {
                assert_eq!(pattern, CredentialPattern::CardNumber);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}

#[sqlx::test]
async fn pages_over_the_size_cap_are_rejected(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let action = WriteAction::Create {
        content: "a".repeat(1_048_577),
    };
    let result = write(&pool, &TimeZone::utc(), &actor, &path("big"), action, None).await;
    assert!(matches!(result, Err(PagesError::TooLarge)));
}

#[sqlx::test]
async fn replace_needs_exactly_one_match(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(
        &pool,
        &actor,
        "a",
        &PAGE.replace("Partner.", "Partner. repeat repeat"),
    )
    .await;
    let done = edit(
        &pool,
        &actor,
        "a",
        Some(("500,000 KRW", "450,000 KRW")),
        None,
    )
    .await
    .unwrap();
    assert_eq!(done.version, 2);
    let missing = edit(&pool, &actor, "a", Some(("absent", "x")), None).await;
    let ambiguous = edit(&pool, &actor, "a", Some(("repeat", "x")), None).await;
    assert!(matches!(missing, Err(PagesError::Replace(_))));
    assert!(matches!(ambiguous, Err(PagesError::Replace(_))));
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert!(page.body.contains("450,000 KRW"));
    assert_eq!(page.version, 2);
}

#[sqlx::test]
async fn append_stamps_today_and_defaults_the_source_to_the_author(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    edit(&pool, &actor, "a", None, Some("fee lowered"))
        .await
        .unwrap();
    let action = WriteAction::Edit {
        replace: None,
        append: Some(Append {
            text: "renewed".to_owned(),
            source: Some("call".to_owned()),
            date: None,
        }),
    };
    write(&pool, &TimeZone::utc(), &actor, &path("a"), action, None)
        .await
        .unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    let today = time::OffsetDateTime::now_utc().date();
    let stamped = openmemory_ops::format_iso_date(today);
    let yesterday = openmemory_ops::format_iso_date(today.previous_day().unwrap());
    let appended = &page.timeline[1];
    assert!(appended.date == stamped || appended.date == yesterday);
    assert_eq!(appended.source, "mino/claude-code");
    assert_eq!(page.timeline[2].source, "call");
    assert_eq!(page.timeline.len(), 3);
    assert!(page.content.starts_with(PAGE));
}

#[sqlx::test]
async fn append_to_a_page_without_a_timeline_creates_the_section(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "just a body\n").await;
    edit(&pool, &actor, "a", None, Some("first note"))
        .await
        .unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert!(page.content.starts_with("just a body\n\n## Timeline\n"));
    assert_eq!(page.timeline.len(), 1);
}

#[sqlx::test]
async fn replace_and_append_land_in_one_version(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let written = edit(
        &pool,
        &actor,
        "a",
        Some(("500,000 KRW", "450,000 KRW")),
        Some("fee lowered"),
    )
    .await
    .unwrap();
    assert_eq!(written.version, 2);
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert!(page.body.contains("450,000"));
    assert_eq!(page.timeline.len(), 2);
}

#[sqlx::test]
async fn edits_that_change_nothing_are_rejected(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let same = edit(&pool, &actor, "a", Some(("signed", "signed")), None).await;
    assert!(matches!(same, Err(PagesError::NoChange)));
}

#[sqlx::test]
async fn editing_a_missing_page_is_not_found(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let result = edit(&pool, &actor, "nowhere", None, Some("x")).await;
    assert!(matches!(result, Err(PagesError::NotFound)));
}

#[sqlx::test]
async fn overwrite_needs_the_current_base_version(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "plain\n").await;
    edit(&pool, &actor, "a", Some(("plain", "plain two")), None)
        .await
        .unwrap();
    let stale = overwrite(&pool, &actor, "a", "replacement", 1).await;
    match stale {
        Err(PagesError::Conflict { current_version }) => assert_eq!(current_version.0, 2),
        other => panic!("unexpected {other:?}"),
    }
    let content = "replacement\n";
    let fresh = overwrite(&pool, &actor, "a", content, 2).await.unwrap();
    assert_eq!(fresh.version, 3);
}

#[sqlx::test]
async fn existing_timeline_lines_can_only_be_appended_to(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let rewrite = PAGE.replace("signed", "cancelled");
    let removal = PAGE.replace("2026-03-02 | call — signed\n", "");
    let appended = format!("{PAGE}2026-04-01 | call — renewed\n");
    for text in [&rewrite, &removal] {
        let result = overwrite(&pool, &actor, "a", text, 1).await;
        assert!(matches!(result, Err(PagesError::TimelineChanged)));
    }
    let replaced = edit(&pool, &actor, "a", Some(("signed", "cancelled")), None).await;
    assert!(matches!(replaced, Err(PagesError::TimelineChanged)));
    assert!(overwrite(&pool, &actor, "a", &appended, 1).await.is_ok());
}

#[sqlx::test]
async fn two_overwrites_from_one_base_let_exactly_one_win(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = crate::test_support::another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "plain\n").await;
    let first = overwrite(&pool, &actor, "a", "first writer\n", 1);
    let second = overwrite(&pool, &other, "a", "second writer\n", 1);
    let (first, second) = tokio::join!(first, second);
    let outcomes = [first, second];
    let winners = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    let conflicts = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Err(PagesError::Conflict { .. })))
        .count();
    assert_eq!((winners, conflicts), (1, 1));
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.version, 2);
}

#[sqlx::test]
async fn concurrent_appends_both_land(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = crate::test_support::another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", PAGE).await;
    let first = edit(&pool, &actor, "a", None, Some("from claude"));
    let second = edit(&pool, &other, "a", None, Some("from codex"));
    let (first, second) = tokio::join!(first, second);
    first.unwrap();
    second.unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.version, 3);
    let texts: Vec<&str> = page
        .timeline
        .iter()
        .map(|line| line.text.as_str())
        .collect();
    assert!(texts.contains(&"from claude") && texts.contains(&"from codex"));
}

#[sqlx::test]
async fn concurrent_creates_of_one_path_let_one_win(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = crate::test_support::another_connection(&pool, &actor, "codex").await;
    let create_for = |who: &crate::access::Actor, text: &str| {
        let action = WriteAction::Create {
            content: text.to_owned(),
        };
        let who = who.clone();
        let pool = pool.clone();
        async move { write(&pool, &TimeZone::utc(), &who, &path("race"), action, None).await }
    };
    let (first, second) = tokio::join!(create_for(&actor, "one"), create_for(&other, "two"));
    let winners = [&first, &second]
        .iter()
        .filter(|result| result.is_ok())
        .count();
    assert_eq!(winners, 1);
    assert!(
        [first, second]
            .iter()
            .any(|result| matches!(result, Err(PagesError::PathTaken)))
    );
}

#[sqlx::test]
async fn nul_bytes_are_stripped_before_storing(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "he\0llo").await;
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.content, "hello");
}

#[sqlx::test]
async fn deleted_pages_disappear_and_their_path_can_be_reused(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let deleted = crate::pages::delete::delete(&pool, &actor, &path("a"), None)
        .await
        .unwrap();
    assert_eq!(deleted.version, 2);
    let gone = read(&pool, &actor, &path("a"), None).await;
    assert!(matches!(gone, Err(PagesError::NotFound)));
    let fields = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM page_fields"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(fields, 0);
    let again = create(&pool, &actor, "a", "fresh").await;
    assert_eq!(again.version, 1);
    let twice = crate::pages::delete::delete(&pool, &actor, &path("missing"), None).await;
    assert!(matches!(twice, Err(PagesError::NotFound)));
}

#[sqlx::test]
async fn append_stamps_the_date_of_the_configured_zone(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let zone: TimeZone = "Pacific/Kiritimati".parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let expected = openmemory_ops::format_iso_date(zone.today(&mut conn).await.unwrap());
    drop(conn);
    let action = WriteAction::Edit {
        replace: None,
        append: Some(Append {
            text: "renewed".to_owned(),
            source: None,
            date: None,
        }),
    };
    write(&pool, &zone, &actor, &path("a"), action, None)
        .await
        .unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.timeline[1].date, expected);
}

async fn append_dated(
    pool: &PgPool,
    actor: &Actor,
    zone: &TimeZone,
    page_path: &str,
    date: &str,
) -> Result<openmemory_ops::WriteOutput, PagesError> {
    let action = WriteAction::Edit {
        replace: None,
        append: Some(Append {
            text: "contract signed".to_owned(),
            source: Some("call".to_owned()),
            date: Some(openmemory_ops::parse_iso_date(date).unwrap()),
        }),
    };
    write(pool, zone, actor, &path(page_path), action, None).await
}

#[sqlx::test]
async fn appended_lines_keep_the_given_past_date_and_the_file_stays_in_append_order(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    append_dated(&pool, &actor, &TimeZone::utc(), "a", "2026-01-15")
        .await
        .unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert!(
        page.content
            .ends_with("2026-03-02 | call — signed\n2026-01-15 | call — contract signed\n")
    );
    let dates: Vec<&str> = page
        .timeline
        .iter()
        .map(|line| line.date.as_str())
        .collect();
    assert_eq!(dates, ["2026-01-15", "2026-03-02"]);
}

#[sqlx::test]
async fn dates_after_today_in_the_configured_zone_are_rejected(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let zone: TimeZone = "Pacific/Kiritimati".parse().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    let today = zone.today(&mut conn).await.unwrap();
    drop(conn);
    let ahead = openmemory_ops::format_iso_date(today.next_day().unwrap());
    let result = append_dated(&pool, &actor, &zone, "a", &ahead).await;
    assert!(matches!(result, Err(PagesError::FutureDate { today: seen }) if seen == today));
    let on_today = append_dated(
        &pool,
        &actor,
        &zone,
        "a",
        &openmemory_ops::format_iso_date(today),
    )
    .await;
    assert!(on_today.is_ok());
}
