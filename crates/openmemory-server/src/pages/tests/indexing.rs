use sqlx::PgPool;

use super::overwrite;
use crate::test_support::{agent, create};

#[derive(Debug, PartialEq)]
struct FieldRow {
    key: String,
    ord: i32,
    text: String,
    num: Option<String>,
    date: Option<String>,
}

async fn fields(pool: &PgPool) -> Vec<FieldRow> {
    sqlx::query!(
        r#"SELECT key, ord, text, num::text AS "num?", date::text AS "date?"
FROM page_fields
ORDER BY key, ord"#
    )
    .fetch_all(pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| FieldRow {
        key: row.key,
        ord: row.ord,
        text: row.text,
        num: row.num,
        date: row.date,
    })
    .collect()
}

async fn link_targets(pool: &PgPool, from: &str) -> Vec<String> {
    sqlx::query_scalar!(
        r#"SELECT COALESCE(tp.path, tf.path) AS "path!"
FROM page_links AS l
JOIN pages AS fp ON fp.id = l.from_page_id
LEFT JOIN pages AS tp ON tp.id = l.to_page_id
LEFT JOIN files AS tf ON tf.id = l.to_file_id
WHERE fp.path = $1
ORDER BY 1"#,
        from
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn fields_are_indexed_by_type_and_rebuilt_on_every_write(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let page =
        "---\nstatus: active\nfee: 450000\nrenewal: 2027-03-01\ntags: [design, 외주]\n---\nbody\n";
    create(&pool, &actor, "a", page).await;
    let text = |key: &str, ord: i32, text: &str| FieldRow {
        key: key.to_owned(),
        ord,
        text: text.to_owned(),
        num: None,
        date: None,
    };
    let indexed = fields(&pool).await;
    assert_eq!(indexed.len(), 5);
    assert_eq!(
        indexed[0],
        FieldRow {
            num: Some("450000".to_owned()),
            ..text("fee", 0, "450000")
        }
    );
    assert_eq!(
        indexed[1],
        FieldRow {
            date: Some("2027-03-01".to_owned()),
            ..text("renewal", 0, "2027-03-01")
        }
    );
    assert_eq!(indexed[2], text("status", 0, "active"));
    assert_eq!(indexed[3], text("tags", 0, "design"));
    assert_eq!(indexed[4], text("tags", 1, "외주"));
    overwrite(&pool, &actor, "a", "---\nstatus: done\n---\nbody\n", 1)
        .await
        .unwrap();
    assert_eq!(fields(&pool).await, [text("status", 0, "done")]);
}

#[sqlx::test]
async fn links_resolve_to_page_and_file_ids_and_skip_unresolved_targets(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    sqlx::query!(
        "INSERT INTO files (path, sha256, blob_key, mime, size, user_id) VALUES ('contracts/acme.pdf', decode(repeat('00', 32), 'hex'), 'k', 'application/pdf', 1, $1)",
        actor.user_id as crate::access::UserId
    )
    .execute(&pool)
    .await
    .unwrap();
    create(&pool, &actor, "clients/acme", "acme\n").await;
    create(
        &pool,
        &actor,
        "notes",
        "see [[clients/acme]], [[contracts/acme.pdf]], [[missing/page]] and [[notes]]\n",
    )
    .await;
    assert_eq!(
        link_targets(&pool, "notes").await,
        ["clients/acme", "contracts/acme.pdf"]
    );
    overwrite(&pool, &actor, "notes", "only [[clients/acme]]\n", 1)
        .await
        .unwrap();
    assert_eq!(link_targets(&pool, "notes").await, ["clients/acme"]);
}
