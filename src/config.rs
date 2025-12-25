use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseType {
    MySQL,
    PostgreSQL,
}

impl DatabaseType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "mysql" => Some(DatabaseType::MySQL),
            "postgresql" | "postgres" | "psql" => Some(DatabaseType::PostgreSQL),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMode {
    Initial,
    Realtime,
    Both,
}

impl SyncMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "initial" => Some(SyncMode::Initial),
            "realtime" => Some(SyncMode::Realtime),
            "both" => Some(SyncMode::Both),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub source_url: String,
    pub target_url: String,
    pub source_type: DatabaseType,
    pub target_type: DatabaseType,
    pub sync_mode: SyncMode,
    pub batch_size: usize,
    pub source_database: String,
    pub target_database: String,
    // Connection details for display/logging and mysqldump
    pub source_host: String,
    pub source_port: u16,
    pub source_username: String,
    pub source_password: String,
    pub target_host: String,
    pub target_port: u16,
    pub target_username: String,
    pub target_password: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        // Determine source and target database types
        let source_type = env::var("SOURCE_DB_TYPE")
            .ok()
            .and_then(|s| DatabaseType::from_str(&s))
            .unwrap_or(DatabaseType::MySQL);

        let target_type = env::var("TARGET_DB_TYPE")
            .ok()
            .and_then(|s| DatabaseType::from_str(&s))
            .unwrap_or(DatabaseType::MySQL);

        // Build source URL based on type
        let source_url = if env::var("SOURCE_DB_URL").is_ok() {
            env::var("SOURCE_DB_URL")?
        } else {
            match source_type {
                DatabaseType::MySQL => build_mysql_url("SOURCE")?,
                DatabaseType::PostgreSQL => build_pg_url("SOURCE")?,
            }
        };

        // Build target URL based on type
        let target_url = if env::var("TARGET_DB_URL").is_ok() {
            env::var("TARGET_DB_URL")?
        } else {
            match target_type {
                DatabaseType::MySQL => build_mysql_url("TARGET")?,
                DatabaseType::PostgreSQL => build_pg_url("TARGET")?,
            }
        };

        let sync_mode = env::var("SYNC_MODE")
            .ok()
            .and_then(|s| SyncMode::from_str(&s))
            .unwrap_or(SyncMode::Both);

        let batch_size = env::var("BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        // Extract database names from URLs
        let source_database = extract_database_name(&source_url)?;
        let target_database = extract_database_name(&target_url)?;
        
        // Extract connection details for display and mysqldump
        let source_host = env::var(format!("SOURCE_DB_HOST")).unwrap_or_default();
        let source_port = env::var(format!("SOURCE_DB_PORT"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3306);
        let source_username = env::var(format!("SOURCE_DB_USERNAME")).unwrap_or_default();
        let source_password = env::var(format!("SOURCE_DB_PASSWORD")).unwrap_or_default();
        
        let target_host = env::var(format!("TARGET_DB_HOST")).unwrap_or_default();
        let target_port = env::var(format!("TARGET_DB_PORT"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3306);
        let target_username = env::var(format!("TARGET_DB_USERNAME")).unwrap_or_default();
        let target_password = env::var(format!("TARGET_DB_PASSWORD")).unwrap_or_default();

        Ok(Config {
            source_url,
            target_url,
            source_type,
            target_type,
            sync_mode,
            batch_size,
            source_database,
            target_database,
            source_host,
            source_port,
            source_username,
            source_password,
            target_host,
            target_port,
            target_username,
            target_password,
        })
    }
}

fn build_mysql_url(prefix: &str) -> anyhow::Result<String> {
    // Support SOURCE_* and TARGET_* prefixed variables
    // Example: SOURCE_DB_HOST, TARGET_DB_HOST
    
    let host = env::var(format!("{}_DB_HOST", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_HOST must be set", prefix))?;
    
    let port = env::var(format!("{}_DB_PORT", prefix))
        .unwrap_or_else(|_| "3306".to_string());
    
    let user = env::var(format!("{}_DB_USERNAME", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_USERNAME must be set", prefix))?;
    
    let password = env::var(format!("{}_DB_PASSWORD", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_PASSWORD must be set", prefix))?;
    
    let database = env::var(format!("{}_DB_DATABASE", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_DATABASE must be set", prefix))?;

    Ok(format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database))
}

fn build_pg_url(prefix: &str) -> anyhow::Result<String> {
    // Support SOURCE_* and TARGET_* prefixed variables
    // Example: SOURCE_DB_HOST, TARGET_DB_HOST
    
    let host = env::var(format!("{}_DB_HOST", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_HOST must be set", prefix))?;
    
    let port = env::var(format!("{}_DB_PORT", prefix))
        .unwrap_or_else(|_| "5432".to_string());
    
    let user = env::var(format!("{}_DB_USERNAME", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_USERNAME must be set", prefix))?;
    
    let password = env::var(format!("{}_DB_PASSWORD", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_PASSWORD must be set", prefix))?;
    
    let database = env::var(format!("{}_DB_DATABASE", prefix))
        .map_err(|_| anyhow::anyhow!("{}_DB_DATABASE must be set", prefix))?;

    Ok(format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database))
}

fn extract_database_name(url: &str) -> anyhow::Result<String> {
    // Parse mysql://user:pass@host:port/database or postgres://user:pass@host:port/database
    if let Some(db_start) = url.rfind('/') {
        let db_part = &url[db_start + 1..];
        if let Some(query_start) = db_part.find('?') {
            Ok(db_part[..query_start].to_string())
        } else {
            Ok(db_part.to_string())
        }
    } else {
        Err(anyhow::anyhow!("Invalid database URL format"))
    }
}

