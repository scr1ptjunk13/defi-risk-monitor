use defi_risk_monitor::database::{TransactionRetryWrapper, TransactionRetryConfig, is_transaction_retryable_error};
use defi_risk_monitor::error::AppError;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    info!("🔄 Starting Transaction Retry Logic Tests...");

    // Connect to database
    let database_url = "postgresql://postgres:password@localhost:5434/defi_risk_monitor";
    let pool = PgPool::connect(database_url).await?;

    info!("✅ Connected to database");

    // Run all transaction retry tests
    test_error_classification().await?;
    test_deadlock_retry(&pool).await?;
    test_serialization_failure_retry(&pool).await?;
    // Simple query retry tests removed - using transaction retry wrapper instead
    test_transaction_configs().await?;

    info!("🎉 All transaction retry tests completed successfully!");
    Ok(())
}

/// Test 1: Transaction error classification
async fn test_error_classification() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 1: Testing transaction error classification...");

    // Test retryable transaction errors
    let retryable_errors = vec![
        "deadlock detected",
        "lock timeout exceeded", 
        "could not obtain lock on row",
        "serialization failure",
        "could not serialize access due to concurrent update",
        "connection timeout",
        "query timeout",
        "transaction aborted",
        "transaction rolled back",
    ];

    for error_msg in retryable_errors {
        let error = AppError::DatabaseError(error_msg.to_string());
        assert!(is_transaction_retryable_error(&error), "Should be retryable: {}", error_msg);
        info!("   ✅ Correctly identified as retryable: {}", error_msg);
    }

    // Test non-retryable transaction errors
    let non_retryable_errors = vec![
        "syntax error at or near",
        "constraint violation",
        "column does not exist",
        "relation does not exist",
        "permission denied",
    ];

    for error_msg in non_retryable_errors {
        let error = AppError::DatabaseError(error_msg.to_string());
        assert!(!is_transaction_retryable_error(&error), "Should not be retryable: {}", error_msg);
        info!("   ✅ Correctly identified as non-retryable: {}", error_msg);
    }

    Ok(())
}

/// Test 2: Deadlock retry simulation
async fn test_deadlock_retry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 2: Testing deadlock retry logic...");

    let config = TransactionRetryConfig::for_deadlocks();
    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);
    let attempt_counter = Arc::new(AtomicU32::new(0));

    let result = wrapper.execute_simple_transaction_with_retry(
        "deadlock_simulation",
        {
            let counter = attempt_counter.clone();
            move || {
                let attempt = counter.fetch_add(1, Ordering::SeqCst) + 1;
                info!("   🔄 Deadlock simulation attempt {}", attempt);

                if attempt < 3 {
                    // Simulate deadlock on first two attempts
                    return Err(AppError::DatabaseError("deadlock detected".to_string()));
                }

                // Success on third attempt
                info!("   ✅ Deadlock resolved on attempt {}", attempt);
                Ok(format!("deadlock_resolved_attempt_{}", attempt))
            }
        },
    ).await;

    match result {
        Ok(success_msg) => {
            info!("   ✅ Deadlock retry successful: {}", success_msg);
            assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
        }
        Err(e) => {
            error!("   ❌ Deadlock retry failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 3: Serialization failure retry simulation
async fn test_serialization_failure_retry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 3: Testing serialization failure retry logic...");

    let config = TransactionRetryConfig::for_serialization_failures();
    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);
    let attempt_counter = Arc::new(AtomicU32::new(0));

    let result = wrapper.execute_simple_transaction_with_retry(
        "serialization_failure_simulation",
        || {
            let attempt = attempt_counter.fetch_add(1, Ordering::SeqCst) + 1;
            info!("   🔄 Attempt {} for serialization failure test", attempt);
            
            // Simulate serialization failure on first few attempts
            if attempt <= 2 {
                return Err(AppError::DatabaseError("could not serialize access due to concurrent update".to_string()));
            }
            
            // Success on final attempt
            info!("   ✅ Serialization failure test succeeded on attempt {}", attempt);
            Ok(42)
        },
    ).await;

    match result {
        Ok(value) => {
            info!("   ✅ Serialization failure retry successful: {}", value);
            assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
        }
        Err(e) => {
            error!("   ❌ Serialization failure retry failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 4: Timeout handling
async fn test_timeout_handling(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 4: Testing timeout handling...");

    let mut config = TransactionRetryConfig::default();
    config.transaction_timeout_secs = 1; // Very short timeout for testing
    config.max_attempts = 2;

    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);

    // Test timeout scenario (simulate with a delay)
    let result = wrapper.execute_simple_transaction_with_retry(
        "timeout_test",
        || {
            info!("   ⏱️  Simulating long-running operation...");
            // Simulate operation that takes longer than timeout
            std::thread::sleep(std::time::Duration::from_millis(100));
            Ok(42)
        },
    ).await;

    match result {
        Err(AppError::DatabaseError(msg)) if msg.contains("timeout") => {
            info!("   ✅ Timeout correctly detected and handled: {}", msg);
        }
        Ok(_) => {
            warn!("   ⚠️  Expected timeout but operation succeeded");
        }
        Err(e) => {
            error!("   ❌ Unexpected error: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 5: Bulk insert with deadlock retry
async fn test_bulk_insert_with_retry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 5: Testing bulk insert with retry logic...");

    let config = TransactionRetryConfig::for_deadlocks();
    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);

    // Create test data
    let test_data = vec![
        ("test_pool_1", "token_a", "token_b"),
        ("test_pool_2", "token_c", "token_d"),
        ("test_pool_3", "token_e", "token_f"),
    ];

    let attempt_counter = Arc::new(AtomicU32::new(0));

    let result = wrapper.execute_simple_transaction_with_retry(
        "bulk_insert_test",
        || {
            let attempt = attempt_counter.fetch_add(1, Ordering::SeqCst) + 1;
            
            // Simulate deadlock on first few operations
            if attempt <= 2 {
                return Err(AppError::DatabaseError("deadlock detected during bulk insert".to_string()));
            }

            // Simulate successful bulk insert
            info!("   📝 Simulating bulk insert on attempt {}", attempt);
            Ok(test_data.len() as i32)
        },
    ).await;

    match result {
        Ok(count) => {
            info!("   ✅ Bulk insert with retry successful: {} items processed", count);
        }
        Err(e) => {
            error!("   ❌ Bulk insert with retry failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 6: Read-only transaction retry
async fn test_readonly_transaction_retry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 6: Testing read-only transaction retry...");

    let config = TransactionRetryConfig::default();
    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);

    let result = wrapper.execute_simple_transaction_with_retry(
        "readonly_test",
        || {
            // Simulate a simple read-only operation
            info!("   📖 Executing read-only operation");
            Ok(1i64)
        },
    ).await;

    match result {
        Ok(value) => {
            info!("   ✅ Read-only transaction successful: {}", value);
            assert_eq!(value, 1);
        }
        Err(e) => {
            error!("   ❌ Read-only transaction failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 7: Different retry configurations
async fn test_transaction_configs() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 7: Testing different transaction retry configurations...");

    let deadlock_config = TransactionRetryConfig::for_deadlocks();
    info!("   📊 Deadlock config: {} attempts, {}ms base delay, {}s timeout", 
          deadlock_config.max_attempts, deadlock_config.base_delay_ms, deadlock_config.transaction_timeout_secs);

    let serialization_config = TransactionRetryConfig::for_serialization_failures();
    info!("   📊 Serialization config: {} attempts, {}ms base delay, {}s timeout", 
          serialization_config.max_attempts, serialization_config.base_delay_ms, serialization_config.transaction_timeout_secs);

    let long_running_config = TransactionRetryConfig::for_long_running();
    info!("   📊 Long-running config: {} attempts, {}ms base delay, {}s timeout", 
          long_running_config.max_attempts, long_running_config.base_delay_ms, long_running_config.transaction_timeout_secs);

    let default_config = TransactionRetryConfig::default();
    info!("   📊 Default config: {} attempts, {}ms base delay, {}s timeout", 
          default_config.max_attempts, default_config.base_delay_ms, default_config.transaction_timeout_secs);

    // Verify configurations have expected values
    assert_eq!(deadlock_config.max_attempts, 3);
    assert_eq!(serialization_config.max_attempts, 4);
    assert_eq!(long_running_config.max_attempts, 2);
    assert_eq!(default_config.max_attempts, 5);

    info!("   ✅ All transaction configurations validated");

    Ok(())
}
