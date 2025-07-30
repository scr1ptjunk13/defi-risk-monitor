use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sqlx::{PgPool, Row};
use tracing::info;
use sqlx::types::ipnetwork::IpNetwork;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use crate::error::AppError;
use crate::models::{Position, Alert};

/// Audit event types for comprehensive logging
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_event_type", rename_all = "snake_case")]
pub enum AuditEventType {
    RiskCalculation,
    AlertTriggered,
    AlertResolved,
    PositionCreated,
    PositionUpdated,
    PositionClosed,
    PriceValidation,
    SystemStartup,
    SystemShutdown,
    ConfigurationChange,
    UserAction,
    ApiCall,
    DatabaseQuery,
    ExternalApiCall,
    CacheOperation,
    SecurityEvent,
}

/// Audit event severity levels
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "audit_severity", rename_all = "snake_case")]
pub enum AuditSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Comprehensive audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Uuid,
    pub event_type: AuditEventType,
    pub severity: AuditSeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub action: String,
    pub description: String,
    pub metadata: serde_json::Value,
    pub before_state: Option<serde_json::Value>,
    pub after_state: Option<serde_json::Value>,
    pub risk_impact: Option<BigDecimal>,
    pub financial_impact: Option<BigDecimal>,
    pub compliance_tags: Vec<String>,
}

impl AuditLogEntry {
    pub fn new(
        event_type: AuditEventType,
        severity: AuditSeverity,
        action: String,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type,
            severity,
            timestamp: chrono::Utc::now(),
            user_id: None,
            session_id: None,
            ip_address: None,
            user_agent: None,
            resource_type: None,
            resource_id: None,
            action,
            description,
            metadata: serde_json::Value::Null,
            before_state: None,
            after_state: None,
            risk_impact: None,
            financial_impact: None,
            compliance_tags: Vec::new(),
        }
    }

    /// Builder pattern for comprehensive audit entries
    pub fn with_user(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_request_info(mut self, ip_address: String, user_agent: String) -> Self {
        self.ip_address = Some(ip_address);
        self.user_agent = Some(user_agent);
        self
    }

    pub fn with_resource(mut self, resource_type: String, resource_id: String) -> Self {
        self.resource_type = Some(resource_type);
        self.resource_id = Some(resource_id);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_state_change(mut self, before: serde_json::Value, after: serde_json::Value) -> Self {
        self.before_state = Some(before);
        self.after_state = Some(after);
        self
    }

    pub fn with_financial_impact(mut self, risk_impact: BigDecimal, financial_impact: BigDecimal) -> Self {
        self.risk_impact = Some(risk_impact);
        self.financial_impact = Some(financial_impact);
        self
    }

    pub fn with_compliance_tags(mut self, tags: Vec<String>) -> Self {
        self.compliance_tags = tags;
        self
    }
}

/// Compliance report configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReportConfig {
    pub report_type: ComplianceReportType,
    pub start_date: chrono::DateTime<chrono::Utc>,
    pub end_date: chrono::DateTime<chrono::Utc>,
    pub include_user_actions: bool,
    pub include_system_events: bool,
    pub include_financial_data: bool,
    pub severity_filter: Option<AuditSeverity>,
    pub compliance_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceReportType {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annual,
    Custom,
    Incident,
    Regulatory,
}

/// Compliance report structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub id: Uuid,
    pub report_type: ComplianceReportType,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
    pub total_events: i64,
    pub critical_events: i64,
    pub error_events: i64,
    pub warning_events: i64,
    pub info_events: i64,
    pub total_financial_impact: BigDecimal,
    pub total_risk_impact: BigDecimal,
    pub unique_users: i64,
    pub unique_sessions: i64,
    pub compliance_violations: i64,
    pub summary: String,
    pub recommendations: Vec<String>,
    pub audit_entries: Vec<AuditLogEntry>,
}

/// Audit and compliance service
pub struct AuditService {
    db_pool: PgPool,
    retention_days: i32,
}

impl AuditService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            retention_days: 2555, // 7 years for financial compliance
        }
    }

    /// Log an audit event to the database
    pub async fn log_event(&self, entry: AuditLogEntry) -> Result<(), AppError> {
        let compliance_tags_json = serde_json::to_value(&entry.compliance_tags)
            .map_err(|e| AppError::InternalError(format!("Failed to serialize compliance tags: {}", e)))?;

        sqlx::query!(
            r#"
            INSERT INTO audit_logs (
                id, event_type, severity, timestamp, user_id, session_id, 
                ip_address, user_agent, resource_type, resource_id, action, 
                description, metadata, before_state, after_state, 
                risk_impact, financial_impact, compliance_tags
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18
            )
            "#,
            entry.id,
            entry.event_type.clone() as AuditEventType,
            entry.severity as AuditSeverity,
            entry.timestamp,
            entry.user_id,
            entry.session_id,
            entry.ip_address.as_deref().and_then(|ip| ip.parse::<IpNetwork>().ok()),
            entry.user_agent,
            entry.resource_type,
            entry.resource_id,
            entry.action,
            entry.description,
            entry.metadata,
            entry.before_state,
            entry.after_state,
            entry.risk_impact,
            entry.financial_impact,
            compliance_tags_json
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to insert audit log: {}", e)))?;

        info!("Audit event logged: {:?} - {}", entry.event_type.clone(), entry.action);
        Ok(())
    }

    /// Log risk calculation event
    pub async fn log_risk_calculation(
        &self,
        position_id: &str,
        user_id: &str,
        risk_score: &BigDecimal,
        financial_impact: &BigDecimal,
        metadata: serde_json::Value,
    ) -> Result<(), AppError> {
        let entry = AuditLogEntry::new(
            AuditEventType::RiskCalculation,
            AuditSeverity::Info,
            "calculate_risk".to_string(),
            format!("Risk calculation performed for position {}", position_id),
        )
        .with_user(user_id.to_string())
        .with_resource("position".to_string(), position_id.to_string())
        .with_metadata(metadata)
        .with_financial_impact(risk_score.clone(), financial_impact.clone())
        .with_compliance_tags(vec!["risk_management".to_string(), "financial_calculation".to_string()]);

        self.log_event(entry).await
    }

    /// Log alert event
    pub async fn log_alert(
        &self,
        alert: &Alert,
        action: &str,
        user_id: Option<&str>,
    ) -> Result<(), AppError> {
        let severity = match alert.severity.as_str() {
            "critical" => AuditSeverity::Critical,
            "high" => AuditSeverity::Error,
            "medium" => AuditSeverity::Warning,
            _ => AuditSeverity::Info,
        };

        let mut entry = AuditLogEntry::new(
            AuditEventType::AlertTriggered,
            severity,
            action.to_string(),
            format!("Alert {} for position {:?}", action, alert.position_id),
        )
        .with_resource("alert".to_string(), alert.id.to_string())
        .with_metadata(serde_json::json!({
            "alert_type": alert.alert_type,
            "severity": alert.severity,
            "threshold_value": alert.threshold_value,
            "current_value": alert.current_value,
            "message": alert.message
        }))
        .with_compliance_tags(vec!["alert_management".to_string(), "risk_monitoring".to_string()]);

        if let Some(uid) = user_id {
            entry = entry.with_user(uid.to_string());
        }

        self.log_event(entry).await
    }

    /// Log position lifecycle event
    pub async fn log_position_event(
        &self,
        position: &Position,
        action: &str,
        user_id: &str,
        before_state: Option<serde_json::Value>,
        after_state: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        let event_type = match action {
            "create" => AuditEventType::PositionCreated,
            "update" => AuditEventType::PositionUpdated,
            "close" => AuditEventType::PositionClosed,
            _ => AuditEventType::UserAction,
        };

        let mut entry = AuditLogEntry::new(
            event_type,
            AuditSeverity::Info,
            action.to_string(),
            format!("Position {} {}", position.id, action),
        )
        .with_user(user_id.to_string())
        .with_resource("position".to_string(), position.id.to_string())
        .with_metadata(serde_json::json!({
            "pool_address": position.pool_address,
            "token0_amount": position.token0_amount,
            "token1_amount": position.token1_amount,
            "tick_lower": position.tick_lower,
            "tick_upper": position.tick_upper
        }))
        .with_compliance_tags(vec!["position_management".to_string(), "trading_activity".to_string()]);

        if let (Some(before), Some(after)) = (before_state, after_state) {
            entry = entry.with_state_change(before, after);
        }

        self.log_event(entry).await
    }

    /// Log system event
    pub async fn log_system_event(
        &self,
        event_type: AuditEventType,
        action: &str,
        description: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), AppError> {
        let mut entry = AuditLogEntry::new(
            event_type,
            AuditSeverity::Info,
            action.to_string(),
            description.to_string(),
        )
        .with_compliance_tags(vec!["system_operation".to_string()]);

        if let Some(meta) = metadata {
            entry = entry.with_metadata(meta);
        }

        self.log_event(entry).await
    }

    /// Generate compliance report
    pub async fn generate_compliance_report(
        &self,
        config: ComplianceReportConfig,
    ) -> Result<ComplianceReport, AppError> {
        info!("Generating compliance report for period {} to {}", 
              config.start_date, config.end_date);

        // Query audit logs for the specified period
        let mut query = sqlx::QueryBuilder::new(
            "SELECT * FROM audit_logs WHERE timestamp >= "
        );
        query.push_bind(config.start_date);
        query.push(" AND timestamp <= ");
        query.push_bind(config.end_date);

        if let Some(severity) = &config.severity_filter {
            query.push(" AND severity = ");
            query.push_bind(severity.clone() as AuditSeverity);
        }

        if !config.compliance_tags.is_empty() {
            query.push(" AND compliance_tags ?| ");
            query.push_bind(&config.compliance_tags);
        }

        query.push(" ORDER BY timestamp DESC");

        let rows = query.build()
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to fetch audit logs: {}", e)))?;

        // Calculate statistics
        let total_events = rows.len() as i64;
        let mut critical_events = 0;
        let mut error_events = 0;
        let mut warning_events = 0;
        let mut info_events = 0;
        let mut unique_users = std::collections::HashSet::new();
        let mut unique_sessions = std::collections::HashSet::new();

        let mut audit_entries = Vec::new();

        for row in rows {
            // Parse severity and count
            let severity_str: String = row.get("severity");
            match severity_str.as_str() {
                "critical" => critical_events += 1,
                "error" => error_events += 1,
                "warning" => warning_events += 1,
                "info" => info_events += 1,
                _ => {}
            }

            // Track unique users and sessions
            if let Some(user_id) = row.get::<Option<String>, _>("user_id") {
                unique_users.insert(user_id);
            }
            if let Some(session_id) = row.get::<Option<String>, _>("session_id") {
                unique_sessions.insert(session_id);
            }

            // Convert row to AuditLogEntry (simplified for demo)
            let entry = AuditLogEntry {
                id: row.get("id"),
                event_type: AuditEventType::UserAction, // Simplified
                severity: AuditSeverity::Info, // Simplified
                timestamp: row.get("timestamp"),
                user_id: row.get("user_id"),
                session_id: row.get("session_id"),
                ip_address: row.get("ip_address"),
                user_agent: row.get("user_agent"),
                resource_type: row.get("resource_type"),
                resource_id: row.get("resource_id"),
                action: row.get("action"),
                description: row.get("description"),
                metadata: row.get("metadata"),
                before_state: row.get("before_state"),
                after_state: row.get("after_state"),
                risk_impact: row.get("risk_impact"),
                financial_impact: row.get("financial_impact"),
                compliance_tags: Vec::new(), // Simplified
            };
            audit_entries.push(entry);
        }

        // Generate summary and recommendations
        let summary = format!(
            "Compliance report for period {} to {}: {} total events, {} critical, {} errors",
            config.start_date.format("%Y-%m-%d"),
            config.end_date.format("%Y-%m-%d"),
            total_events,
            critical_events,
            error_events
        );

        let mut recommendations = Vec::new();
        if critical_events > 0 {
            recommendations.push("Review and address all critical events immediately".to_string());
        }
        if error_events > total_events / 10 {
            recommendations.push("High error rate detected - investigate system stability".to_string());
        }

        let report = ComplianceReport {
            id: Uuid::new_v4(),
            report_type: config.report_type,
            generated_at: chrono::Utc::now(),
            period_start: config.start_date,
            period_end: config.end_date,
            total_events,
            critical_events,
            error_events,
            warning_events,
            info_events,
            total_financial_impact: BigDecimal::from(0), // Would calculate from actual data
            total_risk_impact: BigDecimal::from(0), // Would calculate from actual data
            unique_users: unique_users.len() as i64,
            unique_sessions: unique_sessions.len() as i64,
            compliance_violations: critical_events, // Simplified
            summary,
            recommendations,
            audit_entries,
        };

        info!("Generated compliance report with {} events", total_events);
        Ok(report)
    }

    /// Clean up old audit logs based on retention policy
    pub async fn cleanup_old_logs(&self) -> Result<i64, AppError> {
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(self.retention_days as i64);

        let result = sqlx::query!(
            "DELETE FROM audit_logs WHERE timestamp < $1",
            cutoff_date
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to cleanup audit logs: {}", e)))?;

        let deleted_count = result.rows_affected() as i64;
        info!("Cleaned up {} old audit log entries", deleted_count);
        Ok(deleted_count)
    }

    /// Get audit statistics
    pub async fn get_audit_statistics(&self) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let mut stats = HashMap::new();

        // Total events in last 24 hours
        let last_24h = chrono::Utc::now() - chrono::Duration::hours(24);
        let recent_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM audit_logs WHERE timestamp >= $1"
        )
        .bind(last_24h)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get recent audit count: {}", e)))?;

        stats.insert("events_last_24h".to_string(), serde_json::Value::from(recent_count.0));

        // Events by severity
        let severity_counts = sqlx::query_as::<_, (String, i64)>(
            "SELECT severity::text, COUNT(*) as count FROM audit_logs WHERE created_at >= $1 GROUP BY severity"
        )
        .bind(last_24h)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get severity counts: {}", e)))?;

        let mut severity_map = HashMap::new();
        for row in severity_counts {
            severity_map.insert(row.0, row.1);
        }
        stats.insert("severity_breakdown".to_string(), serde_json::to_value(severity_map).unwrap());

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_log_entry_creation() {
        let entry = AuditLogEntry::new(
            AuditEventType::RiskCalculation,
            AuditSeverity::Info,
            "test_action".to_string(),
            "test description".to_string(),
        );

        assert_eq!(entry.action, "test_action");
        assert_eq!(entry.description, "test description");
        assert!(entry.id != Uuid::nil());
    }

    #[test]
    fn test_audit_log_entry_builder() {
        let entry = AuditLogEntry::new(
            AuditEventType::UserAction,
            AuditSeverity::Warning,
            "test".to_string(),
            "test".to_string(),
        )
        .with_user("user123".to_string())
        .with_resource("position".to_string(), "pos123".to_string())
        .with_compliance_tags(vec!["tag1".to_string(), "tag2".to_string()]);

        assert_eq!(entry.user_id, Some("user123".to_string()));
        assert_eq!(entry.resource_type, Some("position".to_string()));
        assert_eq!(entry.resource_id, Some("pos123".to_string()));
        assert_eq!(entry.compliance_tags.len(), 2);
    }

    #[test]
    fn test_compliance_report_config() {
        let config = ComplianceReportConfig {
            report_type: ComplianceReportType::Daily,
            start_date: chrono::Utc::now() - chrono::Duration::days(1),
            end_date: chrono::Utc::now(),
            include_user_actions: true,
            include_system_events: true,
            include_financial_data: true,
            severity_filter: Some(AuditSeverity::Critical),
            compliance_tags: vec!["risk_management".to_string()],
        };

        assert!(matches!(config.report_type, ComplianceReportType::Daily));
        assert!(config.include_user_actions);
        assert_eq!(config.compliance_tags.len(), 1);
    }
}
