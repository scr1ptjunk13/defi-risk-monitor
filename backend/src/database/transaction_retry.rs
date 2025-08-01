use crate::error::{AppError, retry::{RetryConfig, with_retry}};
use sqlx::PgPool;
use tracing::info;

/// Transaction retry configuration specifically for database transaction operations
#[derive(Debug, Clone)]
pub struct TransactionRetryConfig {
    /// Maximum number of retry attempts for transactions
    pub max_attempts: u32,
    /// Base delay in milliseconds for transaction retries
    pub base_delay_ms: u64,
    /// Maximum delay cap for transaction retries
    pub max_delay_ms: u64,
    /// Jitter factor for randomizing delays
    pub jitter_factor: f64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Timeout for individual transaction operations in seconds
    pub transaction_timeout_secs: u64,
}

impl Default for TransactionRetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            jitter_factor: 0.2,
            backoff_multiplier: 2.0,
            transaction_timeout_secs: 30,
        }
    }
}

impl TransactionRetryConfig {
    /// Configuration optimized for deadlock scenarios
    pub fn for_deadlocks() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 50,
            max_delay_ms: 1000,
            jitter_factor: 0.3,
            backoff_multiplier: 1.5,
            transaction_timeout_secs: 10,
        }
    }

    /// Configuration optimized for serialization failures
    pub fn for_serialization_failures() -> Self {
        Self {
            max_attempts: 4,
            base_delay_ms: 200,
            max_delay_ms: 2000,
            jitter_factor: 0.25,
            backoff_multiplier: 1.8,
            transaction_timeout_secs: 20,
        }
    }

    /// Configuration for long-running transactions with timeout handling
    pub fn for_long_running() -> Self {
        Self {
            max_attempts: 2,
            base_delay_ms: 1000,
            max_delay_ms: 10000,
            jitter_factor: 0.1,
            backoff_multiplier: 2.5,
            transaction_timeout_secs: 60,
        }
    }
}

/// Classifies transaction-specific errors for retry logic
pub fn is_transaction_retryable_error(error: &AppError) -> bool {
    match error {
        AppError::DatabaseError(msg) => {
            let msg_lower = msg.to_lowercase();
            
            // Deadlock detection
            msg_lower.contains("deadlock") ||
            msg_lower.contains("lock timeout") ||
            msg_lower.contains("could not obtain lock") ||
            
            // Serialization failure detection
            msg_lower.contains("serialization failure") ||
            msg_lower.contains("could not serialize access") ||
            msg_lower.contains("concurrent update") ||
            
            // Connection/timeout issues
            msg_lower.contains("connection timeout") ||
            msg_lower.contains("query timeout") ||
            msg_lower.contains("connection reset") ||
            msg_lower.contains("connection lost") ||
            
            // Transaction-specific errors
            msg_lower.contains("transaction aborted") ||
            msg_lower.contains("transaction rolled back")
        }
        AppError::ExternalApiError(msg) => {
            msg.to_lowercase().contains("timeout")
        }
        _ => false,
    }
}

/// Enhanced transaction retry wrapper with specific error handling
pub struct TransactionRetryWrapper {
    pool: PgPool,
    config: TransactionRetryConfig,
}

impl TransactionRetryWrapper {
    pub fn new(pool: PgPool, config: TransactionRetryConfig) -> Self {
        Self { pool, config }
    }

    pub fn with_default_config(pool: PgPool) -> Self {
        Self::new(pool, TransactionRetryConfig::default())
    }

    /// Execute a simple transaction with retry logic
    pub async fn execute_simple_transaction_with_retry<T>(
        &self,
        operation_name: &str,
        operation: impl Fn() -> Result<T, AppError> + Send + Sync + Clone,
    ) -> Result<T, AppError>
    where
        T: Send,
    {
        let retry_config = RetryConfig {
            max_attempts: self.config.max_attempts,
            base_delay_ms: self.config.base_delay_ms,
            max_delay_ms: self.config.max_delay_ms,
            jitter_factor: self.config.jitter_factor,
            backoff_multiplier: self.config.backoff_multiplier,
        };

        with_retry(
            operation_name,
            retry_config,
            || async {
                info!("Starting transaction: {}", operation_name);
                let result = operation()?;
                info!("Transaction completed successfully: {}", operation_name);
                Ok(result)
            },
        ).await
    }

    /// Execute a database query with transaction retry logic
    pub async fn execute_query_with_retry<T>(
        &self,
        operation_name: &str,
        query: &str,
    ) -> Result<T, AppError>
    where
        T: Send + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Unpin,
    {
        let retry_config = RetryConfig {
            max_attempts: self.config.max_attempts,
            base_delay_ms: self.config.base_delay_ms,
            max_delay_ms: self.config.max_delay_ms,
            jitter_factor: self.config.jitter_factor,
            backoff_multiplier: self.config.backoff_multiplier,
        };

        with_retry(
            operation_name,
            retry_config,
            || async {
                info!("Executing query with retry: {}", operation_name);
                
                let result = sqlx::query_as::<_, T>(query)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| {
                        let error_msg = e.to_string();
                        if is_transaction_retryable_error(&AppError::DatabaseError(error_msg.clone())) {
                            AppError::DatabaseError(format!("Retryable query error: {}", error_msg))
                        } else {
                            AppError::DatabaseError(format!("Query error: {}", error_msg))
                        }
                    })?;

                info!("Query completed successfully: {}", operation_name);
                Ok(result)
            },
        ).await
    }

    /// Execute a simple database operation with retry logic
    pub async fn execute_operation_with_retry(
        &self,
        operation_name: &str,
        operation: impl Fn() -> Result<u64, AppError> + Send + Sync + Clone,
    ) -> Result<u64, AppError> {
        let retry_config = RetryConfig {
            max_attempts: self.config.max_attempts,
            base_delay_ms: self.config.base_delay_ms,
            max_delay_ms: self.config.max_delay_ms,
            jitter_factor: self.config.jitter_factor,
            backoff_multiplier: self.config.backoff_multiplier,
        };

        with_retry(
            operation_name,
            retry_config,
            || async {
                info!("Executing operation with retry: {}", operation_name);
                let result = operation()?;
                info!("Operation completed successfully: {}", operation_name);
                Ok(result)
            },
        ).await
    }

    /// Execute a count query with retry logic
    pub async fn execute_count_query_with_retry(
        &self,
        operation_name: &str,
        query: &str,
    ) -> Result<i64, AppError> {
        let retry_config = RetryConfig {
            max_attempts: self.config.max_attempts,
            base_delay_ms: self.config.base_delay_ms,
            max_delay_ms: self.config.max_delay_ms,
            jitter_factor: self.config.jitter_factor,
            backoff_multiplier: self.config.backoff_multiplier,
        };

        with_retry(
            operation_name,
            retry_config,
            || async {
                info!("Executing count query with retry: {}", operation_name);
                
                let result: (i64,) = sqlx::query_as(query)
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| {
                        let error_msg = e.to_string();
                        if is_transaction_retryable_error(&AppError::DatabaseError(error_msg.clone())) {
                            AppError::DatabaseError(format!("Retryable count query error: {}", error_msg))
                        } else {
                            AppError::DatabaseError(format!("Count query error: {}", error_msg))
                        }
                    })?;

                info!("Count query completed successfully: {}", operation_name);
                Ok(result.0)
            },
        ).await
    }
}

/// Convenience macros for transaction retry operations
#[macro_export]
macro_rules! retry_transaction {
    ($wrapper:expr, $name:expr, $tx_fn:expr) => {
        $wrapper.execute_transaction_with_retry($name, $tx_fn).await
    };
}

#[macro_export]
macro_rules! retry_readonly_transaction {
    ($wrapper:expr, $name:expr, $tx_fn:expr) => {
        $wrapper.execute_readonly_transaction_with_retry($name, $tx_fn).await
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_error_classification() {
        // Deadlock errors
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("deadlock detected".to_string())));
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("lock timeout exceeded".to_string())));
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("could not obtain lock on row".to_string())));

        // Serialization failure errors
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("serialization failure".to_string())));
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("could not serialize access due to concurrent update".to_string())));

        // Timeout errors
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("connection timeout".to_string())));
        assert!(is_transaction_retryable_error(&AppError::DatabaseError("query timeout".to_string())));

        // Non-retryable errors
        assert!(!is_transaction_retryable_error(&AppError::DatabaseError("syntax error".to_string())));
        assert!(!is_transaction_retryable_error(&AppError::ValidationError("invalid input".to_string())));
        assert!(!is_transaction_retryable_error(&AppError::NotFound("resource not found".to_string())));
    }

    #[test]
    fn test_retry_configs() {
        let deadlock_config = TransactionRetryConfig::for_deadlocks();
        assert_eq!(deadlock_config.max_attempts, 3);
        assert_eq!(deadlock_config.base_delay_ms, 50);

        let serialization_config = TransactionRetryConfig::for_serialization_failures();
        assert_eq!(serialization_config.max_attempts, 4);
        assert_eq!(serialization_config.base_delay_ms, 200);

        let long_running_config = TransactionRetryConfig::for_long_running();
        assert_eq!(long_running_config.max_attempts, 2);
        assert_eq!(long_running_config.transaction_timeout_secs, 60);
    }
}
