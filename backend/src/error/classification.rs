use std::collections::HashMap;
use tracing::{warn, debug};
use crate::error::AppError;

/// Error classification categories for better error handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Transient errors that should be retried
    Transient,
    /// Permanent errors that should not be retried
    Permanent,
    /// Resource exhaustion errors (special handling)
    ResourceExhaustion,
    /// Constraint violation errors (data integrity issues)
    ConstraintViolation,
    /// Authentication/Authorization errors
    Security,
    /// Configuration errors
    Configuration,
    /// Rate limiting errors
    RateLimit,
    /// Read-only mode compatible errors
    ReadOnlyCompatible,
}

/// Error severity levels for monitoring and alerting
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Detailed error classification with metadata
#[derive(Debug, Clone)]
pub struct ErrorClassification {
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub is_retryable: bool,
    pub is_read_only_compatible: bool,
    pub suggested_action: String,
    pub metrics_label: String,
    pub should_alert: bool,
}

/// Enhanced error classifier with comprehensive error analysis
pub struct ErrorClassifier {
    /// Database-specific error patterns
    db_error_patterns: HashMap<String, ErrorClassification>,
    /// Blockchain-specific error patterns
    blockchain_error_patterns: HashMap<String, ErrorClassification>,
    /// External API error patterns
    api_error_patterns: HashMap<String, ErrorClassification>,
}

impl Default for ErrorClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorClassifier {
    /// Create a new error classifier with predefined patterns
    pub fn new() -> Self {
        let mut classifier = Self {
            db_error_patterns: HashMap::new(),
            blockchain_error_patterns: HashMap::new(),
            api_error_patterns: HashMap::new(),
        };
        
        classifier.initialize_patterns();
        classifier
    }
    
    /// Initialize error patterns for different error types
    fn initialize_patterns(&mut self) {
        self.initialize_database_patterns();
        self.initialize_blockchain_patterns();
        self.initialize_api_patterns();
    }
    
    /// Initialize database error patterns
    fn initialize_database_patterns(&mut self) {
        // Connection errors (transient, retryable)
        let connection_errors = vec![
            "connection", "timeout", "network", "broken pipe", 
            "connection reset", "connection refused", "connection lost"
        ];
        for pattern in connection_errors {
            self.db_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Medium,
                is_retryable: true,
                is_read_only_compatible: true,
                suggested_action: "Retry with exponential backoff".to_string(),
                metrics_label: "database_connection_error".to_string(),
                should_alert: false,
            });
        }
        
        // Deadlock and serialization errors (transient, retryable)
        let concurrency_errors = vec![
            "deadlock", "serialization failure", "could not serialize access",
            "concurrent update", "transaction isolation"
        ];
        for pattern in concurrency_errors {
            self.db_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Medium,
                is_retryable: true,
                is_read_only_compatible: false, // Write operations involved
                suggested_action: "Retry with transaction isolation".to_string(),
                metrics_label: "database_concurrency_error".to_string(),
                should_alert: false,
            });
        }
        
        // Resource exhaustion (special handling)
        let resource_errors = vec![
            "too many connections", "connection pool", "resource temporarily unavailable",
            "out of memory", "disk full", "lock timeout", "statement timeout"
        ];
        for pattern in resource_errors {
            self.db_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::ResourceExhaustion,
                severity: ErrorSeverity::High,
                is_retryable: true,
                is_read_only_compatible: true,
                suggested_action: "Retry with longer delay and circuit breaker".to_string(),
                metrics_label: "database_resource_exhaustion".to_string(),
                should_alert: true,
            });
        }
        
        // Constraint violations (permanent, not retryable)
        let constraint_errors = vec![
            "unique constraint", "foreign key constraint", "check constraint",
            "not null constraint", "primary key", "duplicate key"
        ];
        for pattern in constraint_errors {
            self.db_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::ConstraintViolation,
                severity: ErrorSeverity::Medium,
                is_retryable: false,
                is_read_only_compatible: false,
                suggested_action: "Fix data integrity issue".to_string(),
                metrics_label: "database_constraint_violation".to_string(),
                should_alert: true,
            });
        }
        
        // Permission and syntax errors (permanent)
        let permanent_errors = vec![
            "permission denied", "syntax error", "column", "table", 
            "function", "relation does not exist", "invalid"
        ];
        for pattern in permanent_errors {
            self.db_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Permanent,
                severity: ErrorSeverity::High,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Fix query or schema issue".to_string(),
                metrics_label: "database_permanent_error".to_string(),
                should_alert: true,
            });
        }
    }
    
    /// Initialize blockchain error patterns
    fn initialize_blockchain_patterns(&mut self) {
        // Network-related blockchain errors (retryable)
        let network_errors = vec![
            "network", "timeout", "connection", "rpc", "node", "endpoint",
            "rate limit", "too many requests", "service unavailable"
        ];
        for pattern in network_errors {
            self.blockchain_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Medium,
                is_retryable: true,
                is_read_only_compatible: true,
                suggested_action: "Retry with different RPC endpoint".to_string(),
                metrics_label: "blockchain_network_error".to_string(),
                should_alert: false,
            });
        }
        
        // Gas and transaction errors (permanent for specific tx)
        let gas_errors = vec![
            "gas", "nonce", "insufficient funds", "transaction failed",
            "execution reverted", "out of gas"
        ];
        for pattern in gas_errors {
            self.blockchain_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Permanent,
                severity: ErrorSeverity::Medium,
                is_retryable: false,
                is_read_only_compatible: true, // Read operations not affected
                suggested_action: "Adjust gas parameters or check transaction".to_string(),
                metrics_label: "blockchain_transaction_error".to_string(),
                should_alert: true,
            });
        }
    }
    
    /// Initialize API error patterns
    fn initialize_api_patterns(&mut self) {
        // Rate limiting (retryable with backoff)
        self.api_error_patterns.insert("rate limit".to_string(), ErrorClassification {
            category: ErrorCategory::RateLimit,
            severity: ErrorSeverity::Medium,
            is_retryable: true,
            is_read_only_compatible: true,
            suggested_action: "Retry with exponential backoff".to_string(),
            metrics_label: "api_rate_limit".to_string(),
            should_alert: false,
        });
        
        // Service unavailable (retryable)
        let service_errors = vec![
            "service unavailable", "internal server error", "bad gateway",
            "gateway timeout", "connection timeout"
        ];
        for pattern in service_errors {
            self.api_error_patterns.insert(pattern.to_string(), ErrorClassification {
                category: ErrorCategory::Transient,
                severity: ErrorSeverity::Medium,
                is_retryable: true,
                is_read_only_compatible: true,
                suggested_action: "Retry with circuit breaker".to_string(),
                metrics_label: "api_service_error".to_string(),
                should_alert: false,
            });
        }
    }
    
    /// Classify an error and return detailed classification
    pub fn classify_error(&self, error: &AppError) -> ErrorClassification {
        match error {
            AppError::DatabaseError(msg) => self.classify_database_error(msg),
            AppError::BlockchainError(msg) => self.classify_blockchain_error(msg),
            AppError::ExternalServiceError(msg) | AppError::ExternalApiError(msg) => {
                self.classify_api_error(msg)
            },
            AppError::ValidationError(_) => ErrorClassification {
                category: ErrorCategory::Permanent,
                severity: ErrorSeverity::Low,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Fix input validation".to_string(),
                metrics_label: "validation_error".to_string(),
                should_alert: false,
            },
            AppError::NotFound(_) => ErrorClassification {
                category: ErrorCategory::Permanent,
                severity: ErrorSeverity::Low,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Check resource existence".to_string(),
                metrics_label: "not_found_error".to_string(),
                should_alert: false,
            },
            AppError::AuthenticationError(_) => ErrorClassification {
                category: ErrorCategory::Security,
                severity: ErrorSeverity::High,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Check authentication credentials".to_string(),
                metrics_label: "authentication_error".to_string(),
                should_alert: true,
            },
            AppError::AuthorizationError(_) => ErrorClassification {
                category: ErrorCategory::Security,
                severity: ErrorSeverity::High,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Check user permissions".to_string(),
                metrics_label: "authorization_error".to_string(),
                should_alert: true,
            },
            AppError::RateLimitError(_) => ErrorClassification {
                category: ErrorCategory::RateLimit,
                severity: ErrorSeverity::Medium,
                is_retryable: true,
                is_read_only_compatible: true,
                suggested_action: "Retry with exponential backoff".to_string(),
                metrics_label: "rate_limit_error".to_string(),
                should_alert: false,
            },
            AppError::ConfigError(_) => ErrorClassification {
                category: ErrorCategory::Configuration,
                severity: ErrorSeverity::Critical,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Fix configuration issue".to_string(),
                metrics_label: "configuration_error".to_string(),
                should_alert: true,
            },
            AppError::UnsupportedChain(_) => ErrorClassification {
                category: ErrorCategory::Configuration,
                severity: ErrorSeverity::Medium,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Add chain support or use supported chain".to_string(),
                metrics_label: "unsupported_chain_error".to_string(),
                should_alert: false,
            },
            _ => ErrorClassification {
                category: ErrorCategory::Permanent,
                severity: ErrorSeverity::Medium,
                is_retryable: false,
                is_read_only_compatible: true,
                suggested_action: "Investigate unknown error".to_string(),
                metrics_label: "unknown_error".to_string(),
                should_alert: true,
            },
        }
    }
    
    /// Classify database-specific errors
    fn classify_database_error(&self, msg: &str) -> ErrorClassification {
        let msg_lower = msg.to_lowercase();
        
        // Check for specific patterns in order of specificity (longer patterns first)
        let mut patterns: Vec<_> = self.db_error_patterns.iter().collect();
        patterns.sort_by(|a, b| b.0.len().cmp(&a.0.len())); // Sort by pattern length descending
        
        for (pattern, classification) in patterns {
            if msg_lower.contains(pattern) {
                debug!("Classified database error '{}' as {:?}", msg, classification.category);
                return classification.clone();
            }
        }
        
        // Default classification for unknown database errors
        warn!("Unknown database error pattern: {}", msg);
        ErrorClassification {
            category: ErrorCategory::Permanent,
            severity: ErrorSeverity::Medium,
            is_retryable: false,
            is_read_only_compatible: true,
            suggested_action: "Investigate database error".to_string(),
            metrics_label: "database_unknown_error".to_string(),
            should_alert: true,
        }
    }
    
    /// Classify blockchain-specific errors
    fn classify_blockchain_error(&self, msg: &str) -> ErrorClassification {
        let msg_lower = msg.to_lowercase();
        
        // Check for specific patterns
        for (pattern, classification) in &self.blockchain_error_patterns {
            if msg_lower.contains(pattern) {
                debug!("Classified blockchain error '{}' as {:?}", msg, classification.category);
                return classification.clone();
            }
        }
        
        // Default classification for unknown blockchain errors
        warn!("Unknown blockchain error pattern: {}", msg);
        ErrorClassification {
            category: ErrorCategory::Transient,
            severity: ErrorSeverity::Medium,
            is_retryable: true,
            is_read_only_compatible: true,
            suggested_action: "Retry blockchain operation".to_string(),
            metrics_label: "blockchain_unknown_error".to_string(),
            should_alert: false,
        }
    }
    
    /// Classify API-specific errors
    fn classify_api_error(&self, msg: &str) -> ErrorClassification {
        let msg_lower = msg.to_lowercase();
        
        // Check for specific patterns
        for (pattern, classification) in &self.api_error_patterns {
            if msg_lower.contains(pattern) {
                debug!("Classified API error '{}' as {:?}", msg, classification.category);
                return classification.clone();
            }
        }
        
        // Default classification for unknown API errors
        warn!("Unknown API error pattern: {}", msg);
        ErrorClassification {
            category: ErrorCategory::Transient,
            severity: ErrorSeverity::Medium,
            is_retryable: true,
            is_read_only_compatible: true,
            suggested_action: "Retry API call".to_string(),
            metrics_label: "api_unknown_error".to_string(),
            should_alert: false,
        }
    }
    
    /// Check if an error is retryable (enhanced version)
    pub fn is_retryable(&self, error: &AppError) -> bool {
        let classification = self.classify_error(error);
        classification.is_retryable
    }
    
    /// Check if an error is compatible with read-only mode
    pub fn is_read_only_compatible(&self, error: &AppError) -> bool {
        let classification = self.classify_error(error);
        classification.is_read_only_compatible
    }
    
    /// Get suggested action for an error
    pub fn get_suggested_action(&self, error: &AppError) -> String {
        let classification = self.classify_error(error);
        classification.suggested_action
    }
    
    /// Check if an error should trigger an alert
    pub fn should_alert(&self, error: &AppError) -> bool {
        let classification = self.classify_error(error);
        classification.should_alert
    }
    
    /// Get metrics label for an error
    pub fn get_metrics_label(&self, error: &AppError) -> String {
        let classification = self.classify_error(error);
        classification.metrics_label
    }
}

/// Global error classifier instance
static mut ERROR_CLASSIFIER: Option<ErrorClassifier> = None;
static INIT: std::sync::Once = std::sync::Once::new();

/// Get the global error classifier instance
pub fn get_error_classifier() -> &'static ErrorClassifier {
    unsafe {
        INIT.call_once(|| {
            ERROR_CLASSIFIER = Some(ErrorClassifier::new());
        });
        ERROR_CLASSIFIER.as_ref().unwrap()
    }
}

/// Enhanced error classification function that replaces the basic retry logic
pub fn classify_error(error: &AppError) -> ErrorClassification {
    get_error_classifier().classify_error(error)
}

/// Enhanced retryability check
pub fn is_retryable_error_enhanced(error: &AppError) -> bool {
    get_error_classifier().is_retryable(error)
}

/// Check if operation can be performed in read-only mode
pub fn is_read_only_compatible_error(error: &AppError) -> bool {
    get_error_classifier().is_read_only_compatible(error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_error_classification() {
        let classifier = ErrorClassifier::new();
        
        // Test connection error
        let conn_error = AppError::DatabaseError("connection timeout".to_string());
        let classification = classifier.classify_error(&conn_error);
        assert_eq!(classification.category, ErrorCategory::Transient);
        assert!(classification.is_retryable);
        
        // Test constraint violation
        let constraint_error = AppError::DatabaseError("unique constraint violation".to_string());
        let classification = classifier.classify_error(&constraint_error);
        assert_eq!(classification.category, ErrorCategory::ConstraintViolation);
        assert!(!classification.is_retryable);
        
        // Test deadlock
        let deadlock_error = AppError::DatabaseError("deadlock detected".to_string());
        let classification = classifier.classify_error(&deadlock_error);
        assert_eq!(classification.category, ErrorCategory::Transient);
        assert!(classification.is_retryable);
    }
    
    #[test]
    fn test_blockchain_error_classification() {
        let classifier = ErrorClassifier::new();
        
        // Test network error
        let network_error = AppError::BlockchainError("network timeout".to_string());
        let classification = classifier.classify_error(&network_error);
        assert_eq!(classification.category, ErrorCategory::Transient);
        assert!(classification.is_retryable);
        
        // Test gas error
        let gas_error = AppError::BlockchainError("out of gas".to_string());
        let classification = classifier.classify_error(&gas_error);
        assert_eq!(classification.category, ErrorCategory::Permanent);
        assert!(!classification.is_retryable);
    }
    
    #[test]
    fn test_read_only_compatibility() {
        let classifier = ErrorClassifier::new();
        
        // Connection errors should be read-only compatible
        let conn_error = AppError::DatabaseError("connection timeout".to_string());
        assert!(classifier.is_read_only_compatible(&conn_error));
        
        // Serialization errors should not be read-only compatible (write operations)
        let serial_error = AppError::DatabaseError("serialization failure".to_string());
        assert!(!classifier.is_read_only_compatible(&serial_error));
        
        // Validation errors should be read-only compatible
        let validation_error = AppError::ValidationError("invalid input".to_string());
        assert!(classifier.is_read_only_compatible(&validation_error));
    }
}
