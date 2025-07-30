use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use crate::error::AppError;

/// Enhanced audit trail for security monitoring and compliance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAuditEvent {
    pub id: Uuid,
    pub event_type: SecurityEventType,
    pub severity: SecuritySeverity,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub user_id: Option<Uuid>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub action: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub risk_score: f64,
    pub mitigation_applied: bool,
    pub compliance_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityEventType {
    AuthenticationFailure,
    AuthorizationViolation,
    InputValidationFailure,
    SqlInjectionAttempt,
    XssAttempt,
    RateLimitExceeded,
    SuspiciousActivity,
    DataAccess,
    ConfigurationChange,
    SecurityPolicyViolation,
    AnomalousTransaction,
    PrivilegeEscalation,
    DataExfiltration,
    SystemIntrusion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecuritySeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug)]
pub struct SecurityAuditService {
    db_pool: PgPool,
    risk_threshold: f64,
    #[allow(dead_code)]
    auto_mitigation_enabled: bool,
}

impl SecurityAuditService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            risk_threshold: 7.0, // High risk threshold
            auto_mitigation_enabled: true,
        }
    }

    /// Log a security event
    pub async fn log_security_event(&self, event: SecurityAuditEvent) -> Result<(), AppError> {
        // Store in database
        sqlx::query(
            r#"
            INSERT INTO security_audit_events (
                id, event_type, severity, timestamp, user_id, session_id,
                ip_address, user_agent, resource_type, resource_id, action,
                description, metadata, risk_score, mitigation_applied, compliance_tags
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#
        )
        .bind(event.id)
        .bind(serde_json::to_string(&event.event_type).unwrap())
        .bind(serde_json::to_string(&event.severity).unwrap())
        .bind(event.timestamp)
        .bind(event.user_id)
        .bind(event.session_id.clone())
        .bind(event.ip_address.clone())
        .bind(event.user_agent.clone())
        .bind(event.resource_type.clone())
        .bind(event.resource_id.clone())
        .bind(event.action.clone())
        .bind(event.description.clone())
        .bind(event.metadata.clone())
        .bind(event.risk_score)
        .bind(event.mitigation_applied)
        .bind(serde_json::to_string(&event.compliance_tags).unwrap())
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to log security event: {}", e)))?;

        // Check if immediate action is required
        if event.risk_score >= self.risk_threshold {
            self.handle_high_risk_event(&event).await?;
        }

        Ok(())
    }

    /// Handle high-risk security events
    async fn handle_high_risk_event(&self, event: &SecurityAuditEvent) -> Result<(), AppError> {
        match event.event_type {
            SecurityEventType::SqlInjectionAttempt | 
            SecurityEventType::XssAttempt => {
                // Block IP address temporarily
                if let Some(ip) = &event.ip_address {
                    self.add_to_blocklist(ip, "Malicious activity detected", 3600).await?;
                }
            }
            SecurityEventType::RateLimitExceeded => {
                // Extend rate limiting
                if let Some(user_id) = event.user_id {
                    self.extend_rate_limit(user_id, 1800).await?;
                }
            }
            SecurityEventType::AuthenticationFailure => {
                // Track failed attempts
                if let Some(ip) = &event.ip_address {
                    self.track_failed_authentication(ip).await?;
                }
            }
            _ => {
                // Log for manual review
                tracing::warn!("High-risk security event requires manual review: {:?}", event);
            }
        }

        Ok(())
    }

    /// Add IP to temporary blocklist
    async fn add_to_blocklist(&self, ip: &str, reason: &str, duration_seconds: i32) -> Result<(), AppError> {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(duration_seconds as i64);
        
        sqlx::query(
            "INSERT INTO ip_blocklist (ip_address, reason, expires_at) VALUES ($1, $2, $3)
             ON CONFLICT (ip_address) DO UPDATE SET expires_at = EXCLUDED.expires_at"
        )
        .bind(ip)
        .bind(reason)
        .bind(expires_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to add IP to blocklist: {}", e)))?;

        tracing::warn!("IP {} added to blocklist for {}: {}", ip, duration_seconds, reason);
        Ok(())
    }

    /// Extend rate limiting for user
    async fn extend_rate_limit(&self, user_id: Uuid, duration_seconds: i32) -> Result<(), AppError> {
        let expires_at = chrono::Utc::now() + chrono::Duration::seconds(duration_seconds as i64);
        
        sqlx::query(
            "INSERT INTO rate_limit_overrides (user_id, limit_multiplier, expires_at) VALUES ($1, $2, $3)
             ON CONFLICT (user_id) DO UPDATE SET expires_at = EXCLUDED.expires_at"
        )
        .bind(user_id)
        .bind(0.1) // Reduce rate limit to 10% of normal
        .bind(expires_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to extend rate limit: {}", e)))?;

        Ok(())
    }

    /// Track failed authentication attempts
    async fn track_failed_authentication(&self, ip: &str) -> Result<(), AppError> {
        // Count recent failed attempts
        let recent_failures: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_audit_events 
             WHERE ip_address = $1 AND event_type = $2 AND timestamp > $3"
        )
        .bind(ip)
        .bind(serde_json::to_string(&SecurityEventType::AuthenticationFailure).unwrap())
        .bind(chrono::Utc::now() - chrono::Duration::minutes(15))
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to count auth failures: {}", e)))?;

        // Block IP after 5 failed attempts in 15 minutes
        if recent_failures.0 >= 5 {
            self.add_to_blocklist(ip, "Multiple authentication failures", 1800).await?;
        }

        Ok(())
    }

    /// Get security statistics for monitoring
    pub async fn get_security_statistics(&self) -> Result<SecurityStatistics, AppError> {
        let last_24h = chrono::Utc::now() - chrono::Duration::hours(24);

        // Count events by type
        let event_counts = sqlx::query_as::<_, (String, i64)>(
            "SELECT event_type, COUNT(*) FROM security_audit_events 
             WHERE timestamp >= $1 GROUP BY event_type"
        )
        .bind(last_24h)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get event counts: {}", e)))?;

        // Count by severity
        let severity_counts = sqlx::query_as::<_, (String, i64)>(
            "SELECT severity, COUNT(*) FROM security_audit_events 
             WHERE timestamp >= $1 GROUP BY severity"
        )
        .bind(last_24h)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get severity counts: {}", e)))?;

        // Get high-risk events
        let high_risk_events: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM security_audit_events 
             WHERE timestamp >= $1 AND risk_score >= $2"
        )
        .bind(last_24h)
        .bind(self.risk_threshold)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to count high-risk events: {}", e)))?;

        // Get blocked IPs
        let blocked_ips: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM ip_blocklist WHERE expires_at > $1"
        )
        .bind(chrono::Utc::now())
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to count blocked IPs: {}", e)))?;

        Ok(SecurityStatistics {
            total_events_24h: event_counts.iter().map(|(_, count)| count).sum(),
            event_type_counts: event_counts.into_iter().collect(),
            severity_counts: severity_counts.into_iter().collect(),
            high_risk_events_24h: high_risk_events.0,
            currently_blocked_ips: blocked_ips.0,
            analysis_time: chrono::Utc::now(),
        })
    }

    /// Generate compliance report
    pub async fn generate_compliance_report(&self, start_date: chrono::DateTime<chrono::Utc>, end_date: chrono::DateTime<chrono::Utc>) -> Result<ComplianceReport, AppError> {
        let _events: Vec<SecurityAuditEvent> = Vec::new(); // Placeholder for now

        let mut compliance_violations = 0;
        let mut data_access_events = 0;
        let mut authentication_failures = 0;
        let mut security_incidents = 0;

        for event in &_events {
            match &event.event_type {
                SecurityEventType::SecurityPolicyViolation => compliance_violations += 1,
                SecurityEventType::DataAccess => data_access_events += 1,
                SecurityEventType::AuthenticationFailure => authentication_failures += 1,
                SecurityEventType::SqlInjectionAttempt | 
                SecurityEventType::XssAttempt |
                SecurityEventType::SystemIntrusion => security_incidents += 1,
                _ => {}
            }
        }

        Ok(ComplianceReport {
            report_period_start: start_date,
            report_period_end: end_date,
            total_events: _events.len(),
            compliance_violations,
            data_access_events,
            authentication_failures,
            security_incidents,
            events: _events,
            generated_at: chrono::Utc::now(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityStatistics {
    pub total_events_24h: i64,
    pub event_type_counts: HashMap<String, i64>,
    pub severity_counts: HashMap<String, i64>,
    pub high_risk_events_24h: i64,
    pub currently_blocked_ips: i64,
    pub analysis_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub report_period_start: chrono::DateTime<chrono::Utc>,
    pub report_period_end: chrono::DateTime<chrono::Utc>,
    pub total_events: usize,
    pub compliance_violations: usize,
    pub data_access_events: usize,
    pub authentication_failures: usize,
    pub security_incidents: usize,
    pub events: Vec<SecurityAuditEvent>,
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

/// Helper functions for creating security audit events
impl SecurityAuditEvent {
    pub fn new(
        event_type: SecurityEventType,
        severity: SecuritySeverity,
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
            resource_type: "unknown".to_string(),
            resource_id: None,
            action,
            description,
            metadata: None,
            risk_score: 5.0, // Default medium risk
            mitigation_applied: false,
            compliance_tags: Vec::new(),
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_ip(mut self, ip_address: String) -> Self {
        self.ip_address = Some(ip_address);
        self
    }

    pub fn with_resource(mut self, resource_type: String, resource_id: Option<String>) -> Self {
        self.resource_type = resource_type;
        self.resource_id = resource_id;
        self
    }

    pub fn with_risk_score(mut self, risk_score: f64) -> Self {
        self.risk_score = risk_score;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_compliance_tags(mut self, tags: Vec<String>) -> Self {
        self.compliance_tags = tags;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_audit_event_creation() {
        let event = SecurityAuditEvent::new(
            SecurityEventType::AuthenticationFailure,
            SecuritySeverity::Medium,
            "login_attempt".to_string(),
            "Failed login attempt".to_string(),
        )
        .with_user(Uuid::new_v4())
        .with_ip("192.168.1.100".to_string())
        .with_resource("user_account".to_string(), Some("user123".to_string()))
        .with_risk_score(6.5)
        .with_compliance_tags(vec!["authentication".to_string(), "security".to_string()]);

        assert!(matches!(event.event_type, SecurityEventType::AuthenticationFailure));
        assert!(matches!(event.severity, SecuritySeverity::Medium));
        assert_eq!(event.action, "login_attempt");
        assert_eq!(event.risk_score, 6.5);
        assert!(event.user_id.is_some());
        assert_eq!(event.ip_address, Some("192.168.1.100".to_string()));
        assert_eq!(event.compliance_tags.len(), 2);
    }

    #[test]
    fn test_security_event_serialization() {
        let event = SecurityAuditEvent::new(
            SecurityEventType::SqlInjectionAttempt,
            SecuritySeverity::High,
            "query_execution".to_string(),
            "Potential SQL injection detected".to_string(),
        );

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: SecurityAuditEvent = serde_json::from_str(&serialized).unwrap();

        assert!(matches!(deserialized.event_type, SecurityEventType::SqlInjectionAttempt));
        assert!(matches!(deserialized.severity, SecuritySeverity::High));
    }
}
