use sqlx::{PgPool, postgres::PgPoolOptions, Row};
use crate::error::AppError;
use tracing::{info, error, warn};
use std::time::Duration;
use tokio::time::timeout;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub connection_timeout_secs: u64,
    pub statement_cache_capacity: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            max_connections: 50,  // Increased for high-throughput DeFi operations
            min_connections: 10,  // Higher minimum for consistent performance
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 1800,
            connection_timeout_secs: 10,
            statement_cache_capacity: 1000,  // Cache prepared statements
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ConnectionPoolStats {
    pub size: u32,
    pub idle: u32,
    pub active: u32,
    pub max_connections: u32,
    pub min_connections: u32,
}

pub async fn establish_connection(database_url: &str) -> Result<PgPool, AppError> {
    establish_connection_with_config(database_url, DatabaseConfig::default()).await
}

pub async fn establish_connection_with_config(
    database_url: &str,
    config: DatabaseConfig,
) -> Result<PgPool, AppError> {
    info!("Establishing database connection with config: {:?}", config);
    
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
        .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
        .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
        .test_before_acquire(true)  // Test connections before use
        .connect(database_url)
        .await
        .map_err(|e| {
            error!("Failed to connect to database: {}", e);
            AppError::DatabaseError(format!("Connection failed: {}", e))
        })?;

    // Warm up the connection pool
    if let Err(e) = warm_up_pool(&pool).await {
        warn!("Failed to warm up connection pool: {}", e);
    }

    info!("Database connection established successfully with {} max connections", config.max_connections);
    Ok(pool)
}

pub async fn test_connection(pool: &PgPool) -> Result<(), AppError> {
    let test_timeout = Duration::from_secs(5);
    
    timeout(test_timeout, async {
        sqlx::query("SELECT 1 as test_value")
            .fetch_one(pool)
            .await
    })
    .await
    .map_err(|_| AppError::DatabaseError("Connection test timed out".to_string()))?
    .map_err(|e| AppError::DatabaseError(format!("Connection test failed: {}", e)))?;
    
    info!("Database connection test successful");
    Ok(())
}

/// Perform comprehensive database health check
pub async fn health_check(pool: &PgPool) -> Result<DatabaseHealthStatus, AppError> {
    let start_time = std::time::Instant::now();
    
    // Test basic connectivity
    test_connection(pool).await?;
    
    // Check database version and settings
    let version_row = sqlx::query("SELECT version() as version")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get database version: {}", e)))?;
    
    let version: String = version_row.get("version");
    
    // Check connection pool stats
    let pool_stats = get_pool_stats(pool);
    
    // Test query performance
    let query_start = std::time::Instant::now();
    sqlx::query("SELECT COUNT(*) as count FROM information_schema.tables WHERE table_schema = 'public'")
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Performance test failed: {}", e)))?;
    let query_duration = query_start.elapsed();
    
    let total_duration = start_time.elapsed();
    
    Ok(DatabaseHealthStatus {
        is_healthy: true,
        version,
        pool_stats,
        response_time_ms: total_duration.as_millis() as u64,
        query_performance_ms: query_duration.as_millis() as u64,
        timestamp: chrono::Utc::now(),
    })
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealthStatus {
    pub is_healthy: bool,
    pub version: String,
    pub pool_stats: ConnectionPoolStats,
    pub response_time_ms: u64,
    pub query_performance_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Get connection pool statistics
pub fn get_pool_stats(pool: &PgPool) -> ConnectionPoolStats {
    ConnectionPoolStats {
        size: pool.size(),
        idle: pool.num_idle() as u32,
        active: pool.size() - (pool.num_idle() as u32),
        max_connections: pool.options().get_max_connections(),
        min_connections: pool.options().get_min_connections(),
    }
}

/// Warm up the connection pool by establishing minimum connections
async fn warm_up_pool(pool: &PgPool) -> Result<(), AppError> {
    info!("Warming up connection pool");
    
    let min_connections = pool.options().get_min_connections();
    let mut handles = Vec::new();
    
    // Create concurrent connections up to minimum
    for i in 0..min_connections {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            match sqlx::query("SELECT 1").execute(&pool_clone).await {
                Ok(_) => {
                    info!("Warmed up connection {}/{}", i + 1, min_connections);
                    Ok(())
                }
                Err(e) => {
                    error!("Failed to warm up connection {}: {}", i + 1, e);
                    Err(e)
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all connections to be established
    let mut success_count = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => success_count += 1,
            Ok(Err(e)) => warn!("Connection warm-up failed: {}", e),
            Err(e) => warn!("Connection warm-up task failed: {}", e),
        }
    }
    
    info!("Connection pool warmed up: {}/{} connections established", success_count, min_connections);
    Ok(())
}

/// Execute query with automatic retry logic
pub async fn execute_with_retry<F, T>(
    pool: &PgPool,
    operation: F,
    max_retries: u32,
) -> Result<T, AppError>
where
    F: Fn(&PgPool) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, sqlx::Error>> + Send + '_>>,
{
    let mut last_error = None;
    
    for attempt in 0..=max_retries {
        match operation(pool).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    let delay = Duration::from_millis(100 * (2_u64.pow(attempt)));
                    warn!("Database operation failed (attempt {}/{}), retrying in {:?}: {}", 
                          attempt + 1, max_retries + 1, delay, last_error.as_ref().unwrap());
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
    
    Err(AppError::DatabaseError(format!(
        "Database operation failed after {} attempts: {}",
        max_retries + 1,
        last_error.unwrap()
    )))
}
