use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use num_traits::Zero;

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

    /// Calculate PnL based on current vs entry prices
    pub fn calculate_pnl_usd(
        &self,
        current_token0_price: &BigDecimal,
        current_token1_price: &BigDecimal,
    ) -> BigDecimal {
        if let (Some(entry_token0_price), Some(entry_token1_price)) = 
            (&self.entry_token0_price_usd, &self.entry_token1_price_usd) {
            
            // Current position value
            let current_value = self.calculate_position_value_usd(
                current_token0_price.clone(), 
                current_token1_price.clone()
            );
            
            // Entry position value
            let entry_value = self.calculate_position_value_usd(
                entry_token0_price.clone(), 
                entry_token1_price.clone()
            );
            
            current_value - entry_value
        } else {
            BigDecimal::from(0) // No entry prices available
        }
    }

    /// Estimate fees earned based on liquidity and fee tier
    /// This is a simplified calculation - in production would use actual fee data
    pub fn estimate_fees_earned_usd(
        &self,
        days_active: i64,
        daily_volume_usd: &BigDecimal,
        pool_tvl_usd: &BigDecimal,
    ) -> BigDecimal {
        if pool_tvl_usd.is_zero() {
            return BigDecimal::from(0);
        }

        // Calculate position's share of the pool
        let position_value = &self.token0_amount + &self.token1_amount; // Simplified
        let pool_share = position_value / pool_tvl_usd;
        
        // Calculate fee rate (fee_tier is in hundredths of a bip, e.g., 3000 = 0.3%)
        let fee_rate = BigDecimal::from(self.fee_tier) / BigDecimal::from(1_000_000);
        
        // Estimate fees: pool_share * daily_volume * fee_rate * days_active
        pool_share * daily_volume_usd * fee_rate * BigDecimal::from(days_active)
    }

    /// Determine if position is considered active
    /// A position is active if it has significant liquidity and recent activity
    pub fn is_position_active(&self) -> bool {
        // Position is active if:
        // 1. Has non-zero liquidity
        // 2. Was created/updated recently (within 30 days) OR has significant value
        let has_liquidity = !self.liquidity.is_zero();
        let has_tokens = !self.token0_amount.is_zero() || !self.token1_amount.is_zero();
        
        if !has_liquidity && !has_tokens {
            return false;
        }

        // Check if recently active (within 30 days)
        if let Some(updated_at) = self.updated_at {
            let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
            if updated_at > thirty_days_ago {
                return true;
            }
        }

        // If no recent activity, consider active if has significant value
        let min_active_value = BigDecimal::from(100); // $100 minimum
        let total_token_amount = &self.token0_amount + &self.token1_amount;
        total_token_amount > min_active_value
    }

    /// Get position type based on protocol and characteristics
    pub fn get_position_type(&self) -> String {
        match self.protocol.to_lowercase().as_str() {
            "uniswap_v3" | "uniswap" => "concentrated_liquidity".to_string(),
            "uniswap_v2" | "sushiswap" => "liquidity_pool".to_string(),
            "aave" => "lending".to_string(),
            "yearn" | "curve" => "yield_farming".to_string(),
            _ => "liquidity".to_string(), // Default fallback
        }
    }

    /// Calculate current price based on tick (for Uniswap V3)
    pub fn calculate_current_price_from_tick(&self, current_tick: i32) -> BigDecimal {
        // Uniswap V3 price calculation: price = 1.0001^tick
        let base = BigDecimal::from_str("1.0001").unwrap_or_else(|_| BigDecimal::from(1));
        
        // For simplicity, approximate the calculation
        // In production, would use proper tick-to-price conversion
        if current_tick >= 0 {
            base * BigDecimal::from(current_tick.abs())
        } else {
            BigDecimal::from(1) / (base * BigDecimal::from(current_tick.abs()))
        }
    }
}
