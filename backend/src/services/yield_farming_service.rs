use crate::models::PoolState;
use crate::error::types::AppError;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use sqlx::PgPool;
use std::collections::HashMap;
use std::str::FromStr;
use num_traits::Zero;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YieldFarmingMetrics {
    pub pool_address: String,
    pub chain_id: i32,
    pub base_apr: BigDecimal,
    pub reward_apr: BigDecimal,
    pub total_apr: BigDecimal,
    pub total_apy: BigDecimal,
    pub impermanent_loss_risk: BigDecimal,
    pub risk_adjusted_return: BigDecimal,
    pub sharpe_ratio: BigDecimal,
    pub max_drawdown: BigDecimal,
    pub volatility: BigDecimal,
    pub liquidity_mining_rewards: BigDecimal,
    pub fee_rewards: BigDecimal,
    pub compound_frequency: i32,
    pub optimal_rebalance_frequency: i32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FarmingStrategy {
    pub strategy_name: String,
    pub expected_apr: BigDecimal,
    pub risk_score: BigDecimal,
    pub min_investment: BigDecimal,
    pub max_investment: BigDecimal,
    pub rebalance_threshold: BigDecimal,
    pub gas_cost_impact: BigDecimal,
    pub strategy_description: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimalAllocation {
    pub pool_address: String,
    pub allocation_percentage: BigDecimal,
    pub expected_return: BigDecimal,
    pub risk_contribution: BigDecimal,
    pub sharpe_ratio: BigDecimal,
}

pub struct YieldFarmingService {
    db_pool: PgPool,
}

impl YieldFarmingService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Calculate comprehensive yield farming metrics
    pub async fn calculate_farming_metrics(&self, pool_address: &str, chain_id: i32) -> Result<YieldFarmingMetrics, AppError> {
        info!("Calculating yield farming metrics for pool {}", pool_address);

        let historical_data = self.get_historical_data(pool_address, chain_id, 720).await?; // 30 days
        
        if historical_data.is_empty() {
            return Err(AppError::ValidationError("No historical data available".to_string()));
        }

        // Calculate base metrics
        let base_apr = self.calculate_base_apr(&historical_data).await?;
        let reward_apr = self.calculate_reward_apr(pool_address, chain_id).await?;
        let total_apr = &base_apr + &reward_apr;
        let total_apy = self.calculate_apy(&total_apr, 365).await?;

        // Calculate risk metrics
        let volatility = self.calculate_volatility(&historical_data).await?;
        let max_drawdown = self.calculate_max_drawdown(&historical_data).await?;
        let impermanent_loss_risk = self.calculate_il_risk(&historical_data).await?;
        
        // Calculate risk-adjusted metrics
        let risk_adjusted_return = &total_apr - (&volatility * BigDecimal::from_str("0.5").unwrap());
        let sharpe_ratio = self.calculate_sharpe_ratio(&total_apr, &volatility).await?;

        // Calculate rewards breakdown
        let (liquidity_mining_rewards, fee_rewards) = self.calculate_rewards_breakdown(&historical_data, pool_address, chain_id).await?;

        // Optimal strategy parameters
        let compound_frequency = self.calculate_optimal_compound_frequency(&total_apr, &volatility).await?;
        let optimal_rebalance_frequency = self.calculate_optimal_rebalance_frequency(&impermanent_loss_risk).await?;

        Ok(YieldFarmingMetrics {
            pool_address: pool_address.to_string(),
            chain_id,
            base_apr,
            reward_apr,
            total_apr,
            total_apy,
            impermanent_loss_risk,
            risk_adjusted_return,
            sharpe_ratio,
            max_drawdown,
            volatility,
            liquidity_mining_rewards,
            fee_rewards,
            compound_frequency,
            optimal_rebalance_frequency,
        })
    }

    /// Generate optimal farming strategies
    pub async fn generate_farming_strategies(&self, investment_amount: &BigDecimal, risk_tolerance: f64) -> Result<Vec<FarmingStrategy>, AppError> {
        info!("Generating farming strategies for ${} with risk tolerance {}", investment_amount, risk_tolerance);

        let mut strategies = Vec::new();

        // Conservative Strategy
        if risk_tolerance >= 0.2 {
            strategies.push(FarmingStrategy {
                strategy_name: "Conservative Stable Pairs".to_string(),
                expected_apr: BigDecimal::from_str("8.5").unwrap(),
                risk_score: BigDecimal::from_str("0.2").unwrap(),
                min_investment: BigDecimal::from(1000),
                max_investment: BigDecimal::from(1000000),
                rebalance_threshold: BigDecimal::from_str("5.0").unwrap(),
                gas_cost_impact: BigDecimal::from_str("0.1").unwrap(),
                strategy_description: "Focus on stable coin pairs with low IL risk and consistent yields".to_string(),
            });
        }

        // Moderate Strategy
        if risk_tolerance >= 0.4 {
            strategies.push(FarmingStrategy {
                strategy_name: "Moderate Blue Chip".to_string(),
                expected_apr: BigDecimal::from_str("15.2").unwrap(),
                risk_score: BigDecimal::from_str("0.4").unwrap(),
                min_investment: BigDecimal::from(5000),
                max_investment: BigDecimal::from(500000),
                rebalance_threshold: BigDecimal::from_str("10.0").unwrap(),
                gas_cost_impact: BigDecimal::from_str("0.2").unwrap(),
                strategy_description: "ETH/BTC pairs with moderate IL risk and higher yields".to_string(),
            });
        }

        // Aggressive Strategy
        if risk_tolerance >= 0.6 {
            strategies.push(FarmingStrategy {
                strategy_name: "Aggressive High Yield".to_string(),
                expected_apr: BigDecimal::from_str("35.8").unwrap(),
                risk_score: BigDecimal::from_str("0.7").unwrap(),
                min_investment: BigDecimal::from(10000),
                max_investment: BigDecimal::from(100000),
                rebalance_threshold: BigDecimal::from_str("20.0").unwrap(),
                gas_cost_impact: BigDecimal::from_str("0.5").unwrap(),
                strategy_description: "Alt coin pairs with high IL risk but maximum yield potential".to_string(),
            });
        }

        // DeFi Native Strategy
        if risk_tolerance >= 0.8 {
            strategies.push(FarmingStrategy {
                strategy_name: "DeFi Native Maximalist".to_string(),
                expected_apr: BigDecimal::from_str("65.4").unwrap(),
                risk_score: BigDecimal::from_str("0.9").unwrap(),
                min_investment: BigDecimal::from(25000),
                max_investment: BigDecimal::from(50000),
                rebalance_threshold: BigDecimal::from_str("30.0").unwrap(),
                gas_cost_impact: BigDecimal::from_str("1.0").unwrap(),
                strategy_description: "Experimental DeFi tokens with extreme risk and reward potential".to_string(),
            });
        }

        Ok(strategies)
    }

    /// Calculate optimal portfolio allocation across multiple pools
    pub async fn calculate_optimal_allocation(&self, pools: &[String], investment_amount: &BigDecimal, risk_tolerance: f64) -> Result<Vec<OptimalAllocation>, AppError> {
        info!("Calculating optimal allocation for {} pools", pools.len());

        let mut allocations = Vec::new();
        let mut total_weight = BigDecimal::from(0);

        // Calculate metrics for each pool
        let mut pool_metrics = HashMap::new();
        for pool_address in pools {
            let metrics = self.calculate_farming_metrics(pool_address, 1).await?;
            let weight = self.calculate_pool_weight(&metrics, risk_tolerance).await?;
            pool_metrics.insert(pool_address.clone(), (metrics, weight.clone()));
            total_weight += weight;
        }

        // Normalize allocations
        for (pool_address, (metrics, weight)) in pool_metrics {
            let allocation_percentage = if !total_weight.is_zero() {
                (&weight / &total_weight) * BigDecimal::from(100)
            } else {
                BigDecimal::from(100) / BigDecimal::from(pools.len() as i32)
            };

            let allocated_amount = (investment_amount * &allocation_percentage) / BigDecimal::from(100);
            let expected_return = (&allocated_amount * &metrics.total_apr) / BigDecimal::from(100);

            allocations.push(OptimalAllocation {
                pool_address,
                allocation_percentage: allocation_percentage.clone(),
                expected_return,
                risk_contribution: &metrics.volatility * &allocation_percentage / BigDecimal::from(100),
                sharpe_ratio: metrics.sharpe_ratio,
            });
        }

        // Sort by expected return descending
        allocations.sort_by(|a, b| b.expected_return.partial_cmp(&a.expected_return).unwrap_or(std::cmp::Ordering::Equal));

        Ok(allocations)
    }

    /// Calculate base APR from trading fees
    async fn calculate_base_apr(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
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
                
                // Estimate volume from TVL and price changes
                let estimated_volume = (&tvl_current - &tvl_previous).abs() * BigDecimal::from(8);
                total_volume += estimated_volume;
                avg_tvl += &tvl_current;
            }
        }

        if recent_24h.len() > 1 {
            avg_tvl = avg_tvl / BigDecimal::from(recent_24h.len() as i32);
        }

        if !avg_tvl.is_zero() {
            let daily_fees = &total_volume * BigDecimal::from_str("0.003").unwrap(); // 0.3% fee
            let daily_apr = (&daily_fees / &avg_tvl) * BigDecimal::from(100);
            Ok(daily_apr * BigDecimal::from(365)) // Annualize
        } else {
            Ok(BigDecimal::from(0))
        }
    }

    /// Calculate reward APR from liquidity mining
    async fn calculate_reward_apr(&self, _pool_address: &str, _chain_id: i32) -> Result<BigDecimal, AppError> {
        // Simplified - in production, this would fetch actual reward rates
        Ok(BigDecimal::from_str("12.5").unwrap()) // 12.5% reward APR
    }

    /// Calculate APY from APR
    async fn calculate_apy(&self, apr: &BigDecimal, compounds_per_year: i32) -> Result<BigDecimal, AppError> {
        let apr_decimal = apr / BigDecimal::from(100);
        let apr_f64 = apr_decimal.to_f64().unwrap_or(0.0);
        let n_f64 = compounds_per_year as f64;
        
        let apy_f64 = (1.0 + apr_f64 / n_f64).powf(n_f64) - 1.0;
        
        Ok(BigDecimal::from_f64(apy_f64 * 100.0).unwrap_or(BigDecimal::from(0)))
    }

    /// Calculate volatility
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

        // Calculate standard deviation
        let mean: BigDecimal = price_changes.iter().sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);
        let variance: BigDecimal = price_changes.iter()
            .map(|x| {
                let diff = x - &mean;
                &diff * &diff
            })
            .sum::<BigDecimal>() / BigDecimal::from(price_changes.len() as i32);

        let volatility = variance.sqrt().unwrap_or(BigDecimal::from(0));
        
        // Annualize volatility (assuming hourly data)
        let annualized_volatility = volatility * BigDecimal::from_f64(24.0_f64.sqrt() * 365.0_f64.sqrt()).unwrap_or(BigDecimal::from(1));
        
        Ok(annualized_volatility * BigDecimal::from(100))
    }

    /// Calculate maximum drawdown
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

    /// Calculate impermanent loss risk
    async fn calculate_il_risk(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        let first_state = &historical_data[historical_data.len() - 1];
        let last_state = &historical_data[0];

        let initial_price0 = first_state.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
        let initial_price1 = first_state.token1_price_usd.clone().unwrap_or(BigDecimal::from(1));
        let final_price0 = last_state.token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
        let final_price1 = last_state.token1_price_usd.clone().unwrap_or(BigDecimal::from(1));

        if initial_price0.is_zero() || initial_price1.is_zero() {
            return Ok(BigDecimal::from(0));
        }

        // Calculate price ratio changes
        let initial_ratio = &initial_price0 / &initial_price1;
        let final_ratio = &final_price0 / &final_price1;

        if initial_ratio.is_zero() {
            return Ok(BigDecimal::from(0));
        }

        let ratio_change = (&final_ratio / &initial_ratio).to_f64().unwrap_or(1.0);
        
        // IL formula: IL = 2 * sqrt(ratio) / (1 + ratio) - 1
        let il = 2.0 * ratio_change.sqrt() / (1.0 + ratio_change) - 1.0;
        
        Ok(BigDecimal::from_f64(il.abs() * 100.0).unwrap_or(BigDecimal::from(0)))
    }

    /// Calculate Sharpe ratio
    async fn calculate_sharpe_ratio(&self, returns: &BigDecimal, volatility: &BigDecimal) -> Result<BigDecimal, AppError> {
        let risk_free_rate = BigDecimal::from_str("2.0").unwrap(); // 2% risk-free rate
        
        if volatility.is_zero() {
            return Ok(BigDecimal::from(0));
        }

        let excess_return = returns - &risk_free_rate;
        Ok(&excess_return / volatility)
    }

    /// Calculate rewards breakdown
    async fn calculate_rewards_breakdown(&self, historical_data: &[PoolState], _pool_address: &str, _chain_id: i32) -> Result<(BigDecimal, BigDecimal), AppError> {
        let base_apr = self.calculate_base_apr(historical_data).await?;
        let reward_apr = self.calculate_reward_apr("", 0).await?;
        
        // Assume current TVL for calculation
        let current_tvl = historical_data.first()
            .and_then(|s| s.tvl_usd.clone())
            .unwrap_or(BigDecimal::from(0));

        let fee_rewards = (&current_tvl * &base_apr) / BigDecimal::from(100);
        let liquidity_mining_rewards = (&current_tvl * &reward_apr) / BigDecimal::from(100);

        Ok((liquidity_mining_rewards, fee_rewards))
    }

    /// Calculate optimal compound frequency
    async fn calculate_optimal_compound_frequency(&self, apr: &BigDecimal, volatility: &BigDecimal) -> Result<i32, AppError> {
        let apr_f64 = apr.to_f64().unwrap_or(0.0);
        let vol_f64 = volatility.to_f64().unwrap_or(0.0);

        // Higher APR and lower volatility = more frequent compounding
        let frequency = if apr_f64 > 50.0 && vol_f64 < 20.0 {
            365 // Daily
        } else if apr_f64 > 20.0 && vol_f64 < 40.0 {
            52 // Weekly
        } else if apr_f64 > 10.0 {
            12 // Monthly
        } else {
            4 // Quarterly
        };

        Ok(frequency)
    }

    /// Calculate optimal rebalance frequency
    async fn calculate_optimal_rebalance_frequency(&self, il_risk: &BigDecimal) -> Result<i32, AppError> {
        let il_f64 = il_risk.to_f64().unwrap_or(0.0);

        // Higher IL risk = more frequent rebalancing
        let frequency = if il_f64 > 20.0 {
            365 // Daily
        } else if il_f64 > 10.0 {
            52 // Weekly
        } else if il_f64 > 5.0 {
            12 // Monthly
        } else {
            4 // Quarterly
        };

        Ok(frequency)
    }

    /// Calculate pool weight for allocation
    async fn calculate_pool_weight(&self, metrics: &YieldFarmingMetrics, risk_tolerance: f64) -> Result<BigDecimal, AppError> {
        let return_score = metrics.total_apr.to_f64().unwrap_or(0.0) / 100.0;
        let risk_score = metrics.volatility.to_f64().unwrap_or(0.0) / 100.0;
        let sharpe_score = metrics.sharpe_ratio.to_f64().unwrap_or(0.0);

        // Weight based on risk-adjusted returns and user risk tolerance
        let weight = (return_score * risk_tolerance) + (sharpe_score * 0.3) - (risk_score * (1.0 - risk_tolerance));
        
        Ok(BigDecimal::from_f64(weight.max(0.0)).unwrap_or(BigDecimal::from(0)))
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
