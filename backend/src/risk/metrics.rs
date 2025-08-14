// Risk Metrics Definitions for Different Protocols
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Enum containing protocol-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolRiskMetrics {
    Lido(LidoRiskMetrics),
    UniswapV3(UniswapV3RiskMetrics),
    Aave(AaveRiskMetrics),
    MakerDAO(MakerDAORiskMetrics),
    EigenLayer(EigenLayerRiskMetrics),
    Beefy(BeefyRiskMetrics),
    ConvexFinance(ConvexRiskMetrics),
    YearnFinance(YearnRiskMetrics),
    BalancerV2(BalancerV2RiskMetrics),
    Generic(GenericRiskMetrics), // For protocols without specific calculators
}

impl ProtocolRiskMetrics {
    /// Get the overall risk score regardless of protocol
    pub fn overall_risk_score(&self) -> BigDecimal {
        match self {
            ProtocolRiskMetrics::Lido(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::UniswapV3(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::Aave(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::MakerDAO(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::EigenLayer(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::Beefy(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::ConvexFinance(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::YearnFinance(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::BalancerV2(metrics) => metrics.overall_risk_score.clone(),
            ProtocolRiskMetrics::Generic(metrics) => metrics.overall_risk_score.clone(),
        }
    }
    
    /// Get the protocol name
    pub fn protocol_name(&self) -> &'static str {
        match self {
            ProtocolRiskMetrics::Lido(_) => "lido",
            ProtocolRiskMetrics::UniswapV3(_) => "uniswap_v3",
            ProtocolRiskMetrics::Aave(_) => "aave",
            ProtocolRiskMetrics::MakerDAO(_) => "makerdao",
            ProtocolRiskMetrics::EigenLayer(_) => "eigenlayer",
            ProtocolRiskMetrics::Beefy(_) => "beefy",
            ProtocolRiskMetrics::ConvexFinance(_) => "convex_finance",
            ProtocolRiskMetrics::YearnFinance(_) => "yearn_finance",
            ProtocolRiskMetrics::BalancerV2(_) => "balancer_v2",
            ProtocolRiskMetrics::Generic(_) => "generic",
        }
    }
    
    /// Get risk level as string
    pub fn risk_level(&self) -> String {
        let score = self.overall_risk_score();
        let score_f64 = score.to_string().parse::<f64>().unwrap_or(0.0);
        
        match score_f64 {
            s if s >= 80.0 => "Critical".to_string(),
            s if s >= 60.0 => "High".to_string(),
            s if s >= 40.0 => "Medium".to_string(),
            s if s >= 20.0 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }
}

/// Lido-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidoRiskMetrics {
    pub validator_slashing_risk: BigDecimal,
    pub steth_depeg_risk: BigDecimal,
    pub withdrawal_queue_risk: BigDecimal,
    pub protocol_governance_risk: BigDecimal,
    pub validator_performance_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub current_steth_peg: Option<BigDecimal>,
    pub withdrawal_queue_length: Option<u64>,
    pub active_validators: Option<u64>,
    pub slashed_validators: Option<u64>,
    pub total_staked_eth: Option<BigDecimal>,
    pub apy: Option<BigDecimal>,
}

/// Uniswap V3 specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniswapV3RiskMetrics {
    pub impermanent_loss_risk: BigDecimal,
    pub concentrated_liquidity_risk: BigDecimal,
    pub mev_risk: BigDecimal,
    pub price_impact_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub volatility_risk: BigDecimal,
    pub correlation_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub current_price: Option<BigDecimal>,
    pub price_range: Option<(BigDecimal, BigDecimal)>,
    pub liquidity_utilization: Option<BigDecimal>,
    pub volume_24h: Option<BigDecimal>,
    pub fees_earned: Option<BigDecimal>,
}

/// Aave-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaveRiskMetrics {
    pub liquidation_risk: BigDecimal,
    pub utilization_risk: BigDecimal,
    pub interest_rate_risk: BigDecimal,
    pub bad_debt_risk: BigDecimal,
    pub oracle_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub health_factor: Option<BigDecimal>,
    pub ltv_ratio: Option<BigDecimal>,
    pub liquidation_threshold: Option<BigDecimal>,
    pub current_utilization: Option<BigDecimal>,
    pub borrow_rate: Option<BigDecimal>,
    pub supply_rate: Option<BigDecimal>,
}

/// MakerDAO-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MakerDAORiskMetrics {
    pub collateralization_risk: BigDecimal,
    pub liquidation_risk: BigDecimal,
    pub stability_fee_risk: BigDecimal,
    pub oracle_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub peg_stability_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub collateralization_ratio: Option<BigDecimal>,
    pub liquidation_price: Option<BigDecimal>,
    pub stability_fee: Option<BigDecimal>,
    pub debt_ceiling: Option<BigDecimal>,
    pub dai_peg: Option<BigDecimal>,
}

/// EigenLayer-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EigenLayerRiskMetrics {
    pub multi_avs_slashing_risk: BigDecimal,
    pub operator_centralization_risk: BigDecimal,
    pub restaking_penalty_risk: BigDecimal,
    pub protocol_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub active_avs_count: Option<u64>,
    pub operator_stake: Option<BigDecimal>,
    pub total_restaked: Option<BigDecimal>,
    pub slashing_events: Option<u64>,
}

/// Beefy Finance-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeefyRiskMetrics {
    pub vault_strategy_risk: BigDecimal,
    pub underlying_protocol_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub vault_tvl: Option<BigDecimal>,
    pub strategy_count: Option<u64>,
    pub apy: Option<BigDecimal>,
    pub fees: Option<BigDecimal>,
}

/// Convex Finance-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvexRiskMetrics {
    pub curve_pool_risk: BigDecimal,
    pub convex_protocol_risk: BigDecimal,
    pub liquidity_mining_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub pool_tvl: Option<BigDecimal>,
    pub rewards_apr: Option<BigDecimal>,
    pub cvx_price: Option<BigDecimal>,
}

/// Yearn Finance-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearnRiskMetrics {
    pub vault_strategy_risk: BigDecimal,
    pub underlying_protocol_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub vault_version: Option<String>,
    pub strategy_count: Option<u64>,
    pub vault_tvl: Option<BigDecimal>,
    pub net_apy: Option<BigDecimal>,
}

/// Balancer V2-specific risk metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancerV2RiskMetrics {
    pub impermanent_loss_risk: BigDecimal,
    pub pool_composition_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub pool_type: Option<String>,
    pub pool_weights: Option<Vec<BigDecimal>>,
    pub swap_fee: Option<BigDecimal>,
    pub pool_tvl: Option<BigDecimal>,
}

/// Generic risk metrics for protocols without specific calculators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenericRiskMetrics {
    pub protocol_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub market_risk: BigDecimal,
    pub overall_risk_score: BigDecimal,
    
    // Additional context data
    pub protocol_name: String,
    pub tvl: Option<BigDecimal>,
    pub age_days: Option<u64>,
    pub audit_status: Option<String>,
}

/// Portfolio-level risk aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskMetrics {
    pub protocol_risks: HashMap<String, ProtocolRiskMetrics>,
    pub cross_protocol_correlation_risk: BigDecimal,
    pub concentration_risk: BigDecimal,
    pub liquidity_fragmentation_risk: BigDecimal,
    pub overall_portfolio_risk: BigDecimal,
    pub total_value_usd: BigDecimal,
    pub risk_breakdown: RiskBreakdown,
    pub top_risk_factors: Vec<String>,
    pub recommendations: Vec<String>,
}

/// Risk breakdown by category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBreakdown {
    pub protocol_risk: BigDecimal,
    pub smart_contract_risk: BigDecimal,
    pub liquidity_risk: BigDecimal,
    pub market_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    pub operational_risk: BigDecimal,
}

/// Risk assessment summary for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessmentSummary {
    pub overall_risk_score: BigDecimal,
    pub risk_level: String,
    pub total_positions: usize,
    pub protocols_analyzed: Vec<String>,
    pub high_risk_positions: usize,
    pub recommendations_count: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub confidence_score: BigDecimal,
}

impl PortfolioRiskMetrics {
    /// Create a new empty portfolio risk metrics
    pub fn new() -> Self {
        Self {
            protocol_risks: HashMap::new(),
            cross_protocol_correlation_risk: BigDecimal::from(0),
            concentration_risk: BigDecimal::from(0),
            liquidity_fragmentation_risk: BigDecimal::from(0),
            overall_portfolio_risk: BigDecimal::from(0),
            total_value_usd: BigDecimal::from(0),
            risk_breakdown: RiskBreakdown {
                protocol_risk: BigDecimal::from(0),
                smart_contract_risk: BigDecimal::from(0),
                liquidity_risk: BigDecimal::from(0),
                market_risk: BigDecimal::from(0),
                governance_risk: BigDecimal::from(0),
                operational_risk: BigDecimal::from(0),
            },
            top_risk_factors: Vec::new(),
            recommendations: Vec::new(),
        }
    }
    
    /// Get summary for API responses
    pub fn get_summary(&self) -> RiskAssessmentSummary {
        let high_risk_count = self.protocol_risks.values()
            .filter(|metrics| {
                let score = metrics.overall_risk_score().to_string().parse::<f64>().unwrap_or(0.0);
                score >= 60.0
            })
            .count();
            
        RiskAssessmentSummary {
            overall_risk_score: self.overall_portfolio_risk.clone(),
            risk_level: self.get_risk_level(),
            total_positions: self.protocol_risks.len(),
            protocols_analyzed: self.protocol_risks.keys().cloned().collect(),
            high_risk_positions: high_risk_count,
            recommendations_count: self.recommendations.len(),
            last_updated: chrono::Utc::now(),
            confidence_score: BigDecimal::from(85), // Default confidence
        }
    }
    
    /// Get risk level as string
    pub fn get_risk_level(&self) -> String {
        let score = self.overall_portfolio_risk.to_string().parse::<f64>().unwrap_or(0.0);
        
        match score {
            s if s >= 80.0 => "Critical".to_string(),
            s if s >= 60.0 => "High".to_string(),
            s if s >= 40.0 => "Medium".to_string(),
            s if s >= 20.0 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }
}
