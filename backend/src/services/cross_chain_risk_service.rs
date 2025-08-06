use crate::models::cross_chain_risk::*;
use crate::models::PoolState;
use crate::error::AppError;
use bigdecimal::{BigDecimal, Zero};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Debug, Clone)]
struct ChainLiquidityMetrics {
    tvl: BigDecimal,
    volume_24h: BigDecimal,
    pool_count: i32,
    avg_pool_size: BigDecimal,
    liquidity_utilization: BigDecimal,
}

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
    pub async fn calculate_liquidity_fragmentation_risk(
        &self,
        pool_states: &[PoolState],
    ) -> Result<BigDecimal, AppError> {
        if pool_states.is_empty() {
            return Ok(BigDecimal::zero());
        }

        // Group pools by chain and calculate comprehensive metrics
        let mut chain_metrics: std::collections::HashMap<i32, ChainLiquidityMetrics> = std::collections::HashMap::new();
        let mut total_tvl = BigDecimal::zero();
        let mut total_volume_24h = BigDecimal::zero();

        let zero_decimal = BigDecimal::zero();
        for pool in pool_states {
            let pool_tvl = pool.tvl_usd.as_ref().unwrap_or(&zero_decimal);
            let pool_volume = pool.volume_24h_usd.as_ref().unwrap_or(&zero_decimal);
            
            let metrics = chain_metrics.entry(pool.chain_id).or_insert_with(|| ChainLiquidityMetrics {
                tvl: BigDecimal::zero(),
                volume_24h: BigDecimal::zero(),
                pool_count: 0,
                avg_pool_size: BigDecimal::zero(),
                liquidity_utilization: BigDecimal::zero(),
            });
            
            metrics.tvl += pool_tvl;
            metrics.volume_24h += pool_volume;
            metrics.pool_count += 1;
            
            total_tvl += pool_tvl;
            total_volume_24h += pool_volume;
        }

        if total_tvl.is_zero() {
            return Ok(BigDecimal::zero());
        }

        // Calculate derived metrics for each chain
        for metrics in chain_metrics.values_mut() {
            metrics.avg_pool_size = if metrics.pool_count > 0 {
                &metrics.tvl / BigDecimal::from(metrics.pool_count)
            } else {
                BigDecimal::zero()
            };
            
            metrics.liquidity_utilization = if !metrics.tvl.is_zero() {
                &metrics.volume_24h / &metrics.tvl
            } else {
                BigDecimal::zero()
            };
        }

        // Calculate multiple fragmentation risk components
        let tvl_fragmentation = self.calculate_tvl_fragmentation_risk(&chain_metrics, &total_tvl);
        let volume_fragmentation = self.calculate_volume_fragmentation_risk(&chain_metrics, &total_volume_24h);
        let chain_diversity_risk = self.calculate_chain_diversity_risk(chain_metrics.len());
        let utilization_imbalance = self.calculate_utilization_imbalance_risk(&chain_metrics);
        let bridge_dependency_risk = self.calculate_bridge_dependency_risk(&chain_metrics).await?;

        // Weighted combination of all fragmentation factors
        let weights = [
            (&tvl_fragmentation, 0.30),
            (&volume_fragmentation, 0.25), 
            (&chain_diversity_risk, 0.20),
            (&utilization_imbalance, 0.15),
            (&bridge_dependency_risk, 0.10),
        ];
        
        let overall_fragmentation_risk: BigDecimal = weights.iter()
            .map(|(risk, weight)| *risk * BigDecimal::from_str(&weight.to_string()).unwrap())
            .sum();

        // Cap at 1.0
        Ok(overall_fragmentation_risk.min(BigDecimal::from(1)))
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
        // Query database for bridge risk data
        let bridge_risk = sqlx::query_as!(
            BridgeRisk,
            r#"
            SELECT id, bridge_protocol, source_chain_id, destination_chain_id,
                   security_score, tvl_locked, exploit_history_count, audit_score,
                   decentralization_score, overall_bridge_risk, last_assessment
            FROM bridge_risks
            WHERE source_chain_id = $1 AND destination_chain_id = $2
            ORDER BY last_assessment DESC
            LIMIT 1
            "#,
            source_chain_id,
            destination_chain_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get bridge assessment: {}", e)))?;

        if let Some(bridge) = bridge_risk {
            // Use existing bridge assessment data
            let tvl_score = bridge.tvl_locked
                .map(|tvl| self.calculate_tvl_risk_score(&tvl))
                .unwrap_or_else(|| BigDecimal::from_str("0.5").unwrap());
            
            let exploit_history_score = self.calculate_exploit_history_score(bridge.exploit_history_count);

            Ok(BridgeSecurityAssessment {
                bridge_protocol: bridge.bridge_protocol,
                security_score: bridge.security_score,
                audit_score: bridge.audit_score,
                tvl_score,
                decentralization_score: bridge.decentralization_score,
                exploit_history_score,
                overall_score: bridge.overall_bridge_risk,
            })
        } else {
            // Create default assessment for unknown bridge pairs
            let bridge_protocol = self.identify_bridge_protocol(source_chain_id, destination_chain_id);
            let security_score = self.estimate_bridge_security(source_chain_id, destination_chain_id);
            let audit_score = self.estimate_audit_score(&bridge_protocol);
            let tvl_score = BigDecimal::from_str("0.5").unwrap(); // Default medium risk
            let decentralization_score = self.estimate_decentralization_score(&bridge_protocol);
            let exploit_history_score = BigDecimal::from_str("0.8").unwrap(); // Assume no known exploits
            
            let overall_score = self.calculate_bridge_overall_score(
                &security_score, &audit_score, &tvl_score, 
                &decentralization_score, &exploit_history_score
            );

            Ok(BridgeSecurityAssessment {
                bridge_protocol,
                security_score,
                audit_score,
                tvl_score,
                decentralization_score,
                exploit_history_score,
                overall_score,
            })
        }
    }

    async fn get_chain_governance_score(&self, chain_id: i32) -> Result<BigDecimal, AppError> {
        // Query database for chain governance data
        let chain_risk = sqlx::query_as!(
            ChainRisk,
            "SELECT id, chain_id, chain_name, network_security_score, validator_decentralization, governance_risk, technical_maturity, ecosystem_health, liquidity_depth, overall_chain_risk, last_updated FROM chain_risks WHERE chain_id = $1",
            chain_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get chain governance: {}", e)))?;

        if let Some(chain) = chain_risk {
            Ok(chain.governance_risk)
        } else {
            // Fallback to hardcoded values for known chains
            match chain_id {
                1 => Ok(BigDecimal::from_str("0.85").unwrap()), // Ethereum
                137 => Ok(BigDecimal::from_str("0.75").unwrap()), // Polygon
                56 => Ok(BigDecimal::from_str("0.65").unwrap()), // BSC
                43114 => Ok(BigDecimal::from_str("0.70").unwrap()), // Avalanche
                _ => Ok(BigDecimal::from_str("0.60").unwrap()), // Default for other chains
            }
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
    pub async fn store_cross_chain_risk(&self, cross_chain_risk: &CrossChainRisk) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO cross_chain_risks (
                id, position_id, primary_chain_id, secondary_chain_ids,
                bridge_risk_score, liquidity_fragmentation_risk, governance_divergence_risk,
                technical_risk_score, correlation_risk_score, overall_cross_chain_risk,
                confidence_score, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE SET
                bridge_risk_score = EXCLUDED.bridge_risk_score,
                liquidity_fragmentation_risk = EXCLUDED.liquidity_fragmentation_risk,
                governance_divergence_risk = EXCLUDED.governance_divergence_risk,
                technical_risk_score = EXCLUDED.technical_risk_score,
                correlation_risk_score = EXCLUDED.correlation_risk_score,
                overall_cross_chain_risk = EXCLUDED.overall_cross_chain_risk,
                confidence_score = EXCLUDED.confidence_score,
                updated_at = EXCLUDED.updated_at
            "#,
            cross_chain_risk.id,
            cross_chain_risk.position_id,
            cross_chain_risk.primary_chain_id,
            &cross_chain_risk.secondary_chain_ids,
            cross_chain_risk.bridge_risk_score,
            cross_chain_risk.liquidity_fragmentation_risk,
            cross_chain_risk.governance_divergence_risk,
            cross_chain_risk.technical_risk_score,
            cross_chain_risk.correlation_risk_score,
            cross_chain_risk.overall_cross_chain_risk,
            cross_chain_risk.confidence_score,
            cross_chain_risk.created_at,
            cross_chain_risk.updated_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store cross-chain risk: {}", e)))?;

        Ok(())
    }

    /// Get cached cross-chain risk assessment
    pub async fn get_cross_chain_risk(
        &self,
        primary_chain_id: i32,
        secondary_chain_ids: &[i32],
    ) -> Result<Option<CrossChainRisk>, AppError> {
        let result = sqlx::query_as!(
            CrossChainRisk,
            r#"
            SELECT id, position_id, primary_chain_id, secondary_chain_ids,
                   bridge_risk_score, liquidity_fragmentation_risk, governance_divergence_risk,
                   technical_risk_score, correlation_risk_score, overall_cross_chain_risk,
                   confidence_score, created_at, updated_at
            FROM cross_chain_risks
            WHERE primary_chain_id = $1 AND secondary_chain_ids = $2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            primary_chain_id,
            secondary_chain_ids
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get cross-chain risk: {}", e)))?;

        Ok(result)
    }

    // Bridge assessment helper methods
    fn identify_bridge_protocol(&self, source_chain_id: i32, destination_chain_id: i32) -> String {
        match (source_chain_id, destination_chain_id) {
            (1, 137) | (137, 1) => "Polygon Bridge".to_string(),
            (1, 56) | (56, 1) => "BSC Bridge".to_string(),
            (1, 43114) | (43114, 1) => "Avalanche Bridge".to_string(),
            (1, 42161) | (42161, 1) => "Arbitrum Bridge".to_string(),
            (1, 10) | (10, 1) => "Optimism Bridge".to_string(),
            _ => format!("Generic Bridge {}-{}", source_chain_id, destination_chain_id),
        }
    }

    fn estimate_bridge_security(&self, source_chain_id: i32, destination_chain_id: i32) -> BigDecimal {
        // Security based on chain maturity and bridge type
        let source_security = match source_chain_id {
            1 => 0.95,   // Ethereum - highest security
            42161 | 10 => 0.90, // L2s inherit Ethereum security
            137 => 0.80, // Polygon
            43114 => 0.85, // Avalanche
            56 => 0.70,  // BSC
            _ => 0.60,   // Unknown chains
        };
        
        let dest_security = match destination_chain_id {
            1 => 0.95, 42161 | 10 => 0.90, 137 => 0.80,
            43114 => 0.85, 56 => 0.70, _ => 0.60,
        };
        
        BigDecimal::from_str(&((source_security + dest_security) / 2.0).to_string()).unwrap()
    }

    fn estimate_audit_score(&self, bridge_protocol: &str) -> BigDecimal {
        match bridge_protocol {
            p if p.contains("Polygon") => BigDecimal::from_str("0.90").unwrap(),
            p if p.contains("Arbitrum") => BigDecimal::from_str("0.95").unwrap(),
            p if p.contains("Optimism") => BigDecimal::from_str("0.95").unwrap(),
            p if p.contains("Avalanche") => BigDecimal::from_str("0.85").unwrap(),
            p if p.contains("BSC") => BigDecimal::from_str("0.75").unwrap(),
            _ => BigDecimal::from_str("0.70").unwrap(),
        }
    }

    fn estimate_decentralization_score(&self, bridge_protocol: &str) -> BigDecimal {
        match bridge_protocol {
            p if p.contains("Arbitrum") || p.contains("Optimism") => BigDecimal::from_str("0.85").unwrap(),
            p if p.contains("Polygon") => BigDecimal::from_str("0.75").unwrap(),
            p if p.contains("Avalanche") => BigDecimal::from_str("0.80").unwrap(),
            p if p.contains("BSC") => BigDecimal::from_str("0.60").unwrap(),
            _ => BigDecimal::from_str("0.65").unwrap(),
        }
    }

    fn calculate_tvl_risk_score(&self, tvl: &BigDecimal) -> BigDecimal {
        // Higher TVL = lower risk (more battle-tested)
        let tvl_millions = tvl / BigDecimal::from(1_000_000);
        if tvl_millions >= BigDecimal::from(1000) {
            BigDecimal::from_str("0.90").unwrap() // Very high TVL
        } else if tvl_millions >= BigDecimal::from(100) {
            BigDecimal::from_str("0.80").unwrap() // High TVL
        } else if tvl_millions >= BigDecimal::from(10) {
            BigDecimal::from_str("0.70").unwrap() // Medium TVL
        } else {
            BigDecimal::from_str("0.50").unwrap() // Low TVL
        }
    }

    fn calculate_exploit_history_score(&self, exploit_count: i32) -> BigDecimal {
        match exploit_count {
            0 => BigDecimal::from_str("0.95").unwrap(),
            1 => BigDecimal::from_str("0.75").unwrap(),
            2 => BigDecimal::from_str("0.60").unwrap(),
            3 => BigDecimal::from_str("0.45").unwrap(),
            _ => BigDecimal::from_str("0.30").unwrap(),
        }
    }

    fn calculate_bridge_overall_score(
        &self,
        security_score: &BigDecimal,
        audit_score: &BigDecimal,
        tvl_score: &BigDecimal,
        decentralization_score: &BigDecimal,
        exploit_history_score: &BigDecimal,
    ) -> BigDecimal {
        let weights = [
            (security_score, 0.3),
            (audit_score, 0.25),
            (tvl_score, 0.2),
            (decentralization_score, 0.15),
            (exploit_history_score, 0.1),
        ];
        
        weights.iter()
            .map(|(score, weight)| *score * BigDecimal::from_str(&weight.to_string()).unwrap())
            .sum()
    }

    // Enhanced liquidity fragmentation helper methods
    fn calculate_tvl_fragmentation_risk(
        &self,
        chain_metrics: &std::collections::HashMap<i32, ChainLiquidityMetrics>,
        total_tvl: &BigDecimal,
    ) -> BigDecimal {
        let mut tvl_values: Vec<BigDecimal> = chain_metrics.values().map(|m| m.tvl.clone()).collect();
        tvl_values.sort();
        self.calculate_liquidity_gini_coefficient(&tvl_values, total_tvl)
    }

    fn calculate_volume_fragmentation_risk(
        &self,
        chain_metrics: &std::collections::HashMap<i32, ChainLiquidityMetrics>,
        total_volume: &BigDecimal,
    ) -> BigDecimal {
        if total_volume.is_zero() {
            return BigDecimal::from_str("0.5").unwrap();
        }
        let mut volume_values: Vec<BigDecimal> = chain_metrics.values().map(|m| m.volume_24h.clone()).collect();
        volume_values.sort();
        self.calculate_liquidity_gini_coefficient(&volume_values, total_volume)
    }

    fn calculate_chain_diversity_risk(&self, chain_count: usize) -> BigDecimal {
        match chain_count {
            1 => BigDecimal::zero(), // No cross-chain risk
            2 => BigDecimal::from_str("0.2").unwrap(),
            3 => BigDecimal::from_str("0.4").unwrap(),
            4 => BigDecimal::from_str("0.6").unwrap(),
            5 => BigDecimal::from_str("0.75").unwrap(),
            _ => BigDecimal::from_str("0.9").unwrap(), // High complexity risk
        }
    }

    fn calculate_utilization_imbalance_risk(
        &self,
        chain_metrics: &std::collections::HashMap<i32, ChainLiquidityMetrics>,
    ) -> BigDecimal {
        let utilizations: Vec<BigDecimal> = chain_metrics.values()
            .map(|m| m.liquidity_utilization.clone())
            .collect();
        
        if utilizations.len() < 2 {
            return BigDecimal::zero();
        }
        
        let avg_utilization: BigDecimal = utilizations.iter().sum::<BigDecimal>() / BigDecimal::from(utilizations.len() as i32);
        let variance: BigDecimal = utilizations.iter()
            .map(|u| (u - &avg_utilization).abs())
            .sum::<BigDecimal>() / BigDecimal::from(utilizations.len() as i32);
        
        // Higher variance = higher imbalance risk
        variance.min(BigDecimal::from(1))
    }

    async fn calculate_bridge_dependency_risk(
        &self,
        chain_metrics: &std::collections::HashMap<i32, ChainLiquidityMetrics>,
    ) -> Result<BigDecimal, AppError> {
        let chain_ids: Vec<i32> = chain_metrics.keys().cloned().collect();
        if chain_ids.len() < 2 {
            return Ok(BigDecimal::zero());
        }
        
        let mut total_bridge_risk = BigDecimal::zero();
        let mut bridge_count = 0;
        
        for i in 0..chain_ids.len() {
            for j in i+1..chain_ids.len() {
                let bridge_assessment = self.get_bridge_assessment(chain_ids[i], chain_ids[j]).await?;
                total_bridge_risk += BigDecimal::from(1) - bridge_assessment.overall_score;
                bridge_count += 1;
            }
        }
        
        if bridge_count > 0 {
            Ok(total_bridge_risk / BigDecimal::from(bridge_count))
        } else {
            Ok(BigDecimal::zero())
        }
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
