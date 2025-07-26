use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum ThresholdType {
    ImpermanentLoss,
    TvlDrop,
    LiquidityRisk,
    VolatilityRisk,
    ProtocolRisk,
    MevRisk,
    CrossChainRisk,
    OverallRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "lowercase")]
pub enum ThresholdOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertThreshold {
    pub id: Uuid,
    pub user_address: String,
    pub position_id: Option<Uuid>, // None means applies to all positions
    pub threshold_type: String,    // Will be converted to/from ThresholdType
    pub operator: String,          // Will be converted to/from ThresholdOperator
    pub threshold_value: BigDecimal,
    pub is_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAlertThreshold {
    pub user_address: String,
    pub position_id: Option<Uuid>,
    pub threshold_type: ThresholdType,
    pub operator: ThresholdOperator,
    pub threshold_value: BigDecimal,
    pub is_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAlertThreshold {
    pub threshold_value: Option<BigDecimal>,
    pub is_enabled: Option<bool>,
    pub operator: Option<ThresholdOperator>,
}

impl AlertThreshold {
    pub fn new(create_threshold: CreateAlertThreshold) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_address: create_threshold.user_address,
            position_id: create_threshold.position_id,
            threshold_type: threshold_type_to_string(create_threshold.threshold_type),
            operator: operator_to_string(create_threshold.operator),
            threshold_value: create_threshold.threshold_value,
            is_enabled: create_threshold.is_enabled,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn get_threshold_type(&self) -> ThresholdType {
        string_to_threshold_type(&self.threshold_type)
    }

    pub fn get_operator(&self) -> ThresholdOperator {
        string_to_operator(&self.operator)
    }

    /// Check if the current value exceeds this threshold
    pub fn is_exceeded(&self, current_value: &BigDecimal) -> bool {
        if !self.is_enabled {
            return false;
        }

        match self.get_operator() {
            ThresholdOperator::GreaterThan => current_value > &self.threshold_value,
            ThresholdOperator::LessThan => current_value < &self.threshold_value,
            ThresholdOperator::GreaterThanOrEqual => current_value >= &self.threshold_value,
            ThresholdOperator::LessThanOrEqual => current_value <= &self.threshold_value,
        }
    }
}

fn threshold_type_to_string(threshold_type: ThresholdType) -> String {
    match threshold_type {
        ThresholdType::ImpermanentLoss => "impermanent_loss".to_string(),
        ThresholdType::TvlDrop => "tvl_drop".to_string(),
        ThresholdType::LiquidityRisk => "liquidity_risk".to_string(),
        ThresholdType::VolatilityRisk => "volatility_risk".to_string(),
        ThresholdType::ProtocolRisk => "protocol_risk".to_string(),
        ThresholdType::MevRisk => "mev_risk".to_string(),
        ThresholdType::CrossChainRisk => "cross_chain_risk".to_string(),
        ThresholdType::OverallRisk => "overall_risk".to_string(),
    }
}

fn string_to_threshold_type(s: &str) -> ThresholdType {
    match s {
        "impermanent_loss" => ThresholdType::ImpermanentLoss,
        "tvl_drop" => ThresholdType::TvlDrop,
        "liquidity_risk" => ThresholdType::LiquidityRisk,
        "volatility_risk" => ThresholdType::VolatilityRisk,
        "protocol_risk" => ThresholdType::ProtocolRisk,
        "mev_risk" => ThresholdType::MevRisk,
        "cross_chain_risk" => ThresholdType::CrossChainRisk,
        "overall_risk" => ThresholdType::OverallRisk,
        _ => ThresholdType::OverallRisk,
    }
}

fn operator_to_string(operator: ThresholdOperator) -> String {
    match operator {
        ThresholdOperator::GreaterThan => "greater_than".to_string(),
        ThresholdOperator::LessThan => "less_than".to_string(),
        ThresholdOperator::GreaterThanOrEqual => "greater_than_or_equal".to_string(),
        ThresholdOperator::LessThanOrEqual => "less_than_or_equal".to_string(),
    }
}

fn string_to_operator(s: &str) -> ThresholdOperator {
    match s {
        "greater_than" => ThresholdOperator::GreaterThan,
        "less_than" => ThresholdOperator::LessThan,
        "greater_than_or_equal" => ThresholdOperator::GreaterThanOrEqual,
        "less_than_or_equal" => ThresholdOperator::LessThanOrEqual,
        _ => ThresholdOperator::GreaterThan,
    }
}

/// Default alert thresholds for new users
pub fn get_default_thresholds(user_address: &str) -> Vec<CreateAlertThreshold> {
    vec![
        // Impermanent Loss > 5%
        CreateAlertThreshold {
            user_address: user_address.to_string(),
            position_id: None,
            threshold_type: ThresholdType::ImpermanentLoss,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.05").unwrap(), // 5%
            is_enabled: true,
        },
        // TVL Drop > 50%
        CreateAlertThreshold {
            user_address: user_address.to_string(),
            position_id: None,
            threshold_type: ThresholdType::TvlDrop,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.50").unwrap(), // 50%
            is_enabled: true,
        },
        // Overall Risk > 70%
        CreateAlertThreshold {
            user_address: user_address.to_string(),
            position_id: None,
            threshold_type: ThresholdType::OverallRisk,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.70").unwrap(), // 70%
            is_enabled: true,
        },
        // Liquidity Risk > 60%
        CreateAlertThreshold {
            user_address: user_address.to_string(),
            position_id: None,
            threshold_type: ThresholdType::LiquidityRisk,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.60").unwrap(), // 60%
            is_enabled: true,
        },
        // MEV Risk > 80%
        CreateAlertThreshold {
            user_address: user_address.to_string(),
            position_id: None,
            threshold_type: ThresholdType::MevRisk,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.80").unwrap(), // 80%
            is_enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_creation() {
        let create_threshold = CreateAlertThreshold {
            user_address: "0x123".to_string(),
            position_id: None,
            threshold_type: ThresholdType::ImpermanentLoss,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.05").unwrap(),
            is_enabled: true,
        };

        let threshold = AlertThreshold::new(create_threshold);
        assert_eq!(threshold.user_address, "0x123");
        assert_eq!(threshold.threshold_type, "impermanent_loss");
        assert_eq!(threshold.operator, "greater_than");
        assert!(threshold.is_enabled);
    }

    #[test]
    fn test_threshold_exceeded() {
        let create_threshold = CreateAlertThreshold {
            user_address: "0x123".to_string(),
            position_id: None,
            threshold_type: ThresholdType::ImpermanentLoss,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.05").unwrap(),
            is_enabled: true,
        };

        let threshold = AlertThreshold::new(create_threshold);
        
        // Test values that exceed threshold
        assert!(threshold.is_exceeded(&BigDecimal::from_str("0.06").unwrap()));
        assert!(threshold.is_exceeded(&BigDecimal::from_str("0.10").unwrap()));
        
        // Test values that don't exceed threshold
        assert!(!threshold.is_exceeded(&BigDecimal::from_str("0.04").unwrap()));
        assert!(!threshold.is_exceeded(&BigDecimal::from_str("0.05").unwrap()));
    }

    #[test]
    fn test_disabled_threshold() {
        let create_threshold = CreateAlertThreshold {
            user_address: "0x123".to_string(),
            position_id: None,
            threshold_type: ThresholdType::ImpermanentLoss,
            operator: ThresholdOperator::GreaterThan,
            threshold_value: BigDecimal::from_str("0.05").unwrap(),
            is_enabled: false,
        };

        let threshold = AlertThreshold::new(create_threshold);
        
        // Disabled thresholds should never be exceeded
        assert!(!threshold.is_exceeded(&BigDecimal::from_str("0.10").unwrap()));
    }

    #[test]
    fn test_default_thresholds() {
        let defaults = get_default_thresholds("0x123");
        assert_eq!(defaults.len(), 5);
        
        // Check that all defaults are enabled
        assert!(defaults.iter().all(|t| t.is_enabled));
        
        // Check specific threshold values
        let il_threshold = defaults.iter()
            .find(|t| matches!(t.threshold_type, ThresholdType::ImpermanentLoss))
            .unwrap();
        assert_eq!(il_threshold.threshold_value, BigDecimal::from_str("0.05").unwrap());
    }
}
