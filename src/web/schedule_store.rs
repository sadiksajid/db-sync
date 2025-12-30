use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Schedule {
    pub id: i64,
    pub name: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSchedule {
    pub name: String,
    pub cron_expression: String,
    pub enabled: bool,
}

#[derive(Debug)]
pub struct ScheduleStore {
    pool: SqlitePool,
}

impl ScheduleStore {
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_url = format!("sqlite:{}", db_path.as_ref().display());
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        let store = Self { pool };
        store.init_table().await?;
        Ok(store)
    }

    async fn init_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS schedules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                cron_expression TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                last_run TEXT,
                next_run TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("✅ Schedules table initialized");
        Ok(())
    }

    pub async fn create(&self, schedule: CreateSchedule) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO schedules (name, cron_expression, enabled)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&schedule.name)
        .bind(&schedule.cron_expression)
        .bind(schedule.enabled)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn get_all(&self) -> Result<Vec<Schedule>> {
        let schedules = sqlx::query_as::<_, Schedule>(
            r#"
            SELECT id, name, cron_expression, enabled, last_run, next_run, created_at
            FROM schedules
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(schedules)
    }

    pub async fn get_enabled(&self) -> Result<Vec<Schedule>> {
        let schedules = sqlx::query_as::<_, Schedule>(
            r#"
            SELECT id, name, cron_expression, enabled, last_run, next_run, created_at
            FROM schedules
            WHERE enabled = 1
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(schedules)
    }

    pub async fn get_by_id(&self, id: i64) -> Result<Option<Schedule>> {
        let schedule = sqlx::query_as::<_, Schedule>(
            r#"
            SELECT id, name, cron_expression, enabled, last_run, next_run, created_at
            FROM schedules
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(schedule)
    }

    pub async fn update(&self, id: i64, schedule: CreateSchedule) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE schedules
            SET name = ?, cron_expression = ?, enabled = ?
            WHERE id = ?
            "#,
        )
        .bind(&schedule.name)
        .bind(&schedule.cron_expression)
        .bind(schedule.enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_last_run(&self, id: i64, last_run: &str, next_run: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE schedules
            SET last_run = ?, next_run = ?
            WHERE id = ?
            "#,
        )
        .bind(last_run)
        .bind(next_run)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn toggle_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE schedules
            SET enabled = ?
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        sqlx::query(
            r#"
            DELETE FROM schedules
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

