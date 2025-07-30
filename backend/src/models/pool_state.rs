use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PoolState {
    pub id: Uuid,
    pub pool_address: String,
    pub chain_id: i32,
    pub current_tick: i32,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub token0_price_usd: Option<BigDecimal>,
    pub token1_price_usd: Option<BigDecimal>,
    pub tvl_usd: Option<BigDecimal>,
    pub volume_24h_usd: Option<BigDecimal>,
    pub fees_24h_usd: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePoolState {
    pub pool_address: String,
    pub chain_id: i32,
    pub current_tick: i32,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub token0_price_usd: Option<BigDecimal>,
    pub token1_price_usd: Option<BigDecimal>,
    pub tvl_usd: Option<BigDecimal>,
    pub volume_24h_usd: Option<BigDecimal>,
    pub fees_24h_usd: Option<BigDecimal>,
}

impl PoolState {
    pub fn new(create_pool_state: CreatePoolState) -> Self {
        Self {
            id: Uuid::new_v4(),
            pool_address: create_pool_state.pool_address,
            chain_id: create_pool_state.chain_id,
            current_tick: create_pool_state.current_tick,
            sqrt_price_x96: create_pool_state.sqrt_price_x96,
            liquidity: create_pool_state.liquidity,
            token0_price_usd: create_pool_state.token0_price_usd,
            token1_price_usd: create_pool_state.token1_price_usd,
            tvl_usd: create_pool_state.tvl_usd,
            volume_24h_usd: create_pool_state.volume_24h_usd,
            fees_24h_usd: create_pool_state.fees_24h_usd,
            timestamp: Utc::now(),
        }
    }
}
