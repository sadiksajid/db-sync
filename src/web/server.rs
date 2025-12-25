use super::{state::*, ApiResponse};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware,
    response::{Html, IntoResponse, Json, Response},
    routing::{get, post},
    Extension,
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::services::ServeDir;
use tracing::{error, info};

pub async fn start_web_server(state: AppState) -> anyhow::Result<()> {
    // Protected routes (require authentication)
    let protected_routes = Router::new()
        .route("/api/config", get(get_config).post(update_config))
        .route("/api/status", get(get_status))
        .route("/api/stats", get(get_stats))
        .route("/api/logs", get(get_logs))
        .route("/api/chart-stats", get(get_chart_stats))
        .route("/api/sync/start", post(start_sync))
        .route("/api/sync/stop", post(stop_sync))
        .route("/api/test-connection", post(test_connection))
        .route("/api/auth/logout", post(logout))
        .route("/api/profile/me", get(get_profile))
        .route("/api/profile/update-email", post(update_email))
        .route("/api/profile/update-password", post(update_password))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Public routes (no authentication required)
    let public_routes = Router::new()
        .route("/", get(serve_index_with_check))
        .route("/login", get(serve_login))
        .route("/api/auth/check", get(check_auth))
        .route("/api/auth/has-users", get(has_users))
        .route("/api/auth/setup", post(setup_first_user))
        .route("/api/auth/login", post(login));

    let app = Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        .nest_service("/static", ServeDir::new("static"))
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

async fn serve_index_with_check(
    State(state): State<AppState>,
    req: Request,
) -> Result<Html<String>, StatusCode> {
    // Check if any users exist
    let has_users = state.config_store.has_users().await.unwrap_or(true);
    
    if !has_users {
        // No users exist, allow access to show setup modal
        return Ok(serve_index().await);
    }
    
    // Users exist, check authentication
    let session_id = req
        .headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        });
    
    if let Some(session_id) = session_id {
        // Validate session
        match state.config_store.validate_session(&session_id).await {
            Ok(Some(_user_id)) => {
                // Authenticated, serve the page
                return Ok(serve_index().await);
            }
            _ => {
                // Invalid session, redirect to login
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }
    
    // Not authenticated, redirect to login
    Err(StatusCode::UNAUTHORIZED)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HourlyChartData {
    hour: String,
    inserts: u64,
    updates: u64,
    deletes: u64,
    total: u64,
}

async fn get_chart_stats(State(state): State<AppState>) -> Json<ApiResponse<Vec<HourlyChartData>>> {
    // Get hourly stats from database (last 24 hours)
    match state.config_store.get_hourly_stats(24).await {
        Ok(stats) => {
            // Convert database stats to chart format
            let chart_data: Vec<HourlyChartData> = stats.into_iter().map(|stat| HourlyChartData {
                hour: stat.hour,
                inserts: stat.inserts as u64,
                updates: stat.updates as u64,
                deletes: stat.deletes as u64,
                total: stat.total_operations as u64,
            }).collect();
            
            Json(ApiResponse::success("Chart stats retrieved", Some(chart_data)))
        }
        Err(e) => {
            error!("Failed to get chart stats from database: {}", e);
            Json(ApiResponse::success("No stats available", Some(Vec::new())))
        }
    }
}

async fn start_sync(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    let mut status = state.status.write().await;
    
    if *status == SyncStatus::Running {
        return Json(ApiResponse::error("Sync is already running"));
    }

    let config = state.config.read().await.clone();
    
    // Validate configuration
    if config.source_db_host.is_empty()
        || config.source_db_database.is_empty()
        || config.target_db_host.is_empty()
        || config.target_db_database.is_empty()
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
    
    info!("Test connection request - Type: {}", db_type);
    info!("Payload config: {:?}", payload.get("config"));
    
    // Parse config from payload instead of reading from state
    let config: SyncConfig = match serde_json::from_value(payload.get("config").cloned().unwrap_or(serde_json::json!({}))) {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to parse config: {}", e);
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
        Err(e) => {
            error!("Connection test failed: {}", e);
            Json(ApiResponse::error(format!(
                "{} connection failed: {}",
                db_type.to_uppercase(),
                e
            )))
        }
    }
}

async fn test_mysql_connection(config: &SyncConfig) -> anyhow::Result<()> {
    // Try new field names first (source_db_* or target_db_*), then fall back to old names (db_* or psql_db_*)
    let (host, port, database, username, password) = if !config.source_db_host.is_empty() {
        (&config.source_db_host, config.source_db_port, &config.source_db_database, &config.source_db_username, &config.source_db_password)
    } else if !config.target_db_host.is_empty() {
        (&config.target_db_host, config.target_db_port, &config.target_db_database, &config.target_db_username, &config.target_db_password)
    } else if !config.db_host.is_empty() {
        (&config.db_host, config.db_port, &config.db_database, &config.db_username, &config.db_password)
    } else {
        (&config.psql_db_host, config.psql_db_port, &config.psql_db_database, &config.psql_db_username, &config.psql_db_password)
    };
    
    if host.is_empty() || database.is_empty() || username.is_empty() {
        return Err(anyhow::anyhow!("Missing required database connection fields"));
    }
    
    let url = format!(
        "mysql://{}:{}@{}:{}/{}",
        username, password, host, port, database
    );
    
    info!("Testing MySQL connection to: {}@{}:{}/{}", username, host, port, database);
    
    let pool = sqlx::mysql::MySqlPool::connect(&url).await?;
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    pool.close().await;
    
    Ok(())
}

async fn test_postgres_connection(config: &SyncConfig) -> anyhow::Result<()> {
    // Try new field names first (source_db_* or target_db_*), then fall back to old names (db_* or psql_db_*)
    let (host, port, database, username, password) = if !config.source_db_host.is_empty() {
        (&config.source_db_host, config.source_db_port, &config.source_db_database, &config.source_db_username, &config.source_db_password)
    } else if !config.target_db_host.is_empty() {
        (&config.target_db_host, config.target_db_port, &config.target_db_database, &config.target_db_username, &config.target_db_password)
    } else if !config.psql_db_host.is_empty() {
        (&config.psql_db_host, config.psql_db_port, &config.psql_db_database, &config.psql_db_username, &config.psql_db_password)
    } else {
        (&config.db_host, config.db_port, &config.db_database, &config.db_username, &config.db_password)
    };
    
    if host.is_empty() || database.is_empty() || username.is_empty() {
        return Err(anyhow::anyhow!("Missing required database connection fields"));
    }
    
    let url = format!(
        "postgresql://{}:{}@{}:{}/{}",
        username, password, host, port, database
    );
    
    info!("Testing PostgreSQL connection to: {}@{}:{}/{}", username, host, port, database);
    
    let pool = sqlx::postgres::PgPool::connect(&url).await?;
    sqlx::query("SELECT 1").fetch_one(&pool).await?;
    pool.close().await;
    
    Ok(())
}

async fn run_sync_task(state: AppState) -> anyhow::Result<()> {
    use chrono::Utc;
    
    state.add_log("Starting synchronization...".to_string()).await;
    
    // Load web config from state
    let web_config = state.config.read().await.clone();
    
    // DEBUG: Log the loaded configuration
    state.add_log(format!("📋 Loaded configuration from SQLite:")).await;
    state.add_log(format!("  db_type: '{}'", &web_config.db_type)).await;
    state.add_log(format!("  source_db_host: '{}'", &web_config.source_db_host)).await;
    state.add_log(format!("  source_db_port: {}", web_config.source_db_port)).await;
    state.add_log(format!("  source_db_database: '{}'", &web_config.source_db_database)).await;
    state.add_log(format!("  source_db_username: '{}'", &web_config.source_db_username)).await;
    
    // Get all slave configs for PARALLEL sync
    state.add_log(format!("🔄 Preparing configs for all slave databases...")).await;
    let slave_configs = match web_config.to_slave_configs() {
        Ok(configs) => {
            state.add_log(format!("✓ Found {} slave database(s) to sync", configs.len())).await;
            for (idx, cfg) in configs.iter().enumerate() {
                state.add_log(format!("  Slave #{}: {}@{}:{}/{}", 
                    idx + 1, 
                    cfg.target_username, 
                    cfg.target_host, 
                    cfg.target_port, 
                    cfg.target_database
                )).await;
            }
            configs
        }
        Err(e) => {
            let error_msg = format!("Configuration error: {}", e);
            state.add_log(error_msg.clone()).await;
            return Err(e);
        }
    };
    
    // Update stats
    {
        let mut stats = state.stats.write().await;
        stats.start_time = Some(Utc::now().to_rfc3339());
    }
    
    // Determine which sync mode to run
    let sync_mode = &web_config.sync_mode;
    state.add_log(format!("📋 Selected sync mode: {}", sync_mode)).await;
    state.add_log(format!("🚀 Starting PARALLEL sync to {} slave(s)...", slave_configs.len())).await;
    
    // Run sync to ALL slaves in PARALLEL using tokio::spawn
    let mut handles = vec![];
    
    for (idx, slave_config) in slave_configs.into_iter().enumerate() {
        let slave_num = idx + 1;
        let state_clone = state.clone();
        let sync_mode_clone = sync_mode.clone();
        let slave_db = slave_config.target_database.clone();
        
        // Spawn a separate task for each slave
        let handle = tokio::spawn(async move {
            state_clone.add_log(format!("🔵 [Slave #{}] Starting sync to '{}'...", slave_num, slave_db)).await;
            
            let result = match sync_mode_clone.as_str() {
                "initial-sync" => {
                    crate::run_initial_only_for_ui(slave_config, state_clone.clone()).await
                }
                "realtime-sync" => {
                    crate::run_realtime_only_for_ui(slave_config, state_clone.clone()).await
                }
                _ => {
                    crate::run_full_sync_for_ui(slave_config, state_clone.clone()).await
                }
            };
            
            match result {
                Ok(_) => {
                    state_clone.add_log(format!("✅ [Slave #{}] Sync completed successfully for '{}'", slave_num, slave_db)).await;
                    Ok(())
                }
                Err(e) => {
                    state_clone.add_log(format!("❌ [Slave #{}] Sync failed for '{}': {}", slave_num, slave_db, e)).await;
                    Err(e)
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for ALL slaves to complete
    state.add_log("⏳ Waiting for all slave syncs to complete...".to_string()).await;
    
    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors = Vec::new();
    
    for (idx, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(_)) => {
                success_count += 1;
            }
            Ok(Err(e)) => {
                error_count += 1;
                errors.push(format!("Slave #{}: {}", idx + 1, e));
            }
            Err(e) => {
                error_count += 1;
                errors.push(format!("Slave #{}: Task panicked: {}", idx + 1, e));
            }
        }
    }
    
    // Report final status
    state.add_log(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")).await;
    state.add_log(format!("📊 PARALLEL SYNC RESULTS:")).await;
    state.add_log(format!("  ✅ Successful: {}", success_count)).await;
    state.add_log(format!("  ❌ Failed: {}", error_count)).await;
    state.add_log(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")).await;
    
    if error_count == 0 {
        state.add_log("🎉 All slave databases synchronized successfully!".to_string()).await;
        *state.status.write().await = SyncStatus::Idle;
        Ok(())
    } else {
        let error_summary = errors.join("; ");
        state.add_log(format!("⚠️  Some syncs failed: {}", error_summary)).await;
        *state.status.write().await = SyncStatus::Error(error_summary.clone());
        Err(anyhow::anyhow!("Sync failures: {}", error_summary))
    }
}

// Authentication middleware
async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: middleware::Next,
) -> Result<Response, StatusCode> {
    // Extract session ID from cookie
    let session_id = req
        .headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        });

    if let Some(session_id) = session_id {
        // Validate session
        match state.config_store.validate_session(&session_id).await {
            Ok(Some(user_id)) => {
                // Session is valid, add user_id to extensions
                req.extensions_mut().insert(user_id);
                return Ok(next.run(req).await);
            }
            _ => {
                // Invalid or expired session
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}

// Serve login page
async fn serve_login() -> Html<String> {
    let html = std::fs::read_to_string("static/login.html")
        .unwrap_or_else(|_| {
            r#"<!DOCTYPE html>
<html>
<head><title>Login</title></head>
<body><h1>Failed to load login.html</h1></body>
</html>"#.to_string()
        });
    Html(html)
}

// Check if user is authenticated
async fn check_auth(State(state): State<AppState>, req: Request) -> Json<ApiResponse<bool>> {
    let session_id = req
        .headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        });

    if let Some(session_id) = session_id {
        match state.config_store.validate_session(&session_id).await {
            Ok(Some(_user_id)) => {
                return Json(ApiResponse::success("Authenticated", Some(true)));
            }
            _ => {}
        }
    }

    Json(ApiResponse::success("Not authenticated", Some(false)))
}

// Check if any users exist
async fn has_users(State(state): State<AppState>) -> Json<ApiResponse<bool>> {
    match state.config_store.has_users().await {
        Ok(has_users) => Json(ApiResponse::success("Check completed", Some(has_users))),
        Err(e) => {
            error!("Failed to check users: {}", e);
            Json(ApiResponse::error(format!("Failed to check users: {}", e)))
        }
    }
}

#[derive(Debug, Deserialize)]
struct SetupRequest {
    email: String,
    password: String,
}

// Setup first user
async fn setup_first_user(
    State(state): State<AppState>,
    Json(req): Json<SetupRequest>,
) -> Json<ApiResponse<String>> {
    // Check if users already exist
    match state.config_store.has_users().await {
        Ok(true) => {
            return Json(ApiResponse::error("Users already exist"));
        }
        Ok(false) => {}
        Err(e) => {
            error!("Failed to check users: {}", e);
            return Json(ApiResponse::error(format!("Failed to check users: {}", e)));
        }
    }

    // Validate email
    if !req.email.contains('@') {
        return Json(ApiResponse::error("Invalid email address"));
    }

    // Validate password length
    if req.password.len() < 6 {
        return Json(ApiResponse::error("Password must be at least 6 characters"));
    }

    // Create first user
    match state.config_store.create_user(&req.email, &req.password).await {
        Ok(_user_id) => {
            info!("✅ First user created: {}", req.email);
            Json(ApiResponse::success("First user created successfully", Some(req.email)))
        }
        Err(e) => {
            error!("Failed to create user: {}", e);
            Json(ApiResponse::error(format!("Failed to create user: {}", e)))
        }
    }
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    session_id: String,
}

// Login
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // Verify credentials
    match state.config_store.verify_user(&req.email, &req.password).await {
        Ok(Some(user_id)) => {
            // Create session
            match state.config_store.create_session(&user_id).await {
                Ok(session_id) => {
                    // Set cookie
                    let cookie = format!(
                        "session_id={}; Path=/; HttpOnly; Max-Age=604800; SameSite=Strict",
                        session_id
                    );

                    (
                        StatusCode::OK,
                        [(header::SET_COOKIE, cookie)],
                        Json(ApiResponse::success(
                            "Login successful",
                            Some(LoginResponse { session_id }),
                        )),
                    )
                }
                Err(e) => {
                    error!("Failed to create session: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        [(header::SET_COOKIE, String::new())],
                        Json(ApiResponse::error(format!("Failed to create session: {}", e))),
                    )
                }
            }
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            [(header::SET_COOKIE, String::new())],
            Json(ApiResponse::error("Invalid email or password")),
        ),
        Err(e) => {
            error!("Login error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [(header::SET_COOKIE, String::new())],
                Json(ApiResponse::error(format!("Login error: {}", e))),
            )
        }
    }
}

// Logout
async fn logout(State(state): State<AppState>, req: Request) -> impl IntoResponse {
    let session_id = req
        .headers()
        .get(header::COOKIE)
        .and_then(|cookie| cookie.to_str().ok())
        .and_then(|cookie_str| {
            cookie_str
                .split(';')
                .find_map(|cookie| {
                    let parts: Vec<&str> = cookie.trim().splitn(2, '=').collect();
                    if parts.len() == 2 && parts[0] == "session_id" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                })
        });

    if let Some(session_id) = session_id {
        let _ = state.config_store.delete_session(&session_id).await;
    }

    // Clear cookie
    let cookie = "session_id=; Path=/; HttpOnly; Max-Age=0; SameSite=Strict";

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(ApiResponse::<()>::success("Logout successful", None)),
    )
}

// Get current user profile
async fn get_profile(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
) -> Json<ApiResponse<UserProfile>> {
    match state.config_store.get_user_by_id(&user_id).await {
        Ok(Some(user)) => {
            Json(ApiResponse::success(
                "Profile retrieved",
                Some(UserProfile {
                    email: user.email,
                    created_at: user.created_at,
                }),
            ))
        }
        Ok(None) => {
            Json(ApiResponse::error("User not found"))
        }
        Err(e) => {
            error!("Failed to get user: {}", e);
            Json(ApiResponse::error(format!("Failed to get user: {}", e)))
        }
    }
}

#[derive(Debug, Serialize)]
struct UserProfile {
    email: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct UpdateEmailRequest {
    new_email: String,
    current_password: String,
}

// Update user email
async fn update_email(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(update_req): Json<UpdateEmailRequest>,
) -> Json<ApiResponse<()>> {
    // Verify current password
    match state.config_store.verify_user_password(&user_id, &update_req.current_password).await {
        Ok(true) => {
            // Check if new email is valid
            if !update_req.new_email.contains('@') {
                return Json(ApiResponse::error("Invalid email address"));
            }
            
            // Update email
            match state.config_store.update_user_email(&user_id, &update_req.new_email).await {
                Ok(_) => {
                    Json(ApiResponse::success("Email updated successfully", None))
                }
                Err(e) => {
                    error!("Failed to update email: {}", e);
                    Json(ApiResponse::error(format!("Failed to update email: {}", e)))
                }
            }
        }
        Ok(false) => {
            Json(ApiResponse::error("Current password is incorrect"))
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            Json(ApiResponse::error("Password verification failed"))
        }
    }
}

#[derive(Debug, Deserialize)]
struct UpdatePasswordRequest {
    current_password: String,
    new_password: String,
}

// Update user password
async fn update_password(
    State(state): State<AppState>,
    Extension(user_id): Extension<String>,
    Json(update_req): Json<UpdatePasswordRequest>,
) -> Json<ApiResponse<()>> {
    // Verify current password
    match state.config_store.verify_user_password(&user_id, &update_req.current_password).await {
        Ok(true) => {
            // Validate new password length
            if update_req.new_password.len() < 6 {
                return Json(ApiResponse::error("New password must be at least 6 characters long"));
            }
            
            // Update password
            match state.config_store.update_user_password(&user_id, &update_req.new_password).await {
                Ok(_) => {
                    Json(ApiResponse::success("Password updated successfully", None))
                }
                Err(e) => {
                    error!("Failed to update password: {}", e);
                    Json(ApiResponse::error(format!("Failed to update password: {}", e)))
                }
            }
        }
        Ok(false) => {
            Json(ApiResponse::error("Current password is incorrect"))
        }
        Err(e) => {
            error!("Password verification error: {}", e);
            Json(ApiResponse::error("Password verification failed"))
        }
    }
}
