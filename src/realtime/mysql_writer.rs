use crate::realtime::binlog_reader::BinlogEventType;
use anyhow::Result;
use sqlx::MySqlPool;
use std::time::Duration;
use tracing::{error, info, warn};

pub struct MySQLWriter {
    pool: MySqlPool,
    max_retries: u32,
    retry_delay: Duration,
}

impl MySQLWriter {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
        }
    }

    pub async fn handle_event(&self, event: BinlogEventType) -> Result<()> {
        let mut retries = 0;

        loop {
            match self.execute_event(&event).await {
                Ok(()) => {
                    return Ok(());
                }
                Err(e) => {
                    retries += 1;
                    if retries >= self.max_retries {
                        error!(
                            "Failed to execute event after {} retries: {}",
                            self.max_retries, e
                        );
                        return Err(e);
                    }
                    warn!(
                        "Error executing event (retry {}/{}): {}, retrying...",
                        retries, self.max_retries, e
                    );
                    tokio::time::sleep(self.retry_delay * retries).await;
                }
            }
        }
    }

    async fn execute_event(&self, event: &BinlogEventType) -> Result<()> {
        match event {
            BinlogEventType::Insert { table, values } => {
                self.handle_insert(table, values).await
            }
            BinlogEventType::Update {
                table,
                old_values: _,
                new_values,
            } => {
                self.handle_update(table, new_values).await
            }
            BinlogEventType::Delete { table, values } => {
                self.handle_delete(table, values).await
            }
        }
    }

    async fn handle_insert(&self, table: &str, values: &[(String, String)]) -> Result<()> {
        info!("Processing INSERT for table: {}", table);
        
        if values.is_empty() {
            warn!("INSERT event has no values for table: {}", table);
            return Ok(());
        }

        let columns: Vec<String> = values.iter().map(|(col, _)| col.clone()).collect();
        let quoted_columns: Vec<String> = columns.iter().map(|c| format!("`{}`", c)).collect();

        let value_placeholders: Vec<String> = values
            .iter()
            .map(|_| "?".to_string())
            .collect();

        let sql = format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            table,
            quoted_columns.join(", "),
            value_placeholders.join(", ")
        );

        info!("Executing INSERT: {}", sql);

        let mut query = sqlx::query(&sql);
        for (col, value) in values {
            let cleaned = value.trim_matches('"').trim_matches('\'');
            if cleaned.to_uppercase() == "NULL" {
                query = query.bind(None::<String>);
            } else if col == "id" || col.ends_with("_id") || col.ends_with("Id") {
                if let Ok(int_val) = cleaned.parse::<i64>() {
                    query = query.bind(int_val);
                } else if let Ok(int_val) = cleaned.parse::<i32>() {
                    query = query.bind(int_val);
                } else {
                    query = query.bind(cleaned);
                }
            } else if let Ok(float_val) = cleaned.parse::<f64>() {
                query = query.bind(float_val);
            } else {
                query = query.bind(cleaned);
            }
            info!("  {} = {}", col, value);
        }

        query.execute(&self.pool).await?;
        info!("✓ Successfully inserted row into table: {}", table);
        Ok(())
    }

    async fn handle_update(&self, table: &str, new_values: &[(String, String)]) -> Result<()> {
        info!("Processing UPDATE for table: {}", table);
        
        if new_values.is_empty() {
            warn!("UPDATE event has no values for table: {}", table);
            return Ok(());
        }

        // Find primary key column - prioritize "id", then columns ending with "_id"
        // Exclude columns that look like table-qualified (contain `.`)
        let pk_column = new_values
            .iter()
            .find(|(col, _)| {
                !col.contains('.') && (col == "id" || col.ends_with("_id"))
            })
            .map(|(col, _)| col.clone())
            .or_else(|| {
                // Fallback: find first column that doesn't contain '.'
                new_values.iter()
                    .find(|(col, _)| !col.contains('.'))
                    .map(|(col, _)| col.clone())
            })
            .ok_or_else(|| anyhow::anyhow!("No valid primary key column found for UPDATE"))?;
        
        info!("Identified primary key column: {}", pk_column);

        let pk_value = new_values
            .iter()
            .find(|(col, _)| col == &pk_column)
            .map(|(_, val)| val.clone())
            .ok_or_else(|| anyhow::anyhow!("Primary key value not found"))?;

        // Build SET clause
        let set_clauses: Vec<String> = new_values
            .iter()
            .filter(|(col, _)| col != &pk_column)
            .map(|(col, _)| format!("`{}` = ?", col))
            .collect();

        if set_clauses.is_empty() {
            warn!("UPDATE has no columns to update for table: {}", table);
            return Ok(());
        }

        let sql = format!(
            "UPDATE `{}` SET {} WHERE `{}` = ?",
            table,
            set_clauses.join(", "),
            pk_column
        );

        info!("Executing UPDATE: {}", sql);
        info!("  WHERE {} = {}", pk_column, pk_value);

        let mut query = sqlx::query(&sql);
        for (col, value) in new_values.iter().filter(|(col, _)| col != &pk_column) {
            let cleaned = value.trim_matches('"').trim_matches('\'');
            if cleaned.to_uppercase() == "NULL" {
                query = query.bind(None::<String>);
            } else if col == "id" || col.ends_with("_id") || col.ends_with("Id") {
                if let Ok(int_val) = cleaned.parse::<i64>() {
                    query = query.bind(int_val);
                } else if let Ok(int_val) = cleaned.parse::<i32>() {
                    query = query.bind(int_val);
                } else {
                    query = query.bind(cleaned);
                }
            } else if let Ok(float_val) = cleaned.parse::<f64>() {
                query = query.bind(float_val);
            } else {
                query = query.bind(cleaned);
            }
            info!("  SET {} = {}", col, value);
        }
        
        // Bind primary key value
        let cleaned_pk = pk_value.trim_matches('"').trim_matches('\'');
        if cleaned_pk.to_uppercase() == "NULL" {
            query = query.bind(None::<String>);
        } else if pk_column == "id" || pk_column.ends_with("_id") || pk_column.ends_with("Id") {
            if let Ok(int_val) = cleaned_pk.parse::<i64>() {
                query = query.bind(int_val);
            } else if let Ok(int_val) = cleaned_pk.parse::<i32>() {
                query = query.bind(int_val);
            } else {
                query = query.bind(cleaned_pk);
            }
        } else {
            query = query.bind(cleaned_pk);
        }

        query.execute(&self.pool).await?;
        info!("✓ Successfully updated row in table: {}", table);
        Ok(())
    }

    async fn handle_delete(&self, table: &str, values: &[(String, String)]) -> Result<()> {
        info!("Processing DELETE for table: {}", table);
        
        if values.is_empty() {
            warn!("DELETE event has no values for table: {}", table);
            return Ok(());
        }

        // Find primary key column
        let pk_column = values
            .iter()
            .find(|(col, _)| col == "id" || col.ends_with("_id"))
            .map(|(col, _)| col.clone())
            .or_else(|| values.first().map(|(col, _)| col.clone()))
            .ok_or_else(|| anyhow::anyhow!("No columns for DELETE"))?;

        let pk_value = values
            .iter()
            .find(|(col, _)| col == &pk_column)
            .map(|(_, val)| val.clone())
            .ok_or_else(|| anyhow::anyhow!("Primary key value not found"))?;

        let sql = format!("DELETE FROM `{}` WHERE `{}` = ?", table, pk_column);
        info!("Executing DELETE: {}", sql);
        info!("  WHERE {} = {}", pk_column, pk_value);
        
        let cleaned_pk = pk_value.trim_matches('"').trim_matches('\'');
        let query = if cleaned_pk.to_uppercase() == "NULL" {
            sqlx::query(&sql).bind(None::<String>)
        } else if pk_column == "id" || pk_column.ends_with("_id") || pk_column.ends_with("Id") {
            if let Ok(int_val) = cleaned_pk.parse::<i64>() {
                sqlx::query(&sql).bind(int_val)
            } else if let Ok(int_val) = cleaned_pk.parse::<i32>() {
                sqlx::query(&sql).bind(int_val)
            } else {
                sqlx::query(&sql).bind(cleaned_pk)
            }
        } else {
            sqlx::query(&sql).bind(cleaned_pk)
        };
        query.execute(&self.pool).await?;

        info!("✓ Successfully deleted row from table: {}", table);
        Ok(())
    }

    fn parse_value(&self, value: &str) -> String {
        // Remove quotes if present
        let cleaned = value.trim_matches('"').trim_matches('\'');
        
        // Handle NULL
        if cleaned.to_uppercase() == "NULL" {
            return "NULL".to_string();
        }

        // Return as-is (will be properly escaped by sqlx)
        cleaned.to_string()
    }
}

