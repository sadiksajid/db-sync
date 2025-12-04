mod config;
mod migrator;
mod realtime;
mod schema;

use anyhow::Result;
use clap::{Arg, Command};
use config::{Config, SyncMode};
use migrator::{create_tables::TableCreator, data_transfer::DataTransfer, verify::Verifier};
use realtime::{binlog_reader::BinlogReader, pg_writer::PGWriter};
use schema::mysql_reader::MySQLReader;
use std::sync::mpsc;
use tracing::{error, info};
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
            run_initial_sync(&config).await.map_err(|e| {
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
            run_initial_sync(&config).await.map_err(|e| {
                eprintln!("Initial sync failed: {}", e);
                e
            })?;
            run_realtime_sync(&config).await.map_err(|e| {
                eprintln!("Real-time sync failed: {}", e);
                e
            })?;
        }
    }

    info!("Proxy completed successfully");
    Ok(())
}

async fn run_initial_sync(config: &Config) -> Result<()> {
    info!("Starting initial sync (schema + data)");

    // Connect to MySQL
    info!("Connecting to MySQL: {}", config.mysql_url);
    let mysql_pool = sqlx::MySqlPool::connect(&config.mysql_url).await?;
    info!("Connected to MySQL");

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

