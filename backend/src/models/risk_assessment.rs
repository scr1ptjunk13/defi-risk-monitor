use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// General risk assessment for positions, protocols, or users
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RiskAssessment {
    pub id: Uuid,
    pub entity_type: RiskEntityType, // position, protocol, user, portfolio
    pub entity_id: String, // UUID or address of the entity being assessed
    pub user_id: Option<Uuid>, // User who owns this assessment (for position/portfolio risks)
    pub risk_type: RiskType,
    pub risk_score: BigDecimal, // 0.0 to 1.0 scale
    pub severity: RiskSeverity,
    pub confidence: BigDecimal, // 0.0 to 1.0 confidence in the assessment
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>, // Additional risk-specific data
    pub expires_at: Option<DateTime<Utc>>, // When this assessment expires
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Risk assessment history for tracking changes over time
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RiskAssessmentHistory {
    pub id: Uuid,
    pub risk_assessment_id: Uuid,
    pub previous_risk_score: BigDecimal,
    pub new_risk_score: BigDecimal,
    pub previous_severity: RiskSeverity,
    pub new_severity: RiskSeverity,
    pub change_reason: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "risk_entity_type", rename_all = "snake_case")]
pub enum RiskEntityType {
    Position,
    Protocol,
    User,
    Portfolio,
    Pool,
    Token,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "risk_type", rename_all = "snake_case")]
pub enum RiskType {
    ImpermanentLoss,
    Liquidity,
    Protocol,
    Mev,
    CrossChain,
    Market,
    Slippage,
    Correlation,
    Volatility,
    Overall,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "risk_severity", rename_all = "snake_case")]
pub enum RiskSeverity {
    Critical,
    High,
    Medium,
    Low,
    Minimal,
}

/// Bulk risk assessment for efficient batch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkRiskAssessment {
    pub entity_type: RiskEntityType,
    pub entity_id: String,
    pub user_id: Option<Uuid>,
    pub risk_type: RiskType,
    pub risk_score: BigDecimal,
    pub severity: RiskSeverity,
    pub confidence: BigDecimal,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Risk assessment query filters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentFilter {
    pub entity_type: Option<RiskEntityType>,
    pub entity_id: Option<String>,
    pub user_id: Option<Uuid>,
    pub risk_type: Option<RiskType>,
    pub severity: Option<RiskSeverity>,
    pub min_risk_score: Option<BigDecimal>,
    pub max_risk_score: Option<BigDecimal>,
    pub is_active: Option<bool>,
    pub expired_only: bool,
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

impl Default for RiskAssessmentFilter {
    fn default() -> Self {
        Self {
            entity_type: None,
            entity_id: None,
            user_id: None,
            risk_type: None,
            severity: None,
            min_risk_score: None,
            max_risk_score: None,
            is_active: Some(true),
            expired_only: false,
            created_after: None,
            created_before: None,
            limit: Some(100),
            offset: Some(0),
        }
    }
}
