use std::str::FromStr;

use sqlx::{PgConnection, PgPool};
use time::Date;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimeZone(String);

#[derive(Debug, thiserror::Error)]
pub(crate) enum TimeZoneError {
    #[error("time zone name is empty or has unexpected characters")]
    Malformed,
    #[error("time zone is not in the IANA database")]
    Unknown,
    #[error("database query failed")]
    Database(#[source] sqlx::Error),
}

impl TimeZone {
    pub(crate) fn utc() -> Self {
        Self("UTC".to_owned())
    }

    pub(crate) async fn ensure_known(&self, pool: &PgPool) -> Result<(), TimeZoneError> {
        let known = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM pg_timezone_names WHERE name = $1) AS "known!""#,
            self.0
        )
        .fetch_one(pool)
        .await
        .map_err(TimeZoneError::Database)?;
        if known {
            Ok(())
        } else {
            Err(TimeZoneError::Unknown)
        }
    }

    pub(crate) async fn today(&self, conn: &mut PgConnection) -> Result<Date, sqlx::Error> {
        sqlx::query_scalar!(
            r#"SELECT (now() AT TIME ZONE $1)::date AS "today!""#,
            self.0
        )
        .fetch_one(conn)
        .await
    }
}

impl FromStr for TimeZone {
    type Err = TimeZoneError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let well_formed = !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "/_+-".contains(c));
        if well_formed {
            Ok(Self(value.to_owned()))
        } else {
            Err(TimeZoneError::Malformed)
        }
    }
}

#[cfg(test)]
mod tests {
    use time::{Duration, OffsetDateTime, UtcOffset};

    use super::*;

    fn zone(name: &str) -> TimeZone {
        name.parse().unwrap()
    }

    #[test]
    fn malformed_names_are_rejected() {
        for name in ["", "Asia/Seoul; DROP", "Asia Seoul", "'UTC'"] {
            assert!(matches!(
                name.parse::<TimeZone>(),
                Err(TimeZoneError::Malformed)
            ));
        }
    }

    #[sqlx::test]
    async fn only_iana_names_are_known(pool: PgPool) {
        assert!(zone("Asia/Seoul").ensure_known(&pool).await.is_ok());
        assert!(zone("UTC").ensure_known(&pool).await.is_ok());
        assert!(matches!(
            zone("Mars/Olympus").ensure_known(&pool).await,
            Err(TimeZoneError::Unknown)
        ));
    }

    #[sqlx::test]
    async fn today_follows_the_configured_zone(pool: PgPool) {
        let mut conn = pool.acquire().await.unwrap();
        let cases = [("Pacific/Kiritimati", 14), ("Pacific/Pago_Pago", -11)];
        for (name, hours) in cases {
            let offset = UtcOffset::from_hms(hours, 0, 0).unwrap();
            let before = OffsetDateTime::now_utc().to_offset(offset).date();
            let today = zone(name).today(&mut conn).await.unwrap();
            let after = OffsetDateTime::now_utc().to_offset(offset).date();
            assert!(today == before || today == after, "{name}");
        }
        let far_apart = zone("Pacific/Kiritimati").today(&mut conn).await.unwrap()
            - zone("Pacific/Pago_Pago").today(&mut conn).await.unwrap();
        assert!(far_apart >= Duration::days(1));
    }
}
