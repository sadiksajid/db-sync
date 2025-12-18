use super::{state::*, ApiResponse};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::{error, info};

pub async fn start_web_server(state: AppState) -> anyhow::Result<()> {
    let app = Router::new()
        // Serve static files (HTML, CSS, JS)
        .nest_service("/static", ServeDir::new("static"))
        // Root serves the main HTML
        .route("/", get(serve_index))
        // API routes
        .route("/api/config", get(get_config).post(update_config))
        .route("/api/status", get(get_status))
        .route("/api/stats", get(get_stats))
        .route("/api/logs", get(get_logs))
        .route("/api/chart-stats", get(get_chart_stats))
        .route("/api/sync/start", post(start_sync))
        .route("/api/sync/stop", post(stop_sync))
        .route("/api/test-connection", post(test_connection))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5009")
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to port 5009: {}", e))?;

    info!("🌐 Web UI started at http://0.0.0.0:5009");
    info!("📊 Open your browser to configure and run the sync");

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

async fn serve_index() -> Html<String> {
    // Read the HTML file at runtime from the static directory
    let html = std::fs::read_to_string("static/index.html")
        .unwrap_or_else(|_| {
            r#"<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body><h1>Failed to load index.html</h1><p>Make sure static/index.html exists.</p></body>
</html>"#.to_string()
        });
    Html(html)
}

async fn get_config(State(state): State<AppState>) -> Json<ApiResponse<SyncConfig>> {
    let config = state.config.read().await.clone();
    Json(ApiResponse::success("Configuration retrieved", Some(config)))
}

async fn update_config(
    State(state): State<AppState>,
    Json(new_config): Json<SyncConfig>,
) -> Json<ApiResponse<()>> {
    let status = state.status.read().await.clone();
    
    if status == SyncStatus::Running {
        return Json(ApiResponse::error(
            "Cannot update configuration while sync is running",
        ));
    }

    // Save to database for persistence
    if let Err(e) = state.config_store.save_config(&new_config).await {
        error!("Failed to save config to database: {}", e);
        return Json(ApiResponse::error(format!(
            "Failed to save configuration: {}",
            e
        )));
    }

    *state.config.write().await = new_config;
    state.add_log("✅ Configuration saved (persisted to database)".to_string()).await;
    
    Json(ApiResponse::success("Configuration saved successfully", None))
}

async fn get_status(State(state): State<AppState>) -> Json<ApiResponse<SyncStatus>> {
    let status = state.status.read().await.clone();
    Json(ApiResponse::success("Status retrieved", Some(status)))
}

async fn get_stats(State(state): State<AppState>) -> Json<ApiResponse<SyncStats>> {
    let stats = state.stats.read().await.clone();
    Json(ApiResponse::success("Statistics retrieved", Some(stats)))
}

async fn get_logs(State(state): State<AppState>) -> Json<ApiResponse<Vec<String>>> {
    let logs = state.logs.lock().await.clone();
    Json(ApiResponse::success("Logs retrieved", Some(logs)))
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OperationStatsRecord {
    timestamp: String,
    hour: String,
    operation_type: String,
    table: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HourlyChartData {
    hour: String,
    inserts: u64,
    updates: u64,
    deletes: u64,
    total: u64,
}

async fn get_chart_stats() -> Json<ApiResponse<Vec<HourlyChartData>>> {
    let stats_file_path = "sync_operations_stats.json";
    
    // Read and parse the stats file
    let stats_data: Vec<OperationStatsRecord> = match std::fs::read_to_string(stats_file_path) {
        Ok(content) => {
            match serde_json::from_str(&content) {
                Ok(data) => data,
                Err(e) => {
                    error!("Failed to parse stats file: {}", e);
                    return Json(ApiResponse::success("No stats available", Some(Vec::new())));
                }
            }
        }
        Err(_) => {
            // File doesn't exist yet or can't be read
            return Json(ApiResponse::success("No stats available", Some(Vec::new())));
        }
    };
    
    // Group by hour and count operations
    let mut hourly_map: HashMap<String, HourlyChartData> = HashMap::new();
    
    for record in stats_data {
        let entry = hourly_map.entry(record.hour.clone()).or_insert(HourlyChartData {
            hour: record.hour.clone(),
            inserts: 0,
            updates: 0,
            deletes: 0,
            total: 0,
        });
        
        match record.operation_type.to_uppercase().as_str() {
            "INSERT" => entry.inserts += 1,
            "UPDATE" => entry.updates += 1,
            "DELETE" => entry.deletes += 1,
            _ => {}
        }
        entry.total += 1;
    }
    
    // Convert to sorted vec (by hour)
    let mut chart_data: Vec<HourlyChartData> = hourly_map.into_values().collect();
    chart_data.sort_by(|a, b| a.hour.cmp(&b.hour));
    
    Json(ApiResponse::success("Chart stats retrieved", Some(chart_data)))
}

async fn start_sync(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let mut status = state.status.write().await;
    
    if *status == SyncStatus::Running {
        return Json(ApiResponse::error("Sync is already running"));
    }

    let config = state.config.read().await.clone();
    
    // Validate configuration
    if config.db_host.is_empty()
        || config.db_database.is_empty()
        || config.psql_db_host.is_empty()
        || config.psql_db_database.is_empty()
    {
        return Json(ApiResponse::error(
            "Invalid configuration: Please fill in all required fields",
        ));
    }

    *status = SyncStatus::Running;
    state.add_log("Sync started".to_string()).await;

    // Spawn the sync task in the background
    let state_clone = state.clone();
    tokio::spawn(async move {
        if let Err(e) = run_sync_task(state_clone).await {
            error!("Sync task failed: {}", e);
        }
    });

    Json(ApiResponse::success(
        "Sync started successfully",
        Some("running".to_string()),
    ))
}

async fn stop_sync(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let mut status = state.status.write().await;
    
    if *status != SyncStatus::Running {
        return Json(ApiResponse::error("Sync is not running"));
    }

    *status = SyncStatus::Stopped;
    state.add_log("Sync stopped by user".to_string()).await;
    
    Json(ApiResponse::success(
        "Sync stopped successfully",
        Some("stopped".to_string()),
    ))
}

async fn test_connection(
    State(_state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<ApiResponse<String>> {
    let db_type = payload
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("mysql");
    
    // Parse config from payload instead of reading from state
    let config: SyncConfig = match serde_json::from_value(payload.get("config").cloned().unwrap_or(serde_json::json!({}))) {
        Ok(cfg) => cfg,
        Err(e) => {
            return Json(ApiResponse::error(format!("Invalid configuration: {}", e)));
        }
    };
    
    let result = if db_type == "mysql" {
        test_mysql_connection(&config).await
    } else {
        test_postgres_connection(&config).await
    };

    match result {
        Ok(_) => Json(ApiResponse::success(
            format!("{} connection successful", db_type.to_uppercase()),
            Some("connected".to_string()),
        )),
        Err(e) => Json(ApiResponse::error(format!(
            "{} connection failed: {}",
            db_type.to_uppercase(),
            e
        ))),
    }
}

async fn test_mysql_connection(config: &SyncConfig) -> anyhow::Result<()> {
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        config.db_username,
        config.db_password,
        config.db_host,
        config.db_port,
        config.db_database
    );
    
    let pool = sqlx::mysql::MySqlPool::connect(&url).await?;
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    pool.close().await;
    
    Ok(())
}

async fn test_postgres_connection(config: &SyncConfig) -> anyhow::Result<()> {
    let url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        config.psql_db_username,
        config.psql_db_password,
        config.psql_db_host,
        config.psql_db_port,
        config.psql_db_database
    );
    
    let pool = sqlx::postgres::PgPool::connect(&url).await?;
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    pool.close().await;
    
    Ok(())
}

async fn run_sync_task(state: AppState) -> anyhow::Result<()> {
    use chrono::Utc;
    
    state.add_log("Starting synchronization...".to_string()).await;
    
    let config = state.config.read().await.clone();
    
    // Update stats
    {
        let mut stats = state.stats.write().await;
        stats.start_time = Some(Utc::now().to_rfc3339());
    }
    
    // Set environment variables for the sync process
    std::env::set_var("DB_HOST", &config.db_host);
    std::env::set_var("DB_PORT", config.db_port.to_string());
    std::env::set_var("DB_DATABASE", &config.db_database);
    std::env::set_var("DB_USERNAME", &config.db_username);
    std::env::set_var("DB_PASSWORD", &config.db_password);
    std::env::set_var("PSQL_DB_HOST", &config.psql_db_host);
    std::env::set_var("PSQL_DB_PORT", config.psql_db_port.to_string());
    std::env::set_var("PSQL_DB_DATABASE", &config.psql_db_database);
    std::env::set_var("PSQL_DB_USERNAME", &config.psql_db_username);
    std::env::set_var("PSQL_DB_PASSWORD", &config.psql_db_password);
    std::env::set_var("BATCH_SIZE", config.batch_size.to_string());
    std::env::set_var("POLL_INTERVAL_SECS", config.poll_interval_secs.to_string());
    
    if let Some(api_key) = &config.gemini_api_key {
        std::env::set_var("GEMINI_API_KEY", api_key);
    }
    std::env::set_var("GEMINI_MODEL", &config.gemini_model);
    
    state.add_log("Running full synchronization (initial + realtime)...".to_string()).await;
    
    // Run the sync (this will block until sync completes or is stopped)
    let result = crate::run_full_sync_for_ui(state.clone()).await;
    
    match result {
        Ok(_) => {
            state.add_log("Synchronization completed successfully".to_string()).await;
            *state.status.write().await = SyncStatus::Idle;
        }
        Err(e) => {
            let error_msg = format!("Synchronization failed: {}", e);
            state.add_log(error_msg.clone()).await;
            *state.status.write().await = SyncStatus::Error(e.to_string());
        }
    }
    
    Ok(())
}

