use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RiskConfig {
    pub id: Uuid,
    pub user_address: String,
    pub max_position_size_usd: BigDecimal,
    pub liquidation_threshold: BigDecimal,
    pub price_impact_threshold: BigDecimal,
    pub impermanent_loss_threshold: BigDecimal,
    pub volatility_threshold: BigDecimal,
    pub correlation_threshold: BigDecimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRiskConfig {
    pub user_address: String,
    pub max_position_size_usd: Option<BigDecimal>,
    pub liquidation_threshold: Option<BigDecimal>,
    pub price_impact_threshold: Option<BigDecimal>,
    pub impermanent_loss_threshold: Option<BigDecimal>,
    pub volatility_threshold: Option<BigDecimal>,
    pub correlation_threshold: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRiskConfig {
    pub max_position_size_usd: Option<BigDecimal>,
    pub liquidation_threshold: Option<BigDecimal>,
    pub price_impact_threshold: Option<BigDecimal>,
    pub impermanent_loss_threshold: Option<BigDecimal>,
    pub volatility_threshold: Option<BigDecimal>,
    pub correlation_threshold: Option<BigDecimal>,
}

impl RiskConfig {
    pub fn new(create_config: CreateRiskConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_address: create_config.user_address,
            max_position_size_usd: create_config.max_position_size_usd
                .unwrap_or_else(|| BigDecimal::from(1000000)),
            liquidation_threshold: create_config.liquidation_threshold
                .unwrap_or_else(|| BigDecimal::from(85)), // 0.85
            price_impact_threshold: create_config.price_impact_threshold
                .unwrap_or_else(|| BigDecimal::from(5)), // 0.05
            impermanent_loss_threshold: create_config.impermanent_loss_threshold
                .unwrap_or_else(|| BigDecimal::from(10)), // 0.10
            volatility_threshold: create_config.volatility_threshold
                .unwrap_or_else(|| BigDecimal::from(20)), // 0.20
            correlation_threshold: create_config.correlation_threshold
                .unwrap_or_else(|| BigDecimal::from(80)), // 0.80
            created_at: now,
            updated_at: now,
        }
    }
}
