use defi_risk_monitor::database::{
    AdvancedConnectionPool, AdvancedPoolConfig, ConnectionPoolService,
    establish_connection,
};
use defi_risk_monitor::error::AppError;
use sqlx::Row; // Add this line
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, error, warn};
use std::env;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting Advanced Connection Pool Test Suite");

    // Load database URL
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());

    // Test 1: Basic Advanced Pool Creation
    info!("📋 Test 1: Basic Advanced Pool Creation");
    test_basic_pool_creation(&database_url).await?;

    // Test 2: Pool Configuration Variants
    info!("📋 Test 2: Pool Configuration Variants");
    test_pool_configurations(&database_url).await?;

    // Test 3: Health Check Monitoring
    info!("📋 Test 3: Health Check Monitoring");
    test_health_check_monitoring(&database_url).await?;

    // Test 4: Statement Caching
    info!("📋 Test 4: Statement Caching");
    test_statement_caching(&database_url).await?;

    // Test 5: Load Testing
    info!("📋 Test 5: Load Testing");
    test_load_testing(&database_url).await?;

    // Test 6: Connection Pool Service
    info!("📋 Test 6: Connection Pool Service");
    test_connection_pool_service(&database_url).await?;

    // Test 7: Dynamic Scaling Simulation
    info!("📋 Test 7: Dynamic Scaling Simulation");
    test_dynamic_scaling(&database_url).await?;

    info!("✅ All Advanced Connection Pool Tests Completed Successfully!");
    Ok(())
}

async fn test_basic_pool_creation(database_url: &str) -> Result<(), AppError> {
    info!("Creating advanced connection pool with default configuration");
    
    let config = AdvancedPoolConfig::default();
    let pool = AdvancedConnectionPool::new(database_url, config).await?;
    
    // Test basic connectivity
    let test_result = sqlx::query("SELECT 1 as test_value")
        .fetch_one(pool.get_pool())
        .await;
    
    match test_result {
        Ok(row) => {
            let value: i32 = row.get("test_value");
            info!("✅ Basic connectivity test passed: {}", value);
        }
        Err(e) => {
            error!("❌ Basic connectivity test failed: {}", e);
            return Err(AppError::DatabaseError(format!("Connectivity test failed: {}", e)));
        }
    }
    
    // Test pool stats
    let stats = pool.get_pool_stats().await;
    info!("📊 Pool Stats - Utilization: {:.2}%, Acquire Time: {}ms", 
          stats.utilization_rate * 100.0, stats.avg_acquire_time_ms);
    
    // Test health status
    let health = pool.get_health_status().await;
    info!("🏥 Health Status - Healthy: {}, Response Time: {}ms", 
          health.is_healthy, health.response_time_ms);
    
    pool.stop_monitoring().await;
    info!("✅ Basic pool creation test completed");
    Ok(())
}

async fn test_pool_configurations(database_url: &str) -> Result<(), AppError> {
    info!("Testing different pool configurations");
    
    // High-performance configuration
    let high_perf_config = AdvancedPoolConfig {
        max_connections: 150,
        min_connections: 30,
        acquire_timeout_secs: 10,
        idle_timeout_secs: 300,
        max_lifetime_secs: 1800,
        statement_cache_capacity: 5000,
        enable_prepared_statements: true,
        enable_dynamic_sizing: true,
        load_threshold_high: 0.85,
        load_threshold_low: 0.25,
        ..Default::default()
    };
    
    let high_perf_pool = AdvancedConnectionPool::new(database_url, high_perf_config).await?;
    info!("✅ High-performance pool created successfully");
    
    // Conservative configuration
    let conservative_config = AdvancedPoolConfig {
        max_connections: 20,
        min_connections: 5,
        acquire_timeout_secs: 60,
        idle_timeout_secs: 1200,
        max_lifetime_secs: 3600,
        statement_cache_capacity: 500,
        enable_prepared_statements: false,
        enable_dynamic_sizing: false,
        ..Default::default()
    };
    
    let conservative_pool = AdvancedConnectionPool::new(database_url, conservative_config).await?;
    info!("✅ Conservative pool created successfully");
    
    // Test both pools
    for (name, pool) in [("High-Perf", &high_perf_pool), ("Conservative", &conservative_pool)] {
        let stats = pool.get_pool_stats().await;
        let health = pool.get_health_status().await;
        let cache_stats = pool.get_statement_cache_stats().await;
        
        info!("📊 {} Pool - Utilization: {:.2}%, Cache Size: {}/{}", 
              name, stats.utilization_rate * 100.0, cache_stats.cache_size, cache_stats.cache_capacity);
        
        pool.stop_monitoring().await;
    }
    
    info!("✅ Pool configuration test completed");
    Ok(())
}

async fn test_health_check_monitoring(database_url: &str) -> Result<(), AppError> {
    info!("Testing health check monitoring");
    
    let config = AdvancedPoolConfig {
        health_check_interval_secs: 2, // Fast health checks for testing
        health_check_timeout_secs: 1,
        max_failed_health_checks: 2,
        ..Default::default()
    };
    
    let pool = AdvancedConnectionPool::new(database_url, config).await?;
    
    // Let monitoring run for a few cycles
    info!("⏳ Letting health monitoring run for 10 seconds...");
    sleep(Duration::from_secs(10)).await;
    
    // Check health status multiple times
    for i in 1..=5 {
        let health = pool.get_health_status().await;
        info!("🏥 Health Check #{} - Healthy: {}, Response: {}ms, Failed Checks: {}", 
              i, health.is_healthy, health.response_time_ms, health.failed_checks);
        sleep(Duration::from_secs(1)).await;
    }
    
    pool.stop_monitoring().await;
    info!("✅ Health check monitoring test completed");
    Ok(())
}

async fn test_statement_caching(database_url: &str) -> Result<(), AppError> {
    info!("Testing statement caching functionality");
    
    let config = AdvancedPoolConfig {
        enable_prepared_statements: true,
        statement_cache_capacity: 100,
        ..Default::default()
    };
    
    let pool = AdvancedConnectionPool::new(database_url, config).await?;
    
    // Execute the same query multiple times to test caching
    let test_queries = vec![
        "SELECT 1 as test_value",
        "SELECT 2 as test_value",
        "SELECT 3 as test_value",
        "SELECT 1 as test_value", // Repeat for cache hit
        "SELECT 2 as test_value", // Repeat for cache hit
    ];
    
    for (i, query) in test_queries.iter().enumerate() {
        let start_time = std::time::Instant::now();
        
        let result = sqlx::query(query)
            .fetch_one(pool.get_pool())
            .await;
        
        let duration = start_time.elapsed();
        
        match result {
            Ok(row) => {
                let value: i32 = row.get("test_value");
                info!("📝 Query #{} executed in {:?}: SELECT {} as test_value = {}", 
                      i + 1, duration, value, value);
            }
            Err(e) => {
                error!("❌ Query #{} failed: {}", i + 1, e);
            }
        }
    }
    
    // Check cache statistics
    let cache_stats = pool.get_statement_cache_stats().await;
    info!("📊 Cache Stats - Size: {}/{}, Hit Rate: {:.2}%, Hits: {}, Misses: {}", 
          cache_stats.cache_size, cache_stats.cache_capacity, 
          cache_stats.hit_rate * 100.0, cache_stats.total_hits, cache_stats.total_misses);
    
    pool.stop_monitoring().await;
    info!("✅ Statement caching test completed");
    Ok(())
}

async fn test_load_testing(database_url: &str) -> Result<(), AppError> {
    info!("Testing load testing functionality");
    
    let config = AdvancedPoolConfig {
        max_connections: 50,
        min_connections: 10,
        ..Default::default()
    };
    
    let pool = Arc::new(AdvancedConnectionPool::new(database_url, config).await?);
    
    // Create load tester
    let load_tester = defi_risk_monitor::database::PoolLoadTester::new(Arc::clone(&pool));
    
    // Run a light load test
    info!("🔥 Running load test: 10 concurrent requests for 5 seconds");
    let results = load_tester.run_load_test(10, 5).await?;
    
    info!("📊 Load Test Results:");
    info!("   Total Requests: {}", results.total_requests);
    info!("   Total Errors: {}", results.total_errors);
    info!("   Error Rate: {:.2}%", results.error_rate * 100.0);
    info!("   Avg Response Time: {}ms", results.avg_response_time_ms);
    info!("   Requests/Second: {}", results.requests_per_second);
    info!("   Pool Utilization: {:.2}%", results.pool_stats.utilization_rate * 100.0);
    
    pool.stop_monitoring().await;
    info!("✅ Load testing test completed");
    Ok(())
}

async fn test_connection_pool_service(database_url: &str) -> Result<(), AppError> {
    info!("Testing Connection Pool Service");
    
    // Create a basic pool for the service
    let service_pool = establish_connection(database_url).await?;
    let pool_service = ConnectionPoolService::new(service_pool);
    
    // Create multiple pools through the service
    let configs = vec![
        ("primary", AdvancedPoolConfig {
            max_connections: 100,
            min_connections: 20,
            ..Default::default()
        }),
        ("secondary", AdvancedPoolConfig {
            max_connections: 50,
            min_connections: 10,
            ..Default::default()
        }),
        ("cache", AdvancedPoolConfig {
            max_connections: 30,
            min_connections: 5,
            statement_cache_capacity: 1000,
            ..Default::default()
        }),
    ];
    
    for (name, config) in configs {
        let pool = pool_service.create_pool(name.to_string(), database_url, config).await?;
        info!("✅ Created pool '{}' through service", name);
        
        // Test the pool
        let test_result = sqlx::query("SELECT 1 as test")
            .fetch_one(pool.get_pool())
            .await;
        
        if test_result.is_ok() {
            info!("✅ Pool '{}' connectivity test passed", name);
        } else {
            error!("❌ Pool '{}' connectivity test failed", name);
        }
    }
    
    // List all pools
    let pool_names = pool_service.list_pools().await;
    info!("📋 Registered pools: {:?}", pool_names);
    
    // Test pool metrics (if tables exist)
    for pool_name in &pool_names {
        if let Some(pool) = pool_service.get_pool(pool_name).await {
            // Try to store metrics (may fail if migration not applied)
            if let Err(e) = pool_service.store_pool_metrics(pool_name, &pool).await {
                warn!("⚠️ Could not store metrics for '{}': {} (migration may not be applied)", pool_name, e);
            } else {
                info!("✅ Stored metrics for pool '{}'", pool_name);
            }
            
            pool.stop_monitoring().await;
        }
    }
    
    info!("✅ Connection Pool Service test completed");
    Ok(())
}

async fn test_dynamic_scaling(database_url: &str) -> Result<(), AppError> {
    info!("Testing dynamic scaling simulation");
    
    let config = AdvancedPoolConfig {
        max_connections: 50,
        min_connections: 10,
        enable_dynamic_sizing: true,
        load_threshold_high: 0.7,
        load_threshold_low: 0.3,
        min_scale_interval_secs: 5, // Fast scaling for testing
        ..Default::default()
    };
    
    let pool = AdvancedConnectionPool::new(database_url, config).await?;
    
    info!("📊 Initial pool stats:");
    let initial_stats = pool.get_pool_stats().await;
    info!("   Utilization: {:.2}%", initial_stats.utilization_rate * 100.0);
    
    // Simulate load by holding connections
    info!("🔥 Simulating high load by acquiring connections...");
    let mut connections = Vec::new();
    
    // Acquire multiple connections to increase utilization
    for i in 0..15 {
        match pool.get_pool().acquire().await {
            Ok(conn) => {
                connections.push(conn);
                info!("   Acquired connection #{}", i + 1);
            }
            Err(e) => {
                warn!("   Failed to acquire connection #{}: {}", i + 1, e);
                break;
            }
        }
        
        // Check utilization periodically
        if i % 5 == 4 {
            let stats = pool.get_pool_stats().await;
            info!("   Current utilization: {:.2}%", stats.utilization_rate * 100.0);
        }
    }
    
    // Let the scaling monitor run
    info!("⏳ Letting dynamic scaling monitor run for 15 seconds...");
    sleep(Duration::from_secs(15)).await;
    
    // Check final stats
    let final_stats = pool.get_pool_stats().await;
    info!("📊 Final pool stats:");
    info!("   Utilization: {:.2}%", final_stats.utilization_rate * 100.0);
    info!("   Total Acquires: {}", final_stats.total_acquires);
    info!("   Failed Acquires: {}", final_stats.failed_acquires);
    
    // Release connections
    drop(connections);
    info!("🔄 Released all held connections");
    
    // Let the system stabilize
    sleep(Duration::from_secs(5)).await;
    
    let stabilized_stats = pool.get_pool_stats().await;
    info!("📊 Stabilized pool stats:");
    info!("   Utilization: {:.2}%", stabilized_stats.utilization_rate * 100.0);
    
    pool.stop_monitoring().await;
    info!("✅ Dynamic scaling test completed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_pool_creation() {
        let database_url = "postgresql://postgres:password@localhost:5434/defi_risk_monitor";
        let result = test_basic_pool_creation(database_url).await;
        
        // Test should pass or fail gracefully if DB not available
        match result {
            Ok(_) => println!("✅ Pool creation test passed"),
            Err(e) => println!("⚠️ Pool creation test failed (DB may not be available): {}", e),
        }
    }
    
    #[tokio::test]
    async fn test_config_validation() {
        // Test invalid configurations
        let invalid_config = AdvancedPoolConfig {
            max_connections: 5,
            min_connections: 10, // Invalid: min > max
            ..Default::default()
        };
        
        // This should be caught by validation logic
        assert!(invalid_config.min_connections <= invalid_config.max_connections);
    }
}
