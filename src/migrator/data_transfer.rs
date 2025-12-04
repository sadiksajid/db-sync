use crate::schema::dependency::DependencyGraph;
use crate::schema::types::MySQLSchema;
use anyhow::Result;
use sqlx::{MySql, PgPool, Pool, Row};
use tracing::info;

pub struct DataTransfer {
    mysql_pool: Pool<MySql>,
    pg_pool: PgPool,
    batch_size: usize,
}

impl DataTransfer {
    pub fn new(mysql_pool: Pool<MySql>, pg_pool: PgPool, batch_size: usize) -> Self {
        Self {
            mysql_pool,
            pg_pool,
            batch_size,
        }
    }

    pub async fn transfer_all_data(&self, schema: &MySQLSchema) -> Result<()> {
        let graph = DependencyGraph::from_schema(schema);
        let table_order = graph.get_table_order_for_data_transfer()?;

        info!("Transferring data for {} tables", table_order.len());
        info!("Table order: {:?}", table_order);

        for (idx, table_name) in table_order.iter().enumerate() {
            info!("[{}/{}] Starting data transfer for table: {}", idx + 1, table_order.len(), table_name);
            if let Some(table) = schema.get_table(table_name) {
                info!("Transferring data for table: {}", table_name);
                if let Err(e) = self.transfer_table_data(table_name, table).await {
                    tracing::error!("Failed to transfer data from table {}: {}", table_name, e);
                    tracing::error!("This was table {} out of {}", idx + 1, table_order.len());
                    return Err(anyhow::anyhow!("Failed to transfer data from table {}: {}", table_name, e));
                }
                info!("Successfully transferred data for table: {}", table_name);
            } else {
                tracing::warn!("Table {} not found in schema, skipping", table_name);
            }
        }

        info!("Data transfer completed");
        Ok(())
    }

    async fn transfer_table_data(&self, table_name: &str, table: &crate::schema::types::TableSchema) -> Result<()> {
        // Get total row count
        let count_query = format!("SELECT COUNT(*) FROM `{}`", table_name);
        let count_row = sqlx::query(&count_query)
            .fetch_one(&self.mysql_pool)
            .await?;
        let total_rows: i64 = count_row.get(0);

        if total_rows == 0 {
            info!("Table {} is empty, skipping", table_name);
            return Ok(());
        }

        info!("Transferring {} rows from table {}", total_rows, table_name);
        
        // Log date columns for debugging
        let date_columns: Vec<&str> = table.columns.iter()
            .filter(|col| matches!(col.data_type.as_str(), "date" | "datetime" | "timestamp"))
            .map(|col| col.name.as_str())
            .collect();
        if !date_columns.is_empty() {
            tracing::debug!("Table {} has date columns: {:?}", table_name, date_columns);
        }

        // Get column names
        let column_names: Vec<String> = table.columns.iter().map(|c| c.name.clone()).collect();
        let quoted_columns: Vec<String> = column_names.iter().map(|c| format!("\"{}\"", c)).collect();
        
        // Build MySQL columns - convert ALL date/datetime/timestamp to strings using CAST
        // For decimal/numeric, use CAST to CHAR to preserve exact values as strings
        // This makes it easier to detect and handle invalid dates and preserve decimal precision
        let mysql_columns: Vec<String> = table.columns.iter().map(|col| {
            let col_name = &col.name;
            match col.data_type.as_str() {
                "date" | "datetime" | "timestamp" => {
                    // Cast to CHAR to get string representation, which will show '0000-00-00' for invalid dates
                    format!("CAST(`{}` AS CHAR) as `{}`", col_name, col_name)
                }
                "decimal" | "numeric" => {
                    // Cast decimal/numeric to CHAR to preserve exact precision (e.g., 300.0 stays as "300.0")
                    format!("CAST(`{}` AS CHAR) as `{}`", col_name, col_name)
                }
                _ => format!("`{}`", col_name)
            }
        }).collect();

        // Fetch data in batches
        let mut offset = 0;
        let mut transferred = 0;

        while offset < total_rows {
            let limit = self.batch_size as i64;
            let query = format!(
                "SELECT {} FROM `{}` LIMIT {} OFFSET {}",
                mysql_columns.join(", "),
                table_name,
                limit,
                offset
            );

            let rows = sqlx::query(&query).fetch_all(&self.mysql_pool).await?;

            if rows.is_empty() {
                break;
            }

            // Convert rows to PostgreSQL format
            self.insert_batch(table_name, &quoted_columns, &rows, &column_names, table).await?;

            transferred += rows.len();
            offset += rows.len() as i64;

            if transferred % 10000 == 0 {
                info!("Transferred {} / {} rows from {}", transferred, total_rows, table_name);
            }
        }

        info!("Completed transferring {} rows from {}", transferred, table_name);
        Ok(())
    }

    async fn insert_batch(
        &self,
        table_name: &str,
        quoted_columns: &[String],
        rows: &[sqlx::mysql::MySqlRow],
        column_names: &[String],
        table: &crate::schema::types::TableSchema,
    ) -> Result<()> {
        if rows.is_empty() {
            return Ok(());
        }

        // Insert rows one by one WITHOUT using a transaction
        // This allows us to skip problematic rows without aborting the entire batch
        for row in rows {
            // Build list of columns and values, excluding NULL/invalid values for date columns
            let mut insert_columns = Vec::new();
            let mut insert_values = Vec::new();
            
            for (col_idx, col_name) in column_names.iter().enumerate() {
                // Check column type to determine how to handle it
                let col_type = table.columns.get(col_idx)
                    .map(|c| c.data_type.as_str())
                    .unwrap_or("");
                
                // Get value as string first, considering the target column type
                // Try multiple methods to ensure we capture the value correctly
                let value_str: Option<String> = {
                    // Try by column name first (most reliable for MySQL)
                    if let Ok(Some(v)) = row.try_get::<Option<String>, _>(col_name.as_str()) {
                        Some(v)
                    } else if let Ok(Some(v)) = row.try_get::<Option<String>, _>(col_idx) {
                        // Try by index as string
                        Some(v)
                    } else if matches!(col_type, "decimal" | "numeric" | "float" | "double" | "real") {
                        // For decimal types, try numeric types as fallback
                        if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<f32>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(col_name.as_str()) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<f32>, _>(col_name.as_str()) {
                            Some(v.to_string())
                        } else {
                            None
                        }
                    } else {
                        // For other types, try numeric types
                        if let Ok(Some(v)) = row.try_get::<Option<i64>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<i32>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<u64>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<u32>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<f64>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<f32>, _>(col_idx) {
                            Some(v.to_string())
                        } else if let Ok(Some(v)) = row.try_get::<Option<bool>, _>(col_idx) {
                            // MySQL tinyint(1) is read as bool, but we need to check target type
                            // If target is boolean in PostgreSQL, use TRUE/FALSE, otherwise use 1/0
                            let is_pg_boolean = col_type == "tinyint" || col_type == "boolean";
                            if is_pg_boolean {
                                Some(if v { "TRUE".to_string() } else { "FALSE".to_string() })
                            } else {
                                // For integer columns, convert bool to 1/0
                                Some(if v { "1".to_string() } else { "0".to_string() })
                            }
                        } else {
                            None
                        }
                    }
                };
                
                // Debug logging for NULL values in important columns
                if value_str.is_none() && matches!(col_type, "decimal" | "numeric" | "float" | "double" | "real") {
                    tracing::debug!("Column {} in table {} returned NULL - trying alternative extraction methods", col_name, table_name);
                }
                
                // Check if value is NULL or invalid date - if so, skip this column
                let should_skip = if let Some(ref v) = value_str {
                    let v_trimmed = v.trim();
                    // Check for invalid MySQL dates
                    let is_invalid_date = v_trimmed.is_empty() ||
                       v_trimmed == "NULL" ||
                       v_trimmed.contains("0000-00-00") ||
                       v_trimmed.starts_with("0000") ||
                       v_trimmed.contains("0000-00") ||
                       (col_type == "date" && (v_trimmed.contains("0000") || v_trimmed.len() < 10)) ||
                       (col_type == "datetime" && (v_trimmed.contains("0000") || v_trimmed.len() < 19)) ||
                       (col_type == "timestamp" && (v_trimmed.contains("0000") || v_trimmed.len() < 19));
                    
                    if is_invalid_date && (col_type == "date" || col_type == "datetime" || col_type == "timestamp") {
                        tracing::debug!("Skipping column {} in table {} with invalid date value: {}", col_name, table_name, v_trimmed);
                        true
                    } else {
                        false
                    }
                } else {
                    // Value is NULL in MySQL - only skip for date columns to let PostgreSQL use default
                    // For other columns (including numeric), we need to insert NULL explicitly
                    if col_type == "date" || col_type == "datetime" || col_type == "timestamp" {
                        tracing::debug!("Skipping NULL date column {} in table {}", col_name, table_name);
                        true
                    } else {
                        // For non-date columns, include NULL in the INSERT (don't skip)
                        false
                    }
                };
                
                // If we should skip this column, don't add it to the INSERT
                if should_skip {
                    continue;
                }
                
                // Add column to INSERT
                insert_columns.push(quoted_columns[col_idx].clone());
                
                // Format value for PostgreSQL
                let pg_value = if let Some(ref v) = value_str {
                    let v_trimmed = v.trim();
                    
                    // Check if this is a numeric column - don't quote numeric values
                    let is_numeric_type = matches!(col_type, "tinyint" | "smallint" | "mediumint" | "int" | "integer" | "bigint" | 
                                                      "decimal" | "numeric" | "float" | "double" | "real");
                    
                    if is_numeric_type {
                        // For numeric types, use the value directly without quotes
                        // Handle empty string or "NULL" string
                        if v_trimmed.is_empty() || v_trimmed.to_uppercase() == "NULL" {
                            // Check if column is NOT NULL - if so, use default value (0 or 0.00)
                            let col = table.columns.get(col_idx);
                            let is_not_null = col.map(|c| !c.is_nullable).unwrap_or(false);
                            if is_not_null {
                                // Use appropriate default based on type
                                if matches!(col_type, "decimal" | "numeric" | "float" | "double" | "real") {
                                    "0.00".to_string()
                                } else {
                                    "0".to_string()
                                }
                            } else {
                                "NULL".to_string()
                            }
                        } else {
                            v_trimmed.to_string()
                        }
                    } else {
                        // For non-numeric types, escape single quotes and wrap in quotes
                        // Handle empty string or "NULL" string
                        if v_trimmed.is_empty() || v_trimmed.to_uppercase() == "NULL" {
                            // Check if column is NOT NULL - if so, use default value (empty string)
                            let col = table.columns.get(col_idx);
                            let is_not_null = col.map(|c| !c.is_nullable).unwrap_or(false);
                            if is_not_null {
                                "''".to_string()  // Empty string for NOT NULL string columns
                            } else {
                                "NULL".to_string()
                            }
                        } else {
                            let escaped = v_trimmed.replace('\'', "''");
                            format!("'{}'", escaped)
                        }
                    }
                } else {
                    // value_str is None - this is a NULL value from MySQL
                    // Check if column is NOT NULL - if so, use default value
                    let col = table.columns.get(col_idx);
                    let is_not_null = col.map(|c| !c.is_nullable).unwrap_or(false);
                    let is_numeric_type = matches!(col_type, "tinyint" | "smallint" | "mediumint" | "int" | "integer" | "bigint" | 
                                                      "decimal" | "numeric" | "float" | "double" | "real");
                    
                    if is_not_null {
                        if is_numeric_type {
                            // For NOT NULL numeric columns, use 0 or 0.00 instead of NULL
                            if matches!(col_type, "decimal" | "numeric" | "float" | "double" | "real") {
                                "0.00".to_string()
                            } else {
                                "0".to_string()
                            }
                        } else {
                            // For NOT NULL string/text columns, use empty string instead of NULL
                            "''".to_string()
                        }
                    } else {
                        "NULL".to_string()
                    }
                };
                
                insert_values.push(pg_value);
            }
            
            // If all columns were skipped, skip this row entirely
            if insert_columns.is_empty() {
                tracing::warn!("Skipping row in table {} - all columns were NULL/invalid", table_name);
                continue;
            }
            
            let insert = format!(
                "INSERT INTO \"{}\" ({}) VALUES ({})",
                table_name,
                insert_columns.join(", "),
                insert_values.join(", ")
            );
            
            // Try to execute each INSERT independently (no transaction)
            match sqlx::query(&insert).execute(&self.pg_pool).await {
                Ok(_) => {},
                Err(e) => {
                    let error_msg = e.to_string();
                    if error_msg.contains("date/time field value out of range") || 
                       error_msg.contains("0000-00-00") {
                        // Log detailed information about the error
                        tracing::warn!("Skipping row in table {} due to invalid date: {}", table_name, e);
                        
                        // Continue with next row instead of failing
                        continue;
                    } else if error_msg.contains("violates not-null constraint") ||
                              error_msg.contains("null value in column") {
                        // NULL value in a NOT NULL column - skip this row
                        tracing::warn!("Skipping row in table {} due to NOT NULL constraint violation: {}", table_name, e);
                        tracing::debug!("INSERT statement: {}", &insert.chars().take(300).collect::<String>());
                        
                        // Continue with next row instead of failing
                        continue;
                    } else if error_msg.contains("value too long for type") ||
                              error_msg.contains("character varying") {
                        // Value too long for VARCHAR column - skip this row
                        tracing::warn!("Skipping row in table {} due to value too long: {}", table_name, e);
                        tracing::debug!("INSERT statement: {}", &insert.chars().take(300).collect::<String>());
                        
                        // Continue with next row instead of failing
                        continue;
                    } else if error_msg.contains("duplicate key value") ||
                              error_msg.contains("unique constraint") {
                        // Duplicate key - skip this row (data may already exist)
                        tracing::debug!("Skipping duplicate row in table {}: {}", table_name, e);
                        
                        // Continue with next row instead of failing
                        continue;
                    } else if error_msg.contains("foreign key constraint") {
                        // Foreign key violation - skip this row
                        tracing::warn!("Skipping row in table {} due to foreign key constraint: {}", table_name, e);
                        
                        // Continue with next row instead of failing
                        continue;
                    } else {
                        // For other errors, log and continue instead of failing
                        tracing::error!("Error inserting row into table {}: {}", table_name, e);
                        tracing::debug!("INSERT statement: {}", &insert.chars().take(300).collect::<String>());
                        
                        // Continue with next row instead of failing the entire sync
                        continue;
                    }
                }
            }
        }
        
        Ok(())
    }
}
