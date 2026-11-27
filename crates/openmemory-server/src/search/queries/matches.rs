use openmemory_ops::{Comparison, ResolvedCondition, ResolvedTest, Value};
use sqlx::postgres::Postgres;
use sqlx::{PgConnection, QueryBuilder};
use time::OffsetDateTime;

use crate::files::FileId;
use crate::pages::ids::PageId;
use crate::pages::scope_filter::ScopeFilter;
use crate::search::candidate::Candidate;
use crate::search::position::Document;

pub(crate) struct MatchFilter<'a> {
    pub(crate) scope: &'a ScopeFilter,
    pub(crate) folder: Option<&'a str>,
    pub(crate) like_patterns: &'a [String],
    pub(crate) conditions: &'a [ResolvedCondition],
}

#[derive(Debug, sqlx::FromRow)]
struct MatchRow {
    id: i64,
    path: String,
    author: String,
    at: OffsetDateTime,
}

pub(crate) async fn pages(
    conn: &mut PgConnection,
    filter: &MatchFilter<'_>,
) -> Result<Vec<Candidate>, sqlx::Error> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT p.id, p.path, v.author_name AS author, v.created_at AS at \
         FROM pages AS p \
         JOIN page_versions AS v ON v.page_id = p.id AND v.n = p.current_version \
         WHERE p.deleted_at IS NULL",
    );
    push_place(&mut builder, "p.path", filter);
    for pattern in filter.like_patterns {
        builder
            .push(" AND search_text(p.path || E'\\n' || p.content) LIKE search_text(")
            .push_bind(pattern.clone())
            .push(")");
    }
    for condition in filter.conditions {
        push_condition(&mut builder, condition);
    }
    let rows = builder.build_query_as::<MatchRow>().fetch_all(conn).await?;
    Ok(candidates(rows, |id| Document::Page(PageId(id))))
}

pub(crate) async fn files(
    conn: &mut PgConnection,
    filter: &MatchFilter<'_>,
) -> Result<Vec<Candidate>, sqlx::Error> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT f.id, f.path, u.username AS author, f.created_at AS at \
         FROM files AS f \
         JOIN users AS u ON u.id = f.user_id \
         WHERE f.deleted_at IS NULL",
    );
    push_place(&mut builder, "f.path", filter);
    for pattern in filter.like_patterns {
        builder
            .push(
                " AND f.id IN (SELECT fp.file_id FROM file_pages AS fp \
                 WHERE search_text(fp.text) LIKE search_text(",
            )
            .push_bind(pattern.clone())
            .push(
                ") UNION SELECT n.id FROM files AS n \
                 WHERE n.deleted_at IS NULL AND search_text(n.path) LIKE search_text(",
            )
            .push_bind(pattern.clone())
            .push("))");
    }
    let rows = builder.build_query_as::<MatchRow>().fetch_all(conn).await?;
    Ok(candidates(rows, |id| Document::File(FileId(id))))
}

fn candidates(rows: Vec<MatchRow>, document: impl Fn(i64) -> Document) -> Vec<Candidate> {
    rows.into_iter()
        .map(|row| Candidate {
            document: document(row.id),
            path: row.path,
            author: row.author,
            at: row.at,
        })
        .collect()
}

fn push_place(
    builder: &mut QueryBuilder<Postgres>,
    column: &'static str,
    filter: &MatchFilter<'_>,
) {
    if !filter.scope.unrestricted {
        builder
            .push(" AND (")
            .push(column)
            .push(" = ANY(")
            .push_bind(filter.scope.exact.clone())
            .push(") OR ")
            .push(column)
            .push(" LIKE ANY(")
            .push_bind(filter.scope.patterns.clone())
            .push("))");
    }
    if let Some(folder) = filter.folder {
        builder
            .push(" AND (")
            .push(column)
            .push(" = ")
            .push_bind(folder.to_owned())
            .push(" OR ")
            .push(column)
            .push(" LIKE ")
            .push_bind(format!("{}/%", openmemory_ops::escape_like(folder)))
            .push(" ESCAPE '\\')");
    }
}

fn push_condition(builder: &mut QueryBuilder<Postgres>, condition: &ResolvedCondition) {
    let ResolvedTest::Compare(comparison, value) = condition.test() else {
        builder
            .push(" AND EXISTS (SELECT 1 FROM page_fields AS f WHERE f.page_id = p.id AND f.key = ")
            .push_bind(condition.key().to_owned())
            .push(")");
        return;
    };
    builder.push(if *comparison == Comparison::NotEqual {
        " AND NOT EXISTS"
    } else {
        " AND EXISTS"
    });
    builder
        .push(" (SELECT 1 FROM page_fields AS f WHERE f.page_id = p.id AND f.key = ")
        .push_bind(condition.key().to_owned())
        .push(" AND ");
    push_comparison(builder, *comparison, value);
    builder.push(")");
}

fn push_comparison(builder: &mut QueryBuilder<Postgres>, comparison: Comparison, value: &Value) {
    let operator = match comparison {
        Comparison::Equal | Comparison::NotEqual => " = ",
        Comparison::Less => " < ",
        Comparison::Greater => " > ",
        Comparison::Contains => {
            let needle = match value {
                Value::Text(text) | Value::Number(text) => text.clone(),
                Value::Date(date) => openmemory_ops::format_iso_date(*date),
            };
            builder
                .push("strpos(f.text, ")
                .push_bind(needle)
                .push(") > 0");
            return;
        }
    };
    match value {
        Value::Date(date) => {
            builder.push("f.date").push(operator).push_bind(*date);
        }
        Value::Number(number) => {
            builder
                .push("f.num")
                .push(operator)
                .push_bind(number.clone())
                .push("::numeric");
        }
        Value::Text(text) => {
            builder
                .push("f.text")
                .push(operator)
                .push_bind(text.clone());
        }
    }
}
