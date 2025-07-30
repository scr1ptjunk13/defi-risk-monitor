use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::collections::HashMap;

/// User-specific risk calculation configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRiskConfig {
    pub id: Uuid,
    pub user_address: String,
    pub profile_name: String,
    pub is_active: bool,
    
    // Risk calculation weights (must sum to 1.0)
    pub liquidity_risk_weight: BigDecimal,
    pub volatility_risk_weight: BigDecimal,
    pub protocol_risk_weight: BigDecimal,
    pub mev_risk_weight: BigDecimal,
    pub cross_chain_risk_weight: BigDecimal,
    
    // Liquidity risk parameters
    pub min_tvl_threshold: BigDecimal,
    pub max_slippage_tolerance: BigDecimal,
    pub thin_pool_threshold: BigDecimal,
    pub tvl_drop_threshold: BigDecimal,
    
    // Volatility risk parameters
    pub volatility_lookback_days: i32,
    pub high_volatility_threshold: BigDecimal,
    pub correlation_threshold: BigDecimal,
    
    // Protocol risk parameters
    pub min_audit_score: BigDecimal,
    pub max_exploit_tolerance: i32,
    pub governance_risk_weight: BigDecimal,
    
    // MEV risk parameters
    pub sandwich_attack_threshold: BigDecimal,
    pub frontrun_threshold: BigDecimal,
    pub oracle_deviation_threshold: BigDecimal,
    
    // Cross-chain risk parameters
    pub bridge_risk_tolerance: BigDecimal,
    pub liquidity_fragmentation_threshold: BigDecimal,
    pub governance_divergence_threshold: BigDecimal,
    
    // Overall risk calculation
    pub overall_risk_threshold: BigDecimal,
    pub risk_tolerance_level: RiskToleranceLevel,
    
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Risk tolerance levels for easy user selection
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "risk_tolerance_level", rename_all = "lowercase")]
pub enum RiskToleranceLevel {
    Conservative,
    Moderate,
    Aggressive,
    Custom,
}

/// Create new user risk configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUserRiskConfig {
    pub user_address: String,
    pub profile_name: String,
    pub risk_tolerance_level: RiskToleranceLevel,
    
    // Optional custom parameters (if not provided, defaults based on tolerance level)
    pub liquidity_risk_weight: Option<BigDecimal>,
    pub volatility_risk_weight: Option<BigDecimal>,
    pub protocol_risk_weight: Option<BigDecimal>,
    pub mev_risk_weight: Option<BigDecimal>,
    pub cross_chain_risk_weight: Option<BigDecimal>,
    
    pub min_tvl_threshold: Option<BigDecimal>,
    pub max_slippage_tolerance: Option<BigDecimal>,
    pub thin_pool_threshold: Option<BigDecimal>,
    pub tvl_drop_threshold: Option<BigDecimal>,
    
    pub volatility_lookback_days: Option<i32>,
    pub high_volatility_threshold: Option<BigDecimal>,
    pub correlation_threshold: Option<BigDecimal>,
    
    pub min_audit_score: Option<BigDecimal>,
    pub max_exploit_tolerance: Option<i32>,
    pub governance_risk_weight: Option<BigDecimal>,
    
    pub sandwich_attack_threshold: Option<BigDecimal>,
    pub frontrun_threshold: Option<BigDecimal>,
    pub oracle_deviation_threshold: Option<BigDecimal>,
    
    pub bridge_risk_tolerance: Option<BigDecimal>,
    pub liquidity_fragmentation_threshold: Option<BigDecimal>,
    pub governance_divergence_threshold: Option<BigDecimal>,
    
    pub overall_risk_threshold: Option<BigDecimal>,
}

/// Update user risk configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUserRiskConfig {
    pub profile_name: Option<String>,
    pub is_active: Option<bool>,
    pub risk_tolerance_level: Option<RiskToleranceLevel>,
    
    pub liquidity_risk_weight: Option<BigDecimal>,
    pub volatility_risk_weight: Option<BigDecimal>,
    pub protocol_risk_weight: Option<BigDecimal>,
    pub mev_risk_weight: Option<BigDecimal>,
    pub cross_chain_risk_weight: Option<BigDecimal>,
    
    pub min_tvl_threshold: Option<BigDecimal>,
    pub max_slippage_tolerance: Option<BigDecimal>,
    pub thin_pool_threshold: Option<BigDecimal>,
    pub tvl_drop_threshold: Option<BigDecimal>,
    
    pub volatility_lookback_days: Option<i32>,
    pub high_volatility_threshold: Option<BigDecimal>,
    pub correlation_threshold: Option<BigDecimal>,
    
    pub min_audit_score: Option<BigDecimal>,
    pub max_exploit_tolerance: Option<i32>,
    pub governance_risk_weight: Option<BigDecimal>,
    
    pub sandwich_attack_threshold: Option<BigDecimal>,
    pub frontrun_threshold: Option<BigDecimal>,
    pub oracle_deviation_threshold: Option<BigDecimal>,
    
    pub bridge_risk_tolerance: Option<BigDecimal>,
    pub liquidity_fragmentation_threshold: Option<BigDecimal>,
    pub governance_divergence_threshold: Option<BigDecimal>,
    
    pub overall_risk_threshold: Option<BigDecimal>,
}

impl UserRiskConfig {
    /// Create a new user risk configuration with defaults based on tolerance level
    pub fn new(create_config: CreateUserRiskConfig) -> Self {
        let now = Utc::now();
        let defaults = Self::get_defaults_for_tolerance(&create_config.risk_tolerance_level);
        
        Self {
            id: Uuid::new_v4(),
            user_address: create_config.user_address,
            profile_name: create_config.profile_name,
            is_active: true,
            risk_tolerance_level: create_config.risk_tolerance_level,
            
            // Use provided values or defaults
            liquidity_risk_weight: create_config.liquidity_risk_weight.unwrap_or(defaults.liquidity_risk_weight),
            volatility_risk_weight: create_config.volatility_risk_weight.unwrap_or(defaults.volatility_risk_weight),
            protocol_risk_weight: create_config.protocol_risk_weight.unwrap_or(defaults.protocol_risk_weight),
            mev_risk_weight: create_config.mev_risk_weight.unwrap_or(defaults.mev_risk_weight),
            cross_chain_risk_weight: create_config.cross_chain_risk_weight.unwrap_or(defaults.cross_chain_risk_weight),
            
            min_tvl_threshold: create_config.min_tvl_threshold.unwrap_or(defaults.min_tvl_threshold),
            max_slippage_tolerance: create_config.max_slippage_tolerance.unwrap_or(defaults.max_slippage_tolerance),
            thin_pool_threshold: create_config.thin_pool_threshold.unwrap_or(defaults.thin_pool_threshold),
            tvl_drop_threshold: create_config.tvl_drop_threshold.unwrap_or(defaults.tvl_drop_threshold),
            
            volatility_lookback_days: create_config.volatility_lookback_days.unwrap_or(defaults.volatility_lookback_days),
            high_volatility_threshold: create_config.high_volatility_threshold.unwrap_or(defaults.high_volatility_threshold),
            correlation_threshold: create_config.correlation_threshold.unwrap_or(defaults.correlation_threshold),
            
            min_audit_score: create_config.min_audit_score.unwrap_or(defaults.min_audit_score),
            max_exploit_tolerance: create_config.max_exploit_tolerance.unwrap_or(defaults.max_exploit_tolerance),
            governance_risk_weight: create_config.governance_risk_weight.unwrap_or(defaults.governance_risk_weight),
            
            sandwich_attack_threshold: create_config.sandwich_attack_threshold.unwrap_or(defaults.sandwich_attack_threshold),
            frontrun_threshold: create_config.frontrun_threshold.unwrap_or(defaults.frontrun_threshold),
            oracle_deviation_threshold: create_config.oracle_deviation_threshold.unwrap_or(defaults.oracle_deviation_threshold),
            
            bridge_risk_tolerance: create_config.bridge_risk_tolerance.unwrap_or(defaults.bridge_risk_tolerance),
            liquidity_fragmentation_threshold: create_config.liquidity_fragmentation_threshold.unwrap_or(defaults.liquidity_fragmentation_threshold),
            governance_divergence_threshold: create_config.governance_divergence_threshold.unwrap_or(defaults.governance_divergence_threshold),
            
            overall_risk_threshold: create_config.overall_risk_threshold.unwrap_or(defaults.overall_risk_threshold),
            
            created_at: now,
            updated_at: now,
        }
    }
    
    /// Get default configuration values based on risk tolerance level
    pub fn get_defaults_for_tolerance(tolerance: &RiskToleranceLevel) -> Self {
        use std::str::FromStr;
        let now = Utc::now();
        
        match tolerance {
            RiskToleranceLevel::Conservative => Self {
                id: Uuid::new_v4(),
                user_address: String::new(),
                profile_name: "Conservative".to_string(),
                is_active: true,
                risk_tolerance_level: RiskToleranceLevel::Conservative,
                
                // Conservative: Higher weight on safety factors
                liquidity_risk_weight: BigDecimal::from_str("0.30").unwrap(), // 30%
                volatility_risk_weight: BigDecimal::from_str("0.25").unwrap(), // 25%
                protocol_risk_weight: BigDecimal::from_str("0.20").unwrap(),   // 20%
                mev_risk_weight: BigDecimal::from_str("0.15").unwrap(),        // 15%
                cross_chain_risk_weight: BigDecimal::from_str("0.10").unwrap(), // 10%
                
                // Conservative liquidity thresholds
                min_tvl_threshold: BigDecimal::from_str("10000000").unwrap(), // $10M minimum
                max_slippage_tolerance: BigDecimal::from_str("0.01").unwrap(), // 1% max slippage
                thin_pool_threshold: BigDecimal::from_str("0.8").unwrap(),     // 80% concentration
                tvl_drop_threshold: BigDecimal::from_str("0.20").unwrap(),     // 20% TVL drop
                
                // Conservative volatility settings
                volatility_lookback_days: 30,
                high_volatility_threshold: BigDecimal::from_str("0.15").unwrap(), // 15% volatility
                correlation_threshold: BigDecimal::from_str("0.7").unwrap(),       // 70% correlation
                
                // Conservative protocol requirements
                min_audit_score: BigDecimal::from_str("0.8").unwrap(),  // 80% audit score
                max_exploit_tolerance: 0,                               // No exploits
                governance_risk_weight: BigDecimal::from_str("0.4").unwrap(), // 40% governance weight
                
                // Conservative MEV protection
                sandwich_attack_threshold: BigDecimal::from_str("0.005").unwrap(), // 0.5%
                frontrun_threshold: BigDecimal::from_str("0.01").unwrap(),         // 1%
                oracle_deviation_threshold: BigDecimal::from_str("0.02").unwrap(), // 2%
                
                // Conservative cross-chain settings
                bridge_risk_tolerance: BigDecimal::from_str("0.1").unwrap(),              // 10%
                liquidity_fragmentation_threshold: BigDecimal::from_str("0.3").unwrap(),  // 30%
                governance_divergence_threshold: BigDecimal::from_str("0.2").unwrap(),    // 20%
                
                overall_risk_threshold: BigDecimal::from_str("0.3").unwrap(), // 30% overall risk
                
                created_at: now,
                updated_at: now,
            },
            
            RiskToleranceLevel::Moderate => Self {
                id: Uuid::new_v4(),
                user_address: String::new(),
                profile_name: "Moderate".to_string(),
                is_active: true,
                risk_tolerance_level: RiskToleranceLevel::Moderate,
                
                // Moderate: Balanced approach
                liquidity_risk_weight: BigDecimal::from_str("0.25").unwrap(),   // 25%
                volatility_risk_weight: BigDecimal::from_str("0.20").unwrap(),  // 20%
                protocol_risk_weight: BigDecimal::from_str("0.20").unwrap(),    // 20%
                mev_risk_weight: BigDecimal::from_str("0.20").unwrap(),         // 20%
                cross_chain_risk_weight: BigDecimal::from_str("0.15").unwrap(), // 15%
                
                // Moderate liquidity thresholds
                min_tvl_threshold: BigDecimal::from_str("1000000").unwrap(),  // $1M minimum
                max_slippage_tolerance: BigDecimal::from_str("0.03").unwrap(), // 3% max slippage
                thin_pool_threshold: BigDecimal::from_str("0.6").unwrap(),     // 60% concentration
                tvl_drop_threshold: BigDecimal::from_str("0.40").unwrap(),     // 40% TVL drop
                
                // Moderate volatility settings
                volatility_lookback_days: 14,
                high_volatility_threshold: BigDecimal::from_str("0.30").unwrap(), // 30% volatility
                correlation_threshold: BigDecimal::from_str("0.5").unwrap(),       // 50% correlation
                
                // Moderate protocol requirements
                min_audit_score: BigDecimal::from_str("0.6").unwrap(),  // 60% audit score
                max_exploit_tolerance: 1,                               // 1 minor exploit
                governance_risk_weight: BigDecimal::from_str("0.3").unwrap(), // 30% governance weight
                
                // Moderate MEV protection
                sandwich_attack_threshold: BigDecimal::from_str("0.02").unwrap(),  // 2%
                frontrun_threshold: BigDecimal::from_str("0.03").unwrap(),         // 3%
                oracle_deviation_threshold: BigDecimal::from_str("0.05").unwrap(), // 5%
                
                // Moderate cross-chain settings
                bridge_risk_tolerance: BigDecimal::from_str("0.3").unwrap(),              // 30%
                liquidity_fragmentation_threshold: BigDecimal::from_str("0.5").unwrap(),  // 50%
                governance_divergence_threshold: BigDecimal::from_str("0.4").unwrap(),    // 40%
                
                overall_risk_threshold: BigDecimal::from_str("0.6").unwrap(), // 60% overall risk
                
                created_at: now,
                updated_at: now,
            },
            
            RiskToleranceLevel::Aggressive => Self {
                id: Uuid::new_v4(),
                user_address: String::new(),
                profile_name: "Aggressive".to_string(),
                is_active: true,
                risk_tolerance_level: RiskToleranceLevel::Aggressive,
                
                // Aggressive: Higher risk tolerance
                liquidity_risk_weight: BigDecimal::from_str("0.15").unwrap(),   // 15%
                volatility_risk_weight: BigDecimal::from_str("0.15").unwrap(),  // 15%
                protocol_risk_weight: BigDecimal::from_str("0.15").unwrap(),    // 15%
                mev_risk_weight: BigDecimal::from_str("0.25").unwrap(),         // 25%
                cross_chain_risk_weight: BigDecimal::from_str("0.30").unwrap(), // 30%
                
                // Aggressive liquidity thresholds
                min_tvl_threshold: BigDecimal::from_str("100000").unwrap(),    // $100K minimum
                max_slippage_tolerance: BigDecimal::from_str("0.10").unwrap(), // 10% max slippage
                thin_pool_threshold: BigDecimal::from_str("0.3").unwrap(),     // 30% concentration
                tvl_drop_threshold: BigDecimal::from_str("0.70").unwrap(),     // 70% TVL drop
                
                // Aggressive volatility settings
                volatility_lookback_days: 7,
                high_volatility_threshold: BigDecimal::from_str("0.60").unwrap(), // 60% volatility
                correlation_threshold: BigDecimal::from_str("0.2").unwrap(),       // 20% correlation
                
                // Aggressive protocol requirements
                min_audit_score: BigDecimal::from_str("0.3").unwrap(),  // 30% audit score
                max_exploit_tolerance: 3,                               // 3 exploits allowed
                governance_risk_weight: BigDecimal::from_str("0.1").unwrap(), // 10% governance weight
                
                // Aggressive MEV tolerance
                sandwich_attack_threshold: BigDecimal::from_str("0.10").unwrap(), // 10%
                frontrun_threshold: BigDecimal::from_str("0.15").unwrap(),        // 15%
                oracle_deviation_threshold: BigDecimal::from_str("0.20").unwrap(), // 20%
                
                // Aggressive cross-chain settings
                bridge_risk_tolerance: BigDecimal::from_str("0.7").unwrap(),              // 70%
                liquidity_fragmentation_threshold: BigDecimal::from_str("0.8").unwrap(),  // 80%
                governance_divergence_threshold: BigDecimal::from_str("0.7").unwrap(),    // 70%
                
                overall_risk_threshold: BigDecimal::from_str("0.9").unwrap(), // 90% overall risk
                
                created_at: now,
                updated_at: now,
            },
            
            RiskToleranceLevel::Custom => {
                // Custom defaults to moderate settings, user will override
                Self::get_defaults_for_tolerance(&RiskToleranceLevel::Moderate)
            }
        }
    }
    
    /// Validate that risk weights sum to approximately 1.0
    pub fn validate_weights(&self) -> Result<(), String> {
        use std::str::FromStr;
        
        let total_weight = &self.liquidity_risk_weight + 
                          &self.volatility_risk_weight + 
                          &self.protocol_risk_weight + 
                          &self.mev_risk_weight + 
                          &self.cross_chain_risk_weight;
        
        let one = BigDecimal::from_str("1.0").unwrap();
        let tolerance = BigDecimal::from_str("0.01").unwrap(); // 1% tolerance
        
        if (total_weight.clone() - &one).abs() > tolerance {
            return Err(format!("Risk weights must sum to 1.0, got: {}", total_weight));
        }
        
        Ok(())
    }
    
    /// Get risk configuration as a HashMap for easy parameter passing
    pub fn to_risk_params(&self) -> HashMap<String, BigDecimal> {
        let mut params = HashMap::new();
        
        // Weights
        params.insert("liquidity_risk_weight".to_string(), self.liquidity_risk_weight.clone());
        params.insert("volatility_risk_weight".to_string(), self.volatility_risk_weight.clone());
        params.insert("protocol_risk_weight".to_string(), self.protocol_risk_weight.clone());
        params.insert("mev_risk_weight".to_string(), self.mev_risk_weight.clone());
        params.insert("cross_chain_risk_weight".to_string(), self.cross_chain_risk_weight.clone());
        
        // Thresholds
        params.insert("min_tvl_threshold".to_string(), self.min_tvl_threshold.clone());
        params.insert("max_slippage_tolerance".to_string(), self.max_slippage_tolerance.clone());
        params.insert("thin_pool_threshold".to_string(), self.thin_pool_threshold.clone());
        params.insert("tvl_drop_threshold".to_string(), self.tvl_drop_threshold.clone());
        params.insert("high_volatility_threshold".to_string(), self.high_volatility_threshold.clone());
        params.insert("correlation_threshold".to_string(), self.correlation_threshold.clone());
        params.insert("min_audit_score".to_string(), self.min_audit_score.clone());
        params.insert("governance_risk_weight".to_string(), self.governance_risk_weight.clone());
        params.insert("sandwich_attack_threshold".to_string(), self.sandwich_attack_threshold.clone());
        params.insert("frontrun_threshold".to_string(), self.frontrun_threshold.clone());
        params.insert("oracle_deviation_threshold".to_string(), self.oracle_deviation_threshold.clone());
        params.insert("bridge_risk_tolerance".to_string(), self.bridge_risk_tolerance.clone());
        params.insert("liquidity_fragmentation_threshold".to_string(), self.liquidity_fragmentation_threshold.clone());
        params.insert("governance_divergence_threshold".to_string(), self.governance_divergence_threshold.clone());
        params.insert("overall_risk_threshold".to_string(), self.overall_risk_threshold.clone());
        
        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_conservative_config_creation() {
        let create_config = CreateUserRiskConfig {
            user_address: "0x123".to_string(),
            profile_name: "Test Conservative".to_string(),
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
        
        let config = UserRiskConfig::new(create_config);
        
        assert_eq!(config.user_address, "0x123");
        assert_eq!(config.profile_name, "Test Conservative");
        assert!(config.is_active);
        assert_eq!(config.min_tvl_threshold, BigDecimal::from_str("10000000").unwrap());
        assert_eq!(config.max_slippage_tolerance, BigDecimal::from_str("0.01").unwrap());
        
        // Test weight validation
        assert!(config.validate_weights().is_ok());
    }
    
    #[test]
    fn test_custom_config_with_overrides() {
        let create_config = CreateUserRiskConfig {
            user_address: "0x456".to_string(),
            profile_name: "Custom Profile".to_string(),
            risk_tolerance_level: RiskToleranceLevel::Custom,
            liquidity_risk_weight: Some(BigDecimal::from_str("0.40").unwrap()),
            volatility_risk_weight: Some(BigDecimal::from_str("0.30").unwrap()),
            protocol_risk_weight: Some(BigDecimal::from_str("0.20").unwrap()),
            mev_risk_weight: Some(BigDecimal::from_str("0.05").unwrap()),
            cross_chain_risk_weight: Some(BigDecimal::from_str("0.05").unwrap()),
            min_tvl_threshold: Some(BigDecimal::from_str("5000000").unwrap()),
            max_slippage_tolerance: None,
            thin_pool_threshold: None,
            tvl_drop_threshold: None,
            volatility_lookback_days: Some(60),
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
        
        let config = UserRiskConfig::new(create_config);
        
        assert_eq!(config.liquidity_risk_weight, BigDecimal::from_str("0.40").unwrap());
        assert_eq!(config.min_tvl_threshold, BigDecimal::from_str("5000000").unwrap());
        assert_eq!(config.volatility_lookback_days, 60);
        
        // Test weight validation
        assert!(config.validate_weights().is_ok());
    }
    
    #[test]
    fn test_weight_validation_failure() {
        let mut config = UserRiskConfig::get_defaults_for_tolerance(&RiskToleranceLevel::Conservative);
        
        // Make weights not sum to 1.0
        config.liquidity_risk_weight = BigDecimal::from_str("0.50").unwrap();
        
        assert!(config.validate_weights().is_err());
    }
    
    #[test]
    fn test_risk_params_conversion() {
        let config = UserRiskConfig::get_defaults_for_tolerance(&RiskToleranceLevel::Moderate);
        let params = config.to_risk_params();
        
        assert!(params.contains_key("liquidity_risk_weight"));
        assert!(params.contains_key("min_tvl_threshold"));
        assert!(params.contains_key("overall_risk_threshold"));
        assert_eq!(params.len(), 20); // Should have 20 parameters
    }
}
