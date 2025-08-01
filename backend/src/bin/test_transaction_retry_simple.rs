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
    test_deadlock_simulation().await?;
    test_serialization_failure_simulation().await?;
    test_count_query_with_retry(&pool).await?;
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
async fn test_deadlock_simulation() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 2: Testing deadlock retry logic...");

    let attempt_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = attempt_counter.clone();

    // Simulate deadlock retry logic manually
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        attempts += 1;
        let current_attempt = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
        info!("   🔄 Deadlock simulation attempt {}", current_attempt);

        if current_attempt < 3 {
            // Simulate deadlock on first two attempts
            let error = AppError::DatabaseError("deadlock detected".to_string());
            if is_transaction_retryable_error(&error) {
                warn!("   ⚠️  Deadlock detected, will retry...");
                if attempts >= max_attempts {
                    error!("   ❌ Max attempts reached");
                    return Err("Max attempts reached".into());
                }
                // Simulate exponential backoff delay
                tokio::time::sleep(std::time::Duration::from_millis(50 * attempts as u64)).await;
                continue;
            }
        }

        // Success on third attempt
        info!("   ✅ Deadlock resolved on attempt {}", current_attempt);
        break;
    }

    assert_eq!(attempt_counter.load(Ordering::SeqCst), 3);
    info!("   ✅ Deadlock retry simulation successful");

    Ok(())
}

/// Test 3: Serialization failure retry simulation
async fn test_serialization_failure_simulation() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 3: Testing serialization failure retry logic...");

    let attempt_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = attempt_counter.clone();

    // Simulate serialization failure retry logic manually
    let mut attempts = 0;
    let max_attempts = 4;
    
    loop {
        attempts += 1;
        let current_attempt = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
        info!("   🔄 Serialization failure simulation attempt {}", current_attempt);

        if current_attempt < 2 {
            // Simulate serialization failure on first attempt
            let error = AppError::DatabaseError("could not serialize access due to concurrent update".to_string());
            if is_transaction_retryable_error(&error) {
                warn!("   ⚠️  Serialization failure detected, will retry...");
                if attempts >= max_attempts {
                    error!("   ❌ Max attempts reached");
                    return Err("Max attempts reached".into());
                }
                // Simulate exponential backoff delay
                tokio::time::sleep(std::time::Duration::from_millis(100 * attempts as u64)).await;
                continue;
            }
        }

        // Success on second attempt
        info!("   ✅ Serialization conflict resolved on attempt {}", current_attempt);
        break;
    }

    assert_eq!(attempt_counter.load(Ordering::SeqCst), 2);
    info!("   ✅ Serialization failure retry simulation successful");

    Ok(())
}

/// Test 4: Count query with retry
async fn test_count_query_with_retry(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 4: Testing count query with retry logic...");

    let config = TransactionRetryConfig::default();
    let wrapper = TransactionRetryWrapper::new(pool.clone(), config);

    let result = wrapper.execute_count_query_with_retry(
        "connection_count",
        "SELECT COUNT(*)::bigint FROM pg_stat_activity WHERE state = 'active'"
    ).await;

    match result {
        Ok(count) => {
            info!("   ✅ Count query successful: {} active connections", count);
            assert!(count >= 0);
        }
        Err(e) => {
            error!("   ❌ Count query failed: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Test 5: Different retry configurations
async fn test_transaction_configs() -> Result<(), Box<dyn std::error::Error>> {
    info!("\n🔍 Test 5: Testing different transaction retry configurations...");

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
