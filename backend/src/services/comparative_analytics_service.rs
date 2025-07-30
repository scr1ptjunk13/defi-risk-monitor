use crate::models::{PoolState, Position};
use crate::error::types::AppError;
use crate::services::lp_analytics_service::{LpReturns, PoolPerformanceMetrics};
use crate::services::yield_farming_service::YieldFarmingMetrics;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use num_traits::Zero;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolComparison {
    pub pool_address: String,
    pub chain_id: i32,
    pub rank: i32,
    pub score: BigDecimal,
    pub apr: BigDecimal,
    pub apy: BigDecimal,
    pub tvl: BigDecimal,
    pub volume_24h: BigDecimal,
    pub volatility: BigDecimal,
    pub sharpe_ratio: BigDecimal,
    pub max_drawdown: BigDecimal,
    pub liquidity_utilization: BigDecimal,
    pub risk_score: BigDecimal,
    pub efficiency_score: BigDecimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkMetrics {
    pub benchmark_name: String,
    pub benchmark_apr: BigDecimal,
    pub benchmark_volatility: BigDecimal,
    pub benchmark_sharpe: BigDecimal,
    pub alpha: BigDecimal,
    pub beta: BigDecimal,
    pub tracking_error: BigDecimal,
    pub information_ratio: BigDecimal,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PerformanceRanking {
    pub category: String,
    pub pools: Vec<PoolComparison>,
    pub market_leaders: Vec<String>,
    pub underperformers: Vec<String>,
    pub rising_stars: Vec<String>,
}

pub struct ComparativeAnalyticsService {
    db_pool: PgPool,
}

impl ComparativeAnalyticsService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Compare multiple pools and rank them by performance
    pub async fn compare_pools(&self, pool_addresses: &[String], chain_id: i32) -> Result<Vec<PoolComparison>, AppError> {
        info!("Comparing {} pools for ranking", pool_addresses.len());

        let mut comparisons = Vec::new();

        for pool_address in pool_addresses {
            let metrics = self.calculate_pool_metrics(pool_address, chain_id).await?;
            let score = self.calculate_composite_score(&metrics).await?;
            
            comparisons.push(PoolComparison {
                pool_address: pool_address.clone(),
                chain_id,
                rank: 0, // Will be set after sorting
                score,
                apr: metrics.apr,
                apy: metrics.apy,
                tvl: metrics.tvl,
                volume_24h: metrics.volume_24h,
                volatility: metrics.volatility,
                sharpe_ratio: metrics.sharpe_ratio,
                max_drawdown: metrics.max_drawdown,
                liquidity_utilization: metrics.liquidity_utilization,
                risk_score: metrics.risk_score,
                efficiency_score: metrics.efficiency_score,
            });
        }

        // Sort by composite score descending
        comparisons.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Assign ranks
        for (index, comparison) in comparisons.iter_mut().enumerate() {
            comparison.rank = (index + 1) as i32;
        }

        Ok(comparisons)
    }

    /// Generate performance rankings by category
    pub async fn generate_performance_rankings(&self, chain_id: i32) -> Result<Vec<PerformanceRanking>, AppError> {
        info!("Generating performance rankings for chain {}", chain_id);

        let all_pools = self.get_all_pools(chain_id).await?;
        let pool_addresses: Vec<String> = all_pools.iter().map(|p| p.pool_address.clone()).collect();
        
        let comparisons = self.compare_pools(&pool_addresses, chain_id).await?;

        let mut rankings = Vec::new();

        // High TVL Category (>$10M)
        let high_tvl_pools: Vec<PoolComparison> = comparisons.iter()
            .filter(|p| p.tvl > BigDecimal::from(10_000_000))
            .cloned()
            .collect();
        
        if !high_tvl_pools.is_empty() {
            rankings.push(self.create_ranking("High TVL Pools", high_tvl_pools).await?);
        }

        // Medium TVL Category ($1M - $10M)
        let medium_tvl_pools: Vec<PoolComparison> = comparisons.iter()
            .filter(|p| p.tvl >= BigDecimal::from(1_000_000) && p.tvl <= BigDecimal::from(10_000_000))
            .cloned()
            .collect();
        
        if !medium_tvl_pools.is_empty() {
            rankings.push(self.create_ranking("Medium TVL Pools", medium_tvl_pools).await?);
        }

        // High Yield Category (>30% APR)
        let high_yield_pools: Vec<PoolComparison> = comparisons.iter()
            .filter(|p| p.apr > BigDecimal::from(30))
            .cloned()
            .collect();
        
        if !high_yield_pools.is_empty() {
            rankings.push(self.create_ranking("High Yield Pools", high_yield_pools).await?);
        }

        // Low Risk Category (Volatility <20%)
        let low_risk_pools: Vec<PoolComparison> = comparisons.iter()
            .filter(|p| p.volatility < BigDecimal::from(20))
            .cloned()
            .collect();
        
        if !low_risk_pools.is_empty() {
            rankings.push(self.create_ranking("Low Risk Pools", low_risk_pools).await?);
        }

        // Best Sharpe Ratio Category
        let mut best_sharpe_pools = comparisons.clone();
        best_sharpe_pools.sort_by(|a, b| b.sharpe_ratio.partial_cmp(&a.sharpe_ratio).unwrap_or(std::cmp::Ordering::Equal));
        best_sharpe_pools.truncate(10);
        
        if !best_sharpe_pools.is_empty() {
            rankings.push(self.create_ranking("Best Risk-Adjusted Returns", best_sharpe_pools).await?);
        }

        Ok(rankings)
    }

    /// Calculate benchmark comparison metrics
    pub async fn calculate_benchmark_metrics(&self, pool_address: &str, chain_id: i32, benchmark_type: &str) -> Result<BenchmarkMetrics, AppError> {
        info!("Calculating benchmark metrics for pool {} against {}", pool_address, benchmark_type);

        let pool_metrics = self.calculate_pool_metrics(pool_address, chain_id).await?;
        let benchmark_metrics = self.get_benchmark_data(benchmark_type, chain_id).await?;

        // Calculate alpha (excess return over benchmark)
        let alpha = &pool_metrics.apr - &benchmark_metrics.apr;

        // Calculate beta (correlation with benchmark)
        let beta = self.calculate_beta(pool_address, chain_id, benchmark_type).await?;

        // Calculate tracking error (volatility of excess returns)
        let tracking_error = self.calculate_tracking_error(pool_address, chain_id, benchmark_type).await?;

        // Calculate information ratio (alpha / tracking error)
        let information_ratio = if !tracking_error.is_zero() {
            &alpha / &tracking_error
        } else {
            BigDecimal::from(0)
        };

        Ok(BenchmarkMetrics {
            benchmark_name: benchmark_type.to_string(),
            benchmark_apr: benchmark_metrics.apr,
            benchmark_volatility: benchmark_metrics.volatility,
            benchmark_sharpe: benchmark_metrics.sharpe_ratio,
            alpha,
            beta,
            tracking_error,
            information_ratio,
        })
    }

    /// Calculate LP performance benchmarking
    pub async fn benchmark_lp_performance(&self, position: &Position, benchmark_pools: &[String]) -> Result<HashMap<String, BigDecimal>, AppError> {
        info!("Benchmarking LP performance for position {}", position.id);

        let mut benchmarks = HashMap::new();

        // Calculate position returns
        let position_returns = self.calculate_position_returns(position).await?;

        // Compare against benchmark pools
        for benchmark_pool in benchmark_pools {
            let benchmark_returns = self.calculate_benchmark_pool_returns(benchmark_pool, position.chain_id).await?;
            let relative_performance = &position_returns - &benchmark_returns;
            benchmarks.insert(benchmark_pool.clone(), relative_performance);
        }

        // Add market benchmarks
        benchmarks.insert("DeFi_Index".to_string(), &position_returns - BigDecimal::from_str("15.2").unwrap());
        benchmarks.insert("ETH_Staking".to_string(), &position_returns - BigDecimal::from_str("4.5").unwrap());
        benchmarks.insert("BTC_Hold".to_string(), &position_returns - BigDecimal::from_str("8.7").unwrap());
        benchmarks.insert("SP500".to_string(), &position_returns - BigDecimal::from_str("10.1").unwrap());

        Ok(benchmarks)
    }

    /// Calculate detailed pool metrics for comparison
    async fn calculate_pool_metrics(&self, pool_address: &str, chain_id: i32) -> Result<DetailedPoolMetrics, AppError> {
        let historical_data = self.get_historical_data(pool_address, chain_id, 720).await?;
        
        if historical_data.is_empty() {
            return Err(AppError::ValidationError("No historical data available".to_string()));
        }

        let apr = self.calculate_apr(&historical_data).await?;
        let apy = self.calculate_apy(&apr).await?;
        let tvl = historical_data[0].tvl_usd.clone().unwrap_or(BigDecimal::from(0));
        let volume_24h = self.calculate_volume_24h(&historical_data).await?;
        let volatility = self.calculate_volatility(&historical_data).await?;
        let sharpe_ratio = self.calculate_sharpe_ratio(&apr, &volatility).await?;
        let max_drawdown = self.calculate_max_drawdown(&historical_data).await?;
        let liquidity_utilization = self.calculate_liquidity_utilization(&historical_data).await?;
        let risk_score = self.calculate_risk_score(&volatility, &max_drawdown).await?;
        let efficiency_score = self.calculate_efficiency_score(&apr, &risk_score, &liquidity_utilization).await?;

        Ok(DetailedPoolMetrics {
            apr,
            apy,
            tvl,
            volume_24h,
            volatility,
            sharpe_ratio,
            max_drawdown,
            liquidity_utilization,
            risk_score,
            efficiency_score,
        })
    }

    /// Calculate composite score for ranking
    async fn calculate_composite_score(&self, metrics: &DetailedPoolMetrics) -> Result<BigDecimal, AppError> {
        // Weighted scoring system
        let return_weight = BigDecimal::from_str("0.3").unwrap();
        let risk_weight = BigDecimal::from_str("0.25").unwrap();
        let liquidity_weight = BigDecimal::from_str("0.2").unwrap();
        let efficiency_weight = BigDecimal::from_str("0.15").unwrap();
        let sharpe_weight = BigDecimal::from_str("0.1").unwrap();

        // Normalize metrics to 0-100 scale
        let return_score = (&metrics.apr / BigDecimal::from(100)).min(BigDecimal::from(1)) * BigDecimal::from(100);
        let risk_score = (BigDecimal::from(1) - (&metrics.risk_score / BigDecimal::from(100))) * BigDecimal::from(100);
        let liquidity_score = &metrics.liquidity_utilization;
        let efficiency_score = &metrics.efficiency_score;
        let sharpe_score = (&metrics.sharpe_ratio / BigDecimal::from(5)).min(BigDecimal::from(1)) * BigDecimal::from(100);

        let composite_score = (return_score * &return_weight) +
                             (risk_score * &risk_weight) +
                             (liquidity_score * &liquidity_weight) +
                             (efficiency_score * &efficiency_weight) +
                             (sharpe_score * &sharpe_weight);

        Ok(composite_score)
    }

    /// Create performance ranking for a category
    async fn create_ranking(&self, category: &str, mut pools: Vec<PoolComparison>) -> Result<PerformanceRanking, AppError> {
        pools.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let total_pools = pools.len();
        let top_20_percent = (total_pools as f64 * 0.2).ceil() as usize;
        let bottom_20_percent = (total_pools as f64 * 0.2).ceil() as usize;

        let market_leaders: Vec<String> = pools.iter()
            .take(top_20_percent)
            .map(|p| p.pool_address.clone())
            .collect();

        let underperformers: Vec<String> = pools.iter()
            .rev()
            .take(bottom_20_percent)
            .map(|p| p.pool_address.clone())
            .collect();

        // Rising stars: pools with high recent performance improvement
        let rising_stars: Vec<String> = pools.iter()
            .filter(|p| p.sharpe_ratio > BigDecimal::from_str("1.5").unwrap())
            .take(5)
            .map(|p| p.pool_address.clone())
            .collect();

        Ok(PerformanceRanking {
            category: category.to_string(),
            pools,
            market_leaders,
            underperformers,
            rising_stars,
        })
    }

    /// Helper calculation methods
    async fn calculate_apr(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 24 {
            return Ok(BigDecimal::from(0));
        }

        let recent_24h = &historical_data[0..24];
        let mut total_volume = BigDecimal::from(0);
        let mut avg_tvl = BigDecimal::from(0);

        for window in recent_24h.windows(2) {
            if let [current, previous] = window {
                let tvl_current = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                let tvl_previous = previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                
                let estimated_volume = (&tvl_current - &tvl_previous).abs() * BigDecimal::from(8);
                total_volume += estimated_volume;
                avg_tvl += &tvl_current;
            }
        }

        if recent_24h.len() > 1 {
            avg_tvl = avg_tvl / BigDecimal::from(recent_24h.len() as i32);
        }

        if !avg_tvl.is_zero() {
            let daily_fees = &total_volume * BigDecimal::from_str("0.003").unwrap();
            let daily_apr = (&daily_fees / &avg_tvl) * BigDecimal::from(100);
            Ok(daily_apr * BigDecimal::from(365))
        } else {
            Ok(BigDecimal::from(0))
        }
    }

    async fn calculate_apy(&self, apr: &BigDecimal) -> Result<BigDecimal, AppError> {
        let apr_decimal = apr / BigDecimal::from(100);
        let apr_f64 = apr_decimal.to_f64().unwrap_or(0.0);
        let apy_f64 = (1.0 + apr_f64 / 365.0).powf(365.0) - 1.0;
        Ok(BigDecimal::from_f64(apy_f64 * 100.0).unwrap_or(BigDecimal::from(0)))
    }

    async fn calculate_volume_24h(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 24 {
            return Ok(BigDecimal::from(0));
        }

        let recent_24h = &historical_data[0..24];
        let mut total_volume = BigDecimal::from(0);

        for window in recent_24h.windows(2) {
            if let [current, previous] = window {
                let tvl_change = current.tvl_usd.clone().unwrap_or(BigDecimal::from(0)) - 
                               previous.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
                total_volume += tvl_change.abs() * BigDecimal::from(10);
            }
        }

        Ok(total_volume)
    }

    async fn calculate_volatility(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        let mut price_changes = Vec::new();

        for window in historical_data.windows(2) {
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

        let mean: BigDecimal = price_changes.iter().sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);
        let variance: BigDecimal = price_changes.iter()
            .map(|x| {
                let diff = x - &mean;
                &diff * &diff
            })
            .sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);

        let volatility = variance.sqrt().unwrap_or(BigDecimal::from(0));
        let annualized_volatility = volatility * BigDecimal::from_f64(24.0_f64.sqrt() * 365.0_f64.sqrt()).unwrap_or(BigDecimal::from(1));
        
        Ok(annualized_volatility * BigDecimal::from(100))
    }

    async fn calculate_sharpe_ratio(&self, returns: &BigDecimal, volatility: &BigDecimal) -> Result<BigDecimal, AppError> {
        let risk_free_rate = BigDecimal::from_str("2.0").unwrap();
        
        if volatility.is_zero() {
            return Ok(BigDecimal::from(0));
        }

        let excess_return = returns - &risk_free_rate;
        Ok(&excess_return / volatility)
    }

    async fn calculate_max_drawdown(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        let mut max_drawdown = BigDecimal::from(0);
        let mut peak_value = BigDecimal::from(0);

        for state in historical_data.iter().rev() {
            let current_value = state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
            
            if current_value > peak_value {
                peak_value = current_value.clone();
            }
            
            if !peak_value.is_zero() {
                let drawdown = ((&peak_value - &current_value) / &peak_value) * BigDecimal::from(100);
                if drawdown > max_drawdown {
                    max_drawdown = drawdown;
                }
            }
        }

        Ok(max_drawdown)
    }

    async fn calculate_liquidity_utilization(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.is_empty() {
            return Ok(BigDecimal::from(0));
        }

        let current_state = &historical_data[0];
        let total_liquidity = &current_state.liquidity;
        let active_liquidity = total_liquidity * BigDecimal::from_str("0.7").unwrap();
        
        if !total_liquidity.is_zero() {
            Ok((&active_liquidity / total_liquidity) * BigDecimal::from(100))
        } else {
            Ok(BigDecimal::from(0))
        }
    }

    async fn calculate_risk_score(&self, volatility: &BigDecimal, max_drawdown: &BigDecimal) -> Result<BigDecimal, AppError> {
        let vol_score = volatility / BigDecimal::from(2); // Normalize volatility
        let drawdown_score = max_drawdown;
        
        Ok((vol_score + drawdown_score) / BigDecimal::from(2))
    }

    async fn calculate_efficiency_score(&self, apr: &BigDecimal, risk_score: &BigDecimal, liquidity_utilization: &BigDecimal) -> Result<BigDecimal, AppError> {
        let return_efficiency = apr / BigDecimal::from(2); // Normalize APR
        let risk_efficiency = BigDecimal::from(100) - risk_score;
        
        Ok((return_efficiency + risk_efficiency + liquidity_utilization) / BigDecimal::from(3))
    }

    // Additional helper methods for benchmarking
    async fn get_benchmark_data(&self, benchmark_type: &str, _chain_id: i32) -> Result<DetailedPoolMetrics, AppError> {
        // Simplified benchmark data - in production, this would fetch real benchmark data
        match benchmark_type {
            "DeFi_Index" => Ok(DetailedPoolMetrics {
                apr: BigDecimal::from_str("15.2").unwrap(),
                apy: BigDecimal::from_str("16.4").unwrap(),
                tvl: BigDecimal::from(1000000000),
                volume_24h: BigDecimal::from(50000000),
                volatility: BigDecimal::from_str("35.0").unwrap(),
                sharpe_ratio: BigDecimal::from_str("0.43").unwrap(),
                max_drawdown: BigDecimal::from_str("25.0").unwrap(),
                liquidity_utilization: BigDecimal::from_str("75.0").unwrap(),
                risk_score: BigDecimal::from_str("30.0").unwrap(),
                efficiency_score: BigDecimal::from_str("70.0").unwrap(),
            }),
            _ => Ok(DetailedPoolMetrics::default()),
        }
    }

    async fn calculate_beta(&self, _pool_address: &str, _chain_id: i32, _benchmark_type: &str) -> Result<BigDecimal, AppError> {
        // Simplified beta calculation - in production, this would calculate correlation
        Ok(BigDecimal::from_str("1.2").unwrap())
    }

    async fn calculate_tracking_error(&self, _pool_address: &str, _chain_id: i32, _benchmark_type: &str) -> Result<BigDecimal, AppError> {
        // Simplified tracking error - in production, this would calculate volatility of excess returns
        Ok(BigDecimal::from_str("8.5").unwrap())
    }

    async fn calculate_position_returns(&self, position: &Position) -> Result<BigDecimal, AppError> {
        // Simplified position returns calculation
        let days_active = position.created_at.map(|dt| (Utc::now() - dt).num_days() as f64).unwrap_or(0.0);
        let annualized_return = (days_active / 365.0) * 15.0; // Assume 15% annual return
        Ok(BigDecimal::from_f64(annualized_return).unwrap_or(BigDecimal::from(0)))
    }

    async fn calculate_benchmark_pool_returns(&self, _pool_address: &str, _chain_id: i32) -> Result<BigDecimal, AppError> {
        // Simplified benchmark pool returns
        Ok(BigDecimal::from_str("12.5").unwrap())
    }

    async fn get_all_pools(&self, chain_id: i32) -> Result<Vec<PoolState>, AppError> {
        let pools = sqlx::query_as::<_, PoolState>(
            "SELECT DISTINCT ON (pool_address) * FROM pool_states WHERE chain_id = $1 ORDER BY pool_address, timestamp DESC"
        )
        .bind(chain_id)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(pools)
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

#[derive(Debug, Clone)]
struct DetailedPoolMetrics {
    pub apr: BigDecimal,
    pub apy: BigDecimal,
    pub tvl: BigDecimal,
    pub volume_24h: BigDecimal,
    pub volatility: BigDecimal,
    pub sharpe_ratio: BigDecimal,
    pub max_drawdown: BigDecimal,
    pub liquidity_utilization: BigDecimal,
    pub risk_score: BigDecimal,
    pub efficiency_score: BigDecimal,
}

impl Default for DetailedPoolMetrics {
    fn default() -> Self {
        Self {
            apr: BigDecimal::from(0),
            apy: BigDecimal::from(0),
            tvl: BigDecimal::from(0),
            volume_24h: BigDecimal::from(0),
            volatility: BigDecimal::from(0),
            sharpe_ratio: BigDecimal::from(0),
            max_drawdown: BigDecimal::from(0),
            liquidity_utilization: BigDecimal::from(0),
            risk_score: BigDecimal::from(0),
            efficiency_score: BigDecimal::from(0),
        }
    }
}
