use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PriceHistory {
    pub id: Uuid,
    pub token_address: String,
    pub chain_id: i32,
    pub price_usd: BigDecimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePriceHistory {
    pub token_address: String,
    pub chain_id: i32,
    pub price_usd: BigDecimal,
    pub timestamp: DateTime<Utc>,
}
