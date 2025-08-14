use async_trait::async_trait;
use bigdecimal::BigDecimal;
use num_traits::{FromPrimitive, Zero};
use serde_json::Value;
use std::collections::HashMap;

use crate::models::Position;
use crate::risk::traits::ProtocolRiskCalculator;
use crate::risk::metrics::{ProtocolRiskMetrics, BeefyRiskMetrics};
use crate::risk::errors::RiskError;

/// Beefy Finance risk calculator
/// Focuses on yield farming risks, strategy complexity, and underlying protocol risks
#[derive(Debug, Clone)]
pub struct BeefyRiskCalculator {
    config: BeefyRiskConfig,
}

#[derive(Debug, Clone)]
pub struct BeefyRiskConfig {
    pub high_apy_threshold: f64,
    pub very_high_apy_threshold: f64,
    pub large_position_threshold: f64,
    pub small_position_threshold: f64,
    pub diversification_bonus: u8,
}

impl Default for BeefyRiskConfig {
    fn default() -> Self {
        Self {
            high_apy_threshold: 100.0,
            very_high_apy_threshold: 200.0,
            large_position_threshold: 50_000.0,
            small_position_threshold: 100.0,
            diversification_bonus: 3,
        }
    }
}

impl BeefyRiskCalculator {
    pub fn new() -> Self {
        Self {
            config: BeefyRiskConfig::default(),
        }
    }

    pub fn with_config(config: BeefyRiskConfig) -> Self {
        Self { config }
    }

    /// Calculate yield farming strategy risk
    fn calculate_strategy_risk(&self, position: &Position) -> Result<f64, RiskError> {
        let mut risk_score = 30.0; // Base yield farming risk

        // Strategy complexity risk (infer from protocol name)
        let strategy_complexity = match position.protocol.to_lowercase().as_str() {
            "beefy" => 25, // Standard yield farming
            "convex" => 35, // More complex strategies
            "yearn" => 40, // Advanced strategies
            _ => 30, // Default
        };

        // Underlying protocol risk (estimate based on fee tier and protocol)
        let protocol_risk = match position.fee_tier {
            100 => 15,  // Low fee tier, likely stable
            500 => 25,  // Medium fee tier
            3000 => 35, // High fee tier, more volatile
            _ => 30,    // Default
        };

        risk_score += strategy_complexity as f64;
        risk_score += protocol_risk as f64;

        Ok(risk_score)
    }

    /// Calculate APY sustainability risk
    fn calculate_apy_risk(&self, position: &Position) -> Result<f64, RiskError> {
        // Use fee_tier as a proxy for APY estimation (higher fee = higher risk/reward)
        let apy = match position.fee_tier {
            100 => BigDecimal::from(5),   // Low fee, stable pools ~5% APY
            500 => BigDecimal::from(12),  // Medium fee ~12% APY
            3000 => BigDecimal::from(25), // High fee ~25% APY
            _ => BigDecimal::from(15),    // Default ~15% APY
        };
        
        let apy_f64 = apy.to_string().parse::<f64>().unwrap_or(15.0);
        let risk_score = if apy_f64 > self.config.very_high_apy_threshold {
            25.0 // Extremely high APY is very risky
        } else if apy_f64 > self.config.high_apy_threshold {
            15.0 // Very high APY
        } else if apy_f64 < 1.0 {
            10.0 // Very low APY might indicate problems
        } else if apy_f64 > 50.0 {
            8.0 // Moderately high APY
        } else {
            0.0 // Normal APY range
        };

        Ok(risk_score)
    }

    /// Calculate position size risk
    fn calculate_position_size_risk(&self, position: &Position) -> Result<f64, RiskError> {
        // Use token amounts to estimate position value
        let token0_amount = position.token0_amount.to_string().parse::<f64>().unwrap_or(0.0);
        let token1_amount = position.token1_amount.to_string().parse::<f64>().unwrap_or(0.0);
        let value_f64 = (token0_amount + token1_amount) / 1e18 * 3000.0; // Estimate USD value
        
        let risk_adjustment = if value_f64 > self.config.large_position_threshold {
            -2.0 // Large positions get slight discount (more stable)
        } else if value_f64 < self.config.small_position_threshold {
            5.0 // Very small positions have higher relative gas cost risk
        } else {
            0.0
        };

        Ok(risk_adjustment)
    }

    /// Calculate diversification bonus
    fn calculate_diversification_risk(&self, position: &Position) -> Result<f64, RiskError> {
        // Estimate diversification based on token addresses (if both exist, it's likely a pair)
        let asset_count = if !position.token1_address.is_empty() { 2 } else { 1 }; // Check if token1 exists
        
        if asset_count > 2 {
            Ok(-(self.config.diversification_bonus as f64)) // Diversified assets reduce risk
        } else {
            Ok(0.0)
        }
    }

    /// Calculate smart contract risk
    fn calculate_smart_contract_risk(&self, position: &Position) -> Result<f64, RiskError> {
        let mut risk_score = 15.0; // Base smart contract risk

        // Estimate vault age based on protocol maturity (fallback approach)
        let protocol_age_risk = match position.protocol.to_lowercase().as_str() {
            "beefy" => -3.0,    // Mature protocol
            "yearn" => -3.0,    // Mature protocol
            "convex" => 0.0,    // Established protocol
            _ => 5.0,           // Unknown/newer protocols
        };
        risk_score += protocol_age_risk;

        // Estimate TVL risk based on fee tier (higher fee tiers often have lower TVL)
        let tvl_risk = match position.fee_tier {
            100 => -5.0,   // Low fee tier, likely high TVL stable pools
            500 => 0.0,    // Medium fee tier
            3000 => 8.0,   // High fee tier, likely lower TVL
            _ => 5.0,      // Default moderate risk
        };
        risk_score += tvl_risk;

        Ok(risk_score)
    }

    /// Calculate liquidity risk
    fn calculate_liquidity_risk(&self, position: &Position) -> Result<f64, RiskError> {
        let mut risk_score = 10.0; // Base liquidity risk

        // Estimate withdrawal delay based on protocol type
        let withdrawal_risk = match position.protocol.to_lowercase().as_str() {
            "beefy" => 8.0,     // Standard withdrawal delays
            "yearn" => 5.0,     // Generally good liquidity
            "convex" => 10.0,   // May have longer delays
            _ => 12.0,          // Unknown protocols assumed higher risk
        };
        risk_score += withdrawal_risk;

        // Estimate liquidity based on fee tier (lower fees often mean better liquidity)
        let liquidity_risk = match position.fee_tier {
            100 => 0.0,    // Low fee tier, excellent liquidity
            500 => 5.0,    // Medium fee tier, good liquidity
            3000 => 15.0,  // High fee tier, potentially poor liquidity
            _ => 10.0,     // Default moderate liquidity risk
        };
        risk_score += liquidity_risk;

        Ok(risk_score)
    }

    /// Calculate governance risk
    fn calculate_governance_risk(&self, position: &Position) -> Result<f64, RiskError> {
        let mut risk_score = 8.0; // Base governance risk

        // Estimate governance risk based on protocol maturity and reputation
        let protocol_governance_risk = match position.protocol.to_lowercase().as_str() {
            "beefy" => 0.0,     // Well-established governance
            "yearn" => 0.0,     // Strong governance model
            "convex" => 5.0,    // Moderate governance risk
            _ => 15.0,          // Unknown protocols assumed higher governance risk
        };
        risk_score += protocol_governance_risk;

        // Estimate timelock/multisig risk based on protocol type
        let security_risk = match position.protocol.to_lowercase().as_str() {
            "beefy" => 0.0,     // Generally has good security practices
            "yearn" => 0.0,     // Strong security model
            _ => 8.0,           // Default moderate security risk
        };
        risk_score += security_risk;

        Ok(risk_score)
    }

    /// Validate Beefy-specific position data
    fn validate_beefy_position(&self, position: &Position) -> Result<bool, RiskError> {
        // Check if position is for Beefy protocol
        if position.protocol.to_lowercase() != "beefy" {
            return Ok(false);
        }

        // Basic validation - check if position has required fields
        if position.pool_address.is_empty() {
            return Err(RiskError::InvalidPosition {
                message: "Missing pool address for Beefy position".to_string(),
            });
        }

        Ok(true)
    }
}

#[async_trait]
impl ProtocolRiskCalculator for BeefyRiskCalculator {
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError> {
        if positions.is_empty() {
            return Ok(ProtocolRiskMetrics::Beefy(BeefyRiskMetrics {
                vault_strategy_risk: BigDecimal::from(0),
                underlying_protocol_risk: BigDecimal::from(0),
                smart_contract_risk: BigDecimal::from(0),
                liquidity_risk: BigDecimal::from(0),
                governance_risk: BigDecimal::from(0),
                overall_risk_score: BigDecimal::from(0),
                vault_tvl: None,
                strategy_count: None,
                apy: None,
                fees: None,
            }));
        }

        let mut total_risk = 0.0;
        let mut total_weight = 0.0;
        let mut risk_factors = HashMap::new();
        let mut position_risks = Vec::new();

        for position in positions {
            // Validate position
            if !self.validate_beefy_position(position)? {
                continue;
            }

            // Calculate position weight from token amounts
            let token0_amount = position.token0_amount.to_string().parse::<f64>().unwrap_or(0.0);
            let token1_amount = position.token1_amount.to_string().parse::<f64>().unwrap_or(0.0);
            let position_weight = (token0_amount + token1_amount) / 1e18 * 3000.0; // Estimate USD value
            
            // Calculate individual risk components
            let strategy_risk = self.calculate_strategy_risk(position)?;
            let apy_risk = self.calculate_apy_risk(position)?;
            let position_size_risk = self.calculate_position_size_risk(position)?;
            let diversification_risk = self.calculate_diversification_risk(position)?;
            let smart_contract_risk = self.calculate_smart_contract_risk(position)?;
            let liquidity_risk = self.calculate_liquidity_risk(position)?;
            let governance_risk = self.calculate_governance_risk(position)?;

            // Calculate total position risk
            let position_risk = (strategy_risk + apy_risk + position_size_risk + 
                               diversification_risk + smart_contract_risk + 
                               liquidity_risk + governance_risk).max(0.0).min(98.0);

            // Weight by position value
            total_risk += position_risk * position_weight;
            total_weight += position_weight;

            // Store individual position risk
            position_risks.push((position.id.clone(), position_risk));

            // Aggregate risk factors
            *risk_factors.entry("strategy_risk".to_string()).or_insert(0.0) += strategy_risk * position_weight;
            *risk_factors.entry("apy_risk".to_string()).or_insert(0.0) += apy_risk * position_weight;
            *risk_factors.entry("smart_contract_risk".to_string()).or_insert(0.0) += smart_contract_risk * position_weight;
            *risk_factors.entry("liquidity_risk".to_string()).or_insert(0.0) += liquidity_risk * position_weight;
            *risk_factors.entry("governance_risk".to_string()).or_insert(0.0) += governance_risk * position_weight;
        }

        // Calculate weighted average risk
        let overall_risk = if total_weight > 0.0 {
            (total_risk / total_weight).min(98.0)
        } else {
            35.0 // Default Beefy yield farming risk
        };

        // Normalize risk factors
        if total_weight > 0.0 {
            for (_, value) in risk_factors.iter_mut() {
                *value /= total_weight;
            }
        }

        // Calculate strategy complexity and underlying risks from available data
        let strategy_complexity = risk_factors.get("strategy_complexity").cloned().unwrap_or(25.0);
        let underlying_risks = risk_factors.get("underlying_protocol_risk").cloned().unwrap_or(20.0);
        
        Ok(ProtocolRiskMetrics::Beefy(BeefyRiskMetrics {
            vault_strategy_risk: BigDecimal::from_f64(strategy_complexity).unwrap_or(BigDecimal::zero()),
            underlying_protocol_risk: BigDecimal::from_f64(underlying_risks).unwrap_or(BigDecimal::zero()),
            smart_contract_risk: BigDecimal::from_f64(20.0).unwrap_or(BigDecimal::zero()),
            liquidity_risk: BigDecimal::from_f64(15.0).unwrap_or(BigDecimal::zero()),
            governance_risk: BigDecimal::from_f64(10.0).unwrap_or(BigDecimal::zero()),
            overall_risk_score: BigDecimal::from_f64(overall_risk).unwrap_or(BigDecimal::zero()),
            vault_tvl: Some(BigDecimal::from_f64(total_weight).unwrap_or(BigDecimal::zero())),
            strategy_count: None,
            apy: None,
            fees: None,
        }))
    }

    fn protocol_name(&self) -> &'static str {
        "beefy"
    }

    fn supported_position_types(&self) -> Vec<&'static str> {
        vec!["vault", "farm", "pool"]
    }

    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError> {
        self.validate_beefy_position(position)
    }

    fn risk_factors(&self) -> Vec<&'static str> {
        vec![
            "strategy_risk",
            "apy_risk", 
            "smart_contract_risk",
            "liquidity_risk",
            "governance_risk",
            "position_size_risk",
            "diversification_risk"
        ]
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_position(value_usd: f64, apy: f64, metadata: Value) -> Position {
        Position {
            id: "test_position".to_string(),
            protocol: "beefy".to_string(),
            position_type: "vault".to_string(),
            pair: "USDC-USDT".to_string(),
            value_usd,
            pnl_usd: value_usd * (apy / 100.0),
            pnl_percentage: apy,
            risk_score: 50,
            metadata,
            last_updated: chrono::Utc::now().timestamp() as u64,
        }
    }

    #[tokio::test]
    async fn test_beefy_risk_calculation() {
        let calculator = BeefyRiskCalculator::new();
        
        let position = create_test_position(
            10000.0,
            25.0,
            json!({
                "vault_id": "beefy-usdc-usdt-vault",
                "strategy_type": "lp_farming",
                "underlying_assets": ["USDC", "USDT"],
                "vault_tvl_usd": 5000000.0,
                "vault_age_days": 180
            })
        );

        let result = calculator.calculate_risk(&[position]).await.unwrap();
        
        assert_eq!(result.protocol, "beefy");
        assert!(result.overall_risk_score > 0.0);
        assert!(result.overall_risk_score <= 98.0);
        assert!(result.risk_factors.contains_key("strategy_risk"));
        assert!(result.risk_factors.contains_key("apy_risk"));
        assert_eq!(result.confidence_score, 0.85);
    }

    #[tokio::test]
    async fn test_high_apy_risk() {
        let calculator = BeefyRiskCalculator::new();
        
        let high_apy_position = create_test_position(
            5000.0,
            250.0, // Very high APY
            json!({
                "vault_id": "high-yield-vault",
                "strategy_type": "leveraged"
            })
        );

        let normal_apy_position = create_test_position(
            5000.0,
            15.0, // Normal APY
            json!({
                "vault_id": "stable-vault",
                "strategy_type": "single_asset"
            })
        );

        let high_apy_result = calculator.calculate_risk(&[high_apy_position]).await.unwrap();
        let normal_apy_result = calculator.calculate_risk(&[normal_apy_position]).await.unwrap();
        
        assert!(high_apy_result.overall_risk_score > normal_apy_result.overall_risk_score);
    }

    #[tokio::test]
    async fn test_position_validation() {
        let calculator = BeefyRiskCalculator::new();
        
        let valid_position = create_test_position(
            1000.0,
            20.0,
            json!({
                "vault_id": "valid-vault"
            })
        );

        let invalid_position = Position {
            protocol: "uniswap".to_string(), // Wrong protocol
            ..create_test_position(1000.0, 20.0, json!({}))
        };

        assert!(calculator.validate_position(&valid_position).await.unwrap());
        assert!(!calculator.validate_position(&invalid_position).await.unwrap());
    }

    #[tokio::test]
    async fn test_empty_positions() {
        let calculator = BeefyRiskCalculator::new();
        let result = calculator.calculate_risk(&[]).await.unwrap();
        
        assert_eq!(result.overall_risk_score, 0.0);
        assert_eq!(result.total_value_at_risk, 0.0);
    }
}
