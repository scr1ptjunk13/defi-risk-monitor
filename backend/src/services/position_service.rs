use crate::models::{Position, CreatePosition};
use crate::services::BlockchainService;
use crate::error::AppError;
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

/// Service for managing positions with entry price tracking
pub struct PositionService {
    db_pool: PgPool,
    blockchain_service: BlockchainService,
}

impl PositionService {
    pub fn new(db_pool: PgPool, blockchain_service: BlockchainService) -> Self {
        Self {
            db_pool,
            blockchain_service,
        }
    }

    /// Create a new position with automatic entry price fetching
    pub async fn create_position_with_entry_prices(
        &self,
        mut create_position: CreatePosition,
    ) -> Result<Position, AppError> {
        info!("Creating position with entry price tracking for pool {}", create_position.pool_address);

        // Fetch current token prices as entry prices if not provided
        if create_position.entry_token0_price_usd.is_none() {
            match self.blockchain_service.get_token_price(&create_position.token0_address, create_position.chain_id).await {
                Ok(price) => {
                    create_position.entry_token0_price_usd = Some(price.clone());
                    info!("Fetched entry price for token0 {}: ${}", create_position.token0_address, price);
                },
                Err(e) => {
                    warn!("Failed to fetch entry price for token0 {}: {}. Position will use simplified IL calculation.", 
                          create_position.token0_address, e);
                }
            }
        }

        if create_position.entry_token1_price_usd.is_none() {
            match self.blockchain_service.get_token_price(&create_position.token1_address, create_position.chain_id).await {
                Ok(price) => {
                    create_position.entry_token1_price_usd = Some(price.clone());
                    info!("Fetched entry price for token1 {}: ${}", create_position.token1_address, price);
                },
                Err(e) => {
                    warn!("Failed to fetch entry price for token1 {}: {}. Position will use simplified IL calculation.", 
                          create_position.token1_address, e);
                }
            }
        }

        // Create position with entry prices
        let position = Position::new(create_position);
        
        // Store position in database
        self.store_position(&position).await?;

        info!("Created position {} with entry price tracking: token0=${:?}, token1=${:?}", 
              position.id, 
              position.entry_token0_price_usd, 
              position.entry_token1_price_usd);

        Ok(position)
    }

    /// Store position in database
    async fn store_position(&self, position: &Position) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO positions (
                id, user_address, protocol, pool_address, token0_address, token1_address,
                token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier,
                chain_id, entry_token0_price_usd, entry_token1_price_usd, entry_timestamp,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#,
            position.id,
            position.user_address,
            position.protocol,
            position.pool_address,
            position.token0_address,
            position.token1_address,
            position.token0_amount,
            position.token1_amount,
            position.liquidity,
            position.tick_lower,
            position.tick_upper,
            position.fee_tier,
            position.chain_id,
            position.entry_token0_price_usd,
            position.entry_token1_price_usd,
            position.entry_timestamp,
            position.created_at,
            position.updated_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get position by ID
    pub async fn get_position(&self, position_id: Uuid) -> Result<Option<Position>, AppError> {
        let position = sqlx::query_as!(
            Position,
            r#"
            SELECT id, user_address, protocol, pool_address, token0_address, token1_address,
                   token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier,
                   chain_id, entry_token0_price_usd, entry_token1_price_usd, 
                   entry_timestamp as "entry_timestamp",
                   created_at as "created_at", updated_at as "updated_at"
            FROM positions 
            WHERE id = $1
            "#,
            position_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(position)
    }

    /// Get all positions for a user
    pub async fn get_user_positions(&self, user_address: &str) -> Result<Vec<Position>, AppError> {
        let positions = sqlx::query_as!(
            Position,
            r#"
            SELECT id, user_address, protocol, pool_address, token0_address, token1_address,
                   token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier,
                   chain_id, entry_token0_price_usd, entry_token1_price_usd, 
                   entry_timestamp as "entry_timestamp",
                   created_at as "created_at", updated_at as "updated_at"
            FROM positions 
            WHERE user_address = $1
            ORDER BY created_at DESC
            "#,
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(positions)
    }

    /// Get positions with entry prices (for accurate IL calculations)
    pub async fn get_positions_with_entry_prices(&self) -> Result<Vec<Position>, AppError> {
        let positions = sqlx::query_as!(
            Position,
            r#"
            SELECT id, user_address, protocol, pool_address, token0_address, token1_address,
                   token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier,
                   chain_id, entry_token0_price_usd, entry_token1_price_usd, 
                   entry_timestamp as "entry_timestamp",
                   created_at as "created_at", updated_at as "updated_at"
            FROM positions 
            WHERE entry_token0_price_usd IS NOT NULL AND entry_token1_price_usd IS NOT NULL
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(positions)
    }

    /// Update position entry prices (useful for migrating existing positions)
    pub async fn update_position_entry_prices(
        &self,
        position_id: Uuid,
        token0_address: &str,
        token1_address: &str,
        chain_id: i32,
    ) -> Result<(), AppError> {
        info!("Updating entry prices for position {}", position_id);

        // Fetch current prices as entry prices
        let token0_price = self.blockchain_service.get_token_price(token0_address, chain_id).await?;
        let token1_price = self.blockchain_service.get_token_price(token1_address, chain_id).await?;

        sqlx::query!(
            r#"
            UPDATE positions 
            SET entry_token0_price_usd = $1,
                entry_token1_price_usd = $2,
                updated_at = NOW()
            WHERE id = $3
            "#,
            token0_price,
            token1_price,
            position_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        info!("Updated entry prices for position {}: token0=${}, token1=${}", 
              position_id, token0_price, token1_price);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    #[test]
    fn test_create_position_struct() {
        let create_position = CreatePosition {
            user_address: "0x123".to_string(),
            protocol: "uniswap_v3".to_string(),
            pool_address: "0xpool".to_string(),
            token0_address: "0xtoken0".to_string(),
            token1_address: "0xtoken1".to_string(),
            token0_amount: BigDecimal::from(1000),
            token1_amount: BigDecimal::from(2000),
            liquidity: BigDecimal::from(50000),
            tick_lower: -1000,
            tick_upper: 1000,
            fee_tier: 3000,
            chain_id: 1,
            entry_token0_price_usd: Some(BigDecimal::from_str("1.50").unwrap()),
            entry_token1_price_usd: Some(BigDecimal::from_str("2500.00").unwrap()),
        };

        let position = Position::new(create_position);
        
        assert!(position.has_entry_prices());
        assert_eq!(position.entry_token0_price_usd, Some(BigDecimal::from_str("1.50").unwrap()));
        assert_eq!(position.entry_token1_price_usd, Some(BigDecimal::from_str("2500.00").unwrap()));
    }

    #[test]
    fn test_accurate_il_calculation() {
        let create_position = CreatePosition {
            user_address: "0x123".to_string(),
            protocol: "uniswap_v3".to_string(),
            pool_address: "0xpool".to_string(),
            token0_address: "0xtoken0".to_string(),
            token1_address: "0xtoken1".to_string(),
            token0_amount: BigDecimal::from(1000),
            token1_amount: BigDecimal::from(2000),
            liquidity: BigDecimal::from(50000),
            tick_lower: -1000,
            tick_upper: 1000,
            fee_tier: 3000,
            chain_id: 1,
            entry_token0_price_usd: Some(BigDecimal::from_str("1.00").unwrap()),
            entry_token1_price_usd: Some(BigDecimal::from_str("2000.00").unwrap()),
        };

        let position = Position::new(create_position);
        
        // Test IL calculation with price changes
        let current_token0_price = BigDecimal::from_str("1.50").unwrap(); // 50% increase
        let current_token1_price = BigDecimal::from_str("2000.00").unwrap(); // No change
        
        let il = position.calculate_impermanent_loss_accurate(&current_token0_price, &current_token1_price);
        
        assert!(il.is_some());
        let il_value = il.unwrap();
        assert!(il_value > BigDecimal::from(0)); // Should have some impermanent loss
        
        // Test with no entry prices
        let create_position_no_entry = CreatePosition {
            user_address: "0x123".to_string(),
            protocol: "uniswap_v3".to_string(),
            pool_address: "0xpool".to_string(),
            token0_address: "0xtoken0".to_string(),
            token1_address: "0xtoken1".to_string(),
            token0_amount: BigDecimal::from(1000),
            token1_amount: BigDecimal::from(2000),
            liquidity: BigDecimal::from(50000),
            tick_lower: -1000,
            tick_upper: 1000,
            fee_tier: 3000,
            chain_id: 1,
            entry_token0_price_usd: None,
            entry_token1_price_usd: None,
        };

        let position_no_entry = Position::new(create_position_no_entry);
        let il_no_entry = position_no_entry.calculate_impermanent_loss_accurate(&current_token0_price, &current_token1_price);
        
        assert!(il_no_entry.is_none()); // Should return None when no entry prices
    }
}
