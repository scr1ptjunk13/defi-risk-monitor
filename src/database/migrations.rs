use sqlx::{PgPool, migrate::MigrateDatabase, Postgres};
use crate::error::AppError;
use tracing::{info, error};

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    info!("Running database migrations");
    
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| {
            error!("Migration failed: {}", e);
            AppError::DatabaseError(format!("Migration failed: {}", e))
        })?;
    
    info!("Database migrations completed successfully");
    Ok(())
}

pub async fn create_database_if_not_exists(database_url: &str) -> Result<(), AppError> {
    if !Postgres::database_exists(database_url).await.unwrap_or(false) {
        info!("Database does not exist, creating it");
        Postgres::create_database(database_url)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to create database: {}", e)))?;
        info!("Database created successfully");
    } else {
        info!("Database already exists");
    }
    
    Ok(())
}
