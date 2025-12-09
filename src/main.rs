mod ai;
mod config;
mod migrator;
mod realtime;
mod schema;

use anyhow::Result;
use clap::{Arg, Command};
use config::{Config, SyncMode};
use migrator::{create_tables::TableCreator, data_transfer::DataTransfer, routine_migrator::RoutineMigrator, verify::Verifier};
use realtime::{binlog_reader::BinlogReader, pg_writer::PGWriter};
use schema::{mysql_reader::MySQLReader, routines::RoutineReader};
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
        .get_matches();

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

    // Start binlog reader with connection pool
    let mut binlog_reader = BinlogReader::new(mysql_pool, config.mysql_database.clone(), event_tx)?;

    // Start PostgreSQL writer worker (runs in background, doesn't block listener)
    let pg_pool_clone = pg_pool.clone();
    let writer_handle = tokio::spawn(async move {
        info!("PostgreSQL writer worker started, processing queue...");
        let pg_writer = PGWriter::new(pg_pool_clone);
        let mut event_count = 0;
        
        while let Some(event) = event_rx.recv().await {
            event_count += 1;
            let event_type = match &event {
                crate::realtime::binlog_reader::BinlogEventType::Insert { table, values } => {
                    format!("INSERT: table={}, {} columns", table, values.len())
                }
                crate::realtime::binlog_reader::BinlogEventType::Update { table, new_values, .. } => {
                    format!("UPDATE: table={}, {} columns", table, new_values.len())
                }
                crate::realtime::binlog_reader::BinlogEventType::Delete { table, values } => {
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

    info!("Real-time sync stopped");
    Ok(())
}

