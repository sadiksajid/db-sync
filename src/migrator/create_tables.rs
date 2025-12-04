use crate::schema::dependency::DependencyGraph;
use crate::schema::pg_converter::PGConverter;
use crate::schema::types::MySQLSchema;
use anyhow::Result;
use sqlx::PgPool;
use tracing::info;

pub struct TableCreator {
    pool: PgPool,
}

impl TableCreator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_all_tables(&self, schema: &MySQLSchema) -> Result<()> {
        let graph = DependencyGraph::from_schema(schema);
        let table_order = graph.get_creation_order()?;

        info!("Creating {} tables in order: {:?}", table_order.len(), table_order);

        // First, create tables without foreign key constraints
        for table_name in &table_order {
            if let Some(table) = schema.get_table(table_name) {
                let create_sql = PGConverter::generate_create_table(table)?;
                
                // Split SQL into statements
                let statements: Vec<&str> = create_sql
                    .split(';')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                // Execute CREATE TABLE first
                if let Some(create_stmt) = statements.first() {
                    info!("Creating table: {}", table_name);
                    sqlx::query(create_stmt).execute(&self.pool).await?;
                }

                // Execute index creation
                for stmt in statements.iter().skip(1) {
                    if stmt.starts_with("CREATE") {
                        sqlx::query(stmt).execute(&self.pool).await?;
                    }
                }
            }
        }

        // Then add foreign key constraints
        for table_name in &table_order {
            if let Some(table) = schema.get_table(table_name) {
                if !table.foreign_keys.is_empty() {
                    let create_sql = PGConverter::generate_create_table(table)?;
                    let statements: Vec<&str> = create_sql
                        .split(';')
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect();

                    for stmt in statements {
                        if stmt.starts_with("ALTER TABLE") {
                            info!("Adding foreign key constraint to table: {}", table_name);
                            if let Err(e) = sqlx::query(stmt).execute(&self.pool).await {
                                tracing::warn!("Failed to add FK constraint: {} - {}", stmt, e);
                            }
                        }
                    }
                }
            }
        }

        info!("All tables created successfully");
        Ok(())
    }
}

