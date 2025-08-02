use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Alert {
    pub id: Uuid,
    pub user_address: String, // Added to match handler expectations
    pub position_id: Option<Uuid>,
    pub threshold_id: Option<Uuid>, // Added to match handler expectations
    pub alert_type: String,
    pub severity: String, // Will be converted to/from AlertSeverity
    pub title: String,
    pub message: String,
    pub risk_score: Option<BigDecimal>,
    pub current_value: Option<BigDecimal>,
    pub threshold_value: Option<BigDecimal>,
    pub metadata: Option<serde_json::Value>, // Added to match handler expectations
    pub is_resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlert {
    pub user_address: String, // Added
    pub position_id: Option<Uuid>,
    pub threshold_id: Option<Uuid>, // Added
    pub alert_type: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub risk_score: Option<BigDecimal>,
    pub current_value: Option<BigDecimal>,
    pub threshold_value: Option<BigDecimal>,
    pub metadata: Option<serde_json::Value>, // Added
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAlert {
    pub is_resolved: Option<bool>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl Alert {
    pub fn new(create_alert: CreateAlert) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_address: create_alert.user_address,
            position_id: create_alert.position_id,
            threshold_id: create_alert.threshold_id,
            alert_type: create_alert.alert_type,
            severity: match create_alert.severity {
                AlertSeverity::Low => "low".to_string(),
                AlertSeverity::Medium => "medium".to_string(),
                AlertSeverity::High => "high".to_string(),
                AlertSeverity::Critical => "critical".to_string(),
            },
            title: create_alert.title,
            message: create_alert.message,
            risk_score: create_alert.risk_score,
            current_value: create_alert.current_value,
            threshold_value: create_alert.threshold_value,
            metadata: create_alert.metadata,
            is_resolved: false,
            resolved_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn get_severity(&self) -> AlertSeverity {
        match self.severity.as_str() {
            "low" => AlertSeverity::Low,
            "medium" => AlertSeverity::Medium,
            "high" => AlertSeverity::High,
            "critical" => AlertSeverity::Critical,
            _ => AlertSeverity::Low,
        }
    }

    pub fn resolve(&mut self) {
        self.is_resolved = true;
        self.resolved_at = Some(Utc::now());
    }
}
