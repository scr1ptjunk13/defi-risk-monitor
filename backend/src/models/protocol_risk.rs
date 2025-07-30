use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;
use uuid::Uuid;

/// Protocol risk assessment data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolRisk {
    pub id: Uuid,
    pub protocol_name: String,
    pub protocol_address: String,
    pub chain_id: i32,
    pub audit_score: BigDecimal,
    pub exploit_history_score: BigDecimal,
    pub tvl_score: BigDecimal,
    pub governance_score: BigDecimal,
    pub code_quality_score: BigDecimal,
    pub overall_protocol_risk: BigDecimal,
    pub last_updated: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Audit information for a protocol
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolAudit {
    pub id: Uuid,
    pub protocol_name: String,
    pub auditor_name: String,
    pub audit_date: DateTime<Utc>,
    pub audit_score: BigDecimal, // 0-100 scale
    pub critical_issues: i32,
    pub high_issues: i32,
    pub medium_issues: i32,
    pub low_issues: i32,
    pub audit_report_url: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Exploit/hack history for protocols
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolExploit {
    pub id: Uuid,
    pub protocol_name: String,
    pub exploit_date: DateTime<Utc>,
    pub exploit_type: ExploitType,
    pub amount_lost_usd: BigDecimal,
    pub severity: ExploitSeverity,
    pub description: Option<String>,
    pub was_recovered: bool,
    pub recovery_amount_usd: Option<BigDecimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Protocol TVL and governance metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolMetrics {
    pub id: Uuid,
    pub protocol_name: String,
    pub total_tvl_usd: BigDecimal,
    pub tvl_change_24h: BigDecimal,
    pub tvl_change_7d: BigDecimal,
    pub governance_participation_rate: Option<BigDecimal>,
    pub multisig_threshold: Option<i32>,
    pub timelock_delay_hours: Option<i32>,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "exploit_type", rename_all = "lowercase")]
pub enum ExploitType {
    FlashLoan,
    Reentrancy,
    Oracle,
    Governance,
    Bridge,
    SmartContract,
    Economic,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "exploit_severity", rename_all = "lowercase")]
pub enum ExploitSeverity {
    Critical,
    High,
    Medium,
    Low,
}

/// Protocol risk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolRiskConfig {
    pub audit_weight: BigDecimal,
    pub exploit_weight: BigDecimal,
    pub tvl_weight: BigDecimal,
    pub governance_weight: BigDecimal,
    pub code_quality_weight: BigDecimal,
    pub min_audit_score: BigDecimal,
    pub max_exploit_tolerance: BigDecimal,
    pub min_tvl_threshold: BigDecimal,
}

impl Default for ProtocolRiskConfig {
    fn default() -> Self {
        Self {
            audit_weight: BigDecimal::from_str("0.30").unwrap(),      // 30%
            exploit_weight: BigDecimal::from_str("0.25").unwrap(),    // 25%
            tvl_weight: BigDecimal::from_str("0.20").unwrap(),        // 20%
            governance_weight: BigDecimal::from_str("0.15").unwrap(), // 15%
            code_quality_weight: BigDecimal::from_str("0.10").unwrap(), // 10%
            min_audit_score: BigDecimal::from(70),
            max_exploit_tolerance: BigDecimal::from(10), // Max 10% of TVL lost
            min_tvl_threshold: BigDecimal::from(1000000), // $1M minimum TVL
        }
    }
}
