use crate::schema::types::{Column, ForeignKey, Index, DatabaseSchema, TableSchema};
use anyhow::Result;
use sqlx::{Pool, Postgres, Row};

pub struct PgReader {
    pool: Pool<Postgres>,
    database: String,
}

impl PgReader {
    pub fn new(pool: Pool<Postgres>, database: String) -> Self {
        Self { pool, database }
    }

    pub async fn build_schema(&self) -> Result<DatabaseSchema> {
        let mut schema = DatabaseSchema::new();
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
            SELECT table_name
            FROM information_schema.tables
            WHERE table_schema = 'public'
            AND table_type = 'BASE TABLE'
            ORDER BY table_name
        "#;

        let rows = sqlx::query(query)
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
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                c.character_maximum_length,
                c.numeric_precision,
                c.numeric_scale
            FROM information_schema.columns c
            WHERE c.table_schema = 'public' AND c.table_name = $1
            ORDER BY c.ordinal_position
        "#;

        let rows = sqlx::query(query)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let default_value: Option<String> = row.get(3);
            
            // Check if it's auto-increment (serial/bigserial)
            let is_auto_increment = default_value
                .as_ref()
                .map(|v| v.starts_with("nextval"))
                .unwrap_or(false);

            let character_maximum_length: Option<i32> = row.get(4);
            let character_maximum_length_u64 = character_maximum_length.map(|v| v as u64);

            let numeric_precision: Option<i32> = row.get(5);
            let numeric_precision_u32 = numeric_precision.map(|v| v as u32);

            let numeric_scale: Option<i32> = row.get(6);
            let numeric_scale_u32 = numeric_scale.map(|v| v as u32);

            columns.push(Column {
                name: row.get(0),
                data_type: row.get::<String, _>(1).to_lowercase(),
                is_nullable: row.get::<String, _>(2) == "YES",
                default_value,
                is_auto_increment,
                character_maximum_length: character_maximum_length_u64,
                numeric_precision: numeric_precision_u32,
                numeric_scale: numeric_scale_u32,
            });
        }

        Ok(columns)
    }

    pub async fn fetch_primary_keys(&self, table_name: &str) -> Result<Vec<String>> {
        let query = r#"
            SELECT a.attname
            FROM pg_index i
            JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
            WHERE i.indrelid = $1::regclass AND i.indisprimary
            ORDER BY a.attnum
        "#;

        let rows = sqlx::query(query)
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
                tc.constraint_name,
                kcu.column_name,
                ccu.table_name AS referenced_table_name,
                ccu.column_name AS referenced_column_name
            FROM information_schema.table_constraints AS tc
            JOIN information_schema.key_column_usage AS kcu
              ON tc.constraint_name = kcu.constraint_name
              AND tc.table_schema = kcu.table_schema
            JOIN information_schema.constraint_column_usage AS ccu
              ON ccu.constraint_name = tc.constraint_name
              AND ccu.table_schema = tc.table_schema
            WHERE tc.constraint_type = 'FOREIGN KEY'
              AND tc.table_schema = 'public'
              AND tc.table_name = $1
            ORDER BY tc.constraint_name
        "#;

        let rows = sqlx::query(query)
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
                i.relname AS index_name,
                a.attname AS column_name,
                ix.indisunique AS is_unique
            FROM pg_class t
            JOIN pg_index ix ON t.oid = ix.indrelid
            JOIN pg_class i ON i.oid = ix.indexrelid
            JOIN pg_attribute a ON a.attrelid = t.oid AND a.attnum = ANY(ix.indkey)
            WHERE t.relkind = 'r'
              AND t.relname = $1
              AND NOT ix.indisprimary
            ORDER BY i.relname, a.attnum
        "#;

        let rows = sqlx::query(query)
            .bind(table_name)
            .fetch_all(&self.pool)
            .await?;

        let mut index_map: std::collections::HashMap<String, Index> = std::collections::HashMap::new();

        for row in rows {
            let index_name: String = row.get(0);
            let column_name: String = row.get(1);
            let is_unique: bool = row.get(2);

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

