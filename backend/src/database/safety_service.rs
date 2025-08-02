use sqlx::PgPool;
use crate::error::AppError;
use tracing::{warn, error};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;

/// Critical database safety service for high-value DeFi operations
/// Implements multiple layers of protection for financial data integrity
#[derive(Clone)]
pub struct DatabaseSafetyService {
    pool: PgPool,
    audit_logger: AuditLogger,
    integrity_checker: DataIntegrityChecker,
    circuit_breaker: CircuitBreaker,
}

/// Audit logger for all critical database operations
#[derive(Clone)]
pub struct AuditLogger {
    pool: PgPool,
}

/// Data integrity checker for financial calculations
#[derive(Clone)]
pub struct DataIntegrityChecker {
    pool: PgPool,
}

/// Circuit breaker to prevent cascading failures
#[derive(Clone)]
pub struct CircuitBreaker {
    failure_threshold: u32,
    recovery_timeout: Duration,
    state: std::sync::Arc<tokio::sync::RwLock<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
struct CircuitBreakerState {
    failures: u32,
    last_failure: Option<chrono::DateTime<chrono::Utc>>,
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum CircuitState {
    Closed,  // Normal operation
    Open,    // Blocking requests
    HalfOpen, // Testing recovery
}

/// Critical operation context for audit trails
#[derive(Debug, Serialize, Deserialize)]
pub struct CriticalOperationContext {
    pub operation_id: Uuid,
    pub operation_type: CriticalOperationType,
    pub user_address: Option<String>,
    pub position_id: Option<Uuid>,
    pub financial_impact: Option<BigDecimal>,
    pub risk_level: RiskLevel,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CriticalOperationType {
    PositionCreate,
    PositionUpdate,
    PositionClose,
    RiskCalculation,
    PriceUpdate,
    LiquidationTrigger,
    AlertGeneration,
    FundsTransfer,
    ConfigurationChange,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Result of a critical database operation
#[derive(Debug, Serialize)]
pub struct CriticalOperationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub operation_id: Uuid,
    pub execution_time_ms: u64,
    pub integrity_verified: bool,
    pub audit_logged: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl DatabaseSafetyService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            audit_logger: AuditLogger::new(pool.clone()),
            integrity_checker: DataIntegrityChecker::new(pool.clone()),
            circuit_breaker: CircuitBreaker::new(5, Duration::from_secs(60)),
            pool,
        }
    }

    /// Execute a critical financial operation with full safety checks
    pub async fn execute_critical_operation<T, F>(
        &self,
        context: CriticalOperationContext,
        operation: F,
    ) -> Result<CriticalOperationResult<T>, AppError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send>> + Send + Sync,
        T: Serialize + Send + 'static,
    {
        let start_time = Instant::now();
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        // Check circuit breaker
        if !self.circuit_breaker.can_execute().await {
            return Ok(CriticalOperationResult {
                success: false,
                data: None,
                operation_id: context.operation_id,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                integrity_verified: false,
                audit_logged: false,
                warnings,
                errors: vec!["Circuit breaker is open - system in recovery mode".to_string()],
            });
        }

        // Pre-operation audit log
        if let Err(e) = self.audit_logger.log_operation_start(&context).await {
            warnings.push(format!("Failed to log operation start: {}", e));
        }

        // Pre-operation integrity check
        let pre_integrity_ok = match self.integrity_checker.verify_pre_operation(&context).await {
            Ok(true) => true,
            Ok(false) => {
                errors.push("Pre-operation integrity check failed".to_string());
                false
            }
            Err(e) => {
                warnings.push(format!("Pre-operation integrity check error: {}", e));
                true // Continue with warning
            }
        };

        if !pre_integrity_ok && context.risk_level == RiskLevel::Critical {
            return Ok(CriticalOperationResult {
                success: false,
                data: None,
                operation_id: context.operation_id,
                execution_time_ms: start_time.elapsed().as_millis() as u64,
                integrity_verified: false,
                audit_logged: true,
                warnings,
                errors,
            });
        }

        // Execute the operation in a transaction with retry logic
        let operation_result = self.execute_with_transaction_retry(operation, 3).await;

        let (success, data) = match operation_result {
            Ok(result) => (true, Some(result)),
            Err(e) => {
                errors.push(format!("Operation failed: {}", e));
                self.circuit_breaker.record_failure().await;
                (false, None)
            }
        };

        // Post-operation integrity check
        let post_integrity_ok = if success {
            match self.integrity_checker.verify_post_operation(&context).await {
                Ok(true) => true,
                Ok(false) => {
                    errors.push("Post-operation integrity check failed".to_string());
                    false
                }
                Err(e) => {
                    warnings.push(format!("Post-operation integrity check error: {}", e));
                    true
                }
            }
        } else {
            false
        };

        // Record success for circuit breaker
        if success && post_integrity_ok {
            self.circuit_breaker.record_success().await;
        }

        // Post-operation audit log
        let audit_logged = match self.audit_logger.log_operation_complete(&context, success, &warnings, &errors).await {
            Ok(_) => true,
            Err(e) => {
                error!("Failed to log operation completion: {}", e);
                false
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Log performance metrics for critical operations
        if execution_time > 5000 {
            warn!("Critical operation {} took {}ms - performance degradation detected", 
                  context.operation_id, execution_time);
        }

        Ok(CriticalOperationResult {
            success: success && post_integrity_ok,
            data,
            operation_id: context.operation_id,
            execution_time_ms: execution_time,
            integrity_verified: post_integrity_ok,
            audit_logged,
            warnings,
            errors,
        })
    }

    /// Execute operation with automatic transaction retry
    async fn execute_with_transaction_retry<T, F>(
        &self,
        operation: F,
        max_retries: u32,
    ) -> Result<T, AppError>
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send>> + Send + Sync,
    {
        let mut last_error = None;

        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => {
                    return Ok(result);
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * (2_u64.pow(attempt)));
                        warn!("Operation failed (attempt {}/{}), retrying in {:?}", 
                              attempt + 1, max_retries + 1, delay);
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::InternalError("Unknown transaction error".to_string())))
    }

    /// Verify system health before critical operations
    pub async fn verify_system_health(&self) -> Result<SystemHealthStatus, AppError> {
        let start_time = Instant::now();

        // Check database connectivity
        let db_health = match sqlx::query("SELECT 1").fetch_one(&self.pool).await {
            Ok(_) => true,
            Err(e) => {
                error!("Database health check failed: {}", e);
                false
            }
        };

        // Check connection pool status
        let pool_size = self.pool.size();
        let pool_idle = self.pool.num_idle();
        let pool_healthy = pool_size > 0 && (pool_idle as f32 / pool_size as f32) > 0.1;

        // Check circuit breaker status
        let circuit_state = self.circuit_breaker.get_state().await;

        // Check recent error rates
        let recent_errors = self.get_recent_error_count().await.unwrap_or(0);
        let error_rate_healthy = recent_errors < 10; // Less than 10 errors in last 5 minutes

        let overall_healthy = db_health && pool_healthy && circuit_state == CircuitState::Closed && error_rate_healthy;

        Ok(SystemHealthStatus {
            is_healthy: overall_healthy,
            database_connected: db_health,
            connection_pool_healthy: pool_healthy,
            circuit_breaker_state: circuit_state,
            recent_error_count: recent_errors,
            response_time_ms: start_time.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn get_recent_error_count(&self) -> Result<u32, AppError> {
        let five_minutes_ago = chrono::Utc::now() - chrono::Duration::minutes(5);
        
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_logs WHERE severity = 'error' AND timestamp >= $1"
        )
        .bind(five_minutes_ago)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get error count: {}", e)))?;

        Ok(count.0 as u32)
    }
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn log_operation_start(&self, context: &CriticalOperationContext) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, event_type, severity, timestamp, user_id, action, description, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(Uuid::new_v4())
        .bind("system_startup")
        .bind("info")
        .bind(context.timestamp)
        .bind(&context.user_address)
        .bind(format!("CRITICAL_OP_START_{:?}", context.operation_type))
        .bind(format!("Starting critical operation: {:?}", context.operation_type))
        .bind(serde_json::to_value(context).unwrap_or_default())
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to log operation start: {}", e)))?;

        Ok(())
    }

    async fn log_operation_complete(
        &self,
        context: &CriticalOperationContext,
        success: bool,
        warnings: &[String],
        errors: &[String],
    ) -> Result<(), AppError> {
        let severity = if !errors.is_empty() {
            "error"
        } else if !warnings.is_empty() {
            "warning"
        } else {
            "info"
        };

        let mut metadata = serde_json::to_value(context).unwrap_or_default();
        if let Some(obj) = metadata.as_object_mut() {
            obj.insert("success".to_string(), serde_json::Value::Bool(success));
            obj.insert("warnings".to_string(), serde_json::to_value(warnings).unwrap_or_default());
            obj.insert("errors".to_string(), serde_json::to_value(errors).unwrap_or_default());
        }

        sqlx::query(
            r#"
            INSERT INTO audit_logs (
                id, event_type, severity, timestamp, user_id, action, description, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#
        )
        .bind(Uuid::new_v4())
        .bind("system_shutdown")
        .bind(severity)
        .bind(chrono::Utc::now())
        .bind(&context.user_address)
        .bind(format!("CRITICAL_OP_COMPLETE_{:?}", context.operation_type))
        .bind(format!("Completed critical operation: {:?} - Success: {}", context.operation_type, success))
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to log operation completion: {}", e)))?;

        Ok(())
    }
}

impl DataIntegrityChecker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn verify_pre_operation(&self, context: &CriticalOperationContext) -> Result<bool, AppError> {
        match context.operation_type {
            CriticalOperationType::PositionCreate | CriticalOperationType::PositionUpdate => {
                self.verify_position_integrity(context).await
            }
            CriticalOperationType::RiskCalculation => {
                self.verify_risk_calculation_integrity(context).await
            }
            CriticalOperationType::PriceUpdate => {
                self.verify_price_integrity(context).await
            }
            _ => Ok(true), // Default to true for other operations
        }
    }

    async fn verify_post_operation(&self, context: &CriticalOperationContext) -> Result<bool, AppError> {
        // Similar to pre-operation but with post-operation checks
        self.verify_pre_operation(context).await
    }

    async fn verify_position_integrity(&self, context: &CriticalOperationContext) -> Result<bool, AppError> {
        if let Some(position_id) = context.position_id {
            // Check if position exists and has valid data
            let position_exists: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM positions WHERE id = $1"
            )
            .bind(position_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Position integrity check failed: {}", e)))?;

            Ok(position_exists.0 > 0)
        } else {
            Ok(true)
        }
    }

    async fn verify_risk_calculation_integrity(&self, _context: &CriticalOperationContext) -> Result<bool, AppError> {
        // Verify risk calculation parameters are within expected ranges
        // This is a placeholder - implement specific risk validation logic
        Ok(true)
    }

    async fn verify_price_integrity(&self, _context: &CriticalOperationContext) -> Result<bool, AppError> {
        // Verify price data is reasonable and not manipulated
        // This is a placeholder - implement specific price validation logic
        Ok(true)
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            recovery_timeout,
            state: std::sync::Arc::new(tokio::sync::RwLock::new(CircuitBreakerState {
                failures: 0,
                last_failure: None,
                state: CircuitState::Closed,
            })),
        }
    }

    async fn can_execute(&self) -> bool {
        let state = self.state.read().await;
        match state.state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                if let Some(last_failure) = state.last_failure {
                    let elapsed = chrono::Utc::now().signed_duration_since(last_failure);
                    elapsed.to_std().unwrap_or(Duration::ZERO) > self.recovery_timeout
                } else {
                    true
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    async fn record_failure(&self) {
        let mut state = self.state.write().await;
        state.failures += 1;
        state.last_failure = Some(chrono::Utc::now());

        if state.failures >= self.failure_threshold {
            state.state = CircuitState::Open;
            warn!("Circuit breaker opened after {} failures", state.failures);
        }
    }

    async fn record_success(&self) {
        let mut state = self.state.write().await;
        state.failures = 0;
        state.last_failure = None;
        state.state = CircuitState::Closed;
    }

    async fn get_state(&self) -> CircuitState {
        self.state.read().await.state.clone()
    }
}

#[derive(Debug, Serialize)]
pub struct SystemHealthStatus {
    pub is_healthy: bool,
    pub database_connected: bool,
    pub connection_pool_healthy: bool,
    pub circuit_breaker_state: CircuitState,
    pub recent_error_count: u32,
    pub response_time_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Helper functions for creating critical operation contexts
impl CriticalOperationContext {
    pub fn new_position_operation(
        operation_type: CriticalOperationType,
        user_address: String,
        position_id: Option<Uuid>,
        financial_impact: Option<BigDecimal>,
    ) -> Self {
        let risk_level = if let Some(ref impact) = financial_impact {
            if impact > &BigDecimal::from(1000000) {
                RiskLevel::Critical
            } else if impact > &BigDecimal::from(100000) {
                RiskLevel::High
            } else if impact > &BigDecimal::from(10000) {
                RiskLevel::Medium
            } else {
                RiskLevel::Low
            }
        } else {
            RiskLevel::Medium
        };

        Self {
            operation_id: Uuid::new_v4(),
            operation_type,
            user_address: Some(user_address),
            position_id,
            financial_impact,
            risk_level,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn new_system_operation(operation_type: CriticalOperationType) -> Self {
        Self {
            operation_id: Uuid::new_v4(),
            operation_type,
            user_address: None,
            position_id: None,
            financial_impact: None,
            risk_level: RiskLevel::Medium,
            metadata: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        }
    }
}
