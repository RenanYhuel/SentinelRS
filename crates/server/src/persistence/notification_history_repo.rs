use sqlx::PgPool;

pub struct NotificationHistoryRecord {
    pub id: String,
    pub alert_id: String,
    pub notifier_id: String,
    pub ntype: String,
    pub status: String,
    pub error: Option<String>,
    pub attempts: i32,
    pub duration_ms: i32,
    pub sent_at_ms: i64,
}

pub struct NotificationHistoryRepo {
    pool: PgPool,
}

impl NotificationHistoryRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, r: &NotificationHistoryRecord) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"INSERT INTO notification_history
               (id, alert_id, notifier_id, ntype, status, error, attempts, duration_ms, sent_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, to_timestamp($9::double precision / 1000))
               ON CONFLICT (id) DO NOTHING"#,
        )
        .bind(&r.id)
        .bind(&r.alert_id)
        .bind(&r.notifier_id)
        .bind(&r.ntype)
        .bind(&r.status)
        .bind(&r.error)
        .bind(r.attempts)
        .bind(r.duration_ms)
        .bind(r.sent_at_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<NotificationHistoryRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, HistoryRow>(
            "SELECT id, alert_id, notifier_id, ntype, status, error, attempts, duration_ms,
                    EXTRACT(EPOCH FROM sent_at)::bigint * 1000 AS sent_at_ms
             FROM notification_history
             ORDER BY sent_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_by_notifier(
        &self,
        notifier_id: &str,
        limit: i64,
    ) -> Result<Vec<NotificationHistoryRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, HistoryRow>(
            "SELECT id, alert_id, notifier_id, ntype, status, error, attempts, duration_ms,
                    EXTRACT(EPOCH FROM sent_at)::bigint * 1000 AS sent_at_ms
             FROM notification_history
             WHERE notifier_id = $1
             ORDER BY sent_at DESC
             LIMIT $2",
        )
        .bind(notifier_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn list_by_alert(
        &self,
        alert_id: &str,
        limit: i64,
    ) -> Result<Vec<NotificationHistoryRecord>, sqlx::Error> {
        let rows = sqlx::query_as::<_, HistoryRow>(
            "SELECT id, alert_id, notifier_id, ntype, status, error, attempts, duration_ms,
                    EXTRACT(EPOCH FROM sent_at)::bigint * 1000 AS sent_at_ms
             FROM notification_history
             WHERE alert_id = $1
             ORDER BY sent_at DESC
             LIMIT $2",
        )
        .bind(alert_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn stats(&self) -> Result<HistoryStats, sqlx::Error> {
        let row = sqlx::query_as::<_, StatsRow>(
            "SELECT
                COUNT(*)::bigint AS total,
                COUNT(*) FILTER (WHERE status = 'sent')::bigint AS sent,
                COUNT(*) FILTER (WHERE status = 'failed')::bigint AS failed,
                COALESCE(AVG(duration_ms) FILTER (WHERE status = 'sent'), 0)::bigint AS avg_duration_ms
             FROM notification_history",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(HistoryStats {
            total: row.total,
            sent: row.sent,
            failed: row.failed,
            avg_duration_ms: row.avg_duration_ms,
        })
    }
}

pub struct HistoryStats {
    pub total: i64,
    pub sent: i64,
    pub failed: i64,
    pub avg_duration_ms: i64,
}

#[derive(sqlx::FromRow)]
struct HistoryRow {
    id: String,
    alert_id: String,
    notifier_id: String,
    ntype: String,
    status: String,
    error: Option<String>,
    attempts: i32,
    duration_ms: i32,
    sent_at_ms: i64,
}

impl From<HistoryRow> for NotificationHistoryRecord {
    fn from(r: HistoryRow) -> Self {
        Self {
            id: r.id,
            alert_id: r.alert_id,
            notifier_id: r.notifier_id,
            ntype: r.ntype,
            status: r.status,
            error: r.error,
            attempts: r.attempts,
            duration_ms: r.duration_ms,
            sent_at_ms: r.sent_at_ms,
        }
    }
}

#[derive(sqlx::FromRow)]
struct StatsRow {
    total: i64,
    sent: i64,
    failed: i64,
    avg_duration_ms: i64,
}
