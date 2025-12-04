use anyhow::Result;
use sqlx::{MySql, Pool, Row};
use tokio::sync::mpsc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub enum BinlogEventType {
    Insert {
        table: String,
        values: Vec<(String, String)>, // column_name, value
    },
    Update {
        table: String,
        old_values: Vec<(String, String)>,
        new_values: Vec<(String, String)>,
    },
    Delete {
        table: String,
        values: Vec<(String, String)>,
    },
}

pub struct BinlogReader {
    mysql_pool: Pool<MySql>,
    database: String,
    event_tx: mpsc::Sender<BinlogEventType>,
    last_check_time: SystemTime,
    last_processed_event_time: Option<String>, // Track last processed event_time as string
    poll_interval: Duration,
}

impl BinlogReader {
    pub fn new(mysql_pool: Pool<MySql>, database: String, event_tx: mpsc::Sender<BinlogEventType>) -> Result<Self> {
        Ok(Self {
            mysql_pool,
            database,
            event_tx,
            last_check_time: SystemTime::now(),
            last_processed_event_time: None,
            poll_interval: Duration::from_secs(1), // Poll every second
        })
    }

    pub async fn start_streaming(&mut self) -> Result<()> {
        info!("Starting MySQL change monitoring (polling mode)...");
        info!("Monitoring database: {}", self.database);
        info!("Poll interval: {:?}", self.poll_interval);
        
        // Enable general_log if possible (for query monitoring)
        // Note: This requires SUPER privilege
        self.enable_general_log().await?;
        
        info!("Change monitoring started. Waiting for INSERT/UPDATE/DELETE operations...");
        info!("Make a change in MySQL to see it replicated to PostgreSQL");
        
        let mut iteration = 0;
        loop {
            iteration += 1;
            if iteration % 60 == 0 {
                // Log every 60 iterations (every minute if polling every second)
                info!("Change monitor is running... (iteration {})", iteration);
            }
            
            if let Err(e) = self.check_for_changes().await {
                error!("Error checking for changes: {}", e);
                // Still update timestamp to avoid getting stuck
                self.last_check_time = SystemTime::now();
            }
            // Note: last_check_time is updated inside check_for_changes()
            
            tokio::time::sleep(self.poll_interval).await;
        }
    }

    async fn enable_general_log(&self) -> Result<()> {
        // Try to enable general_log for query monitoring
        // This requires SUPER privilege
        info!("Attempting to enable MySQL general_log...");
        
        match sqlx::query("SET GLOBAL general_log = 'ON'")
            .execute(&self.mysql_pool)
            .await
        {
            Ok(_) => {
                info!("✓ Successfully enabled general_log");
            }
            Err(e) => {
                warn!("Could not enable general_log: {} (requires SUPER privilege)", e);
            }
        }
        
        match sqlx::query("SET GLOBAL log_output = 'TABLE'")
            .execute(&self.mysql_pool)
            .await
        {
            Ok(_) => {
                info!("✓ Successfully set log_output to TABLE");
            }
            Err(e) => {
                warn!("Could not set log_output: {} (requires SUPER privilege)", e);
            }
        }
        
        // Check if general_log is actually enabled
        match sqlx::query("SHOW VARIABLES LIKE 'general_log'")
            .fetch_all(&self.mysql_pool)
            .await
        {
            Ok(rows) => {
                for row in rows {
                    if let Ok(var_name) = row.try_get::<String, _>(0) {
                        if let Ok(var_value) = row.try_get::<String, _>(1) {
                            info!("MySQL variable {} = {}", var_name, var_value);
                            if var_value.to_uppercase() != "ON" {
                                warn!("general_log is not ON - real-time sync may not work properly");
                            }
                        }
                    }
                }
            }
            Err(e) => {
                warn!("Could not check general_log status: {}", e);
            }
        }
        
        Ok(())
    }

    async fn check_for_changes(&mut self) -> Result<()> {
        // Build query - use last_processed_event_time if available, otherwise use timestamp
        let query = if let Some(ref last_time) = self.last_processed_event_time {
            // Use the last processed event_time to avoid reprocessing
            // Only log at debug level to avoid spam
            debug!("Using last processed event_time: {}", last_time);
            format!(r#"
                SELECT 
                    CAST(argument AS CHAR) as query,
                    CAST(event_time AS CHAR) as event_time
                FROM mysql.general_log
                WHERE 
                    command_type = 'Query'
                    AND (
                        UPPER(CAST(argument AS CHAR)) LIKE 'INSERT%' OR
                        UPPER(CAST(argument AS CHAR)) LIKE 'UPDATE%' OR
                        UPPER(CAST(argument AS CHAR)) LIKE 'DELETE%'
                    )
                    AND event_time > '{}'
                ORDER BY event_time ASC
                LIMIT 100
            "#, last_time)
        } else {
            // First run - use timestamp with buffer
            let check_start_time = SystemTime::now();
            let timestamp = check_start_time
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                .saturating_sub(5); // Subtract 5 seconds as buffer for first run
            
            debug!("First run - using timestamp: {}", timestamp);
            format!(r#"
                SELECT 
                    CAST(argument AS CHAR) as query,
                    CAST(event_time AS CHAR) as event_time
                FROM mysql.general_log
                WHERE 
                    command_type = 'Query'
                    AND (
                        UPPER(CAST(argument AS CHAR)) LIKE 'INSERT%' OR
                        UPPER(CAST(argument AS CHAR)) LIKE 'UPDATE%' OR
                        UPPER(CAST(argument AS CHAR)) LIKE 'DELETE%'
                    )
                    AND event_time >= FROM_UNIXTIME({})
                ORDER BY event_time ASC
                LIMIT 100
            "#, timestamp)
        };

        match sqlx::query(&query)
            .fetch_all(&self.mysql_pool)
            .await
        {
            Ok(rows) => {
                if rows.is_empty() {
                    // No new queries detected - this is normal, only log at debug level
                    debug!("No new queries detected in general_log");
                } else {
                    info!("Found {} new queries in general_log", rows.len());
                    let mut latest_event_time: Option<String> = None;
                    
                    for (idx, row) in rows.iter().enumerate() {
                        info!("Processing row {} of {}", idx + 1, rows.len());
                        
                        // Extract event_time first to track it - try multiple methods
                        let event_time_str: Option<String> = {
                            // Try by index 1 first (event_time column)
                            if let Ok(et) = row.try_get::<String, _>(1) {
                                Some(et)
                            } else if let Ok(Some(et)) = row.try_get::<Option<String>, _>(1) {
                                Some(et)
                            } else if let Ok(et) = row.try_get::<String, _>("event_time") {
                                Some(et)
                            } else if let Ok(Some(et)) = row.try_get::<Option<String>, _>("event_time") {
                                Some(et)
                            } else {
                                // Try to get as chrono::NaiveDateTime and convert to string
                                // This handles MySQL DATETIME type
                                None
                            }
                        };
                        
                        if let Some(ref et) = event_time_str {
                            // Update latest_event_time if this one is newer (string comparison works for MySQL datetime)
                            let should_update = match &latest_event_time {
                                None => true,
                                Some(existing) => et > existing,
                            };
                            if should_update {
                                latest_event_time = Some(et.clone());
                            }
                            info!("  Event time: {}", et);
                        } else {
                            warn!("⚠️ Could not extract event_time from row {} - event tracking may be incomplete", idx);
                        }
                        
                        // Extract query text - the column is CAST as CHAR, so it should be String
                        let query_text: Option<String> = {
                            // Try by index 0 (the query column)
                            if let Ok(q) = row.try_get::<String, _>(0) {
                                Some(q)
                            } else if let Ok(Some(q)) = row.try_get::<Option<String>, _>(0) {
                                Some(q)
                            } else if let Ok(q) = row.try_get::<String, _>("query") {
                                Some(q)
                            } else if let Ok(Some(q)) = row.try_get::<Option<String>, _>("query") {
                                Some(q)
                            } else {
                                // Try as bytes and convert
                                if let Ok(bytes) = row.try_get::<Vec<u8>, _>(0) {
                                    String::from_utf8(bytes).ok()
                                } else {
                                    None
                                }
                            }
                        };
                        
                        if let Some(query) = query_text {
                            let query_trimmed = query.trim();
                            if !query_trimmed.is_empty() {
                                info!("Processing query from general_log: {}", query_trimmed);
                                if let Err(e) = self.parse_and_send_query(query_trimmed).await {
                                    error!("Failed to parse query: {} - Error: {}", query_trimmed, e);
                                } else {
                                    info!("✓ Query parsed and event sent successfully");
                                }
                            } else {
                                warn!("Query text is empty for row {}", idx);
                            }
                        } else {
                            error!("Could not extract query text from row {}", idx);
                        }
                    }
                    
                    // Update last_processed_event_time to the most recent event we processed
                    if let Some(latest_time) = latest_event_time {
                        self.last_processed_event_time = Some(latest_time.clone());
                        info!("✓ Updated last_processed_event_time to: {}", latest_time);
                    } else {
                        warn!("⚠️ No event_time extracted from rows - cannot track processed events");
                    }
                    
                    // Also update last_check_time
                    self.last_check_time = SystemTime::now();
                }
                // If rows.is_empty(), we already updated last_check_time in the if branch above
            }
            Err(e) => {
                // General log might not be accessible
                let error_msg = e.to_string();
                if error_msg.contains("Access denied") || error_msg.contains("denied") {
                    error!("Cannot access mysql.general_log - requires SUPER privilege");
                    error!("Please run: GRANT SUPER ON *.* TO 'your_user'@'%';");
                    error!("Or enable general_log manually: SET GLOBAL general_log = 'ON'; SET GLOBAL log_output = 'TABLE';");
                } else if error_msg.contains("doesn't exist") || error_msg.contains("Unknown table") {
                    error!("mysql.general_log table doesn't exist or general_log is disabled");
                    error!("Enable it with: SET GLOBAL general_log = 'ON'; SET GLOBAL log_output = 'TABLE';");
                } else {
                    warn!("Error querying general_log: {}", e);
                }
                
                // Try alternative: monitor specific tables using updated_at timestamps
                if let Err(e) = self.detect_table_changes().await {
                    warn!("Table-based change detection also failed: {}", e);
                }
            }
        }

        Ok(())
    }

    async fn detect_table_changes(&self) -> Result<()> {
        // Alternative: Query INFORMATION_SCHEMA to get all tables and check for recent changes
        // This is a fallback when general_log is not available
        info!("Attempting table-based change detection...");
        
        // Get list of all tables in the database
        let tables_query = format!(
            "SELECT TABLE_NAME FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_SCHEMA = '{}'",
            self.database
        );
        
        match sqlx::query(&tables_query)
            .fetch_all(&self.mysql_pool)
            .await
        {
            Ok(rows) => {
                info!("Found {} tables to monitor", rows.len());
                for row in rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        // Check if table has updated_at or created_at column
                        let check_query = format!(
                            "SELECT COLUMN_NAME FROM INFORMATION_SCHEMA.COLUMNS 
                             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' 
                             AND COLUMN_NAME IN ('updated_at', 'created_at', 'modified_at')",
                            self.database, table_name
                        );
                        
                        if let Ok(col_rows) = sqlx::query(&check_query)
                            .fetch_all(&self.mysql_pool)
                            .await
                        {
                            if !col_rows.is_empty() {
                                // Table has timestamp column - could monitor it
                                tracing::debug!("Table {} has timestamp columns for monitoring", table_name);
                            }
                        }
                    }
                }
                warn!("Table-based monitoring is limited - general_log access is recommended for full functionality");
            }
            Err(e) => {
                warn!("Could not get table list: {}", e);
            }
        }
        
        Ok(())
    }

    async fn parse_and_send_query(&self, query: &str) -> Result<()> {
        let query_upper = query.to_uppercase().trim().to_string();
        
        info!("Parsing query: {}", query);
        
        if query_upper.starts_with("INSERT") {
            info!("Detected INSERT query type");
            self.parse_insert(query).await
        } else if query_upper.starts_with("UPDATE") {
            info!("Detected UPDATE query type");
            self.parse_update(query).await
        } else if query_upper.starts_with("DELETE") {
            info!("Detected DELETE query type");
            self.parse_delete(query).await
        } else {
            info!("Query type not recognized, skipping: {}", query);
            Ok(())
        }
    }

    async fn parse_insert(&self, query: &str) -> Result<()> {
        // Parse INSERT INTO table (cols) VALUES (vals)
        info!("Parsing INSERT query: {}", query);
        
        // Extract table name
        let query_upper = query.to_uppercase();
        if let Some(table_start) = query_upper.find("INSERT INTO") {
            let rest = &query[table_start + 11..].trim_start();
            
            // Handle backticks: `table_name` or table_name
            let table = if rest.starts_with('`') {
                // Find the closing backtick
                if let Some(end) = rest[1..].find('`') {
                    rest[1..end + 1].to_string()
                } else {
                    // No closing backtick, take until space or (
                    rest[1..].find(|c: char| c == ' ' || c == '(')
                        .map(|i| rest[1..i + 1].to_string())
                        .unwrap_or_else(|| rest[1..].to_string())
                }
            } else {
                // No backtick, take until space or (
                let end = rest.find(|c: char| c == ' ' || c == '(' || c == '\n' || c == '\t')
                    .unwrap_or(rest.len());
                rest[..end].trim().to_string()
            };
            
            let table = table.trim_matches('`').trim_matches('"').trim_matches('\'').to_string();
            
            if table.is_empty() {
                error!("Failed to extract table name from query: {}", query);
                return Err(anyhow::anyhow!("Empty table name extracted from query"));
            }
            
            info!("Extracted table name: {}", table);
            
            // Try to extract column names and values
            // Look for pattern: INSERT INTO table (col1, col2) VALUES (val1, val2)
            if let Some(values_start) = query_upper.find("VALUES") {
                let values_part = &query[values_start + 6..];
                
                // Extract column names (between first parentheses)
                let cols_start = rest.find('(');
                let cols_end = rest.find(')');
                
                let mut columns = Vec::new();
                if let (Some(start), Some(end)) = (cols_start, cols_end) {
                    let cols_str = &rest[start + 1..end];
                    columns = cols_str.split(',')
                        .map(|c| c.trim().trim_matches('`').trim_matches('"').trim_matches('\'').to_string())
                        .collect();
                    info!("Extracted columns: {:?}", columns);
                }
                
                // Extract values (between VALUES and next parentheses)
                if let Some(val_start) = values_part.find('(') {
                    if let Some(val_end) = values_part[val_start + 1..].find(')') {
                        let vals_str = &values_part[val_start + 1..val_start + 1 + val_end];
                        let values: Vec<String> = vals_str.split(',')
                            .map(|v| v.trim().trim_matches('\'').trim_matches('"').to_string())
                            .collect();
                        
                        info!("Extracted values: {:?}", values);
                        
                        // Match columns with values
                        let mut column_values = Vec::new();
                        for (i, col) in columns.iter().enumerate() {
                            if i < values.len() {
                                column_values.push((col.clone(), values[i].clone()));
                            }
                        }
                        
                        let values_count = column_values.len();
                        info!("Matched {} column-value pairs", values_count);
                        
                        let table_for_log = table.clone();
                        let event = BinlogEventType::Insert {
                            table,
                            values: column_values,
                        };
                        
                        info!("Enqueuing INSERT event: table={}, values_count={}", table_for_log, values_count);
                        match self.event_tx.try_send(event) {
                            Ok(_) => {
                                info!("✓ INSERT event enqueued successfully");
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                warn!("⚠️ Event queue is full! INSERT event dropped. Consider increasing queue size or fixing PostgreSQL writer.");
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                error!("Event channel closed! Writer may have stopped.");
                                return Err(anyhow::anyhow!("Event channel closed"));
                            }
                        }
                    }
                }
            }
        } else {
            warn!("Could not find 'INSERT INTO' in query: {}", query);
        }
        
        Ok(())
    }

    async fn parse_update(&self, query: &str) -> Result<()> {
        info!("Parsing UPDATE query: {}", query);
        
        let query_upper = query.to_uppercase();
        if let Some(table_start) = query_upper.find("UPDATE") {
            let rest = &query[table_start + 6..].trim_start();
            
            // Handle backticks: `table_name` or table_name
            let table = if rest.starts_with('`') {
                // Find the closing backtick
                if let Some(end) = rest[1..].find('`') {
                    rest[1..end + 1].to_string()
                } else {
                    // No closing backtick, take until space
                    rest[1..].split_whitespace().next().unwrap_or("").to_string()
                }
            } else {
                // No backtick, take until space or SET
                let end = rest.find(|c: char| c == ' ' || c == '\n' || c == '\t' || c == '\r')
                    .or_else(|| rest.to_uppercase().find("SET").map(|i| i - 1))
                    .unwrap_or(rest.len());
                rest[..end].trim().to_string()
            };
            
            let table = table.trim_matches('`').trim_matches('"').trim_matches('\'').to_string();
            
            if table.is_empty() {
                error!("Failed to extract table name from query: {}", query);
                return Err(anyhow::anyhow!("Empty table name extracted from query"));
            }
            
            info!("Extracted table name: {}", table);
            
            // Extract SET clause values
            // Pattern: UPDATE table SET col1=val1, col2=val2 WHERE ...
            if let Some(set_start) = query_upper.find("SET") {
                let set_part = &query[set_start + 3..];
                let where_start = set_part.to_uppercase().find("WHERE");
                let set_end = where_start.unwrap_or(set_part.len());
                let set_clause = &set_part[..set_end];
                
                let mut new_values = Vec::new();
                for assignment in set_clause.split(',') {
                    let assignment = assignment.trim();
                    if let Some(eq_pos) = assignment.find('=') {
                        let col = assignment[..eq_pos].trim().trim_matches('`').trim_matches('"').trim_matches('\'').to_string();
                        let val = assignment[eq_pos + 1..].trim().trim_matches('\'').trim_matches('"').to_string();
                        new_values.push((col, val));
                    }
                }
                
                info!("Extracted {} SET values: {:?}", new_values.len(), new_values);
                
                // Extract WHERE clause for primary key
                if let Some(where_start) = set_part.to_uppercase().find("WHERE") {
                    let where_clause = &set_part[where_start + 5..];
                    // Try to extract WHERE col=val
                    // Handle cases like: `table`.`col` = val or `col` = val or col = val
                    if let Some(eq_pos) = where_clause.find('=') {
                        let mut pk_col = where_clause[..eq_pos].trim().to_string();
                        // Remove table prefix if present (e.g., `table`.`col` -> `col`)
                        if let Some(dot_pos) = pk_col.rfind('.') {
                            pk_col = pk_col[dot_pos + 1..].to_string();
                        }
                        // Remove backticks, quotes
                        pk_col = pk_col.trim_matches('`').trim_matches('"').trim_matches('\'').trim().to_string();
                        let pk_val = where_clause[eq_pos + 1..].trim().trim_matches('\'').trim_matches('"').trim_matches('`').to_string();
                        info!("Extracted WHERE clause: {} = {}", pk_col, pk_val);
                        new_values.push((pk_col, pk_val));
                    }
                }
                
                let values_count = new_values.len();
                let table_for_log = table.clone();
                let event = BinlogEventType::Update {
                    table,
                    old_values: vec![],
                    new_values,
                };
                
                        info!("Enqueuing UPDATE event: table={}, values_count={}", table_for_log, values_count);
                        // Non-blocking send - if queue is full, log warning but don't fail
                        match self.event_tx.try_send(event) {
                            Ok(_) => {
                                info!("✓ UPDATE event enqueued successfully");
                            }
                            Err(mpsc::error::TrySendError::Full(_)) => {
                                warn!("⚠️ Event queue is full! UPDATE event dropped. Consider increasing queue size or fixing PostgreSQL writer.");
                            }
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                error!("Event channel closed! Writer may have stopped.");
                                return Err(anyhow::anyhow!("Event channel closed"));
                            }
                        }
            }
        }
        
        Ok(())
    }

    async fn parse_delete(&self, query: &str) -> Result<()> {
        info!("Parsing DELETE query: {}", query);
        
        let query_upper = query.to_uppercase();
        if let Some(table_start) = query_upper.find("DELETE FROM") {
            let rest = &query[table_start + 11..].trim_start();
            
            // Handle backticks: `table_name` or table_name
            let table = if rest.starts_with('`') {
                // Find the closing backtick
                if let Some(end) = rest[1..].find('`') {
                    rest[1..end + 1].to_string()
                } else {
                    // No closing backtick, take until space
                    rest[1..].split_whitespace().next().unwrap_or("").to_string()
                }
            } else {
                // No backtick, take until space or WHERE
                let end = rest.find(|c: char| c == ' ' || c == '\n' || c == '\t' || c == '\r')
                    .or_else(|| rest.to_uppercase().find("WHERE").map(|i| i - 1))
                    .unwrap_or(rest.len());
                rest[..end].trim().to_string()
            };
            
            let table = table.trim_matches('`').trim_matches('"').trim_matches('\'').to_string();
            
            if table.is_empty() {
                error!("Failed to extract table name from query: {}", query);
                return Err(anyhow::anyhow!("Empty table name extracted from query"));
            }
            
            info!("Extracted table name: {}", table);
            
            // Extract WHERE clause
            let mut values = Vec::new();
            if let Some(where_start) = rest.to_uppercase().find("WHERE") {
                let where_clause = &rest[where_start + 5..];
                // Try to extract WHERE col=val
                // Handle cases like: `table`.`col` = val or `col` = val or col = val
                if let Some(eq_pos) = where_clause.find('=') {
                    let mut col = where_clause[..eq_pos].trim().to_string();
                    // Remove table prefix if present (e.g., `table`.`col` -> `col`)
                    if let Some(dot_pos) = col.rfind('.') {
                        col = col[dot_pos + 1..].to_string();
                    }
                    // Remove backticks, quotes
                    col = col.trim_matches('`').trim_matches('"').trim_matches('\'').trim().to_string();
                    let val = where_clause[eq_pos + 1..].trim().trim_matches('\'').trim_matches('"').trim_matches('`').to_string();
                    info!("Extracted WHERE clause: {} = {}", col, val);
                    values.push((col, val));
                }
            }
            
            let values_count = values.len();
            let table_for_log = table.clone();
            let event = BinlogEventType::Delete {
                table,
                values,
            };
            
            info!("Enqueuing DELETE event: table={}, values_count={}", table_for_log, values_count);
            match self.event_tx.try_send(event) {
                Ok(_) => {
                    info!("✓ DELETE event enqueued successfully");
                }
                Err(mpsc::error::TrySendError::Full(_)) => {
                    warn!("⚠️ Event queue is full! DELETE event dropped. Consider increasing queue size or fixing PostgreSQL writer.");
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    error!("Event channel closed! Writer may have stopped.");
                    return Err(anyhow::anyhow!("Event channel closed"));
                }
            }
        }
        
        Ok(())
    }
}

