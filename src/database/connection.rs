use sqlx::{PgPool, postgres::PgPoolOptions};
use crate::error::AppError;
use tracing::{info, error};
use std::time::Duration;

pub async fn establish_connection(database_url: &str) -> Result<PgPool, AppError> {
    info!("Establishing database connection");
    
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .map_err(|e| {
            error!("Failed to connect to database: {}", e);
            AppError::DatabaseError(format!("Connection failed: {}", e))
        })?;

    info!("Database connection established successfully");
    Ok(pool)
}

pub async fn test_connection(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Connection test failed: {}", e)))?;
    
    info!("Database connection test successful");
    Ok(())
}
