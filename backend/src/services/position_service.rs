use crate::models::{Position, CreatePosition, UpdatePosition};
use crate::services::BlockchainService;
use crate::error::AppError;
use sqlx::{PgPool, Row};
use tracing::{info, warn};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

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

    /// Update position amounts and liquidity
    pub async fn update_position(&self, position_id: Uuid, update: UpdatePosition) -> Result<Position, AppError> {
        info!("Updating position {}", position_id);

        // Build dynamic query based on what fields are being updated
        let mut query_parts = Vec::new();
        let mut param_count = 1;

        if update.token0_amount.is_some() {
            query_parts.push(format!("token0_amount = ${}", param_count));
            param_count += 1;
        }
        if update.token1_amount.is_some() {
            query_parts.push(format!("token1_amount = ${}", param_count));
            param_count += 1;
        }
        if update.liquidity.is_some() {
            query_parts.push(format!("liquidity = ${}", param_count));
            param_count += 1;
        }

        if query_parts.is_empty() {
            return Err(AppError::ValidationError("No fields to update".to_string()));
        }

        query_parts.push("updated_at = NOW()".to_string());
        let set_clause = query_parts.join(", ");
        
        let query = format!(
            "UPDATE positions SET {} WHERE id = ${} RETURNING *",
            set_clause, param_count
        );

        let mut query_builder = sqlx::query_as::<_, Position>(&query);
        
        if let Some(token0_amount) = &update.token0_amount {
            query_builder = query_builder.bind(token0_amount);
        }
        if let Some(token1_amount) = &update.token1_amount {
            query_builder = query_builder.bind(token1_amount);
        }
        if let Some(liquidity) = &update.liquidity {
            query_builder = query_builder.bind(liquidity);
        }
        query_builder = query_builder.bind(position_id);

        let updated_position = query_builder
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to update position: {}", e)))?;

        info!("Successfully updated position {}", position_id);
        Ok(updated_position)
    }

    /// Delete position by ID
    pub async fn delete_position(&self, position_id: Uuid) -> Result<(), AppError> {
        info!("Deleting position {}", position_id);

        let result = sqlx::query!(
            "DELETE FROM positions WHERE id = $1",
            position_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to delete position: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("Position {} not found", position_id)));
        }

        info!("Successfully deleted position {}", position_id);
        Ok(())
    }

    /// Get position by ID (alias for existing get_position method)
    pub async fn get_position_by_id(&self, position_id: Uuid) -> Result<Option<Position>, AppError> {
        self.get_position(position_id).await
    }

    /// Get all positions for a specific pool
    pub async fn get_positions_by_pool(&self, pool_address: &str, chain_id: Option<i32>) -> Result<Vec<Position>, AppError> {
        info!("Getting positions for pool {} on chain {:?}", pool_address, chain_id);

        let positions = if let Some(chain_id) = chain_id {
            sqlx::query_as!(
                Position,
                "SELECT * FROM positions WHERE pool_address = $1 AND chain_id = $2 ORDER BY created_at DESC",
                pool_address,
                chain_id
            )
            .fetch_all(&self.db_pool)
            .await
        } else {
            sqlx::query_as!(
                Position,
                "SELECT * FROM positions WHERE pool_address = $1 ORDER BY created_at DESC",
                pool_address
            )
            .fetch_all(&self.db_pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(format!("Failed to get positions by pool: {}", e)))?;

        info!("Found {} positions for pool {}", positions.len(), pool_address);
        Ok(positions)
    }

    /// Get all positions for a specific protocol
    pub async fn get_positions_by_protocol(&self, protocol: &str, chain_id: Option<i32>) -> Result<Vec<Position>, AppError> {
        info!("Getting positions for protocol {} on chain {:?}", protocol, chain_id);

        let positions = if let Some(chain_id) = chain_id {
            sqlx::query_as!(
                Position,
                "SELECT * FROM positions WHERE protocol = $1 AND chain_id = $2 ORDER BY created_at DESC",
                protocol,
                chain_id
            )
            .fetch_all(&self.db_pool)
            .await
        } else {
            sqlx::query_as!(
                Position,
                "SELECT * FROM positions WHERE protocol = $1 ORDER BY created_at DESC",
                protocol
            )
            .fetch_all(&self.db_pool)
            .await
        }
        .map_err(|e| AppError::DatabaseError(format!("Failed to get positions by protocol: {}", e)))?;

        info!("Found {} positions for protocol {}", positions.len(), protocol);
        Ok(positions)
    }

    /// Get historical positions (older than specified date)
    pub async fn get_historical_positions(&self, before_date: DateTime<Utc>, limit: Option<i64>) -> Result<Vec<Position>, AppError> {
        info!("Getting historical positions before {}", before_date);

        let limit = limit.unwrap_or(1000); // Default limit to prevent huge queries

        let positions = sqlx::query_as!(
            Position,
            "SELECT * FROM positions WHERE created_at < $1 ORDER BY created_at DESC LIMIT $2",
            before_date,
            limit
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get historical positions: {}", e)))?;

        info!("Found {} historical positions before {}", positions.len(), before_date);
        Ok(positions)
    }

    /// Archive old positions (move to archive table or mark as archived)
    pub async fn archive_old_positions(&self, before_date: DateTime<Utc>) -> Result<u64, AppError> {
        info!("Archiving positions older than {}", before_date);

        // First, let's create an archive table if it doesn't exist
        sqlx::query!(
            r#"
            CREATE TABLE IF NOT EXISTS positions_archive (
                LIKE positions INCLUDING ALL
            )
            "#
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create archive table: {}", e)))?;

        // Move old positions to archive table using dynamic SQL
        let result = sqlx::query(
            r#"
            WITH moved_positions AS (
                DELETE FROM positions 
                WHERE created_at < $1 
                RETURNING *
            )
            INSERT INTO positions_archive 
            SELECT * FROM moved_positions
            "#
        )
        .bind(before_date)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to archive positions: {}", e)))?;

        let archived_count = result.rows_affected();
        info!("Successfully archived {} positions older than {}", archived_count, before_date);
        Ok(archived_count)
    }

    /// Get positions count by status/filters (utility method)
    pub async fn get_positions_count(&self, user_address: Option<&str>, protocol: Option<&str>, pool_address: Option<&str>) -> Result<i64, AppError> {
        let mut query = "SELECT COUNT(*) as count FROM positions WHERE 1=1".to_string();
        let mut param_count = 1;
        
        if user_address.is_some() {
            query.push_str(&format!(" AND user_address = ${}", param_count));
            param_count += 1;
        }
        if protocol.is_some() {
            query.push_str(&format!(" AND protocol = ${}", param_count));
            param_count += 1;
        }
        if pool_address.is_some() {
            query.push_str(&format!(" AND pool_address = ${}", param_count));
        }

        let mut query_builder = sqlx::query(&query);
        
        if let Some(addr) = user_address {
            query_builder = query_builder.bind(addr);
        }
        if let Some(proto) = protocol {
            query_builder = query_builder.bind(proto);
        }
        if let Some(pool) = pool_address {
            query_builder = query_builder.bind(pool);
        }

        let row = query_builder
            .fetch_one(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get positions count: {}", e)))?;

        let count: i64 = row.get("count");
        Ok(count)
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
