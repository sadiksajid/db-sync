use anyhow::Result;
use tracing::{debug, info};

use super::routines::{MySQLFunction, MySQLProcedure, MySQLTrigger, MySQLView};

pub struct RoutineConverter;

impl RoutineConverter {
    /// Convert MySQL view to PostgreSQL view
    pub fn convert_view(view: &MySQLView) -> Result<String> {
        info!("Converting view: {}", view.name);

        let mut pg_definition = view.definition.clone();

        // Remove schema qualifiers (e.g., testing.customers -> customers)
        pg_definition = Self::remove_schema_qualifiers(&pg_definition);

        // Basic syntax conversions
        pg_definition = Self::convert_sql_syntax(&pg_definition);

        // Create the view statement
        let mut sql = format!("CREATE OR REPLACE VIEW \"{}\" AS\n{}", view.name, pg_definition);

        // Add check option if present
        if let Some(ref check_option) = view.check_option {
            if check_option.to_uppercase() != "NONE" {
                sql.push_str(&format!("\nWITH {} CHECK OPTION", check_option.to_uppercase()));
            }
        }

        sql.push(';');

        debug!("Converted view: {}", view.name);
        Ok(sql)
    }

    /// Convert MySQL function to PostgreSQL function
    pub fn convert_function(func: &MySQLFunction) -> Result<String> {
        info!("Converting function: {}", func.name);

        let mut pg_definition = func.definition.clone();

        // Extract parameters BEFORE removing syntax
        let function_signature = Self::extract_function_params_from_create(&pg_definition, &func.name);

        // Remove MySQL-specific syntax
        pg_definition = Self::remove_mysql_function_syntax(&pg_definition);

        // Convert SQL syntax
        pg_definition = Self::convert_sql_syntax(&pg_definition);

        // Convert return type
        let pg_return_type = Self::convert_data_type(&func.returns);

        // Build PostgreSQL function
        let immutability = if func.is_deterministic {
            "IMMUTABLE"
        } else {
            "VOLATILE"
        };

        let sql = format!(
            "CREATE OR REPLACE FUNCTION \"{}\"{}
RETURNS {} AS $$
{}
$$ LANGUAGE plpgsql {};",
            func.name,
            function_signature,
            pg_return_type,
            pg_definition,
            immutability
        );

        debug!("Converted function: {}", func.name);
        Ok(sql)
    }

    /// Convert MySQL procedure to PostgreSQL function (PostgreSQL doesn't have procedures in older versions)
    pub fn convert_procedure(proc: &MySQLProcedure) -> Result<String> {
        info!("Converting procedure: {}", proc.name);

        let mut pg_definition = proc.definition.clone();

        // Extract parameters BEFORE removing syntax
        let procedure_signature = Self::extract_procedure_params_from_create(&pg_definition, &proc.name);

        // Remove MySQL-specific syntax
        pg_definition = Self::remove_mysql_procedure_syntax(&pg_definition);

        // Convert SQL syntax
        pg_definition = Self::convert_sql_syntax(&pg_definition);

        // PostgreSQL 11+ has procedures, but we'll use functions for compatibility
        let sql = format!(
            "CREATE OR REPLACE FUNCTION \"{}\"{}
RETURNS void AS $$
{}
$$ LANGUAGE plpgsql;",
            proc.name,
            procedure_signature,
            pg_definition
        );

        debug!("Converted procedure: {}", proc.name);
        Ok(sql)
    }

    /// Convert MySQL trigger to PostgreSQL trigger
    pub fn convert_trigger(trigger: &MySQLTrigger) -> Result<String> {
        info!("Converting trigger: {}", trigger.name);

        let mut pg_statement = trigger.action_statement.clone();

        // Convert SQL syntax
        pg_statement = Self::convert_sql_syntax(&pg_statement);
        
        // Convert SET NEW.x = y to NEW.x := y
        pg_statement = pg_statement.replace("SET NEW.", "NEW.").replace(" = ", " := ");

        // Remove BEGIN/END if present (we'll add them back)
        pg_statement = pg_statement.trim().to_string();
        if pg_statement.to_uppercase().starts_with("BEGIN") {
            pg_statement = pg_statement[5..].trim().to_string();
        }
        if pg_statement.to_uppercase().ends_with("END") {
            pg_statement = pg_statement[..pg_statement.len() - 3].trim().to_string();
        }
        
        // Ensure all statements end with semicolon
        if !pg_statement.trim().ends_with(';') {
            pg_statement.push(';');
        }

        // Create trigger function
        let function_name = format!("{}_func", trigger.name);
        let trigger_function = format!(
            "CREATE OR REPLACE FUNCTION \"{}\"()
RETURNS TRIGGER AS $$
BEGIN
    {}
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;",
            function_name, pg_statement
        );

        // Create trigger
        let trigger_sql = format!(
            "CREATE TRIGGER \"{}\"
{} {} ON \"{}\"
FOR EACH ROW
EXECUTE FUNCTION \"{}\"();",
            trigger.name,
            trigger.action_timing.to_uppercase(),
            trigger.event_manipulation.to_uppercase(),
            trigger.event_object_table,
            function_name
        );

        let sql = format!("{}\n\n{}", trigger_function, trigger_sql);

        debug!("Converted trigger: {}", trigger.name);
        Ok(sql)
    }

    /// Convert MySQL SQL syntax to PostgreSQL
    fn convert_sql_syntax(sql: &str) -> String {
        let mut result = sql.to_string();

        // Remove backticks, replace with double quotes for identifiers
        result = result.replace('`', "\"");

        // Convert common MySQL functions to PostgreSQL equivalents
        result = Self::convert_functions(&result);

        // Convert data types
        result = Self::convert_inline_data_types(&result);

        // Convert variable declarations (DECLARE vs PostgreSQL style)
        result = Self::convert_variable_declarations(&result);

        result
    }

    /// Convert MySQL functions to PostgreSQL equivalents
    fn convert_functions(sql: &str) -> String {
        let mut result = sql.to_string();

        // Common function conversions
        result = result.replace("IFNULL(", "COALESCE(");
        result = result.replace("NOW()", "CURRENT_TIMESTAMP");
        result = result.replace("CURDATE()", "CURRENT_DATE");
        result = result.replace("CURTIME()", "CURRENT_TIME");
        result = result.replace("UNIX_TIMESTAMP(", "EXTRACT(EPOCH FROM ");
        
        // String functions
        result = result.replace("CONCAT_WS(", "CONCAT(");
        
        // Note: IF() function is more complex and may need manual review

        result
    }

    /// Convert inline data type references
    fn convert_inline_data_types(sql: &str) -> String {
        let mut result = sql.to_string();

        // Common type conversions
        result = result.replace("TINYINT", "SMALLINT");
        result = result.replace("INT ", "INTEGER ");
        result = result.replace("DATETIME", "TIMESTAMP");
        result = result.replace("TEXT", "TEXT");

        result
    }

    /// Convert data type from MySQL to PostgreSQL
    fn convert_data_type(mysql_type: &str) -> String {
        let type_upper = mysql_type.to_uppercase();

        match type_upper.as_str() {
            t if t.contains("INT") => "INTEGER".to_string(),
            t if t.contains("VARCHAR") => mysql_type.to_string(), // Keep length
            t if t.contains("CHAR") => mysql_type.to_string(),
            "TEXT" | "LONGTEXT" | "MEDIUMTEXT" | "TINYTEXT" => "TEXT".to_string(),
            "DATETIME" | "TIMESTAMP" => "TIMESTAMP".to_string(),
            "DATE" => "DATE".to_string(),
            "DECIMAL" => mysql_type.to_string(), // Keep precision
            "FLOAT" | "DOUBLE" => "DOUBLE PRECISION".to_string(),
            "BOOLEAN" | "BOOL" => "BOOLEAN".to_string(),
            _ => mysql_type.to_string(),
        }
    }

    /// Convert variable declarations
    fn convert_variable_declarations(sql: &str) -> String {
        // This is a simplified conversion
        // MySQL: DECLARE var_name data_type [DEFAULT value];
        // PostgreSQL: Same syntax works in PL/pgSQL
        sql.to_string()
    }

    /// Remove MySQL-specific function syntax
    fn remove_mysql_function_syntax(definition: &str) -> String {
        let mut result = definition.to_string();

        // Remove CREATE FUNCTION statement if present (we'll recreate it)
        if result.to_uppercase().contains("CREATE FUNCTION") {
            if let Some(begin_pos) = result.to_uppercase().find("BEGIN") {
                result = result[begin_pos..].to_string();
            }
        }

        // Remove DETERMINISTIC, NO SQL, etc. keywords (we handle them separately)
        result = result.replace("DETERMINISTIC", "");
        result = result.replace("NO SQL", "");
        result = result.replace("READS SQL DATA", "");
        result = result.replace("MODIFIES SQL DATA", "");
        result = result.replace("CONTAINS SQL", "");

        result.trim().to_string()
    }

    /// Remove MySQL-specific procedure syntax
    fn remove_mysql_procedure_syntax(definition: &str) -> String {
        let mut result = definition.to_string();

        // Remove CREATE PROCEDURE statement if present
        if result.to_uppercase().contains("CREATE PROCEDURE") {
            if let Some(begin_pos) = result.to_uppercase().find("BEGIN") {
                result = result[begin_pos..].to_string();
            }
        }

        result.trim().to_string()
    }

    /// Remove schema qualifiers from SQL (e.g., database.table -> table)
    fn remove_schema_qualifiers(sql: &str) -> String {
        use regex::Regex;
        
        // Pattern to match: `schema`.`table` or schema.table or "schema"."table"
        let re = Regex::new(r#"(`[^`]+`\.)|("[^"]+"\.)|([\w]+\.)"#).unwrap();
        let result = re.replace_all(sql, "");
        result.to_string()
    }

    /// Extract function parameters from CREATE FUNCTION statement
    fn extract_function_params_from_create(definition: &str, name: &str) -> String {
        // Look for FUNCTION name(params) or FUNCTION `name`(params)
        let pattern = format!(r"(?i)FUNCTION\s+(`?{}`?)\s*\(([^)]*)\)", regex::escape(name));
        
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(caps) = re.captures(definition) {
                if let Some(params) = caps.get(2) {
                    let params_str = params.as_str().trim();
                    if !params_str.is_empty() {
                        // Convert parameter types
                        let converted_params = Self::convert_parameter_list(params_str);
                        return format!("({})", converted_params);
                    }
                }
            }
        }

        // Default: no parameters
        "()".to_string()
    }

    /// Extract procedure parameters from CREATE PROCEDURE statement
    fn extract_procedure_params_from_create(definition: &str, name: &str) -> String {
        // Look for PROCEDURE name(params) or PROCEDURE `name`(params)
        let pattern = format!(r"(?i)PROCEDURE\s+(`?{}`?)\s*\(([^)]*)\)", regex::escape(name));
        
        if let Ok(re) = regex::Regex::new(&pattern) {
            if let Some(caps) = re.captures(definition) {
                if let Some(params) = caps.get(2) {
                    let params_str = params.as_str().trim();
                    if !params_str.is_empty() {
                        // Convert parameter types
                        let converted_params = Self::convert_parameter_list(params_str);
                        return format!("({})", converted_params);
                    }
                }
            }
        }

        // Default: no parameters
        "()".to_string()
    }

    /// Convert MySQL parameter list to PostgreSQL format
    fn convert_parameter_list(params: &str) -> String {
        let mut result = Vec::new();
        
        // Split by comma, but be careful with commas inside types like DECIMAL(10,2)
        let parts = Self::split_parameters(params);
        
        for part in parts {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            
            // Remove IN/OUT/INOUT keywords
            let part = part.replace("IN ", "").replace("OUT ", "").replace("INOUT ", "");
            
            // Split into name and type
            let tokens: Vec<&str> = part.split_whitespace().collect();
            if tokens.len() >= 2 {
                let param_name = tokens[0].trim_matches('`');
                let param_type = tokens[1..].join(" ");
                let pg_type = Self::convert_data_type(&param_type);
                result.push(format!("{} {}", param_name, pg_type));
            }
        }
        
        result.join(", ")
    }

    /// Split parameters by comma, respecting parentheses
    fn split_parameters(params: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0;
        
        for ch in params.chars() {
            match ch {
                '(' => {
                    paren_depth += 1;
                    current.push(ch);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(ch);
                }
                ',' if paren_depth == 0 => {
                    if !current.trim().is_empty() {
                        result.push(current.trim().to_string());
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }
        
        if !current.trim().is_empty() {
            result.push(current.trim().to_string());
        }
        
        result
    }

    /// Extract function signature (parameters) from definition
    fn extract_function_signature(definition: &str, _name: &str) -> String {
        // Try to find parameters in the definition
        // This is a simplified extraction - may need enhancement
        
        // Look for pattern: function_name(params)
        if let Some(start) = definition.find('(') {
            if let Some(end) = definition[start..].find(')') {
                let params = &definition[start..start + end + 1];
                return params.to_string();
            }
        }

        // Default: no parameters
        "()".to_string()
    }

    /// Extract procedure signature (parameters) from definition
    fn extract_procedure_signature(definition: &str, _name: &str) -> String {
        // Similar to function signature extraction
        Self::extract_function_signature(definition, _name)
    }
}

