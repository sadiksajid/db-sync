use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};
use tracing::{debug, info, warn};

use crate::schema::routines::{MySQLFunction, MySQLProcedure, MySQLTrigger, MySQLView};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    temperature: f32,
    top_k: u32,
    top_p: f32,
    max_output_tokens: u32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ResponseContent,
}

#[derive(Deserialize)]
struct ResponseContent {
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
struct ResponsePart {
    text: String,
}

pub struct GeminiConverter {
    api_key: String,
    model: String,
    client: reqwest::Client,
    last_call_time: Arc<Mutex<Option<Instant>>>,
    rate_limit_duration: Duration,
}

impl GeminiConverter {
    pub fn new(api_key: String) -> Self {
        // Get model from environment variable, default to gemini-2.0-flash-exp
        let model = std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-2.0-flash-exp".to_string());
        
        info!("✨ Gemini AI model: {}", model);
        info!("⏱️  Gemini API rate limit: 1 call per minute");
        
        Self {
            api_key,
            model,
            client: reqwest::Client::new(),
            last_call_time: Arc::new(Mutex::new(None)),
            rate_limit_duration: Duration::from_secs(60), // 1 minute between calls
        }
    }

    /// Check if Gemini API is available (API key is set)
    pub fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Ensure SQL is complete - fix common truncation issues
    fn ensure_complete_sql(sql: &str) -> String {
        let mut result = sql.to_string();
        
        // Count $$ delimiters - should be even (pairs)
        let dollar_count = result.matches("$$").count();
        if dollar_count % 2 == 1 {
            // Odd number - add closing $$
            result.push_str("\n$$");
        }
        
        // Check for unclosed BEGIN blocks
        let begin_count = result.to_uppercase().matches("BEGIN").count();
        let end_count = result.to_uppercase().matches("END;").count();
        
        if begin_count > end_count {
            // Missing END; statements
            for _ in 0..(begin_count - end_count) {
                result.push_str("\nEND;");
            }
        }
        
        // Ensure ends with semicolon
        if !result.trim().ends_with(';') {
            result.push(';');
        }
        
        result
    }

    /// Convert MySQL view to PostgreSQL using Gemini API
    pub async fn convert_view(&self, view: &MySQLView) -> Result<String> {
        info!("Using Gemini AI to convert view: {}", view.name);

        let prompt = format!(
            r#"Convert this MySQL VIEW to PostgreSQL-compatible syntax.

MySQL View Name: {}
MySQL Definition:
```sql
{}
```

Requirements:
1. Remove any schema qualifiers (e.g., database.table -> table)
2. Convert backticks to double quotes
3. Convert MySQL functions to PostgreSQL equivalents:
   - IFNULL() -> COALESCE()
   - NOW() -> CURRENT_TIMESTAMP
   - CONCAT() stays the same but verify syntax
4. Keep the view name as: "{}"
5. Ensure all SQL is valid PostgreSQL syntax

Output ONLY the PostgreSQL CREATE OR REPLACE VIEW statement, nothing else.
"#,
            view.name, view.definition, view.name
        );

        self.call_gemini_api(&prompt).await
    }

    /// Convert MySQL function to PostgreSQL using Gemini API
    pub async fn convert_function(&self, func: &MySQLFunction) -> Result<String> {
        info!("Using Gemini AI to convert function: {}", func.name);

        let prompt = format!(
            r#"Convert this MySQL FUNCTION to PostgreSQL PL/pgSQL function.

MySQL Function Name: {}
MySQL Definition:
```sql
{}
```

Requirements:
1. Extract parameters with correct types (INT -> INTEGER, DECIMAL -> NUMERIC, etc.)
2. Convert MySQL syntax to PostgreSQL PL/pgSQL:
   - DECLARE variables properly
   - Use PostgreSQL data types
   - DATEDIFF() -> date subtraction
   - IF statements stay the same in PL/pgSQL
3. Determine volatility: DETERMINISTIC -> IMMUTABLE, otherwise VOLATILE
4. Keep the function name as: "{}"
5. Use proper PostgreSQL function format:
   CREATE OR REPLACE FUNCTION "name"(params)
   RETURNS type AS $$
   BEGIN
     -- body
   END;
   $$ LANGUAGE plpgsql [IMMUTABLE|VOLATILE];

Output ONLY the PostgreSQL function definition, nothing else.
"#,
            func.name, func.definition, func.name
        );

        self.call_gemini_api(&prompt).await
    }

    /// Convert MySQL procedure to PostgreSQL function using Gemini API
    pub async fn convert_procedure(&self, proc: &MySQLProcedure) -> Result<String> {
        info!("Using Gemini AI to convert procedure: {}", proc.name);

        let prompt = format!(
            r#"Convert this MySQL PROCEDURE to PostgreSQL function (PostgreSQL doesn't have procedures in older versions).

MySQL Procedure Name: {}
MySQL Definition:
```sql
{}
```

Requirements:
1. Convert to a PostgreSQL function that RETURNS void
2. Extract parameters (IN, OUT, INOUT) - handle appropriately:
   - IN parameters: normal parameters
   - OUT parameters: may need to return a record type or use INOUT
   - INOUT parameters: use INOUT in PostgreSQL
3. Convert MySQL syntax to PostgreSQL PL/pgSQL:
   - Remove DELIMITER statements
   - Use PostgreSQL data types
   - LAST_INSERT_ID() -> use RETURNING clause or currval()
4. Keep the procedure name as: "{}"
5. Use format:
   CREATE OR REPLACE FUNCTION "name"(params)
   RETURNS void AS $$
   BEGIN
     -- body
   END;
   $$ LANGUAGE plpgsql;

Output ONLY the PostgreSQL function definition, nothing else.
"#,
            proc.name, proc.definition, proc.name
        );

        self.call_gemini_api(&prompt).await
    }

    /// Convert MySQL trigger to PostgreSQL trigger using Gemini API
    pub async fn convert_trigger(&self, trigger: &MySQLTrigger) -> Result<String> {
        info!("Using Gemini AI to convert trigger: {}", trigger.name);

        let prompt = format!(
            r#"Convert this MySQL TRIGGER to PostgreSQL trigger.

MySQL Trigger Name: {}
Event: {} {} on table {}
MySQL Action Statement:
```sql
{}
```

Requirements:
1. Create a trigger function first, then the trigger
2. Convert syntax:
   - SET NEW.col = val -> NEW.col := val
   - SET OLD.col = val -> OLD.col := val (read-only, should not be set)
   - Remove SIGNAL statements or convert to RAISE EXCEPTION
3. Function must RETURN TRIGGER and end with RETURN NEW (or OLD for DELETE)
4. Use format:
   CREATE OR REPLACE FUNCTION "{}_func"()
   RETURNS TRIGGER AS $$
   BEGIN
     -- converted action
     RETURN NEW;
   END;
   $$ LANGUAGE plpgsql;

   CREATE TRIGGER "{}"
   {} {} ON "{}"
   FOR EACH ROW
   EXECUTE FUNCTION "{}_func"();

Output ONLY the PostgreSQL trigger function and trigger statements, nothing else.
"#,
            trigger.name,
            trigger.action_timing,
            trigger.event_manipulation,
            trigger.event_object_table,
            trigger.action_statement,
            trigger.name,
            trigger.name,
            trigger.action_timing,
            trigger.event_manipulation,
            trigger.event_object_table,
            trigger.name
        );

        self.call_gemini_api(&prompt).await
    }

    /// Call Gemini API with a prompt (with rate limiting)
    async fn call_gemini_api(&self, prompt: &str) -> Result<String> {
        // Rate limiting: ensure at least 60 seconds between API calls
        let mut last_call = self.last_call_time.lock().await;
        
        if let Some(last_time) = *last_call {
            let elapsed = last_time.elapsed();
            if elapsed < self.rate_limit_duration {
                let wait_time = self.rate_limit_duration - elapsed;
                info!("⏳ Rate limiting: waiting {:?} before next Gemini API call...", wait_time);
                tokio::time::sleep(wait_time).await;
            }
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.model
        );

        let request = GeminiRequest {
            contents: vec![Content {
                parts: vec![Part {
                    text: prompt.to_string(),
                }],
            }],
            generation_config: GenerationConfig {
                temperature: 0.1, // Low temperature for more deterministic output
                top_k: 1,
                top_p: 0.95,
                max_output_tokens: 8192, // Increased to handle longer functions/procedures
            },
        };

        debug!("Calling Gemini API with model: {}", self.model);

        // Retry logic for 503 errors (model overloaded)
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 1..=max_retries {
            let response = self
                .client
                .post(&url)
                .header("x-goog-api-key", &self.api_key)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await?;

            let status = response.status();

            // Update last call time after successful call
            *last_call = Some(Instant::now());

            if status.is_success() {
                let gemini_response: GeminiResponse = response.json().await?;

                if gemini_response.candidates.is_empty() {
                    return Err(anyhow::anyhow!("Gemini API returned no candidates"));
                }

                let text = &gemini_response.candidates[0].content.parts[0].text;
                
                // Clean up the response - remove markdown code blocks if present
                let mut cleaned = text
                    .trim()
                    .trim_start_matches("```sql")
                    .trim_start_matches("```postgresql")
                    .trim_start_matches("```")
                    .trim_end_matches("```")
                    .trim()
                    .to_string();

                // Ensure the SQL is complete - fix common truncation issues
                cleaned = Self::ensure_complete_sql(&cleaned);

                debug!("Gemini API response received ({} chars)", cleaned.len());
                return Ok(cleaned);
            } else if status == 503 {
                // Service unavailable / model overloaded - retry
                let error_text = response.text().await?;
                last_error = Some(format!("{} - {}", status, error_text));
                
                if attempt < max_retries {
                    let retry_delay = Duration::from_secs(10 * attempt as u64); // 10s, 20s, 30s
                    warn!("Gemini API overloaded (attempt {}/{}), retrying in {:?}...", attempt, max_retries, retry_delay);
                    tokio::time::sleep(retry_delay).await;
                    continue;
                } else {
                    warn!("Gemini API error after {} attempts: {}", max_retries, error_text);
                }
            } else {
                // Other error - don't retry
                let error_text = response.text().await?;
                warn!("Gemini API error: {} - {}", status, error_text);
                return Err(anyhow::anyhow!("Gemini API error: {} - {}", status, error_text));
            }
        }

        // All retries failed
        Err(anyhow::anyhow!("Gemini API error: {}", last_error.unwrap_or_else(|| "Unknown error".to_string())))
    }
}

