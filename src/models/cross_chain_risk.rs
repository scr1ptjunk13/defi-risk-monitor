use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Cross-chain risk assessment for multi-chain DeFi positions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CrossChainRisk {
    pub id: Uuid,
    pub position_id: Option<Uuid>,
    pub primary_chain_id: i32,
    pub secondary_chain_ids: Vec<i32>,
    pub bridge_risk_score: BigDecimal,
    pub liquidity_fragmentation_risk: BigDecimal,
    pub governance_divergence_risk: BigDecimal,
    pub technical_risk_score: BigDecimal,
    pub correlation_risk_score: BigDecimal,
    pub overall_cross_chain_risk: BigDecimal,
    pub confidence_score: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bridge risk assessment for cross-chain transactions
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BridgeRisk {
    pub id: Uuid,
    pub bridge_protocol: String,
    pub source_chain_id: i32,
    pub destination_chain_id: i32,
    pub security_score: BigDecimal,
    pub tvl_locked: Option<BigDecimal>,
    pub exploit_history_count: i32,
    pub audit_score: BigDecimal,
    pub decentralization_score: BigDecimal,
    pub overall_bridge_risk: BigDecimal,
    pub last_assessment: DateTime<Utc>,
}

/// Chain-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChainRisk {
    pub id: Uuid,
    pub chain_id: i32,
    pub chain_name: String,
    pub network_security_score: BigDecimal,
    pub validator_decentralization: BigDecimal,
    pub governance_risk: BigDecimal,
    pub technical_maturity: BigDecimal,
    pub ecosystem_health: BigDecimal,
    pub liquidity_depth: BigDecimal,
    pub overall_chain_risk: BigDecimal,
    pub last_updated: DateTime<Utc>,
}

/// Cross-chain correlation metrics
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChainCorrelation {
    pub id: Uuid,
    pub chain_id_1: i32,
    pub chain_id_2: i32,
    pub price_correlation: BigDecimal,
    pub volume_correlation: BigDecimal,
    pub volatility_correlation: BigDecimal,
    pub overall_correlation: BigDecimal,
    pub calculation_period_days: i32,
    pub last_calculated: DateTime<Utc>,
}

/// Cross-chain risk severity levels
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "cross_chain_severity", rename_all = "lowercase")]
pub enum CrossChainSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Bridge protocol types
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "bridge_type", rename_all = "lowercase")]
pub enum BridgeType {
    LockAndMint,
    Atomic,
    Liquidity,
    Optimistic,
    ZkProof,
    Federated,
}

/// Cross-chain risk configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainRiskConfig {
    pub bridge_risk_weight: BigDecimal,
    pub liquidity_fragmentation_weight: BigDecimal,
    pub governance_divergence_weight: BigDecimal,
    pub technical_risk_weight: BigDecimal,
    pub correlation_risk_weight: BigDecimal,
    
    // Bridge risk thresholds
    pub bridge_tvl_critical_threshold: BigDecimal,
    pub bridge_exploit_critical_count: i32,
    pub bridge_audit_minimum_score: BigDecimal,
    
    // Chain risk thresholds
    pub chain_security_minimum_score: BigDecimal,
    pub validator_decentralization_minimum: BigDecimal,
    pub ecosystem_maturity_minimum: BigDecimal,
    
    // Correlation thresholds
    pub high_correlation_threshold: BigDecimal,
    pub critical_correlation_threshold: BigDecimal,
    
    // Liquidity fragmentation thresholds
    pub fragmentation_warning_threshold: BigDecimal,
    pub fragmentation_critical_threshold: BigDecimal,
}

impl Default for CrossChainRiskConfig {
    fn default() -> Self {
        Self {
            // Risk component weights (total = 100%)
            bridge_risk_weight: BigDecimal::from(30),           // 30% - Bridge security is critical
            liquidity_fragmentation_weight: BigDecimal::from(25), // 25% - Liquidity spread impact
            governance_divergence_weight: BigDecimal::from(20),  // 20% - Governance alignment
            technical_risk_weight: BigDecimal::from(15),        // 15% - Technical compatibility
            correlation_risk_weight: BigDecimal::from(10),      // 10% - Chain correlation risks
            
            // Bridge risk thresholds
            bridge_tvl_critical_threshold: BigDecimal::from(10000000), // $10M minimum TVL
            bridge_exploit_critical_count: 2,                   // 2+ exploits = high risk
            bridge_audit_minimum_score: BigDecimal::from(80),   // 80% minimum audit score
            
            // Chain risk thresholds
            chain_security_minimum_score: BigDecimal::from(70), // 70% minimum security
            validator_decentralization_minimum: BigDecimal::from(60), // 60% decentralization
            ecosystem_maturity_minimum: BigDecimal::from(50),   // 50% ecosystem maturity
            
            // Correlation thresholds
            high_correlation_threshold: BigDecimal::from(70),   // 70% correlation = high risk
            critical_correlation_threshold: BigDecimal::from(85), // 85% correlation = critical
            
            // Liquidity fragmentation thresholds
            fragmentation_warning_threshold: BigDecimal::from(30), // 30% fragmentation warning
            fragmentation_critical_threshold: BigDecimal::from(60), // 60% fragmentation critical
        }
    }
}

/// Cross-chain risk assessment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainRiskResult {
    pub overall_cross_chain_risk: BigDecimal,
    pub bridge_risk_score: BigDecimal,
    pub liquidity_fragmentation_risk: BigDecimal,
    pub governance_divergence_risk: BigDecimal,
    pub technical_risk_score: BigDecimal,
    pub correlation_risk_score: BigDecimal,
    pub confidence_score: BigDecimal,
    pub risk_factors: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Bridge security assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeSecurityAssessment {
    pub bridge_protocol: String,
    pub security_score: BigDecimal,
    pub audit_score: BigDecimal,
    pub tvl_score: BigDecimal,
    pub decentralization_score: BigDecimal,
    pub exploit_history_score: BigDecimal,
    pub overall_score: BigDecimal,
}

/// Chain ecosystem health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEcosystemHealth {
    pub chain_id: i32,
    pub total_tvl: BigDecimal,
    pub active_protocols: i32,
    pub daily_transactions: i64,
    pub validator_count: i32,
    pub governance_participation: BigDecimal,
    pub developer_activity: BigDecimal,
    pub health_score: BigDecimal,
}
