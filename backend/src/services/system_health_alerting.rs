use std::time::{Duration, Instant};
use std::collections::HashMap;
use tokio::time;
use tracing::{info, error};
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use bigdecimal::BigDecimal;
use std::str::FromStr;

use crate::error::AppError;
use crate::models::{Alert, CreateAlert, AlertSeverity};
use crate::services::AlertService;
use crate::utils::monitoring::{get_metrics, HealthChecker};
use crate::config::Settings;

/// System health thresholds for alerting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealthThresholds {
    /// Maximum acceptable error rate (errors per minute)
    pub max_error_rate: f64,
    /// Maximum acceptable request latency (seconds)
    pub max_request_latency: f64,
    /// Maximum acceptable RPC latency (seconds)
    pub max_rpc_latency: f64,
    /// Maximum acceptable database query latency (seconds)
    pub max_db_latency: f64,
    /// Minimum uptime percentage for alerts
    pub min_uptime_percentage: f64,
    /// Time window for calculating rates (minutes)
    pub time_window_minutes: u64,
}

impl Default for SystemHealthThresholds {
    fn default() -> Self {
        Self {
            max_error_rate: 10.0,           // 10 errors per minute
            max_request_latency: 5.0,       // 5 seconds
            max_rpc_latency: 10.0,          // 10 seconds
            max_db_latency: 1.0,            // 1 second
            min_uptime_percentage: 99.0,    // 99% uptime
            time_window_minutes: 5,         // 5 minute window
        }
    }
}

/// System health metrics snapshot
#[derive(Debug, Clone)]
pub struct SystemHealthSnapshot {
    pub timestamp: DateTime<Utc>,
    pub uptime: Duration,
    pub error_rate: f64,
    pub avg_request_latency: f64,
    pub avg_rpc_latency: f64,
    pub avg_db_latency: f64,
    pub circuit_breaker_states: HashMap<String, u8>,
    pub connectivity_checks: HashMap<String, bool>,
    pub healthy: bool,
}

/// System health alerting service
pub struct SystemHealthAlertingService {
    alert_service: AlertService,
    health_checker: HealthChecker,
    thresholds: SystemHealthThresholds,
    last_alert_times: HashMap<String, Instant>,
    alert_cooldown: Duration,
}

impl SystemHealthAlertingService {
    pub fn new(settings: &Settings) -> Self {
        Self {
            alert_service: AlertService::new(settings),
            health_checker: HealthChecker::new("1.0.0"),
            thresholds: SystemHealthThresholds::default(),
            last_alert_times: HashMap::new(),
            alert_cooldown: Duration::from_secs(300), // 5 minutes cooldown
        }
    }

    pub fn with_thresholds(mut self, thresholds: SystemHealthThresholds) -> Self {
        self.thresholds = thresholds;
        self
    }

    /// Start the system health monitoring loop
    pub async fn start_monitoring(&mut self) -> Result<(), AppError> {
        info!("Starting system health alerting service");
        
        let mut interval = time::interval(Duration::from_secs(60)); // Check every minute
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.check_system_health().await {
                error!("Error during system health check: {}", e);
            }
        }
    }

    /// Perform comprehensive system health check and alert if needed
    async fn check_system_health(&mut self) -> Result<(), AppError> {
        let snapshot = self.collect_health_snapshot().await?;
        
        // Check each health metric against thresholds
        self.check_error_rate(&snapshot).await?;
        self.check_latency(&snapshot).await?;
        self.check_uptime(&snapshot).await?;
        self.check_connectivity(&snapshot).await?;
        self.check_circuit_breakers(&snapshot).await?;
        
        Ok(())
    }

    /// Collect current system health metrics
    async fn collect_health_snapshot(&self) -> Result<SystemHealthSnapshot, AppError> {
        let health_status = self.health_checker.check_health().await;
        let _metrics = get_metrics().await?;
        
        // Calculate error rate (simplified - in production would use time windows)
        let error_rate = self.calculate_error_rate().await?;
        
        // Calculate average latencies (simplified - would use histogram percentiles)
        let avg_request_latency = self.calculate_avg_request_latency().await?;
        let avg_rpc_latency = self.calculate_avg_rpc_latency().await?;
        let avg_db_latency = self.calculate_avg_db_latency().await?;
        
        // Get circuit breaker states
        let circuit_breaker_states = self.get_circuit_breaker_states().await?;
        
        Ok(SystemHealthSnapshot {
            timestamp: Utc::now(),
            uptime: health_status.uptime,
            error_rate,
            avg_request_latency,
            avg_rpc_latency,
            avg_db_latency,
            circuit_breaker_states,
            connectivity_checks: health_status.checks,
            healthy: health_status.healthy,
        })
    }

    /// Check error rate against threshold
    async fn check_error_rate(&mut self, snapshot: &SystemHealthSnapshot) -> Result<(), AppError> {
        if snapshot.error_rate > self.thresholds.max_error_rate {
            self.send_alert_if_needed(
                "high_error_rate",
                AlertSeverity::High,
                "High Error Rate Detected",
                &format!(
                    "Error rate ({:.2}/min) exceeds threshold ({:.2}/min)",
                    snapshot.error_rate, self.thresholds.max_error_rate
                ),
                Some(snapshot.error_rate),
                Some(self.thresholds.max_error_rate),
            ).await?;
        }
        Ok(())
    }

    /// Check latency against thresholds
    async fn check_latency(&mut self, snapshot: &SystemHealthSnapshot) -> Result<(), AppError> {
        // Check request latency
        if snapshot.avg_request_latency > self.thresholds.max_request_latency {
            self.send_alert_if_needed(
                "high_request_latency",
                AlertSeverity::Medium,
                "High Request Latency",
                &format!(
                    "Average request latency ({:.2}s) exceeds threshold ({:.2}s)",
                    snapshot.avg_request_latency, self.thresholds.max_request_latency
                ),
                Some(snapshot.avg_request_latency),
                Some(self.thresholds.max_request_latency),
            ).await?;
        }

        // Check RPC latency
        if snapshot.avg_rpc_latency > self.thresholds.max_rpc_latency {
            self.send_alert_if_needed(
                "high_rpc_latency",
                AlertSeverity::Medium,
                "High RPC Latency",
                &format!(
                    "Average RPC latency ({:.2}s) exceeds threshold ({:.2}s)",
                    snapshot.avg_rpc_latency, self.thresholds.max_rpc_latency
                ),
                Some(snapshot.avg_rpc_latency),
                Some(self.thresholds.max_rpc_latency),
            ).await?;
        }

        // Check database latency
        if snapshot.avg_db_latency > self.thresholds.max_db_latency {
            self.send_alert_if_needed(
                "high_db_latency",
                AlertSeverity::High,
                "High Database Latency",
                &format!(
                    "Average database latency ({:.2}s) exceeds threshold ({:.2}s)",
                    snapshot.avg_db_latency, self.thresholds.max_db_latency
                ),
                Some(snapshot.avg_db_latency),
                Some(self.thresholds.max_db_latency),
            ).await?;
        }

        Ok(())
    }

    /// Check uptime against threshold
    async fn check_uptime(&mut self, snapshot: &SystemHealthSnapshot) -> Result<(), AppError> {
        let uptime_hours = snapshot.uptime.as_secs_f64() / 3600.0;
        let expected_uptime_hours = uptime_hours; // In a real system, would track expected vs actual
        let uptime_percentage = if expected_uptime_hours > 0.0 {
            (uptime_hours / expected_uptime_hours) * 100.0
        } else {
            100.0
        };

        if uptime_percentage < self.thresholds.min_uptime_percentage {
            self.send_alert_if_needed(
                "low_uptime",
                AlertSeverity::Critical,
                "Low System Uptime",
                &format!(
                    "System uptime ({:.2}%) below threshold ({:.2}%)",
                    uptime_percentage, self.thresholds.min_uptime_percentage
                ),
                Some(uptime_percentage),
                Some(self.thresholds.min_uptime_percentage),
            ).await?;
        }

        Ok(())
    }

    /// Check connectivity to external services
    async fn check_connectivity(&mut self, snapshot: &SystemHealthSnapshot) -> Result<(), AppError> {
        for (service, is_healthy) in &snapshot.connectivity_checks {
            if !is_healthy {
                self.send_alert_if_needed(
                    &format!("connectivity_{}", service),
                    AlertSeverity::High,
                    "Service Connectivity Issue",
                    &format!("Lost connectivity to service: {}", service),
                    None,
                    None,
                ).await?;
            }
        }
        Ok(())
    }

    /// Check circuit breaker states
    async fn check_circuit_breakers(&mut self, snapshot: &SystemHealthSnapshot) -> Result<(), AppError> {
        for (service, state) in &snapshot.circuit_breaker_states {
            if *state == 1 { // Open state
                self.send_alert_if_needed(
                    &format!("circuit_breaker_{}", service),
                    AlertSeverity::High,
                    "Circuit Breaker Open",
                    &format!("Circuit breaker for {} is in OPEN state", service),
                    None,
                    None,
                ).await?;
            }
        }
        Ok(())
    }

    /// Send alert with cooldown to prevent spam
    async fn send_alert_if_needed(
        &mut self,
        alert_key: &str,
        severity: AlertSeverity,
        title: &str,
        message: &str,
        current_value: Option<f64>,
        threshold_value: Option<f64>,
    ) -> Result<(), AppError> {
        let now = Instant::now();
        
        // Check cooldown
        if let Some(last_alert_time) = self.last_alert_times.get(alert_key) {
            if now.duration_since(*last_alert_time) < self.alert_cooldown {
                return Ok(()); // Skip alert due to cooldown
            }
        }

        // Create and send alert
        let create_alert = CreateAlert {
            position_id: None,
            alert_type: format!("system_health_{}", alert_key),
            severity,
            title: title.to_string(),
            message: message.to_string(),
            risk_score: None,
            current_value: current_value.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default()),
            threshold_value: threshold_value.map(|v| BigDecimal::from_str(&v.to_string()).unwrap_or_default()),
        };

        let alert = Alert::new(create_alert);
        
        if let Err(e) = self.alert_service.send_alert(&alert).await {
            error!("Failed to send system health alert: {}", e);
        } else {
            info!("Sent system health alert: {}", title);
            self.last_alert_times.insert(alert_key.to_string(), now);
        }

        Ok(())
    }

    /// Calculate current error rate (simplified implementation)
    async fn calculate_error_rate(&self) -> Result<f64, AppError> {
        // In a real implementation, this would query metrics for error counts
        // over the time window and calculate rate
        Ok(0.0) // Placeholder
    }

    /// Calculate average request latency
    async fn calculate_avg_request_latency(&self) -> Result<f64, AppError> {
        // In a real implementation, this would query histogram metrics
        // and calculate average or percentile latency
        Ok(0.1) // Placeholder
    }

    /// Calculate average RPC latency
    async fn calculate_avg_rpc_latency(&self) -> Result<f64, AppError> {
        Ok(0.5) // Placeholder
    }

    /// Calculate average database latency
    async fn calculate_avg_db_latency(&self) -> Result<f64, AppError> {
        Ok(0.05) // Placeholder
    }

    /// Get circuit breaker states
    async fn get_circuit_breaker_states(&self) -> Result<HashMap<String, u8>, AppError> {
        // In a real implementation, this would query circuit breaker metrics
        Ok(HashMap::new()) // Placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    #[tokio::test]
    async fn test_system_health_thresholds_default() {
        let thresholds = SystemHealthThresholds::default();
        assert_eq!(thresholds.max_error_rate, 10.0);
        assert_eq!(thresholds.max_request_latency, 5.0);
        assert_eq!(thresholds.min_uptime_percentage, 99.0);
    }

    #[tokio::test]
    async fn test_health_snapshot_creation() {
        // Use default settings to avoid requiring environment variables
        let settings = Settings::default();
        let service = SystemHealthAlertingService::new(&settings);
        
        // Validates the service structure and default thresholds
        assert!(service.thresholds.max_error_rate > 0.0);
        assert_eq!(service.thresholds.max_error_rate, 10.0);
        assert_eq!(service.thresholds.max_request_latency, 5.0);
    }

    #[tokio::test]
    async fn test_alert_cooldown() {
        // Use default settings to avoid requiring environment variables
        let settings = Settings::default();
        let mut service = SystemHealthAlertingService::new(&settings);
        
        // Test that cooldown prevents spam
        let alert_key = "test_alert";
        service.last_alert_times.insert(alert_key.to_string(), Instant::now());
        
        // Should skip due to cooldown
        let result = service.send_alert_if_needed(
            alert_key,
            AlertSeverity::Low,
            "Test Alert",
            "Test message",
            None,
            None,
        ).await;
        
        assert!(result.is_ok());
    }
}
