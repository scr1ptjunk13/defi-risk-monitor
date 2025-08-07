use crate::models::{CreatePriceHistory, PriceHistory};
use crate::error::AppError;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PriceStorageService {
    db_pool: PgPool,
}

impl PriceStorageService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    pub async fn store_price(&self, price: &CreatePriceHistory) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO price_history (token_address, chain_id, price_usd, timestamp)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (token_address, chain_id, timestamp) DO NOTHING
            "#,
            price.token_address,
            price.chain_id,
            &price.price_usd,
            price.timestamp
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    pub async fn get_history(
        &self,
        token_address: &str,
        chain_id: i32,
        from: chrono::DateTime<chrono::Utc>,
        to: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<PriceHistory>, AppError> {
        let records = sqlx::query_as!(
            PriceHistory,
            r#"
            SELECT * FROM price_history
            WHERE token_address = $1 AND chain_id = $2
              AND timestamp >= $3 AND timestamp <= $4
            ORDER BY timestamp DESC
            "#,
            token_address,
            chain_id,
            from,
            to
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(records)
    }
}
