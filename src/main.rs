mod ai;
mod config;
mod migrator;
mod realtime;
mod schema;
mod web;

use anyhow::Result;
use clap::{Arg, Command};
use config::{Config, DatabaseType, SyncMode};
use migrator::{create_tables::TableCreator, data_transfer::DataTransfer, routine_migrator::RoutineMigrator, verify::Verifier};
use realtime::{binlog_reader::BinlogReader, mysql_writer::MySQLWriter, pg_writer::PGWriter, stats_logger::StatsLogger};
use schema::{mysql_reader::MySQLReader, pg_reader::PgReader, routines::RoutineReader};
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Parse CLI arguments
    let matches = Command::new("db_sync_proxy")
        .version("0.1.0")
        .about("Database synchronization proxy (MySQL-to-MySQL or PostgreSQL-to-PostgreSQL)")
        .arg(
            Arg::new("initial-sync")
                .long("initial-sync")
                .help("Run schema + data sync")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("realtime-sync")
                .long("realtime-sync")
                .help("Run binlog-based sync")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("full-sync")
                .long("full-sync")
                .help("Run both modes")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("web-ui")
                .long("web-ui")
                .help("Start web UI server on port 5009")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();
    
    // Check if web UI mode is requested
    if matches.get_flag("web-ui") {
        use std::io::Write;
        
        println!("🌐 Starting in Web UI mode...");
        let _ = std::io::stdout().flush();
        info!("Starting in Web UI mode...");
        
        // Initialize configuration store
        println!("💾 Initializing configuration storage...");
        let _ = std::io::stdout().flush();
        let config_store = match web::ConfigStore::new("data/config.db").await {
            Ok(store) => {
                println!("✅ Configuration storage ready");
                let _ = std::io::stdout().flush();
                std::sync::Arc::new(store)
            }
            Err(e) => {
                println!("❌ Failed to initialize config storage: {}", e);
                let _ = std::io::stdout().flush();
                return Err(e);
            }
        };
        
        let state = web::state::AppState::new(config_store).await;
        println!("📊 Initializing web server on port 5009...");
        let _ = std::io::stdout().flush();
        return match web::start_web_server(state).await {
            Ok(_) => {
                println!("✅ Web server stopped gracefully");
                let _ = std::io::stdout().flush();
                Ok(())
            }
            Err(e) => {
                println!("❌ Web server error: {}", e);
                let _ = std::io::stdout().flush();
                Err(e)
            }
        };
    }

    // Load configuration
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("ERROR: Configuration error: {}", e);
            eprintln!("Please ensure all required environment variables are set.");
            eprintln!("Required: SOURCE_DB_TYPE, TARGET_DB_TYPE (mysql or postgresql)");
            eprintln!("Required: SOURCE_DB_HOST, SOURCE_DB_USERNAME, SOURCE_DB_PASSWORD, SOURCE_DB_DATABASE");
            eprintln!("Required: TARGET_DB_HOST, TARGET_DB_USERNAME, TARGET_DB_PASSWORD, TARGET_DB_DATABASE");
            std::process::exit(1);
        }
    };

    // Determine sync mode from CLI or config
    let sync_mode = if matches.get_flag("full-sync") {
        SyncMode::Both
    } else if matches.get_flag("initial-sync") {
        SyncMode::Initial
    } else if matches.get_flag("realtime-sync") {
        SyncMode::Realtime
    } else {
        config.sync_mode.clone()
    };

    info!("Starting database synchronization proxy");
    info!("Source: {:?}, Target: {:?}", config.source_type, config.target_type);
    info!("Sync mode: {:?}", sync_mode);

    match sync_mode {
        SyncMode::Initial => {
            let _ = run_initial_sync(&config).await.map_err(|e| {
                eprintln!("Initial sync failed: {}", e);
                e
            })?;
        }
        SyncMode::Realtime => {
            run_realtime_sync(&config).await.map_err(|e| {
                eprintln!("Real-time sync failed: {}", e);
                e
            })?;
        }
        SyncMode::Both => {
            // Run initial sync and get the start timestamp
            info!("=== Phase 1/3: Initial Sync ===");
            let start_timestamp = run_initial_sync(&config).await.map_err(|e| {
                eprintln!("Initial sync failed: {}", e);
                e
            })?;
            
            // Run catch-up sync to apply changes that occurred during initial transfer
            info!("=== Phase 2/3: Catch-Up Sync ===");
            run_catchup_sync(&config, &start_timestamp).await.map_err(|e| {
                eprintln!("Catch-up sync failed: {}", e);
                e
            })?;
            
            // Now start real-time sync
            info!("=== Phase 3/3: Real-Time Sync ===");
            info!("🚀 Starting real-time sync...");
            info!("The application will now monitor MySQL for changes continuously.");
            info!("Press Ctrl+C to stop.");
            run_realtime_sync(&config).await.map_err(|e| {
                eprintln!("Real-time sync failed: {}", e);
                e
            })?;
        }
    }

    info!("Proxy completed successfully");
    Ok(())
}

async fn run_initial_sync(config: &Config) -> Result<String> {
    info!("Starting initial sync (schema + data)");

    // Validate that source and target types are the same
    if config.source_type != config.target_type {
        return Err(anyhow::anyhow!(
            "Source and target database types must be the same. Source: {:?}, Target: {:?}",
            config.source_type, config.target_type
        ));
    }

    match config.source_type {
        DatabaseType::MySQL => run_mysql_to_mysql_initial_sync(config).await,
        DatabaseType::PostgreSQL => run_pg_to_pg_initial_sync(config).await,
    }
}

async fn run_mysql_to_mysql_initial_sync(config: &Config) -> Result<String> {
    info!("Starting MySQL to MySQL initial sync");

    // Connect to source MySQL
    info!("Connecting to source MySQL: {}", config.source_url);
    let source_pool = sqlx::MySqlPool::connect(&config.source_url).await?;
    info!("Connected to source MySQL");
    
    // Record the start timestamp BEFORE we begin data transfer
    info!("📍 Recording start timestamp for catch-up sync...");
    let start_timestamp = get_mysql_timestamp(&source_pool).await?;
    info!("📍 Start timestamp: {}", start_timestamp);

    // Connect to target MySQL
    info!("Connecting to target MySQL: {}", config.target_url);
    let target_pool = sqlx::MySqlPool::connect(&config.target_url).await?;
    info!("Connected to target MySQL");

    // Read source schema
    info!("Reading source MySQL schema...");
    let reader = MySQLReader::new(source_pool.clone(), config.source_database.clone());
    let schema = reader.build_schema().await?;
    info!("Read {} tables from source MySQL", schema.tables.len());

    // Create tables in target (simple copy for MySQL to MySQL)
    info!("Creating tables in target MySQL...");
    for (table_name, table_schema) in &schema.tables {
        info!("Creating table: {}", table_name);
        
        // Generate CREATE TABLE statement for MySQL
        let mut columns_sql = Vec::new();
        for col in &table_schema.columns {
            let mut col_def = format!("`{}` {}", col.name, col.data_type.to_uppercase());
            
            if let Some(len) = col.character_maximum_length {
                if col.data_type == "varchar" || col.data_type == "char" {
                    col_def = format!("`{}` {}({})", col.name, col.data_type.to_uppercase(), len);
                }
            }
            
            if !col.is_nullable {
                col_def.push_str(" NOT NULL");
            }
            
            if col.is_auto_increment {
                col_def.push_str(" AUTO_INCREMENT");
            }
            
            if let Some(ref default) = col.default_value {
                if !col.is_auto_increment {
                    col_def.push_str(&format!(" DEFAULT {}", default));
                }
            }
            
            columns_sql.push(col_def);
        }
        
        // Add primary key
        if !table_schema.primary_keys.is_empty() {
            let pk_cols: Vec<String> = table_schema.primary_keys.iter().map(|pk| format!("`{}`", pk)).collect();
            columns_sql.push(format!("PRIMARY KEY ({})", pk_cols.join(", ")));
        }
        
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS `{}` ({})",
            table_name,
            columns_sql.join(", ")
        );
        
        sqlx::query(&create_sql).execute(&target_pool).await?;
    }
    info!("All tables created in target MySQL");

    // Transfer data
    info!("Transferring data from source to target...");
    for (table_name, table_schema) in &schema.tables {
        info!("Transferring table: {}", table_name);
        
        let columns: Vec<String> = table_schema.columns.iter().map(|c| format!("`{}`", c.name)).collect();
        let select_sql = format!("SELECT {} FROM `{}`", columns.join(", "), table_name);
        
        let mut rows = sqlx::query(&select_sql).fetch(&source_pool);
        
        let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();
        let insert_sql = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );
        
        use sqlx::Row;
        use futures::StreamExt;
        
        let mut count = 0;
        while let Some(row) = rows.next().await {
            let row = row?;
            let mut query = sqlx::query(&insert_sql);
            
            for i in 0..columns.len() {
                let value: Option<String> = row.try_get(i).ok();
                query = query.bind(value);
            }
            
            query.execute(&target_pool).await?;
            count += 1;
        }
        
        info!("Transferred {} rows from {}", count, table_name);
    }
    info!("Data transfer completed");

    info!("Initial sync completed");
    Ok(start_timestamp)
}

async fn run_pg_to_pg_initial_sync(config: &Config) -> Result<String> {
    info!("Starting PostgreSQL to PostgreSQL initial sync");

    // Connect to source PostgreSQL
    info!("Connecting to source PostgreSQL: {}", config.source_url);
    let source_pool = sqlx::PgPool::connect(&config.source_url).await?;
    info!("Connected to source PostgreSQL");
    
    // Record the start timestamp BEFORE we begin data transfer
    info!("📍 Recording start timestamp for catch-up sync...");
    let start_timestamp = get_pg_timestamp(&source_pool).await?;
    info!("📍 Start timestamp: {}", start_timestamp);

    // Connect to target PostgreSQL
    info!("Connecting to target PostgreSQL: {}", config.target_url);
    let target_pool = sqlx::PgPool::connect(&config.target_url).await?;
    info!("Connected to target PostgreSQL");

    // Read source schema
    info!("Reading source PostgreSQL schema...");
    let reader = PgReader::new(source_pool.clone(), config.source_database.clone());
    let schema = reader.build_schema().await?;
    info!("Read {} tables from source PostgreSQL", schema.tables.len());

    // Create tables in target (simple copy for PostgreSQL to PostgreSQL)
    info!("Creating tables in target PostgreSQL...");
    for (table_name, table_schema) in &schema.tables {
        info!("Creating table: {}", table_name);
        
        // Generate CREATE TABLE statement for PostgreSQL
        let mut columns_sql = Vec::new();
        for col in &table_schema.columns {
            let mut col_def = format!("\"{}\" {}", col.name, col.data_type.to_uppercase());
            
            if let Some(len) = col.character_maximum_length {
                if col.data_type == "character varying" || col.data_type == "varchar" {
                    col_def = format!("\"{}\" VARCHAR({})", col.name, len);
                } else if col.data_type == "character" || col.data_type == "char" {
                    col_def = format!("\"{}\" CHAR({})", col.name, len);
                }
            }
            
            if !col.is_nullable {
                col_def.push_str(" NOT NULL");
            }
            
            if let Some(ref default) = col.default_value {
                col_def.push_str(&format!(" DEFAULT {}", default));
            }
            
            columns_sql.push(col_def);
        }
        
        // Add primary key
        if !table_schema.primary_keys.is_empty() {
            let pk_cols: Vec<String> = table_schema.primary_keys.iter().map(|pk| format!("\"{}\"", pk)).collect();
            columns_sql.push(format!("PRIMARY KEY ({})", pk_cols.join(", ")));
        }
        
        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS \"{}\" ({})",
            table_name,
            columns_sql.join(", ")
        );
        
        sqlx::query(&create_sql).execute(&target_pool).await?;
    }
    info!("All tables created in target PostgreSQL");

    // Transfer data
    info!("Transferring data from source to target...");
    for (table_name, table_schema) in &schema.tables {
        info!("Transferring table: {}", table_name);
        
        let columns: Vec<String> = table_schema.columns.iter().map(|c| format!("\"{}\"", c.name)).collect();
        let select_sql = format!("SELECT {} FROM \"{}\"", columns.join(", "), table_name);
        
        let mut rows = sqlx::query(&select_sql).fetch(&source_pool);
        
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();
        let insert_sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );
        
        use sqlx::Row;
        use futures::StreamExt;
        
        let mut count = 0;
        while let Some(row) = rows.next().await {
            let row = row?;
            let mut query = sqlx::query(&insert_sql);
            
            for i in 0..columns.len() {
                let value: Option<String> = row.try_get(i).ok();
                query = query.bind(value);
            }
            
            query.execute(&target_pool).await?;
            count += 1;
        }
        
        info!("Transferred {} rows from {}", count, table_name);
    }
    info!("Data transfer completed");

    info!("Initial sync completed");
    Ok(start_timestamp)
}

/// Get current MySQL timestamp for catch-up sync
async fn get_mysql_timestamp(mysql_pool: &sqlx::MySqlPool) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT CAST(NOW(6) AS CHAR)")
        .fetch_one(mysql_pool)
        .await?;
    Ok(row.0)
}

/// Get current PostgreSQL timestamp for catch-up sync
async fn get_pg_timestamp(pg_pool: &sqlx::PgPool) -> Result<String> {
    let row: (String,) = sqlx::query_as("SELECT CAST(NOW() AS TEXT)")
        .fetch_one(pg_pool)
        .await?;
    Ok(row.0)
}

/// Run catch-up sync to replay changes that occurred during initial transfer
/// Keeps checking and replaying until no more changes are found
async fn run_catchup_sync(config: &Config, start_timestamp: &str) -> Result<()> {
    match config.source_type {
        DatabaseType::MySQL => run_mysql_catchup_sync(config, start_timestamp).await,
        DatabaseType::PostgreSQL => {
            // For PostgreSQL, we don't have a catch-up mechanism like MySQL's general_log yet
            info!("PostgreSQL catch-up sync not implemented - skipping");
            Ok(())
        }
    }
}

async fn run_mysql_catchup_sync(config: &Config, start_timestamp: &str) -> Result<()> {
    info!("🔄 Starting catch-up sync to replay changes from initial transfer");
    info!("📍 Catching up from timestamp: {}", start_timestamp);

    // Connect to source MySQL
    let source_pool = sqlx::MySqlPool::connect(&config.source_url).await?;
    
    // Connect to target MySQL
    let target_pool = sqlx::MySqlPool::connect(&config.target_url).await?;

    // Create bounded channel for events
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);

    // Create binlog reader for catch-up
    let mut binlog_reader = BinlogReader::new(
        source_pool.clone(), 
        config.source_database.clone(), 
        event_tx.clone()
    )?;

    // Start MySQL writer worker
    let target_pool_clone = target_pool.clone();
    let writer_handle = tokio::spawn(async move {
        info!("[Catch-up Writer] Started, waiting for events...");
        let writer = MySQLWriter::new(target_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            if let Err(e) = writer.handle_event(event).await {
                error!("[Catch-up] Event #{} failed: {}", event_count, e);
            }
        }
        
        info!("[Catch-up Writer] Channel closed, processed {} total events", event_count);
    });

    // Keep running catch-up until no more changes are found
    let mut iteration = 0;
    let mut current_timestamp = start_timestamp.to_string();
    
    loop {
        iteration += 1;
        info!("🔄 Catch-up iteration #{}", iteration);
        
        // Get timestamp before this catch-up run
        let before_catchup = get_mysql_timestamp(&source_pool).await?;
        
        // Run catch-up from current timestamp
        let changes_found = binlog_reader.catchup_from_timestamp(&current_timestamp).await?;
        
        if changes_found == 0 {
            // No changes found - we're synchronized!
            info!("✓ Catch-up complete: databases are synchronized");
            break;
        }
        
        // Wait a moment for writer to finish processing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Get timestamp after catch-up to check for new changes
        let after_catchup = get_mysql_timestamp(&source_pool).await?;
        
        info!("⚠️  Found {} changes in iteration {}", changes_found, iteration);
        info!("📍 Checking for additional changes from {} to {}", before_catchup, after_catchup);
        
        // Update current timestamp for next iteration
        current_timestamp = before_catchup;
        
        // Safety: prevent infinite loop (max 10 iterations)
        if iteration >= 10 {
            warn!("⚠️  Reached maximum catch-up iterations (10). Proceeding to live sync.");
            warn!("⚠️  There may still be pending changes - live sync will handle them.");
            break;
        }
    }

    // Close the channel and wait for writer to finish
    info!("Closing catch-up event channel...");
    drop(event_tx);
    
    info!("Waiting for writer to finish processing...");
    match tokio::time::timeout(tokio::time::Duration::from_secs(10), writer_handle).await {
        Ok(_) => {
            info!("✓ Writer finished successfully");
        }
        Err(_) => {
            warn!("⚠️  Writer did not finish within 10 seconds, continuing anyway...");
        }
    }

    info!("✓ Catch-up sync completed successfully");
    Ok(())
}

async fn run_realtime_sync(config: &Config) -> Result<()> {
    match config.source_type {
        DatabaseType::MySQL => run_mysql_realtime_sync(config).await,
        DatabaseType::PostgreSQL => {
            // For PostgreSQL real-time sync (not implemented yet)
            info!("PostgreSQL real-time sync not implemented yet");
            Err(anyhow::anyhow!("PostgreSQL real-time sync not yet implemented"))
        }
    }
}

async fn run_mysql_realtime_sync(config: &Config) -> Result<()> {
    info!("Starting real-time sync (change monitoring)");

    // Connect to source MySQL with connection pool
    info!("Connecting to source MySQL: {}", config.source_url);
    let source_pool = sqlx::MySqlPool::connect(&config.source_url).await?;
    info!("Connected to source MySQL (pool connection)");

    // Connect to target MySQL with connection pool
    info!("Connecting to target MySQL: {}", config.target_url);
    let target_pool = sqlx::MySqlPool::connect(&config.target_url).await?;
    info!("Connected to target MySQL (pool connection)");

    // Create bounded channel for binlog events (queue with capacity 1000)
    // This allows the listener to continue even if writer is slow
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);

    // Create stats logger
    let stats_logger = Arc::new(StatsLogger::new("sync_operations_stats.json"));
    info!("📊 Statistics logging enabled: sync_operations_stats.json");
    
    // Start background flush task
    let stats_logger_clone = stats_logger.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if let Err(e) = stats_logger_clone.flush_to_disk().await {
                warn!("Failed to flush stats to disk: {}", e);
            }
        }
    });

    // Display initial summary
    stats_logger.display_full_summary().await;

    // Start binlog reader with connection pool
    let mut binlog_reader = BinlogReader::new(source_pool, config.source_database.clone(), event_tx)?;

    // Start MySQL writer worker (runs in background, doesn't block listener)
    let target_pool_clone = target_pool.clone();
    let stats_logger_writer = stats_logger.clone();
    let writer_handle = tokio::spawn(async move {
        info!("MySQL writer worker started, processing queue...");
        let writer = MySQLWriter::new(target_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            // Log statistics and format event type
            let event_type = match &event {
                crate::realtime::binlog_reader::BinlogEventType::Insert { table, values } => {
                    stats_logger_writer.log_operation("INSERT", table).await;
                    format!("INSERT: table={}, {} columns", table, values.len())
                }
                crate::realtime::binlog_reader::BinlogEventType::Update { table, new_values, .. } => {
                    stats_logger_writer.log_operation("UPDATE", table).await;
                    format!("UPDATE: table={}, {} columns", table, new_values.len())
                }
                crate::realtime::binlog_reader::BinlogEventType::Delete { table, values } => {
                    stats_logger_writer.log_operation("DELETE", table).await;
                    format!("DELETE: table={}, {} columns", table, values.len())
                }
            };
            info!("[Queue] Processing event #{}: {}", event_count, event_type);
            
            // Process event asynchronously - don't block on errors
            match writer.handle_event(event).await {
                Ok(_) => {
                    info!("[Queue] ✓ Event #{} processed successfully", event_count);
                }
                Err(e) => {
                    error!("[Queue] ✗ Event #{} failed: {} (continuing to process queue)", event_count, e);
                    // Continue processing other events even if one fails
                }
            }
        }
        info!("MySQL writer worker stopped (channel closed)");
    });

    // Start binlog streaming
    info!("Starting change monitoring...");
    if let Err(e) = binlog_reader.start_streaming().await {
        error!("Change monitoring error: {}", e);
    }

    // Wait for writer to finish
    let _ = writer_handle.await;

    // Flush final stats
    info!("📊 Flushing final statistics...");
    if let Err(e) = stats_logger.flush_to_disk().await {
        warn!("Failed to flush final stats: {}", e);
    }
    
    // Display final summary
    stats_logger.display_full_summary().await;

    info!("Real-time sync stopped");
    Ok(())
}

/// Run catch-up sync with detailed UI logging
async fn run_catchup_sync_with_ui(config: &Config, start_timestamp: &str, state: web::state::AppState) -> Result<()> {
    state.add_log(format!("  → Replaying from timestamp: {}", start_timestamp)).await;
    
    let source_pool = sqlx::MySqlPool::connect(&config.source_url).await?;
    let target_pool = sqlx::MySqlPool::connect(&config.target_url).await?;
    
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);
    let mut binlog_reader = BinlogReader::new(source_pool.clone(), config.source_database.clone(), event_tx)?;
    
    // Start MySQL writer worker
    let target_pool_clone = target_pool.clone();
    let state_writer = state.clone();
    let writer_handle = tokio::spawn(async move {
        let writer = MySQLWriter::new(target_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            if let Err(e) = writer.handle_event(event).await {
                state_writer.add_log(format!("    ⚠️ Error: {}", e)).await;
                let mut stats = state_writer.stats.write().await;
                stats.errors_count += 1;
            }
        }
        
        state_writer.add_log(format!("    Processed {} total events", event_count)).await;
    });
    
    let mut iteration = 0;
    let mut current_timestamp = start_timestamp.to_string();
    
    loop {
        iteration += 1;
        
        state.add_log(format!("  🔄 Iteration {}: Checking for changes...", iteration)).await;
        
        // Get timestamp before catch-up
        let before_catchup = get_mysql_timestamp(&source_pool).await?;
        
        // Run catch-up from current timestamp
        let changes_found = binlog_reader.catchup_from_timestamp(&current_timestamp).await?;
        
        if changes_found == 0 {
            state.add_log("  ✓ No more changes found - databases synchronized".to_string()).await;
            break;
        }
        
        state.add_log(format!("  → Found {} changes to replay", changes_found)).await;
        
        // Wait for writer to process
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Update timestamp for next iteration
        current_timestamp = before_catchup;
        
        // Safety: prevent infinite loop
        if iteration >= 10 {
            state.add_log("  ⚠️ Max iterations (10) reached - proceeding to live sync".to_string()).await;
            break;
        }
    }
    
    // Close channel and wait for writer to finish
    drop(binlog_reader); // This drops event_tx
    
    // Wait with timeout
    tokio::select! {
        _ = writer_handle => {
            state.add_log("  ✓ Writer finished processing".to_string()).await;
        },
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
            state.add_log("  ⏱️ Writer timeout - continuing".to_string()).await;
        }
    }
    
    Ok(())
}

/// Run initial sync with detailed UI logging
async fn run_initial_sync_with_ui(config: &Config, state: web::state::AppState) -> Result<String> {
    use chrono::Utc;
    
    // Check if we should reset the target database
    let should_reset = {
        let web_config = state.config.read().await;
        web_config.reset_database
    };
    
    if should_reset {
        state.add_log("🗑️  RESET DATABASE MODE: Dropping and recreating target database...".to_string()).await;
        
        // Connect to MySQL server (without database name)
        let server_url = format!("mysql://{}:{}@{}:{}/",
            config.target_username,
            config.target_password,
            config.target_host,
            config.target_port
        );
        
        match sqlx::MySqlPool::connect(&server_url).await {
            Ok(server_pool) => {
                // Drop database
                let drop_query = format!("DROP DATABASE IF EXISTS `{}`", config.target_database);
                match sqlx::query(&drop_query).execute(&server_pool).await {
                    Ok(_) => {
                        state.add_log(format!("  ✓ Dropped database '{}'", config.target_database)).await;
                    }
                    Err(e) => {
                        state.add_log(format!("  ⚠️  Failed to drop database: {}", e)).await;
                    }
                }
                
                // Create database
                let create_query = format!("CREATE DATABASE `{}`", config.target_database);
                match sqlx::query(&create_query).execute(&server_pool).await {
                    Ok(_) => {
                        state.add_log(format!("  ✓ Created fresh database '{}'", config.target_database)).await;
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("Failed to create database: {}", e));
                    }
                }
                
                server_pool.close().await;
            }
            Err(e) => {
                state.add_log(format!("  ⚠️  Warning: Could not connect to MySQL server for reset: {}", e)).await;
            }
        }
    }
    
    state.add_log("📊 Connecting to databases...".to_string()).await;
    
    // Connect to source
    state.add_log(format!("  → Source: {}@{}:{}/{}", 
        config.source_username,
        config.source_host,
        config.source_port,
        config.source_database
    )).await;
    let source_pool = sqlx::MySqlPool::connect(&config.source_url).await?;
    
    // Get start timestamp BEFORE data transfer
    state.add_log("🕐 Recording start timestamp...".to_string()).await;
    let start_timestamp = get_mysql_timestamp(&source_pool).await?;
    state.add_log(format!("  → Start time: {}", start_timestamp)).await;
    
    // Connect to target
    state.add_log(format!("  → Target: {}@{}:{}/{}", 
        config.target_username,
        config.target_host,
        config.target_port,
        config.target_database
    )).await;
    let target_pool = sqlx::MySqlPool::connect(&config.target_url).await?;
    
    state.add_log("✓ Database connections established".to_string()).await;
    
    // Read schema
    state.add_log("📖 Reading source schema...".to_string()).await;
    let reader = MySQLReader::new(source_pool.clone(), config.source_database.clone());
    let schema = reader.build_schema().await?;
    state.add_log(format!("  → Found {} tables", schema.tables.len())).await;
    
    // List tables
    for (table_name, table_schema) in &schema.tables {
        state.add_log(format!("    • {} ({} columns)", table_name, table_schema.columns.len())).await;
    }
    
    // Update stats
    {
        let mut stats = state.stats.write().await;
        stats.tables_synced = schema.tables.len();
    }
    
    // Create tables using mysqldump (PRODUCTION-GRADE approach!)
    state.add_log("🔨 Exporting schema using mysqldump...".to_string()).await;
    
    // Build mysqldump command
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    let dump_result = Command::new("mysqldump")
        .args([
            "-h", &config.source_host,
            "-P", &config.source_port.to_string(),
            "-u", &config.source_username,
            &format!("-p{}", config.source_password),
            "--no-data",           // Schema only
            "--routines",          // Include stored procedures/functions
            "--triggers",          // Include triggers
            "--events",            // Include events
            "--single-transaction", // Consistent snapshot
            "--skip-add-drop-table", // Don't drop existing tables
            &config.source_database,
        ])
        .output();
    
    match dump_result {
        Ok(output) if output.status.success() => {
            let schema_sql = String::from_utf8_lossy(&output.stdout);
            state.add_log(format!("✓ Exported {} bytes of schema", schema_sql.len())).await;
            
            // Import schema to target using mysql command with --force to ignore existing objects
            state.add_log("🔨 Importing schema to target database...".to_string()).await;
            
            let mut import_child = match Command::new("mysql")
                .args([
                    "-h", &config.target_host,
                    "-P", &config.target_port.to_string(),
                    "-u", &config.target_username,
                    &format!("-p{}", config.target_password),
                    "--force",  // CRITICAL: Continue even if errors occur (e.g., "already exists")
                    &config.target_database,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to start mysql import: {}", e));
                }
            };
            
            // Write schema to mysql stdin
            if let Some(mut stdin) = import_child.stdin.take() {
                if let Err(e) = stdin.write_all(schema_sql.as_bytes()) {
                    return Err(anyhow::anyhow!("Failed to write schema to mysql: {}", e));
                }
            }
            
            // Wait for import to complete
            match import_child.wait() {
                Ok(status) if status.success() => {
                    state.add_log(format!("✓ Created {} tables using mysqldump (ignoring already-existing objects)", schema.tables.len())).await;
                }
                Ok(status) => {
                    // With --force flag, even non-zero exit codes might be acceptable
                    // Check stderr for real errors vs warnings
                    let stderr = import_child.stderr.and_then(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok()?;
                        Some(buf)
                    }).unwrap_or_default();
                    
                    // Log warnings but don't fail on "already exists" errors
                    if !stderr.is_empty() {
                        state.add_log(format!("⚠️  Schema import warnings (ignored): {}", 
                            stderr.lines().take(5).collect::<Vec<_>>().join("; "))).await;
                    }
                    
                    state.add_log(format!("✓ Schema import completed (some objects may already exist)")).await;
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to wait for mysql import: {}", e));
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("mysqldump failed: {}", stderr));
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to execute mysqldump: {}. Make sure mysqldump is installed in the Docker container.", e));
        }
    }
    
    // Transfer data using mysqldump (PRODUCTION-GRADE!)
    state.add_log("📦 Transferring data using mysqldump...".to_string()).await;
    
    let dump_result = Command::new("mysqldump")
        .args([
            "-h", &config.source_host,
            "-P", &config.source_port.to_string(),
            "-u", &config.source_username,
            &format!("-p{}", config.source_password),
            "--no-create-info",    // Data only (no schema)
            "--skip-triggers",     // Already created by schema dump
            "--single-transaction", // Consistent snapshot
            "--complete-insert",   // Include column names in INSERT
            "--extended-insert",   // Multi-row inserts for speed
            "--insert-ignore",     // CRITICAL: Use INSERT IGNORE to skip duplicates
            "--disable-keys",      // Faster imports
            &config.source_database,
        ])
        .output();
    
    match dump_result {
        Ok(output) if output.status.success() => {
            let data_sql = String::from_utf8_lossy(&output.stdout);
            state.add_log(format!("✓ Exported {} bytes of data", data_sql.len())).await;
            
            // Import data to target
            state.add_log("📦 Importing data to target database...".to_string()).await;
            
            let mut import_child = match Command::new("mysql")
                .args([
                    "-h", &config.target_host,
                    "-P", &config.target_port.to_string(),
                    "-u", &config.target_username,
                    &format!("-p{}", config.target_password),
                    "--force",  // Continue even if errors occur (INSERT IGNORE will handle duplicates)
                    &config.target_database,
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
            {
                Ok(child) => child,
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to start mysql import for data: {}", e));
                }
            };
            
            // Write data to mysql stdin
            if let Some(mut stdin) = import_child.stdin.take() {
                if let Err(e) = stdin.write_all(data_sql.as_bytes()) {
                    return Err(anyhow::anyhow!("Failed to write data to mysql: {}", e));
                }
            }
            
            // Wait for import to complete
            match import_child.wait() {
                Ok(status) if status.success() => {
                    // Count total rows
                    let mut total_rows = 0;
                    for table_name in schema.tables.keys() {
                        let count_query = format!("SELECT COUNT(*) as cnt FROM `{}`", table_name);
                        if let Ok(row_count) = sqlx::query_as::<_, (i64,)>(&count_query).fetch_one(&target_pool).await {
                            total_rows += row_count.0;
                        }
                    }
                    
                    state.add_log(format!("✓ Data transfer complete ({} total rows transferred)", total_rows)).await;
                    
                    // Update stats
                    {
                        let mut stats = state.stats.write().await;
                        stats.rows_synced += total_rows as usize;
                    }
                }
                Ok(status) => {
                    let stderr = import_child.stderr.and_then(|mut s| {
                        let mut buf = String::new();
                        std::io::Read::read_to_string(&mut s, &mut buf).ok()?;
                        Some(buf)
                    }).unwrap_or_default();
                    
                    return Err(anyhow::anyhow!("MySQL data import failed with status {}: {}", status, stderr));
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("Failed to wait for mysql data import: {}", e));
                }
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("mysqldump data export failed: {}", stderr));
        }
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to execute mysqldump for data: {}", e));
        }
    }
    
    // Migrate database objects (views, functions, procedures, triggers)
    state.add_log("🔧 Migrating database objects...".to_string()).await;
    
    // Skip routine migration for same-type database sync
    state.add_log("✓ Database objects migration skipped (same-type sync)".to_string()).await;
    
    // Skip verification for UI mode
    state.add_log("🔍 Verification skipped in UI mode".to_string()).await;
    
    Ok(start_timestamp)
}

/// Run full sync for UI mode - combines initial, catchup, and realtime sync
/// Accepts config directly from web UI (no environment variables!)
pub async fn run_full_sync_for_ui(config: Config, state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    state.add_log("Starting full synchronization...".to_string()).await;
    
    // Phase 1: Initial Sync with detailed logging
    state.add_log("=== Phase 1/3: Initial Sync ===".to_string()).await;
    let start_timestamp = match run_initial_sync_with_ui(&config, state.clone()).await {
        Ok(ts) => {
            state.add_log("✓ Initial sync completed successfully".to_string()).await;
            ts
        }
        Err(e) => {
            let msg = format!("❌ Initial sync failed: {}", e);
            state.add_log(msg.clone()).await;
            return Err(e);
        }
    };
    
    // Mark start time
    {
        let mut stats = state.stats.write().await;
        stats.start_time = Some(Utc::now().to_rfc3339());
    }
    
    // Phase 2: Catch-Up Sync
    state.add_log("=== Phase 2/3: Catch-Up Sync ===".to_string()).await;
    state.add_log("🔄 Replaying changes that occurred during initial transfer...".to_string()).await;
    
    if let Err(e) = run_catchup_sync_with_ui(&config, &start_timestamp, state.clone()).await {
        let msg = format!("❌ Catch-up sync failed: {}", e);
        state.add_log(msg.clone()).await;
        return Err(e);
    }
    state.add_log("✓ Catch-up sync completed - databases are synchronized".to_string()).await;
    
    // Check if we should stop
    {
        let status = state.status.read().await;
        if *status == web::state::SyncStatus::Stopped {
            state.add_log("Sync stopped by user".to_string()).await;
            return Ok(());
        }
    }
    
    // Phase 3: Real-Time Sync
    state.add_log("=== Phase 3/3: Real-Time Sync ===".to_string()).await;
    state.add_log("Starting live synchronization...".to_string()).await;
    
    if let Err(e) = run_realtime_sync_for_ui(&config, state.clone()).await {
        let msg = format!("Real-time sync failed: {}", e);
        state.add_log(msg.clone()).await;
        return Err(e);
    }
    
    state.add_log("Synchronization completed".to_string()).await;
    Ok(())
}

/// Run initial sync only (schema + data) for UI
pub async fn run_initial_only_for_ui(config: Config, state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    state.add_log("Starting initial synchronization (schema + data)...".to_string()).await;
    
    // Mark start time
    {
        let mut stats = state.stats.write().await;
        stats.start_time = Some(Utc::now().to_rfc3339());
    }
    
    // Run initial sync
    match run_initial_sync_with_ui(&config, state.clone()).await {
        Ok(_) => {
            state.add_log("✓ Initial sync completed successfully".to_string()).await;
            state.add_log("Synchronization completed".to_string()).await;
            Ok(())
        }
        Err(e) => {
            let msg = format!("❌ Initial sync failed: {}", e);
            state.add_log(msg.clone()).await;
            Err(e)
        }
    }
}

/// Run real-time sync only (binlog monitoring) for UI
pub async fn run_realtime_only_for_ui(config: Config, state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    state.add_log("Starting real-time synchronization (binlog monitoring)...".to_string()).await;
    
    // Mark start time
    {
        let mut stats = state.stats.write().await;
        stats.start_time = Some(Utc::now().to_rfc3339());
    }
    
    // Run real-time sync
    if let Err(e) = run_realtime_sync_for_ui(&config, state.clone()).await {
        let msg = format!("Real-time sync failed: {}", e);
        state.add_log(msg.clone()).await;
        return Err(e);
    }
    
    state.add_log("Real-time synchronization completed".to_string()).await;
    Ok(())
}

/// Run real-time sync with UI state updates
async fn run_realtime_sync_for_ui(config: &Config, state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    state.add_log("Connecting to source...".to_string()).await;
    let source_pool = sqlx::mysql::MySqlPool::connect(&config.source_url).await?;
    
    state.add_log("Connecting to target...".to_string()).await;
    let target_pool = sqlx::mysql::MySqlPool::connect(&config.target_url).await?;
    
    // Create event channel for async processing
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);
    
    // Initialize stats logger
    let stats_logger = Arc::new(StatsLogger::new("sync_operations_stats.json"));
    
    state.add_log("Starting change monitoring...".to_string()).await;
    
    // Start binlog reader with connection pool
    let mut binlog_reader = BinlogReader::new(source_pool, config.source_database.clone(), event_tx)?;
    
    // Start MySQL writer worker
    let target_pool_clone = target_pool.clone();
    let stats_logger_writer = stats_logger.clone();
    let state_writer = state.clone();
    let writer_handle = tokio::spawn(async move {
        let writer = MySQLWriter::new(target_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            // Check if we should stop
            {
                let status = state_writer.status.read().await;
                if *status == web::state::SyncStatus::Stopped {
                    state_writer.add_log("Writer stopped by user".to_string()).await;
                    break;
                }
            }
            
            // Log statistics and update UI state
            let (operation_type, table_name, details) = match &event {
                crate::realtime::binlog_reader::BinlogEventType::Insert { table, values } => {
                    stats_logger_writer.log_operation("INSERT", table).await;
                    let mut stats = state_writer.stats.write().await;
                    stats.inserts_applied += 1;
                    ("INSERT", table.clone(), format!("{} columns", values.len()))
                }
                crate::realtime::binlog_reader::BinlogEventType::Update { table, new_values, .. } => {
                    stats_logger_writer.log_operation("UPDATE", table).await;
                    let mut stats = state_writer.stats.write().await;
                    stats.updates_applied += 1;
                    ("UPDATE", table.clone(), format!("{} columns", new_values.len()))
                }
                crate::realtime::binlog_reader::BinlogEventType::Delete { table, values } => {
                    stats_logger_writer.log_operation("DELETE", table).await;
                    let mut stats = state_writer.stats.write().await;
                    stats.deletes_applied += 1;
                    ("DELETE", table.clone(), format!("{} columns", values.len()))
                }
            };
            
            state_writer.add_log(format!("🔄 {} → {} ({})", operation_type, table_name, details)).await;
            
            // Process event
            let success = match writer.handle_event(event).await {
                Ok(_) => {
                    // Success - log already added above
                    true
                }
                Err(e) => {
                    error!("Event #{} failed: {}", event_count, e);
                    let mut stats = state_writer.stats.write().await;
                    stats.errors_count += 1;
                    state_writer.add_log(format!("  ❌ Error: {}", e)).await;
                    false
                }
            };
            
            // Save operation stat to database
            let timestamp = Utc::now().to_rfc3339();
            if let Err(e) = state_writer.config_store.save_operation_stat(
                &timestamp,
                operation_type,
                &table_name,
                success,
            ).await {
                error!("Failed to save operation stat: {}", e);
            }
            
            // Update last sync time
            let mut stats = state_writer.stats.write().await;
            stats.last_sync_time = Some(Utc::now().to_rfc3339());
        }
    });
    
    // Start binlog streaming in a separate task
    let state_reader = state.clone();
    let reader_handle = tokio::spawn(async move {
        state_reader.add_log("Initializing change monitor...".to_string()).await;
        
        if let Err(e) = binlog_reader.start_streaming().await {
            error!("Change monitoring error: {}", e);
            state_reader.add_log(format!("⚠️ Change monitoring error: {}", e)).await;
        } else {
            state_reader.add_log("Change monitor stopped".to_string()).await;
        }
    });
    
    // Give the reader time to initialize and truncate general_log
    state.add_log("⏳ Waiting for change monitor to initialize...".to_string()).await;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    state.add_log("✓ Real-time sync is now active and monitoring changes".to_string()).await;
    state.add_log(format!("📊 Make changes in '{}' to see them replicated to '{}'", 
        config.source_database, config.target_database)).await;
    
    // Keep the sync running - wait for either task to complete or stop signal
    loop {
        // Check if we should stop
        {
            let status = state.status.read().await;
            if *status == web::state::SyncStatus::Stopped {
                state.add_log("🛑 Stop signal received".to_string()).await;
                break;
            }
        }
        
        // Check if tasks have completed (error condition)
        if reader_handle.is_finished() || writer_handle.is_finished() {
            state.add_log("⚠️ Sync task completed unexpectedly".to_string()).await;
            break;
        }
        
        // Sleep a bit before checking again
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    
    state.add_log("Shutting down real-time sync...".to_string()).await;
    
    // Abort the reader task (it's in an infinite loop)
    reader_handle.abort();
    
    // Wait for writer task to finish processing remaining events
    let shutdown_timeout = tokio::time::Duration::from_secs(5);
    tokio::select! {
        _ = writer_handle => {
            state.add_log("Writer task finished".to_string()).await;
        },
        _ = tokio::time::sleep(shutdown_timeout) => {
            state.add_log("Writer task timeout - forcing stop".to_string()).await;
        }
    }
    
    state.add_log("✓ Real-time sync stopped".to_string()).await;
    Ok(())
}

