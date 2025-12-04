use crate::schema::types::{Column, ForeignKey, Index, MySQLSchema, TableSchema};
use anyhow::Result;
use sqlx::{MySql, Pool, Row};

pub struct MySQLReader {
    pool: Pool<MySql>,
    database: String,
}

impl MySQLReader {
    pub fn new(pool: Pool<MySql>, database: String) -> Self {
        Self { pool, database }
    }

    pub async fn build_schema(&self) -> Result<MySQLSchema> {
        let mut schema = MySQLSchema::new();
        let tables = self.fetch_tables().await?;

        for table_name in tables {
            let columns = self.fetch_columns(&table_name).await?;
            let primary_keys = self.fetch_primary_keys(&table_name).await?;
            let foreign_keys = self.fetch_foreign_keys(&table_name).await?;
            let indexes = self.fetch_indexes(&table_name).await?;

            let table_schema = TableSchema {
                name: table_name.clone(),
                columns,
                primary_keys,
                foreign_keys,
                indexes,
            };

            schema.add_table(table_schema);
        }

        Ok(schema)
    }

    pub async fn fetch_tables(&self) -> Result<Vec<String>> {
        let query = r#"
            SELECT TABLE_NAME
            FROM INFORMATION_SCHEMA.TABLES
            WHERE TABLE_SCHEMA = ?
            AND TABLE_TYPE = 'BASE TABLE'
            ORDER BY TABLE_NAME
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|row| row.get::<String, _>(0))
            .collect())
    }

    pub async fn fetch_columns(&self, table_name: &str) -> Result<Vec<Column>> {
        let query = r#"
            SELECT
                COLUMN_NAME,
                DATA_TYPE,
                IS_NULLABLE,
                COLUMN_DEFAULT,
                EXTRA,
                CHARACTER_MAXIMUM_LENGTH,
                NUMERIC_PRECISION,
                NUMERIC_SCALE
            FROM INFORMATION_SCHEMA.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let extra: String = row.get(4);
            let is_auto_increment = extra.contains("auto_increment");

            let default_value: Option<String> = row.get(3);

            // Handle CHARACTER_MAXIMUM_LENGTH which can be BIGINT UNSIGNED
            let character_maximum_length: Option<u64> = row.try_get(5).ok().flatten();

            // Handle NUMERIC_PRECISION and NUMERIC_SCALE
            let numeric_precision: Option<u32> = row.try_get(6).ok().flatten();
            let numeric_scale: Option<u32> = row.try_get(7).ok().flatten();

            columns.push(Column {
                name: row.get(0),
                data_type: row.get::<String, _>(1).to_lowercase(),
                is_nullable: row.get::<String, _>(2) == "YES",
                default_value,
                is_auto_increment,
                character_maximum_length,
                numeric_precision,
                numeric_scale,
            });
        }

        Ok(columns)
    }

    pub async fn fetch_primary_keys(&self, table_name: &str) -> Result<Vec<String>> {
        let query = r#"
            SELECT COLUMN_NAME
            FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
            WHERE TABLE_SCHEMA = ?
            AND TABLE_NAME = ?
            AND CONSTRAINT_NAME = 'PRIMARY'
            ORDER BY ORDINAL_POSITION
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .iter()
            .map(|row| row.get::<String, _>(0))
            .collect())
    }

    pub async fn fetch_foreign_keys(&self, table_name: &str) -> Result<Vec<ForeignKey>> {
        let query = r#"
            SELECT
                CONSTRAINT_NAME,
                COLUMN_NAME,
                REFERENCED_TABLE_NAME,
                REFERENCED_COLUMN_NAME
            FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
            WHERE TABLE_SCHEMA = ?
            AND TABLE_NAME = ?
            AND REFERENCED_TABLE_NAME IS NOT NULL
            ORDER BY CONSTRAINT_NAME, ORDINAL_POSITION
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        let mut foreign_keys = Vec::new();
        for row in rows {
            foreign_keys.push(ForeignKey {
                name: row.get(0),
                column_name: row.get(1),
                referenced_table: row.get(2),
                referenced_column: row.get(3),
            });
        }

        Ok(foreign_keys)
    }

    pub async fn fetch_indexes(&self, table_name: &str) -> Result<Vec<Index>> {
        let query = r#"
            SELECT
                INDEX_NAME,
                COLUMN_NAME,
                NON_UNIQUE
            FROM INFORMATION_SCHEMA.STATISTICS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            AND INDEX_NAME != 'PRIMARY'
            ORDER BY INDEX_NAME, SEQ_IN_INDEX
        "#;

        let rows = sqlx::query(query)
            .bind(&self.database)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        let mut index_map: std::collections::HashMap<String, Index> = std::collections::HashMap::new();

        for row in rows {
            let index_name: String = row.get(0);
            let column_name: String = row.get(1);
            // NON_UNIQUE is TINYINT UNSIGNED, handle it properly
            let non_unique: u8 = row.try_get(2).unwrap_or(1);
            let is_unique = non_unique == 0;

            index_map
                .entry(index_name.clone())
                .or_insert_with(|| Index {
                    name: index_name,
                    columns: Vec::new(),
                    is_unique,
                })
                .columns
                .push(column_name);
        }

        Ok(index_map.into_values().collect())
    }
}

