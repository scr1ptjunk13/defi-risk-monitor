use sqlx::{PgPool, Row};
use crate::error::{AppError, retry::{with_retry, RetryConfig}};
use crate::retry_db_operation;
use tracing::{info, debug};
use uuid::Uuid;

/// Database operations wrapper with built-in retry logic
pub struct RetryableDatabase {
    pool: PgPool,
}

impl RetryableDatabase {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Execute a simple query with retry logic
    pub async fn execute_query_with_retry(&self, query: &str) -> Result<u64, AppError> {
        retry_db_operation!(
            "execute_query",
            {
                let result = sqlx::query(query)
                    .execute(&self.pool)
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Query execution failed: {}", e)))?;
                
                Ok(result.rows_affected())
            }
        )
    }

    /// Fetch a single row with retry logic
    pub async fn fetch_one_with_retry(&self, query: &str) -> Result<sqlx::postgres::PgRow, AppError> {
        retry_db_operation!(
            "fetch_one",
            {
                sqlx::query(query)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Fetch one failed: {}", e)))
            }
        )
    }

    /// Fetch multiple rows with retry logic
    pub async fn fetch_all_with_retry(&self, query: &str) -> Result<Vec<sqlx::postgres::PgRow>, AppError> {
        retry_db_operation!(
            "fetch_all",
            {
                sqlx::query(query)
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Fetch all failed: {}", e)))
            }
        )
    }

    /// Execute a transaction with retry logic
    pub async fn execute_transaction<F, T>(&self, operation: F) -> Result<T, AppError>
    where
        F: Fn(sqlx::Transaction<'_, sqlx::Postgres>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + '_>>,
    {
        with_retry(
            "database_transaction",
            RetryConfig::for_database(),
            || async {
                let tx = self.pool.begin()
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Failed to begin transaction: {}", e)))?;
                
                let result = operation(tx).await?;
                Ok(result)
            },
        ).await
    }

    /// Health check with retry logic
    pub async fn health_check_with_retry(&self) -> Result<bool, AppError> {
        retry_db_operation!(
            "health_check",
            {
                let row = sqlx::query("SELECT 1 as health")
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Health check failed: {}", e)))?;
                
                let health: i32 = row.get("health");
                Ok(health == 1)
            }
        )
    }

    /// Get database connection count with retry logic
    pub async fn get_connection_count_with_retry(&self) -> Result<i64, AppError> {
        retry_db_operation!(
            "get_connection_count",
            {
                let row = sqlx::query(
                    "SELECT count(*) as connection_count FROM pg_stat_activity WHERE state = 'active'"
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to get connection count: {}", e)))?;
                
                Ok(row.get::<i64, _>("connection_count"))
            }
        )
    }

    /// Insert a risk assessment with retry logic (example of complex operation)
    pub async fn insert_risk_assessment_with_retry(
        &self,
        user_id: Uuid,
        entity_type: &str,
        entity_id: &str,
        risk_type: &str,
        risk_score: f64,
        severity: &str,
    ) -> Result<Uuid, AppError> {
        retry_db_operation!(
            "insert_risk_assessment",
            {
                let risk_id = Uuid::new_v4();
                
                sqlx::query(
                    r#"
                    INSERT INTO risk_assessments (
                        id, user_id, entity_type, entity_id, risk_type, 
                        risk_score, severity, is_active, created_at, updated_at
                    ) VALUES ($1, $2, $3::text, $4, $5::text, $6, $7::text, true, NOW(), NOW())
                    "#
                )
                .bind(risk_id)
                .bind(user_id)
                .bind(entity_type)
                .bind(entity_id)
                .bind(risk_type)
                .bind(risk_score)
                .bind(severity)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to insert risk assessment: {}", e)))?;
                
                Ok(risk_id)
            }
        )
    }

    /// Update user preferences with retry logic and custom retry config
    pub async fn update_user_preferences_with_custom_retry(
        &self,
        user_id: Uuid,
        risk_tolerance: &str,
    ) -> Result<(), AppError> {
        // Custom retry config for user operations (more aggressive)
        let custom_config = RetryConfig {
            max_attempts: 5,
            base_delay_ms: 50,
            max_delay_ms: 2000,
            jitter_factor: 0.1,
            backoff_multiplier: 1.8,
        };

        with_retry(
            "update_user_preferences",
            custom_config,
            || async {
                sqlx::query(
                    "UPDATE users SET risk_tolerance = $1, updated_at = NOW() WHERE id = $2"
                )
                .bind(risk_tolerance)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to update user preferences: {}", e)))?;
                
                Ok(())
            },
        ).await
    }

    /// Bulk insert positions with transaction retry logic (simplified example)
    pub async fn bulk_insert_positions_with_retry(
        &self,
        positions: Vec<(String, String, String, i32, i32, i32, String, String)>,
    ) -> Result<u64, AppError> {
        with_retry(
            "bulk_insert_positions",
            RetryConfig::for_database(),
            || async {
                let mut tx = self.pool.begin()
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Failed to begin transaction: {}", e)))?;
                
                let mut total_inserted = 0u64;
                
                for (pool_address, token0, token1, fee_tier, tick_lower, tick_upper, liquidity, amount0) in &positions {
                    let result = sqlx::query(
                        r#"
                        INSERT INTO positions (
                            id, user_address, pool_address, token0_address, token1_address,
                            fee_tier, tick_lower, tick_upper, liquidity, token0_amount,
                            token1_amount, created_at, updated_at
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
                        "#
                    )
                    .bind(Uuid::new_v4())
                    .bind("0x1234567890123456789012345678901234567890") // placeholder user_address
                    .bind(pool_address)
                    .bind(token0)
                    .bind(token1)
                    .bind(fee_tier)
                    .bind(tick_lower)
                    .bind(tick_upper)
                    .bind(liquidity)
                    .bind(amount0)
                    .bind("0") // token1_amount placeholder
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Failed to insert position: {}", e)))?;
                    
                    total_inserted += result.rows_affected();
                }
                
                tx.commit()
                    .await
                    .map_err(|e| AppError::DatabaseError(format!("Failed to commit transaction: {}", e)))?;
                
                Ok(total_inserted)
            },
        ).await
    }

    /// Get database statistics with retry logic (complex query example)
    pub async fn get_database_stats_with_retry(&self) -> Result<DatabaseStats, AppError> {
        retry_db_operation!(
            "get_database_stats",
            {
                let row = sqlx::query(
                    r#"
                    SELECT 
                        pg_database_size(current_database()) as db_size,
                        (SELECT count(*) FROM pg_stat_activity WHERE state = 'active') as active_connections,
                        (SELECT count(*) FROM pg_stat_activity) as total_connections,
                        (SELECT sum(numbackends) FROM pg_stat_database WHERE datname = current_database()) as backends
                    "#
                )
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to get database stats: {}", e)))?;
                
                Ok(DatabaseStats {
                    database_size: row.get::<i64, _>("db_size"),
                    active_connections: row.get::<i64, _>("active_connections"),
                    total_connections: row.get::<i64, _>("total_connections"),
                    backends: row.get::<Option<i64>, _>("backends").unwrap_or(0),
                })
            }
        )
    }
}

/// Database statistics structure
#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub database_size: i64,
    pub active_connections: i64,
    pub total_connections: i64,
    pub backends: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::establish_connection;
    use std::env;

    async fn get_test_pool() -> PgPool {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
        establish_connection(&database_url).await.expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_health_check_with_retry() {
        let pool = get_test_pool().await;
        let retryable_db = RetryableDatabase::new(pool);
        
        let result = retryable_db.health_check_with_retry().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_connection_count_with_retry() {
        let pool = get_test_pool().await;
        let retryable_db = RetryableDatabase::new(pool);
        
        let result = retryable_db.get_connection_count_with_retry().await;
        assert!(result.is_ok());
        assert!(result.unwrap() >= 0);
    }

    #[tokio::test]
    async fn test_database_stats_with_retry() {
        let pool = get_test_pool().await;
        let retryable_db = RetryableDatabase::new(pool);
        
        let result = retryable_db.get_database_stats_with_retry().await;
        assert!(result.is_ok());
        
        let stats = result.unwrap();
        assert!(stats.database_size > 0);
        assert!(stats.active_connections >= 0);
        assert!(stats.total_connections >= 0);
    }

    #[tokio::test]
    async fn test_execute_query_with_retry() {
        let pool = get_test_pool().await;
        let retryable_db = RetryableDatabase::new(pool);
        
        // Test a simple query that should succeed
        let result = retryable_db.execute_query_with_retry("SELECT 1").await;
        assert!(result.is_ok());
    }
}
