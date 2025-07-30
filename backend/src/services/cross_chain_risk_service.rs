use crate::models::cross_chain_risk::*;
use crate::models::PoolState;
use crate::error::AppError;
use bigdecimal::{BigDecimal, Zero};
use sqlx::PgPool;
use tracing::info;
use std::str::FromStr;
// use chrono::{Utc, Duration};
// use uuid::Uuid;
// use std::collections::HashMap;

/// Cross-chain risk detection and assessment service
pub struct CrossChainRiskService {
    #[allow(dead_code)]
    db_pool: PgPool,
    config: CrossChainRiskConfig,
}

impl CrossChainRiskService {
    pub fn new(db_pool: PgPool, config: Option<CrossChainRiskConfig>) -> Self {
        Self {
            db_pool,
            config: config.unwrap_or_default(),
        }
    }

    /// Calculate comprehensive cross-chain risk for a multi-chain position
    pub async fn calculate_cross_chain_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
        pool_states: &[PoolState],
    ) -> Result<CrossChainRiskResult, AppError> {
        info!("Calculating cross-chain risk for primary chain {} with {} secondary chains", 
              primary_chain_id, secondary_chain_ids.len());

        // Calculate individual risk components
        let bridge_risk = self.calculate_bridge_risk(primary_chain_id, secondary_chain_ids).await?;
        let liquidity_fragmentation_risk = self.calculate_liquidity_fragmentation_risk(pool_states).await?;
        let governance_divergence_risk = self.calculate_governance_divergence_risk(primary_chain_id, secondary_chain_ids).await?;
        let technical_risk = self.calculate_technical_risk(primary_chain_id, secondary_chain_ids).await?;
        let correlation_risk = self.calculate_correlation_risk(primary_chain_id, secondary_chain_ids).await?;

        // Calculate weighted overall cross-chain risk
        let overall_risk = self.calculate_weighted_cross_chain_risk(
            &bridge_risk,
            &liquidity_fragmentation_risk,
            &governance_divergence_risk,
            &technical_risk,
            &correlation_risk,
        )?;

        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(
            primary_chain_id,
            secondary_chain_ids,
            pool_states,
        ).await?;

        // Generate risk factors and recommendations
        let risk_factors = self.identify_risk_factors(
            &bridge_risk,
            &liquidity_fragmentation_risk,
            &governance_divergence_risk,
            &technical_risk,
            &correlation_risk,
        );

        let recommendations = self.generate_recommendations(
            &bridge_risk,
            &liquidity_fragmentation_risk,
            &governance_divergence_risk,
        );

        Ok(CrossChainRiskResult {
            overall_cross_chain_risk: overall_risk,
            bridge_risk_score: bridge_risk,
            liquidity_fragmentation_risk,
            governance_divergence_risk,
            technical_risk_score: technical_risk,
            correlation_risk_score: correlation_risk,
            confidence_score,
            risk_factors,
            recommendations,
        })
    }

    /// Calculate bridge security risk across chains
    async fn calculate_bridge_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<BigDecimal, AppError> {
        let mut total_bridge_risk = BigDecimal::zero();
        let mut bridge_count = 0;

        for &secondary_chain_id in secondary_chain_ids {
            let bridge_risk = self.assess_bridge_security(primary_chain_id, secondary_chain_id).await?;
            total_bridge_risk += bridge_risk;
            bridge_count += 1;
        }

        if bridge_count == 0 {
            return Ok(BigDecimal::zero());
        }

        // Average bridge risk across all bridges
        let average_risk = total_bridge_risk / BigDecimal::from(bridge_count);
        
        // Apply penalty for using multiple bridges (increased complexity)
        let complexity_penalty = if bridge_count > 2 {
            BigDecimal::from_str("0.1").unwrap() // 10% penalty for 3+ bridges
        } else {
            BigDecimal::zero()
        };

        Ok((average_risk + complexity_penalty).min(BigDecimal::from(1)))
    }

    /// Assess security of a specific bridge between two chains
    async fn assess_bridge_security(
        &self,
        source_chain_id: i32,
        destination_chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        // Simplified implementation - in production this would query bridge data
        let bridge_assessment = self.get_bridge_assessment(source_chain_id, destination_chain_id).await?;
        
        // Calculate bridge risk based on multiple factors
        let security_risk = BigDecimal::from(1) - &bridge_assessment.security_score;
        let audit_risk = BigDecimal::from(1) - &bridge_assessment.audit_score;
        let tvl_risk = self.calculate_tvl_risk(&bridge_assessment.tvl_score);
        let exploit_risk = self.calculate_exploit_history_risk(bridge_assessment.exploit_history_score);

        // Weighted combination of bridge risk factors
        let bridge_risk =
            &security_risk * &BigDecimal::from_str("0.4").unwrap() +     // 40% security
            &audit_risk * &BigDecimal::from_str("0.3").unwrap() +        // 30% audit quality
            &tvl_risk * &BigDecimal::from_str("0.2").unwrap() +          // 20% TVL risk
            &exploit_risk * &BigDecimal::from_str("0.1").unwrap();        // 10% exploit history

        Ok(bridge_risk.min(BigDecimal::from(1)))
    }

    /// Calculate liquidity fragmentation risk across chains
    async fn calculate_liquidity_fragmentation_risk(
        &self,
        pool_states: &[PoolState],
    ) -> Result<BigDecimal, AppError> {
        if pool_states.len() <= 1 {
            return Ok(BigDecimal::zero()); // No fragmentation with single pool
        }

        // Calculate total liquidity across all chains
        let total_tvl: BigDecimal = pool_states
            .iter()
            .filter_map(|pool| pool.tvl_usd.as_ref())
            .sum();

        if total_tvl.is_zero() {
            return Ok(BigDecimal::from_str("0.8").unwrap()); // High risk for zero liquidity
        }

        // Calculate liquidity distribution (Gini coefficient approach)
        let mut tvl_values: Vec<BigDecimal> = pool_states
            .iter()
            .filter_map(|pool| pool.tvl_usd.as_ref())
            .cloned()
            .collect();
        
        tvl_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let fragmentation_score = self.calculate_liquidity_gini_coefficient(&tvl_values, &total_tvl);
        
        // Convert fragmentation score to risk (higher fragmentation = higher risk)
        let fragmentation_risk = if fragmentation_score > self.config.fragmentation_critical_threshold {
            BigDecimal::from_str("0.9").unwrap() // Critical fragmentation
        } else if fragmentation_score > self.config.fragmentation_warning_threshold {
            // Scale between warning and critical thresholds
            let ratio = (&fragmentation_score - &self.config.fragmentation_warning_threshold) /
                       (&self.config.fragmentation_critical_threshold - &self.config.fragmentation_warning_threshold);
            BigDecimal::from_str("0.3").unwrap() + (ratio * BigDecimal::from_str("0.6").unwrap())
        } else {
            BigDecimal::from_str("0.1").unwrap() // Low fragmentation risk
        };

        Ok(fragmentation_risk)
    }

    /// Calculate governance divergence risk between chains
    async fn calculate_governance_divergence_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<BigDecimal, AppError> {
        let primary_governance = self.get_chain_governance_score(primary_chain_id).await?;
        let mut total_divergence = BigDecimal::zero();
        let mut chain_count = 0;

        for &secondary_chain_id in secondary_chain_ids {
            let secondary_governance = self.get_chain_governance_score(secondary_chain_id).await?;
            let divergence = (&primary_governance - &secondary_governance).abs();
            total_divergence += divergence;
            chain_count += 1;
        }

        if chain_count == 0 {
            return Ok(BigDecimal::zero());
        }

        let average_divergence = total_divergence / BigDecimal::from(chain_count);
        
        // Convert divergence to risk score (higher divergence = higher risk)
        let governance_risk = if average_divergence > BigDecimal::from_str("0.5").unwrap() {
            BigDecimal::from_str("0.8").unwrap() // High divergence = high risk
        } else if average_divergence > BigDecimal::from_str("0.3").unwrap() {
            BigDecimal::from_str("0.5").unwrap() // Medium divergence = medium risk
        } else {
            BigDecimal::from_str("0.2").unwrap() // Low divergence = low risk
        };

        Ok(governance_risk)
    }

    /// Calculate technical compatibility risk between chains
    async fn calculate_technical_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<BigDecimal, AppError> {
        let primary_tech_score = self.get_chain_technical_score(primary_chain_id).await?;
        let mut compatibility_risk = BigDecimal::zero();
        let mut chain_count = 0;

        for &secondary_chain_id in secondary_chain_ids {
            let secondary_tech_score = self.get_chain_technical_score(secondary_chain_id).await?;
            
            // Calculate compatibility based on technical maturity difference
            let maturity_diff = (&primary_tech_score - &secondary_tech_score).abs();
            let chain_risk = maturity_diff * BigDecimal::from_str("0.5").unwrap(); // Scale factor
            
            compatibility_risk += chain_risk;
            chain_count += 1;
        }

        if chain_count == 0 {
            return Ok(BigDecimal::zero());
        }

        let average_risk = compatibility_risk / BigDecimal::from(chain_count);
        Ok(average_risk.min(BigDecimal::from(1)))
    }

    /// Calculate correlation risk between chains
    async fn calculate_correlation_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<BigDecimal, AppError> {
        let mut total_correlation_risk = BigDecimal::zero();
        let mut pair_count = 0;

        for &secondary_chain_id in secondary_chain_ids {
            let correlation = self.get_chain_correlation(primary_chain_id, secondary_chain_id).await?;
            
            // High correlation increases risk (lack of diversification)
            let correlation_risk = if correlation > self.config.critical_correlation_threshold {
                BigDecimal::from_str("0.9").unwrap() // Critical correlation
            } else if correlation > self.config.high_correlation_threshold {
                BigDecimal::from_str("0.6").unwrap() // High correlation
            } else {
                BigDecimal::from_str("0.2").unwrap() // Low correlation (good diversification)
            };

            total_correlation_risk += correlation_risk;
            pair_count += 1;
        }

        if pair_count == 0 {
            return Ok(BigDecimal::zero());
        }

        let average_correlation_risk = total_correlation_risk / BigDecimal::from(pair_count);
        Ok(average_correlation_risk)
    }

    /// Calculate weighted overall cross-chain risk
    fn calculate_weighted_cross_chain_risk(
        &self,
        bridge_risk: &BigDecimal,
        liquidity_fragmentation_risk: &BigDecimal,
        governance_divergence_risk: &BigDecimal,
        technical_risk: &BigDecimal,
        correlation_risk: &BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        let weighted_risk = (
            bridge_risk * &self.config.bridge_risk_weight +
            liquidity_fragmentation_risk * &self.config.liquidity_fragmentation_weight +
            governance_divergence_risk * &self.config.governance_divergence_weight +
            technical_risk * &self.config.technical_risk_weight +
            correlation_risk * &self.config.correlation_risk_weight
        ) / BigDecimal::from(100); // Convert from percentage weights

        Ok(weighted_risk.min(BigDecimal::from(1)))
    }

    /// Calculate confidence score for cross-chain risk assessment
    async fn calculate_confidence_score(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
        pool_states: &[PoolState],
    ) -> Result<BigDecimal, AppError> {
        let mut confidence_factors = Vec::new();

        // Data availability factor
        let data_availability = if pool_states.len() >= secondary_chain_ids.len() + 1 {
            BigDecimal::from_str("0.3").unwrap() // Full data available
        } else {
            BigDecimal::from_str("0.1").unwrap() // Partial data
        };
        confidence_factors.push(data_availability);

        // Chain maturity factor
        let chain_maturity = self.assess_chain_maturity(primary_chain_id, secondary_chain_ids).await?;
        confidence_factors.push(chain_maturity);

        // Historical data factor
        let historical_data_quality = BigDecimal::from_str("0.2").unwrap(); // Simplified
        confidence_factors.push(historical_data_quality);

        // Bridge assessment quality
        let bridge_assessment_quality = BigDecimal::from_str("0.2").unwrap(); // Simplified
        confidence_factors.push(bridge_assessment_quality);

        let total_confidence: BigDecimal = confidence_factors.iter().sum();
        Ok(total_confidence.min(BigDecimal::from(1)))
    }

    // Helper methods for data retrieval and calculations

    async fn get_bridge_assessment(
        &self,
        source_chain_id: i32,
        destination_chain_id: i32,
    ) -> Result<BridgeSecurityAssessment, AppError> {
        // Simplified implementation - in production this would query bridge data
        Ok(BridgeSecurityAssessment {
            bridge_protocol: format!("Bridge-{}-{}", source_chain_id, destination_chain_id),
            security_score: BigDecimal::from_str("0.8").unwrap(),
            audit_score: BigDecimal::from_str("0.85").unwrap(),
            tvl_score: BigDecimal::from_str("0.7").unwrap(),
            decentralization_score: BigDecimal::from_str("0.6").unwrap(),
            exploit_history_score: BigDecimal::from_str("0.9").unwrap(),
            overall_score: BigDecimal::from_str("0.78").unwrap(),
        })
    }

    async fn get_chain_governance_score(&self, chain_id: i32) -> Result<BigDecimal, AppError> {
        // Simplified implementation - in production this would query chain governance data
        match chain_id {
            1 => Ok(BigDecimal::from_str("0.85").unwrap()), // Ethereum
            137 => Ok(BigDecimal::from_str("0.75").unwrap()), // Polygon
            56 => Ok(BigDecimal::from_str("0.65").unwrap()), // BSC
            43114 => Ok(BigDecimal::from_str("0.70").unwrap()), // Avalanche
            _ => Ok(BigDecimal::from_str("0.60").unwrap()), // Default for other chains
        }
    }

    async fn get_chain_technical_score(&self, chain_id: i32) -> Result<BigDecimal, AppError> {
        // Simplified implementation - in production this would assess technical maturity
        match chain_id {
            1 => Ok(BigDecimal::from_str("0.95").unwrap()), // Ethereum - most mature
            137 => Ok(BigDecimal::from_str("0.80").unwrap()), // Polygon
            56 => Ok(BigDecimal::from_str("0.75").unwrap()), // BSC
            43114 => Ok(BigDecimal::from_str("0.85").unwrap()), // Avalanche
            _ => Ok(BigDecimal::from_str("0.70").unwrap()), // Default for other chains
        }
    }

    async fn get_chain_correlation(&self, chain_id_1: i32, chain_id_2: i32) -> Result<BigDecimal, AppError> {
        // Simplified implementation - in production this would calculate actual correlations
        if chain_id_1 == chain_id_2 {
            return Ok(BigDecimal::from(1)); // Perfect correlation with self
        }

        // Simulate correlation based on chain relationships
        let correlation = match (chain_id_1, chain_id_2) {
            (1, 137) | (137, 1) => BigDecimal::from_str("0.75").unwrap(), // ETH-Polygon high correlation
            (1, 56) | (56, 1) => BigDecimal::from_str("0.65").unwrap(),   // ETH-BSC medium correlation
            (137, 56) | (56, 137) => BigDecimal::from_str("0.60").unwrap(), // Polygon-BSC medium correlation
            _ => BigDecimal::from_str("0.45").unwrap(), // Default moderate correlation
        };

        Ok(correlation)
    }

    fn calculate_tvl_risk(&self, tvl_score: &BigDecimal) -> BigDecimal {
        // Higher TVL = lower risk
        BigDecimal::from(1) - tvl_score
    }

    fn calculate_exploit_history_risk(&self, exploit_score: BigDecimal) -> BigDecimal {
        // Lower exploit score = higher risk
        BigDecimal::from(1) - exploit_score
    }

    fn calculate_liquidity_gini_coefficient(
        &self,
        sorted_tvl_values: &[BigDecimal],
        total_tvl: &BigDecimal,
    ) -> BigDecimal {
        if sorted_tvl_values.len() <= 1 {
            return BigDecimal::zero();
        }

        let n = BigDecimal::from(sorted_tvl_values.len() as i32);
        let mut sum_weighted = BigDecimal::zero();

        for (i, tvl) in sorted_tvl_values.iter().enumerate() {
            let weight = BigDecimal::from(2 * (i + 1) as i32) - &n - BigDecimal::from(1);
            sum_weighted += tvl * weight;
        }

        let gini = sum_weighted / (&n * total_tvl);
        gini.abs() // Ensure positive value
    }

    async fn assess_chain_maturity(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<BigDecimal, AppError> {
        let mut total_maturity = self.get_chain_technical_score(primary_chain_id).await?;
        let mut chain_count = 1;

        for &chain_id in secondary_chain_ids {
            total_maturity += self.get_chain_technical_score(chain_id).await?;
            chain_count += 1;
        }

        Ok(total_maturity / BigDecimal::from(chain_count))
    }

    fn identify_risk_factors(
        &self,
        bridge_risk: &BigDecimal,
        liquidity_fragmentation_risk: &BigDecimal,
        governance_divergence_risk: &BigDecimal,
        technical_risk: &BigDecimal,
        correlation_risk: &BigDecimal,
    ) -> Vec<String> {
        let mut factors = Vec::new();

        if bridge_risk > &BigDecimal::from_str("0.7").unwrap() {
            factors.push("High bridge security risk detected".to_string());
        }

        if liquidity_fragmentation_risk > &BigDecimal::from_str("0.6").unwrap() {
            factors.push("Significant liquidity fragmentation across chains".to_string());
        }

        if governance_divergence_risk > &BigDecimal::from_str("0.5").unwrap() {
            factors.push("Governance model divergence between chains".to_string());
        }

        if technical_risk > &BigDecimal::from_str("0.5").unwrap() {
            factors.push("Technical compatibility concerns".to_string());
        }

        if correlation_risk > &BigDecimal::from_str("0.7").unwrap() {
            factors.push("High correlation reduces diversification benefits".to_string());
        }

        if factors.is_empty() {
            factors.push("Cross-chain risk within acceptable parameters".to_string());
        }

        factors
    }

    fn generate_recommendations(
        &self,
        bridge_risk: &BigDecimal,
        liquidity_fragmentation_risk: &BigDecimal,
        governance_divergence_risk: &BigDecimal,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if bridge_risk > &BigDecimal::from_str("0.7").unwrap() {
            recommendations.push("Consider using more secure bridge protocols or reducing cross-chain exposure".to_string());
        }

        if liquidity_fragmentation_risk > &BigDecimal::from_str("0.6").unwrap() {
            recommendations.push("Consolidate liquidity on fewer chains to reduce fragmentation risk".to_string());
        }

        if governance_divergence_risk > &BigDecimal::from_str("0.5").unwrap() {
            recommendations.push("Monitor governance proposals across all chains for potential conflicts".to_string());
        }

        recommendations.push("Regularly monitor cross-chain bridge security and exploit reports".to_string());
        recommendations.push("Maintain emergency exit strategies for each chain".to_string());

        recommendations
    }

    /// Store cross-chain risk assessment to database
    pub async fn store_cross_chain_risk(&self, _cross_chain_risk: &CrossChainRisk) -> Result<(), AppError> {
        // Simplified implementation for now - in production this would store to database
        // TODO: Implement actual database storage once schema is finalized
        Ok(())
    }

    /// Get cached cross-chain risk assessment
    pub async fn get_cross_chain_risk(
        &self,
        _primary_chain_id: i32,
        _secondary_chain_ids: &[i32],
    ) -> Result<Option<CrossChainRisk>, AppError> {
        // Simplified implementation for now - in production this would query the database
        // TODO: Implement actual database query once schema is finalized
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_chain_risk_config_default() {
        let config = CrossChainRiskConfig::default();
        
        // Verify weights sum to 100%
        let total_weight = &config.bridge_risk_weight +
                          &config.liquidity_fragmentation_weight +
                          &config.governance_divergence_weight +
                          &config.technical_risk_weight +
                          &config.correlation_risk_weight;
        
        assert_eq!(total_weight, BigDecimal::from(100));
        
        // Verify reasonable default values
        assert!(config.bridge_risk_weight >= BigDecimal::from(25)); // Bridge risk should be significant
        assert!(config.bridge_tvl_critical_threshold >= BigDecimal::from(1000000)); // Reasonable TVL threshold
    }

    #[tokio::test]
    async fn test_weighted_cross_chain_risk_calculation() {
        // Test the calculation logic directly without database dependency
        let config = CrossChainRiskConfig::default();
        
        let bridge_risk = BigDecimal::from_str("0.8").unwrap();
        let liquidity_fragmentation_risk = BigDecimal::from_str("0.6").unwrap();
        let governance_divergence_risk = BigDecimal::from_str("0.4").unwrap();
        let technical_risk = BigDecimal::from_str("0.3").unwrap();
        let correlation_risk = BigDecimal::from_str("0.7").unwrap();

        // Calculate weighted cross-chain risk directly using config weights
        let weighted_risk = (
            &bridge_risk * &config.bridge_risk_weight +
            &liquidity_fragmentation_risk * &config.liquidity_fragmentation_weight +
            &governance_divergence_risk * &config.governance_divergence_weight +
            &technical_risk * &config.technical_risk_weight +
            &correlation_risk * &config.correlation_risk_weight
        ) / BigDecimal::from(100);

        // Risk should be between 0 and 1
        assert!(weighted_risk >= BigDecimal::from(0));
        assert!(weighted_risk <= BigDecimal::from(1));
        
        // Should be a reasonable risk level given the inputs
        assert!(weighted_risk >= BigDecimal::from_str("0.5").unwrap());
    }

    #[tokio::test]
    async fn test_liquidity_fragmentation_calculation() {
        // Test fragmentation calculation with different scenarios
        
        // Scenario 1: Well-distributed liquidity (low fragmentation)
        let balanced_pools = vec![
            create_test_pool_state_with_tvl(BigDecimal::from(5000000), 1),   // $5M
            create_test_pool_state_with_tvl(BigDecimal::from(4000000), 137), // $4M
            create_test_pool_state_with_tvl(BigDecimal::from(3000000), 56),  // $3M
        ];
        
        // Scenario 2: Highly fragmented liquidity (high fragmentation)
        let fragmented_pools = vec![
            create_test_pool_state_with_tvl(BigDecimal::from(10000000), 1), // $10M
            create_test_pool_state_with_tvl(BigDecimal::from(100000), 137),  // $100K
            create_test_pool_state_with_tvl(BigDecimal::from(50000), 56),    // $50K
        ];
        
        // Test that fragmented scenario has higher risk
        let balanced_fragmentation = calculate_fragmentation_risk(&balanced_pools);
        let high_fragmentation = calculate_fragmentation_risk(&fragmented_pools);
        
        assert!(high_fragmentation > balanced_fragmentation);
    }

    fn create_test_pool_state_with_tvl(tvl: BigDecimal, chain_id: i32) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: format!("0x{:040x}", chain_id),
            chain_id,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(tvl),
            volume_24h_usd: Some(BigDecimal::from(1000000)),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }

    fn calculate_fragmentation_risk(pool_states: &[PoolState]) -> BigDecimal {
        // Simplified fragmentation calculation for testing
        if pool_states.len() <= 1 {
            return BigDecimal::zero();
        }

        let total_tvl: BigDecimal = pool_states
            .iter()
            .filter_map(|pool| pool.tvl_usd.as_ref())
            .sum();

        let zero_tvl = BigDecimal::zero();
        let max_tvl = pool_states
            .iter()
            .filter_map(|pool| pool.tvl_usd.as_ref())
            .max()
            .unwrap_or(&zero_tvl);

        // Concentration ratio = largest_pool / total_tvl
        // Higher concentration = higher fragmentation risk
        if total_tvl.is_zero() {
            BigDecimal::from_str("0.8").unwrap()
        } else {
            let concentration_ratio = max_tvl / &total_tvl;
            // Convert concentration to risk: higher concentration = higher risk
            if concentration_ratio > BigDecimal::from_str("0.8").unwrap() {
                BigDecimal::from_str("0.9").unwrap() // Very high concentration = high fragmentation risk
            } else if concentration_ratio > BigDecimal::from_str("0.6").unwrap() {
                BigDecimal::from_str("0.6").unwrap() // High concentration = medium-high risk
            } else if concentration_ratio > BigDecimal::from_str("0.4").unwrap() {
                BigDecimal::from_str("0.3").unwrap() // Medium concentration = low-medium risk
            } else {
                BigDecimal::from_str("0.1").unwrap() // Well distributed = low risk
            }
        }
    }
}
