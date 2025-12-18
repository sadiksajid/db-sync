use chrono::{DateTime, Utc, Timelike, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub timestamp: String,
    pub hour: String, // "YYYY-MM-DD HH:00:00"
    pub operation_type: String, // "INSERT", "UPDATE", "DELETE"
    pub table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStats {
    pub hour: String,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub total: u64,
}

pub struct StatsLogger {
    log_file_path: String,
    stats: Arc<Mutex<Vec<OperationStats>>>,
    hourly_cache: Arc<Mutex<HashMap<String, HourlyStats>>>,
    last_display: Arc<Mutex<DateTime<Utc>>>,
}

impl StatsLogger {
    pub fn new(log_file_path: &str) -> Self {
        // Load existing stats if file exists
        let existing_stats = if Path::new(log_file_path).exists() {
            Self::load_stats_from_file(log_file_path)
        } else {
            Vec::new()
        };

        let hourly_cache = Self::build_hourly_cache(&existing_stats);

        Self {
            log_file_path: log_file_path.to_string(),
            stats: Arc::new(Mutex::new(existing_stats)),
            hourly_cache: Arc::new(Mutex::new(hourly_cache)),
            last_display: Arc::new(Mutex::new(Utc::now())),
        }
    }

    /// Load existing stats from JSON file
    fn load_stats_from_file(path: &str) -> Vec<OperationStats> {
        match File::open(path) {
            Ok(file) => {
                let reader = BufReader::new(file);
                match serde_json::from_reader(reader) {
                    Ok(stats) => {
                        let count: usize = serde_json::to_value(&stats)
                            .ok()
                            .and_then(|v| v.as_array().map(|a| a.len()))
                            .unwrap_or(0);
                        info!("📊 Loaded {} existing stats from {}", count, path);
                        stats
                    }
                    Err(e) => {
                        warn!("Failed to parse stats file: {}", e);
                        Vec::new()
                    }
                }
            }
            Err(_) => Vec::new(),
        }
    }

    /// Build hourly summary cache from stats
    fn build_hourly_cache(stats: &[OperationStats]) -> HashMap<String, HourlyStats> {
        let mut cache = HashMap::new();

        for stat in stats {
            let entry = cache.entry(stat.hour.clone()).or_insert(HourlyStats {
                hour: stat.hour.clone(),
                inserts: 0,
                updates: 0,
                deletes: 0,
                total: 0,
            });

            match stat.operation_type.as_str() {
                "INSERT" => entry.inserts += 1,
                "UPDATE" => entry.updates += 1,
                "DELETE" => entry.deletes += 1,
                _ => {}
            }
            entry.total += 1;
        }

        cache
    }

    /// Log an operation
    pub async fn log_operation(&self, operation_type: &str, table: &str) {
        let now = Utc::now();
        let hour = format!(
            "{:04}-{:02}-{:02} {:02}:00:00",
            now.year(),
            now.month(),
            now.day(),
            now.hour()
        );

        let stat = OperationStats {
            timestamp: now.to_rfc3339(),
            hour: hour.clone(),
            operation_type: operation_type.to_string(),
            table: table.to_string(),
        };

        // Update in-memory stats
        {
            let mut stats = self.stats.lock().await;
            stats.push(stat);
        }

        // Update hourly cache
        {
            let mut cache = self.hourly_cache.lock().await;
            let entry = cache.entry(hour.clone()).or_insert(HourlyStats {
                hour: hour.clone(),
                inserts: 0,
                updates: 0,
                deletes: 0,
                total: 0,
            });

            match operation_type {
                "INSERT" => entry.inserts += 1,
                "UPDATE" => entry.updates += 1,
                "DELETE" => entry.deletes += 1,
                _ => {}
            }
            entry.total += 1;
        }

        debug!("📊 Logged {} on table {}", operation_type, table);

        // Check if we should display hourly summary (every 5 minutes)
        self.maybe_display_hourly_summary().await;
    }

    /// Display hourly summary if enough time has passed
    async fn maybe_display_hourly_summary(&self) {
        let mut last_display = self.last_display.lock().await;
        let now = Utc::now();
        let elapsed = now.signed_duration_since(*last_display).num_seconds();

        if elapsed >= 300 {
            // Display every 5 minutes
            self.display_current_hour_stats().await;
            *last_display = now;
        }
    }

    /// Display current hour statistics
    async fn display_current_hour_stats(&self) {
        let now = Utc::now();
        let current_hour = format!(
            "{:04}-{:02}-{:02} {:02}:00:00",
            now.year(),
            now.month(),
            now.day(),
            now.hour()
        );

        let cache = self.hourly_cache.lock().await;
        if let Some(stats) = cache.get(&current_hour) {
            info!(
                "📊 {} - {} inserts, {} updates, {} deletes (total: {})",
                current_hour, stats.inserts, stats.updates, stats.deletes, stats.total
            );
        } else {
            info!("📊 {} - No operations yet this hour", current_hour);
        }
    }

    /// Flush stats to disk
    pub async fn flush_to_disk(&self) -> Result<(), anyhow::Error> {
        let stats = self.stats.lock().await;
        
        let json = serde_json::to_string_pretty(&*stats)?;
        
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.log_file_path)?;
        
        file.write_all(json.as_bytes())?;
        file.flush()?;
        
        debug!("💾 Flushed {} stats to {}", stats.len(), self.log_file_path);
        Ok(())
    }

    /// Get hourly statistics (for displaying full summary)
    pub async fn get_hourly_stats(&self) -> Vec<HourlyStats> {
        let cache = self.hourly_cache.lock().await;
        let mut stats: Vec<HourlyStats> = cache.values().cloned().collect();
        stats.sort_by(|a, b| a.hour.cmp(&b.hour));
        stats
    }

    /// Display full hourly summary
    pub async fn display_full_summary(&self) {
        let stats = self.get_hourly_stats().await;
        
        if stats.is_empty() {
            info!("📊 No operations recorded yet");
            return;
        }

        info!("📊 ═══════════════════════════════════════════════════════════════");
        info!("📊 HOURLY OPERATION STATISTICS");
        info!("📊 ═══════════════════════════════════════════════════════════════");
        
        for stat in &stats {
            info!(
                "📊 {} - {} inserts, {} updates, {} deletes (total: {})",
                stat.hour, stat.inserts, stat.updates, stat.deletes, stat.total
            );
        }

        // Find the hour with lowest activity
        if let Some(min_stat) = stats.iter().min_by_key(|s| s.total) {
            info!("📊 ═══════════════════════════════════════════════════════════════");
            info!("📊 💡 BEST TIME TO SWITCH: {} (only {} operations)", min_stat.hour, min_stat.total);
            info!("📊 ═══════════════════════════════════════════════════════════════");
        }
    }

    /// Start background flush task (flush every 30 seconds)
    pub fn start_flush_task(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                if let Err(e) = self.flush_to_disk().await {
                    warn!("Failed to flush stats to disk: {}", e);
                }
            }
        });
    }
}

