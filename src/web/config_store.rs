use super::state::SyncConfig;
use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct ConfigStore {
    pool: SqlitePool,
}

impl ConfigStore {
    /// Initialize the config store with SQLite database
    pub async fn new(db_path: &str) -> Result<Self> {
        // Convert to absolute path
        let abs_path = std::path::PathBuf::from(db_path);
        let abs_path_str = abs_path.to_string_lossy();
        
        // Ensure parent directory exists
        if let Some(parent) = abs_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create the file if it doesn't exist (SQLite needs write permission)
        if !abs_path.exists() {
            std::fs::File::create(&abs_path)?;
            info!("Created new config database at {}", abs_path_str);
        }

        // SQLite connection string format with proper file URI
        let database_url = format!("sqlite:{}", abs_path_str);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&database_url)
            .await?;

        // Create table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                db_host TEXT NOT NULL,
                db_port INTEGER NOT NULL,
                db_database TEXT NOT NULL,
                db_username TEXT NOT NULL,
                db_password TEXT NOT NULL,
                psql_db_host TEXT NOT NULL,
                psql_db_port INTEGER NOT NULL,
                psql_db_database TEXT NOT NULL,
                psql_db_username TEXT NOT NULL,
                psql_db_password TEXT NOT NULL,
                batch_size INTEGER NOT NULL DEFAULT 100,
                poll_interval_secs INTEGER NOT NULL DEFAULT 10,
                gemini_api_key TEXT,
                gemini_model TEXT NOT NULL DEFAULT 'gemini-2.0-flash-exp',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        info!("✅ Configuration store initialized at {}", db_path);

        Ok(Self { pool })
    }

    /// Save configuration to database
    pub async fn save_config(&self, config: &SyncConfig) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO config (
                id, db_host, db_port, db_database, db_username, db_password,
                psql_db_host, psql_db_port, psql_db_database, psql_db_username, psql_db_password,
                batch_size, poll_interval_secs, gemini_api_key, gemini_model, updated_at
            ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                db_host = ?1,
                db_port = ?2,
                db_database = ?3,
                db_username = ?4,
                db_password = ?5,
                psql_db_host = ?6,
                psql_db_port = ?7,
                psql_db_database = ?8,
                psql_db_username = ?9,
                psql_db_password = ?10,
                batch_size = ?11,
                poll_interval_secs = ?12,
                gemini_api_key = ?13,
                gemini_model = ?14,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&config.db_host)
        .bind(config.db_port as i64)
        .bind(&config.db_database)
        .bind(&config.db_username)
        .bind(&config.db_password)
        .bind(&config.psql_db_host)
        .bind(config.psql_db_port as i64)
        .bind(&config.psql_db_database)
        .bind(&config.psql_db_username)
        .bind(&config.psql_db_password)
        .bind(config.batch_size as i64)
        .bind(config.poll_interval_secs as i64)
        .bind(&config.gemini_api_key)
        .bind(&config.gemini_model)
        .execute(&self.pool)
        .await?;

        info!("✅ Configuration saved to database");
        Ok(())
    }

    /// Load configuration from database
    pub async fn load_config(&self) -> Result<Option<SyncConfig>> {
        let result = sqlx::query_as::<_, ConfigRow>(
            r#"
            SELECT 
                db_host, db_port, db_database, db_username, db_password,
                psql_db_host, psql_db_port, psql_db_database, psql_db_username, psql_db_password,
                batch_size, poll_interval_secs, gemini_api_key, gemini_model
            FROM config
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                info!("✅ Configuration loaded from database");
                Ok(Some(SyncConfig {
                    db_host: row.db_host,
                    db_port: row.db_port as u16,
                    db_database: row.db_database,
                    db_username: row.db_username,
                    db_password: row.db_password,
                    psql_db_host: row.psql_db_host,
                    psql_db_port: row.psql_db_port as u16,
                    psql_db_database: row.psql_db_database,
                    psql_db_username: row.psql_db_username,
                    psql_db_password: row.psql_db_password,
                    batch_size: row.batch_size as usize,
                    poll_interval_secs: row.poll_interval_secs as u64,
                    gemini_api_key: row.gemini_api_key,
                    gemini_model: row.gemini_model,
                }))
            }
            None => {
                info!("ℹ️  No saved configuration found");
                Ok(None)
            }
        }
    }

    /// Clear saved configuration
    pub async fn clear_config(&self) -> Result<()> {
        sqlx::query("DELETE FROM config WHERE id = 1")
            .execute(&self.pool)
            .await?;

        info!("✅ Configuration cleared from database");
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigRow {
    db_host: String,
    db_port: i64,
    db_database: String,
    db_username: String,
    db_password: String,
    psql_db_host: String,
    psql_db_port: i64,
    psql_db_database: String,
    psql_db_username: String,
    psql_db_password: String,
    batch_size: i64,
    poll_interval_secs: i64,
    gemini_api_key: Option<String>,
    gemini_model: String,
}

