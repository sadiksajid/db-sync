use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool, Row};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLView {
    pub name: String,
    pub definition: String,
    pub check_option: Option<String>,
    pub is_updatable: bool,
    pub definer: Option<String>,
    pub security_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLFunction {
    pub name: String,
    pub definition: String,
    pub returns: String,
    pub is_deterministic: bool,
    pub sql_data_access: String,
    pub definer: Option<String>,
    pub security_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLProcedure {
    pub name: String,
    pub definition: String,
    pub sql_data_access: String,
    pub is_deterministic: bool,
    pub definer: Option<String>,
    pub security_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLTrigger {
    pub name: String,
    pub event_manipulation: String, // INSERT, UPDATE, DELETE
    pub event_object_table: String,
    pub action_statement: String,
    pub action_timing: String, // BEFORE, AFTER
    pub definer: Option<String>,
}

pub struct RoutineReader {
    pool: Pool<MySql>,
    database: String,
}

impl RoutineReader {
    pub fn new(pool: Pool<MySql>, database: String) -> Self {
        Self { pool, database }
    }

    /// Read all views from the database
    pub async fn read_views(&self) -> Result<Vec<MySQLView>> {
        info!("Reading views from MySQL database: {}", self.database);

        let query = r#"
            SELECT 
                TABLE_NAME as name,
                VIEW_DEFINITION as definition,
                CHECK_OPTION as check_option,
                IS_UPDATABLE as is_updatable,
                DEFINER as definer,
                SECURITY_TYPE as security_type
            FROM INFORMATION_SCHEMA.VIEWS
            WHERE TABLE_SCHEMA = ?
            ORDER BY TABLE_NAME
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .fetch_all(&self.pool)
            .await?;

        let mut views = Vec::new();

        for row in rows {
            let name: String = row.try_get("name")?;
            let definition: String = row.try_get("definition")?;
            let check_option: Option<String> = row.try_get("check_option").ok();
            let is_updatable: String = row.try_get("is_updatable")?;
            let definer: Option<String> = row.try_get("definer").ok();
            let security_type: Option<String> = row.try_get("security_type").ok();

            views.push(MySQLView {
                name: name.clone(),
                definition,
                check_option,
                is_updatable: is_updatable == "YES",
                definer,
                security_type,
            });

            debug!("Found view: {}", name);
        }

        info!("Found {} views", views.len());
        Ok(views)
    }

    /// Read all functions from the database
    pub async fn read_functions(&self) -> Result<Vec<MySQLFunction>> {
        info!("Reading functions from MySQL database: {}", self.database);

        let query = r#"
            SELECT 
                ROUTINE_NAME as name,
                ROUTINE_DEFINITION as definition,
                DTD_IDENTIFIER as returns,
                IS_DETERMINISTIC as is_deterministic,
                SQL_DATA_ACCESS as sql_data_access,
                DEFINER as definer,
                SECURITY_TYPE as security_type
            FROM INFORMATION_SCHEMA.ROUTINES
            WHERE ROUTINE_SCHEMA = ?
            AND ROUTINE_TYPE = 'FUNCTION'
            ORDER BY ROUTINE_NAME
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .fetch_all(&self.pool)
            .await?;

        let mut functions = Vec::new();

        for row in rows {
            let name: String = row.try_get("name")?;
            let definition: Option<String> = row.try_get("definition").ok();
            let returns: String = row.try_get("returns")?;
            let is_deterministic: String = row.try_get("is_deterministic")?;
            let sql_data_access: String = row.try_get("sql_data_access")?;
            let definer: Option<String> = row.try_get("definer").ok();
            let security_type: Option<String> = row.try_get("security_type").ok();

            // If definition is empty, try to get it from SHOW CREATE FUNCTION
            let definition = if definition.is_none() || definition.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                self.get_function_definition(&name).await?
            } else {
                definition.unwrap()
            };

            functions.push(MySQLFunction {
                name: name.clone(),
                definition,
                returns,
                is_deterministic: is_deterministic == "YES",
                sql_data_access,
                definer,
                security_type,
            });

            debug!("Found function: {}", name);
        }

        info!("Found {} functions", functions.len());
        Ok(functions)
    }

    /// Read all stored procedures from the database
    pub async fn read_procedures(&self) -> Result<Vec<MySQLProcedure>> {
        info!("Reading procedures from MySQL database: {}", self.database);

        let query = r#"
            SELECT 
                ROUTINE_NAME as name,
                ROUTINE_DEFINITION as definition,
                SQL_DATA_ACCESS as sql_data_access,
                IS_DETERMINISTIC as is_deterministic,
                DEFINER as definer,
                SECURITY_TYPE as security_type
            FROM INFORMATION_SCHEMA.ROUTINES
            WHERE ROUTINE_SCHEMA = ?
            AND ROUTINE_TYPE = 'PROCEDURE'
            ORDER BY ROUTINE_NAME
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .fetch_all(&self.pool)
            .await?;

        let mut procedures = Vec::new();

        for row in rows {
            let name: String = row.try_get("name")?;
            let definition: Option<String> = row.try_get("definition").ok();
            let sql_data_access: String = row.try_get("sql_data_access")?;
            let is_deterministic: String = row.try_get("is_deterministic")?;
            let definer: Option<String> = row.try_get("definer").ok();
            let security_type: Option<String> = row.try_get("security_type").ok();

            // If definition is empty, try to get it from SHOW CREATE PROCEDURE
            let definition = if definition.is_none() || definition.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
                self.get_procedure_definition(&name).await?
            } else {
                definition.unwrap()
            };

            procedures.push(MySQLProcedure {
                name: name.clone(),
                definition,
                sql_data_access,
                is_deterministic: is_deterministic == "YES",
                definer,
                security_type,
            });

            debug!("Found procedure: {}", name);
        }

        info!("Found {} procedures", procedures.len());
        Ok(procedures)
    }

    /// Read all triggers from the database
    pub async fn read_triggers(&self) -> Result<Vec<MySQLTrigger>> {
        info!("Reading triggers from MySQL database: {}", self.database);

        let query = r#"
            SELECT 
                TRIGGER_NAME as name,
                EVENT_MANIPULATION as event_manipulation,
                EVENT_OBJECT_TABLE as event_object_table,
                ACTION_STATEMENT as action_statement,
                ACTION_TIMING as action_timing,
                DEFINER as definer
            FROM INFORMATION_SCHEMA.TRIGGERS
            WHERE TRIGGER_SCHEMA = ?
            ORDER BY EVENT_OBJECT_TABLE, TRIGGER_NAME
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .fetch_all(&self.pool)
            .await?;

        let mut triggers = Vec::new();

        for row in rows {
            let name: String = row.try_get("name")?;
            let event_manipulation: String = row.try_get("event_manipulation")?;
            let event_object_table: String = row.try_get("event_object_table")?;
            let action_statement: String = row.try_get("action_statement")?;
            let action_timing: String = row.try_get("action_timing")?;
            let definer: Option<String> = row.try_get("definer").ok();

            triggers.push(MySQLTrigger {
                name: name.clone(),
                event_manipulation,
                event_object_table: event_object_table.clone(),
                action_statement,
                action_timing,
                definer,
            });

            debug!("Found trigger: {} on table {}", name, event_object_table);
        }

        info!("Found {} triggers", triggers.len());
        Ok(triggers)
    }

    /// Get function definition using SHOW CREATE FUNCTION
    async fn get_function_definition(&self, name: &str) -> Result<String> {
        let query = format!("SHOW CREATE FUNCTION `{}`", name);
        
        match sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => {
                // The definition is in the "Create Function" column (index 2)
                if let Ok(definition) = row.try_get::<String, _>(2) {
                    Ok(definition)
                } else if let Ok(definition) = row.try_get::<String, _>("Create Function") {
                    Ok(definition)
                } else {
                    Ok(String::new())
                }
            }
            Err(e) => {
                debug!("Could not get function definition for {}: {}", name, e);
                Ok(String::new())
            }
        }
    }

    /// Get procedure definition using SHOW CREATE PROCEDURE
    async fn get_procedure_definition(&self, name: &str) -> Result<String> {
        let query = format!("SHOW CREATE PROCEDURE `{}`", name);
        
        match sqlx::query(&query)
            .fetch_one(&self.pool)
            .await
        {
            Ok(row) => {
                // The definition is in the "Create Procedure" column (index 2)
                if let Ok(definition) = row.try_get::<String, _>(2) {
                    Ok(definition)
                } else if let Ok(definition) = row.try_get::<String, _>("Create Procedure") {
                    Ok(definition)
                } else {
                    Ok(String::new())
                }
            }
            Err(e) => {
                debug!("Could not get procedure definition for {}: {}", name, e);
                Ok(String::new())
            }
        }
    }
}

