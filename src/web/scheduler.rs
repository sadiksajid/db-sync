use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use chrono::Utc;

use super::schedule_store::ScheduleStore;
use super::state::AppState;
use crate::Config;

pub struct SchedulerService {
    scheduler: JobScheduler,
    schedule_store: Arc<ScheduleStore>,
    app_state: AppState,
}

impl SchedulerService {
    pub async fn new(schedule_store: Arc<ScheduleStore>, app_state: AppState) -> Result<Self> {
        let scheduler = JobScheduler::new().await?;
        
        Ok(Self {
            scheduler,
            schedule_store,
            app_state,
        })
    }

    pub async fn start(&self) -> Result<()> {
        // Start the scheduler
        self.scheduler.start().await?;
        tracing::info!("✅ Scheduler service started");

        // Load and register all enabled schedules
        self.reload_schedules().await?;

        Ok(())
    }

    pub async fn reload_schedules(&self) -> Result<()> {
        // Load enabled schedules from database
        let schedules = self.schedule_store.get_enabled().await?;
        
        tracing::info!("📅 Loading {} enabled schedule(s)", schedules.len());

        for schedule in schedules {
            match self.add_schedule_job(schedule.id, &schedule.cron_expression).await {
                Ok(_) => {
                    tracing::info!("✅ Registered schedule: {} ({})", schedule.name, schedule.cron_expression);
                }
                Err(e) => {
                    tracing::error!("❌ Failed to register schedule {}: {}", schedule.name, e);
                }
            }
        }

        Ok(())
    }

    async fn add_schedule_job(&self, schedule_id: i64, cron_expr: &str) -> Result<()> {
        let schedule_store = self.schedule_store.clone();
        let app_state = self.app_state.clone();
        
        let job = Job::new_async(cron_expr, move |_uuid, _l| {
            let schedule_store = schedule_store.clone();
            let app_state = app_state.clone();
            let schedule_id = schedule_id;
            
            Box::pin(async move {
                if let Err(e) = run_scheduled_sync(schedule_id, schedule_store, app_state).await {
                    tracing::error!("❌ Scheduled sync failed for schedule {}: {}", schedule_id, e);
                }
            })
        })?;

        self.scheduler.add(job).await?;
        
        Ok(())
    }

    pub async fn stop(mut self) -> Result<()> {
        self.scheduler.shutdown().await?;
        tracing::info!("🛑 Scheduler service stopped");
        Ok(())
    }
}

async fn run_scheduled_sync(
    schedule_id: i64,
    schedule_store: Arc<ScheduleStore>,
    app_state: AppState,
) -> Result<()> {
    tracing::info!("🔄 Starting scheduled sync for schedule ID: {}", schedule_id);
    
    // Update last_run timestamp
    let now = Utc::now().to_rfc3339();
    schedule_store.update_last_run(schedule_id, &now, None).await?;

    // Get configuration
    let config_result = app_state.config_store.load_config().await;
    let config = match config_result {
        Ok(Some(cfg)) => cfg,
        Ok(None) => {
            tracing::error!("❌ No configuration found for scheduled sync");
            app_state.add_log(format!("❌ [Schedule #{}] No configuration found", schedule_id)).await;
            return Ok(());
        }
        Err(e) => {
            tracing::error!("❌ Failed to load configuration: {}", e);
            app_state.add_log(format!("❌ [Schedule #{}] Failed to load configuration: {}", schedule_id, e)).await;
            return Ok(());
        }
    };

    // Get all slave configurations from config.slaves
    let slaves = &config.slaves;

    if slaves.is_empty() {
        tracing::warn!("⚠️  No slave databases configured for scheduled sync");
        app_state.add_log(format!("⚠️  [Schedule #{}] No slave databases configured", schedule_id)).await;
        return Ok(());
    }

    //Convert configuration to slave configs
    let slave_configs_result = config.to_slave_configs();
    let slave_configs = match slave_configs_result {
        Ok(configs) => configs,
        Err(e) => {
            tracing::error!("❌ Failed to create slave configs: {}", e);
            app_state.add_log(format!("❌ [Schedule #{}] Failed to create configs: {}", schedule_id, e)).await;
            return Ok(());
        }
    };

    // Enable reset_database for scheduled syncs (always fresh sync)
    {
        let mut web_config = app_state.config.write().await;
        web_config.reset_database = true;
    }

    app_state.add_log(format!("🔄 [Schedule #{}] Starting RESET + SYNC to {} slave(s)...", schedule_id, slave_configs.len())).await;

    // Run initial sync for all slaves in parallel
    let mut handles = Vec::new();

    for (idx, slave_config) in slave_configs.into_iter().enumerate() {
        let app_state_clone = app_state.clone();
        let slave_num = idx + 1;
        let schedule_id_clone = schedule_id;

        let handle = tokio::spawn(async move {
            app_state_clone.add_log(format!("🔵 [Schedule #{} - Slave #{}] Starting sync to '{}'...", 
                schedule_id_clone, slave_num, slave_config.target_database)).await;

            let result = crate::run_initial_only_for_ui(slave_config.clone(), app_state_clone.clone()).await;

            match result {
                Ok(_) => {
                    app_state_clone.add_log(format!("✅ [Schedule #{} - Slave #{}] Sync completed successfully for '{}'", 
                        schedule_id_clone, slave_num, slave_config.target_database)).await;
                    Ok(())
                }
                Err(e) => {
                    app_state_clone.add_log(format!("❌ [Schedule #{} - Slave #{}] Sync failed for '{}': {}", 
                        schedule_id_clone, slave_num, slave_config.target_database, e)).await;
                    Err(e)
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all to complete
    let results = futures::future::join_all(handles).await;
    
    let mut success_count = 0;
    let mut error_count = 0;

    for result in results {
        match result {
            Ok(Ok(_)) => success_count += 1,
            _ => error_count += 1,
        }
    }

    app_state.add_log(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")).await;
    app_state.add_log(format!("📊 [Schedule #{}] SYNC RESULTS:", schedule_id)).await;
    app_state.add_log(format!("✅ Successful: {}", success_count)).await;
    app_state.add_log(format!("❌ Failed: {}", error_count)).await;
    app_state.add_log(format!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━")).await;

    tracing::info!("✅ Scheduled sync completed for schedule ID: {}", schedule_id);
    
    Ok(())
}

