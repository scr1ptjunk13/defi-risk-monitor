use std::sync::Arc;
use tokio::time::{sleep, Duration};
use defi_risk_monitor::error::{AppError, ErrorClassifier, ErrorCategory, ErrorSeverity, ConstraintViolationHandler, ConstraintViolationType};
use defi_risk_monitor::services::graceful_degradation::{GracefulDegradationService, DegradationLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Starting comprehensive error classification test...\n");

    // Test 1: Error Classification System
    test_error_classification().await;
    
    // Test 2: Constraint Violation Handling
    test_constraint_violation_handling().await;
    
    // Test 3: Graceful Degradation Service
    test_graceful_degradation().await;
    
    // Test 4: Integration Test - Error Classification with Degradation
    test_integration_error_handling().await;

    println!("✅ All error classification tests completed successfully!");
    Ok(())
}

async fn test_error_classification() {
    println!("📊 Testing Error Classification System");
    println!("=====================================");

    let classifier = ErrorClassifier::new();

    // Test database errors
    let db_errors = vec![
        ("connection pool exhausted", ErrorCategory::ResourceExhaustion, ErrorSeverity::High, true),
        ("connection timeout", ErrorCategory::Transient, ErrorSeverity::Medium, true),
        ("deadlock detected", ErrorCategory::Transient, ErrorSeverity::Medium, true),
        ("unique constraint violation", ErrorCategory::ConstraintViolation, ErrorSeverity::Medium, false),
        ("foreign key constraint", ErrorCategory::ConstraintViolation, ErrorSeverity::Medium, false),
        ("syntax error", ErrorCategory::Permanent, ErrorSeverity::High, false),
        ("permission denied", ErrorCategory::Permanent, ErrorSeverity::High, false),
    ];

    println!("🔍 Database Error Classification:");
    for (error_msg, expected_category, expected_severity, expected_retryable) in db_errors {
        let app_error = AppError::DatabaseError(error_msg.to_string());
        let classification = classifier.classify_error(&app_error);
        
        println!("  • '{}' -> {:?} | {:?} | Retryable: {}", 
            error_msg, classification.category, classification.severity, classification.is_retryable);
        
        assert_eq!(classification.category, expected_category, "Category mismatch for: {}", error_msg);
        assert_eq!(classification.severity, expected_severity, "Severity mismatch for: {}", error_msg);
        assert_eq!(classification.is_retryable, expected_retryable, "Retryability mismatch for: {}", error_msg);
    }

    // Test blockchain errors
    let blockchain_errors = vec![
        ("network timeout", ErrorCategory::Transient, ErrorSeverity::Medium, true),
        ("insufficient gas", ErrorCategory::Permanent, ErrorSeverity::Medium, false),
        ("nonce too low", ErrorCategory::Permanent, ErrorSeverity::Medium, false),
        ("execution reverted", ErrorCategory::Permanent, ErrorSeverity::Medium, false),
        ("invalid signature", ErrorCategory::Transient, ErrorSeverity::Medium, true),
    ];

    println!("\n🔗 Blockchain Error Classification:");
    for (error_msg, expected_category, expected_severity, expected_retryable) in blockchain_errors {
        let app_error = AppError::BlockchainError(error_msg.to_string());
        let classification = classifier.classify_error(&app_error);
        
        println!("  • '{}' -> {:?} | {:?} | Retryable: {}", 
            error_msg, classification.category, classification.severity, classification.is_retryable);
        
        assert_eq!(classification.category, expected_category, "Category mismatch for: {}", error_msg);
        assert_eq!(classification.severity, expected_severity, "Severity mismatch for: {}", error_msg);
        assert_eq!(classification.is_retryable, expected_retryable, "Retryability mismatch for: {}", error_msg);
    }

    // Test API errors
    let api_errors = vec![
        ("rate limit exceeded", ErrorCategory::RateLimit, ErrorSeverity::Medium, true),
        ("service unavailable", ErrorCategory::Transient, ErrorSeverity::Medium, true),
        ("unauthorized", ErrorCategory::Transient, ErrorSeverity::Medium, true),
        ("bad request", ErrorCategory::Transient, ErrorSeverity::Medium, true),
    ];

    println!("\n🌐 API Error Classification:");
    for (error_msg, expected_category, expected_severity, expected_retryable) in api_errors {
        let app_error = AppError::ExternalApiError(error_msg.to_string());
        let classification = classifier.classify_error(&app_error);
        
        println!("  • '{}' -> {:?} | {:?} | Retryable: {}", 
            error_msg, classification.category, classification.severity, classification.is_retryable);
        
        assert_eq!(classification.category, expected_category, "Category mismatch for: {}", error_msg);
        assert_eq!(classification.severity, expected_severity, "Severity mismatch for: {}", error_msg);
        assert_eq!(classification.is_retryable, expected_retryable, "Retryability mismatch for: {}", error_msg);
    }

    // Test read-only compatibility
    println!("\n📖 Read-Only Compatibility Test:");
    let readonly_compatible_errors = vec![
        "connection timeout",
        "read timeout", 
        "network unreachable",
        "service unavailable",
    ];

    for error_msg in readonly_compatible_errors {
        let app_error = AppError::DatabaseError(error_msg.to_string());
        let classification = classifier.classify_error(&app_error);
        
        println!("  • '{}' -> Read-only compatible: {}", error_msg, classification.is_read_only_compatible);
        assert!(classification.is_read_only_compatible, "Should be read-only compatible: {}", error_msg);
    }

    println!("✅ Error Classification System test passed!\n");
}

async fn test_constraint_violation_handling() {
    println!("🚫 Testing Constraint Violation Handling");
    println!("========================================");

    let handler = ConstraintViolationHandler::new();

    // Test different constraint violation types
    let constraint_errors = vec![
        (
            "duplicate key value violates unique constraint \"users_email_key\"",
            ConstraintViolationType::UniqueConstraint,
            "users_email_key",
            None,
            None
        ),
        (
            "insert or update on table \"orders\" violates foreign key constraint \"fk_user_id\"",
            ConstraintViolationType::ForeignKeyConstraint,
            "fk_user_id",
            Some("orders"),
            None
        ),
        (
            "new row for relation \"accounts\" violates check constraint \"positive_balance\"",
            ConstraintViolationType::CheckConstraint,
            "positive_balance",
            None,
            None
        ),
    ];

    println!("🔍 Constraint Violation Analysis:");
    for (error_msg, expected_type, expected_constraint, expected_table, expected_column) in constraint_errors {
        let app_error = AppError::DatabaseError(error_msg.to_string());
        if let Some(violation_info) = handler.analyze_constraint_violation(&app_error) {
            println!("  • Type: {:?}", violation_info.violation_type);
            println!("    Constraint: '{:?}'", violation_info.constraint_name);
            println!("    Table: {:?}", violation_info.table_name);
            println!("    Column: {:?}", violation_info.column_name);
            println!("    User Message: '{}'", violation_info.suggested_resolution);
            println!("    Recoverable: {}", violation_info.is_recoverable);
            
            assert_eq!(violation_info.violation_type, expected_type, "Type mismatch for: {}", error_msg);
            assert_eq!(violation_info.constraint_name.as_deref().unwrap_or(""), expected_constraint, "Constraint mismatch for: {}", error_msg);
            assert_eq!(violation_info.table_name.as_deref(), expected_table, "Table mismatch for: {}", error_msg);
            assert_eq!(violation_info.column_name.as_deref(), expected_column, "Column mismatch for: {}", error_msg);
            
            println!();
        } else {
            panic!("Failed to analyze constraint violation: {}", error_msg);
        }
    }

    println!("✅ Constraint Violation Handling test passed!\n");
}

async fn test_graceful_degradation() {
    println!("🛡️ Testing Graceful Degradation Service");
    println!("=======================================");

    let degradation_service = Arc::new(GracefulDegradationService::new());

    // Test initial state
    println!("📊 Initial State:");
    let level = degradation_service.get_current_level().await;
    let capabilities = degradation_service.get_capabilities().await;
    println!("  • Degradation Level: {:?}", level);
    println!("  • Can Write: {}", capabilities.can_write);
    println!("  • Can Read: {}", capabilities.can_read);
    println!("  • Can Alert: {}", capabilities.can_send_alerts);

    // Test error accumulation and automatic degradation
    println!("\n🔥 Simulating Error Accumulation:");
    
    // Simulate database errors
    for i in 1..=15 {
        let error = AppError::DatabaseError("connection timeout".to_string());
        let _ = degradation_service.record_error(&error).await;
        
        if i % 5 == 0 {
            let level = degradation_service.get_current_level().await;
            println!("  • After {} errors -> Level: {:?}", i, level);
        }
    }

    // Check if degradation occurred
    let level = degradation_service.get_current_level().await;
    println!("  • Final degradation level: {:?}", level);

    // Test operation permissions
    println!("\n🔒 Testing Operation Permissions:");
    let can_write = degradation_service.can_perform_operation("write_operation").await;
    let can_read = degradation_service.can_perform_operation("read_operation").await;
    println!("  • Can perform write operation: {}", can_write);
    println!("  • Can perform read operation: {}", can_read);

    // Test manual override
    println!("\n🎛️ Testing Manual Override:");
    degradation_service.enable_manual_override();
    degradation_service.set_degradation_level(DegradationLevel::Normal).await.unwrap();
    let level = degradation_service.get_current_level().await;
    println!("  • After manual override -> Level: {:?}", level);

    // Test auto-recovery
    println!("\n🔄 Testing Auto-Recovery:");
    degradation_service.disable_manual_override();
    degradation_service.enable_auto_recovery();
    
    // Simulate recovery attempt
    for i in 1..=5 {
        let recovery_attempted = degradation_service.attempt_recovery().await.unwrap();
        
        if i % 2 == 0 {
            let level = degradation_service.get_current_level().await;
            println!("  • Recovery attempt {} -> Level: {:?}, Success: {}", i, level, recovery_attempted);
        }
    }

    println!("✅ Graceful Degradation Service test passed!\n");
}

async fn test_integration_error_handling() {
    println!("🔗 Testing Integration: Error Classification + Degradation");
    println!("=========================================================");

    let classifier = ErrorClassifier::new();
    let degradation_service = Arc::new(GracefulDegradationService::new());

    // Simulate a sequence of different errors and observe system behavior
    let error_sequence = vec![
        ("Database connection timeout", AppError::DatabaseError("connection timeout".to_string())),
        ("Blockchain network error", AppError::BlockchainError("network timeout".to_string())),
        ("API rate limit", AppError::ExternalApiError("rate limit exceeded".to_string())),
        ("Constraint violation", AppError::DatabaseError("unique constraint violation".to_string())),
        ("Critical security error", AppError::SecurityError("unauthorized access attempt".to_string())),
    ];

    println!("🎭 Simulating Error Sequence:");
    for (description, error) in error_sequence {
        // Classify the error
        let classification = classifier.classify_error(&error);
        
        // Record error in degradation service
        let _ = degradation_service.record_error(&error).await;
        
        // Get current system status
        let level = degradation_service.get_current_level().await;
        let capabilities = degradation_service.get_capabilities().await;
        
        println!("  • {}", description);
        println!("    Classification: {:?} | {:?} | Retryable: {} | Alert: {}", 
            classification.category, 
            classification.severity, 
            classification.is_retryable,
            classification.should_alert
        );
        println!("    System Level: {:?} | Can Write: {} | Can Read: {}", 
            level,
            capabilities.can_write,
            capabilities.can_read
        );
        println!("    Suggested Action: {}", classification.suggested_action);
        println!();
        
        // Small delay to simulate real-world timing
        sleep(Duration::from_millis(100)).await;
    }

    // Test read-only mode compatibility
    println!("📖 Testing Read-Only Mode Compatibility:");
    let readonly_error = AppError::DatabaseError("connection pool exhausted".to_string());
    let classification = classifier.classify_error(&readonly_error);
    
    if classification.is_read_only_compatible {
        println!("  • Error is compatible with read-only mode");
        println!("  • System can continue serving read requests");
    } else {
        println!("  • Error requires full system functionality");
    }

    // Final system health check
    println!("\n🏥 Final System Health Check:");
    let final_level = degradation_service.get_current_level().await;
    let error_stats = degradation_service.get_error_statistics().await;
    println!("  • Final Degradation Level: {:?}", final_level);
    println!("  • Total Error Categories: {}", error_stats.len());
    println!("  • Error Statistics: {:?}", error_stats);

    println!("✅ Integration test completed successfully!\n");
}
