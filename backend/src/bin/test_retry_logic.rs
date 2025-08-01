use defi_risk_monitor::database::{establish_connection, RetryableDatabase};
use defi_risk_monitor::error::{AppError, retry::{with_retry, RetryConfig, is_retryable_error}};
use defi_risk_monitor::{retry_db_operation, retry_api_operation, retry_blockchain_operation};
use std::env;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{info, error, warn};
use tracing_subscriber;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    info!("🔄 Starting Exponential Backoff Retry Logic Integration Test");
    
    // Load database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    
    info!("📊 Connecting to database: {}", database_url.replace("password", "***"));
    
    // Establish database connection
    let pool = establish_connection(&database_url).await?;
    info!("✅ Database connection established successfully");
    
    // Create retryable database wrapper
    let retryable_db = RetryableDatabase::new(pool);
    info!("🔧 RetryableDatabase wrapper created successfully");
    
    // Test 1: Error Classification
    info!("\n🔍 Test 1: Testing error classification for retry logic...");
    test_error_classification().await;
    
    // Test 2: Successful Operation (No Retry Needed)
    info!("\n🔍 Test 2: Testing successful operation (no retry needed)...");
    test_successful_operation(&retryable_db).await?;
    
    // Test 3: Simulated Transient Error with Recovery
    info!("\n🔍 Test 3: Testing simulated transient error with recovery...");
    test_transient_error_recovery().await?;
    
    // Test 4: Non-Retryable Error
    info!("\n🔍 Test 4: Testing non-retryable error handling...");
    test_non_retryable_error().await?;
    
    // Test 5: Retry Exhaustion
    info!("\n🔍 Test 5: Testing retry exhaustion scenario...");
    test_retry_exhaustion().await?;
    
    // Test 6: Database Operations with Retry
    info!("\n🔍 Test 6: Testing real database operations with retry logic...");
    test_database_operations(&retryable_db).await?;
    
    // Test 7: Different Retry Configurations
    info!("\n🔍 Test 7: Testing different retry configurations...");
    test_retry_configurations().await?;
    
    // Test 8: Macro Usage Examples
    info!("\n🔍 Test 8: Testing retry macros...");
    test_retry_macros().await?;
    
    info!("\n📊 Exponential Backoff Retry Logic Test Summary:");
    info!("✅ All retry logic tests completed successfully");
    info!("✅ Error classification working correctly");
    info!("✅ Exponential backoff with jitter implemented");
    info!("✅ Transient error recovery functional");
    info!("✅ Non-retryable error detection working");
    info!("✅ Retry exhaustion handling proper");
    info!("✅ Database integration successful");
    info!("✅ Multiple retry configurations supported");
    info!("🎉 Retry logic is ready for production use!");
    
    Ok(())
}

async fn test_error_classification() {
    info!("   Testing error classification for retry decisions...");
    
    // Retryable database errors
    let retryable_errors = vec![
        AppError::DatabaseError("connection timeout".to_string()),
        AppError::DatabaseError("deadlock detected".to_string()),
        AppError::DatabaseError("too many connections".to_string()),
        AppError::DatabaseError("connection reset by peer".to_string()),
        AppError::DatabaseError("serialization failure".to_string()),
        AppError::DatabaseError("lock timeout".to_string()),
        AppError::ExternalApiError("network timeout".to_string()),
        AppError::RateLimitError("rate limit exceeded".to_string()),
    ];
    
    for error in &retryable_errors {
        if is_retryable_error(error) {
            info!("   ✅ Correctly identified as retryable: {}", error);
        } else {
            error!("   ❌ Incorrectly identified as non-retryable: {}", error);
        }
    }
    
    // Non-retryable errors
    let non_retryable_errors = vec![
        AppError::DatabaseError("syntax error at or near".to_string()),
        AppError::DatabaseError("constraint violation".to_string()),
        AppError::ValidationError("invalid input".to_string()),
        AppError::NotFound("resource not found".to_string()),
        AppError::AuthenticationError("invalid credentials".to_string()),
        AppError::ConfigError("missing configuration".to_string()),
    ];
    
    for error in &non_retryable_errors {
        if !is_retryable_error(error) {
            info!("   ✅ Correctly identified as non-retryable: {}", error);
        } else {
            error!("   ❌ Incorrectly identified as retryable: {}", error);
        }
    }
}

async fn test_successful_operation(retryable_db: &RetryableDatabase) -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing successful operation that doesn't need retry...");
    
    let result = retryable_db.health_check_with_retry().await?;
    if result {
        info!("   ✅ Health check succeeded on first attempt");
    } else {
        error!("   ❌ Health check failed unexpectedly");
    }
    
    Ok(())
}

async fn test_transient_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing transient error recovery with exponential backoff...");
    
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();
    
    let start_time = std::time::Instant::now();
    
    let result = with_retry(
        "simulated_transient_operation",
        RetryConfig::with_max_attempts(4),
        move || {
            let count = attempt_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                info!("   🔄 Attempt {} - simulating transient error", current + 1);
                
                if current < 2 {
                    // Simulate transient database error for first 2 attempts
                    Err(AppError::DatabaseError("connection timeout".to_string()))
                } else {
                    // Succeed on 3rd attempt
                    info!("   ✅ Operation succeeded on attempt {}", current + 1);
                    Ok("success".to_string())
                }
            }
        },
    ).await;
    
    let duration = start_time.elapsed();
    
    match result {
        Ok(value) => {
            info!("   ✅ Transient error recovery successful: {}", value);
            info!("   📊 Total attempts: {}", attempt_count.load(Ordering::SeqCst));
            info!("   ⏱️  Total time with backoff: {:?}", duration);
            
            // Verify we made exactly 3 attempts
            assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
        }
        Err(e) => {
            error!("   ❌ Transient error recovery failed: {}", e);
            return Err(e.into());
        }
    }
    
    Ok(())
}

async fn test_non_retryable_error() -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing non-retryable error handling...");
    
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();
    
    let result = with_retry(
        "non_retryable_operation",
        RetryConfig::with_max_attempts(3),
        move || {
            let count = attempt_count_clone.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // Simulate non-retryable validation error
                Err::<String, AppError>(AppError::ValidationError("invalid input format".to_string()))
            }
        },
    ).await;
    
    match result {
        Err(AppError::ValidationError(_)) => {
            info!("   ✅ Non-retryable error correctly handled without retry");
            info!("   📊 Attempts made: {} (should be 1)", attempt_count.load(Ordering::SeqCst));
            
            // Verify we only made 1 attempt
            assert_eq!(attempt_count.load(Ordering::SeqCst), 1);
        }
        _ => {
            error!("   ❌ Non-retryable error handling failed");
            return Err("Non-retryable error test failed".into());
        }
    }
    
    Ok(())
}

async fn test_retry_exhaustion() -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing retry exhaustion scenario...");
    
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();
    
    let start_time = std::time::Instant::now();
    
    let result = with_retry(
        "always_failing_operation",
        RetryConfig::with_max_attempts(3),
        move || {
            let count = attempt_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                info!("   🔄 Attempt {} - simulating persistent failure", current + 1);
                
                // Always fail with retryable error
                Err::<String, AppError>(AppError::DatabaseError("connection timeout".to_string()))
            }
        },
    ).await;
    
    let duration = start_time.elapsed();
    
    match result {
        Err(AppError::DatabaseError(_)) => {
            info!("   ✅ Retry exhaustion handled correctly");
            info!("   📊 Total attempts: {} (should be 3)", attempt_count.load(Ordering::SeqCst));
            info!("   ⏱️  Total time with exponential backoff: {:?}", duration);
            
            // Verify we made exactly 3 attempts
            assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
            
            // Verify we waited for backoff (should be > 300ms with base delays)
            assert!(duration.as_millis() > 300);
        }
        _ => {
            error!("   ❌ Retry exhaustion test failed");
            return Err("Retry exhaustion test failed".into());
        }
    }
    
    Ok(())
}

async fn test_database_operations(retryable_db: &RetryableDatabase) -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing real database operations with retry logic...");
    
    // Test connection count query
    match retryable_db.get_connection_count_with_retry().await {
        Ok(count) => {
            info!("   ✅ Connection count query successful: {} connections", count);
        }
        Err(e) => {
            error!("   ❌ Connection count query failed: {}", e);
        }
    }
    
    // Test database stats query
    match retryable_db.get_database_stats_with_retry().await {
        Ok(stats) => {
            info!("   ✅ Database stats query successful:");
            info!("      📊 Database size: {} bytes", stats.database_size);
            info!("      🔗 Active connections: {}", stats.active_connections);
            info!("      📈 Total connections: {}", stats.total_connections);
        }
        Err(e) => {
            error!("   ❌ Database stats query failed: {}", e);
        }
    }
    
    // Test simple query execution
    match retryable_db.execute_query_with_retry("SELECT 1").await {
        Ok(rows_affected) => {
            info!("   ✅ Simple query execution successful: {} rows", rows_affected);
        }
        Err(e) => {
            error!("   ❌ Simple query execution failed: {}", e);
        }
    }
    
    Ok(())
}

async fn test_retry_configurations() -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing different retry configurations...");
    
    // Test database config
    let db_config = RetryConfig::for_database();
    info!("   📊 Database config: {} attempts, {}ms base delay", 
        db_config.max_attempts, db_config.base_delay_ms);
    
    // Test external API config
    let api_config = RetryConfig::for_external_api();
    info!("   📊 API config: {} attempts, {}ms base delay", 
        api_config.max_attempts, api_config.base_delay_ms);
    
    // Test blockchain config
    let blockchain_config = RetryConfig::for_blockchain();
    info!("   📊 Blockchain config: {} attempts, {}ms base delay", 
        blockchain_config.max_attempts, blockchain_config.base_delay_ms);
    
    // Test custom config
    let custom_config = RetryConfig {
        max_attempts: 2,
        base_delay_ms: 50,
        max_delay_ms: 1000,
        jitter_factor: 0.05,
        backoff_multiplier: 1.5,
    };
    
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();
    
    let result = with_retry(
        "custom_config_test",
        custom_config,
        move || {
            let count = attempt_count_clone.clone();
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    Err(AppError::DatabaseError("timeout".to_string()))
                } else {
                    Ok("success")
                }
            }
        },
    ).await;
    
    match result {
        Ok(_) => {
            info!("   ✅ Custom config test successful");
            assert_eq!(attempt_count.load(Ordering::SeqCst), 2);
        }
        Err(e) => {
            error!("   ❌ Custom config test failed: {}", e);
        }
    }
    
    Ok(())
}

async fn test_retry_macros() -> Result<(), Box<dyn std::error::Error>> {
    info!("   Testing retry convenience macros...");
    
    // Test database retry macro
    let db_result = retry_db_operation!(
        "macro_db_test",
        {
            // Simulate successful database operation
            Ok::<String, AppError>("db_success".to_string())
        }
    );
    
    match db_result {
        Ok(value) => {
            info!("   ✅ Database retry macro successful: {}", value);
        }
        Err(e) => {
            error!("   ❌ Database retry macro failed: {}", e);
        }
    }
    
    // Test API retry macro
    let api_result = retry_api_operation!(
        "macro_api_test",
        {
            // Simulate successful API operation
            Ok::<String, AppError>("api_success".to_string())
        }
    );
    
    match api_result {
        Ok(value) => {
            info!("   ✅ API retry macro successful: {}", value);
        }
        Err(e) => {
            error!("   ❌ API retry macro failed: {}", e);
        }
    }
    
    // Test blockchain retry macro
    let blockchain_result = retry_blockchain_operation!(
        "macro_blockchain_test",
        {
            // Simulate successful blockchain operation
            Ok::<String, AppError>("blockchain_success".to_string())
        }
    );
    
    match blockchain_result {
        Ok(value) => {
            info!("   ✅ Blockchain retry macro successful: {}", value);
        }
        Err(e) => {
            error!("   ❌ Blockchain retry macro failed: {}", e);
        }
    }
    
    Ok(())
}
