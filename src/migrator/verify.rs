use crate::schema::types::MySQLSchema;
use anyhow::Result;
use sqlx::{MySql, PgPool, Pool};
use tracing::{info, warn};

pub struct Verifier {
    mysql_pool: Pool<MySql>,
    pg_pool: PgPool,
}

impl Verifier {
    pub fn new(mysql_pool: Pool<MySql>, pg_pool: PgPool) -> Self {
        Self {
            mysql_pool,
            pg_pool,
        }
    }

    pub async fn verify_schema(&self, schema: &MySQLSchema) -> Result<VerificationReport> {
        let mut report = VerificationReport {
            table_reports: Vec::new(),
            total_tables: schema.tables.len(),
            tables_match: 0,
            tables_mismatch: 0,
        };

        info!("Verifying schema for {} tables", schema.tables.len());

        for (table_name, _table) in &schema.tables {
            let table_report = self.verify_table(table_name).await?;
            
            if table_report.matches {
                report.tables_match += 1;
            } else {
                report.tables_mismatch += 1;
            }
            
            report.table_reports.push(table_report);
        }

        info!(
            "Verification complete: {} match, {} mismatch",
            report.tables_match, report.tables_mismatch
        );

        Ok(report)
    }

    async fn verify_table(&self, table_name: &str) -> Result<TableVerification> {
        // Get MySQL row count
        let mysql_count_query = format!("SELECT COUNT(*) FROM `{}`", table_name);
        let mysql_count: i64 = sqlx::query_scalar(&mysql_count_query)
            .fetch_one(&self.mysql_pool)
            .await?;

        // Get PostgreSQL row count
        let pg_count_query = format!("SELECT COUNT(*) FROM \"{}\"", table_name);
        let pg_count: i64 = match sqlx::query_scalar::<_, i64>(&pg_count_query)
            .fetch_one(&self.pg_pool)
            .await
        {
            Ok(count) => count,
            Err(e) => {
                warn!("Failed to get row count for table {} in PostgreSQL: {}", table_name, e);
                return Ok(TableVerification {
                    table_name: table_name.to_string(),
                    mysql_count,
                    pg_count: -1,
                    matches: false,
                    error: Some(format!("PostgreSQL query failed: {}", e)),
                });
            }
        };

        let matches = mysql_count == pg_count;

        if !matches {
            warn!(
                "Row count mismatch for table {}: MySQL={}, PostgreSQL={}",
                table_name, mysql_count, pg_count
            );
        } else {
            info!("Table {} verified: {} rows match", table_name, mysql_count);
        }

        Ok(TableVerification {
            table_name: table_name.to_string(),
            mysql_count,
            pg_count,
            matches,
            error: None,
        })
    }
}

#[derive(Debug)]
pub struct VerificationReport {
    pub table_reports: Vec<TableVerification>,
    pub total_tables: usize,
    pub tables_match: usize,
    pub tables_mismatch: usize,
}

#[derive(Debug)]
pub struct TableVerification {
    pub table_name: String,
    pub mysql_count: i64,
    pub pg_count: i64,
    pub matches: bool,
    pub error: Option<String>,
}

