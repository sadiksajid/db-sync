use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    // MySQL Configuration
    pub db_host: String,
    pub db_port: u16,
    pub db_database: String,
    pub db_username: String,
    pub db_password: String,
    
    // PostgreSQL Configuration
    pub psql_db_host: String,
    pub psql_db_port: u16,
    pub psql_db_database: String,
    pub psql_db_username: String,
    pub psql_db_password: String,
    
    // Sync Configuration
    pub batch_size: usize,
    pub poll_interval_secs: u64,
    
    // Gemini API Configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            db_host: String::new(),
            db_port: 3306,
            db_database: String::new(),
            db_username: String::new(),
            db_password: String::new(),
            psql_db_host: String::new(),
            psql_db_port: 5432,
            psql_db_database: String::new(),
            psql_db_username: String::new(),
            psql_db_password: String::new(),
            batch_size: 100,
            poll_interval_secs: 10,
            gemini_api_key: None,
            gemini_model: "gemini-2.0-flash-exp".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SyncStatus {
    Idle,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStats {
    pub tables_synced: usize,
    pub rows_synced: usize,
    pub views_synced: usize,
    pub functions_synced: usize,
    pub procedures_synced: usize,
    pub triggers_synced: usize,
    pub inserts_applied: usize,
    pub updates_applied: usize,
    pub deletes_applied: usize,
    pub errors_count: usize,
    pub start_time: Option<String>,
    pub last_sync_time: Option<String>,
}

impl Default for SyncStats {
    fn default() -> Self {
        Self {
            tables_synced: 0,
            rows_synced: 0,
            views_synced: 0,
            functions_synced: 0,
            procedures_synced: 0,
            triggers_synced: 0,
            inserts_applied: 0,
            updates_applied: 0,
            deletes_applied: 0,
            errors_count: 0,
            start_time: None,
            last_sync_time: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: Arc<RwLock<SyncConfig>>,
    pub status: Arc<RwLock<SyncStatus>>,
    pub stats: Arc<RwLock<SyncStats>>,
    pub logs: Arc<Mutex<Vec<String>>>,
    pub config_store: Arc<super::ConfigStore>,
}

impl AppState {
    pub async fn new(config_store: Arc<super::ConfigStore>) -> Self {
        // Try to load saved configuration
        let saved_config = match config_store.load_config().await {
            Ok(Some(cfg)) => {
                tracing::info!("✅ Loaded saved configuration");
                cfg
            }
            Ok(None) => {
                tracing::info!("ℹ️  No saved configuration, using defaults");
                SyncConfig::default()
            }
            Err(e) => {
                tracing::warn!("⚠️  Failed to load config: {}, using defaults", e);
                SyncConfig::default()
            }
        };

        Self {
            config: Arc::new(RwLock::new(saved_config)),
            status: Arc::new(RwLock::new(SyncStatus::Idle)),
            stats: Arc::new(RwLock::new(SyncStats::default())),
            logs: Arc::new(Mutex::new(Vec::new())),
            config_store,
        }
    }

    pub async fn add_log(&self, message: String) {
        let mut logs = self.logs.lock().await;
        logs.push(message);
        // Keep only last 1000 logs
        if logs.len() > 1000 {
            let drain_count = logs.len() - 1000;
            logs.drain(0..drain_count);
        }
    }
}

