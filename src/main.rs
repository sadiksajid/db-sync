mod ai;
mod config;
mod migrator;
mod realtime;
mod schema;
mod web;

use anyhow::Result;
use clap::{Arg, Command};
use config::{Config, SyncMode};
use migrator::{create_tables::TableCreator, data_transfer::DataTransfer, routine_migrator::RoutineMigrator, verify::Verifier};
use realtime::{binlog_reader::BinlogReader, pg_writer::PGWriter, stats_logger::StatsLogger};
use schema::{mysql_reader::MySQLReader, routines::RoutineReader};
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
    let matches = Command::new("mysql_psql_proxy")
        .version("0.1.0")
        .about("MySQL to PostgreSQL synchronization proxy")
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
            eprintln!("Required for MySQL: DB_HOST, DB_USERNAME, DB_PASSWORD, DB_DATABASE (or MYSQL_* variables)");
            eprintln!("Required for PostgreSQL: PSQL_DB_HOST, PSQL_DB_USERNAME, PSQL_DB_PASSWORD, PSQL_DB_DATABASE (or POSTGRES_* variables)");
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

    info!("Starting MySQL to PostgreSQL proxy");
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

    // Connect to MySQL
    info!("Connecting to MySQL: {}", config.mysql_url);
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    info!("Connected to MySQL");
    
    // Record the start timestamp BEFORE we begin data transfer
    info!("📍 Recording start timestamp for catch-up sync...");
    let start_timestamp = get_mysql_timestamp(&mysql_pool).await?;
    info!("📍 Start timestamp: {}", start_timestamp);

    // Connect to PostgreSQL
    info!("Connecting to PostgreSQL: {}", config.pg_url);
    let pg_pool = sqlx::PgPool::connect(&config.pg_url).await?;
    info!("Connected to PostgreSQL");

    // Read MySQL schema
    info!("Reading MySQL schema...");
    let reader = MySQLReader::new(mysql_pool.clone(), config.mysql_database.clone());
    let schema = reader.build_schema().await?;
    info!("Read {} tables from MySQL", schema.tables.len());

    // Create tables in PostgreSQL
    info!("Creating tables in PostgreSQL...");
    let table_creator = TableCreator::new(pg_pool.clone());
    table_creator.create_all_tables(&schema).await?;
    info!("All tables created in PostgreSQL");

    // Transfer data
    info!("Transferring data from MySQL to PostgreSQL...");
    let data_transfer = DataTransfer::new(mysql_pool.clone(), pg_pool.clone(), config.batch_size);
    data_transfer.transfer_all_data(&schema).await?;
    info!("Data transfer completed");

    // Read and migrate database objects (views, functions, procedures, triggers)
    info!("Reading database objects from MySQL...");
    let routine_reader = RoutineReader::new(mysql_pool.clone(), config.mysql_database.clone());
    
    let views = routine_reader.read_views().await?;
    let functions = routine_reader.read_functions().await?;
    let procedures = routine_reader.read_procedures().await?;
    let triggers = routine_reader.read_triggers().await?;

    info!("Found {} views, {} functions, {} procedures, {} triggers", 
        views.len(), functions.len(), procedures.len(), triggers.len());

    if !views.is_empty() || !functions.is_empty() || !procedures.is_empty() || !triggers.is_empty() {
        info!("Migrating database objects to PostgreSQL...");
        let routine_migrator = RoutineMigrator::new(pg_pool.clone());
        routine_migrator.migrate_all(&views, &functions, &procedures, &triggers).await?;
        info!("Database objects migration completed");
    } else {
        info!("No database objects to migrate");
    }

    // Verify
    info!("Verifying schema and data...");
    let verifier = Verifier::new(mysql_pool, pg_pool);
    let report = verifier.verify_schema(&schema).await?;

    info!(
        "Verification complete: {} tables match, {} tables mismatch",
        report.tables_match, report.tables_mismatch
    );

    for table_report in &report.table_reports {
        if !table_report.matches {
            error!(
                "Table {} mismatch: MySQL={}, PostgreSQL={}",
                table_report.table_name, table_report.mysql_count, table_report.pg_count
            );
        }
    }

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

/// Run catch-up sync to replay changes that occurred during initial transfer
/// Keeps checking and replaying until no more changes are found
async fn run_catchup_sync(config: &Config, start_timestamp: &str) -> Result<()> {
    info!("🔄 Starting catch-up sync to replay changes from initial transfer");
    info!("📍 Catching up from timestamp: {}", start_timestamp);

    // Connect to MySQL
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    
    // Connect to PostgreSQL
    let pg_pool = sqlx::PgPool::connect(&config.pg_url).await?;

    // Create bounded channel for events
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);

    // Create binlog reader for catch-up
    let mut binlog_reader = BinlogReader::new(
        mysql_pool.clone(), 
        config.mysql_database.clone(), 
        event_tx.clone()
    )?;

    // Start PostgreSQL writer worker
    let pg_pool_clone = pg_pool.clone();
    let writer_handle = tokio::spawn(async move {
        info!("[Catch-up Writer] Started, waiting for events...");
        let pg_writer = PGWriter::new(pg_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            if let Err(e) = pg_writer.handle_event(event).await {
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
        let before_catchup = get_mysql_timestamp(&mysql_pool).await?;
        
        // Run catch-up from current timestamp
        let changes_found = binlog_reader.catchup_from_timestamp(&current_timestamp).await?;
        
        if changes_found == 0 {
            // No changes found - we're synchronized!
            info!("✓ Catch-up complete: databases are synchronized");
            break;
        }
        
        // Wait a moment for PostgreSQL writer to finish processing
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Get timestamp after catch-up to check for new changes
        let after_catchup = get_mysql_timestamp(&mysql_pool).await?;
        
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
    
    info!("Waiting for PostgreSQL writer to finish processing...");
    match tokio::time::timeout(tokio::time::Duration::from_secs(10), writer_handle).await {
        Ok(_) => {
            info!("✓ PostgreSQL writer finished successfully");
        }
        Err(_) => {
            warn!("⚠️  PostgreSQL writer did not finish within 10 seconds, continuing anyway...");
        }
    }

    info!("✓ Catch-up sync completed successfully");
    Ok(())
}

async fn run_realtime_sync(config: &Config) -> Result<()> {
    info!("Starting real-time sync (change monitoring)");

    // Connect to MySQL with connection pool
    info!("Connecting to MySQL: {}", config.mysql_url);
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    info!("Connected to MySQL (pool connection)");

    // Connect to PostgreSQL with connection pool
    info!("Connecting to PostgreSQL: {}", config.pg_url);
    let pg_pool = sqlx::PgPool::connect(&config.pg_url).await?;
    info!("Connected to PostgreSQL (pool connection)");

    // Create bounded channel for binlog events (queue with capacity 1000)
    // This allows the listener to continue even if PostgreSQL writer is slow
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
    let mut binlog_reader = BinlogReader::new(mysql_pool, config.mysql_database.clone(), event_tx)?;

    // Start PostgreSQL writer worker (runs in background, doesn't block listener)
    let pg_pool_clone = pg_pool.clone();
    let stats_logger_writer = stats_logger.clone();
    let writer_handle = tokio::spawn(async move {
        info!("PostgreSQL writer worker started, processing queue...");
        let pg_writer = PGWriter::new(pg_pool_clone);
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
            match pg_writer.handle_event(event).await {
                Ok(_) => {
                    info!("[Queue] ✓ Event #{} processed successfully", event_count);
                }
                Err(e) => {
                    error!("[Queue] ✗ Event #{} failed: {} (continuing to process queue)", event_count, e);
                    // Continue processing other events even if one fails
                }
            }
        }
        info!("PostgreSQL writer worker stopped (channel closed)");
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
    
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    let pg_pool = sqlx::PgPool::connect(&config.pg_url).await?;
    
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);
    let mut binlog_reader = BinlogReader::new(mysql_pool.clone(), config.mysql_database.clone(), event_tx)?;
    
    // Start PostgreSQL writer worker
    let pg_pool_clone = pg_pool.clone();
    let state_writer = state.clone();
    let writer_handle = tokio::spawn(async move {
        let pg_writer = PGWriter::new(pg_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            
            if let Err(e) = pg_writer.handle_event(event).await {
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
        let before_catchup = get_mysql_timestamp(&mysql_pool).await?;
        
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
    
    state.add_log("📊 Connecting to databases...".to_string()).await;
    
    // Connect to MySQL
    state.add_log(format!("  → MySQL: {}@{}:{}/{}", 
        std::env::var("DB_USERNAME").unwrap_or_default(),
        std::env::var("DB_HOST").unwrap_or_default(),
        std::env::var("DB_PORT").unwrap_or_default(),
        std::env::var("DB_DATABASE").unwrap_or_default()
    )).await;
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    
    // Get start timestamp BEFORE data transfer
    state.add_log("🕐 Recording start timestamp...".to_string()).await;
    let start_timestamp = get_mysql_timestamp(&mysql_pool).await?;
    state.add_log(format!("  → Start time: {}", start_timestamp)).await;
    
    // Connect to PostgreSQL
    state.add_log(format!("  → PostgreSQL: {}@{}:{}/{}", 
        std::env::var("PSQL_DB_USERNAME").unwrap_or_default(),
        std::env::var("PSQL_DB_HOST").unwrap_or_default(),
        std::env::var("PSQL_DB_PORT").unwrap_or_default(),
        std::env::var("PSQL_DB_DATABASE").unwrap_or_default()
    )).await;
    let pg_pool = sqlx::PgPool::connect(&config.pg_url).await?;
    
    state.add_log("✓ Database connections established".to_string()).await;
    
    // Read schema
    state.add_log("📖 Reading MySQL schema...".to_string()).await;
    let reader = MySQLReader::new(mysql_pool.clone(), config.mysql_database.clone());
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
    
    // Create tables in PostgreSQL
    state.add_log("🔨 Creating tables in PostgreSQL...".to_string()).await;
    let table_creator = TableCreator::new(pg_pool.clone());
    table_creator.create_all_tables(&schema).await?;
    state.add_log(format!("✓ Created {} tables", schema.tables.len())).await;
    
    // Transfer data
    state.add_log("📦 Transferring data...".to_string()).await;
    let data_transfer = DataTransfer::new(mysql_pool.clone(), pg_pool.clone(), config.batch_size);
    data_transfer.transfer_all_data(&schema).await?;
    state.add_log("✓ Data transfer complete".to_string()).await;
    
    // Migrate database objects (views, functions, procedures, triggers)
    state.add_log("🔧 Migrating database objects...".to_string()).await;
    
    let routine_reader = RoutineReader::new(mysql_pool.clone(), config.mysql_database.clone());
    let routine_migrator = RoutineMigrator::new(pg_pool.clone());
    
    // Views
    state.add_log("  → Reading views...".to_string()).await;
    let views = routine_reader.read_views().await?;
    state.add_log(format!("    Found {} views", views.len())).await;
    for view in &views {
        state.add_log(format!("    • {}", view.name)).await;
    }
    routine_migrator.migrate_views(&views).await?;
    state.add_log(format!("    ✓ Migrated {} views", views.len())).await;
    {
        let mut stats = state.stats.write().await;
        stats.views_synced = views.len();
    }
    
    // Functions
    state.add_log("  → Reading functions...".to_string()).await;
    let functions = routine_reader.read_functions().await?;
    state.add_log(format!("    Found {} functions", functions.len())).await;
    for func in &functions {
        state.add_log(format!("    • {}", func.name)).await;
    }
    routine_migrator.migrate_functions(&functions).await?;
    state.add_log(format!("    ✓ Migrated {} functions", functions.len())).await;
    {
        let mut stats = state.stats.write().await;
        stats.functions_synced = functions.len();
    }
    
    // Procedures
    state.add_log("  → Reading procedures...".to_string()).await;
    let procedures = routine_reader.read_procedures().await?;
    state.add_log(format!("    Found {} procedures", procedures.len())).await;
    for proc in &procedures {
        state.add_log(format!("    • {}", proc.name)).await;
    }
    routine_migrator.migrate_procedures(&procedures).await?;
    state.add_log(format!("    ✓ Migrated {} procedures", procedures.len())).await;
    {
        let mut stats = state.stats.write().await;
        stats.procedures_synced = procedures.len();
    }
    
    // Triggers
    state.add_log("  → Reading triggers...".to_string()).await;
    let triggers = routine_reader.read_triggers().await?;
    state.add_log(format!("    Found {} triggers", triggers.len())).await;
    for trigger in &triggers {
        state.add_log(format!("    • {}", trigger.name)).await;
    }
    routine_migrator.migrate_triggers(&triggers).await?;
    state.add_log(format!("    ✓ Migrated {} triggers", triggers.len())).await;
    {
        let mut stats = state.stats.write().await;
        stats.triggers_synced = triggers.len();
    }
    
    state.add_log("✓ Database objects migration complete".to_string()).await;
    
    // Verify data
    state.add_log("🔍 Verifying data integrity...".to_string()).await;
    let verifier = Verifier::new(mysql_pool.clone(), pg_pool.clone());
    match verifier.verify_schema(&schema).await {
        Ok(report) => {
            state.add_log(format!("✓ Verified {} tables successfully", report.tables_match)).await;
            if report.tables_mismatch > 0 {
                state.add_log(format!("  ⚠️ Found {} table(s) with mismatches", report.tables_mismatch)).await;
            }
        }
        Err(e) => {
            state.add_log(format!("  ⚠️ Verification error: {}", e)).await;
        }
    }
    
    Ok(start_timestamp)
}

/// Run full sync for UI mode - combines initial, catchup, and realtime sync
pub async fn run_full_sync_for_ui(state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    // Load config from environment
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            state.add_log(format!("Configuration error: {}", e)).await;
            return Err(e.into());
        }
    };
    
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
pub async fn run_initial_only_for_ui(state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    // Load config from environment
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            state.add_log(format!("Configuration error: {}", e)).await;
            return Err(e.into());
        }
    };
    
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
pub async fn run_realtime_only_for_ui(state: web::state::AppState) -> Result<()> {
    use chrono::Utc;
    
    // Load config from environment
    let config = match Config::from_env() {
        Ok(cfg) => cfg,
        Err(e) => {
            state.add_log(format!("Configuration error: {}", e)).await;
            return Err(e.into());
        }
    };
    
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
    
    state.add_log("Connecting to MySQL...".to_string()).await;
    let mysql_pool = sqlx::mysql::MySqlPool::connect(&config.mysql_url).await?;
    
    state.add_log("Connecting to PostgreSQL...".to_string()).await;
    let pg_pool = sqlx::postgres::PgPool::connect(&config.pg_url).await?;
    
    // Create event channel for async processing
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(1000);
    
    // Initialize stats logger
    let stats_logger = Arc::new(StatsLogger::new("sync_operations_stats.json"));
    
    state.add_log("Starting change monitoring...".to_string()).await;
    
    // Start binlog reader with connection pool
    let mut binlog_reader = BinlogReader::new(mysql_pool, config.mysql_database.clone(), event_tx)?;
    
    // Start PostgreSQL writer worker
    let pg_pool_clone = pg_pool.clone();
    let stats_logger_writer = stats_logger.clone();
    let state_writer = state.clone();
    let writer_handle = tokio::spawn(async move {
        let pg_writer = PGWriter::new(pg_pool_clone);
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
            let success = match pg_writer.handle_event(event).await {
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
    state.add_log("📊 Make changes in MySQL to see them replicated to PostgreSQL".to_string()).await;
    
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

