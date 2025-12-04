use crate::schema::types::{Column, TableSchema};
use anyhow::Result;

pub struct PGConverter;

impl PGConverter {
    pub fn convert_type(column: &Column) -> String {
        let mysql_type = column.data_type.as_str();
        let length = column.character_maximum_length;
        let precision = column.numeric_precision;
        let scale = column.numeric_scale;

        match mysql_type {
            "tinyint" => {
                if length == Some(1) {
                    "BOOLEAN".to_string()
                } else {
                    "SMALLINT".to_string()
                }
            }
            "smallint" => "SMALLINT".to_string(),
            "mediumint" => "INTEGER".to_string(),
            "int" | "integer" => "INTEGER".to_string(),
            "bigint" => "BIGINT".to_string(),
            "decimal" | "numeric" => {
                if let (Some(p), Some(s)) = (precision, scale) {
                    format!("NUMERIC({},{})", p, s)
                } else {
                    "NUMERIC".to_string()
                }
            }
            "float" => "REAL".to_string(),
            "double" => "DOUBLE PRECISION".to_string(),
            "char" => {
                if let Some(len) = length {
                    // Use the actual length from MySQL
                    format!("CHAR({})", len)
                } else {
                    // Default to CHAR(1) if no length specified (safer than 255)
                    "CHAR(1)".to_string()
                }
            }
            "varchar" => {
                if let Some(len) = length {
                    // Use the actual length from MySQL, but PostgreSQL has a max of 10,485,760
                    // For very large lengths, use TEXT instead
                    if len > 10485760 {
                        "TEXT".to_string()
                    } else {
                        format!("VARCHAR({})", len)
                    }
                } else {
                    // If no length specified, use TEXT for safety (MySQL VARCHAR without length can be very large)
                    "TEXT".to_string()
                }
            }
            "text" => "TEXT".to_string(),
            "tinytext" => "TEXT".to_string(),
            "mediumtext" => "TEXT".to_string(),
            "longtext" => "TEXT".to_string(),
            "blob" => "BYTEA".to_string(),
            "tinyblob" => "BYTEA".to_string(),
            "mediumblob" => "BYTEA".to_string(),
            "longblob" => "BYTEA".to_string(),
            "binary" => "BYTEA".to_string(),
            "varbinary" => "BYTEA".to_string(),
            "date" => "DATE".to_string(),
            "time" => "TIME".to_string(),
            "datetime" => "TIMESTAMP".to_string(),
            "timestamp" => "TIMESTAMP".to_string(),
            "year" => "INTEGER".to_string(),
            "json" => "JSONB".to_string(),
            "enum" => "VARCHAR(255)".to_string(),
            "set" => "VARCHAR(255)".to_string(),
            _ => {
                tracing::warn!("Unknown MySQL type: {}, defaulting to TEXT", mysql_type);
                "TEXT".to_string()
            }
        }
    }

    pub fn generate_create_table(table: &TableSchema) -> Result<String> {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS \"{}\" (\n", table.name);

        let mut column_defs = Vec::new();

        for column in &table.columns {
            let mut def = format!("  \"{}\" {}", column.name, Self::convert_type(column));

            // Handle auto-increment
            if column.is_auto_increment {
                // Check if it's part of primary key
                if table.primary_keys.len() == 1 && table.primary_keys[0] == column.name {
                    def = format!("  \"{}\" SERIAL", column.name);
                } else {
                    // Use IDENTITY for non-primary auto-increment
                    def = format!("  \"{}\" INTEGER GENERATED ALWAYS AS IDENTITY", column.name);
                }
            }

            // Handle nullable
            if !column.is_nullable && !column.is_auto_increment {
                def.push_str(" NOT NULL");
            }

            // Handle default values
            if let Some(ref default) = column.default_value {
                if default.to_uppercase() != "NULL" {
                    // Skip invalid MySQL dates in default values
                    let default_trimmed = default.trim();
                    if default_trimmed.contains("0000-00-00") || 
                       default_trimmed.starts_with("0000") ||
                       default_trimmed == "0000-00-00 00:00:00" {
                        tracing::warn!("Skipping invalid date default value for column {}: {}", column.name, default);
                        // Don't set any default - let it be NULL or use PostgreSQL's default
                    } else {
                        // Handle special MySQL defaults
                        let pg_default = match default.to_uppercase().as_str() {
                            "CURRENT_TIMESTAMP" => "CURRENT_TIMESTAMP".to_string(),
                            "NOW()" => "CURRENT_TIMESTAMP".to_string(),
                            _ => {
                                // Quote string defaults
                                if default.starts_with('\'') || default.parse::<f64>().is_ok() {
                                    default.clone()
                                } else {
                                    format!("'{}'", default)
                                }
                            }
                        };
                        def.push_str(&format!(" DEFAULT {}", pg_default));
                    }
                }
            }

            column_defs.push(def);
        }

        // Add primary key constraint
        if !table.primary_keys.is_empty() {
            let pk_cols = table
                .primary_keys
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");
            column_defs.push(format!("  PRIMARY KEY ({})", pk_cols));
        }

        sql.push_str(&column_defs.join(",\n"));
        sql.push_str("\n);\n");

        // Add indexes
        for index in &table.indexes {
            if index.is_unique {
                let cols = index
                    .columns
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                sql.push_str(&format!(
                    "CREATE UNIQUE INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});\n",
                    index.name, table.name, cols
                ));
            } else {
                let cols = index
                    .columns
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ");
                sql.push_str(&format!(
                    "CREATE INDEX IF NOT EXISTS \"{}\" ON \"{}\" ({});\n",
                    index.name, table.name, cols
                ));
            }
        }

        // Add foreign key constraints
        for fk in &table.foreign_keys {
            sql.push_str(&format!(
                "ALTER TABLE \"{}\" ADD CONSTRAINT \"{}\" FOREIGN KEY (\"{}\") REFERENCES \"{}\" (\"{}\");\n",
                table.name, fk.name, fk.column_name, fk.referenced_table, fk.referenced_column
            ));
        }

        Ok(sql)
    }
}

