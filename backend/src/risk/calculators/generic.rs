// Generic Risk Calculator for protocols without specific implementations
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use num_traits::{Zero, ToPrimitive, FromPrimitive};
use tracing::{info, debug};

use crate::models::Position;
use crate::risk::{
    RiskError, 
    ProtocolRiskCalculator, 
    ProtocolRiskMetrics, 
    GenericRiskMetrics,
};

/// Generic risk calculator for protocols without specific implementations
pub struct GenericRiskCalculator {
    protocol_name: String,
    base_risk_score: f64,
}

impl GenericRiskCalculator {
    /// Create a new generic risk calculator for a protocol
    pub fn new(protocol_name: String) -> Self {
        Self {
            protocol_name,
            base_risk_score: 40.0, // Default moderate risk
        }
    }
    
    /// Create with custom base risk score
    pub fn with_base_risk(protocol_name: String, base_risk_score: f64) -> Self {
        Self {
            protocol_name,
            base_risk_score: base_risk_score.max(0.0).min(100.0),
        }
    }
    
    /// Calculate protocol risk based on general factors
    async fn calculate_protocol_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Base protocol risk depends on protocol maturity and recognition
        let protocol_risk = match self.protocol_name.to_lowercase().as_str() {
            // Well-established protocols
            "compound" | "yearn" | "curve" | "balancer" => 25.0,
            // Moderately established protocols
            "convex" | "frax" | "rocket_pool" => 35.0,
            // Newer or less established protocols
            _ => 50.0,
        };
        
        debug!(
            protocol = %self.protocol_name,
            protocol_risk = %protocol_risk,
            "Calculated generic protocol risk"
        );
        
        Ok(BigDecimal::from_f64(protocol_risk).unwrap_or_else(|| BigDecimal::from(40)))
    }
    
    /// Calculate smart contract risk
    async fn calculate_smart_contract_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Generic smart contract risk assessment
        let contract_risk = match self.protocol_name.to_lowercase().as_str() {
            // Well-audited, mature protocols
            "compound" | "aave" | "uniswap" | "curve" => 20.0,
            // Moderately audited protocols
            "yearn" | "balancer" | "convex" => 30.0,
            // Less established or newer protocols
            _ => 45.0,
        };
        
        debug!(
            protocol = %self.protocol_name,
            contract_risk = %contract_risk,
            "Calculated generic smart contract risk"
        );
        
        Ok(BigDecimal::from_f64(contract_risk).unwrap_or_else(|| BigDecimal::from(35)))
    }
    
    /// Calculate liquidity risk
    async fn calculate_liquidity_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Calculate total value from token amounts (simplified)
        let total_value: f64 = positions.iter()
            .map(|p| (&p.token0_amount + &p.token1_amount).to_f64().unwrap_or(0.0))
            .sum();
        
        // Liquidity risk based on position size and protocol type
        let base_liquidity_risk: f64 = if total_value > 1_000_000.0 {
            35.0 // Large positions have higher liquidity risk
        } else if total_value > 100_000.0 {
            25.0
        } else {
            15.0
        };
        
        // Adjust based on protocol type
        let protocol_adjustment: f64 = match self.protocol_name.to_lowercase().as_str() {
            // DEX protocols typically have good liquidity
            "uniswap" | "curve" | "balancer" => -5.0,
            // Lending protocols have moderate liquidity
            "compound" | "aave" => 0.0,
            // Yield farming protocols may have liquidity constraints
            "yearn" | "convex" | "beefy" => 5.0,
            // Unknown protocols get penalty
            _ => 10.0,
        };
        
        let liquidity_risk = (base_liquidity_risk + protocol_adjustment).max(5.0f64).min(60.0f64);
        
        debug!(
            protocol = %self.protocol_name,
            total_value_usd = %total_value,
            base_risk = %base_liquidity_risk,
            protocol_adjustment = %protocol_adjustment,
            liquidity_risk = %liquidity_risk,
            "Calculated generic liquidity risk"
        );
        
        Ok(BigDecimal::from_f64(liquidity_risk).unwrap_or_else(|| BigDecimal::from(25)))
    }
    
    /// Calculate governance risk
    async fn calculate_governance_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Governance risk based on protocol governance maturity
        let governance_risk = match self.protocol_name.to_lowercase().as_str() {
            // Mature governance systems
            "compound" | "aave" | "uniswap" => 20.0,
            // Developing governance
            "yearn" | "curve" | "balancer" => 30.0,
            // Centralized or immature governance
            _ => 40.0,
        };
        
        debug!(
            protocol = %self.protocol_name,
            governance_risk = %governance_risk,
            "Calculated generic governance risk"
        );
        
        Ok(BigDecimal::from_f64(governance_risk).unwrap_or_else(|| BigDecimal::from(30)))
    }
    
    /// Calculate market risk
    async fn calculate_market_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        let position_count = positions.len();
        
        // Market risk based on diversification and protocol exposure
        let market_risk = if position_count == 1 {
            40.0 // Single position = higher market risk
        } else if position_count <= 3 {
            30.0 // Few positions = moderate market risk
        } else {
            20.0 // Multiple positions = lower market risk
        };
        
        debug!(
            protocol = %self.protocol_name,
            position_count = position_count,
            market_risk = %market_risk,
            "Calculated generic market risk"
        );
        
        Ok(BigDecimal::from_f64(market_risk).unwrap_or_else(|| BigDecimal::from(30)))
    }
    
    /// Estimate protocol age for risk assessment
    fn estimate_protocol_age_days(&self) -> Option<u64> {
        // Rough estimates for protocol ages (in days)
        match self.protocol_name.to_lowercase().as_str() {
            "compound" => Some(1800), // ~5 years
            "uniswap" => Some(1500), // ~4 years
            "aave" => Some(1200), // ~3.3 years
            "curve" => Some(1100), // ~3 years
            "yearn" => Some(1000), // ~2.7 years
            "balancer" => Some(900), // ~2.5 years
            "convex" => Some(700), // ~2 years
            _ => None, // Unknown age
        }
    }
    
    /// Get estimated TVL for risk assessment
    fn estimate_protocol_tvl(&self) -> Option<BigDecimal> {
        // Rough TVL estimates (in USD)
        let tvl_usd = match self.protocol_name.to_lowercase().as_str() {
            "aave" => 10_000_000_000.0, // $10B
            "compound" => 3_000_000_000.0, // $3B
            "curve" => 4_000_000_000.0, // $4B
            "uniswap" => 5_000_000_000.0, // $5B
            "yearn" => 500_000_000.0, // $500M
            "balancer" => 1_000_000_000.0, // $1B
            "convex" => 2_000_000_000.0, // $2B
            _ => return None,
        };
        
        Some(BigDecimal::from_f64(tvl_usd).unwrap_or_else(|| BigDecimal::from(1_000_000_000)))
    }
}

#[async_trait]
impl ProtocolRiskCalculator for GenericRiskCalculator {
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError> {
        info!(
            protocol = %self.protocol_name,
            position_count = positions.len(),
            "Starting generic risk calculation"
        );
        
        if positions.is_empty() {
            return Err(RiskError::InvalidPosition {
                message: format!("No positions provided for {} risk calculation", self.protocol_name),
            });
        }
        
        // Validate positions
        for position in positions {
            self.validate_position(position).await?;
        }
        
        // Calculate individual risk components
        let protocol_risk = self.calculate_protocol_risk(positions).await?;
        let smart_contract_risk = self.calculate_smart_contract_risk(positions).await?;
        let liquidity_risk = self.calculate_liquidity_risk(positions).await?;
        let governance_risk = self.calculate_governance_risk(positions).await?;
        let market_risk = self.calculate_market_risk(positions).await?;
        
        // Calculate overall risk score (weighted average)
        let overall_risk_score = (&protocol_risk * BigDecimal::from_f64(0.25).unwrap()) +
                                (&smart_contract_risk * BigDecimal::from_f64(0.25).unwrap()) +
                                (&liquidity_risk * BigDecimal::from_f64(0.20).unwrap()) +
                                (&governance_risk * BigDecimal::from_f64(0.15).unwrap()) +
                                (&market_risk * BigDecimal::from_f64(0.15).unwrap());
        
        let metrics = GenericRiskMetrics {
            protocol_risk,
            smart_contract_risk,
            liquidity_risk,
            governance_risk,
            market_risk,
            overall_risk_score: overall_risk_score.clone(),
            
            // Context data
            protocol_name: self.protocol_name.clone(),
            tvl: self.estimate_protocol_tvl(),
            age_days: self.estimate_protocol_age_days(),
            audit_status: Some("Unknown".to_string()), // Would need external data
        };
        
        info!(
            protocol = %self.protocol_name,
            overall_risk_score = %overall_risk_score,
            protocol_risk = %metrics.protocol_risk,
            smart_contract_risk = %metrics.smart_contract_risk,
            liquidity_risk = %metrics.liquidity_risk,
            "Completed generic risk calculation"
        );
        
        Ok(ProtocolRiskMetrics::Generic(metrics))
    }
    
    fn protocol_name(&self) -> &'static str {
        // This is a bit tricky since we need a static str but have a String
        // In practice, you'd use a different approach or store static protocol names
        "generic"
    }
    
    fn supported_position_types(&self) -> Vec<&'static str> {
        vec!["any", "generic", "unknown"]
    }
    
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError> {
        // Generic validation - just check basic requirements
        let position_value = (&position.token0_amount + &position.token1_amount).to_f64().unwrap_or(0.0);
        if position_value < 0.0 {
            return Err(RiskError::ValidationError {
                reason: "Position balance cannot be negative".to_string(),
            });
        }
        
        if position.protocol.is_empty() {
            return Err(RiskError::ValidationError {
                reason: "Position must have a protocol specified".to_string(),
            });
        }
        
        Ok(true)
    }
    
    fn risk_factors(&self) -> Vec<&'static str> {
        vec![
            "protocol_maturity",
            "smart_contract_security",
            "liquidity_depth",
            "governance_decentralization",
            "market_conditions"
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Position;
    
    fn create_test_position(protocol: &str, balance_usd: f64) -> Position {
        Position {
            protocol: protocol.to_string(),
            position_type: "generic".to_string(),
            balance_usd,
            ..Default::default()
        }
    }
    
    #[tokio::test]
    async fn test_generic_calculator_creation() {
        let calculator = GenericRiskCalculator::new("test_protocol".to_string());
        assert_eq!(calculator.protocol_name(), "generic");
        assert!(calculator.supported_position_types().contains(&"any"));
    }
    
    #[tokio::test]
    async fn test_established_protocol_risk() {
        let calculator = GenericRiskCalculator::new("compound".to_string());
        let positions = vec![create_test_position("compound", 1000.0)];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(ProtocolRiskMetrics::Generic(metrics)) = result {
            // Established protocols should have lower risk
            let risk_score = metrics.overall_risk_score.to_string().parse::<f64>().unwrap();
            assert!(risk_score < 40.0); // Should be lower than default
        }
    }
    
    #[tokio::test]
    async fn test_unknown_protocol_risk() {
        let calculator = GenericRiskCalculator::new("unknown_protocol".to_string());
        let positions = vec![create_test_position("unknown_protocol", 1000.0)];
        
        let result = calculator.calculate_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(ProtocolRiskMetrics::Generic(metrics)) = result {
            // Unknown protocols should have higher risk
            let risk_score = metrics.overall_risk_score.to_string().parse::<f64>().unwrap();
            assert!(risk_score > 35.0); // Should be higher risk
        }
    }
    
    #[tokio::test]
    async fn test_position_validation() {
        let calculator = GenericRiskCalculator::new("test".to_string());
        
        let valid_position = create_test_position("test", 1000.0);
        assert!(calculator.validate_position(&valid_position).await.unwrap());
        
        let invalid_position = Position {
            protocol: "test".to_string(),
            balance_usd: -100.0, // Negative balance
            ..Default::default()
        };
        assert!(calculator.validate_position(&invalid_position).await.is_err());
    }
    
    #[tokio::test]
    async fn test_liquidity_risk_scaling() {
        let calculator = GenericRiskCalculator::new("test".to_string());
        
        // Small position
        let small_positions = vec![create_test_position("test", 1000.0)];
        let small_risk = calculator.calculate_liquidity_risk(&small_positions).await.unwrap();
        
        // Large position
        let large_positions = vec![create_test_position("test", 2_000_000.0)];
        let large_risk = calculator.calculate_liquidity_risk(&large_positions).await.unwrap();
        
        // Large positions should have higher liquidity risk
        assert!(large_risk > small_risk);
    }
}
