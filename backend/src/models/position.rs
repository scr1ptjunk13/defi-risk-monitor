use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Position {
    pub id: Uuid,
    pub user_address: String,
    pub protocol: String,
    pub pool_address: String,
    pub token0_address: String,
    pub token1_address: String,
    pub token0_amount: BigDecimal,
    pub token1_amount: BigDecimal,
    pub liquidity: BigDecimal,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_tier: i32,
    pub chain_id: i32,
    // Entry price tracking for accurate IL calculations
    pub entry_token0_price_usd: Option<BigDecimal>,
    pub entry_token1_price_usd: Option<BigDecimal>,
    pub entry_timestamp: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePosition {
    pub user_address: String,
    pub protocol: String,
    pub pool_address: String,
    pub token0_address: String,
    pub token1_address: String,
    pub token0_amount: BigDecimal,
    pub token1_amount: BigDecimal,
    pub liquidity: BigDecimal,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub fee_tier: i32,
    pub chain_id: i32,
    // Entry price tracking for accurate IL calculations
    pub entry_token0_price_usd: Option<BigDecimal>,
    pub entry_token1_price_usd: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePosition {
    pub token0_amount: Option<BigDecimal>,
    pub token1_amount: Option<BigDecimal>,
    pub liquidity: Option<BigDecimal>,
}

impl Position {
    pub fn new(create_position: CreatePosition) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            user_address: create_position.user_address,
            protocol: create_position.protocol,
            pool_address: create_position.pool_address,
            token0_address: create_position.token0_address,
            token1_address: create_position.token1_address,
            token0_amount: create_position.token0_amount,
            token1_amount: create_position.token1_amount,
            liquidity: create_position.liquidity,
            tick_lower: create_position.tick_lower,
            tick_upper: create_position.tick_upper,
            fee_tier: create_position.fee_tier,
            chain_id: create_position.chain_id,
            entry_token0_price_usd: create_position.entry_token0_price_usd,
            entry_token1_price_usd: create_position.entry_token1_price_usd,
            entry_timestamp: Some(now),
            created_at: Some(now),
            updated_at: Some(now),
        }
    }

    pub fn calculate_position_value_usd(&self, token0_price: BigDecimal, token1_price: BigDecimal) -> BigDecimal {
        &self.token0_amount * &token0_price + &self.token1_amount * &token1_price
    }

    /// Calculate accurate impermanent loss using entry prices if available
    pub fn calculate_impermanent_loss_accurate(
        &self,
        current_token0_price: &BigDecimal,
        current_token1_price: &BigDecimal,
    ) -> Option<BigDecimal> {
        // Only calculate if we have entry prices
        if let (Some(entry_token0_price), Some(entry_token1_price)) = 
            (&self.entry_token0_price_usd, &self.entry_token1_price_usd) {
            
            // Calculate price ratio changes
            let price_ratio_change = (current_token0_price / entry_token0_price) / 
                                   (current_token1_price / entry_token1_price);
            
            // Calculate impermanent loss using the standard formula:
            // IL = 2 * sqrt(price_ratio) / (1 + price_ratio) - 1
            let sqrt_ratio = price_ratio_change.sqrt().unwrap_or_else(|| BigDecimal::from(1));
            let il = (BigDecimal::from(2) * &sqrt_ratio) / (BigDecimal::from(1) + &price_ratio_change) - BigDecimal::from(1);
            
            Some(il.abs()) // Return absolute value as risk metric
        } else {
            None // No entry prices available
        }
    }

    /// Get entry price ratio (token0/token1) if available
    pub fn get_entry_price_ratio(&self) -> Option<BigDecimal> {
        if let (Some(entry_token0_price), Some(entry_token1_price)) = 
            (&self.entry_token0_price_usd, &self.entry_token1_price_usd) {
            Some(entry_token0_price / entry_token1_price)
        } else {
            None
        }
    }

    /// Check if position has entry price data for accurate calculations
    pub fn has_entry_prices(&self) -> bool {
        self.entry_token0_price_usd.is_some() && self.entry_token1_price_usd.is_some()
    }
}
