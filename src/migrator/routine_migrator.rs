use anyhow::Result;
use sqlx::{PgPool, Pool, Postgres};
use tracing::{error, info, warn};

use crate::ai::gemini_converter::GeminiConverter;
use crate::schema::routine_converter::RoutineConverter;
use crate::schema::routines::{MySQLFunction, MySQLProcedure, MySQLTrigger, MySQLView};

pub struct RoutineMigrator {
    pg_pool: Pool<Postgres>,
    gemini: Option<GeminiConverter>,
}

impl RoutineMigrator {
    pub fn new(pg_pool: PgPool) -> Self {
        // Try to get Gemini API key from environment
        let gemini = std::env::var("GEMINI_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .map(GeminiConverter::new);

        if gemini.is_some() {
            info!("✨ Gemini AI enabled for database objects conversion");
        } else {
            warn!("⚠️  GEMINI_API_KEY not set - database objects (views, functions, procedures, triggers) will be SKIPPED");
            warn!("   To migrate database objects, set GEMINI_API_KEY environment variable");
            warn!("   Get your free API key at: https://makersuite.google.com/app/apikey");
        }

        Self { pg_pool, gemini }
    }

    /// Migrate all views to PostgreSQL
    pub async fn migrate_views(&self, views: &[MySQLView]) -> Result<()> {
        if views.is_empty() {
            info!("No views to migrate");
            return Ok(());
        }

        // Skip if Gemini is not available
        if self.gemini.is_none() {
            warn!("⚠️  Skipping {} views migration (GEMINI_API_KEY not set)", views.len());
            warn!("   Set GEMINI_API_KEY environment variable to enable AI-powered conversion");
            return Ok(());
        }

        info!("Migrating {} views to PostgreSQL...", views.len());

        let mut successful = 0;
        let mut failed = 0;

        for view in views {
            // Use Gemini AI for conversion (required at this point)
            let pg_sql_result = if let Some(ref gemini) = self.gemini {
                gemini.convert_view(view).await
            } else {
                // This should never happen due to earlier check, but handle it anyway
                continue;
            };

            match pg_sql_result {
                Ok(pg_sql) => {
                    match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                        Ok(_) => {
                            info!("✓ Created view: {}", view.name);
                            successful += 1;
                        }
                        Err(e) => {
                            error!("✗ Failed to create view {}: {}", view.name, e);
                            warn!("  SQL was: {}", pg_sql);
                            failed += 1;
                        }
                    }
                }
            Err(e) => {
                // Check if quota exceeded - fallback to regex converter
                if e.to_string().contains("QUOTA_EXCEEDED") {
                    warn!("⚠️  Gemini quota exceeded, falling back to regex converter for view: {}", view.name);
                    
                    // Try regex-based conversion
                    match RoutineConverter::convert_view(view) {
                        Ok(pg_sql) => {
                            match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                                Ok(_) => {
                                    info!("✓ Created view (using regex): {}", view.name);
                                    successful += 1;
                                }
                                Err(create_err) => {
                                    error!("✗ Failed to create view {} (regex fallback): {}", view.name, create_err);
                                    failed += 1;
                                }
                            }
                        }
                        Err(conv_err) => {
                            error!("✗ Failed to convert view {} (regex fallback also failed): {}", view.name, conv_err);
                            failed += 1;
                        }
                    }
                } else {
                    error!("✗ Failed to convert view {}: {}", view.name, e);
                    failed += 1;
                }
            }
            }
        }

        info!(
            "View migration complete: {} successful, {} failed",
            successful, failed
        );

        Ok(())
    }

    /// Migrate all functions to PostgreSQL
    pub async fn migrate_functions(&self, functions: &[MySQLFunction]) -> Result<()> {
        if functions.is_empty() {
            info!("No functions to migrate");
            return Ok(());
        }

        // Skip if Gemini is not available
        if self.gemini.is_none() {
            warn!("⚠️  Skipping {} functions migration (GEMINI_API_KEY not set)", functions.len());
            warn!("   Set GEMINI_API_KEY environment variable to enable AI-powered conversion");
            return Ok(());
        }

        info!("Migrating {} functions to PostgreSQL...", functions.len());

        let mut successful = 0;
        let mut failed = 0;

        for func in functions {
            // Use Gemini AI for conversion (required at this point)
            let pg_sql_result = if let Some(ref gemini) = self.gemini {
                gemini.convert_function(func).await
            } else {
                // This should never happen due to earlier check, but handle it anyway
                continue;
            };

            match pg_sql_result {
                Ok(pg_sql) => {
                    match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                        Ok(_) => {
                            info!("✓ Created function: {}", func.name);
                            successful += 1;
                        }
                        Err(e) => {
                            error!("✗ Failed to create function {}: {}", func.name, e);
                            warn!("  SQL was: {}", pg_sql);
                            warn!("  Note: Function conversion may require manual adjustment");
                            failed += 1;
                        }
                    }
                }
            Err(e) => {
                // Check if quota exceeded - fallback to regex converter
                if e.to_string().contains("QUOTA_EXCEEDED") {
                    warn!("⚠️  Gemini quota exceeded, falling back to regex converter for function: {}", func.name);
                    
                    // Try regex-based conversion
                    match RoutineConverter::convert_function(func) {
                        Ok(pg_sql) => {
                            match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                                Ok(_) => {
                                    info!("✓ Created function (using regex): {}", func.name);
                                    successful += 1;
                                }
                                Err(create_err) => {
                                    error!("✗ Failed to create function {} (regex fallback): {}", func.name, create_err);
                                    failed += 1;
                                }
                            }
                        }
                        Err(conv_err) => {
                            error!("✗ Failed to convert function {} (regex fallback also failed): {}", func.name, conv_err);
                            failed += 1;
                        }
                    }
                } else {
                    error!("✗ Failed to convert function {}: {}", func.name, e);
                    failed += 1;
                }
            }
            }
        }

        info!(
            "Function migration complete: {} successful, {} failed",
            successful, failed
        );

        if failed > 0 {
            warn!("Some functions failed to migrate. This is common due to syntax differences.");
            warn!("Review the errors above and manually adjust the functions in PostgreSQL.");
        }

        Ok(())
    }

    /// Migrate all procedures to PostgreSQL
    pub async fn migrate_procedures(&self, procedures: &[MySQLProcedure]) -> Result<()> {
        if procedures.is_empty() {
            info!("No procedures to migrate");
            return Ok(());
        }

        // Skip if Gemini is not available
        if self.gemini.is_none() {
            warn!("⚠️  Skipping {} procedures migration (GEMINI_API_KEY not set)", procedures.len());
            warn!("   Set GEMINI_API_KEY environment variable to enable AI-powered conversion");
            return Ok(());
        }

        info!("Migrating {} procedures to PostgreSQL...", procedures.len());

        let mut successful = 0;
        let mut failed = 0;

        for proc in procedures {
            // Use Gemini AI for conversion (required at this point)
            let pg_sql_result = if let Some(ref gemini) = self.gemini {
                gemini.convert_procedure(proc).await
            } else {
                // This should never happen due to earlier check, but handle it anyway
                continue;
            };

            match pg_sql_result {
                Ok(pg_sql) => {
                    match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                        Ok(_) => {
                            info!("✓ Created procedure: {}", proc.name);
                            successful += 1;
                        }
                        Err(e) => {
                            error!("✗ Failed to create procedure {}: {}", proc.name, e);
                            warn!("  SQL was: {}", pg_sql);
                            warn!("  Note: Procedure conversion may require manual adjustment");
                            failed += 1;
                        }
                    }
                }
            Err(e) => {
                // Check if quota exceeded - fallback to regex converter
                if e.to_string().contains("QUOTA_EXCEEDED") {
                    warn!("⚠️  Gemini quota exceeded, falling back to regex converter for procedure: {}", proc.name);
                    
                    // Try regex-based conversion
                    match RoutineConverter::convert_procedure(proc) {
                        Ok(pg_sql) => {
                            match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                                Ok(_) => {
                                    info!("✓ Created procedure (using regex): {}", proc.name);
                                    successful += 1;
                                }
                                Err(create_err) => {
                                    error!("✗ Failed to create procedure {} (regex fallback): {}", proc.name, create_err);
                                    failed += 1;
                                }
                            }
                        }
                        Err(conv_err) => {
                            error!("✗ Failed to convert procedure {} (regex fallback also failed): {}", proc.name, conv_err);
                            failed += 1;
                        }
                    }
                } else {
                    error!("✗ Failed to convert procedure {}: {}", proc.name, e);
                    failed += 1;
                }
            }
            }
        }

        info!(
            "Procedure migration complete: {} successful, {} failed",
            successful, failed
        );

        if failed > 0 {
            warn!("Some procedures failed to migrate. This is common due to syntax differences.");
            warn!("Review the errors above and manually adjust the procedures in PostgreSQL.");
        }

        Ok(())
    }

    /// Migrate all triggers to PostgreSQL
    pub async fn migrate_triggers(&self, triggers: &[MySQLTrigger]) -> Result<()> {
        if triggers.is_empty() {
            info!("No triggers to migrate");
            return Ok(());
        }

        // Skip if Gemini is not available
        if self.gemini.is_none() {
            warn!("⚠️  Skipping {} triggers migration (GEMINI_API_KEY not set)", triggers.len());
            warn!("   Set GEMINI_API_KEY environment variable to enable AI-powered conversion");
            return Ok(());
        }

        info!("Migrating {} triggers to PostgreSQL...", triggers.len());

        let mut successful = 0;
        let mut failed = 0;

        for trigger in triggers {
            // Use Gemini AI for conversion (required at this point)
            let pg_sql_result = if let Some(ref gemini) = self.gemini {
                gemini.convert_trigger(trigger).await
            } else {
                // This should never happen due to earlier check, but handle it anyway
                continue;
            };

            match pg_sql_result {
                Ok(pg_sql) => {
                    // Split into function and trigger creation
                    let mut trigger_created = true;
                    for statement in pg_sql.split(';').filter(|s| !s.trim().is_empty()) {
                        let stmt = statement.trim();
                        if stmt.is_empty() {
                            continue;
                        }

                        match sqlx::query(&format!("{};", stmt))
                            .execute(&self.pg_pool)
                            .await
                        {
                            Ok(_) => {}
                            Err(e) => {
                                error!("✗ Failed to execute statement for trigger {}: {}", trigger.name, e);
                                warn!("  SQL was: {}", stmt);
                                failed += 1;
                                trigger_created = false;
                                break;
                            }
                        }
                    }
                    
                    if trigger_created {
                        info!("✓ Created trigger: {} on table {}", trigger.name, trigger.event_object_table);
                        successful += 1;
                    }
                }
            Err(e) => {
                // Check if quota exceeded - fallback to regex converter
                if e.to_string().contains("QUOTA_EXCEEDED") {
                    warn!("⚠️  Gemini quota exceeded, falling back to regex converter for trigger: {}", trigger.name);
                    
                    // Try regex-based conversion
                    match RoutineConverter::convert_trigger(trigger) {
                        Ok(pg_sql) => {
                            // Execute the converted SQL
                            match sqlx::query(&pg_sql).execute(&self.pg_pool).await {
                                Ok(_) => {
                                    info!("✓ Created trigger (using regex): {}", trigger.name);
                                    successful += 1;
                                }
                                Err(create_err) => {
                                    error!("✗ Failed to create trigger {} (regex fallback): {}", trigger.name, create_err);
                                    failed += 1;
                                }
                            }
                        }
                        Err(conv_err) => {
                            error!("✗ Failed to convert trigger {} (regex fallback also failed): {}", trigger.name, conv_err);
                            failed += 1;
                        }
                    }
                } else {
                    error!("✗ Failed to convert trigger {}: {}", trigger.name, e);
                    failed += 1;
                }
            }
            }
        }

        info!(
            "Trigger migration complete: {} successful, {} failed",
            successful, failed
        );

        if failed > 0 {
            warn!("Some triggers failed to migrate. This is common due to syntax differences.");
            warn!("Review the errors above and manually adjust the triggers in PostgreSQL.");
        }

        Ok(())
    }

    /// Migrate all database objects (views, functions, procedures, triggers)
    pub async fn migrate_all(
        &self,
        views: &[MySQLView],
        functions: &[MySQLFunction],
        procedures: &[MySQLProcedure],
        triggers: &[MySQLTrigger],
    ) -> Result<()> {
        info!("=== Starting database objects migration ===");

        // Migrate in order: views -> functions -> procedures -> triggers
        // This order ensures dependencies are handled correctly

        self.migrate_views(views).await?;
        self.migrate_functions(functions).await?;
        self.migrate_procedures(procedures).await?;
        self.migrate_triggers(triggers).await?;

        info!("=== Database objects migration complete ===");

        Ok(())
    }
}

