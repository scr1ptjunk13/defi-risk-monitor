use std::time::Duration;
use tokio::time::sleep;
use rand::Rng;
use tracing::{warn, debug, error};
use crate::error::AppError;

/// Configuration for exponential backoff retry logic
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_attempts: u32,
    /// Base delay for exponential backoff in milliseconds (default: 100ms)
    pub base_delay_ms: u64,
    /// Maximum delay cap in milliseconds (default: 5000ms)
    pub max_delay_ms: u64,
    /// Jitter factor to prevent thundering herd (0.0 to 1.0, default: 0.1)
    pub jitter_factor: f64,
    /// Exponential backoff multiplier (default: 2.0)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            jitter_factor: 0.1,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Create a new retry configuration with custom max attempts
    pub fn with_max_attempts(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Create a new retry configuration with custom base delay
    pub fn with_base_delay(base_delay_ms: u64) -> Self {
        Self {
            base_delay_ms,
            ..Default::default()
        }
    }

    /// Create a new retry configuration for database operations
    pub fn for_database() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 200,
            max_delay_ms: 3000,
            jitter_factor: 0.15,
            backoff_multiplier: 2.0,
        }
    }

    /// Create a new retry configuration for external API calls
    pub fn for_external_api() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 500,
            max_delay_ms: 10000,
            jitter_factor: 0.2,
            backoff_multiplier: 2.5,
        }
    }

    /// Create a new retry configuration for blockchain operations
    pub fn for_blockchain() -> Self {
        Self {
            max_attempts: 4,
            base_delay_ms: 1000,
            max_delay_ms: 8000,
            jitter_factor: 0.25,
            backoff_multiplier: 2.0,
        }
    }
}

/// Determines if an error is retryable based on its type and characteristics
pub fn is_retryable_error(error: &AppError) -> bool {
    match error {
        // Database errors that are typically transient
        AppError::DatabaseError(msg) => {
            let msg_lower = msg.to_lowercase();
            
            // Connection-related errors (retryable)
            if msg_lower.contains("connection") 
                || msg_lower.contains("timeout") 
                || msg_lower.contains("network")
                || msg_lower.contains("broken pipe")
                || msg_lower.contains("connection reset")
                || msg_lower.contains("connection refused") {
                return true;
            }
            
            // Deadlock and serialization errors (retryable)
            if msg_lower.contains("deadlock") 
                || msg_lower.contains("serialization failure")
                || msg_lower.contains("could not serialize access") {
                return true;
            }
            
            // Temporary resource exhaustion (retryable)
            if msg_lower.contains("too many connections")
                || msg_lower.contains("connection pool")
                || msg_lower.contains("resource temporarily unavailable") {
                return true;
            }
            
            // Lock timeout (retryable)
            if msg_lower.contains("lock timeout") 
                || msg_lower.contains("statement timeout") {
                return true;
            }
            
            // Constraint violations and syntax errors are NOT retryable
            if msg_lower.contains("constraint")
                || msg_lower.contains("syntax error")
                || msg_lower.contains("column")
                || msg_lower.contains("table")
                || msg_lower.contains("permission denied") {
                return false;
            }
            
            // Default to non-retryable for unknown database errors
            false
        },
        
        // External service errors (often retryable)
        AppError::ExternalServiceError(_) | AppError::ExternalApiError(_) => true,
        
        // Blockchain errors (often retryable due to network issues)
        AppError::BlockchainError(msg) => {
            let msg_lower = msg.to_lowercase();
            // Retry network-related blockchain errors
            msg_lower.contains("network") 
                || msg_lower.contains("timeout")
                || msg_lower.contains("connection")
                || msg_lower.contains("rpc")
                || msg_lower.contains("node")
        },
        
        // Rate limit errors (retryable with backoff)
        AppError::RateLimitError(_) => true,
        
        // These errors are typically not retryable
        AppError::ValidationError(_) 
        | AppError::NotFound(_) 
        | AppError::AuthenticationError(_) 
        | AppError::AuthorizationError(_) 
        | AppError::ConfigError(_) 
        | AppError::UnsupportedChain(_) => false,
        
        // Internal errors and others - default to non-retryable
        _ => false,
    }
}

/// Calculate the delay for the next retry attempt with exponential backoff and jitter
fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
    // Calculate exponential backoff delay
    let exponential_delay = config.base_delay_ms as f64 
        * config.backoff_multiplier.powi(attempt as i32);
    
    // Cap the delay at max_delay_ms
    let capped_delay = exponential_delay.min(config.max_delay_ms as f64);
    
    // Add jitter to prevent thundering herd
    let mut rng = rand::thread_rng();
    let jitter_range = capped_delay * config.jitter_factor;
    let jitter = rng.gen_range(-jitter_range..=jitter_range);
    let final_delay = (capped_delay + jitter).max(0.0) as u64;
    
    Duration::from_millis(final_delay)
}

/// Execute a future with exponential backoff retry logic
pub async fn with_retry<F, Fut, T>(
    operation_name: &str,
    config: RetryConfig,
    operation: F,
) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let mut last_error = None;
    
    for attempt in 0..config.max_attempts {
        debug!(
            operation = operation_name,
            attempt = attempt + 1,
            max_attempts = config.max_attempts,
            "Executing operation with retry logic"
        );
        
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        "Operation succeeded after retry"
                    );
                }
                return Ok(result);
            }
            Err(error) => {
                last_error = Some(error.clone());
                
                // Check if this error is retryable
                if !is_retryable_error(&error) {
                    warn!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        error = %error,
                        "Operation failed with non-retryable error"
                    );
                    return Err(error);
                }
                
                // If this is the last attempt, don't wait
                if attempt == config.max_attempts - 1 {
                    error!(
                        operation = operation_name,
                        attempt = attempt + 1,
                        max_attempts = config.max_attempts,
                        error = %error,
                        "Operation failed after all retry attempts"
                    );
                    break;
                }
                
                // Calculate delay and wait before next attempt
                let delay = calculate_delay(attempt, &config);
                warn!(
                    operation = operation_name,
                    attempt = attempt + 1,
                    max_attempts = config.max_attempts,
                    delay_ms = delay.as_millis(),
                    error = %error,
                    "Operation failed, retrying after delay"
                );
                
                sleep(delay).await;
            }
        }
    }
    
    // Return the last error if all attempts failed
    Err(last_error.unwrap_or_else(|| {
        AppError::InternalError("Retry logic failed without capturing error".to_string())
    }))
}

/// Convenience macro for retrying database operations
#[macro_export]
macro_rules! retry_db_operation {
    ($operation_name:expr, $operation:expr) => {
        $crate::error::retry::with_retry(
            $operation_name,
            $crate::error::retry::RetryConfig::for_database(),
            || async { $operation },
        ).await
    };
}

/// Convenience macro for retrying external API operations
#[macro_export]
macro_rules! retry_api_operation {
    ($operation_name:expr, $operation:expr) => {
        $crate::error::retry::with_retry(
            $operation_name,
            $crate::error::retry::RetryConfig::for_external_api(),
            || async { $operation },
        ).await
    };
}

/// Convenience macro for retrying blockchain operations
#[macro_export]
macro_rules! retry_blockchain_operation {
    ($operation_name:expr, $operation:expr) => {
        $crate::error::retry::with_retry(
            $operation_name,
            $crate::error::retry::RetryConfig::for_blockchain(),
            || async { $operation },
        ).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_is_retryable_error() {
        // Retryable database errors
        assert!(is_retryable_error(&AppError::DatabaseError("connection timeout".to_string())));
        assert!(is_retryable_error(&AppError::DatabaseError("deadlock detected".to_string())));
        assert!(is_retryable_error(&AppError::DatabaseError("too many connections".to_string())));
        
        // Non-retryable database errors
        assert!(!is_retryable_error(&AppError::DatabaseError("syntax error".to_string())));
        assert!(!is_retryable_error(&AppError::DatabaseError("constraint violation".to_string())));
        
        // Other error types
        assert!(is_retryable_error(&AppError::ExternalApiError("timeout".to_string())));
        assert!(!is_retryable_error(&AppError::ValidationError("invalid input".to_string())));
        assert!(!is_retryable_error(&AppError::NotFound("resource not found".to_string())));
    }

    #[test]
    fn test_calculate_delay() {
        let config = RetryConfig::default();
        
        let delay1 = calculate_delay(0, &config);
        let delay2 = calculate_delay(1, &config);
        let delay3 = calculate_delay(2, &config);
        
        // Delays should generally increase (accounting for jitter)
        assert!(delay1.as_millis() >= 90); // Base 100ms with jitter
        assert!(delay2.as_millis() >= 180); // ~200ms with jitter
        assert!(delay3.as_millis() >= 360); // ~400ms with jitter
        
        // Should not exceed max delay
        let long_delay = calculate_delay(10, &config);
        assert!(long_delay.as_millis() <= config.max_delay_ms as u128 * 2); // Account for jitter
    }

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let result = with_retry(
            "test_operation",
            RetryConfig::with_max_attempts(3),
            || async { Ok::<i32, AppError>(42) },
        ).await;
        
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = with_retry(
            "test_operation",
            RetryConfig::with_max_attempts(3),
            move || {
                let count = attempt_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst);
                    if current < 2 {
                        Err(AppError::DatabaseError("connection timeout".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            },
        ).await;
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_non_retryable_error() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = with_retry(
            "test_operation",
            RetryConfig::with_max_attempts(3),
            move || {
                let count = attempt_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, AppError>(AppError::ValidationError("invalid input".to_string()))
                }
            },
        ).await;
        
        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 1); // Should not retry
    }

    #[tokio::test]
    async fn test_retry_exhausted_attempts() {
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let result = with_retry(
            "test_operation",
            RetryConfig::with_max_attempts(3),
            move || {
                let count = attempt_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, AppError>(AppError::DatabaseError("connection timeout".to_string()))
                }
            },
        ).await;
        
        assert!(result.is_err());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3); // Should retry 3 times
    }
}
