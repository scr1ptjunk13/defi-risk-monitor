// Ether.fi Protocol Risk Calculator
// Specialized risk assessment for Ether.fi liquid staking and restaking positions

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
    EtherFiRiskMetrics,
    RealTimeRiskCalculator,
    ExplainableRiskCalculator,
    RiskExplanation,
    RiskFactorContribution
};

/// Ether.fi-specific risk calculator
#[derive(Debug)]
pub struct EtherFiRiskCalculator {
    // Configuration
    validator_slashing_threshold: f64,
    depeg_risk_threshold: f64,
    withdrawal_queue_threshold: u64,
    restaking_exposure_threshold: f64,
    
    // Cached data (in production, this would be more sophisticated)
    last_updated: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    cached_validator_metrics: std::sync::Mutex<Option<ValidatorMetrics>>,
    cached_eeth_peg: std::sync::Mutex<Option<f64>>,
    cached_protocol_metrics: std::sync::Mutex<Option<ProtocolMetrics>>,
    cached_restaking_metrics: std::sync::Mutex<Option<RestakingMetrics>>,
}

#[derive(Debug, Clone)]
struct ValidatorMetrics {
    total_validators: u64,
    active_validators: u64,
    validator_performance: f64,
    slashing_incidents: u64,
    average_uptime: f64,
}

#[derive(Debug, Clone)]
struct ProtocolMetrics {
    total_eth_staked: f64,
    eeth_supply: f64,
    eeth_exchange_rate: f64,
    liquid_capacity: f64,
    protocol_tvl_usd: f64,
    withdrawal_queue_length: u64,
    node_operator_count: u64,
}

#[derive(Debug, Clone)]
struct RestakingMetrics {
    total_restaked_eth: f64,
    eigenlayer_tvl: f64,
    active_avs_count: u64,
    restaking_yield: f64,
    slashing_conditions: u64,
}

impl EtherFiRiskCalculator {
    /// Create a new Ether.fi risk calculator with default thresholds
    pub fn new() -> Self {
        Self {
            validator_slashing_threshold: 0.01, // 1% slashing rate threshold
            depeg_risk_threshold: 0.03, // 3% depeg threshold
            withdrawal_queue_threshold: 1000, // 1000 ETH withdrawal queue threshold
            restaking_exposure_threshold: 0.5, // 50% restaking exposure threshold
            last_updated: std::sync::Mutex::new(None),
            cached_validator_metrics: std::sync::Mutex::new(None),
            cached_eeth_peg: std::sync::Mutex::new(None),
            cached_protocol_metrics: std::sync::Mutex::new(None),
            cached_restaking_metrics: std::sync::Mutex::new(None),
        }
    }
    
    /// Calculate validator slashing risk
    async fn calculate_validator_slashing_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating validator slashing risk for {} positions", positions.len());
        
        // Get cached or fetch validator metrics
        let validator_metrics = self.get_validator_metrics().await?;
        
        // Calculate validator performance
        let validator_performance = if validator_metrics.total_validators > 0 {
            validator_metrics.active_validators as f64 / validator_metrics.total_validators as f64
        } else {
            1.0
        };
        
        // Base slashing risk (0-20 points) - normalized for mainnet
        let base_slashing_risk: f64 = if validator_performance < 0.85 {
            20.0 // High slashing risk
        } else if validator_performance < 0.92 {
            12.0 // Medium slashing risk
        } else {
            6.0 // Low slashing risk for good performance
        };
        
        // Adjust based on slashing incidents
        let slashing_adjustment: f64 = if validator_metrics.slashing_incidents > 5 {
            8.0 // Recent slashing incidents increase risk
        } else if validator_metrics.slashing_incidents > 2 {
            4.0 // Some slashing incidents
        } else {
            0.0 // No recent slashing incidents
        };
        
        let total_slashing_risk = (base_slashing_risk + slashing_adjustment).min(25.0f64);
        
        BigDecimal::from_f64(total_slashing_risk)
            .ok_or_else(|| RiskError::CalculationError { message: "Failed to convert slashing risk to BigDecimal".to_string() })
    }
    
    /// Calculate eETH depeg risk
    async fn calculate_eeth_depeg_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating eETH depeg risk");
        
        // Get current eETH/ETH peg
        let peg_ratio = self.get_eeth_peg().await?;
        
        // Calculate deviation from expected peg (eETH should trade close to 1:1 with ETH)
        let deviation = (peg_ratio - 1.0).abs();
        
        // Base depeg risk (0-20 points) - normalized for mainnet
        let base_depeg_risk: f64 = if deviation > 0.05 {
            20.0 // High deviation - significant depeg risk
        } else if deviation > 0.02 {
            12.0 // Medium deviation
        } else if deviation > 0.01 {
            8.0 // Small deviation
        } else {
            3.0 // Healthy peg (lower for mainnet)
        };
        
        let total_depeg_risk = base_depeg_risk.min(20.0f64);
        
        BigDecimal::from_f64(total_depeg_risk)
            .ok_or_else(|| RiskError::CalculationError { message: "Failed to convert depeg risk to BigDecimal".to_string() })
    }
    
    /// Calculate restaking exposure risk (unique to Ether.fi)
    async fn calculate_restaking_exposure_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating restaking exposure risk");
        
        // Get restaking metrics
        let restaking_metrics = self.get_restaking_metrics().await?;
        
        // Calculate restaking exposure ratio
        let protocol_metrics = self.get_protocol_metrics().await?;
        let restaking_ratio = if protocol_metrics.total_eth_staked > 0.0 {
            restaking_metrics.total_restaked_eth / protocol_metrics.total_eth_staked
        } else {
            0.0
        };
        
        // Base restaking risk (0-25 points) - EigenLayer adds complexity
        let base_restaking_risk: f64 = if restaking_ratio > 0.8 {
            25.0 // Very high restaking exposure
        } else if restaking_ratio > 0.6 {
            18.0 // High restaking exposure
        } else if restaking_ratio > 0.4 {
            12.0 // Medium restaking exposure
        } else if restaking_ratio > 0.2 {
            8.0 // Low restaking exposure
        } else {
            4.0 // Minimal restaking exposure
        };
        
        // Adjust for number of AVS (Actively Validated Services)
        let avs_adjustment: f64 = if restaking_metrics.active_avs_count > 10 {
            5.0 // Many AVS increase complexity and risk
        } else if restaking_metrics.active_avs_count > 5 {
            3.0 // Moderate AVS count
        } else {
            0.0 // Few AVS
        };
        
        let total_restaking_risk = (base_restaking_risk + avs_adjustment).min(30.0f64);
        
        BigDecimal::from_f64(total_restaking_risk)
            .ok_or_else(|| RiskError::CalculationError { message: "Failed to convert restaking risk to BigDecimal".to_string() })
    }
    
    /// Get validator metrics (mock implementation)
    async fn get_validator_metrics(&self) -> Result<ValidatorMetrics, RiskError> {
        // Check cache first
        {
            let cache = self.cached_validator_metrics.lock().unwrap();
            if let Some(metrics) = cache.as_ref() {
                return Ok(metrics.clone());
            }
        }
        
        // In production, this would fetch from Ether.fi contracts and beacon chain
        let metrics = ValidatorMetrics {
            total_validators: 8500,
            active_validators: 8200,
            validator_performance: 0.965,
            slashing_incidents: 1,
            average_uptime: 0.992,
        };
        
        // Cache the result
        {
            let mut cache = self.cached_validator_metrics.lock().unwrap();
            *cache = Some(metrics.clone());
        }
        
        Ok(metrics)
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
        
        // In production, this would fetch from Ether.fi contracts
        let metrics = ProtocolMetrics {
            total_eth_staked: 650000.0,
            eeth_supply: 640000.0,
            eeth_exchange_rate: 1.015,
            liquid_capacity: 50000.0,
            protocol_tvl_usd: 2_600_000_000.0,
            withdrawal_queue_length: 150,
            node_operator_count: 45,
        };
        
        // Cache the result
        {
            let mut cache = self.cached_protocol_metrics.lock().unwrap();
            *cache = Some(metrics.clone());
        }
        
        Ok(metrics)
    }
    
    /// Get restaking metrics (mock implementation)
    async fn get_restaking_metrics(&self) -> Result<RestakingMetrics, RiskError> {
        // Check cache first
        {
            let cache = self.cached_restaking_metrics.lock().unwrap();
            if let Some(metrics) = cache.as_ref() {
                return Ok(metrics.clone());
            }
        }
        
        // In production, this would fetch from EigenLayer contracts
        let metrics = RestakingMetrics {
            total_restaked_eth: 320000.0,
            eigenlayer_tvl: 1_280_000_000.0,
            active_avs_count: 8,
            restaking_yield: 2.8,
            slashing_conditions: 24,
        };
        
        // Cache the result
        {
            let mut cache = self.cached_restaking_metrics.lock().unwrap();
            *cache = Some(metrics.clone());
        }
        
        Ok(metrics)
    }
    
    /// Get eETH/ETH peg (mock implementation)
    async fn get_eeth_peg(&self) -> Result<f64, RiskError> {
        // Check cache first
        {
            let cache = self.cached_eeth_peg.lock().unwrap();
            if let Some(peg) = cache.as_ref() {
                return Ok(*peg);
            }
        }
        
        // In production, this would fetch from DEX prices or Ether.fi contracts
        let peg_ratio = 1.002; // eETH trading at 0.2% premium
        
        // Cache the result
        {
            let mut cache = self.cached_eeth_peg.lock().unwrap();
            *cache = Some(peg_ratio);
        }
        
        Ok(peg_ratio)
    }
}

#[async_trait]
impl ProtocolRiskCalculator for EtherFiRiskCalculator {
    fn protocol_name(&self) -> &'static str {
        "ether_fi"
    }
    
    fn supported_position_types(&self) -> Vec<&'static str> {
        vec!["staking", "liquid_staking", "restaking", "node_operator"]
    }
    
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError> {
        if positions.is_empty() {
            return Err(RiskError::ValidationError { reason: "No positions provided".to_string() });
        }
        
        info!("Calculating Ether.fi risk for {} positions", positions.len());
        
        // Calculate individual risk components
        let validator_slashing_risk = self.calculate_validator_slashing_risk(positions).await?;
        let eeth_depeg_risk = self.calculate_eeth_depeg_risk(positions).await?;
        let restaking_exposure_risk = self.calculate_restaking_exposure_risk(positions).await?;
        
        // Normalized risk factors for Ether.fi mainnet
        let withdrawal_queue_risk = BigDecimal::from_f64(4.0).unwrap_or_default();
        let protocol_governance_risk = BigDecimal::from_f64(10.0).unwrap_or_default(); // Higher than Rocket Pool (newer protocol)
        let validator_performance_risk = BigDecimal::from_f64(6.0).unwrap_or_default();
        let liquidity_risk = BigDecimal::from_f64(8.0).unwrap_or_default(); // Slightly higher due to restaking complexity
        let smart_contract_risk = BigDecimal::from_f64(8.0).unwrap_or_default(); // Higher due to EigenLayer integration
        
        // Calculate weighted overall risk score using proper averaging
        let weights = [
            (&validator_slashing_risk, 0.20), // 20% weight
            (&eeth_depeg_risk, 0.18),         // 18% weight
            (&restaking_exposure_risk, 0.22), // 22% weight (highest - unique to Ether.fi)
            (&withdrawal_queue_risk, 0.08),   // 8% weight
            (&protocol_governance_risk, 0.12), // 12% weight
            (&validator_performance_risk, 0.08), // 8% weight
            (&liquidity_risk, 0.06),          // 6% weight
            (&smart_contract_risk, 0.06),     // 6% weight
        ];
        
        let mut weighted_sum = BigDecimal::zero();
        for (risk_score, weight) in weights.iter() {
            let weighted_score = *risk_score * BigDecimal::from_f64(*weight).unwrap_or_default();
            weighted_sum += weighted_score;
        }
        
        // Cap at reasonable maximum for Ether.fi mainnet (40 max due to restaking complexity)
        let max_mainnet_score = BigDecimal::from_f64(40.0).unwrap_or_default();
        let overall_risk_score = weighted_sum.min(max_mainnet_score);
        
        // Get additional metrics
        let protocol_metrics = self.get_protocol_metrics().await?;
        let validator_metrics = self.get_validator_metrics().await?;
        let restaking_metrics = self.get_restaking_metrics().await?;
        let peg_price = self.get_eeth_peg().await?;
        
        let etherfi_metrics = EtherFiRiskMetrics {
            overall_risk_score: overall_risk_score.clone(),
            validator_slashing_risk,
            eeth_depeg_risk,
            withdrawal_queue_risk,
            protocol_governance_risk,
            validator_performance_risk,
            liquidity_risk,
            smart_contract_risk,
            restaking_exposure_risk,
            
            // Additional metadata
            peg_price: BigDecimal::from_f64(peg_price).unwrap_or_default(),
            peg_deviation_percent: BigDecimal::from_f64((peg_price - 1.0) * 100.0).unwrap_or_default(),
            protocol_tvl_usd: BigDecimal::from_f64(protocol_metrics.protocol_tvl_usd).unwrap_or_default(),
            validator_count_total: validator_metrics.total_validators,
            withdrawal_queue_time_days: BigDecimal::from_str("1.5").unwrap_or_else(|_| BigDecimal::zero()),
            current_apy: BigDecimal::from_str("3.2").unwrap_or_else(|_| BigDecimal::zero()),
            restaking_tvl_usd: BigDecimal::from_f64(restaking_metrics.eigenlayer_tvl).unwrap_or_default(),
            active_avs_count: restaking_metrics.active_avs_count,
            
            // Historical data (mock values for now)
            historical_30d_avg: overall_risk_score.clone(),
            historical_7d_avg: overall_risk_score.clone(),
        };
        
        // Update last updated timestamp
        {
            let mut last_updated = self.last_updated.lock().unwrap();
            *last_updated = Some(chrono::Utc::now());
        }
        
        Ok(ProtocolRiskMetrics::EtherFi(etherfi_metrics))
    }
    
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError> {
        // Validate that this is an Ether.fi position
        if position.protocol != "ether_fi" {
            return Ok(false);
        }
        
        // Validate position type for Ether.fi
        match position.protocol.as_str() {
            "ether_fi" => Ok(true),
            _ => Ok(false),
        }
    }
    
    fn risk_factors(&self) -> Vec<&'static str> {
        vec![
            "validator_slashing_risk",
            "eeth_depeg_risk", 
            "withdrawal_queue_risk",
            "protocol_governance_risk",
            "validator_performance_risk",
            "liquidity_risk",
            "smart_contract_risk",
            "restaking_exposure_risk"
        ]
    }
}

#[async_trait]
impl RealTimeRiskCalculator for EtherFiRiskCalculator {
    async fn update_real_time_data(&self) -> Result<(), RiskError> {
        info!("Updating Ether.fi real-time risk data");
        
        // Clear caches to force refresh on next calculation
        {
            let mut cache = self.cached_validator_metrics.lock().unwrap();
            *cache = None;
        }
        {
            let mut cache = self.cached_eeth_peg.lock().unwrap();
            *cache = None;
        }
        {
            let mut cache = self.cached_protocol_metrics.lock().unwrap();
            *cache = None;
        }
        {
            let mut cache = self.cached_restaking_metrics.lock().unwrap();
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

impl ExplainableRiskCalculator for EtherFiRiskCalculator {
    fn explain_risk_calculation(&self, metrics: &ProtocolRiskMetrics) -> RiskExplanation {
        if let ProtocolRiskMetrics::EtherFi(etherfi_metrics) = metrics {
            let overall_score = etherfi_metrics.overall_risk_score.to_f64().unwrap_or(0.0);
            
            let risk_level = if overall_score < 15.0 {
                "very_low"
            } else if overall_score < 25.0 {
                "low"
            } else if overall_score < 35.0 {
                "medium"
            } else if overall_score < 50.0 {
                "high"
            } else {
                "critical"
            };
            
            let explanation = format!(
                "Ether.fi risk assessment considers decentralized node operators, eETH peg stability, liquidity depth, governance, and EigenLayer restaking exposure. Current risk level: {} (score: {:.1}).",
                risk_level, overall_score
            );
            
            RiskExplanation {
                overall_risk_score: overall_score,
                risk_level: risk_level.to_string(),
                primary_risk_factors: vec![
                    "Restaking exposure to EigenLayer".to_string(),
                    "Validator slashing risk".to_string(),
                    "eETH depeg risk".to_string(),
                ],
                explanation,
                methodology: "Comprehensive multi-factor risk analysis including restaking exposure".to_string(),
                confidence_score: 0.82,
                data_quality: "High".to_string(),
            }
        } else {
            RiskExplanation {
                overall_risk_score: 0.0,
                risk_level: "unknown".to_string(),
                primary_risk_factors: vec![],
                explanation: "Invalid metrics provided for Ether.fi risk explanation".to_string(),
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
    
    fn create_test_etherfi_position(value_usd: f64) -> Position {
        Position {
            id: "test_etherfi_position".to_string(),
            protocol: "ether_fi".to_string(),
            position_type: "staking".to_string(),
            pair: "eETH/ETH".to_string(),
            value_usd,
            pnl_usd: 0.0,
            pnl_percentage: 0.0,
            risk_score: 0,
            metadata: serde_json::json!({
                "token_address": "0x35fA164735182de50811E8e2E824cFb9B6118ac2",
                "current_apy": 3.2
            }),
            last_updated: 0,
        }
    }
    
    #[tokio::test]
    async fn test_etherfi_calculator_creation() {
        let calculator = EtherFiRiskCalculator::new();
        assert_eq!(calculator.protocol_name(), "ether_fi");
        assert!(calculator.supported_position_types().contains(&"staking"));
        assert!(calculator.supported_position_types().contains(&"restaking"));
    }
    
    #[tokio::test]
    async fn test_risk_calculation() {
        let calculator = EtherFiRiskCalculator::new();
        let positions = vec![create_test_etherfi_position(10000.0)];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(ProtocolRiskMetrics::EtherFi(metrics)) = result {
            assert!(metrics.overall_risk_score > BigDecimal::zero());
            assert!(metrics.validator_slashing_risk >= BigDecimal::zero());
            assert!(metrics.eeth_depeg_risk >= BigDecimal::zero());
            assert!(metrics.restaking_exposure_risk >= BigDecimal::zero());
        } else {
            panic!("Expected Ether.fi risk metrics");
        }
    }
    
    #[tokio::test]
    async fn test_position_validation() {
        let calculator = EtherFiRiskCalculator::new();
        
        let valid_position = create_test_etherfi_position(1000.0);
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
}
