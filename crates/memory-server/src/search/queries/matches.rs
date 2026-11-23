use memory_ops::{Comparison, ResolvedCondition, ResolvedTest, Value};
use sqlx::postgres::Postgres;
use sqlx::{PgConnection, QueryBuilder};
use time::OffsetDateTime;

use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct MatchRow {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) author_name: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) seq: i64,
}

pub(crate) struct MatchFilter<'a> {
    pub(crate) scope: &'a ScopeFilter,
    pub(crate) folder: Option<&'a str>,
    pub(crate) like_pattern: Option<&'a str>,
    pub(crate) match_paths: bool,
    pub(crate) conditions: &'a [ResolvedCondition],
    pub(crate) before_seq: Option<i64>,
    pub(crate) limit: i64,
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    filter: &MatchFilter<'_>,
) -> Result<Vec<MatchRow>, sqlx::Error> {
    let mut builder = QueryBuilder::<Postgres>::new(
        "SELECT p.path, p.content, v.author_name, v.created_at, v.seq \
         FROM pages AS p \
         JOIN page_versions AS v ON v.page_id = p.id AND v.n = p.current_version \
         WHERE p.deleted_at IS NULL",
    );
    push_scope(&mut builder, filter.scope);
    if let Some(folder) = filter.folder {
        builder.push(" AND (p.path = ").push_bind(folder.to_owned());
        builder
            .push(" OR p.path LIKE ")
            .push_bind(format!("{}/%", memory_ops::escape_like(folder)))
            .push(" ESCAPE '\\')");
    }
    if let Some(pattern) = filter.like_pattern {
        builder
            .push(" AND (p.content LIKE ")
            .push_bind(pattern.to_owned())
            .push(" ESCAPE '\\'");
        if filter.match_paths {
            builder
                .push(" OR p.path ILIKE ")
                .push_bind(pattern.to_owned())
                .push(" ESCAPE '\\'");
        }
        builder.push(")");
    }
    for condition in filter.conditions {
        push_condition(&mut builder, condition);
    }
    if let Some(seq) = filter.before_seq {
        builder.push(" AND v.seq < ").push_bind(seq);
    }
    builder
        .push(" ORDER BY v.seq DESC LIMIT ")
        .push_bind(filter.limit);
    builder.build_query_as::<MatchRow>().fetch_all(conn).await
}

fn push_scope(builder: &mut QueryBuilder<Postgres>, scope: &ScopeFilter) {
    if scope.unrestricted {
        return;
    }
    builder
        .push(" AND (p.path = ANY(")
        .push_bind(scope.exact.clone())
        .push(") OR p.path LIKE ANY(")
        .push_bind(scope.patterns.clone())
        .push("))");
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
                Value::Date(date) => memory_ops::format_iso_date(*date),
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
