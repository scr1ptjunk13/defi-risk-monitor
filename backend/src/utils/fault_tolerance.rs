use std::time::Duration;
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use tracing::{warn, error, info};
use crate::error::AppError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Mutex;

/// Circuit breaker state with enhanced tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl CircuitState {
    /// Convert state to numeric value for metrics
    pub fn to_metric_value(&self) -> f64 {
        match self {
            CircuitState::Closed => 0.0,
            CircuitState::Open => 1.0,
            CircuitState::HalfOpen => 2.0,
        }
    }

    /// Get human-readable state name
    pub fn as_str(&self) -> &'static str {
        match self {
            CircuitState::Closed => "closed",
            CircuitState::Open => "open",
            CircuitState::HalfOpen => "half_open",
        }
    }
}

/// Configuration for circuit breaker behavior
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: usize,
    /// Number of successes in half-open state before closing
    pub success_threshold: usize,
    /// Time to wait before transitioning from open to half-open
    pub timeout: Duration,
    /// Maximum number of concurrent requests in half-open state
    pub half_open_max_calls: usize,
    /// Minimum time between half-open state tests
    pub half_open_test_interval: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            half_open_max_calls: 1,
            half_open_test_interval: Duration::from_secs(30),
        }
    }
}

impl CircuitBreakerConfig {
    /// Configuration for critical services (more conservative)
    pub fn critical() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 5,
            timeout: Duration::from_secs(30),
            half_open_max_calls: 1,
            half_open_test_interval: Duration::from_secs(15),
        }
    }

    /// Configuration for external APIs (more tolerant)
    pub fn external_api() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 3,
            timeout: Duration::from_secs(120),
            half_open_max_calls: 2,
            half_open_test_interval: Duration::from_secs(60),
        }
    }

    /// Configuration for database operations (balanced)
    pub fn database() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(45),
            half_open_max_calls: 1,
            half_open_test_interval: Duration::from_secs(20),
        }
    }
}

/// Circuit breaker metrics for monitoring
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub total_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub rejected_calls: u64,
    pub state_transitions: u64,
    pub current_state: CircuitState,
    pub time_in_current_state: Duration,
    pub last_state_change: Option<std::time::Instant>,
    pub half_open_calls_attempted: u64,
    pub half_open_calls_successful: u64,
}

impl Default for CircuitBreakerMetrics {
    fn default() -> Self {
        Self {
            total_calls: 0,
            successful_calls: 0,
            failed_calls: 0,
            rejected_calls: 0,
            state_transitions: 0,
            current_state: CircuitState::Closed,
            time_in_current_state: Duration::from_secs(0),
            last_state_change: None,
            half_open_calls_attempted: 0,
            half_open_calls_successful: 0,
        }
    }
}

/// Enhanced circuit breaker for external service calls
pub struct ServiceCircuitBreaker {
    failure_count: AtomicUsize,
    success_count: AtomicUsize,
    half_open_calls: AtomicUsize,
    state: Arc<Mutex<CircuitState>>,
    config: CircuitBreakerConfig,
    last_failure_time: Arc<Mutex<Option<std::time::Instant>>>,
    last_state_change: Arc<Mutex<Option<std::time::Instant>>>,
    last_half_open_test: Arc<Mutex<Option<std::time::Instant>>>,
    service_name: String,
    metrics: Arc<Mutex<CircuitBreakerMetrics>>,
}

// Implement Clone manually since AtomicUsize doesn't implement Clone
impl Clone for ServiceCircuitBreaker {
    fn clone(&self) -> Self {
        Self {
            failure_count: AtomicUsize::new(self.failure_count.load(Ordering::SeqCst)),
            success_count: AtomicUsize::new(self.success_count.load(Ordering::SeqCst)),
            half_open_calls: AtomicUsize::new(self.half_open_calls.load(Ordering::SeqCst)),
            state: self.state.clone(),
            config: self.config.clone(),
            last_failure_time: self.last_failure_time.clone(),
            last_state_change: self.last_state_change.clone(),
            last_half_open_test: self.last_half_open_test.clone(),
            service_name: self.service_name.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl ServiceCircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new(service_name: &str) -> Self {
        Self::with_config(service_name, CircuitBreakerConfig::default())
    }

    /// Create a new circuit breaker with custom configuration
    pub fn with_config(service_name: &str, config: CircuitBreakerConfig) -> Self {
        info!("Initializing circuit breaker for service: {} with config: {:?}", service_name, config);
        
        let now = Some(std::time::Instant::now());
        let mut metrics = CircuitBreakerMetrics::default();
        metrics.last_state_change = now;
        
        Self {
            failure_count: AtomicUsize::new(0),
            success_count: AtomicUsize::new(0),
            half_open_calls: AtomicUsize::new(0),
            state: Arc::new(Mutex::new(CircuitState::Closed)),
            config,
            last_failure_time: Arc::new(Mutex::new(None)),
            last_state_change: Arc::new(Mutex::new(now)),
            last_half_open_test: Arc::new(Mutex::new(None)),
            service_name: service_name.to_string(),
            metrics: Arc::new(Mutex::new(metrics)),
        }
    }

    /// Create circuit breaker for critical services
    pub fn critical(service_name: &str) -> Self {
        Self::with_config(service_name, CircuitBreakerConfig::critical())
    }

    /// Create circuit breaker for external APIs
    pub fn external_api(service_name: &str) -> Self {
        Self::with_config(service_name, CircuitBreakerConfig::external_api())
    }

    /// Create circuit breaker for database operations
    pub fn database(service_name: &str) -> Self {
        Self::with_config(service_name, CircuitBreakerConfig::database())
    }

    /// Get current circuit breaker metrics
    pub async fn get_metrics(&self) -> CircuitBreakerMetrics {
        let mut metrics = self.metrics.lock().await.clone();
        let current_state = *self.state.lock().await;
        
        metrics.current_state = current_state;
        
        // Update time in current state
        if let Some(last_change) = metrics.last_state_change {
            metrics.time_in_current_state = last_change.elapsed();
        }
        
        metrics
    }

    /// Get current configuration
    pub fn get_config(&self) -> &CircuitBreakerConfig {
        &self.config
    }

    /// Force circuit breaker to a specific state (for testing)
    pub async fn force_state(&self, new_state: CircuitState) {
        let old_state = *self.state.lock().await;
        if old_state != new_state {
            *self.state.lock().await = new_state;
            *self.last_state_change.lock().await = Some(std::time::Instant::now());
            
            let mut metrics = self.metrics.lock().await;
            metrics.state_transitions += 1;
            metrics.last_state_change = Some(std::time::Instant::now());
            
            info!("Circuit breaker for service {} forced from {:?} to {:?}", 
                  self.service_name, old_state, new_state);
        }
    }

    /// Check if circuit breaker should allow a call in half-open state
    async fn can_attempt_half_open_call(&self) -> bool {
        let current_calls = self.half_open_calls.load(Ordering::SeqCst);
        
        // Check if we've exceeded max concurrent calls in half-open
        if current_calls >= self.config.half_open_max_calls {
            return false;
        }
        
        // Check minimum interval between half-open tests
        if let Some(last_test) = *self.last_half_open_test.lock().await {
            if last_test.elapsed() < self.config.half_open_test_interval {
                return false;
            }
        }
        
        true
    }

    /// Execute a function with circuit breaker protection
    pub async fn call<F, Fut, T>(&self, operation: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, AppError>>,
    {
        // Update total calls metric
        {
            let mut metrics = self.metrics.lock().await;
            metrics.total_calls += 1;
        }
        
        // Check if circuit should transition from Open to HalfOpen
        self.check_timeout().await;
        
        let current_state = *self.state.lock().await;
        
        match current_state {
            CircuitState::Open => {
                warn!("Circuit breaker is OPEN for service: {}", self.service_name);
                
                // Update rejected calls metric
                {
                    let mut metrics = self.metrics.lock().await;
                    metrics.rejected_calls += 1;
                }
                
                return Err(AppError::InternalError(format!(
                    "Service {} circuit breaker is open", self.service_name
                )));
            }
            CircuitState::HalfOpen => {
                // Enhanced half-open state testing
                if !self.can_attempt_half_open_call().await {
                    warn!("Circuit breaker HALF-OPEN for service: {} - rejecting call (max concurrent calls or test interval not met)", self.service_name);
                    
                    // Update rejected calls metric
                    {
                        let mut metrics = self.metrics.lock().await;
                        metrics.rejected_calls += 1;
                    }
                    
                    return Err(AppError::InternalError(format!(
                        "Service {} circuit breaker is half-open and not ready for new calls", self.service_name
                    )));
                }
                
                info!("Circuit breaker is HALF-OPEN for service: {}, attempting test call", self.service_name);
                
                // Track half-open call attempt
                self.half_open_calls.fetch_add(1, Ordering::SeqCst);
                *self.last_half_open_test.lock().await = Some(std::time::Instant::now());
                
                {
                    let mut metrics = self.metrics.lock().await;
                    metrics.half_open_calls_attempted += 1;
                }
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }
        
        // Execute the operation
        let result = operation().await;
        
        // Decrement half-open calls counter if we were in half-open state
        if current_state == CircuitState::HalfOpen {
            self.half_open_calls.fetch_sub(1, Ordering::SeqCst);
        }
        
        match result {
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
        
        // Update success metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.successful_calls += 1;
            if current_state == CircuitState::HalfOpen {
                metrics.half_open_calls_successful += 1;
            }
        }
        
        match current_state {
            CircuitState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::SeqCst) + 1;
                info!("Circuit breaker HALF-OPEN success {}/{} for service: {}", 
                      success_count, self.config.success_threshold, self.service_name);
                
                if success_count >= self.config.success_threshold {
                    self.transition_to_state(CircuitState::Closed).await;
                    self.failure_count.store(0, Ordering::SeqCst);
                    self.success_count.store(0, Ordering::SeqCst);
                    info!("Circuit breaker CLOSED for service: {} after {} successful half-open calls", 
                          self.service_name, success_count);
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::SeqCst);
            }
            _ => {}
        }
    }
    
    async fn on_failure(&self) {
        let current_state = *self.state.lock().await;
        let failure_count = self.failure_count.fetch_add(1, Ordering::SeqCst) + 1;
        *self.last_failure_time.lock().await = Some(std::time::Instant::now());
        
        // Update failure metrics
        {
            let mut metrics = self.metrics.lock().await;
            metrics.failed_calls += 1;
        }
        
        // Log failure with context
        warn!("Circuit breaker failure {}/{} for service: {} (state: {:?})", 
              failure_count, self.config.failure_threshold, self.service_name, current_state);
        
        // Handle state transitions based on current state
        match current_state {
            CircuitState::HalfOpen => {
                // In half-open state, any failure immediately opens the circuit
                self.transition_to_state(CircuitState::Open).await;
                warn!("Circuit breaker OPENED for service: {} - failure during half-open test", 
                      self.service_name);
            }
            CircuitState::Closed => {
                // In closed state, open after reaching failure threshold
                if failure_count >= self.config.failure_threshold {
                    self.transition_to_state(CircuitState::Open).await;
                    warn!("Circuit breaker OPENED for service: {} after {} failures", 
                          self.service_name, failure_count);
                }
            }
            CircuitState::Open => {
                // Already open, just log
                info!("Circuit breaker already OPEN for service: {} - additional failure recorded", 
                      self.service_name);
            }
        }
    }
    
    async fn check_timeout(&self) {
        let current_state = *self.state.lock().await;
        
        if current_state == CircuitState::Open {
            if let Some(last_failure) = *self.last_failure_time.lock().await {
                if last_failure.elapsed() >= self.config.timeout {
                    self.transition_to_state(CircuitState::HalfOpen).await;
                    self.success_count.store(0, Ordering::SeqCst);
                    self.half_open_calls.store(0, Ordering::SeqCst);
                    info!("Circuit breaker transitioned to HALF-OPEN for service: {} after timeout of {:?}", 
                          self.service_name, self.config.timeout);
                }
            }
        }
    }
    
    /// Internal method to handle state transitions with metrics tracking
    async fn transition_to_state(&self, new_state: CircuitState) {
        let old_state = *self.state.lock().await;
        if old_state != new_state {
            *self.state.lock().await = new_state;
            *self.last_state_change.lock().await = Some(std::time::Instant::now());
            
            // Update metrics
            {
                let mut metrics = self.metrics.lock().await;
                metrics.state_transitions += 1;
                metrics.last_state_change = Some(std::time::Instant::now());
                metrics.current_state = new_state;
            }
            
            info!("Circuit breaker for service {} transitioned from {:?} to {:?}", 
                  self.service_name, old_state, new_state);
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
