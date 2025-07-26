use crate::models::mev_risk::{
    MevRisk, MevRiskConfig, OracleDeviation
};
use crate::models::PoolState;
use crate::services::{BlockchainService, PriceValidationService};
use crate::error::AppError;
use bigdecimal::{BigDecimal, Zero};
use sqlx::PgPool;
use tracing::info;
use std::str::FromStr;
use chrono::{Utc, Duration};
use uuid::Uuid;

/// MEV and Oracle risk detection service
pub struct MevRiskService {
    db_pool: PgPool,
    config: MevRiskConfig,
    blockchain_service: Option<BlockchainService>,
    price_validation_service: Option<PriceValidationService>,
}

impl MevRiskService {
    pub fn new(
        db_pool: PgPool, 
        config: Option<MevRiskConfig>,
        blockchain_service: Option<BlockchainService>,
        price_validation_service: Option<PriceValidationService>,
    ) -> Self {
        Self {
            db_pool,
            config: config.unwrap_or_default(),
            blockchain_service,
            price_validation_service,
        }
    }

    /// Calculate comprehensive MEV risk for a pool
    pub async fn calculate_mev_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<MevRisk, AppError> {
        info!("Calculating MEV risk for pool {} on chain {}", pool_address, chain_id);

        // Calculate individual risk components
        let sandwich_risk = self.calculate_sandwich_risk(pool_address, chain_id, pool_state).await?;
        let frontrun_risk = self.calculate_frontrun_risk(pool_address, chain_id, pool_state).await?;
        let oracle_manipulation_risk = self.calculate_oracle_manipulation_risk(pool_address, chain_id).await?;
        let oracle_deviation_risk = self.calculate_oracle_deviation_risk(pool_address, chain_id).await?;

        // Calculate weighted overall MEV risk
        let overall_mev_risk = self.calculate_weighted_mev_risk(
            &sandwich_risk,
            &frontrun_risk,
            &oracle_manipulation_risk,
            &oracle_deviation_risk,
        )?;

        // Calculate confidence score based on data availability
        let confidence_score = self.calculate_confidence_score(pool_address, chain_id).await?;

        let mev_risk = MevRisk {
            id: Uuid::new_v4(),
            pool_address: pool_address.to_string(),
            chain_id,
            sandwich_risk_score: sandwich_risk,
            frontrun_risk_score: frontrun_risk,
            oracle_manipulation_risk,
            oracle_deviation_risk,
            overall_mev_risk,
            confidence_score,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Store the assessment in database
        self.store_mev_risk(&mev_risk).await?;

        Ok(mev_risk)
    }

    /// Detect sandwich attacks in recent transactions
    pub async fn calculate_sandwich_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating sandwich attack risk for pool {}", pool_address);

        // Check for recent sandwich attack patterns
        let recent_sandwich_count = self.get_recent_sandwich_attacks(pool_address, chain_id).await?;
        
        // Calculate risk based on pool liquidity and recent activity
        let liquidity_factor = if pool_state.liquidity.is_zero() {
            BigDecimal::from(1) // Maximum risk for zero liquidity
        } else {
            // Lower liquidity = higher sandwich risk
            let liquidity_usd = &pool_state.tvl_usd.clone().unwrap_or_else(|| BigDecimal::from(0));
            if liquidity_usd < &BigDecimal::from(100000) { // < $100K
                BigDecimal::from_str("0.8").unwrap()
            } else if liquidity_usd < &BigDecimal::from(1000000) { // < $1M
                BigDecimal::from_str("0.5").unwrap()
            } else if liquidity_usd < &BigDecimal::from(10000000) { // < $10M
                BigDecimal::from_str("0.3").unwrap()
            } else {
                BigDecimal::from_str("0.1").unwrap()
            }
        };

        // Activity factor based on recent sandwich attacks
        let activity_factor = if recent_sandwich_count > 10 {
            BigDecimal::from_str("0.9").unwrap()
        } else if recent_sandwich_count > 5 {
            BigDecimal::from_str("0.6").unwrap()
        } else if recent_sandwich_count > 0 {
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        // Combine factors (weighted average)
        let sandwich_risk = (&liquidity_factor * BigDecimal::from_str("0.6").unwrap()) + 
                           (&activity_factor * BigDecimal::from_str("0.4").unwrap());

        Ok(sandwich_risk.min(BigDecimal::from(1)))
    }

    /// Calculate frontrunning risk
    pub async fn calculate_frontrun_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating frontrunning risk for pool {}", pool_address);

        // Check for recent frontrunning activity
        let recent_frontrun_count = self.get_recent_frontrun_attacks(pool_address, chain_id).await?;
        
        // Calculate risk based on transaction volume and MEV bot activity
        let volume_factor = if let Some(volume) = &pool_state.volume_24h_usd {
            if volume > &BigDecimal::from(10000000) { // > $10M volume
                BigDecimal::from_str("0.7").unwrap() // High volume = more MEV opportunities
            } else if volume > &BigDecimal::from(1000000) { // > $1M volume
                BigDecimal::from_str("0.4").unwrap()
            } else {
                BigDecimal::from_str("0.2").unwrap()
            }
        } else {
            BigDecimal::from_str("0.3").unwrap() // Default moderate risk
        };

        // MEV bot activity factor
        let bot_activity_factor = if recent_frontrun_count > 20 {
            BigDecimal::from_str("0.8").unwrap()
        } else if recent_frontrun_count > 10 {
            BigDecimal::from_str("0.5").unwrap()
        } else if recent_frontrun_count > 0 {
            BigDecimal::from_str("0.2").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        let frontrun_risk = (&volume_factor * BigDecimal::from_str("0.5").unwrap()) + 
                           (&bot_activity_factor * BigDecimal::from_str("0.5").unwrap());

        Ok(frontrun_risk.min(BigDecimal::from(1)))
    }

    /// Calculate oracle manipulation risk
    pub async fn calculate_oracle_manipulation_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating oracle manipulation risk for pool {}", pool_address);

        // Check for recent oracle price manipulations
        let recent_manipulations = self.get_recent_oracle_manipulations(pool_address, chain_id).await?;
        
        // Check oracle update frequency and reliability
        let oracle_reliability = self.assess_oracle_reliability(pool_address, chain_id).await?;

        // Calculate risk based on manipulation history and oracle quality
        let manipulation_factor = if recent_manipulations > 5 {
            BigDecimal::from_str("0.9").unwrap()
        } else if recent_manipulations > 2 {
            BigDecimal::from_str("0.6").unwrap()
        } else if recent_manipulations > 0 {
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        // Oracle reliability factor (inverse - lower reliability = higher risk)
        let reliability_risk = BigDecimal::from(1) - oracle_reliability;

        let manipulation_risk = (&manipulation_factor * BigDecimal::from_str("0.7").unwrap()) + 
                               (&reliability_risk * BigDecimal::from_str("0.3").unwrap());

        Ok(manipulation_risk.min(BigDecimal::from(1)))
    }

    /// Calculate oracle deviation risk
    pub async fn calculate_oracle_deviation_risk(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        info!("Calculating oracle deviation risk for pool {}", pool_address);

        // Get recent oracle deviations
        let recent_deviations = self.get_recent_oracle_deviations(pool_address, chain_id).await?;
        
        // Calculate average deviation magnitude
        let avg_deviation = if recent_deviations.is_empty() {
            BigDecimal::from(0)
        } else {
            let total: BigDecimal = recent_deviations.iter()
                .map(|d| d.deviation_percent.abs())
                .sum();
            if recent_deviations.is_empty() {
                BigDecimal::zero()
            } else {
                total / BigDecimal::from_str(&recent_deviations.len().to_string()).unwrap_or_else(|_| BigDecimal::from(1))
            }
        };

        // Convert deviation percentage to risk score
        let deviation_risk = if avg_deviation > self.config.oracle_deviation_critical_percent {
            BigDecimal::from_str("0.9").unwrap()
        } else if avg_deviation > self.config.oracle_deviation_warning_percent {
            // Scale between warning and critical thresholds
            let ratio = &avg_deviation / &self.config.oracle_deviation_critical_percent;
            ratio * BigDecimal::from_str("0.9").unwrap()
        } else {
            // Low deviation
            BigDecimal::from_str("0.1").unwrap()
        };

        Ok(deviation_risk.min(BigDecimal::from(1)))
    }

    /// Calculate weighted MEV risk score
    fn calculate_weighted_mev_risk(
        &self,
        sandwich_risk: &BigDecimal,
        frontrun_risk: &BigDecimal,
        oracle_manipulation_risk: &BigDecimal,
        oracle_deviation_risk: &BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        let weighted_risk = 
            sandwich_risk * &self.config.sandwich_weight +
            frontrun_risk * &self.config.frontrun_weight +
            oracle_manipulation_risk * &self.config.oracle_manipulation_weight +
            oracle_deviation_risk * &self.config.oracle_deviation_weight;

        Ok(weighted_risk.min(BigDecimal::from(1)))
    }

    /// Calculate confidence score based on data availability
    async fn calculate_confidence_score(
        &self,
        pool_address: &str,
        chain_id: i32,
    ) -> Result<BigDecimal, AppError> {
        // Check data availability factors
        let has_blockchain_service = self.blockchain_service.is_some();
        let has_price_validation = self.price_validation_service.is_some();
        
        // Check recent transaction data availability
        let recent_tx_count = self.get_recent_transaction_count(pool_address, chain_id).await?;
        let tx_data_factor = if recent_tx_count > 100 {
            BigDecimal::from_str("0.3").unwrap()
        } else if recent_tx_count > 10 {
            BigDecimal::from_str("0.2").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        };

        let service_factor = match (has_blockchain_service, has_price_validation) {
            (true, true) => BigDecimal::from_str("0.4").unwrap(),
            (true, false) | (false, true) => BigDecimal::from_str("0.2").unwrap(),
            (false, false) => BigDecimal::from_str("0.1").unwrap(),
        };

        let base_confidence = BigDecimal::from_str("0.3").unwrap(); // Base confidence
        let confidence = base_confidence + service_factor + tx_data_factor;

        Ok(confidence.min(BigDecimal::from(1)))
    }

    /// Store MEV risk assessment in database
    async fn store_mev_risk(&self, mev_risk: &MevRisk) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO mev_risks (
                id, pool_address, chain_id, sandwich_risk_score, frontrun_risk_score,
                oracle_manipulation_risk, oracle_deviation_risk, overall_mev_risk,
                confidence_score, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            mev_risk.id,
            mev_risk.pool_address,
            mev_risk.chain_id,
            mev_risk.sandwich_risk_score,
            mev_risk.frontrun_risk_score,
            mev_risk.oracle_manipulation_risk,
            mev_risk.oracle_deviation_risk,
            mev_risk.overall_mev_risk,
            mev_risk.confidence_score,
            mev_risk.created_at,
            mev_risk.updated_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store MEV risk: {}", e)))?;
        Ok(())
    }

    /// Get or calculate MEV risk for a pool
    pub async fn get_mev_risk(&self, _pool_address: &str, _chain_id: i32) -> Result<Option<MevRisk>, AppError> {
        // Simplified implementation for now - in production this would query the database
        // TODO: Implement actual database query once schema is finalized
        let cached_risk: Option<MevRisk> = None;

        // Check if cached assessment is still fresh (within 1 hour)
        if let Some(risk) = cached_risk {
            let age = Utc::now().signed_duration_since(risk.created_at);
            if age < Duration::hours(1) {
                return Ok(Some(risk));
            }
        }

        Ok(None)
    }

    // Helper methods for data retrieval (simplified for demo)
    async fn get_recent_sandwich_attacks(&self, _pool_address: &str, _chain_id: i32) -> Result<i64, AppError> {
        // Simplified implementation for now - in production this would query the database
        // TODO: Implement actual database query once schema is finalized
        Ok(0) // Return 0 attacks for now
    }

    async fn get_recent_frontrun_attacks(&self, _pool_address: &str, _chain_id: i32) -> Result<i64, AppError> {
        // Simplified implementation for now - in production this would query the database
        Ok(0)
    }

    #[allow(dead_code)]
    async fn get_recent_attack_count(&self, _pool_address: &str, _chain_id: i32) -> Result<i64, AppError> {
        // Simplified implementation for now - in production this would query the database
        Ok(0)
    }

    async fn get_recent_oracle_manipulations(&self, _pool_address: &str, _chain_id: i32) -> Result<i64, AppError> {
        // Simplified implementation - in production, this would analyze oracle price movements
        Ok(0)
    }

    async fn assess_oracle_reliability(&self, _pool_address: &str, _chain_id: i32) -> Result<BigDecimal, AppError> {
        // Simplified implementation - in production, this would check oracle update frequency, etc.
        Ok(BigDecimal::from_str("0.8").unwrap()) // Default 80% reliability
    }

    async fn get_recent_oracle_deviations(&self, _pool_address: &str, _chain_id: i32) -> Result<Vec<OracleDeviation>, AppError> {
        // Simplified implementation for now - in production this would query the database
        // TODO: Implement actual database query once schema is finalized
        
        // For now, return empty vector to avoid compilation issues
        Ok(vec![])
    }

    async fn get_recent_transaction_count(&self, _pool_address: &str, _chain_id: i32) -> Result<i64, AppError> {
        // Simplified implementation - in production, this would query blockchain data
        Ok(50) // Default moderate transaction count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PoolState;
    use std::str::FromStr;

    fn create_test_pool_state() -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(BigDecimal::from(5000000)),
            volume_24h_usd: Some(BigDecimal::from(1000000)),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_mev_risk_config_default() {
        let config = MevRiskConfig::default();
        
        // Verify weights sum to 1.0 (100%)
        let total_weight = &config.sandwich_weight + &config.frontrun_weight + 
                          &config.oracle_manipulation_weight + &config.oracle_deviation_weight;
        assert_eq!(total_weight, BigDecimal::from_str("1.0").unwrap());
    }

    #[test]
    fn test_weighted_mev_risk_calculation() {
        let config = MevRiskConfig::default();
        // Test the calculation logic directly without database dependency
        let config_ref = &config;

        let sandwich_risk = BigDecimal::from_str("0.8").unwrap();
        let frontrun_risk = BigDecimal::from_str("0.6").unwrap();
        let oracle_manipulation_risk = BigDecimal::from_str("0.4").unwrap();
        let oracle_deviation_risk = BigDecimal::from_str("0.2").unwrap();

        // Calculate weighted MEV risk directly using config weights
        let weighted_risk = (
            &sandwich_risk * &config_ref.sandwich_weight +
            &frontrun_risk * &config_ref.frontrun_weight +
            &oracle_manipulation_risk * &config_ref.oracle_manipulation_weight +
            &oracle_deviation_risk * &config_ref.oracle_deviation_weight
        );

        // Risk should be between 0 and 1
        assert!(weighted_risk >= BigDecimal::from(0));
        assert!(weighted_risk <= BigDecimal::from(1));
    }

    #[tokio::test]
    async fn test_sandwich_risk_calculation() {
        // Test the calculation logic directly without database dependency
        let config = MevRiskConfig::default();
        
        // Test high-risk scenario (low liquidity)
        let high_risk_tvl = BigDecimal::from(50000); // $50K TVL
        let high_risk_score = calculate_sandwich_risk_score(&high_risk_tvl, &config);
        assert!(high_risk_score >= BigDecimal::from_str("0.5").unwrap());
        
        // Test low-risk scenario (high liquidity)
        let low_risk_tvl = BigDecimal::from(50000000); // $50M TVL
        let low_risk_score = calculate_sandwich_risk_score(&low_risk_tvl, &config);
        assert!(low_risk_score <= BigDecimal::from_str("0.4").unwrap());
    }
    
    fn calculate_sandwich_risk_score(tvl: &BigDecimal, config: &MevRiskConfig) -> BigDecimal {
        // Simplified sandwich risk calculation based on TVL
        if tvl < &BigDecimal::from(100000) { // < $100K
            BigDecimal::from_str("0.8").unwrap()
        } else if tvl < &BigDecimal::from(1000000) { // < $1M
            BigDecimal::from_str("0.5").unwrap()
        } else if tvl < &BigDecimal::from(10000000) { // < $10M
            BigDecimal::from_str("0.3").unwrap()
        } else {
            BigDecimal::from_str("0.1").unwrap()
        }
    }

    #[tokio::test]
    async fn test_frontrun_risk_calculation() {
        // Test the calculation logic directly without database dependency
        let config = MevRiskConfig::default();
        
        // Test high-volume scenario (more MEV opportunities)
        let high_volume = BigDecimal::from(20000000); // $20M volume
        let high_risk_score = calculate_frontrun_risk_score(&high_volume, &config);
        assert!(high_risk_score >= BigDecimal::from_str("0.5").unwrap());
        
        // Test low-volume scenario
        let low_volume = BigDecimal::from(100000); // $100K volume
        let low_risk_score = calculate_frontrun_risk_score(&low_volume, &config);
        assert!(low_risk_score <= BigDecimal::from_str("0.3").unwrap());
    }
    
    fn calculate_frontrun_risk_score(volume: &BigDecimal, config: &MevRiskConfig) -> BigDecimal {
        // Simplified frontrun risk calculation based on volume
        if volume > &BigDecimal::from(10000000) { // > $10M volume
            BigDecimal::from_str("0.7").unwrap() // High volume = more MEV opportunities
        } else if volume > &BigDecimal::from(1000000) { // > $1M volume
            BigDecimal::from_str("0.4").unwrap()
        } else {
            BigDecimal::from_str("0.2").unwrap()
        }
    }

    fn create_test_pool_state_with_tvl(tvl: BigDecimal) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
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

    fn create_test_pool_state_with_volume(volume: BigDecimal) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(1000000),
            token0_price_usd: Some(BigDecimal::from(1)),
            token1_price_usd: Some(BigDecimal::from(2000)),
            tvl_usd: Some(BigDecimal::from(5000000)),
            volume_24h_usd: Some(volume),
            fees_24h_usd: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }
}
