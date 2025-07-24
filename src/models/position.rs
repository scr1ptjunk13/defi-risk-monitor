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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
            created_at: now,
            updated_at: now,
        }
    }

    pub fn calculate_position_value_usd(&self, token0_price: BigDecimal, token1_price: BigDecimal) -> BigDecimal {
        &self.token0_amount * &token0_price + &self.token1_amount * &token1_price
    }
}
