use super::state::SyncConfig;
use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use tracing::info;
use chrono::{DateTime, Utc};

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

        // Create config table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS config (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                db_type TEXT NOT NULL DEFAULT 'mysql',
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
                sync_mode TEXT NOT NULL DEFAULT 'full-sync',
                gemini_api_key TEXT,
                gemini_model TEXT NOT NULL DEFAULT 'gemini-2.0-flash-exp',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;
        
        // Migrate existing tables: Add db_type column if it doesn't exist
        sqlx::query(
            r#"
            ALTER TABLE config ADD COLUMN db_type TEXT NOT NULL DEFAULT 'mysql'
            "#,
        )
        .execute(&pool)
        .await
        .ok(); // Ignore error if column already exists
        
        // Migrate existing tables: Add reset_database column if it doesn't exist
        sqlx::query(
            r#"
            ALTER TABLE config ADD COLUMN reset_database INTEGER NOT NULL DEFAULT 0
            "#,
        )
        .execute(&pool)
        .await
        .ok(); // Ignore error if column already exists

        // Create users table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create sessions table if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                expires_at DATETIME NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Migration: Add sync_mode column if it doesn't exist
        let _ = sqlx::query(
            r#"
            ALTER TABLE config ADD COLUMN sync_mode TEXT NOT NULL DEFAULT 'full-sync'
            "#,
        )
        .execute(&pool)
        .await; // Ignore errors - column might already exist

        // Create slaves table for multiple slave databases
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS slaves (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                database TEXT NOT NULL,
                username TEXT NOT NULL,
                password TEXT NOT NULL,
                sync_status TEXT NOT NULL DEFAULT 'pending',
                last_synced_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(host, port, database)
            )
            "#,
        )
        .execute(&pool)
        .await?;
        
        // Migrate existing tables: Add sync_status column if it doesn't exist
        sqlx::query(
            r#"
            ALTER TABLE slaves ADD COLUMN sync_status TEXT NOT NULL DEFAULT 'pending'
            "#,
        )
        .execute(&pool)
        .await
        .ok(); // Ignore error if column already exists
        
        // Migrate existing tables: Add last_synced_at column if it doesn't exist
        sqlx::query(
            r#"
            ALTER TABLE slaves ADD COLUMN last_synced_at DATETIME
            "#,
        )
        .execute(&pool)
        .await
        .ok(); // Ignore error if column already exists

        // Create operation_stats table for live sync statistics
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS operation_stats (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME NOT NULL,
                operation_type TEXT NOT NULL,
                table_name TEXT NOT NULL,
                success INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create index for faster queries
        let _ = sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_operation_stats_timestamp 
            ON operation_stats(timestamp)
            "#,
        )
        .execute(&pool)
        .await;

        info!("✅ Configuration store initialized at {}", db_path);

        Ok(Self { pool })
    }

    /// Save configuration to database
    pub async fn save_config(&self, config: &SyncConfig) -> Result<()> {
        // Map new field names to old database columns for backward compatibility
        // source_db_* → db_* columns
        // target_db_* → psql_db_* columns
        sqlx::query(
            r#"
            INSERT INTO config (
                id, db_type, db_host, db_port, db_database, db_username, db_password,
                psql_db_host, psql_db_port, psql_db_database, psql_db_username, psql_db_password,
                batch_size, poll_interval_secs, sync_mode, reset_database, gemini_api_key, gemini_model, updated_at
            ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, CURRENT_TIMESTAMP)
            ON CONFLICT(id) DO UPDATE SET
                db_type = ?1,
                db_host = ?2,
                db_port = ?3,
                db_database = ?4,
                db_username = ?5,
                db_password = ?6,
                psql_db_host = ?7,
                psql_db_port = ?8,
                psql_db_database = ?9,
                psql_db_username = ?10,
                psql_db_password = ?11,
                batch_size = ?12,
                poll_interval_secs = ?13,
                sync_mode = ?14,
                reset_database = ?15,
                gemini_api_key = ?16,
                gemini_model = ?17,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&config.db_type)
        .bind(&config.source_db_host)         // NEW: source_db_* → db_* columns
        .bind(config.source_db_port as i64)
        .bind(&config.source_db_database)
        .bind(&config.source_db_username)
        .bind(&config.source_db_password)
        .bind(&config.target_db_host)         // NEW: target_db_* → psql_db_* columns
        .bind(config.target_db_port as i64)
        .bind(&config.target_db_database)
        .bind(&config.target_db_username)
        .bind(&config.target_db_password)
        .bind(config.batch_size as i64)
        .bind(config.poll_interval_secs as i64)
        .bind(&config.sync_mode)
        .bind(config.reset_database as i64)  // Boolean as integer (0 or 1)
        .bind(&config.gemini_api_key)
        .bind(&config.gemini_model)
        .execute(&self.pool)
        .await?;

        // Save slaves to slaves table
        // First, clear existing slaves
        sqlx::query("DELETE FROM slaves")
            .execute(&self.pool)
            .await?;
        
        // Remove duplicates: track unique host:port:database combinations
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        let mut unique_slaves = Vec::new();
        
        for slave in &config.slaves {
            let key = format!("{}:{}:{}", slave.host, slave.port, slave.database);
            if seen.insert(key) {
                unique_slaves.push(slave);
            } else {
                info!("⚠️  Skipping duplicate slave: {}@{}:{}/{}", 
                    slave.username, slave.host, slave.port, slave.database);
            }
        }
        
        // Then insert unique slaves only
        let saved_count = unique_slaves.len();
        
        for slave in &unique_slaves {
            sqlx::query(
                r#"
                INSERT INTO slaves (host, port, database, username, password, sync_status, last_synced_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
            )
            .bind(&slave.host)
            .bind(slave.port as i64)
            .bind(&slave.database)
            .bind(&slave.username)
            .bind(&slave.password)
            .bind(&slave.sync_status)
            .bind(&slave.last_synced_at)
            .execute(&self.pool)
            .await?;
        }
        let duplicate_count = config.slaves.len() - saved_count;
        
        if duplicate_count > 0 {
            info!("✅ Configuration saved to database ({} unique slave(s), {} duplicate(s) removed)", 
                saved_count, duplicate_count);
        } else {
            info!("✅ Configuration saved to database (including {} slave(s))", saved_count);
        }
        Ok(())
    }

    /// Load configuration from database
    pub async fn load_config(&self) -> Result<Option<SyncConfig>> {
        let result = sqlx::query_as::<_, ConfigRow>(
            r#"
            SELECT 
                db_type, db_host, db_port, db_database, db_username, db_password,
                psql_db_host, psql_db_port, psql_db_database, psql_db_username, psql_db_password,
                batch_size, poll_interval_secs, sync_mode, 
                COALESCE(reset_database, 0) as reset_database, 
                gemini_api_key, gemini_model
            FROM config
            WHERE id = 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                // Load slaves from slaves table
                let slaves_result = sqlx::query_as::<_, SlaveRow>(
                    r#"
                    SELECT host, port, database, username, password, 
                           COALESCE(sync_status, 'pending') as sync_status, 
                           last_synced_at
                    FROM slaves
                    ORDER BY id
                    "#,
                )
                .fetch_all(&self.pool)
                .await?;
                
                let slaves: Vec<super::state::SlaveConfig> = slaves_result.into_iter().map(|s| super::state::SlaveConfig {
                    host: s.host,
                    port: s.port as u16,
                    database: s.database,
                    username: s.username,
                    password: s.password,
                    sync_status: s.sync_status,
                    last_synced_at: s.last_synced_at,
                }).collect();
                
                info!("✅ Configuration loaded from database (with {} slave(s))", slaves.len());
                // Map old database columns to new field names
                // db_* columns → source_db_*
                // psql_db_* columns → target_db_*
                Ok(Some(SyncConfig {
                    db_type: row.db_type,
                    source_db_host: row.db_host.clone(),            // NEW: db_* → source_db_*
                    source_db_port: row.db_port as u16,
                    source_db_database: row.db_database.clone(),
                    source_db_username: row.db_username.clone(),
                    source_db_password: row.db_password.clone(),
                    target_db_host: row.psql_db_host.clone(),       // NEW: psql_db_* → target_db_*
                    target_db_port: row.psql_db_port as u16,
                    target_db_database: row.psql_db_database.clone(),
                    target_db_username: row.psql_db_username.clone(),
                    target_db_password: row.psql_db_password.clone(),
                    // Keep old fields for backward compatibility
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
                    sync_mode: row.sync_mode,
                    reset_database: row.reset_database != 0,  // Convert i64 to bool
                    gemini_api_key: row.gemini_api_key,
                    gemini_model: row.gemini_model,
                    slaves,
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
    
    /// Update sync status for a slave database
    pub async fn update_slave_sync_status(&self, host: &str, port: u16, database: &str, status: &str, last_synced_at: Option<&str>) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE slaves 
            SET sync_status = ?1, last_synced_at = ?2
            WHERE host = ?3 AND port = ?4 AND database = ?5
            "#,
        )
        .bind(status)
        .bind(last_synced_at)
        .bind(host)
        .bind(port as i64)
        .bind(database)
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
    
    /// Get list of pending (unsynced) slaves
    pub async fn get_pending_slaves(&self) -> Result<Vec<super::state::SlaveConfig>> {
        let slaves_result = sqlx::query_as::<_, SlaveRow>(
            r#"
            SELECT host, port, database, username, password, 
                   COALESCE(sync_status, 'pending') as sync_status, 
                   last_synced_at
            FROM slaves
            WHERE sync_status = 'pending'
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        
        let slaves: Vec<super::state::SlaveConfig> = slaves_result.into_iter().map(|s| super::state::SlaveConfig {
            host: s.host,
            port: s.port as u16,
            database: s.database,
            username: s.username,
            password: s.password,
            sync_status: s.sync_status,
            last_synced_at: s.last_synced_at,
        }).collect();
        
        Ok(slaves)
    }

    /// Check if any users exist
    pub async fn has_users(&self) -> Result<bool> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await?;
        Ok(count.0 > 0)
    }

    /// Create a new user
    pub async fn create_user(&self, email: &str, password: &str) -> Result<String> {
        let user_id = uuid::Uuid::new_v4().to_string();
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;

        sqlx::query(
            r#"
            INSERT INTO users (id, email, password_hash)
            VALUES (?1, ?2, ?3)
            "#,
        )
        .bind(&user_id)
        .bind(email)
        .bind(&password_hash)
        .execute(&self.pool)
        .await?;

        info!("✅ User created: {}", email);
        Ok(user_id)
    }

    /// Get user by email
    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE email = ?1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Verify user credentials
    pub async fn verify_user(&self, email: &str, password: &str) -> Result<Option<String>> {
        if let Some(user) = self.get_user_by_email(email).await? {
            if bcrypt::verify(password, &user.password_hash)? {
                info!("✅ User verified: {}", email);
                return Ok(Some(user.id));
            }
        }
        Ok(None)
    }

    /// Create a new session
    pub async fn create_session(&self, user_id: &str) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::days(7); // 7 days

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at)
            VALUES (?1, ?2, ?3)
            "#,
        )
        .bind(&session_id)
        .bind(user_id)
        .bind(expires_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        info!("✅ Session created for user: {}", user_id);
        Ok(session_id)
    }

    /// Validate session
    pub async fn validate_session(&self, session_id: &str) -> Result<Option<String>> {
        let session = sqlx::query_as::<_, SessionRow>(
            r#"
            SELECT user_id, expires_at
            FROM sessions
            WHERE id = ?1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(session) = session {
            let expires_at = DateTime::parse_from_rfc3339(&session.expires_at)?;
            if expires_at.timestamp() > Utc::now().timestamp() {
                return Ok(Some(session.user_id));
            } else {
                // Delete expired session
                self.delete_session(session_id).await?;
            }
        }

        Ok(None)
    }

    /// Delete session (logout)
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM sessions WHERE id = ?1")
            .bind(session_id)
            .execute(&self.pool)
            .await?;

        info!("✅ Session deleted");
        Ok(())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("DELETE FROM sessions WHERE expires_at < ?1")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get user by ID
    pub async fn get_user_by_id(&self, user_id: &str) -> Result<Option<User>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, email, password_hash, created_at, updated_at
            FROM users
            WHERE id = ?1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update user email
    pub async fn update_user_email(&self, user_id: &str, new_email: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET email = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            "#,
        )
        .bind(new_email)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        info!("✅ User email updated: {}", new_email);
        Ok(())
    }

    /// Update user password
    pub async fn update_user_password(&self, user_id: &str, new_password: &str) -> Result<()> {
        let password_hash = bcrypt::hash(new_password, bcrypt::DEFAULT_COST)?;

        sqlx::query(
            r#"
            UPDATE users
            SET password_hash = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            "#,
        )
        .bind(&password_hash)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        info!("✅ User password updated for user ID: {}", user_id);
        Ok(())
    }

    /// Verify current password for user
    pub async fn verify_user_password(&self, user_id: &str, password: &str) -> Result<bool> {
        if let Some(user) = self.get_user_by_id(user_id).await? {
            return Ok(bcrypt::verify(password, &user.password_hash)?);
        }
        Ok(false)
    }

    /// Save an operation statistic
    pub async fn save_operation_stat(
        &self,
        timestamp: &str,
        operation_type: &str,
        table_name: &str,
        success: bool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO operation_stats (timestamp, operation_type, table_name, success)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(timestamp)
        .bind(operation_type)
        .bind(table_name)
        .bind(if success { 1 } else { 0 })
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Get hourly operation statistics
    pub async fn get_hourly_stats(&self, hours: i64) -> Result<Vec<HourlyStat>> {
        let stats = sqlx::query_as::<_, HourlyStat>(
            r#"
            SELECT 
                strftime('%Y-%m-%d %H:00:00', timestamp) as hour,
                COUNT(*) as total_operations,
                SUM(CASE WHEN operation_type = 'INSERT' THEN 1 ELSE 0 END) as inserts,
                SUM(CASE WHEN operation_type = 'UPDATE' THEN 1 ELSE 0 END) as updates,
                SUM(CASE WHEN operation_type = 'DELETE' THEN 1 ELSE 0 END) as deletes,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as successful,
                SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as failed
            FROM operation_stats
            WHERE timestamp >= datetime('now', '-' || ?1 || ' hours')
            GROUP BY strftime('%Y-%m-%d %H:00:00', timestamp)
            ORDER BY hour ASC
            "#,
        )
        .bind(hours)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(stats)
    }

    /// Clear old operation statistics (older than N days)
    pub async fn cleanup_old_stats(&self, days: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            DELETE FROM operation_stats
            WHERE timestamp < datetime('now', '-' || ?1 || ' days')
            "#,
        )
        .bind(days)
        .execute(&self.pool)
        .await?;
        
        Ok(result.rows_affected())
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ConfigRow {
    db_type: String,
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
    sync_mode: String,
    reset_database: i64,  // Boolean stored as integer (0 or 1)
    gemini_api_key: Option<String>,
    gemini_model: String,
}

#[derive(Debug, sqlx::FromRow)]
struct SlaveRow {
    host: String,
    port: i64,
    database: String,
    username: String,
    password: String,
    sync_status: String,
    last_synced_at: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
struct SessionRow {
    user_id: String,
    expires_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct HourlyStat {
    pub hour: String,
    pub total_operations: i64,
    pub inserts: i64,
    pub updates: i64,
    pub deletes: i64,
    pub successful: i64,
    pub failed: i64,
}

