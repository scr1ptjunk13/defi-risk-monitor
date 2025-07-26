use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::str::FromStr;
use uuid::Uuid;

/// MEV risk assessment data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MevRisk {
    pub id: Uuid,
    pub pool_address: String,
    pub chain_id: i32,
    pub sandwich_risk_score: BigDecimal,
    pub frontrun_risk_score: BigDecimal,
    pub oracle_manipulation_risk: BigDecimal,
    pub oracle_deviation_risk: BigDecimal,
    pub overall_mev_risk: BigDecimal,
    pub confidence_score: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// MEV transaction detection result
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MevTransaction {
    pub id: Uuid,
    pub transaction_hash: String,
    pub block_number: i64,
    pub chain_id: i32,
    pub mev_type: MevType,
    pub severity: MevSeverity,
    pub profit_usd: Option<BigDecimal>,
    pub victim_loss_usd: Option<BigDecimal>,
    pub pool_address: String,
    pub detected_at: DateTime<Utc>,
}

/// Oracle price deviation event
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OracleDeviation {
    pub id: Uuid,
    pub oracle_address: String,
    pub token_address: String,
    pub chain_id: i32,
    pub oracle_price: BigDecimal,
    pub market_price: BigDecimal,
    pub deviation_percent: BigDecimal,
    pub severity: OracleDeviationSeverity,
    pub timestamp: DateTime<Utc>,
}

/// MEV attack types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mev_type", rename_all = "snake_case")]
pub enum MevType {
    SandwichAttack,
    Frontrunning,
    Backrunning,
    Arbitrage,
    Liquidation,
    Unknown,
}

/// MEV severity levels
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "mev_severity", rename_all = "snake_case")]
pub enum MevSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Oracle deviation severity levels
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "oracle_deviation_severity", rename_all = "snake_case")]
pub enum OracleDeviationSeverity {
    Minor,
    Moderate,
    Significant,
    Critical,
}

/// MEV risk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevRiskConfig {
    // Sandwich attack detection thresholds
    pub sandwich_price_impact_threshold: BigDecimal,
    pub sandwich_time_window_seconds: i64,
    
    // Oracle deviation thresholds
    pub oracle_deviation_warning_percent: BigDecimal,
    pub oracle_deviation_critical_percent: BigDecimal,
    pub oracle_staleness_threshold_seconds: i64,
    
    // Risk scoring weights
    pub sandwich_weight: BigDecimal,
    pub frontrun_weight: BigDecimal,
    pub oracle_manipulation_weight: BigDecimal,
    pub oracle_deviation_weight: BigDecimal,
    
    // Detection sensitivity
    pub min_transaction_value_usd: BigDecimal,
    pub mev_detection_lookback_blocks: i64,
}

impl Default for MevRiskConfig {
    fn default() -> Self {
        Self {
            sandwich_price_impact_threshold: BigDecimal::from_str("0.05").unwrap(), // 5%
            sandwich_time_window_seconds: 60, // 1 minute window
            
            oracle_deviation_warning_percent: BigDecimal::from_str("0.02").unwrap(), // 2%
            oracle_deviation_critical_percent: BigDecimal::from_str("0.10").unwrap(), // 10%
            oracle_staleness_threshold_seconds: 3600, // 1 hour
            
            sandwich_weight: BigDecimal::from_str("0.30").unwrap(), // 30%
            frontrun_weight: BigDecimal::from_str("0.25").unwrap(), // 25%
            oracle_manipulation_weight: BigDecimal::from_str("0.25").unwrap(), // 25%
            oracle_deviation_weight: BigDecimal::from_str("0.20").unwrap(), // 20%
            
            min_transaction_value_usd: BigDecimal::from_str("1000.0").unwrap(), // $1K minimum
            mev_detection_lookback_blocks: 100, // Look back 100 blocks
        }
    }
}

/// MEV detection result for a specific transaction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MevDetectionResult {
    pub mev_type: MevType,
    pub confidence: f64,
    pub risk_score: BigDecimal,
    pub evidence: Vec<String>,
    pub affected_transactions: Vec<String>,
    pub estimated_profit: Option<BigDecimal>,
    pub victim_loss: Option<BigDecimal>,
}

/// Oracle risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRiskResult {
    pub oracle_address: String,
    pub token_address: String,
    pub deviation_risk: BigDecimal,
    pub manipulation_risk: BigDecimal,
    pub staleness_risk: BigDecimal,
    pub overall_oracle_risk: BigDecimal,
    pub confidence: f64,
    pub last_update: DateTime<Utc>,
}
