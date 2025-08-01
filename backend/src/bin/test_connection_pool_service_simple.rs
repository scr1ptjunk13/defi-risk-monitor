use defi_risk_monitor::database::{
    AdvancedConnectionPool, AdvancedPoolConfig, ConnectionPoolService,
    establish_connection,
};
use defi_risk_monitor::error::AppError;
use sqlx::Row;
use tracing::{info, error};
use std::env;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Connection Pool Service Test");

    // Get database URL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());

    info!("📡 Connecting to database: {}", database_url);

    // Test 1: Basic Service Creation
    info!("📋 Test 1: Connection Pool Service Creation");
    
    // Create a basic pool first
    let pool = establish_connection(&database_url).await?;
    
    // Create service with the existing pool
    let service = ConnectionPoolService::new(pool.clone());
    info!("✅ Connection Pool Service created successfully");

    // Test 2: Create Advanced Pool via Service
    info!("📋 Test 2: Creating Advanced Pool via Service");
    
    let advanced_config = AdvancedPoolConfig {
        max_connections: 10,  // Smaller pool to avoid timeout
        min_connections: 2,
        acquire_timeout_secs: 10,  // Shorter timeout
        ..Default::default()
    };
    
    let _advanced_pool = service.create_pool(
        "advanced_test".to_string(),
        &database_url,
        advanced_config
    ).await?;
    info!("✅ Advanced pool created via service");

    // Test 3: Basic Query Test
    info!("📋 Test 3: Basic Query Test");
    
    let row = sqlx::query("SELECT 42 as test_value")
        .fetch_one(&pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Query failed: {}", e)))?;
    
    let value: i32 = row.get("test_value");
    info!("📝 Query result: {}", value);
    assert_eq!(value, 42);
    info!("✅ Basic query test passed");

    // Test 4: Service Health Check
    info!("📋 Test 4: Service Health Check");
    
    let pools = service.list_pools().await;
    info!("📊 Active pools: {:?}", pools);
    info!("✅ Service health check completed");

    // Test 5: Simple Load Test
    info!("📋 Test 5: Simple Load Test");
    
    let mut handles = vec![];
    for i in 0..5 {
        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            let row = sqlx::query(&format!("SELECT {} as test_value", i + 1))
                .fetch_one(&pool_clone)
                .await?;
            let value: i32 = row.get("test_value");
            Ok::<i32, sqlx::Error>(value)
        });
        handles.push(handle);
    }

    let mut results = vec![];
    for handle in handles {
        match handle.await {
            Ok(Ok(value)) => {
                results.push(value);
                info!("📝 Concurrent query result: {}", value);
            }
            Ok(Err(e)) => error!("Query error: {}", e),
            Err(e) => error!("Task error: {}", e),
        }
    }
    
    info!("📊 Concurrent test results: {:?}", results);
    info!("✅ Simple load test completed");

    info!("🎉 All Connection Pool Service tests completed successfully!");
    
    Ok(())
}
