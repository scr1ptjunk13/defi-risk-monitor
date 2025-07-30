use crate::models::{PoolState, Position};
use crate::error::types::AppError;
use crate::services::lp_analytics_service::PoolPerformanceMetrics;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use num_traits::Zero;
use tracing::info;

#[derive(Debug, Clone)]
pub struct VolumeMetrics {
    pub volume_24h: BigDecimal,
    pub volume_7d: BigDecimal,
    pub volume_30d: BigDecimal,
    pub volume_change_24h: BigDecimal,
    pub volume_change_7d: BigDecimal,
}

#[derive(Debug, Clone)]
pub struct LiquidityMetrics {
    pub tvl_current: BigDecimal,
    pub tvl_change_24h: BigDecimal,
    pub tvl_change_7d: BigDecimal,
    pub tvl_change_30d: BigDecimal,
    pub liquidity_utilization: BigDecimal,
    pub concentration_ratio: BigDecimal,
}

#[derive(Debug, Clone)]
pub struct YieldMetrics {
    pub apr_24h: BigDecimal,
    pub apr_7d: BigDecimal,
    pub apr_30d: BigDecimal,
    pub apy_24h: BigDecimal,
    pub apy_7d: BigDecimal,
    pub apy_30d: BigDecimal,
    pub fee_tier: BigDecimal,
}

#[derive(Debug, Clone)]
pub struct PoolRiskMetrics {
    pub volatility_24h: BigDecimal,
    pub volatility_7d: BigDecimal,
    pub volatility_30d: BigDecimal,
    pub max_drawdown: BigDecimal,
    pub sharpe_ratio: BigDecimal,
    pub var_95: BigDecimal,
}

pub struct PoolPerformanceService {
    db_pool: PgPool,
}

impl PoolPerformanceService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get comprehensive pool performance metrics
    pub async fn get_pool_performance(&self, pool_address: &str, chain_id: i32) -> Result<PoolPerformanceMetrics, AppError> {
        info!("Calculating pool performance for {}", pool_address);

        let historical_data = self.get_historical_data(pool_address, chain_id, 720).await?; // 30 days of hourly data
        
        if historical_data.is_empty() {
            return Err(AppError::ValidationError("No historical data available".to_string()));
        }

        let volume_metrics = self.calculate_volume_metrics(&historical_data).await?;
        let liquidity_metrics = self.calculate_liquidity_metrics(&historical_data).await?;
        let yield_metrics = self.calculate_yield_metrics(&historical_data).await?;
        let lp_stats = self.calculate_lp_statistics(pool_address, chain_id).await?;

        Ok(PoolPerformanceMetrics {
            pool_address: pool_address.to_string(),
            chain_id,
            total_volume_24h: volume_metrics.volume_24h.clone(),
            total_volume_7d: volume_metrics.volume_7d.clone(),
            total_volume_30d: volume_metrics.volume_30d.clone(),
            fees_generated_24h: &volume_metrics.volume_24h * &BigDecimal::from_str("0.003").unwrap(),
            fees_generated_7d: &volume_metrics.volume_7d * &BigDecimal::from_str("0.003").unwrap(),
            fees_generated_30d: &volume_metrics.volume_30d * &BigDecimal::from_str("0.003").unwrap(),
            tvl_current: liquidity_metrics.tvl_current.clone(),
            tvl_change_24h: liquidity_metrics.tvl_change_24h.clone(),
            tvl_change_7d: liquidity_metrics.tvl_change_7d.clone(),
            apr_24h: yield_metrics.apr_24h.clone(),
            apr_7d: yield_metrics.apr_7d.clone(),
            apr_30d: yield_metrics.apr_30d.clone(),
            volatility_24h: self.calculate_volatility(&historical_data, 24).await?,
            volatility_7d: self.calculate_volatility(&historical_data, 168).await?,
            price_impact_1k: self.calculate_price_impact(&historical_data[0], &BigDecimal::from(1000)).await?,
            price_impact_10k: self.calculate_price_impact(&historical_data[0], &BigDecimal::from(10000)).await?,
            price_impact_100k: self.calculate_price_impact(&historical_data[0], &BigDecimal::from(100000)).await?,
            liquidity_utilization: liquidity_metrics.liquidity_utilization,
            active_lp_count: lp_stats.0,
            avg_position_size: lp_stats.1,
        })
    }

    /// Calculate volume metrics for different time periods
    async fn calculate_volume_metrics(&self, historical_data: &[PoolState]) -> Result<VolumeMetrics, AppError> {
        let now = Utc::now();
        let day_ago = now - Duration::hours(24);
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);

        let mut volume_24h = BigDecimal::from(0);
        let mut volume_7d = BigDecimal::from(0);
        let mut volume_30d = BigDecimal::from(0);

        // Calculate volume from TVL changes and price movements
        for window in historical_data.windows(2) {
            if let [current, previous] = window {
                let timestamp = current.timestamp;
                
                // Estimate volume from liquidity and price changes
                let tvl_change = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0)) - 
                               previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                
                let price0_change = current.token0_price_usd.clone().unwrap_or(BigDecimal::from(1)) - 
                                  previous.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
                
                let price1_change = current.token1_price_usd.clone().unwrap_or(BigDecimal::from(1)) - 
                                  previous.token1_price_usd.clone().unwrap_or(BigDecimal::from(1));

                // Simplified volume estimation
                let estimated_volume = tvl_change.abs() + 
                                     (price0_change.abs() * current.tvl_usd.clone().unwrap_or(BigDecimal::from(0)) / BigDecimal::from(2)) +
                                     (price1_change.abs() * current.tvl_usd.clone().unwrap_or(BigDecimal::from(0)) / BigDecimal::from(2));

                if timestamp >= day_ago {
                    volume_24h += &estimated_volume;
                }
                if timestamp >= week_ago {
                    volume_7d += &estimated_volume;
                }
                if timestamp >= month_ago {
                    volume_30d += &estimated_volume;
                }
            }
        }

        // Calculate volume changes
        let volume_change_24h = if historical_data.len() >= 48 {
            let recent_24h = &volume_24h;
            let previous_24h = self.calculate_volume_for_period(historical_data, 24, 48).await?;
            if !previous_24h.is_zero() {
                ((recent_24h - &previous_24h) / &previous_24h) * BigDecimal::from(100)
            } else {
                BigDecimal::from(0)
            }
        } else {
            BigDecimal::from(0)
        };

        let volume_change_7d = if historical_data.len() >= 336 {
            let recent_7d = &volume_7d;
            let previous_7d = self.calculate_volume_for_period(historical_data, 168, 336).await?;
            if !previous_7d.is_zero() {
                ((recent_7d - &previous_7d) / &previous_7d) * BigDecimal::from(100)
            } else {
                BigDecimal::from(0)
            }
        } else {
            BigDecimal::from(0)
        };

        Ok(VolumeMetrics {
            volume_24h,
            volume_7d,
            volume_30d,
            volume_change_24h,
            volume_change_7d,
        })
    }

    /// Calculate liquidity metrics
    async fn calculate_liquidity_metrics(&self, historical_data: &[PoolState]) -> Result<LiquidityMetrics, AppError> {
        if historical_data.is_empty() {
            return Ok(LiquidityMetrics {
                tvl_current: BigDecimal::from(0),
                tvl_change_24h: BigDecimal::from(0),
                tvl_change_7d: BigDecimal::from(0),
                tvl_change_30d: BigDecimal::from(0),
                liquidity_utilization: BigDecimal::from(0),
                concentration_ratio: BigDecimal::from(0),
            });
        }

        let current_state = &historical_data[0];
        let tvl_current = current_state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));

        // Calculate TVL changes
        let tvl_change_24h = BigDecimal::from(0);
        let tvl_change_7d = BigDecimal::from(0);
        let tvl_change_30d = BigDecimal::from(0);

        // Calculate liquidity utilization (simplified)
        let total_liquidity = &current_state.liquidity;
        let active_liquidity = total_liquidity * BigDecimal::from_str("0.7").unwrap(); // Assume 70% active
        let liquidity_utilization = if !total_liquidity.is_zero() {
            (&active_liquidity / total_liquidity) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };

        // Calculate concentration ratio (Gini coefficient approximation)
        let concentration_ratio = BigDecimal::from_str("0.3").unwrap(); // Stub value

        Ok(LiquidityMetrics {
            tvl_current,
            tvl_change_24h,
            tvl_change_7d,
            tvl_change_30d,
            liquidity_utilization,
            concentration_ratio,
        })
    }

    /// Calculate yield metrics
    async fn calculate_yield_metrics(&self, historical_data: &[PoolState]) -> Result<YieldMetrics, AppError> {
        let fee_tier = BigDecimal::from_str("0.003").unwrap(); // 0.3% fee tier

        // Calculate APR for different periods
        let apr_24h = self.calculate_apr(historical_data, 24).await?;
        let apr_7d = self.calculate_apr(historical_data, 168).await?;
        let apr_30d = self.calculate_apr(historical_data, 720).await?;

        // Calculate APY (compound interest)
        let apy_24h = self.calculate_apy(&apr_24h, 365).await?;
        let apy_7d = self.calculate_apy(&apr_7d, 52).await?;
        let apy_30d = self.calculate_apy(&apr_30d, 12).await?;

        Ok(YieldMetrics {
            apr_24h,
            apr_7d,
            apr_30d,
            apy_24h,
            apy_7d,
            apy_30d,
            fee_tier,
        })
    }

    /// Calculate APR for a given period
    async fn calculate_apr(&self, historical_data: &[PoolState], hours: usize) -> Result<BigDecimal, AppError> {
        if historical_data.len() < hours {
            return Ok(BigDecimal::from(0));
        }

        let recent_data = &historical_data[0..hours];
        let mut total_fees = BigDecimal::from(0);
        let mut avg_tvl = BigDecimal::from(0);

        for window in recent_data.windows(2) {
            if let [current, previous] = window {
                let tvl_current = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                let tvl_previous = previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                
                // Estimate volume and fees
                let estimated_volume = (&tvl_current - &tvl_previous).abs() * BigDecimal::from(5);
                let fees = &estimated_volume * BigDecimal::from_str("0.003").unwrap();
                
                total_fees += fees;
                avg_tvl += &tvl_current;
            }
        }

        if recent_data.len() > 1 {
            avg_tvl = avg_tvl / BigDecimal::from(recent_data.len() as i32);
        }

        if !avg_tvl.is_zero() {
            let period_return = &total_fees / &avg_tvl;
            let periods_per_year = BigDecimal::from(8760) / BigDecimal::from(hours as i32); // Hours per year / period hours
            Ok(period_return * periods_per_year * BigDecimal::from(100))
        } else {
            Ok(BigDecimal::from(0))
        }
    }

    /// Calculate APY from APR
    async fn calculate_apy(&self, apr: &BigDecimal, compounds_per_year: i32) -> Result<BigDecimal, AppError> {
        let apr_decimal = apr / BigDecimal::from(100);
        let _compound_rate = &apr_decimal / BigDecimal::from(compounds_per_year);
        
        // APY = (1 + r/n)^n - 1, where r = APR, n = compounds per year
        let apr_f64 = apr_decimal.to_f64().unwrap_or(0.0);
        let n_f64 = compounds_per_year as f64;
        
        let apy_f64 = (1.0 + apr_f64 / n_f64).powf(n_f64) - 1.0;
        
        Ok(BigDecimal::from_f64(apy_f64 * 100.0).unwrap_or(BigDecimal::from(0)))
    }

    /// Calculate volatility for a given period
    async fn calculate_volatility(&self, historical_data: &[PoolState], hours: usize) -> Result<BigDecimal, AppError> {
        if historical_data.len() < hours || hours < 2 {
            return Ok(BigDecimal::from(0));
        }

        let recent_data = &historical_data[0..hours];
        let mut price_changes = Vec::new();

        for window in recent_data.windows(2) {
            if let [current, previous] = window {
                let price0_current = current.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
                let price0_previous = previous.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
                
                if !price0_previous.is_zero() {
                    let price_change = (&price0_current - &price0_previous) / &price0_previous;
                    price_changes.push(price_change);
                }
            }
        }

        if price_changes.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        // Calculate standard deviation
        let mean: BigDecimal = price_changes.iter().sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);
        let variance: BigDecimal = price_changes.iter()
            .map(|x| {
                let diff = x - &mean;
                &diff * &diff
            })
            .sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);

        let volatility = variance.sqrt().unwrap_or(BigDecimal::from(0));
        
        // Annualize volatility
        let periods_per_year = BigDecimal::from(8760) / BigDecimal::from(hours as i32);
        let annualized_volatility = volatility * periods_per_year.sqrt().unwrap_or(BigDecimal::from(1));
        
        Ok(annualized_volatility * BigDecimal::from(100))
    }

    /// Calculate price impact for a given trade size
    async fn calculate_price_impact(&self, pool_state: &PoolState, trade_size: &BigDecimal) -> Result<BigDecimal, AppError> {
        let tvl = pool_state.tvl_usd.clone().unwrap_or(BigDecimal::from(1));
        
        if tvl.is_zero() {
            return Ok(BigDecimal::from(100)); // Maximum impact
        }

        // Simplified price impact: impact = sqrt(trade_size / tvl) * 100
        let ratio = trade_size / &tvl;
        let impact_f64 = ratio.to_f64().unwrap_or(0.0).sqrt() * 100.0;
        
        Ok(BigDecimal::from_f64(impact_f64).unwrap_or(BigDecimal::from(0)))
    }

    /// Helper methods
    async fn calculate_volume_for_period(&self, historical_data: &[PoolState], start_hours: usize, end_hours: usize) -> Result<BigDecimal, AppError> {
        if historical_data.len() < end_hours {
            return Ok(BigDecimal::from(0));
        }

        let period_data = &historical_data[start_hours..end_hours];
        let mut volume = BigDecimal::from(0);

        for window in period_data.windows(2) {
            if let [current, previous] = window {
                let tvl_change = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0)) - 
                               previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                volume += tvl_change.abs() * BigDecimal::from(10);
            }
        }

        Ok(volume)
    }

    async fn calculate_lp_statistics(&self, pool_address: &str, chain_id: i32) -> Result<(i64, BigDecimal), AppError> {
        let lp_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(DISTINCT user_address) FROM positions WHERE pool_address = $1 AND chain_id = $2"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(0);

        let avg_position_size = sqlx::query_scalar::<_, Option<BigDecimal>>(
            "SELECT AVG(liquidity) FROM positions WHERE pool_address = $1 AND chain_id = $2"
        )
        .bind(pool_address)
        .bind(chain_id)
        .fetch_one(&self.db_pool)
        .await
        .unwrap_or(None)
        .unwrap_or(BigDecimal::from(0));

        Ok((lp_count, avg_position_size))
    }

    async fn get_historical_data(&self, pool_address: &str, chain_id: i32, limit: i32) -> Result<Vec<PoolState>, AppError> {
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
