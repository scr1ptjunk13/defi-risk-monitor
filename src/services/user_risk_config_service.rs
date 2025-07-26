use sqlx::PgPool;
use uuid::Uuid;
use std::collections::HashMap;
use bigdecimal::BigDecimal;
use crate::models::{UserRiskConfig, CreateUserRiskConfig, UpdateUserRiskConfig, RiskToleranceLevel};
use crate::error::AppError;

/// Service for managing user risk configurations
pub struct UserRiskConfigService {
    db_pool: PgPool,
}

impl UserRiskConfigService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }
    
    /// Create a new user risk configuration
    pub async fn create_config(&self, create_config: CreateUserRiskConfig) -> Result<UserRiskConfig, AppError> {
        let config = UserRiskConfig::new(create_config);
        
        // Validate weights before saving
        config.validate_weights()
            .map_err(|e| AppError::ValidationError(e))?;
        
        let result = sqlx::query_as!(
            UserRiskConfig,
            r#"
            INSERT INTO user_risk_configs (
                id, user_address, profile_name, is_active, risk_tolerance_level,
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight, 
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9, $10,
                $11, $12, $13, $14,
                $15, $16, $17,
                $18, $19, $20,
                $21, $22, $23,
                $24, $25, $26,
                $27, $28, $29
            )
            RETURNING 
                id, user_address, profile_name, is_active, 
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            "#,
            config.id,
            config.user_address,
            config.profile_name,
            config.is_active,
            config.risk_tolerance_level as RiskToleranceLevel,
            config.liquidity_risk_weight,
            config.volatility_risk_weight,
            config.protocol_risk_weight,
            config.mev_risk_weight,
            config.cross_chain_risk_weight,
            config.min_tvl_threshold,
            config.max_slippage_tolerance,
            config.thin_pool_threshold,
            config.tvl_drop_threshold,
            config.volatility_lookback_days,
            config.high_volatility_threshold,
            config.correlation_threshold,
            config.min_audit_score,
            config.max_exploit_tolerance,
            config.governance_risk_weight,
            config.sandwich_attack_threshold,
            config.frontrun_threshold,
            config.oracle_deviation_threshold,
            config.bridge_risk_tolerance,
            config.liquidity_fragmentation_threshold,
            config.governance_divergence_threshold,
            config.overall_risk_threshold,
            config.created_at,
            config.updated_at
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Get user's active risk configuration
    pub async fn get_active_config(&self, user_address: &str) -> Result<Option<UserRiskConfig>, AppError> {
        let result = sqlx::query_as!(
            UserRiskConfig,
            r#"
            SELECT 
                id, user_address, profile_name, is_active,
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            FROM user_risk_configs 
            WHERE user_address = $1 AND is_active = true
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
            user_address
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Get all risk configurations for a user
    pub async fn get_user_configs(&self, user_address: &str) -> Result<Vec<UserRiskConfig>, AppError> {
        let results = sqlx::query_as!(
            UserRiskConfig,
            r#"
            SELECT 
                id, user_address, profile_name, is_active,
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            FROM user_risk_configs 
            WHERE user_address = $1
            ORDER BY is_active DESC, updated_at DESC
            "#,
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(results)
    }
    
    /// Get a specific risk configuration by ID
    pub async fn get_config(&self, config_id: Uuid) -> Result<Option<UserRiskConfig>, AppError> {
        let result = sqlx::query_as!(
            UserRiskConfig,
            r#"
            SELECT 
                id, user_address, profile_name, is_active,
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            FROM user_risk_configs 
            WHERE id = $1
            "#,
            config_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Update a risk configuration
    pub async fn update_config(&self, config_id: Uuid, update_config: UpdateUserRiskConfig) -> Result<UserRiskConfig, AppError> {
        // First get the existing config
        let mut existing = self.get_config(config_id).await?
            .ok_or_else(|| AppError::NotFound("Risk configuration not found".to_string()))?;
        
        // Apply updates
        if let Some(profile_name) = update_config.profile_name {
            existing.profile_name = profile_name;
        }
        if let Some(is_active) = update_config.is_active {
            existing.is_active = is_active;
        }
        if let Some(risk_tolerance_level) = update_config.risk_tolerance_level {
            existing.risk_tolerance_level = risk_tolerance_level;
        }
        
        // Update risk weights
        if let Some(weight) = update_config.liquidity_risk_weight {
            existing.liquidity_risk_weight = weight;
        }
        if let Some(weight) = update_config.volatility_risk_weight {
            existing.volatility_risk_weight = weight;
        }
        if let Some(weight) = update_config.protocol_risk_weight {
            existing.protocol_risk_weight = weight;
        }
        if let Some(weight) = update_config.mev_risk_weight {
            existing.mev_risk_weight = weight;
        }
        if let Some(weight) = update_config.cross_chain_risk_weight {
            existing.cross_chain_risk_weight = weight;
        }
        
        // Update liquidity parameters
        if let Some(threshold) = update_config.min_tvl_threshold {
            existing.min_tvl_threshold = threshold;
        }
        if let Some(tolerance) = update_config.max_slippage_tolerance {
            existing.max_slippage_tolerance = tolerance;
        }
        if let Some(threshold) = update_config.thin_pool_threshold {
            existing.thin_pool_threshold = threshold;
        }
        if let Some(threshold) = update_config.tvl_drop_threshold {
            existing.tvl_drop_threshold = threshold;
        }
        
        // Update volatility parameters
        if let Some(days) = update_config.volatility_lookback_days {
            existing.volatility_lookback_days = days;
        }
        if let Some(threshold) = update_config.high_volatility_threshold {
            existing.high_volatility_threshold = threshold;
        }
        if let Some(threshold) = update_config.correlation_threshold {
            existing.correlation_threshold = threshold;
        }
        
        // Update protocol parameters
        if let Some(score) = update_config.min_audit_score {
            existing.min_audit_score = score;
        }
        if let Some(tolerance) = update_config.max_exploit_tolerance {
            existing.max_exploit_tolerance = tolerance;
        }
        if let Some(weight) = update_config.governance_risk_weight {
            existing.governance_risk_weight = weight;
        }
        
        // Update MEV parameters
        if let Some(threshold) = update_config.sandwich_attack_threshold {
            existing.sandwich_attack_threshold = threshold;
        }
        if let Some(threshold) = update_config.frontrun_threshold {
            existing.frontrun_threshold = threshold;
        }
        if let Some(threshold) = update_config.oracle_deviation_threshold {
            existing.oracle_deviation_threshold = threshold;
        }
        
        // Update cross-chain parameters
        if let Some(tolerance) = update_config.bridge_risk_tolerance {
            existing.bridge_risk_tolerance = tolerance;
        }
        if let Some(threshold) = update_config.liquidity_fragmentation_threshold {
            existing.liquidity_fragmentation_threshold = threshold;
        }
        if let Some(threshold) = update_config.governance_divergence_threshold {
            existing.governance_divergence_threshold = threshold;
        }
        
        // Update overall threshold
        if let Some(threshold) = update_config.overall_risk_threshold {
            existing.overall_risk_threshold = threshold;
        }
        
        // Validate weights
        existing.validate_weights()
            .map_err(|e| AppError::ValidationError(e))?;
        
        // Update in database
        let result = sqlx::query_as!(
            UserRiskConfig,
            r#"
            UPDATE user_risk_configs SET
                profile_name = $2,
                is_active = $3,
                risk_tolerance_level = $4,
                liquidity_risk_weight = $5,
                volatility_risk_weight = $6,
                protocol_risk_weight = $7,
                mev_risk_weight = $8,
                cross_chain_risk_weight = $9,
                min_tvl_threshold = $10,
                max_slippage_tolerance = $11,
                thin_pool_threshold = $12,
                tvl_drop_threshold = $13,
                volatility_lookback_days = $14,
                high_volatility_threshold = $15,
                correlation_threshold = $16,
                min_audit_score = $17,
                max_exploit_tolerance = $18,
                governance_risk_weight = $19,
                sandwich_attack_threshold = $20,
                frontrun_threshold = $21,
                oracle_deviation_threshold = $22,
                bridge_risk_tolerance = $23,
                liquidity_fragmentation_threshold = $24,
                governance_divergence_threshold = $25,
                overall_risk_threshold = $26,
                updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_address, profile_name, is_active,
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            "#,
            config_id,
            existing.profile_name,
            existing.is_active,
            existing.risk_tolerance_level as RiskToleranceLevel,
            existing.liquidity_risk_weight,
            existing.volatility_risk_weight,
            existing.protocol_risk_weight,
            existing.mev_risk_weight,
            existing.cross_chain_risk_weight,
            existing.min_tvl_threshold,
            existing.max_slippage_tolerance,
            existing.thin_pool_threshold,
            existing.tvl_drop_threshold,
            existing.volatility_lookback_days,
            existing.high_volatility_threshold,
            existing.correlation_threshold,
            existing.min_audit_score,
            existing.max_exploit_tolerance,
            existing.governance_risk_weight,
            existing.sandwich_attack_threshold,
            existing.frontrun_threshold,
            existing.oracle_deviation_threshold,
            existing.bridge_risk_tolerance,
            existing.liquidity_fragmentation_threshold,
            existing.governance_divergence_threshold,
            existing.overall_risk_threshold
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Delete a risk configuration
    pub async fn delete_config(&self, config_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            "DELETE FROM user_risk_configs WHERE id = $1",
            config_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Risk configuration not found".to_string()));
        }
        
        Ok(())
    }
    
    /// Set a configuration as active (and deactivate others for the user)
    pub async fn set_active_config(&self, config_id: Uuid) -> Result<UserRiskConfig, AppError> {
        // First get the config to get the user address
        let config = self.get_config(config_id).await?
            .ok_or_else(|| AppError::NotFound("Risk configuration not found".to_string()))?;
        
        // Start a transaction
        let mut tx = self.db_pool.begin().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // Deactivate all configs for this user
        sqlx::query!(
            "UPDATE user_risk_configs SET is_active = false WHERE user_address = $1",
            config.user_address
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // Activate the specified config
        let result = sqlx::query_as!(
            UserRiskConfig,
            r#"
            UPDATE user_risk_configs SET is_active = true, updated_at = NOW()
            WHERE id = $1
            RETURNING 
                id, user_address, profile_name, is_active,
                risk_tolerance_level as "risk_tolerance_level: RiskToleranceLevel",
                liquidity_risk_weight, volatility_risk_weight, protocol_risk_weight,
                mev_risk_weight, cross_chain_risk_weight,
                min_tvl_threshold, max_slippage_tolerance, thin_pool_threshold, tvl_drop_threshold,
                volatility_lookback_days, high_volatility_threshold, correlation_threshold,
                min_audit_score, max_exploit_tolerance, governance_risk_weight,
                sandwich_attack_threshold, frontrun_threshold, oracle_deviation_threshold,
                bridge_risk_tolerance, liquidity_fragmentation_threshold, governance_divergence_threshold,
                overall_risk_threshold, created_at, updated_at
            "#,
            config_id
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // Commit transaction
        tx.commit().await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(result)
    }
    
    /// Initialize default risk configurations for a new user
    pub async fn initialize_default_configs(&self, user_address: &str) -> Result<Vec<UserRiskConfig>, AppError> {
        let mut configs = Vec::new();
        
        // Create conservative config
        let conservative = CreateUserRiskConfig {
            user_address: user_address.to_string(),
            profile_name: "Conservative".to_string(),
            risk_tolerance_level: RiskToleranceLevel::Conservative,
            liquidity_risk_weight: None,
            volatility_risk_weight: None,
            protocol_risk_weight: None,
            mev_risk_weight: None,
            cross_chain_risk_weight: None,
            min_tvl_threshold: None,
            max_slippage_tolerance: None,
            thin_pool_threshold: None,
            tvl_drop_threshold: None,
            volatility_lookback_days: None,
            high_volatility_threshold: None,
            correlation_threshold: None,
            min_audit_score: None,
            max_exploit_tolerance: None,
            governance_risk_weight: None,
            sandwich_attack_threshold: None,
            frontrun_threshold: None,
            oracle_deviation_threshold: None,
            bridge_risk_tolerance: None,
            liquidity_fragmentation_threshold: None,
            governance_divergence_threshold: None,
            overall_risk_threshold: None,
        };
        configs.push(self.create_config(conservative).await?);
        
        // Create moderate config (set as active)
        let moderate = CreateUserRiskConfig {
            user_address: user_address.to_string(),
            profile_name: "Moderate".to_string(),
            risk_tolerance_level: RiskToleranceLevel::Moderate,
            liquidity_risk_weight: None,
            volatility_risk_weight: None,
            protocol_risk_weight: None,
            mev_risk_weight: None,
            cross_chain_risk_weight: None,
            min_tvl_threshold: None,
            max_slippage_tolerance: None,
            thin_pool_threshold: None,
            tvl_drop_threshold: None,
            volatility_lookback_days: None,
            high_volatility_threshold: None,
            correlation_threshold: None,
            min_audit_score: None,
            max_exploit_tolerance: None,
            governance_risk_weight: None,
            sandwich_attack_threshold: None,
            frontrun_threshold: None,
            oracle_deviation_threshold: None,
            bridge_risk_tolerance: None,
            liquidity_fragmentation_threshold: None,
            governance_divergence_threshold: None,
            overall_risk_threshold: None,
        };
        let moderate_config = self.create_config(moderate).await?;
        configs.push(self.set_active_config(moderate_config.id).await?);
        
        // Create aggressive config
        let aggressive = CreateUserRiskConfig {
            user_address: user_address.to_string(),
            profile_name: "Aggressive".to_string(),
            risk_tolerance_level: RiskToleranceLevel::Aggressive,
            liquidity_risk_weight: None,
            volatility_risk_weight: None,
            protocol_risk_weight: None,
            mev_risk_weight: None,
            cross_chain_risk_weight: None,
            min_tvl_threshold: None,
            max_slippage_tolerance: None,
            thin_pool_threshold: None,
            tvl_drop_threshold: None,
            volatility_lookback_days: None,
            high_volatility_threshold: None,
            correlation_threshold: None,
            min_audit_score: None,
            max_exploit_tolerance: None,
            governance_risk_weight: None,
            sandwich_attack_threshold: None,
            frontrun_threshold: None,
            oracle_deviation_threshold: None,
            bridge_risk_tolerance: None,
            liquidity_fragmentation_threshold: None,
            governance_divergence_threshold: None,
            overall_risk_threshold: None,
        };
        configs.push(self.create_config(aggressive).await?);
        
        Ok(configs)
    }
    
    /// Get risk parameters for calculation (returns active config or defaults)
    pub async fn get_risk_params(&self, user_address: &str) -> Result<HashMap<String, BigDecimal>, AppError> {
        if let Some(config) = self.get_active_config(user_address).await? {
            Ok(config.to_risk_params())
        } else {
            // Return moderate defaults if no config exists
            let default_config = UserRiskConfig::get_defaults_for_tolerance(&RiskToleranceLevel::Moderate);
            Ok(default_config.to_risk_params())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Note: These tests would require a test database setup
    // For now, we'll test the configuration logic without database calls
    
    #[test]
    fn test_user_risk_config_service_creation() {
        // This would require a mock pool for proper testing
        // let service = UserRiskConfigService::new(mock_pool);
        // assert!(service is properly constructed);
    }
}
