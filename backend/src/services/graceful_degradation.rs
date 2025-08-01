use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use crate::error::{AppError, classification::{classify_error, ErrorCategory, ErrorSeverity}};

/// Service degradation levels
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DegradationLevel {
    /// Full functionality available
    Normal,
    /// Some write operations disabled, reads available
    ReadOnlyMode,
    /// Limited read operations only
    LimitedMode,
    /// Emergency mode - minimal functionality
    EmergencyMode,
}

/// Service capability flags
#[derive(Debug, Clone)]
pub struct ServiceCapabilities {
    pub can_write: bool,
    pub can_read: bool,
    pub can_calculate_risk: bool,
    pub can_send_alerts: bool,
    pub can_fetch_prices: bool,
    pub can_query_blockchain: bool,
    pub max_concurrent_requests: usize,
}

impl ServiceCapabilities {
    /// Full capabilities (normal mode)
    pub fn full() -> Self {
        Self {
            can_write: true,
            can_read: true,
            can_calculate_risk: true,
            can_send_alerts: true,
            can_fetch_prices: true,
            can_query_blockchain: true,
            max_concurrent_requests: 1000,
        }
    }
    
    /// Read-only capabilities
    pub fn read_only() -> Self {
        Self {
            can_write: false,
            can_read: true,
            can_calculate_risk: true,
            can_send_alerts: false, // No alerts in read-only mode
            can_fetch_prices: true,
            can_query_blockchain: true,
            max_concurrent_requests: 500,
        }
    }
    
    /// Limited capabilities
    pub fn limited() -> Self {
        Self {
            can_write: false,
            can_read: true,
            can_calculate_risk: false, // No complex calculations
            can_send_alerts: false,
            can_fetch_prices: false, // Use cached prices only
            can_query_blockchain: false, // Use cached data only
            max_concurrent_requests: 100,
        }
    }
    
    /// Emergency capabilities (minimal)
    pub fn emergency() -> Self {
        Self {
            can_write: false,
            can_read: true, // Basic reads only
            can_calculate_risk: false,
            can_send_alerts: false,
            can_fetch_prices: false,
            can_query_blockchain: false,
            max_concurrent_requests: 50,
        }
    }
}

/// Graceful degradation service for handling service failures
pub struct GracefulDegradationService {
    /// Current degradation level
    current_level: Arc<RwLock<DegradationLevel>>,
    /// Service capabilities based on current level
    capabilities: Arc<RwLock<ServiceCapabilities>>,
    /// Error counters by category
    error_counters: Arc<RwLock<HashMap<String, u64>>>,
    /// Manual override flag
    manual_override: AtomicBool,
    /// Auto-recovery enabled
    auto_recovery_enabled: AtomicBool,
    /// Degradation thresholds
    degradation_thresholds: DegradationThresholds,
}

/// Thresholds for automatic degradation
#[derive(Debug, Clone)]
pub struct DegradationThresholds {
    /// Database error threshold for read-only mode
    pub database_error_threshold: u64,
    /// Critical error threshold for limited mode
    pub critical_error_threshold: u64,
    /// Emergency error threshold
    pub emergency_error_threshold: u64,
    /// Time window for error counting (seconds)
    pub error_window_seconds: u64,
}

impl Default for DegradationThresholds {
    fn default() -> Self {
        Self {
            database_error_threshold: 10,
            critical_error_threshold: 20,
            emergency_error_threshold: 50,
            error_window_seconds: 300, // 5 minutes
        }
    }
}

impl GracefulDegradationService {
    /// Create a new graceful degradation service
    pub fn new() -> Self {
        Self {
            current_level: Arc::new(RwLock::new(DegradationLevel::Normal)),
            capabilities: Arc::new(RwLock::new(ServiceCapabilities::full())),
            error_counters: Arc::new(RwLock::new(HashMap::new())),
            manual_override: AtomicBool::new(false),
            auto_recovery_enabled: AtomicBool::new(true),
            degradation_thresholds: DegradationThresholds::default(),
        }
    }
    
    /// Create with custom thresholds
    pub fn with_thresholds(thresholds: DegradationThresholds) -> Self {
        Self {
            current_level: Arc::new(RwLock::new(DegradationLevel::Normal)),
            capabilities: Arc::new(RwLock::new(ServiceCapabilities::full())),
            error_counters: Arc::new(RwLock::new(HashMap::new())),
            manual_override: AtomicBool::new(false),
            auto_recovery_enabled: AtomicBool::new(true),
            degradation_thresholds: thresholds,
        }
    }
    
    /// Get current degradation level
    pub async fn get_current_level(&self) -> DegradationLevel {
        self.current_level.read().await.clone()
    }
    
    /// Get current service capabilities
    pub async fn get_capabilities(&self) -> ServiceCapabilities {
        self.capabilities.read().await.clone()
    }
    
    /// Check if a specific capability is available
    pub async fn can_perform_operation(&self, operation: &str) -> bool {
        let capabilities = self.capabilities.read().await;
        
        match operation {
            "write" => capabilities.can_write,
            "read" => capabilities.can_read,
            "calculate_risk" => capabilities.can_calculate_risk,
            "send_alerts" => capabilities.can_send_alerts,
            "fetch_prices" => capabilities.can_fetch_prices,
            "query_blockchain" => capabilities.can_query_blockchain,
            _ => true, // Unknown operations allowed by default
        }
    }
    
    /// Record an error and potentially trigger degradation
    pub async fn record_error(&self, error: &AppError) -> Result<(), AppError> {
        let classification = classify_error(error);
        
        // Update error counters
        {
            let mut counters = self.error_counters.write().await;
            let counter = counters.entry(classification.metrics_label.clone()).or_insert(0);
            *counter += 1;
        }
        
        // Log error with classification
        match classification.severity {
            ErrorSeverity::Critical => {
                error!("Critical error recorded: {} (category: {:?})", error, classification.category);
            },
            ErrorSeverity::High => {
                warn!("High severity error recorded: {} (category: {:?})", error, classification.category);
            },
            ErrorSeverity::Medium => {
                info!("Medium severity error recorded: {} (category: {:?})", error, classification.category);
            },
            ErrorSeverity::Low => {
                debug!("Low severity error recorded: {} (category: {:?})", error, classification.category);
            },
        }
        
        // Check if degradation is needed (only if not manually overridden)
        if !self.manual_override.load(Ordering::Relaxed) {
            self.evaluate_degradation_need().await?;
        }
        
        Ok(())
    }
    
    /// Evaluate if service degradation is needed based on error patterns
    async fn evaluate_degradation_need(&self) -> Result<(), AppError> {
        let counters = self.error_counters.read().await;
        let current_level = self.current_level.read().await.clone();
        
        // Count different types of errors
        let database_errors = counters.get("database_connection_error").unwrap_or(&0) +
                             counters.get("database_resource_exhaustion").unwrap_or(&0) +
                             counters.get("database_permanent_error").unwrap_or(&0);
        
        let critical_errors = counters.values().filter(|&&count| count > 0).count() as u64;
        
        let total_errors: u64 = counters.values().sum();
        
        // Determine appropriate degradation level
        let suggested_level = if total_errors >= self.degradation_thresholds.emergency_error_threshold {
            DegradationLevel::EmergencyMode
        } else if critical_errors >= self.degradation_thresholds.critical_error_threshold {
            DegradationLevel::LimitedMode
        } else if database_errors >= self.degradation_thresholds.database_error_threshold {
            DegradationLevel::ReadOnlyMode
        } else {
            DegradationLevel::Normal
        };
        
        // Only degrade, don't automatically recover (unless explicitly enabled)
        if suggested_level != current_level && 
           (suggested_level.clone() as u8) > (current_level.clone() as u8) {
            drop(counters);
            self.set_degradation_level(suggested_level.clone()).await?;
        }
        
        Ok(())
    }
    
    /// Manually set degradation level
    pub async fn set_degradation_level(&self, level: DegradationLevel) -> Result<(), AppError> {
        let old_level = {
            let mut current = self.current_level.write().await;
            let old = current.clone();
            *current = level.clone();
            old
        };
        
        // Update capabilities based on new level
        {
            let mut capabilities = self.capabilities.write().await;
            *capabilities = match level {
                DegradationLevel::Normal => ServiceCapabilities::full(),
                DegradationLevel::ReadOnlyMode => ServiceCapabilities::read_only(),
                DegradationLevel::LimitedMode => ServiceCapabilities::limited(),
                DegradationLevel::EmergencyMode => ServiceCapabilities::emergency(),
            };
        }
        
        if old_level != level {
            warn!("Service degradation level changed from {:?} to {:?}", old_level, level);
            
            // Clear error counters when degrading to give the system a fresh start
            if (level as u8) > (old_level as u8) {
                let mut counters = self.error_counters.write().await;
                counters.clear();
                info!("Error counters cleared due to degradation");
            }
        }
        
        Ok(())
    }
    
    /// Enable manual override (prevents automatic degradation)
    pub fn enable_manual_override(&self) {
        self.manual_override.store(true, Ordering::Relaxed);
        info!("Manual override enabled - automatic degradation disabled");
    }
    
    /// Disable manual override (allows automatic degradation)
    pub fn disable_manual_override(&self) {
        self.manual_override.store(false, Ordering::Relaxed);
        info!("Manual override disabled - automatic degradation enabled");
    }
    
    /// Enable auto-recovery
    pub fn enable_auto_recovery(&self) {
        self.auto_recovery_enabled.store(true, Ordering::Relaxed);
        info!("Auto-recovery enabled");
    }
    
    /// Disable auto-recovery
    pub fn disable_auto_recovery(&self) {
        self.auto_recovery_enabled.store(false, Ordering::Relaxed);
        info!("Auto-recovery disabled");
    }
    
    /// Attempt to recover to normal mode (if conditions are met)
    pub async fn attempt_recovery(&self) -> Result<bool, AppError> {
        if !self.auto_recovery_enabled.load(Ordering::Relaxed) {
            return Ok(false);
        }
        
        let current_level = self.current_level.read().await.clone();
        
        if current_level == DegradationLevel::Normal {
            return Ok(false); // Already at normal level
        }
        
        // Check if error conditions have improved
        let counters = self.error_counters.read().await;
        let total_errors: u64 = counters.values().sum();
        
        // Only recover if error count is low
        if total_errors < self.degradation_thresholds.database_error_threshold / 2 {
            drop(counters);
            
            // Try to recover one level at a time
            let recovery_level = match current_level {
                DegradationLevel::EmergencyMode => DegradationLevel::LimitedMode,
                DegradationLevel::LimitedMode => DegradationLevel::ReadOnlyMode,
                DegradationLevel::ReadOnlyMode => DegradationLevel::Normal,
                DegradationLevel::Normal => return Ok(false),
            };
            
            let recovery_level_clone = recovery_level.clone();
            self.set_degradation_level(recovery_level).await?;
            info!("Service recovery attempted from {:?} to {:?}", current_level, recovery_level_clone);
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Get error statistics
    pub async fn get_error_statistics(&self) -> HashMap<String, u64> {
        self.error_counters.read().await.clone()
    }
    
    /// Clear error counters (useful for testing or manual recovery)
    pub async fn clear_error_counters(&self) {
        let mut counters = self.error_counters.write().await;
        counters.clear();
        info!("Error counters manually cleared");
    }
    
    /// Check if an operation should be allowed based on error classification
    pub async fn should_allow_operation(&self, error: &AppError, operation: &str) -> bool {
        let classification = classify_error(error);
        
        // If error is read-only compatible and we're trying a read operation, allow it
        if classification.is_read_only_compatible && operation == "read" {
            return true;
        }
        
        // If error is not retryable and it's a write operation, don't allow
        if !classification.is_retryable && operation == "write" {
            return false;
        }
        
        // Check current capabilities
        self.can_perform_operation(operation).await
    }
    
    /// Get degradation status summary
    pub async fn get_status_summary(&self) -> DegradationStatusSummary {
        let level = self.current_level.read().await.clone();
        let capabilities = self.capabilities.read().await.clone();
        let error_stats = self.error_counters.read().await.clone();
        
        DegradationStatusSummary {
            current_level: level,
            capabilities,
            error_statistics: error_stats,
            manual_override_enabled: self.manual_override.load(Ordering::Relaxed),
            auto_recovery_enabled: self.auto_recovery_enabled.load(Ordering::Relaxed),
            thresholds: self.degradation_thresholds.clone(),
        }
    }
}

/// Summary of degradation service status
#[derive(Debug, Clone)]
pub struct DegradationStatusSummary {
    pub current_level: DegradationLevel,
    pub capabilities: ServiceCapabilities,
    pub error_statistics: HashMap<String, u64>,
    pub manual_override_enabled: bool,
    pub auto_recovery_enabled: bool,
    pub thresholds: DegradationThresholds,
}

impl Default for GracefulDegradationService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_normal_operation() {
        let service = GracefulDegradationService::new();
        
        assert_eq!(service.get_current_level().await, DegradationLevel::Normal);
        assert!(service.can_perform_operation("write").await);
        assert!(service.can_perform_operation("read").await);
    }
    
    #[tokio::test]
    async fn test_degradation_on_database_errors() {
        let mut thresholds = DegradationThresholds::default();
        thresholds.database_error_threshold = 2; // Low threshold for testing
        
        let service = GracefulDegradationService::with_thresholds(thresholds);
        
        // Record multiple database errors
        for _ in 0..3 {
            let error = AppError::DatabaseError("connection timeout".to_string());
            service.record_error(&error).await.unwrap();
        }
        
        // Should have degraded to read-only mode
        assert_eq!(service.get_current_level().await, DegradationLevel::ReadOnlyMode);
        assert!(!service.can_perform_operation("write").await);
        assert!(service.can_perform_operation("read").await);
    }
    
    #[tokio::test]
    async fn test_manual_override() {
        let service = GracefulDegradationService::new();
        
        // Enable manual override
        service.enable_manual_override();
        
        // Record errors that would normally trigger degradation
        for _ in 0..20 {
            let error = AppError::DatabaseError("connection timeout".to_string());
            service.record_error(&error).await.unwrap();
        }
        
        // Should still be in normal mode due to manual override
        assert_eq!(service.get_current_level().await, DegradationLevel::Normal);
    }
    
    #[tokio::test]
    async fn test_read_only_compatibility() {
        let service = GracefulDegradationService::new();
        
        // Test read-only compatible error
        let read_error = AppError::DatabaseError("connection timeout".to_string());
        assert!(service.should_allow_operation(&read_error, "read").await);
        
        // Test non-read-only compatible error
        let write_error = AppError::DatabaseError("serialization failure".to_string());
        assert!(!service.should_allow_operation(&write_error, "write").await);
    }
}
