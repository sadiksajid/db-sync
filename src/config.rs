use serde::{Deserialize, Serialize};
use std::env;

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
    pub mysql_url: String,
    pub pg_url: String,
    pub sync_mode: SyncMode,
    pub batch_size: usize,
    pub mysql_database: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        // Build MySQL URL - support multiple naming conventions
        let mysql_url = if env::var("MYSQL_URL").is_ok() {
            env::var("MYSQL_URL")?
        } else {
            build_mysql_url()?
        };

        // Build PostgreSQL URL - support multiple naming conventions
        let pg_url = if env::var("PG_URL").is_ok() {
            env::var("PG_URL")?
        } else {
            build_pg_url()?
        };

        let sync_mode = env::var("SYNC_MODE")
            .ok()
            .and_then(|s| SyncMode::from_str(&s))
            .unwrap_or(SyncMode::Both);

        let batch_size = env::var("BATCH_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1000);

        // Extract database name from MySQL URL
        let mysql_database = extract_database_name(&mysql_url)?;

        Ok(Config {
            mysql_url,
            pg_url,
            sync_mode,
            batch_size,
            mysql_database,
        })
    }
}

fn build_mysql_url() -> anyhow::Result<String> {
    // Support multiple patterns (in order of preference):
    // 1. MYSQL_* variables
    // 2. DB_* variables (for MySQL)
    
    let host = env::var("MYSQL_HOST")
        .or_else(|_| env::var("DB_HOST"))
        .map_err(|_| anyhow::anyhow!("MYSQL_HOST or DB_HOST must be set"))?;
    
    let port = env::var("MYSQL_PORT")
        .or_else(|_| env::var("DB_PORT"))
        .unwrap_or_else(|_| "3306".to_string());
    
    let user = env::var("MYSQL_USER")
        .or_else(|_| env::var("DB_USERNAME"))
        .map_err(|_| anyhow::anyhow!("MYSQL_USER or DB_USERNAME must be set"))?;
    
    let password = env::var("MYSQL_PASSWORD")
        .or_else(|_| env::var("DB_PASSWORD"))
        .map_err(|_| anyhow::anyhow!("MYSQL_PASSWORD or DB_PASSWORD must be set"))?;
    
    let database = env::var("MYSQL_DATABASE")
        .or_else(|_| env::var("DB_DATABASE"))
        .map_err(|_| anyhow::anyhow!("MYSQL_DATABASE or DB_DATABASE must be set"))?;

    Ok(format!("mysql://{}:{}@{}:{}/{}", user, password, host, port, database))
}

fn build_pg_url() -> anyhow::Result<String> {
    // Support multiple patterns (in order of preference):
    // 1. POSTGRES_* variables
    // 2. PSQL_DB_* variables (for PostgreSQL)
    
    let host = env::var("POSTGRES_HOST")
        .or_else(|_| env::var("PSQL_DB_HOST"))
        .map_err(|_| anyhow::anyhow!("POSTGRES_HOST or PSQL_DB_HOST must be set"))?;
    
    let port = env::var("POSTGRES_PORT")
        .or_else(|_| env::var("PSQL_DB_PORT"))
        .unwrap_or_else(|_| "5432".to_string());
    
    let user = env::var("POSTGRES_USER")
        .or_else(|_| env::var("PSQL_DB_USERNAME"))
        .map_err(|_| anyhow::anyhow!("POSTGRES_USER or PSQL_DB_USERNAME must be set"))?;
    
    let password = env::var("POSTGRES_PASSWORD")
        .or_else(|_| env::var("PSQL_DB_PASSWORD"))
        .map_err(|_| anyhow::anyhow!("POSTGRES_PASSWORD or PSQL_DB_PASSWORD must be set"))?;
    
    let database = env::var("POSTGRES_DB")
        .or_else(|_| env::var("PSQL_DB_DATABASE"))
        .map_err(|_| anyhow::anyhow!("POSTGRES_DB or PSQL_DB_DATABASE must be set"))?;

    Ok(format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, database))
}

fn extract_database_name(url: &str) -> anyhow::Result<String> {
    // Parse mysql://user:pass@host:port/database
    if let Some(db_start) = url.rfind('/') {
        let db_part = &url[db_start + 1..];
        if let Some(query_start) = db_part.find('?') {
            Ok(db_part[..query_start].to_string())
        } else {
            Ok(db_part.to_string())
        }
    } else {
        Err(anyhow::anyhow!("Invalid MySQL URL format"))
    }
}

