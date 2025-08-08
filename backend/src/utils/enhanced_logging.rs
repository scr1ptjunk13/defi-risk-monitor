use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug, Level};
use tracing_subscriber::{
    fmt,
    EnvFilter,
    Registry,
    Layer,
};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Enhanced logging system with structured logging, correlation IDs, and metrics
pub struct EnhancedLogger {
    config: LoggingConfig,
    metrics: Arc<RwLock<LoggingMetrics>>,
    sensitive_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
    pub file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub enable_structured_logging: bool,
    pub enable_request_logging: bool,
    pub enable_performance_logging: bool,
    pub enable_security_logging: bool,
    pub correlation_id_header: String,
    pub sensitive_fields: Vec<String>,
    pub log_sampling_rate: f64,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Pretty,
    Compact,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    File,
    Both,
    Syslog,
    Network(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub target: String,
    pub correlation_id: Option<String>,
    pub user_id: Option<String>,
    pub request_id: Option<String>,
    pub session_id: Option<String>,
    pub fields: HashMap<String, serde_json::Value>,
    pub span_id: Option<String>,
    pub trace_id: Option<String>,
    pub duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub component: String,
    pub environment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMetrics {
    pub total_logs: u64,
    pub logs_by_level: HashMap<String, u64>,
    pub logs_by_component: HashMap<String, u64>,
    pub error_rate: f64,
    pub average_log_size: f64,
    pub last_error: Option<DateTime<Utc>>,
    pub performance_logs: u64,
    pub security_logs: u64,
    pub dropped_logs: u64,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            file_path: None,
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
        }
    }
}

impl Default for LoggingMetrics {
    fn default() -> Self {
        Self {
            total_logs: 0,
            logs_by_level: HashMap::new(),
            logs_by_component: HashMap::new(),
            error_rate: 0.0,
            average_log_size: 0.0,
            last_error: None,
            performance_logs: 0,
            security_logs: 0,
            dropped_logs: 0,
        }
    }
}

impl EnhancedLogger {
    /// Create a new enhanced logger with configuration
    pub fn new(config: LoggingConfig) -> Self {
        let sensitive_fields = config.sensitive_fields.clone();
        
        Self {
            config,
            metrics: Arc::new(RwLock::new(LoggingMetrics::default())),
            sensitive_fields,
        }
    }

    /// Initialize the enhanced logging system
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let level = self.config.level.parse::<Level>()
            .unwrap_or(Level::INFO);

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("defi_risk_monitor={}", level).into());

        // Simplified logging initialization to avoid trait object complexity
        // TODO: Restore advanced logging features after fixing trait object issues
        match self.config.format {
            LogFormat::Json => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .json()
                    .init();
            }
            LogFormat::Pretty => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .pretty()
                    .init();
            }
            LogFormat::Compact => {
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .compact()
                    .init();
            }
            LogFormat::Custom(_) => {
                // Fallback to JSON for custom format
                tracing_subscriber::fmt()
                    .with_env_filter(env_filter)
                    .json()
                    .init();
            }
        }

        info!("Enhanced logging system initialized with level: {}", self.config.level);
        Ok(())
    }

    /// Create JSON formatting layer
    #[allow(dead_code)]
    fn create_json_layer(&self) -> Result<Box<dyn Layer<Registry> + Send + Sync>, Box<dyn std::error::Error>> {
        match self.config.output {
            LogOutput::Stdout => {
                Ok(Box::new(fmt::layer().json().with_writer(std::io::stdout)))
            }
            LogOutput::File => {
                if let Some(file_path) = &self.config.file_path {
                    let file = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(file_path)?;
                    Ok(Box::new(fmt::layer().json().with_writer(file)))
                } else {
                    Ok(Box::new(fmt::layer().json().with_writer(std::io::stdout)))
                }
            }
            LogOutput::Both => {
                // For both, we'll use stdout and let the application handle file writing
                Ok(Box::new(fmt::layer().json().with_writer(std::io::stdout)))
            }
            _ => Ok(Box::new(fmt::layer().json().with_writer(std::io::stdout))),
        }
    }

    /// Create pretty formatting layer
    #[allow(dead_code)]
    fn create_pretty_layer(&self) -> Result<Box<dyn Layer<Registry> + Send + Sync>, Box<dyn std::error::Error>> {
        Ok(Box::new(fmt::layer().pretty().with_writer(std::io::stdout)))
    }

    /// Create compact formatting layer
    #[allow(dead_code)]
    fn create_compact_layer(&self) -> Result<Box<dyn Layer<Registry> + Send + Sync>, Box<dyn std::error::Error>> {
        Ok(Box::new(fmt::layer().compact().with_writer(std::io::stdout)))
    }

    /// Create custom formatting layer
    #[allow(dead_code)]
    fn create_custom_layer(&self) -> Result<Box<dyn Layer<Registry> + Send + Sync>, Box<dyn std::error::Error>> {
        // For now, fallback to JSON
        self.create_json_layer()
    }

    /// Create metrics collection layer
    #[allow(dead_code)]
    fn create_metrics_layer(&self) -> MetricsLayer {
        MetricsLayer::new(self.metrics.clone())
    }

    /// Log a structured entry
    pub async fn log_structured(&self, entry: LogEntry) {
        let sanitized_entry = self.sanitize_log_entry(entry).await;
        
        // Update metrics
        self.update_metrics(&sanitized_entry).await;
        
        // Log based on level
        match sanitized_entry.level.as_str() {
            "error" => {
                error!(
                    correlation_id = sanitized_entry.correlation_id,
                    user_id = sanitized_entry.user_id,
                    component = sanitized_entry.component,
                    error_code = sanitized_entry.error_code,
                    target = sanitized_entry.target,
                    "{}", sanitized_entry.message
                );
            }
            "warn" => {
                warn!(
                    correlation_id = sanitized_entry.correlation_id,
                    user_id = sanitized_entry.user_id,
                    component = sanitized_entry.component,
                    target = sanitized_entry.target,
                    "{}", sanitized_entry.message
                );
            }
            "info" => {
                info!(
                    correlation_id = sanitized_entry.correlation_id,
                    user_id = sanitized_entry.user_id,
                    component = sanitized_entry.component,
                    target = sanitized_entry.target,
                    "{}", sanitized_entry.message
                );
            }
            _ => {
                debug!(
                    correlation_id = sanitized_entry.correlation_id,
                    user_id = sanitized_entry.user_id,
                    component = sanitized_entry.component,
                    target = sanitized_entry.target,
                    "{}", sanitized_entry.message
                );
            }
        }
    }

    /// Sanitize log entry by removing sensitive fields
    async fn sanitize_log_entry(&self, mut entry: LogEntry) -> LogEntry {
        for sensitive_field in &self.sensitive_fields {
            if entry.fields.contains_key(sensitive_field) {
                entry.fields.insert(sensitive_field.clone(), serde_json::Value::String("[REDACTED]".to_string()));
            }
        }
        
        // Sanitize message content
        let mut message = entry.message.clone();
        for sensitive_field in &self.sensitive_fields {
            if message.contains(sensitive_field) {
                message = message.replace(&format!("{}=", sensitive_field), &format!("{}=[REDACTED]", sensitive_field));
            }
        }
        entry.message = message;
        
        entry
    }

    /// Update logging metrics
    async fn update_metrics(&self, entry: &LogEntry) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_logs += 1;
        
        // Update level metrics
        *metrics.logs_by_level.entry(entry.level.clone()).or_insert(0) += 1;
        
        // Update component metrics
        *metrics.logs_by_component.entry(entry.component.clone()).or_insert(0) += 1;
        
        // Update error rate
        let error_count = metrics.logs_by_level.get("error").unwrap_or(&0);
        metrics.error_rate = (*error_count as f64) / (metrics.total_logs as f64);
        
        // Update last error timestamp
        if entry.level == "error" {
            metrics.last_error = Some(entry.timestamp);
        }
        
        // Update specialized log counts
        if entry.fields.contains_key("performance") {
            metrics.performance_logs += 1;
        }
        
        if entry.fields.contains_key("security") {
            metrics.security_logs += 1;
        }
    }

    /// Get current logging metrics
    pub async fn get_metrics(&self) -> LoggingMetrics {
        self.metrics.read().await.clone()
    }

    /// Create a correlation ID for request tracking
    pub fn create_correlation_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Create a performance log entry
    pub fn create_performance_log(
        component: &str,
        operation: &str,
        duration_ms: u64,
        correlation_id: Option<String>,
    ) -> LogEntry {
        let mut fields = HashMap::new();
        fields.insert("performance".to_string(), serde_json::Value::Bool(true));
        fields.insert("operation".to_string(), serde_json::Value::String(operation.to_string()));
        
        LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: format!("Performance: {} completed in {}ms", operation, duration_ms),
            target: "performance".to_string(),
            correlation_id,
            user_id: None,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: Some(duration_ms),
            error_code: None,
            component: component.to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Create a security log entry
    pub fn create_security_log(
        component: &str,
        event: &str,
        user_id: Option<String>,
        severity: &str,
        correlation_id: Option<String>,
    ) -> LogEntry {
        let mut fields = HashMap::new();
        fields.insert("security".to_string(), serde_json::Value::Bool(true));
        fields.insert("event".to_string(), serde_json::Value::String(event.to_string()));
        fields.insert("severity".to_string(), serde_json::Value::String(severity.to_string()));
        
        LogEntry {
            timestamp: Utc::now(),
            level: match severity {
                "critical" | "high" => "error".to_string(),
                "medium" => "warn".to_string(),
                _ => "info".to_string(),
            },
            message: format!("Security Event: {}", event),
            target: "security".to_string(),
            correlation_id,
            user_id,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: None,
            error_code: None,
            component: component.to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Create an error log entry
    pub fn create_error_log(
        component: &str,
        error: &str,
        error_code: Option<String>,
        correlation_id: Option<String>,
        context: Option<HashMap<String, serde_json::Value>>,
    ) -> LogEntry {
        let mut fields = context.unwrap_or_default();
        fields.insert("error".to_string(), serde_json::Value::Bool(true));
        
        LogEntry {
            timestamp: Utc::now(),
            level: "error".to_string(),
            message: error.to_string(),
            target: "error".to_string(),
            correlation_id,
            user_id: None,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: None,
            error_code,
            component: component.to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        }
    }

    /// Log rotation and cleanup
    pub async fn rotate_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = &self.config.file_path {
            let metadata = std::fs::metadata(file_path)?;
            let file_size_mb = metadata.len() / (1024 * 1024);
            
            if file_size_mb > self.config.max_file_size_mb {
                // Rotate log file
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
                let rotated_path = format!("{}.{}", file_path, timestamp);
                std::fs::rename(file_path, rotated_path)?;
                
                // Clean up old log files
                self.cleanup_old_logs().await?;
                
                info!("Log file rotated: {}", file_path);
            }
        }
        
        Ok(())
    }

    /// Clean up old log files
    async fn cleanup_old_logs(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(file_path) = &self.config.file_path {
            let parent_dir = std::path::Path::new(file_path).parent().unwrap_or(std::path::Path::new("."));
            let file_stem = std::path::Path::new(file_path).file_stem().unwrap_or(std::ffi::OsStr::new("app"));
            
            let mut log_files = Vec::new();
            for entry in std::fs::read_dir(parent_dir)? {
                let entry = entry?;
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    if file_name.to_string_lossy().starts_with(&file_stem.to_string_lossy().to_string()) {
                        if let Ok(metadata) = entry.metadata() {
                            log_files.push((path, metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH)));
                        }
                    }
                }
            }
            
            // Sort by modification time (newest first)
            log_files.sort_by(|a, b| b.1.cmp(&a.1));
            
            // Remove excess files
            if log_files.len() > self.config.max_files as usize {
                for (path, _) in log_files.iter().skip(self.config.max_files as usize) {
                    std::fs::remove_file(path)?;
                    info!("Removed old log file: {:?}", path);
                }
            }
        }
        
        Ok(())
    }
}

/// Custom metrics collection layer for tracing
pub struct MetricsLayer {
    metrics: Arc<RwLock<LoggingMetrics>>,
}

impl MetricsLayer {
    pub fn new(metrics: Arc<RwLock<LoggingMetrics>>) -> Self {
        Self { metrics }
    }
}

impl<S> tracing_subscriber::Layer<S> for MetricsLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let level = event.metadata().level().to_string();
        let target = event.metadata().target().to_string();
        
        // Update metrics asynchronously
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            let mut metrics_guard = metrics.write().await;
            metrics_guard.total_logs += 1;
            *metrics_guard.logs_by_level.entry(level).or_insert(0) += 1;
            *metrics_guard.logs_by_component.entry(target).or_insert(0) += 1;
        });
    }
}

/// Logging middleware for HTTP requests
pub struct RequestLoggingMiddleware {
    logger: Arc<EnhancedLogger>,
}

impl RequestLoggingMiddleware {
    pub fn new(logger: Arc<EnhancedLogger>) -> Self {
        Self { logger }
    }

    /// Create request log entry
    pub async fn log_request(
        &self,
        method: &str,
        path: &str,
        status: u16,
        duration_ms: u64,
        correlation_id: Option<String>,
        user_id: Option<String>,
    ) {
        let mut fields = HashMap::new();
        fields.insert("http_method".to_string(), serde_json::Value::String(method.to_string()));
        fields.insert("http_path".to_string(), serde_json::Value::String(path.to_string()));
        fields.insert("http_status".to_string(), serde_json::Value::Number(status.into()));
        fields.insert("request".to_string(), serde_json::Value::Bool(true));
        
        let log_entry = LogEntry {
            timestamp: Utc::now(),
            level: if status >= 400 { "warn".to_string() } else { "info".to_string() },
            message: format!("{} {} - {} ({}ms)", method, path, status, duration_ms),
            target: "http".to_string(),
            correlation_id,
            user_id,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: Some(duration_ms),
            error_code: if status >= 400 { Some(status.to_string()) } else { None },
            component: "http_server".to_string(),
            environment: std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
        };
        
        self.logger.log_structured(log_entry).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_enhanced_logger_creation() {
        let config = LoggingConfig::default();
        let logger = EnhancedLogger::new(config);
        
        let metrics = logger.get_metrics().await;
        assert_eq!(metrics.total_logs, 0);
    }

    #[tokio::test]
    async fn test_log_sanitization() {
        let config = LoggingConfig::default();
        let logger = EnhancedLogger::new(config);
        
        let mut fields = HashMap::new();
        fields.insert("password".to_string(), serde_json::Value::String("secret123".to_string()));
        
        let entry = LogEntry {
            timestamp: Utc::now(),
            level: "info".to_string(),
            message: "User login with password=secret123".to_string(),
            target: "auth".to_string(),
            correlation_id: None,
            user_id: None,
            request_id: None,
            session_id: None,
            fields,
            span_id: None,
            trace_id: None,
            duration_ms: None,
            error_code: None,
            component: "auth".to_string(),
            environment: "test".to_string(),
        };
        
        let sanitized = logger.sanitize_log_entry(entry).await;
        assert_eq!(sanitized.fields.get("password").unwrap(), &serde_json::Value::String("[REDACTED]".to_string()));
        assert!(sanitized.message.contains("[REDACTED]"));
    }

    #[test]
    fn test_correlation_id_generation() {
        let correlation_id = EnhancedLogger::create_correlation_id();
        assert!(!correlation_id.is_empty());
        assert!(correlation_id.contains('-'));
    }

    #[test]
    fn test_performance_log_creation() {
        let log_entry = EnhancedLogger::create_performance_log(
            "database",
            "query_execution",
            150,
            Some("test-correlation-id".to_string()),
        );
        
        assert_eq!(log_entry.component, "database");
        assert_eq!(log_entry.duration_ms, Some(150));
        assert!(log_entry.fields.contains_key("performance"));
    }

    #[test]
    fn test_security_log_creation() {
        let log_entry = EnhancedLogger::create_security_log(
            "auth",
            "failed_login_attempt",
            Some("user123".to_string()),
            "medium",
            Some("test-correlation-id".to_string()),
        );
        
        assert_eq!(log_entry.component, "auth");
        assert_eq!(log_entry.level, "warn");
        assert!(log_entry.fields.contains_key("security"));
    }
}
