use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::utils::enhanced_logging::{EnhancedLogger, LogEntry, LoggingConfig, RequestLoggingMiddleware};
use crate::utils::monitoring_enhanced::{EnhancedMonitoringSystem, MonitoringConfig};
use crate::config::ProductionConfig;

/// Integrated logging and monitoring system for DeFi Risk Monitor
pub struct LoggingMonitoringIntegration {
    logger: Arc<EnhancedLogger>,
    monitoring: Arc<EnhancedMonitoringSystem>,
    config: IntegrationConfig,
    correlation_tracker: Arc<RwLock<CorrelationTracker>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub enable_correlation_tracking: bool,
    pub enable_performance_logging: bool,
    pub enable_security_logging: bool,
    pub enable_business_metrics: bool,
    pub log_sampling_rate: f64,
    pub metrics_sampling_rate: f64,
    pub correlation_ttl_minutes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationTracker {
    pub active_correlations: HashMap<String, CorrelationContext>,
    pub correlation_metrics: HashMap<String, CorrelationMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationContext {
    pub correlation_id: String,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub request_path: Option<String>,
    pub start_time: DateTime<Utc>,
    pub operations: Vec<String>,
    pub errors: Vec<String>,
    pub performance_data: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelationMetrics {
    pub total_requests: u64,
    pub average_duration_ms: f64,
    pub error_count: u64,
    pub success_rate: f64,
    pub last_activity: DateTime<Utc>,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enable_correlation_tracking: true,
            enable_performance_logging: true,
            enable_security_logging: true,
            enable_business_metrics: true,
            log_sampling_rate: 1.0,
            metrics_sampling_rate: 1.0,
            correlation_ttl_minutes: 60,
        }
    }
}

impl Default for CorrelationTracker {
    fn default() -> Self {
        Self {
            active_correlations: HashMap::new(),
            correlation_metrics: HashMap::new(),
        }
    }
}

impl LoggingMonitoringIntegration {
    /// Create a new integrated logging and monitoring system
    pub fn new(
        logging_config: LoggingConfig,
        monitoring_config: MonitoringConfig,
        integration_config: IntegrationConfig,
    ) -> Self {
        let logger = Arc::new(EnhancedLogger::new(logging_config));
        let monitoring = Arc::new(EnhancedMonitoringSystem::new(monitoring_config));

        Self {
            logger,
            monitoring,
            config: integration_config,
            correlation_tracker: Arc::new(RwLock::new(CorrelationTracker::default())),
        }
    }

    /// Create from production configuration
    pub fn from_production_config(config: &ProductionConfig) -> Self {
        let logging_config = LoggingConfig {
            level: config.logging.level.clone(),
            format: match config.logging.format.as_str() {
                "json" => crate::utils::enhanced_logging::LogFormat::Json,
                "pretty" => crate::utils::enhanced_logging::LogFormat::Pretty,
                "compact" => crate::utils::enhanced_logging::LogFormat::Compact,
                _ => crate::utils::enhanced_logging::LogFormat::Json,
            },
            output: match config.logging.file_path.as_ref() {
                Some(_) => crate::utils::enhanced_logging::LogOutput::Both,
                None => crate::utils::enhanced_logging::LogOutput::Stdout,
            },
            file_path: config.logging.file_path.clone(),
            max_file_size_mb: 100,
            max_files: 10,
            enable_structured_logging: true,
            enable_request_logging: true,
            enable_performance_logging: true,
            enable_security_logging: true,
            correlation_id_header: "x-correlation-id".to_string(),
            sensitive_fields: vec![
                "password".to_string(),
                "jwt_secret".to_string(),
                "api_key".to_string(),
                "private_key".to_string(),
                "webhook_url".to_string(),
            ],
            log_sampling_rate: 1.0,
            buffer_size: 8192,
        };

        let monitoring_config = MonitoringConfig {
            enable_metrics: true,
            enable_health_checks: true,
            enable_performance_tracking: true,
            enable_alerting: true,
            metrics_retention_hours: 24,
            health_check_interval_seconds: 30,
            performance_sampling_rate: 1.0,
            alert_cooldown_minutes: 5,
            prometheus_endpoint: None,
            jaeger_endpoint: None,
        };

        let integration_config = IntegrationConfig::default();

        Self::new(logging_config, monitoring_config, integration_config)
    }

    /// Initialize the integrated system
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing integrated logging and monitoring system");

        // Initialize logger
        self.logger.init()?;

        // Initialize monitoring
        self.monitoring.init().await?;

        // Start correlation tracking cleanup
        if self.config.enable_correlation_tracking {
            self.start_correlation_cleanup().await;
        }

        info!("Integrated logging and monitoring system initialized successfully");
        Ok(())
    }

    /// Start correlation tracking cleanup task
    async fn start_correlation_cleanup(&self) {
        let correlation_tracker = self.correlation_tracker.clone();
        let ttl_minutes = self.config.correlation_ttl_minutes;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                
                let mut tracker = correlation_tracker.write().await;
                let cutoff_time = Utc::now() - chrono::Duration::minutes(ttl_minutes as i64);
                
                // Clean up old correlations
                tracker.active_correlations.retain(|_, context| context.start_time > cutoff_time);
                
                // Clean up old metrics
                tracker.correlation_metrics.retain(|_, metrics| metrics.last_activity > cutoff_time);
                
                debug!("Correlation tracking cleanup completed, TTL: {} minutes", ttl_minutes);
            }
        });
    }

    /// Start a new correlation context
    pub async fn start_correlation(
        &self,
        correlation_id: String,
        user_id: Option<String>,
        session_id: Option<String>,
        request_path: Option<String>,
    ) {
        if !self.config.enable_correlation_tracking {
            return;
        }

        let mut tracker = self.correlation_tracker.write().await;
        
        let context = CorrelationContext {
            correlation_id: correlation_id.clone(),
            user_id,
            session_id,
            request_path,
            start_time: Utc::now(),
            operations: Vec::new(),
            errors: Vec::new(),
            performance_data: HashMap::new(),
        };

        tracker.active_correlations.insert(correlation_id, context);
    }

    /// Add operation to correlation context
    pub async fn add_operation(&self, correlation_id: &str, operation: String) {
        if !self.config.enable_correlation_tracking {
            return;
        }

        let mut tracker = self.correlation_tracker.write().await;
        if let Some(context) = tracker.active_correlations.get_mut(correlation_id) {
            context.operations.push(operation);
        }
    }

    /// Add error to correlation context
    pub async fn add_error(&self, correlation_id: &str, error: String) {
        if !self.config.enable_correlation_tracking {
            return;
        }

        let mut tracker = self.correlation_tracker.write().await;
        if let Some(context) = tracker.active_correlations.get_mut(correlation_id) {
            context.errors.push(error);
        }
    }

    /// Add performance data to correlation context
    pub async fn add_performance_data(&self, correlation_id: &str, operation: String, duration_ms: u64) {
        if !self.config.enable_correlation_tracking {
            return;
        }

        let mut tracker = self.correlation_tracker.write().await;
        if let Some(context) = tracker.active_correlations.get_mut(correlation_id) {
            context.performance_data.insert(operation, duration_ms);
        }
    }

    /// End correlation and generate summary
    pub async fn end_correlation(&self, correlation_id: &str) {
        if !self.config.enable_correlation_tracking {
            return;
        }

        let mut tracker = self.correlation_tracker.write().await;
        if let Some(context) = tracker.active_correlations.remove(correlation_id) {
            let duration = Utc::now().signed_duration_since(context.start_time);
            let duration_ms = duration.num_milliseconds() as u64;

            // Update correlation metrics
            let metrics = tracker.correlation_metrics.entry(correlation_id.to_string()).or_insert(CorrelationMetrics {
                total_requests: 0,
                average_duration_ms: 0.0,
                error_count: 0,
                success_rate: 0.0,
                last_activity: Utc::now(),
            });

            metrics.total_requests += 1;
            metrics.error_count += context.errors.len() as u64;
            metrics.average_duration_ms = ((metrics.average_duration_ms * (metrics.total_requests - 1) as f64) + duration_ms as f64) / metrics.total_requests as f64;
            metrics.success_rate = ((metrics.total_requests - metrics.error_count) as f64) / (metrics.total_requests as f64);
            metrics.last_activity = Utc::now();

            // Log correlation summary
            let mut fields = HashMap::new();
            fields.insert("correlation_summary".to_string(), serde_json::Value::Bool(true));
            fields.insert("operations_count".to_string(), serde_json::Value::Number(context.operations.len().into()));
            fields.insert("errors_count".to_string(), serde_json::Value::Number(context.errors.len().into()));
            fields.insert("total_duration_ms".to_string(), serde_json::Value::Number(duration_ms.into()));

            let log_entry = LogEntry {
                timestamp: Utc::now(),
                level: if context.errors.is_empty() { "info".to_string() } else { "warn".to_string() },
                message: format!("Correlation {} completed: {} operations, {} errors, {}ms", 
                    correlation_id, context.operations.len(), context.errors.len(), duration_ms),
                target: "correlation".to_string(),
                correlation_id: Some(correlation_id.to_string()),
                user_id: context.user_id,
                request_id: None,
                session_id: context.session_id,
                fields,
                span_id: None,
                trace_id: None,
                duration_ms: Some(duration_ms),
                error_code: None,
                component: "correlation_tracker".to_string(),
                environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            };

            self.logger.log_structured(log_entry).await;
        }
    }

    /// Log a DeFi-specific business event
    pub async fn log_business_event(
        &self,
        event_type: &str,
        event_data: HashMap<String, serde_json::Value>,
        correlation_id: Option<String>,
        user_id: Option<String>,
    ) {
        if !self.config.enable_business_metrics {
            return;
        }

        let mut fields = event_data;
        fields.insert("business_event".to_string(), serde_json::Value::Bool(true));
        fields.insert("event_type".to_string(), serde_json::Value::String(event_type.to_string()));

        let log_entry = LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: format!("Business Event: {}", event_type),
            target: "business".to_string(),
            correlation_id,
            user_id,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: None,
            error_code: None,
            component: "business_events".to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        };

        self.logger.log_structured(log_entry).await;

        // Update business metrics
        let mut labels = HashMap::new();
        labels.insert("event_type".to_string(), event_type.to_string());
        self.monitoring.increment_counter("business_events_total", labels).await;
    }

    /// Log a performance event with monitoring integration
    pub async fn log_performance_event(
        &self,
        operation: &str,
        duration_ms: u64,
        success: bool,
        correlation_id: Option<String>,
        _additional_data: Option<HashMap<String, serde_json::Value>>,
    ) {
        if !self.config.enable_performance_logging {
            return;
        }

        // Log the performance event
        let performance_log = EnhancedLogger::create_performance_log(
            "performance_tracker",
            operation,
            duration_ms,
            correlation_id.clone(),
        );

        self.logger.log_structured(performance_log).await;

        // Update monitoring metrics
        let mut labels = HashMap::new();
        labels.insert("operation".to_string(), operation.to_string());
        labels.insert("success".to_string(), success.to_string());

        self.monitoring.increment_counter("operations_total", labels.clone()).await;
        self.monitoring.observe_histogram("operation_duration_seconds", duration_ms as f64 / 1000.0, labels).await;

        if !success {
            let mut error_labels = HashMap::new();
            error_labels.insert("operation".to_string(), operation.to_string());
            self.monitoring.increment_counter("operation_errors_total", error_labels).await;
        }

        // Add to correlation context if available
        if let Some(correlation_id) = &correlation_id {
            self.add_operation(correlation_id, operation.to_string()).await;
            self.add_performance_data(correlation_id, operation.to_string(), duration_ms).await;
            
            if !success {
                self.add_error(correlation_id, format!("Operation {} failed", operation)).await;
            }
        }
    }

    /// Log a security event with monitoring integration
    pub async fn log_security_event(
        &self,
        event_type: &str,
        severity: &str,
        user_id: Option<String>,
        ip_address: Option<String>,
        _details: HashMap<String, serde_json::Value>,
        correlation_id: Option<String>,
    ) {
        if !self.config.enable_security_logging {
            return;
        }

        // Create security log entry
        let security_log = EnhancedLogger::create_security_log(
            "security_monitor",
            event_type,
            user_id.clone(),
            severity,
            correlation_id.clone(),
        );

        self.logger.log_structured(security_log).await;

        // Update security metrics
        let mut labels = HashMap::new();
        labels.insert("event_type".to_string(), event_type.to_string());
        labels.insert("severity".to_string(), severity.to_string());

        self.monitoring.increment_counter("security_events_total", labels).await;

        // Add to correlation context if available
        if let Some(correlation_id) = &correlation_id {
            self.add_operation(correlation_id, format!("security_event_{}", event_type)).await;
            
            if severity == "high" || severity == "critical" {
                self.add_error(correlation_id, format!("Security event: {} ({})", event_type, severity)).await;
            }
        }

        // Log additional details for high-severity events
        if severity == "high" || severity == "critical" {
            warn!(
                "High-severity security event: {} from user {:?} at IP {:?}",
                event_type, user_id, ip_address
            );
        }
    }

    /// Get request logging middleware
    pub fn get_request_middleware(&self) -> RequestLoggingMiddleware {
        RequestLoggingMiddleware::new(self.logger.clone())
    }

    /// Get logger instance
    pub fn get_logger(&self) -> Arc<EnhancedLogger> {
        self.logger.clone()
    }

    /// Get monitoring system instance
    pub fn get_monitoring(&self) -> Arc<EnhancedMonitoringSystem> {
        self.monitoring.clone()
    }

    /// Get correlation tracking statistics
    pub async fn get_correlation_stats(&self) -> HashMap<String, CorrelationMetrics> {
        let tracker = self.correlation_tracker.read().await;
        tracker.correlation_metrics.clone()
    }

    /// Get active correlations count
    pub async fn get_active_correlations_count(&self) -> usize {
        let tracker = self.correlation_tracker.read().await;
        tracker.active_correlations.len()
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus_metrics(&self) -> String {
        let metrics = self.monitoring.get_metrics().await;
        let mut output = String::new();

        // Export counters
        for (_name, counter) in &metrics.counters {
            output.push_str(&format!("# HELP {} Counter metric\n", counter.name));
            output.push_str(&format!("# TYPE {} counter\n", counter.name));
            
            let labels = counter.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(",");
            
            if labels.is_empty() {
                output.push_str(&format!("{} {}\n", counter.name, counter.value));
            } else {
                output.push_str(&format!("{}{{{}}} {}\n", counter.name, labels, counter.value));
            }
        }

        // Export gauges
        for (_name, gauge) in &metrics.gauges {
            output.push_str(&format!("# HELP {} Gauge metric\n", gauge.name));
            output.push_str(&format!("# TYPE {} gauge\n", gauge.name));
            
            let labels = gauge.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(",");
            
            if labels.is_empty() {
                output.push_str(&format!("{} {}\n", gauge.name, gauge.value));
            } else {
                output.push_str(&format!("{}{{{}}} {}\n", gauge.name, labels, gauge.value));
            }
        }

        // Export histograms
        for (_name, histogram) in &metrics.histograms {
            output.push_str(&format!("# HELP {} Histogram metric\n", histogram.name));
            output.push_str(&format!("# TYPE {} histogram\n", histogram.name));
            
            let labels = histogram.labels.iter()
                .map(|(k, v)| format!("{}=\"{}\"", k, v))
                .collect::<Vec<_>>()
                .join(",");
            
            for bucket in &histogram.buckets {
                let bucket_labels = if labels.is_empty() {
                    format!("le=\"{}\"", bucket.upper_bound)
                } else {
                    format!("{},le=\"{}\"", labels, bucket.upper_bound)
                };
                
                output.push_str(&format!("{}_bucket{{{}}} {}\n", histogram.name, bucket_labels, bucket.count));
            }
            
            if labels.is_empty() {
                output.push_str(&format!("{}_count {}\n", histogram.name, histogram.count));
                output.push_str(&format!("{}_sum {}\n", histogram.name, histogram.sum));
            } else {
                output.push_str(&format!("{}_count{{{}}} {}\n", histogram.name, labels, histogram.count));
                output.push_str(&format!("{}_sum{{{}}} {}\n", histogram.name, labels, histogram.sum));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_integration_creation() {
        let logging_config = LoggingConfig::default();
        let monitoring_config = MonitoringConfig::default();
        let integration_config = IntegrationConfig::default();
        
        let integration = LoggingMonitoringIntegration::new(
            logging_config,
            monitoring_config,
            integration_config,
        );
        
        let stats = integration.get_correlation_stats().await;
        assert_eq!(stats.len(), 0);
    }

    #[tokio::test]
    async fn test_correlation_tracking() {
        let logging_config = LoggingConfig::default();
        let monitoring_config = MonitoringConfig::default();
        let integration_config = IntegrationConfig::default();
        
        let integration = LoggingMonitoringIntegration::new(
            logging_config,
            monitoring_config,
            integration_config,
        );
        
        let correlation_id = "test-correlation-123".to_string();
        
        integration.start_correlation(
            correlation_id.clone(),
            Some("user123".to_string()),
            None,
            Some("/api/test".to_string()),
        ).await;
        
        integration.add_operation(&correlation_id, "test_operation".to_string()).await;
        integration.add_performance_data(&correlation_id, "test_operation".to_string(), 150).await;
        
        let count = integration.get_active_correlations_count().await;
        assert_eq!(count, 1);
        
        integration.end_correlation(&correlation_id).await;
        
        let count = integration.get_active_correlations_count().await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_business_event_logging() {
        let logging_config = LoggingConfig::default();
        let monitoring_config = MonitoringConfig::default();
        let integration_config = IntegrationConfig::default();
        
        let integration = LoggingMonitoringIntegration::new(
            logging_config,
            monitoring_config,
            integration_config,
        );
        
        let mut event_data = HashMap::new();
        event_data.insert("position_id".to_string(), serde_json::Value::String("pos123".to_string()));
        event_data.insert("amount".to_string(), serde_json::Value::Number(1000.into()));
        
        integration.log_business_event(
            "position_created",
            event_data,
            Some("correlation123".to_string()),
            Some("user123".to_string()),
        ).await;
        
        // Test passes if no panic occurs
    }
}
