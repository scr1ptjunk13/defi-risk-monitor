use crate::models::{Position, PoolState, PriceHistory};
use crate::error::types::AppError;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use num_traits::Zero;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LpReturns {
    pub position_id: uuid::Uuid,
    pub total_return_usd: BigDecimal,
    pub total_return_percentage: BigDecimal,
    pub impermanent_loss_usd: BigDecimal,
    pub impermanent_loss_percentage: BigDecimal,
    pub fees_earned_usd: BigDecimal,
    pub fees_earned_percentage: BigDecimal,
    pub net_return_usd: BigDecimal,
    pub net_return_percentage: BigDecimal,
    pub apy: BigDecimal,
    pub apr: BigDecimal,
    pub days_active: i32,
    pub roi: BigDecimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolPerformanceMetrics {
    pub pool_address: String,
    pub chain_id: i32,
    pub total_volume_24h: BigDecimal,
    pub total_volume_7d: BigDecimal,
    pub total_volume_30d: BigDecimal,
    pub fees_generated_24h: BigDecimal,
    pub fees_generated_7d: BigDecimal,
    pub fees_generated_30d: BigDecimal,
    pub tvl_current: BigDecimal,
    pub tvl_change_24h: BigDecimal,
    pub tvl_change_7d: BigDecimal,
    pub apr_24h: BigDecimal,
    pub apr_7d: BigDecimal,
    pub apr_30d: BigDecimal,
    pub volatility_24h: BigDecimal,
    pub volatility_7d: BigDecimal,
    pub price_impact_1k: BigDecimal,
    pub price_impact_10k: BigDecimal,
    pub price_impact_100k: BigDecimal,
    pub liquidity_utilization: BigDecimal,
    pub active_lp_count: i64,
    pub avg_position_size: BigDecimal,
}

pub struct LpAnalyticsService {
    db_pool: PgPool,
}

impl LpAnalyticsService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Calculate comprehensive LP returns for a position
    pub async fn calculate_lp_returns(&self, position: &Position) -> Result<LpReturns, AppError> {
        info!("Calculating LP returns for position {}", position.id);

        let current_time = Utc::now();
        let days_active = position.created_at.map(|dt| (current_time - dt).num_days() as i32).unwrap_or(0);
        
        // Get current pool state
        let pool_state = self.get_current_pool_state(&position.pool_address, position.chain_id).await?;
        
        // Get historical data for fee calculation
        let historical_data = self.get_historical_pool_data(&position.pool_address, position.chain_id, 1000).await?;
        
        // Calculate current position value
        let current_token0_price = pool_state.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
        let current_token1_price = pool_state.token1_price_usd.clone().unwrap_or(BigDecimal::from(1));
        let current_value = position.calculate_position_value_usd(current_token0_price.clone(), current_token1_price.clone());
        
        // Calculate initial investment value
        let entry_token0_price = position.entry_token0_price_usd.clone().unwrap_or(current_token0_price.clone());
        let entry_token1_price = position.entry_token1_price_usd.clone().unwrap_or(current_token1_price.clone());
        let initial_value = position.calculate_position_value_usd(entry_token0_price.clone(), entry_token1_price.clone());
        
        // Calculate total return
        let total_return_usd = &current_value - &initial_value;
        let total_return_percentage = if !initial_value.is_zero() {
            (&total_return_usd / &initial_value) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        // Calculate impermanent loss
        let il_result = position.calculate_impermanent_loss_accurate(&current_token0_price, &current_token1_price);
        let impermanent_loss_percentage = il_result.unwrap_or(BigDecimal::from(0));
        let impermanent_loss_usd = (&impermanent_loss_percentage / BigDecimal::from(100)) * &initial_value;
        
        // Calculate fees earned
        let fees_earned_usd = self.calculate_fees_earned(position, &historical_data).await?;
        let fees_earned_percentage = if !initial_value.is_zero() {
            (&fees_earned_usd / &initial_value) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        // Calculate net return (total return + fees - IL)
        let net_return_usd = &total_return_usd + &fees_earned_usd - &impermanent_loss_usd.abs();
        let net_return_percentage = if !initial_value.is_zero() {
            (&net_return_usd / &initial_value) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        // Calculate APY and APR
        let (apy, apr) = self.calculate_yield_metrics(&net_return_percentage, days_active)?;
        
        // Calculate ROI
        let roi = net_return_percentage.clone();
        
        Ok(LpReturns {
            position_id: position.id,
            total_return_usd,
            total_return_percentage,
            impermanent_loss_usd,
            impermanent_loss_percentage,
            fees_earned_usd,
            fees_earned_percentage,
            net_return_usd,
            net_return_percentage,
            apy,
            apr,
            days_active,
            roi,
        })
    }

    /// Calculate fees earned by LP position
    async fn calculate_fees_earned(&self, position: &Position, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.is_empty() {
            return Ok(BigDecimal::from(0));
        }

        // Calculate LP's share of pool
        let current_pool_state = historical_data.first().unwrap();
        let total_liquidity = &current_pool_state.liquidity;
        
        if total_liquidity.is_zero() {
            return Ok(BigDecimal::from(0));
        }
        
        let position_liquidity = &position.liquidity;
        let lp_share = position_liquidity / total_liquidity;
        
        // Calculate total fees generated over time
        let mut total_fees = BigDecimal::from(0);
        
        for window in historical_data.windows(2) {
            if let [current, previous] = window {
                // Estimate volume from TVL changes and price movements
                let tvl_current = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                let tvl_previous = previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                
                // Simplified volume estimation (in production, use actual volume data)
                let estimated_volume = (&tvl_current - &tvl_previous).abs() * BigDecimal::from(10);
                
                // Assume 0.3% fee tier
                let fees_generated = &estimated_volume * BigDecimal::from_str("0.003").unwrap();
                total_fees += fees_generated;
            }
        }
        
        // LP's share of total fees
        let lp_fees = &total_fees * &lp_share;
        
        Ok(lp_fees)
    }

    /// Calculate APY and APR from return percentage and time period
    fn calculate_yield_metrics(&self, return_percentage: &BigDecimal, days_active: i32) -> Result<(BigDecimal, BigDecimal), AppError> {
        if days_active <= 0 {
            return Ok((BigDecimal::from(0), BigDecimal::from(0)));
        }
        
        let days_f64 = days_active as f64;
        let return_f64 = return_percentage.to_f64().unwrap_or(0.0);
        
        // APR (simple annualized return)
        let apr = (return_f64 / days_f64) * 365.0;
        
        // APY (compound annualized return)
        let daily_return = return_f64 / 100.0 / days_f64;
        let apy = if daily_return > -1.0 {
            ((1.0 + daily_return).powf(365.0) - 1.0) * 100.0
        } else {
            apr // Fallback to APR if daily return is too negative
        };
        
        Ok((
            BigDecimal::from_f64(apy).unwrap_or(BigDecimal::from(0)),
            BigDecimal::from_f64(apr).unwrap_or(BigDecimal::from(0))
        ))
    }

    /// Get current pool state
    async fn get_current_pool_state(&self, pool_address: &str, chain_id: i32) -> Result<PoolState, AppError> {
        let pool_state = sqlx::query_as::<_, PoolState>(
            "SELECT * FROM pool_states WHERE pool_address = $1 AND chain_id = $2 ORDER BY timestamp DESC LIMIT 1"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(pool_state)
    }

    /// Get historical pool data
    async fn get_historical_pool_data(&self, pool_address: &str, chain_id: i32, limit: i32) -> Result<Vec<PoolState>, AppError> {
        let pool_states = sqlx::query_as::<_, PoolState>(
            "SELECT * FROM pool_states WHERE pool_address = $1 AND chain_id = $2 ORDER BY timestamp DESC LIMIT $3"
        )
        .bind(pool_address)
        .bind(chain_id)
        .bind(limit)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(pool_states)
    }
}
