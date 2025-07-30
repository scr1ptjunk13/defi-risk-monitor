use std::time::Duration;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{warn, error, info};
use crate::error::AppError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;

/// Simple circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Circuit breaker for external service calls
pub struct ServiceCircuitBreaker {
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    state: Arc<Mutex<CircuitState>>,
    failure_threshold: usize,
    success_threshold: usize,
    timeout: Duration,
    last_failure_time: Arc<Mutex<Option<std::time::Instant>>>,
    service_name: String,
}

// Implement Clone manually since AtomicUsize doesn't implement Clone
impl Clone for ServiceCircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            failure_count: AtomicUsize::new(self.failure_count.load(Ordering::SeqCst)),
            success_count: AtomicUsize::new(self.success_count.load(Ordering::SeqCst)),
            state: self.state.clone(),
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
            timeout: self.timeout,
            last_failure_time: self.last_failure_time.clone(),
            service_name: self.service_name.clone(),
        }
    }
}

impl ServiceCircuitBreaker {
    /// Create a new circuit breaker with production-grade settings
    pub fn new(service_name: &str) -> Self {
        info!("Initializing circuit breaker for service: {}", service_name);
        
        Self {
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            last_failure_time: Arc::new(Mutex::new(None)),
            service_name: service_name.to_string(),
        }
    }

    /// Execute a function with circuit breaker protection
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        // Check if circuit should transition from Open to HalfOpen
        self.check_timeout().await;
        
        let current_state = *self.state.lock().await;
        
        match current_state {
            CircuitState::Open => {
                warn!("Circuit breaker is OPEN for service: {}", self.service_name);
                return Err(AppError::InternalError(format!(
                    "Service {} circuit breaker is open", self.service_name
                )));
            }
            CircuitState::HalfOpen => {
                info!("Circuit breaker is HALF-OPEN for service: {}, trying operation", self.service_name);
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
        
        // Execute the operation
        match operation().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }
    
    async fn on_success(&self) {
        let current_state = *self.state.lock().await;
        
        match current_state {
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                if success_count >= self.success_threshold {
                    *self.state.lock().await = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!("Circuit breaker CLOSED for service: {}", self.service_name);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.lock().await = Some(std::time::Instant::now());
        
        if failure_count >= self.failure_threshold {
            *self.state.lock().await = CircuitState::Open;
            warn!("Circuit breaker OPENED for service: {} after {} failures", 
                  self.service_name, failure_count);
        }
    }
    
    async fn check_timeout(&self) {
        let current_state = *self.state.lock().await;
        
        if current_state == CircuitState::Open {
            if let Some(last_failure) = *self.last_failure_time.lock().await {
                if last_failure.elapsed() >= self.timeout {
                    *self.state.lock().await = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::SeqCst);
                    info!("Circuit breaker transitioned to HALF-OPEN for service: {}", self.service_name);
                }
            }
        }
    }
}

/// Retry configuration for different types of operations
#[derive(Clone)]
pub struct RetryConfig {
    pub max_retries: usize,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

impl RetryConfig {
    /// Configuration for blockchain RPC calls (more aggressive)
    pub fn blockchain_rpc() -> Self {
        Self {
            max_retries: 5,
            initial_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }

    /// Configuration for price API calls (moderate)
    pub fn price_api() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(15),
            multiplier: 1.5,
        }
    }

    /// Configuration for database operations (conservative)
    pub fn database() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(50),
            max_delay: Duration::from_secs(5),
            multiplier: 2.0,
        }
    }
}

/// Execute an operation with exponential backoff retry
pub async fn retry_with_backoff<F, Fut, T>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, AppError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, AppError>>,
{
    let retry_strategy = ExponentialBackoff::from_millis(config.initial_delay.as_millis() as u64)
        .max_delay(config.max_delay)
        .take(config.max_retries);

    info!("Starting retry operation: {} (max_retries: {})", operation_name, config.max_retries);

    let result = Retry::spawn(retry_strategy, || async {
        match operation().await {
            Ok(value) => Ok(value),
            Err(e) => {
                warn!("Operation {} failed, will retry: {}", operation_name, e);
                Err(e)
            }
        }
    }).await;

    match &result {
        Ok(_) => info!("Operation {} succeeded", operation_name),
        Err(e) => error!("Operation {} failed after all retries: {}", operation_name, e),
    }

    result
}

/// Combined circuit breaker + retry wrapper for critical operations
#[derive(Clone)]
pub struct FaultTolerantService {
    circuit_breaker: ServiceCircuitBreaker,
    retry_config: RetryConfig,
    service_name: String,
}

impl FaultTolerantService {
    pub fn new(service_name: &str, retry_config: RetryConfig) -> Self {
        Self {
            circuit_breaker: ServiceCircuitBreaker::new(service_name),
            retry_config,
            service_name: service_name.to_string(),
        }
    }

    /// Execute operation with both circuit breaker and retry protection
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, AppError>
    where
        F: Fn() -> Fut + Clone,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        let operation_name = format!("{}_operation", self.service_name);
        
        self.circuit_breaker.call(|| async {
            retry_with_backoff(
                operation.clone(),
                self.retry_config.clone(),
                &operation_name,
            ).await
        }).await
    }
}

/// Timeout wrapper for operations
pub async fn with_timeout<F, T>(
    future: F,
    timeout: Duration,
    operation_name: &str,
) -> Result<T, AppError>
where
    F: std::future::Future<Output = Result<T, AppError>>,
{
    match tokio::time::timeout(timeout, future).await {
        Ok(result) => result,
        Err(_) => {
            error!("Operation {} timed out after {:?}", operation_name, timeout);
            Err(AppError::InternalError(format!("Operation {} timed out", operation_name)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_with_backoff_success() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry_with_backoff(
            move || {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(AppError::InternalError("Temporary failure".to_string()))
                    } else {
                        Ok("Success".to_string())
                    }
                }
            },
            RetryConfig::default(),
            "test_operation",
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_timeout_wrapper() {
        let result = with_timeout(
            async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok::<String, AppError>("Success".to_string())
            },
            Duration::from_millis(50),
            "test_timeout",
        ).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }
}
