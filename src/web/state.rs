use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Slave database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaveConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    #[serde(default = "default_sync_status")]
    pub sync_status: String,  // "pending", "syncing", "synced", "failed"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
}

fn default_sync_status() -> String {
    "pending".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    // Database Type Configuration (mysql or postgresql)
    #[serde(default = "default_db_type")]
    pub db_type: String,
    
    // Source Database Configuration (Master)
    #[serde(default)]
    pub source_db_host: String,
    #[serde(default = "default_db_port")]
    pub source_db_port: u16,
    #[serde(default)]
    pub source_db_database: String,
    #[serde(default)]
    pub source_db_username: String,
    #[serde(default)]
    pub source_db_password: String,
    
    // Target Database Configuration (Primary Slave - for backward compatibility)
    #[serde(default)]
    pub target_db_host: String,
    #[serde(default = "default_db_port")]
    pub target_db_port: u16,
    #[serde(default)]
    pub target_db_database: String,
    #[serde(default)]
    pub target_db_username: String,
    #[serde(default)]
    pub target_db_password: String,
    
    // Multiple Slave Databases (for parallel sync)
    #[serde(default)]
    pub slaves: Vec<SlaveConfig>,
    
    // OLD field names for backward compatibility (will be removed)
    #[serde(default, skip_serializing)]
    pub db_host: String,
    #[serde(default, skip_serializing)]
    pub db_port: u16,
    #[serde(default, skip_serializing)]
    pub db_database: String,
    #[serde(default, skip_serializing)]
    pub db_username: String,
    #[serde(default, skip_serializing)]
    pub db_password: String,
    #[serde(default, skip_serializing)]
    pub psql_db_host: String,
    #[serde(default, skip_serializing)]
    pub psql_db_port: u16,
    #[serde(default, skip_serializing)]
    pub psql_db_database: String,
    #[serde(default, skip_serializing)]
    pub psql_db_username: String,
    #[serde(default, skip_serializing)]
    pub psql_db_password: String,
    
    // Sync Configuration
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_sync_mode")]
    pub sync_mode: String,
    #[serde(default)]
    pub reset_database: bool,  // Drop and recreate databases before sync
    
    // Gemini API Configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini_api_key: Option<String>,
    #[serde(default = "default_gemini_model")]
    pub gemini_model: String,
}

fn default_db_type() -> String {
    "mysql".to_string()
}

fn default_db_port() -> u16 {
    3306  // Default, will change based on db_type
}

fn default_batch_size() -> usize {
    100
}

fn default_poll_interval() -> u64 {
    10
}

fn default_sync_mode() -> String {
    "full-sync".to_string()
}

fn default_gemini_model() -> String {
    "gemini-2.0-flash-exp".to_string()
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            db_type: "mysql".to_string(),
            source_db_host: String::new(),
            source_db_port: 3306,
            source_db_database: String::new(),
            source_db_username: String::new(),
            source_db_password: String::new(),
            target_db_host: String::new(),
            target_db_port: 3306,
            target_db_database: String::new(),
            target_db_username: String::new(),
            target_db_password: String::new(),
            // Old fields for backward compatibility
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
            sync_mode: "full-sync".to_string(),
            reset_database: false,
            gemini_api_key: None,
            gemini_model: "gemini-2.0-flash-exp".to_string(),
            slaves: Vec::new(),
        }
    }
}

impl SyncConfig {
    /// Convert web SyncConfig to main Config struct (bypassing environment variables!)
    pub fn to_main_config(&self) -> anyhow::Result<crate::config::Config> {
        use crate::config::{Config, DatabaseType, SyncMode};
        
        // Validate required fields
        if self.source_db_host.is_empty() {
            return Err(anyhow::anyhow!("Source database host is required"));
        }
        if self.source_db_database.is_empty() {
            return Err(anyhow::anyhow!("Source database name is required"));
        }
        if self.target_db_host.is_empty() {
            return Err(anyhow::anyhow!("Target database host is required"));
        }
        if self.target_db_database.is_empty() {
            return Err(anyhow::anyhow!("Target database name is required"));
        }
        
        // Determine database type
        let db_type = DatabaseType::from_str(&self.db_type)
            .ok_or_else(|| anyhow::anyhow!("Invalid database type: {}", self.db_type))?;
        
        // Build database URLs
        let source_url = match db_type {
            DatabaseType::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.source_db_username,
                self.source_db_password,
                self.source_db_host,
                self.source_db_port,
                self.source_db_database
            ),
            DatabaseType::PostgreSQL => format!(
                "postgresql://{}:{}@{}:{}/{}",
                self.source_db_username,
                self.source_db_password,
                self.source_db_host,
                self.source_db_port,
                self.source_db_database
            ),
        };
        
        let target_url = match db_type {
            DatabaseType::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.target_db_username,
                self.target_db_password,
                self.target_db_host,
                self.target_db_port,
                self.target_db_database
            ),
            DatabaseType::PostgreSQL => format!(
                "postgresql://{}:{}@{}:{}/{}",
                self.target_db_username,
                self.target_db_password,
                self.target_db_host,
                self.target_db_port,
                self.target_db_database
            ),
        };
        
        // Determine sync mode
        let sync_mode = SyncMode::from_str(&self.sync_mode)
            .unwrap_or(SyncMode::Both);
        
        Ok(Config {
            source_url,
            target_url,
            source_type: db_type.clone(),
            target_type: db_type,  // Same type for same-type sync
            sync_mode,
            batch_size: self.batch_size,
            source_database: self.source_db_database.clone(),
            target_database: self.target_db_database.clone(),
            // Connection details for display/logging and mysqldump
            source_host: self.source_db_host.clone(),
            source_port: self.source_db_port,
            source_username: self.source_db_username.clone(),
            source_password: self.source_db_password.clone(),
            target_host: self.target_db_host.clone(),
            target_port: self.target_db_port,
            target_username: self.target_db_username.clone(),
            target_password: self.target_db_password.clone(),
        })
    }
    
    /// Create Config objects for all slave databases (for parallel sync)
    pub fn to_slave_configs(&self) -> anyhow::Result<Vec<crate::config::Config>> {
        use crate::config::{Config, DatabaseType, SyncMode};
        
        // Validate source fields
        if self.source_db_host.is_empty() {
            return Err(anyhow::anyhow!("Source database host is required"));
        }
        if self.source_db_database.is_empty() {
            return Err(anyhow::anyhow!("Source database name is required"));
        }
        
        // Determine database type
        let db_type = DatabaseType::from_str(&self.db_type)
            .ok_or_else(|| anyhow::anyhow!("Invalid database type: {}", self.db_type))?;
        
        // Build source URL (same for all slaves)
        let source_url = match db_type {
            DatabaseType::MySQL => format!(
                "mysql://{}:{}@{}:{}/{}",
                self.source_db_username,
                self.source_db_password,
                self.source_db_host,
                self.source_db_port,
                self.source_db_database
            ),
            DatabaseType::PostgreSQL => format!(
                "postgresql://{}:{}@{}:{}/{}",
                self.source_db_username,
                self.source_db_password,
                self.source_db_host,
                self.source_db_port,
                self.source_db_database
            ),
        };
        
        // Determine sync mode
        let sync_mode = SyncMode::from_str(&self.sync_mode)
            .unwrap_or(SyncMode::Both);
        
        let mut configs = Vec::new();
        
        // If no slaves configured, use primary target for backward compatibility
        if self.slaves.is_empty() && !self.target_db_host.is_empty() && !self.target_db_database.is_empty() {
            let target_url = match db_type {
                DatabaseType::MySQL => format!(
                    "mysql://{}:{}@{}:{}/{}",
                    self.target_db_username,
                    self.target_db_password,
                    self.target_db_host,
                    self.target_db_port,
                    self.target_db_database
                ),
                DatabaseType::PostgreSQL => format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    self.target_db_username,
                    self.target_db_password,
                    self.target_db_host,
                    self.target_db_port,
                    self.target_db_database
                ),
            };
            
            configs.push(Config {
                source_url: source_url.clone(),
                target_url,
                source_type: db_type.clone(),
                target_type: db_type.clone(),
                sync_mode: sync_mode.clone(),
                batch_size: self.batch_size,
                source_database: self.source_db_database.clone(),
                target_database: self.target_db_database.clone(),
                source_host: self.source_db_host.clone(),
                source_port: self.source_db_port,
                source_username: self.source_db_username.clone(),
                source_password: self.source_db_password.clone(),
                target_host: self.target_db_host.clone(),
                target_port: self.target_db_port,
                target_username: self.target_db_username.clone(),
                target_password: self.target_db_password.clone(),
            });
        }
        // Otherwise, ONLY use slaves array (don't duplicate with primary target)
        else {
            for slave in &self.slaves {
            let target_url = match db_type {
                DatabaseType::MySQL => format!(
                    "mysql://{}:{}@{}:{}/{}",
                    slave.username,
                    slave.password,
                    slave.host,
                    slave.port,
                    slave.database
                ),
                DatabaseType::PostgreSQL => format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    slave.username,
                    slave.password,
                    slave.host,
                    slave.port,
                    slave.database
                ),
            };
            
            configs.push(Config {
                source_url: source_url.clone(),
                target_url,
                source_type: db_type.clone(),
                target_type: db_type.clone(),
                sync_mode: sync_mode.clone(),
                batch_size: self.batch_size,
                source_database: self.source_db_database.clone(),
                target_database: slave.database.clone(),
                source_host: self.source_db_host.clone(),
                source_port: self.source_db_port,
                source_username: self.source_db_username.clone(),
                source_password: self.source_db_password.clone(),
                target_host: slave.host.clone(),
                target_port: slave.port,
                target_username: slave.username.clone(),
                target_password: slave.password.clone(),
            });
            }
        }
        
        if configs.is_empty() {
            return Err(anyhow::anyhow!("No slave databases configured"));
        }
        
        Ok(configs)
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

