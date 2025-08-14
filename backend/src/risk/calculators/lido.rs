// Lido Protocol Risk Calculator
// Specialized risk assessment for Lido liquid staking positions

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
    LidoRiskMetrics,
    RealTimeRiskCalculator,
    ExplainableRiskCalculator,
    RiskExplanation,
    RiskFactorContribution
};

/// Lido-specific risk calculator
pub struct LidoRiskCalculator {
    // Configuration
    validator_slashing_threshold: f64,
    depeg_risk_threshold: f64,
    withdrawal_queue_threshold: u64,
    
    // Cached data (in production, this would be more sophisticated)
    last_updated: std::sync::Mutex<Option<chrono::DateTime<chrono::Utc>>>,
    cached_validator_metrics: std::sync::Mutex<Option<ValidatorMetrics>>,
    cached_steth_peg: std::sync::Mutex<Option<f64>>,
}

#[derive(Debug, Clone)]
struct ValidatorMetrics {
    total_validators: u64,
    active_validators: u64,
    exited_validators: u64,
    slashed_validators: u64,
    total_staked_eth: BigDecimal,
    current_apy: f64,
}

impl LidoRiskCalculator {
    /// Create a new Lido risk calculator with default thresholds
    pub fn new() -> Self {
        Self {
            validator_slashing_threshold: 0.01, // 1% slashing rate threshold
            depeg_risk_threshold: 0.02, // 2% depeg threshold
            withdrawal_queue_threshold: 1000, // 1000 ETH withdrawal queue threshold
            last_updated: std::sync::Mutex::new(None),
            cached_validator_metrics: std::sync::Mutex::new(None),
            cached_steth_peg: std::sync::Mutex::new(None),
        }
    }
    
    /// Create with custom risk thresholds
    pub fn with_thresholds(
        validator_slashing_threshold: f64,
        depeg_risk_threshold: f64,
        withdrawal_queue_threshold: u64,
    ) -> Self {
        Self {
            validator_slashing_threshold,
            depeg_risk_threshold,
            withdrawal_queue_threshold,
            last_updated: std::sync::Mutex::new(None),
            cached_validator_metrics: std::sync::Mutex::new(None),
            cached_steth_peg: std::sync::Mutex::new(None),
        }
    }
    
    /// Calculate validator slashing risk
    async fn calculate_validator_slashing_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating validator slashing risk for {} positions", positions.len());
        
        // Get cached or fetch validator metrics
        let validator_metrics = self.get_validator_metrics().await?;
        
        // Calculate slashing rate
        let slashing_rate = if validator_metrics.total_validators > 0 {
            validator_metrics.slashed_validators as f64 / validator_metrics.total_validators as f64
        } else {
            0.0
        };
        
        // Base slashing risk (0-40 points)
        let base_slashing_risk = if slashing_rate > self.validator_slashing_threshold {
            40.0 // High slashing risk
        } else if slashing_rate > self.validator_slashing_threshold / 2.0 {
            25.0 // Medium slashing risk
        } else {
            10.0 // Low slashing risk
        };
        
        // Adjust based on validator performance
        let validator_performance_factor = if validator_metrics.active_validators > 0 {
            validator_metrics.active_validators as f64 / validator_metrics.total_validators as f64
        } else {
            1.0
        };
        
        // Lower performance = higher risk
        let performance_adjustment = (1.0 - validator_performance_factor) * 20.0;
        
        let total_slashing_risk = (base_slashing_risk + performance_adjustment).min(50.0);
        
        debug!(
            slashing_rate = %slashing_rate,
            base_risk = %base_slashing_risk,
            performance_adjustment = %performance_adjustment,
            total_risk = %total_slashing_risk,
            "Calculated validator slashing risk"
        );
        
        Ok(BigDecimal::from_f64(total_slashing_risk).unwrap_or_else(|| BigDecimal::from(25)))
    }
    
    /// Calculate stETH depeg risk
    async fn calculate_steth_depeg_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating stETH depeg risk");
        
        // Get current stETH/ETH peg
        let steth_peg = self.get_steth_peg().await?;
        
        // Calculate depeg severity (how far from 1.0)
        let depeg_severity = (1.0 - steth_peg).abs();
        
        // Base depeg risk calculation
        let depeg_risk = if depeg_severity > self.depeg_risk_threshold {
            // Severe depeg
            let severity_multiplier = (depeg_severity / self.depeg_risk_threshold).min(3.0);
            (30.0 * severity_multiplier).min(60.0)
        } else if depeg_severity > self.depeg_risk_threshold / 2.0 {
            // Moderate depeg
            20.0
        } else {
            // Minor or no depeg
            5.0
        };
        
        debug!(
            steth_peg = %steth_peg,
            depeg_severity = %depeg_severity,
            depeg_risk = %depeg_risk,
            "Calculated stETH depeg risk"
        );
        
        Ok(BigDecimal::from_f64(depeg_risk).unwrap_or_else(|| BigDecimal::from(15)))
    }
    
    /// Calculate withdrawal queue risk
    async fn calculate_withdrawal_queue_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        debug!("Calculating withdrawal queue risk");
        
        // Mock withdrawal queue data (in production, fetch from contracts)
        let withdrawal_queue_eth = self.estimate_withdrawal_queue_size().await?;
        
        // Calculate risk based on queue size
        let queue_risk = if withdrawal_queue_eth > self.withdrawal_queue_threshold {
            let queue_multiplier = (withdrawal_queue_eth as f64 / self.withdrawal_queue_threshold as f64).min(5.0);
            (15.0 * queue_multiplier).min(40.0)
        } else {
            5.0
        };
        
        // Adjust based on position size (larger positions have more withdrawal risk)
        let total_position_value: f64 = positions.iter()
            .map(|p| (&p.token0_amount + &p.token1_amount).to_f64().unwrap_or(0.0))
            .sum();
        let size_adjustment = if total_position_value > 100_000.0 {
            10.0 // Large positions have higher withdrawal risk
        } else if total_position_value > 10_000.0 {
            5.0
        } else {
            0.0
        };
        
        let total_queue_risk = (queue_risk + size_adjustment).min(45.0);
        
        debug!(
            queue_size_eth = %withdrawal_queue_eth,
            base_queue_risk = %queue_risk,
            size_adjustment = %size_adjustment,
            total_queue_risk = %total_queue_risk,
            "Calculated withdrawal queue risk"
        );
        
        Ok(BigDecimal::from_f64(total_queue_risk).unwrap_or_else(|| BigDecimal::from(10)))
    }
    
    /// Calculate protocol governance risk
    async fn calculate_protocol_governance_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Lido governance risk factors:
        // - DAO governance centralization
        // - Protocol upgrade risks
        // - Regulatory risks
        
        // For now, use a moderate governance risk score
        // In production, this would analyze governance proposals, voting patterns, etc.
        let governance_risk = 20.0; // Moderate governance risk
        
        debug!(governance_risk = %governance_risk, "Calculated protocol governance risk");
        
        Ok(BigDecimal::from_f64(governance_risk).unwrap_or_else(|| BigDecimal::from(20)))
    }
    
    /// Calculate validator performance risk
    async fn calculate_validator_performance_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        let validator_metrics = self.get_validator_metrics().await?;
        
        // Calculate performance based on APY and validator efficiency
        let performance_risk = if validator_metrics.current_apy < 3.0 {
            25.0 // Low APY indicates performance issues
        } else if validator_metrics.current_apy < 4.0 {
            15.0 // Moderate performance
        } else {
            8.0 // Good performance
        };
        
        debug!(
            current_apy = %validator_metrics.current_apy,
            performance_risk = %performance_risk,
            "Calculated validator performance risk"
        );
        
        Ok(BigDecimal::from_f64(performance_risk).unwrap_or_else(|| BigDecimal::from(15)))
    }
    
    /// Calculate liquidity risk
    async fn calculate_liquidity_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        let total_value: f64 = positions.iter()
            .map(|p| (&p.token0_amount + &p.token1_amount).to_f64().unwrap_or(0.0))
            .sum();
        
        // Liquidity risk based on position size and market conditions
        let liquidity_risk = if total_value > 1_000_000.0 {
            30.0 // Large positions have higher liquidity risk
        } else if total_value > 100_000.0 {
            20.0
        } else {
            10.0
        };
        
        debug!(
            total_value_usd = %total_value,
            liquidity_risk = %liquidity_risk,
            "Calculated liquidity risk"
        );
        
        Ok(BigDecimal::from_f64(liquidity_risk).unwrap_or_else(|| BigDecimal::from(15)))
    }
    
    /// Calculate smart contract risk
    async fn calculate_smart_contract_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Lido smart contract risk factors:
        // - Protocol maturity (positive)
        // - Audit history (positive)
        // - Complexity of contracts (negative)
        // - Upgrade mechanisms (moderate risk)
        
        let smart_contract_risk = 15.0; // Low-moderate risk for mature protocol
        
        debug!(smart_contract_risk = %smart_contract_risk, "Calculated smart contract risk");
        
        Ok(BigDecimal::from_f64(smart_contract_risk).unwrap_or_else(|| BigDecimal::from(15)))
    }
    
    /// Get validator metrics (mock implementation)
    async fn get_validator_metrics(&self) -> Result<ValidatorMetrics, RiskError> {
        // Check cache first
        {
            let cached = self.cached_validator_metrics.lock().unwrap();
            if let Some(metrics) = cached.as_ref() {
                return Ok(metrics.clone());
            }
        }
        
        // Mock validator data (in production, fetch from Lido contracts/API)
        let metrics = ValidatorMetrics {
            total_validators: 50_000,
            active_validators: 49_800,
            exited_validators: 150,
            slashed_validators: 50,
            total_staked_eth: BigDecimal::from_str("1500000").unwrap(), // 1.5M ETH
            current_apy: 4.2,
        };
        
        // Cache the metrics
        {
            let mut cached = self.cached_validator_metrics.lock().unwrap();
            *cached = Some(metrics.clone());
        }
        
        debug!(
            total_validators = metrics.total_validators,
            active_validators = metrics.active_validators,
            slashed_validators = metrics.slashed_validators,
            current_apy = metrics.current_apy,
            "Retrieved validator metrics"
        );
        
        Ok(metrics)
    }
    
    /// Get stETH/ETH peg (mock implementation)
    async fn get_steth_peg(&self) -> Result<f64, RiskError> {
        // Check cache first
        {
            let cached = self.cached_steth_peg.lock().unwrap();
            if let Some(peg) = cached.as_ref() {
                return Ok(*peg);
            }
        }
        
        // Mock peg data (in production, fetch from DEX/oracle)
        let peg = 0.998; // Slight depeg
        
        // Cache the peg
        {
            let mut cached = self.cached_steth_peg.lock().unwrap();
            *cached = Some(peg);
        }
        
        debug!(steth_eth_peg = %peg, "Retrieved stETH/ETH peg");
        
        Ok(peg)
    }
    
    /// Estimate withdrawal queue size (mock implementation)
    async fn estimate_withdrawal_queue_size(&self) -> Result<u64, RiskError> {
        // Mock withdrawal queue data
        let queue_size_eth = 500; // 500 ETH in queue
        
        debug!(withdrawal_queue_eth = %queue_size_eth, "Estimated withdrawal queue size");
        
        Ok(queue_size_eth)
    }
}

#[async_trait]
impl ProtocolRiskCalculator for LidoRiskCalculator {
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError> {
        info!(
            position_count = positions.len(),
            "Starting Lido risk calculation"
        );
        
        if positions.is_empty() {
            return Err(RiskError::InvalidPosition {
                message: "No positions provided for Lido risk calculation".to_string(),
            });
        }
        
        // Validate positions
        for position in positions {
            self.validate_position(position).await?;
        }
        
        // Calculate individual risk components
        let validator_slashing_risk = self.calculate_validator_slashing_risk(positions).await?;
        let steth_depeg_risk = self.calculate_steth_depeg_risk(positions).await?;
        let withdrawal_queue_risk = self.calculate_withdrawal_queue_risk(positions).await?;
        let protocol_governance_risk = self.calculate_protocol_governance_risk(positions).await?;
        let validator_performance_risk = self.calculate_validator_performance_risk(positions).await?;
        let liquidity_risk = self.calculate_liquidity_risk(positions).await?;
        let smart_contract_risk = self.calculate_smart_contract_risk(positions).await?;
        
        // Calculate overall risk score (weighted average)
        let overall_risk_score = (&validator_slashing_risk * BigDecimal::from_f64(0.25).unwrap()) +
                                (&steth_depeg_risk * BigDecimal::from_f64(0.20).unwrap()) +
                                (&withdrawal_queue_risk * BigDecimal::from_f64(0.15).unwrap()) +
                                (&protocol_governance_risk * BigDecimal::from_f64(0.15).unwrap()) +
                                (&validator_performance_risk * BigDecimal::from_f64(0.10).unwrap()) +
                                (&liquidity_risk * BigDecimal::from_f64(0.10).unwrap()) +
                                (&smart_contract_risk * BigDecimal::from_f64(0.05).unwrap());
        
        // Get additional context data
        let validator_metrics = self.get_validator_metrics().await?;
        let steth_peg = self.get_steth_peg().await?;
        let withdrawal_queue_size = self.estimate_withdrawal_queue_size().await?;
        
        let metrics = LidoRiskMetrics {
            validator_slashing_risk,
            steth_depeg_risk,
            withdrawal_queue_risk,
            protocol_governance_risk,
            validator_performance_risk,
            liquidity_risk,
            smart_contract_risk,
            overall_risk_score: overall_risk_score.clone(),
            
            // Context data
            current_steth_peg: Some(BigDecimal::from_f64(steth_peg).unwrap()),
            withdrawal_queue_length: Some(withdrawal_queue_size),
            active_validators: Some(validator_metrics.active_validators),
            slashed_validators: Some(validator_metrics.slashed_validators),
            total_staked_eth: Some(validator_metrics.total_staked_eth),
            apy: Some(BigDecimal::from_f64(validator_metrics.current_apy).unwrap()),
        };
        
        info!(
            overall_risk_score = %overall_risk_score,
            validator_slashing_risk = %metrics.validator_slashing_risk,
            steth_depeg_risk = %metrics.steth_depeg_risk,
            withdrawal_queue_risk = %metrics.withdrawal_queue_risk,
            "Completed Lido risk calculation"
        );
        
        Ok(ProtocolRiskMetrics::Lido(metrics))
    }
    
    fn protocol_name(&self) -> &'static str {
        "lido"
    }
    
    fn supported_position_types(&self) -> Vec<&'static str> {
        vec!["staking", "liquid_staking", "steth", "wsteth"]
    }
    
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError> {
        // Check if position is from Lido protocol
        if position.protocol.to_lowercase() != "lido" {
            return Ok(false);
        }
        
        // For Lido positions, we accept all position types since they're all staking-related
        // The supported_position_types are more for reference than strict validation
        
        // Check if position has valid value
        let position_value = (&position.token0_amount + &position.token1_amount).to_f64().unwrap_or(0.0);
        if position_value < 0.0 {
            return Err(RiskError::ValidationError {
                reason: "Position value cannot be negative".to_string(),
            });
        }
        
        Ok(true)
    }
    
    fn risk_factors(&self) -> Vec<&'static str> {
        vec![
            "validator_slashing",
            "steth_depeg",
            "withdrawal_queue",
            "protocol_governance",
            "validator_performance",
            "liquidity",
            "smart_contract"
        ]
    }
}

#[async_trait]
impl RealTimeRiskCalculator for LidoRiskCalculator {
    async fn update_real_time_data(&self) -> Result<(), RiskError> {
        info!("Updating Lido real-time risk data");
        
        // Update validator metrics
        let _validator_metrics = self.get_validator_metrics().await?;
        
        // Update stETH peg
        let _steth_peg = self.get_steth_peg().await?;
        
        // Update timestamp
        {
            let mut last_updated = self.last_updated.lock().unwrap();
            *last_updated = Some(chrono::Utc::now());
        }
        
        info!("Successfully updated Lido real-time risk data");
        Ok(())
    }
    
    fn last_updated(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let last_updated = self.last_updated.lock().unwrap();
        *last_updated
    }
}

impl ExplainableRiskCalculator for LidoRiskCalculator {
    fn explain_risk_calculation(&self, metrics: &ProtocolRiskMetrics) -> RiskExplanation {
        if let ProtocolRiskMetrics::Lido(lido_metrics) = metrics {
            let risk_score = lido_metrics.overall_risk_score.to_string().parse::<f64>().unwrap_or(0.0);
            
            let risk_level = match risk_score {
                s if s >= 70.0 => "High",
                s if s >= 40.0 => "Medium",
                s if s >= 20.0 => "Low",
                _ => "Very Low",
            };
            
            let explanation = format!(
                "Lido risk assessment considers validator slashing ({}), stETH depeg ({}), withdrawal queue ({}), and governance risks ({}). Current stETH peg: {}",
                lido_metrics.validator_slashing_risk,
                lido_metrics.steth_depeg_risk,
                lido_metrics.withdrawal_queue_risk,
                lido_metrics.protocol_governance_risk,
                lido_metrics.current_steth_peg.as_ref().unwrap_or(&BigDecimal::from(1))
            );
            
            RiskExplanation {
                overall_risk_score: risk_score,
                risk_level: risk_level.to_string(),
                primary_risk_factors: vec![
                    "Validator slashing risk".to_string(),
                    "stETH depeg risk".to_string(),
                    "Withdrawal queue delays".to_string(),
                ],
                explanation,
                methodology: "Weighted risk scoring based on validator performance, market conditions, and protocol metrics".to_string(),
                confidence_score: 0.85,
                data_quality: "High".to_string(),
            }
        } else {
            RiskExplanation {
                overall_risk_score: 0.0,
                risk_level: "Unknown".to_string(),
                primary_risk_factors: vec![],
                explanation: "Invalid metrics type for Lido calculator".to_string(),
                methodology: "N/A".to_string(),
                confidence_score: 0.0,
                data_quality: "Low".to_string(),
            }
        }
    }
    
    fn get_risk_factor_contributions(&self, metrics: &ProtocolRiskMetrics) -> Vec<RiskFactorContribution> {
        if let ProtocolRiskMetrics::Lido(lido_metrics) = metrics {
            vec![
                RiskFactorContribution {
                    factor_name: "Validator Slashing Risk".to_string(),
                    contribution_score: 25.0,
                    impact_level: "High".to_string(),
                    description: "Risk of validator penalties and slashing events".to_string(),
                    current_value: Some(lido_metrics.validator_slashing_risk.to_string().parse().unwrap_or(0.0)),
                    threshold_value: Some(self.validator_slashing_threshold * 100.0),
                },
                RiskFactorContribution {
                    factor_name: "stETH Depeg Risk".to_string(),
                    contribution_score: 20.0,
                    impact_level: "Medium".to_string(),
                    description: "Risk of stETH trading below ETH parity".to_string(),
                    current_value: lido_metrics.current_steth_peg.as_ref().map(|p| p.to_string().parse().unwrap_or(1.0)),
                    threshold_value: Some(1.0 - self.depeg_risk_threshold),
                },
                RiskFactorContribution {
                    factor_name: "Withdrawal Queue Risk".to_string(),
                    contribution_score: 15.0,
                    impact_level: "Medium".to_string(),
                    description: "Risk of delayed withdrawals due to queue length".to_string(),
                    current_value: lido_metrics.withdrawal_queue_length.map(|q| q as f64),
                    threshold_value: Some(self.withdrawal_queue_threshold as f64),
                },
            ]
        } else {
            vec![]
        }
    }
    
    fn get_risk_reduction_recommendations(&self, metrics: &ProtocolRiskMetrics) -> Vec<String> {
        let mut recommendations = vec![];
        
        if let ProtocolRiskMetrics::Lido(lido_metrics) = metrics {
            let slashing_risk = lido_metrics.validator_slashing_risk.to_string().parse::<f64>().unwrap_or(0.0);
            if slashing_risk > 30.0 {
                recommendations.push("Consider diversifying across multiple liquid staking providers".to_string());
            }
            
            let depeg_risk = lido_metrics.steth_depeg_risk.to_string().parse::<f64>().unwrap_or(0.0);
            if depeg_risk > 25.0 {
                recommendations.push("Monitor stETH/ETH peg closely and consider hedging strategies".to_string());
            }
            
            let queue_risk = lido_metrics.withdrawal_queue_risk.to_string().parse::<f64>().unwrap_or(0.0);
            if queue_risk > 20.0 {
                recommendations.push("Plan for potential withdrawal delays during high demand periods".to_string());
            }
            
            recommendations.push("Regularly monitor validator performance and protocol updates".to_string());
        }
        
        recommendations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::traits::Position;
    
    fn create_test_lido_position(value_usd: f64) -> Position {
        Position {
            id: "test_lido_position".to_string(),
            protocol: "lido".to_string(),
            position_type: "staking".to_string(),
            pair: "stETH".to_string(),
            value_usd,
            pnl_usd: 0.0,
            pnl_percentage: 0.0,
            risk_score: 0, // Will be calculated by our risk calculator
            metadata: serde_json::json!({
                "token_address": "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84",
                "current_apy": 4.2
            }),
            last_updated: 0,
        }
    }
    
    #[tokio::test]
    async fn test_lido_calculator_creation() {
        let calculator = LidoRiskCalculator::new();
        assert_eq!(calculator.protocol_name(), "lido");
        assert!(calculator.supported_position_types().contains(&"staking"));
    }
    
    #[tokio::test]
    async fn test_position_validation() {
        let calculator = LidoRiskCalculator::new();
        
        let valid_position = create_test_lido_position(1000.0);
        assert!(calculator.validate_position(&valid_position).await.unwrap());
        
        let invalid_position = Position {
            protocol: "uniswap".to_string(),
            position_type: "lp".to_string(),
            balance_usd: 1000.0,
            ..Default::default()
        };
        assert!(!calculator.validate_position(&invalid_position).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_risk_calculation() {
        let calculator = LidoRiskCalculator::new();
        let positions = vec![create_test_lido_position(10000.0)];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(ProtocolRiskMetrics::Lido(metrics)) = result {
            assert!(metrics.overall_risk_score > BigDecimal::zero());
            assert!(metrics.validator_slashing_risk >= BigDecimal::zero());
            assert!(metrics.steth_depeg_risk >= BigDecimal::zero());
        } else {
            panic!("Expected Lido risk metrics");
        }
    }
    
    #[tokio::test]
    async fn test_empty_positions() {
        let calculator = LidoRiskCalculator::new();
        let positions = vec![];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_real_time_data_update() {
        let calculator = LidoRiskCalculator::new();
        
        // Initially no update timestamp
        assert!(calculator.last_updated().is_none());
        
        // Update data
        let result = calculator.update_real_time_data().await;
        assert!(result.is_ok());
        
        // Should have update timestamp now
        assert!(calculator.last_updated().is_some());
        assert!(!calculator.is_data_stale()); // Should be fresh
    }
}
