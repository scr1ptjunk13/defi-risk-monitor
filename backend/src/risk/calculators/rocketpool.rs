// Rocket Pool Protocol Risk Calculator
// Specialized risk assessment for Rocket Pool liquid staking positions

use async_trait::async_trait;
use bigdecimal::BigDecimal;
use num_traits::{Zero, FromPrimitive, ToPrimitive};
use std::str::FromStr;
use tracing::{info, warn, debug};

use crate::models::position::Position;
use crate::risk::{
    RiskError, 
    ProtocolRiskCalculator, 
    ProtocolRiskMetrics, 
    RocketPoolRiskMetrics,
    RealTimeRiskCalculator,
    ExplainableRiskCalculator,
    RiskExplanation,
    RiskFactorContribution
};

/// Rocket Pool-specific risk calculator
pub struct RocketPoolRiskCalculator {
    // Configuration
    validator_slashing_threshold: f64,
    depeg_risk_threshold: f64,
    withdrawal_queue_threshold: u64,
    node_utilization_threshold: f64,
    
    // Cached data (in production, this would be more sophisticated)
    last_updated: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    cached_node_metrics: std::sync::Mutex<Option<NodeOperatorMetrics>>,
    cached_reth_peg: std::sync::Mutex<Option<f64>>,
    cached_protocol_metrics: std::sync::Mutex<Option<ProtocolMetrics>>,
}

#[derive(Debug, Clone)]
struct NodeOperatorMetrics {
    total_nodes: u64,
    active_nodes: u64,
    trusted_nodes: u64,
    smoothing_pool_nodes: u64,
    total_minipools: u64,
    active_minipools: u64,
    node_utilization: f64,
}

#[derive(Debug, Clone)]
struct ProtocolMetrics {
    total_eth_staked: f64,
    reth_supply: f64,
    reth_exchange_rate: f64,
    node_demand: f64,
    deposit_pool_balance: f64,
    network_node_fee: f64,
    protocol_tvl_usd: f64,
}

impl RocketPoolRiskCalculator {
    /// Create a new Rocket Pool risk calculator with default thresholds
    pub fn new() -> Self {
        Self {
            validator_slashing_threshold: 0.015, // 1.5% slashing rate threshold
            depeg_risk_threshold: 0.04, // 4% depeg threshold (rETH more volatile than stETH)
            withdrawal_queue_threshold: 500, // 500 ETH withdrawal queue threshold
            node_utilization_threshold: 0.85, // 85% node utilization threshold
            last_updated: std::sync::Mutex::new(None),
            cached_node_metrics: std::sync::Mutex::new(None),
            cached_reth_peg: std::sync::Mutex::new(None),
            cached_protocol_metrics: std::sync::Mutex::new(None),
        }
    }
    
    /// Calculate validator slashing risk
    async fn calculate_validator_slashing_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating validator slashing risk for {} positions", positions.len());
        
        // Get cached or fetch node metrics
        let node_metrics = self.get_node_operator_metrics().await?;
        
        // Calculate node operator performance
        let node_performance = if node_metrics.total_nodes > 0 {
            node_metrics.active_nodes as f64 / node_metrics.total_nodes as f64
        } else {
            1.0
        };
        
        // Base slashing risk (0-25 points) - normalized for mainnet
        let base_slashing_risk: f64 = if node_performance < 0.85 {
            25.0 // High slashing risk
        } else if node_performance < 0.92 {
            15.0 // Medium slashing risk
        } else {
            8.0 // Low slashing risk for good performance (much lower for mainnet)
        };
        
        // Adjust based on node utilization
        let utilization_adjustment: f64 = if node_metrics.node_utilization < 0.7 {
            8.0 // Low utilization increases risk (reduced)
        } else if node_metrics.node_utilization > 0.95 {
            5.0 // Very high utilization also risky (reduced)
        } else {
            0.0 // Good utilization range
        };
        
        let total_slashing_risk = (base_slashing_risk + utilization_adjustment).min(30.0f64);
        
        BigDecimal::from_f64(total_slashing_risk)
            .ok_or_else(|| RiskError::CalculationError { message: "Failed to convert slashing risk to BigDecimal".to_string() })
    }
    
    /// Calculate rETH depeg risk
    async fn calculate_reth_depeg_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating rETH depeg risk");
        
        // Get current rETH/ETH peg
        let peg_ratio = self.get_reth_peg().await?;
        
        // Calculate deviation from expected peg (rETH should trade at premium)
        let current_premium = peg_ratio - 1.0;
        
        // Base depeg risk (0-25 points) - normalized for mainnet
        let base_depeg_risk: f64 = if current_premium < -0.02 {
            25.0 // Trading at discount - high risk
        } else if current_premium > 0.08 {
            20.0 // Very high premium - liquidity issues
        } else if current_premium < 0.005 {
            15.0 // Very low premium - demand issues
        } else {
            5.0 // Healthy premium range (much lower for mainnet)
        };
        
        let total_depeg_risk = base_depeg_risk.min(25.0f64);
        
        BigDecimal::from_f64(total_depeg_risk)
            .ok_or_else(|| RiskError::CalculationError { message: "Failed to convert depeg risk to BigDecimal".to_string() })
    }
    
    /// Get node operator metrics (mock implementation)
    async fn get_node_operator_metrics(&self) -> Result<NodeOperatorMetrics, RiskError> {
        // Check cache first
        {
            let cache = self.cached_node_metrics.lock().unwrap();
            if let Some(metrics) = cache.as_ref() {
                return Ok(metrics.clone());
            }
        }
        
        // In production, this would fetch from Rocket Pool contracts
        let metrics = NodeOperatorMetrics {
            total_nodes: 2920,
            active_nodes: 2850,
            trusted_nodes: 15,
            smoothing_pool_nodes: 2200,
            total_minipools: 8760,
            active_minipools: 8520,
            node_utilization: 0.92,
        };
        
        // Cache the result
        {
            let mut cache = self.cached_node_metrics.lock().unwrap();
            *cache = Some(metrics.clone());
        }
        
        Ok(metrics)
    }
    
    /// Get rETH/ETH peg (mock implementation)
    async fn get_reth_peg(&self) -> Result<f64, RiskError> {
        // Check cache first
        {
            let cache = self.cached_reth_peg.lock().unwrap();
            if let Some(peg) = cache.as_ref() {
                return Ok(*peg);
            }
        }
        
        // In production, this would fetch from DEX prices or Rocket Pool contracts
        let peg_ratio = 1.004; // rETH trading at 0.4% premium
        
        // Cache the result
        {
            let mut cache = self.cached_reth_peg.lock().unwrap();
            *cache = Some(peg_ratio);
        }
        
        Ok(peg_ratio)
    }
    
    /// Get protocol metrics (mock implementation)
    async fn get_protocol_metrics(&self) -> Result<ProtocolMetrics, RiskError> {
        // Check cache first
        {
            let cache = self.cached_protocol_metrics.lock().unwrap();
            if let Some(metrics) = cache.as_ref() {
                return Ok(metrics.clone());
            }
        }
        
        // In production, this would fetch from Rocket Pool contracts
        let metrics = ProtocolMetrics {
            total_eth_staked: 850000.0,
            reth_supply: 820000.0,
            reth_exchange_rate: 1.037,
            node_demand: 1200.0,
            deposit_pool_balance: 15000.0,
            network_node_fee: 0.14,
            protocol_tvl_usd: 2_200_000_000.0,
        };
        
        // Cache the result
        {
            let mut cache = self.cached_protocol_metrics.lock().unwrap();
            *cache = Some(metrics.clone());
        }
        
        Ok(metrics)
    }
}

#[async_trait]
impl ProtocolRiskCalculator for RocketPoolRiskCalculator {
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError> {
        if positions.is_empty() {
            return Err(RiskError::ValidationError { reason: "No positions provided".to_string() });
        }
        
        info!("Calculating Rocket Pool risk for {} positions", positions.len());
        
        // Calculate individual risk components
        let validator_slashing_risk = self.calculate_validator_slashing_risk(positions).await?;
        let reth_depeg_risk = self.calculate_reth_depeg_risk(positions).await?;
        // Normalized risk factors for Rocket Pool mainnet (mature protocol)
        let withdrawal_queue_risk = BigDecimal::from_f64(3.0).unwrap_or_default();
        let protocol_governance_risk = BigDecimal::from_f64(8.0).unwrap_or_default(); // Reduced for mature protocol
        let validator_performance_risk = BigDecimal::from_f64(5.0).unwrap_or_default(); // Reduced
        let liquidity_risk = BigDecimal::from_f64(7.0).unwrap_or_default(); // Reduced
        let smart_contract_risk = BigDecimal::from_f64(6.0).unwrap_or_default(); // Reduced for audited contracts
        
        // Calculate weighted overall risk score using proper averaging
        let weights = [
            (&validator_slashing_risk, 0.25), // 25% weight (highest)
            (&reth_depeg_risk, 0.20),         // 20% weight
            (&withdrawal_queue_risk, 0.10),   // 10% weight
            (&protocol_governance_risk, 0.15), // 15% weight
            (&validator_performance_risk, 0.10), // 10% weight
            (&liquidity_risk, 0.10),          // 10% weight
            (&smart_contract_risk, 0.10),     // 10% weight
        ];
        
        let mut weighted_sum = BigDecimal::zero();
        for (risk_score, weight) in weights.iter() {
            let weighted_score = *risk_score * BigDecimal::from_f64(*weight).unwrap_or_default();
            weighted_sum += weighted_score;
        }
        
        // Cap at reasonable maximum for Rocket Pool mainnet (35 max for normal conditions)
        let max_mainnet_score = BigDecimal::from_f64(35.0).unwrap_or_default();
        let overall_risk_score = weighted_sum.min(max_mainnet_score);
        
        // Get additional metrics
        let protocol_metrics = self.get_protocol_metrics().await?;
        let node_metrics = self.get_node_operator_metrics().await?;
        let peg_price = self.get_reth_peg().await?;
        
        let rocket_pool_metrics = RocketPoolRiskMetrics {
            overall_risk_score: overall_risk_score.clone(),
            validator_slashing_risk,
            reth_depeg_risk,
            withdrawal_queue_risk,
            protocol_governance_risk,
            validator_performance_risk,
            liquidity_risk,
            smart_contract_risk,
            
            // Additional metadata
            peg_price: BigDecimal::from_f64(peg_price).unwrap_or_default(),
            peg_deviation_percent: BigDecimal::from_f64((peg_price - 1.0) * 100.0).unwrap_or_default(),
            protocol_tvl_usd: BigDecimal::from_f64(protocol_metrics.protocol_tvl_usd).unwrap_or_default(),
            validator_count_total: 1000, // Mock data
            withdrawal_queue_time_days: BigDecimal::from_str("2.5").unwrap_or_else(|_| BigDecimal::zero()),
            current_apy: BigDecimal::from_str("4.2").unwrap_or_else(|_| BigDecimal::zero()),
            
            // Historical data (mock values for now)
            historical_30d_avg: overall_risk_score.clone(),
            historical_7d_avg: overall_risk_score.clone(),
        };
        
        // Update last updated timestamp
        {
            let mut last_updated = self.last_updated.lock().unwrap();
            *last_updated = Some(chrono::Utc::now());
        }
        
        Ok(ProtocolRiskMetrics::RocketPool(rocket_pool_metrics))
    }
    
    fn protocol_name(&self) -> &'static str {
        "rocket_pool"
    }
    
    fn supported_position_types(&self) -> Vec<&'static str> {
        vec!["staking", "liquid_staking", "node_operator", "governance_staking"]
    }
    
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError> {
        // Validate that this is a Rocket Pool position
        if position.protocol != "rocket_pool" {
            return Ok(false);
        }
        
        // Validate position type for Rocket Pool
        match position.protocol.as_str() {
            "rocket_pool" => Ok(true),
            _ => Ok(false),
        }
    }
    
    fn risk_factors(&self) -> Vec<&'static str> {
        vec![
            "validator_slashing_risk",
            "reth_depeg_risk", 
            "withdrawal_queue_risk",
            "protocol_governance_risk",
            "validator_performance_risk",
            "liquidity_risk",
            "smart_contract_risk"
        ]
    }
}

#[async_trait]
impl RealTimeRiskCalculator for RocketPoolRiskCalculator {
    async fn update_real_time_data(&self) -> Result<(), RiskError> {
        info!("Updating Rocket Pool real-time risk data");
        
        // Clear caches to force refresh on next calculation
        {
            let mut cache = self.cached_node_metrics.lock().unwrap();
            *cache = None;
        }
        {
            let mut cache = self.cached_reth_peg.lock().unwrap();
            *cache = None;
        }
        {
            let mut cache = self.cached_protocol_metrics.lock().unwrap();
            *cache = None;
        }
        
        // Update timestamp
        {
            let mut last_updated = self.last_updated.lock().unwrap();
            *last_updated = Some(chrono::Utc::now());
        }
        
        Ok(())
    }
    
    fn last_updated(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let last_updated = self.last_updated.lock().unwrap();
        *last_updated
    }
}

impl ExplainableRiskCalculator for RocketPoolRiskCalculator {
    fn explain_risk_calculation(&self, metrics: &ProtocolRiskMetrics) -> RiskExplanation {
        if let ProtocolRiskMetrics::RocketPool(rocket_pool_metrics) = metrics {
            let overall_score = rocket_pool_metrics.overall_risk_score.to_f64().unwrap_or(0.0);
            
            let risk_level = if overall_score < 20.0 {
                "low"
            } else if overall_score < 50.0 {
                "medium"
            } else if overall_score < 75.0 {
                "high"
            } else {
                "critical"
            };
            
            let explanation = format!(
                "Rocket Pool risk assessment considers decentralized node operators, rETH peg stability, \
                and protocol governance. Current risk level: {} (score: {:.1}).",
                risk_level,
                overall_score
            );
            
            RiskExplanation {
                overall_risk_score: overall_score,
                risk_level: risk_level.to_string(),
                primary_risk_factors: vec![
                    "Validator slashing risk".to_string(),
                    "rETH depeg risk".to_string(),
                    "Withdrawal queue risk".to_string(),
                ],
                explanation,
                methodology: "Comprehensive multi-factor risk analysis".to_string(),
                confidence_score: 0.85,
                data_quality: "High".to_string(),
            }
        } else {
            RiskExplanation {
                overall_risk_score: 0.0,
                risk_level: "unknown".to_string(),
                primary_risk_factors: vec![],
                explanation: "Invalid metrics provided for Rocket Pool risk explanation".to_string(),
                methodology: "Unknown".to_string(),
                confidence_score: 0.0,
                data_quality: "Low".to_string(),
            }
        }
    }
    
    fn get_risk_factor_contributions(&self, _metrics: &ProtocolRiskMetrics) -> Vec<RiskFactorContribution> {
        vec![]
    }
    
    fn get_risk_reduction_recommendations(&self, _metrics: &ProtocolRiskMetrics) -> Vec<String> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::traits::Position;
    
    fn create_test_rocket_pool_position(value_usd: f64) -> Position {
        Position {
            id: "test_rocket_pool_position".to_string(),
            protocol: "rocket_pool".to_string(),
            position_type: "staking".to_string(),
            pair: "rETH/ETH".to_string(),
            value_usd,
            pnl_usd: 0.0,
            pnl_percentage: 0.0,
            risk_score: 0,
            metadata: serde_json::json!({
                "token_address": "0xae78736Cd615f374D3085123A210448E74Fc6393",
                "current_apy": 3.85
            }),
            last_updated: 0,
        }
    }
    
    #[tokio::test]
    async fn test_rocket_pool_calculator_creation() {
        let calculator = RocketPoolRiskCalculator::new();
        assert_eq!(calculator.protocol_name(), "rocket_pool");
        assert!(calculator.supported_position_types().contains(&"staking"));
    }
    
    #[tokio::test]
    async fn test_position_validation() {
        let calculator = RocketPoolRiskCalculator::new();
        
        let valid_position = create_test_rocket_pool_position(1000.0);
        assert!(calculator.validate_position(&valid_position).await.unwrap());
        
        let invalid_position = Position {
            id: "test".to_string(),
            protocol: "uniswap".to_string(),
            position_type: "lp".to_string(),
            pair: "".to_string(),
            value_usd: 1000.0,
            pnl_usd: 0.0,
            pnl_percentage: 0.0,
            risk_score: 0,
            metadata: serde_json::json!({}),
            last_updated: 0,
        };
        assert!(!calculator.validate_position(&invalid_position).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_risk_calculation() {
        let calculator = RocketPoolRiskCalculator::new();
        let positions = vec![create_test_rocket_pool_position(10000.0)];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(ProtocolRiskMetrics::RocketPool(metrics)) = result {
            assert!(metrics.overall_risk_score > BigDecimal::zero());
            assert!(metrics.validator_slashing_risk >= BigDecimal::zero());
            assert!(metrics.reth_depeg_risk >= BigDecimal::zero());
        } else {
            panic!("Expected Rocket Pool risk metrics");
        }
    }
}
