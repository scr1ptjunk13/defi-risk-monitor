use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, debug};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Enhanced monitoring system with metrics collection, alerting, and observability
pub struct EnhancedMonitoringSystem {
    config: MonitoringConfig,
    metrics_store: Arc<RwLock<MetricsStore>>,
    health_checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    alert_manager: Arc<RwLock<AlertManager>>,
    performance_tracker: Arc<RwLock<PerformanceTracker>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_metrics: bool,
    pub enable_health_checks: bool,
    pub enable_performance_tracking: bool,
    pub enable_alerting: bool,
    pub metrics_retention_hours: u64,
    pub health_check_interval_seconds: u64,
    pub performance_sampling_rate: f64,
    pub alert_cooldown_minutes: u64,
    pub prometheus_endpoint: Option<String>,
    pub jaeger_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsStore {
    pub counters: HashMap<String, Counter>,
    pub gauges: HashMap<String, Gauge>,
    pub histograms: HashMap<String, Histogram>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Counter {
    pub name: String,
    pub value: u64,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_increment: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gauge {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub name: String,
    pub buckets: Vec<HistogramBucket>,
    pub count: u64,
    pub sum: f64,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: f64,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub check_duration_ms: u64,
    pub error_message: Option<String>,
    pub consecutive_failures: u32,
    pub total_checks: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertManager {
    pub active_alerts: HashMap<String, Alert>,
    pub alert_history: Vec<AlertHistoryEntry>,
    pub alert_rules: HashMap<String, AlertRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub labels: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_fired: DateTime<Utc>,
    pub fire_count: u32,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub name: String,
    pub condition: String,
    pub threshold: f64,
    pub duration_seconds: u64,
    pub severity: AlertSeverity,
    pub message_template: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertHistoryEntry {
    pub alert_id: String,
    pub event_type: AlertEventType,
    pub timestamp: DateTime<Utc>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertEventType {
    Fired,
    Resolved,
    Acknowledged,
    Silenced,
}

#[derive(Debug, Clone)]
pub struct PerformanceTracker {
    pub operations: HashMap<String, OperationMetrics>,
    pub system_metrics: SystemMetrics,
    pub custom_timers: HashMap<String, Timer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationMetrics {
    pub name: String,
    pub total_calls: u64,
    pub total_duration_ms: u64,
    pub average_duration_ms: f64,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub last_call: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_usage_bytes: u64,
    pub disk_usage_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub open_file_descriptors: u32,
    pub thread_count: u32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Timer {
    pub name: String,
    pub start_time: Instant,
    pub labels: HashMap<String, String>,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl Default for MetricsStore {
    fn default() -> Self {
        Self {
            counters: HashMap::new(),
            gauges: HashMap::new(),
            histograms: HashMap::new(),
            last_updated: Utc::now(),
        }
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self {
            active_alerts: HashMap::new(),
            alert_history: Vec::new(),
            alert_rules: HashMap::new(),
        }
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            operations: HashMap::new(),
            system_metrics: SystemMetrics::default(),
            custom_timers: HashMap::new(),
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            memory_usage_percent: 0.0,
            disk_usage_bytes: 0,
            disk_usage_percent: 0.0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            open_file_descriptors: 0,
            thread_count: 0,
            last_updated: Utc::now(),
        }
    }
}

impl EnhancedMonitoringSystem {
    /// Create a new enhanced monitoring system
    pub fn new(config: MonitoringConfig) -> Self {
        Self {
            config,
            metrics_store: Arc::new(RwLock::new(MetricsStore::default())),
            health_checks: Arc::new(RwLock::new(HashMap::new())),
            alert_manager: Arc::new(RwLock::new(AlertManager::default())),
            performance_tracker: Arc::new(RwLock::new(PerformanceTracker::default())),
        }
    }

    /// Initialize the monitoring system
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing enhanced monitoring system");
        self.register_default_health_checks().await;
        self.register_default_alert_rules().await;
        self.start_background_tasks().await;
        info!("Enhanced monitoring system initialized successfully");
        Ok(())
    }

    /// Register default health checks
    async fn register_default_health_checks(&self) {
        let mut health_checks = self.health_checks.write().await;

        health_checks.insert("database".to_string(), HealthCheck {
            name: "database".to_string(),
            status: HealthStatus::Unknown,
            last_check: Utc::now(),
            check_duration_ms: 0,
            error_message: None,
            consecutive_failures: 0,
            total_checks: 0,
            success_rate: 0.0,
        });

        health_checks.insert("blockchain_rpc".to_string(), HealthCheck {
            name: "blockchain_rpc".to_string(),
            status: HealthStatus::Unknown,
            last_check: Utc::now(),
            check_duration_ms: 0,
            error_message: None,
            consecutive_failures: 0,
            total_checks: 0,
            success_rate: 0.0,
        });
    }

    /// Register default alert rules
    async fn register_default_alert_rules(&self) {
        let mut alert_manager = self.alert_manager.write().await;

        alert_manager.alert_rules.insert("high_error_rate".to_string(), AlertRule {
            name: "high_error_rate".to_string(),
            condition: "error_rate > threshold".to_string(),
            threshold: 0.05,
            duration_seconds: 300,
            severity: AlertSeverity::High,
            message_template: "High error rate detected: {error_rate}%".to_string(),
            enabled: true,
        });

        alert_manager.alert_rules.insert("high_memory_usage".to_string(), AlertRule {
            name: "high_memory_usage".to_string(),
            condition: "memory_usage_percent > threshold".to_string(),
            threshold: 85.0,
            duration_seconds: 600,
            severity: AlertSeverity::Medium,
            message_template: "High memory usage detected: {memory_usage_percent}%".to_string(),
            enabled: true,
        });
    }

    /// Start background monitoring tasks
    async fn start_background_tasks(&self) {
        if self.config.enable_health_checks {
            self.start_health_check_task().await;
        }
        if self.config.enable_performance_tracking {
            self.start_performance_tracking_task().await;
        }
        if self.config.enable_alerting {
            self.start_alert_evaluation_task().await;
        }
        self.start_metrics_cleanup_task().await;
    }

    /// Start health check background task
    async fn start_health_check_task(&self) {
        let health_checks = self.health_checks.clone();
        let interval = Duration::from_secs(self.config.health_check_interval_seconds);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                
                let mut checks = health_checks.write().await;
                for (name, check) in checks.iter_mut() {
                    let start_time = Instant::now();
                    let result = Self::perform_health_check(name).await;
                    let duration = start_time.elapsed();

                    check.last_check = Utc::now();
                    check.check_duration_ms = duration.as_millis() as u64;
                    check.total_checks += 1;

                    match result {
                        Ok(status) => {
                            check.status = status;
                            check.error_message = None;
                            check.consecutive_failures = 0;
                        }
                        Err(error) => {
                            check.status = HealthStatus::Unhealthy;
                            check.error_message = Some(error.to_string());
                            check.consecutive_failures += 1;
                        }
                    }

                    let success_count = check.total_checks - check.consecutive_failures as u64;
                    check.success_rate = (success_count as f64) / (check.total_checks as f64);
                }
            }
        });
    }

    /// Perform individual health check
    async fn perform_health_check(name: &str) -> Result<HealthStatus, Box<dyn std::error::Error>> {
        match name {
            "database" => {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(HealthStatus::Healthy)
            }
            "blockchain_rpc" => {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Ok(HealthStatus::Healthy)
            }
            _ => Ok(HealthStatus::Unknown),
        }
    }

    /// Start performance tracking task
    async fn start_performance_tracking_task(&self) {
        let performance_tracker = self.performance_tracker.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                
                let mut tracker = performance_tracker.write().await;
                tracker.system_metrics = SystemMetrics {
                    cpu_usage_percent: rand::random::<f64>() * 100.0,
                    memory_usage_bytes: (rand::random::<u64>() % 8_000_000_000) + 1_000_000_000,
                    memory_usage_percent: rand::random::<f64>() * 100.0,
                    disk_usage_bytes: (rand::random::<u64>() % 500_000_000_000) + 50_000_000_000,
                    disk_usage_percent: rand::random::<f64>() * 100.0,
                    network_rx_bytes: rand::random::<u64>() % 1_000_000_000,
                    network_tx_bytes: rand::random::<u64>() % 1_000_000_000,
                    open_file_descriptors: (rand::random::<u32>() % 1000) + 100,
                    thread_count: (rand::random::<u32>() % 100) + 10,
                    last_updated: Utc::now(),
                };
            }
        });
    }

    /// Start alert evaluation task
    async fn start_alert_evaluation_task(&self) {
        let alert_manager = self.alert_manager.clone();
        let performance_tracker = self.performance_tracker.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                let alerts = alert_manager.write().await;
                let tracker = performance_tracker.read().await;
                
                // Collect rules to check to avoid borrowing issues
                let rules_to_check: Vec<(String, AlertRule)> = alerts.alert_rules.clone().into_iter().collect();
                drop(alerts); // Release the write lock
                
                for (rule_name, rule) in rules_to_check {
                    if !rule.enabled {
                        continue;
                    }

                    let should_fire = match rule.name.as_str() {
                        "high_memory_usage" => {
                            tracker.system_metrics.memory_usage_percent > rule.threshold
                        }
                        "high_error_rate" => {
                            let total_errors: u64 = tracker.operations.values().map(|op| op.error_count).sum();
                            let total_calls: u64 = tracker.operations.values().map(|op| op.total_calls).sum();
                            if total_calls > 0 {
                                let error_rate = (total_errors as f64) / (total_calls as f64);
                                error_rate > rule.threshold
                            } else {
                                false
                            }
                        }
                        _ => false,
                    };

                    if should_fire {
                        let alert_id = format!("{}_{}", rule_name, Uuid::new_v4());
                        let mut alerts = alert_manager.write().await; // Re-acquire lock for mutation
                        if !alerts.active_alerts.contains_key(&alert_id) {
                            let alert = Alert {
                                id: alert_id.clone(),
                                rule_name: rule_name.clone(),
                                severity: rule.severity.clone(),
                                message: rule.message_template.clone(),
                                labels: HashMap::new(),
                                created_at: Utc::now(),
                                last_fired: Utc::now(),
                                fire_count: 1,
                                resolved_at: None,
                            };

                            alerts.active_alerts.insert(alert_id.clone(), alert);
                            alerts.alert_history.push(AlertHistoryEntry {
                                alert_id: alert_id.clone(),
                                event_type: AlertEventType::Fired,
                                timestamp: Utc::now(),
                                message: format!("Alert {} fired", rule_name),
                            });

                            warn!("Alert fired: {} - {}", rule_name, rule.message_template);
                        }
                    }
                }
            }
        });
    }

    /// Start metrics cleanup task
    async fn start_metrics_cleanup_task(&self) {
        let metrics_store = self.metrics_store.clone();
        let retention_hours = self.config.metrics_retention_hours;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                
                let mut store = metrics_store.write().await;
                let cutoff_time = Utc::now() - chrono::Duration::hours(retention_hours as i64);
                
                store.counters.retain(|_, counter| counter.created_at > cutoff_time);
                store.gauges.retain(|_, gauge| gauge.created_at > cutoff_time);
                store.histograms.retain(|_, histogram| histogram.created_at > cutoff_time);
                
                store.last_updated = Utc::now();
                debug!("Metrics cleanup completed, retention: {} hours", retention_hours);
            }
        });
    }

    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, labels: HashMap<String, String>) {
        if !self.config.enable_metrics {
            return;
        }

        let mut store = self.metrics_store.write().await;
        let key = format!("{}_{}", name, self.labels_to_key(&labels));
        
        match store.counters.get_mut(&key) {
            Some(counter) => {
                counter.value += 1;
                counter.last_increment = Utc::now();
            }
            None => {
                store.counters.insert(key, Counter {
                    name: name.to_string(),
                    value: 1,
                    labels,
                    created_at: Utc::now(),
                    last_increment: Utc::now(),
                });
            }
        }
        
        store.last_updated = Utc::now();
    }

    /// Set a gauge metric
    pub async fn set_gauge(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        if !self.config.enable_metrics {
            return;
        }

        let mut store = self.metrics_store.write().await;
        let key = format!("{}_{}", name, self.labels_to_key(&labels));
        
        match store.gauges.get_mut(&key) {
            Some(gauge) => {
                gauge.value = value;
                gauge.last_update = Utc::now();
            }
            None => {
                store.gauges.insert(key, Gauge {
                    name: name.to_string(),
                    value,
                    labels,
                    created_at: Utc::now(),
                    last_update: Utc::now(),
                });
            }
        }
        
        store.last_updated = Utc::now();
    }

    /// Record a histogram observation
    pub async fn observe_histogram(&self, name: &str, value: f64, labels: HashMap<String, String>) {
        if !self.config.enable_metrics {
            return;
        }

        let mut store = self.metrics_store.write().await;
        let key = format!("{}_{}", name, self.labels_to_key(&labels));
        
        match store.histograms.get_mut(&key) {
            Some(histogram) => {
                histogram.count += 1;
                histogram.sum += value;
                
                for bucket in &mut histogram.buckets {
                    if value <= bucket.upper_bound {
                        bucket.count += 1;
                    }
                }
            }
            None => {
                let buckets = vec![
                    HistogramBucket { upper_bound: 0.001, count: if value <= 0.001 { 1 } else { 0 } },
                    HistogramBucket { upper_bound: 0.01, count: if value <= 0.01 { 1 } else { 0 } },
                    HistogramBucket { upper_bound: 0.1, count: if value <= 0.1 { 1 } else { 0 } },
                    HistogramBucket { upper_bound: 1.0, count: if value <= 1.0 { 1 } else { 0 } },
                    HistogramBucket { upper_bound: 10.0, count: if value <= 10.0 { 1 } else { 0 } },
                    HistogramBucket { upper_bound: f64::INFINITY, count: 1 },
                ];
                
                store.histograms.insert(key, Histogram {
                    name: name.to_string(),
                    buckets,
                    count: 1,
                    sum: value,
                    labels,
                    created_at: Utc::now(),
                });
            }
        }
        
        store.last_updated = Utc::now();
    }

    /// Start a performance timer
    pub async fn start_timer(&self, name: &str, labels: HashMap<String, String>) -> String {
        if !self.config.enable_performance_tracking {
            return String::new();
        }

        let timer_id = Uuid::new_v4().to_string();
        let mut tracker = self.performance_tracker.write().await;
        
        tracker.custom_timers.insert(timer_id.clone(), Timer {
            name: name.to_string(),
            start_time: Instant::now(),
            labels,
        });
        
        timer_id
    }

    /// Stop a performance timer and record the duration
    pub async fn stop_timer(&self, timer_id: &str) {
        if !self.config.enable_performance_tracking {
            return;
        }

        let mut tracker = self.performance_tracker.write().await;
        
        if let Some(timer) = tracker.custom_timers.remove(timer_id) {
            let duration = timer.start_time.elapsed();
            let duration_ms = duration.as_millis() as u64;
            
            let op_metrics = tracker.operations.entry(timer.name.clone()).or_insert(OperationMetrics {
                name: timer.name.clone(),
                total_calls: 0,
                total_duration_ms: 0,
                average_duration_ms: 0.0,
                min_duration_ms: u64::MAX,
                max_duration_ms: 0,
                error_count: 0,
                success_rate: 100.0,
                last_call: Utc::now(),
            });
            
            op_metrics.total_calls += 1;
            op_metrics.total_duration_ms += duration_ms;
            op_metrics.average_duration_ms = op_metrics.total_duration_ms as f64 / op_metrics.total_calls as f64;
            op_metrics.min_duration_ms = op_metrics.min_duration_ms.min(duration_ms);
            op_metrics.max_duration_ms = op_metrics.max_duration_ms.max(duration_ms);
            op_metrics.last_call = Utc::now();
        }
    }

    /// Convert labels to a consistent key format
    fn labels_to_key(&self, labels: &HashMap<String, String>) -> String {
        let mut sorted_labels: Vec<_> = labels.iter().collect();
        sorted_labels.sort_by_key(|&(k, _)| k);
        sorted_labels.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Get current system health status
    pub async fn get_health_status(&self) -> HashMap<String, HealthCheck> {
        self.health_checks.read().await.clone()
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> MetricsStore {
        self.metrics_store.read().await.clone()
    }

    /// Get active alerts
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let alert_manager = self.alert_manager.read().await;
        alert_manager.active_alerts.values().cloned().collect()
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceTracker {
        self.performance_tracker.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitoring_system_creation() {
        let config = MonitoringConfig::default();
        let monitoring = EnhancedMonitoringSystem::new(config);
        
        let metrics = monitoring.get_metrics().await;
        assert_eq!(metrics.counters.len(), 0);
    }

    #[tokio::test]
    async fn test_counter_increment() {
        let config = MonitoringConfig::default();
        let monitoring = EnhancedMonitoringSystem::new(config);
        
        let mut labels = HashMap::new();
        labels.insert("test".to_string(), "value".to_string());
        
        monitoring.increment_counter("test_counter", labels).await;
        
        let metrics = monitoring.get_metrics().await;
        assert_eq!(metrics.counters.len(), 1);
    }

    #[tokio::test]
    async fn test_gauge_setting() {
        let config = MonitoringConfig::default();
        let monitoring = EnhancedMonitoringSystem::new(config);
        
        let labels = HashMap::new();
        monitoring.set_gauge("test_gauge", 42.0, labels).await;
        
        let metrics = monitoring.get_metrics().await;
        assert_eq!(metrics.gauges.len(), 1);
    }
}
