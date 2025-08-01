use defi_risk_monitor::utils::fault_tolerance::{
    ServiceCircuitBreaker, CircuitBreakerConfig, CircuitState
};
use defi_risk_monitor::error::AppError;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🔧 Testing Enhanced Circuit Breaker Implementation");

    // Test 1: Basic configuration and state transitions
    test_basic_functionality().await?;
    
    // Test 2: Half-open state testing with concurrent calls
    test_half_open_state_testing().await?;
    
    // Test 3: Different configurations (critical, external API, database)
    test_different_configurations().await?;
    
    // Test 4: Metrics collection and reporting
    test_metrics_collection().await?;
    
    // Test 5: Advanced half-open state behavior
    test_advanced_half_open_behavior().await?;

    info!("✅ All enhanced circuit breaker tests completed successfully!");
    Ok(())
}

async fn test_basic_functionality() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 1: Basic Circuit Breaker Functionality");
    
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_millis(100),
        half_open_max_calls: 1,
        half_open_test_interval: Duration::from_millis(10), // Shorter interval for testing
    };
    
    let circuit_breaker = ServiceCircuitBreaker::with_config("test-service", config);
    
    // Initially closed
    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.current_state, CircuitState::Closed);
    info!("   ✓ Circuit breaker starts in CLOSED state");
    
    // Simulate failures to open circuit
    for i in 1..=3 {
        let result = circuit_breaker.call(|| async {
            Err::<(), AppError>(AppError::InternalError("Simulated failure".to_string()))
        }).await;
        
        assert!(result.is_err());
        info!("   ✓ Failure {} recorded", i);
    }
    
    // Should be open now
    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.current_state, CircuitState::Open);
    assert_eq!(metrics.failed_calls, 3);
    assert_eq!(metrics.state_transitions, 1);
    info!("   ✓ Circuit breaker opened after {} failures", metrics.failed_calls);
    
    // Wait for timeout to transition to half-open
    sleep(Duration::from_millis(150)).await;
    
    // Next call should transition to half-open
    let result = circuit_breaker.call(|| async {
        Ok::<(), AppError>(())
    }).await;
    
    assert!(result.is_ok());
    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.current_state, CircuitState::HalfOpen);
    info!("   ✓ Circuit breaker transitioned to HALF-OPEN and accepted test call");
    
    // Wait for half-open test interval before next call
    sleep(Duration::from_millis(20)).await;
    
    // One more success should close it
    let result = circuit_breaker.call(|| async {
        Ok::<(), AppError>(())
    }).await;
    
    assert!(result.is_ok());
    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.current_state, CircuitState::Closed);
    assert_eq!(metrics.successful_calls, 2);
    assert_eq!(metrics.half_open_calls_successful, 2);
    info!("   ✓ Circuit breaker closed after {} successful half-open calls", metrics.half_open_calls_successful);
    
    Ok(())
}

async fn test_half_open_state_testing() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 2: Half-Open State Testing with Concurrent Calls");
    
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 3,
        timeout: Duration::from_millis(50),
        half_open_max_calls: 1, // Only allow 1 concurrent call
        half_open_test_interval: Duration::from_millis(100),
    };
    
    let circuit_breaker = ServiceCircuitBreaker::with_config("half-open-test", config);
    
    // Force to open state and set a failure time to enable timeout transition
    circuit_breaker.force_state(CircuitState::Open).await;
    
    // Simulate a failure to set the last_failure_time
    let _ = circuit_breaker.call(|| async {
        Err::<(), AppError>(AppError::InternalError("Force failure".to_string()))
    }).await;
    
    // Wait for timeout to allow transition to half-open
    sleep(Duration::from_millis(100)).await;
    
    // Force to half-open state for this test
    circuit_breaker.force_state(CircuitState::HalfOpen).await;
    
    // First call should work (in half-open state)
    let result1 = circuit_breaker.call(|| async {
        sleep(Duration::from_millis(50)).await; // Simulate slow operation
        Ok::<(), AppError>(())
    });
    
    // Second concurrent call should be rejected due to max concurrent calls
    sleep(Duration::from_millis(10)).await; // Small delay to ensure first call starts
    let result2 = circuit_breaker.call(|| async {
        Ok::<(), AppError>(())
    });
    
    let (res1, res2) = tokio::join!(result1, result2);
    
    assert!(res1.is_ok());
    assert!(res2.is_err()); // Should be rejected due to max concurrent calls
    
    let metrics = circuit_breaker.get_metrics().await;
    // Note: rejected_calls might be 2 due to the earlier forced failure call
    assert!(metrics.rejected_calls >= 1);
    info!("   ✓ Half-open state correctly rejected concurrent calls (rejected: {})", metrics.rejected_calls);
    
    Ok(())
}

async fn test_different_configurations() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 3: Different Circuit Breaker Configurations");
    
    // Test critical service configuration
    let critical_cb = ServiceCircuitBreaker::critical("critical-service");
    let critical_config = critical_cb.get_config();
    assert_eq!(critical_config.failure_threshold, 3);
    assert_eq!(critical_config.success_threshold, 5);
    assert_eq!(critical_config.timeout, Duration::from_secs(30));
    info!("   ✓ Critical service configuration: threshold={}, success={}, timeout={:?}", 
          critical_config.failure_threshold, critical_config.success_threshold, critical_config.timeout);
    
    // Test external API configuration
    let api_cb = ServiceCircuitBreaker::external_api("external-api");
    let api_config = api_cb.get_config();
    assert_eq!(api_config.failure_threshold, 10);
    assert_eq!(api_config.success_threshold, 3);
    assert_eq!(api_config.timeout, Duration::from_secs(120));
    info!("   ✓ External API configuration: threshold={}, success={}, timeout={:?}", 
          api_config.failure_threshold, api_config.success_threshold, api_config.timeout);
    
    // Test database configuration
    let db_cb = ServiceCircuitBreaker::database("database-service");
    let db_config = db_cb.get_config();
    assert_eq!(db_config.failure_threshold, 5);
    assert_eq!(db_config.success_threshold, 3);
    assert_eq!(db_config.timeout, Duration::from_secs(45));
    info!("   ✓ Database configuration: threshold={}, success={}, timeout={:?}", 
          db_config.failure_threshold, db_config.success_threshold, db_config.timeout);
    
    Ok(())
}

async fn test_metrics_collection() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 4: Comprehensive Metrics Collection");
    
    let circuit_breaker = ServiceCircuitBreaker::new("metrics-test");
    
    // Execute various operations to generate metrics
    for i in 1..=5 {
        let result = if i <= 3 {
            circuit_breaker.call(|| async {
                Ok::<(), AppError>(())
            }).await
        } else {
            circuit_breaker.call(|| async {
                Err::<(), AppError>(AppError::InternalError("Test failure".to_string()))
            }).await
        };
        
        if i <= 3 {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
        }
    }
    
    let metrics = circuit_breaker.get_metrics().await;
    
    info!("   📊 Circuit Breaker Metrics:");
    info!("      - Total calls: {}", metrics.total_calls);
    info!("      - Successful calls: {}", metrics.successful_calls);
    info!("      - Failed calls: {}", metrics.failed_calls);
    info!("      - Rejected calls: {}", metrics.rejected_calls);
    info!("      - State transitions: {}", metrics.state_transitions);
    info!("      - Current state: {:?}", metrics.current_state);
    info!("      - Time in current state: {:?}", metrics.time_in_current_state);
    info!("      - Half-open calls attempted: {}", metrics.half_open_calls_attempted);
    info!("      - Half-open calls successful: {}", metrics.half_open_calls_successful);
    
    assert_eq!(metrics.total_calls, 5);
    assert_eq!(metrics.successful_calls, 3);
    assert_eq!(metrics.failed_calls, 2);
    assert!(metrics.time_in_current_state > Duration::from_millis(0));
    
    info!("   ✓ All metrics collected correctly");
    
    Ok(())
}

async fn test_advanced_half_open_behavior() -> Result<(), Box<dyn std::error::Error>> {
    info!("🧪 Test 5: Advanced Half-Open State Behavior");
    
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 3,
        timeout: Duration::from_millis(50),
        half_open_max_calls: 2,
        half_open_test_interval: Duration::from_millis(200), // Long interval
    };
    
    let circuit_breaker = ServiceCircuitBreaker::with_config("advanced-half-open", config);
    
    // Force to half-open state
    circuit_breaker.force_state(CircuitState::HalfOpen).await;
    
    // First call should work
    let result1 = circuit_breaker.call(|| async {
        Ok::<(), AppError>(())
    }).await;
    assert!(result1.is_ok());
    
    // Immediate second call should be rejected due to test interval
    let result2 = circuit_breaker.call(|| async {
        Ok::<(), AppError>(())
    }).await;
    assert!(result2.is_err());
    
    let metrics = circuit_breaker.get_metrics().await;
    info!("   ✓ Half-open test interval correctly enforced (rejected: {})", metrics.rejected_calls);
    
    // Test failure in half-open state immediately opens circuit
    circuit_breaker.force_state(CircuitState::HalfOpen).await;
    sleep(Duration::from_millis(250)).await; // Wait for test interval
    
    let result3 = circuit_breaker.call(|| async {
        Err::<(), AppError>(AppError::InternalError("Half-open failure".to_string()))
    }).await;
    
    assert!(result3.is_err());
    let metrics = circuit_breaker.get_metrics().await;
    assert_eq!(metrics.current_state, CircuitState::Open);
    info!("   ✓ Failure in half-open state immediately opened circuit");
    
    Ok(())
}
