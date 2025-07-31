use crate::models::position::Position;
use crate::error::types::AppError;
use crate::services::price_validation::PriceValidationService;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::{info, warn, error};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PositionSummary {
    pub id: String,
    pub pool_address: String,
    pub current_value_usd: BigDecimal,
    pub entry_value_usd: BigDecimal,
    pub pnl_usd: BigDecimal,
    pub fees_usd: BigDecimal,
    pub risk_score: Option<BigDecimal>,
    pub protocol: String,
    pub chain: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioSummary {
    pub user_address: String,
    pub total_value_usd: BigDecimal,
    pub total_pnl_usd: BigDecimal,
    pub total_fees_usd: BigDecimal,
    pub positions: Vec<PositionSummary>,
    pub protocol_breakdown: HashMap<String, BigDecimal>,
    pub chain_breakdown: HashMap<String, BigDecimal>,
    pub risk_aggregation: HashMap<String, BigDecimal>,
    pub historical_values: Vec<(DateTime<Utc>, BigDecimal)>,
}

// Portfolio Analytics Data Structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortfolioPerformance {
    pub user_address: String,
    pub total_return_usd: BigDecimal,
    pub total_return_percentage: BigDecimal,
    pub daily_return_percentage: BigDecimal,
    pub weekly_return_percentage: BigDecimal,
    pub monthly_return_percentage: BigDecimal,
    pub sharpe_ratio: Option<BigDecimal>,
    pub max_drawdown: BigDecimal,
    pub volatility: BigDecimal,
    pub best_performing_position: Option<String>,
    pub worst_performing_position: Option<String>,
    pub performance_period_days: i32,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PnlHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub total_value_usd: BigDecimal,
    pub realized_pnl_usd: BigDecimal,
    pub unrealized_pnl_usd: BigDecimal,
    pub fees_earned_usd: BigDecimal,
    pub impermanent_loss_usd: BigDecimal,
    pub position_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PnlHistory {
    pub user_address: String,
    pub entries: Vec<PnlHistoryEntry>,
    pub total_realized_pnl: BigDecimal,
    pub total_unrealized_pnl: BigDecimal,
    pub total_fees_earned: BigDecimal,
    pub total_impermanent_loss: BigDecimal,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetAllocation {
    pub token_address: String,
    pub token_symbol: String,
    pub token_name: String,
    pub total_amount: BigDecimal,
    pub total_value_usd: BigDecimal,
    pub percentage_of_portfolio: BigDecimal,
    pub position_count: i32,
    pub protocols: Vec<String>,
    pub chains: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssetAllocationSummary {
    pub user_address: String,
    pub total_portfolio_value_usd: BigDecimal,
    pub allocations: Vec<AssetAllocation>,
    pub top_5_assets: Vec<AssetAllocation>,
    pub diversification_score: BigDecimal, // 0-100, higher = more diversified
    pub concentration_risk: BigDecimal, // Percentage in top asset
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolExposure {
    pub protocol_name: String,
    pub total_value_usd: BigDecimal,
    pub percentage_of_portfolio: BigDecimal,
    pub position_count: i32,
    pub avg_position_size_usd: BigDecimal,
    pub chains: Vec<String>,
    pub risk_score: Option<BigDecimal>,
    pub tvl_usd: Option<BigDecimal>,
    pub yield_apr: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProtocolExposureSummary {
    pub user_address: String,
    pub total_portfolio_value_usd: BigDecimal,
    pub exposures: Vec<ProtocolExposure>,
    pub top_5_protocols: Vec<ProtocolExposure>,
    pub protocol_diversification_score: BigDecimal,
    pub highest_risk_protocol: Option<String>,
    pub last_updated: DateTime<Utc>,
}

pub struct PortfolioService {
    db_pool: PgPool,
    price_validation_service: PriceValidationService,
}

impl PortfolioService {
    pub async fn new(db_pool: PgPool, price_validation_service: PriceValidationService) -> Self {
        Self { 
            db_pool,
            price_validation_service,
        }
    }

    /// Aggregate all positions for a user and return a portfolio summary
    pub async fn get_portfolio_summary(&mut self, user_address: &str) -> Result<PortfolioSummary, AppError> {
        // Fetch all positions for the user
        let positions: Vec<Position> = sqlx::query_as!(
            Position,
            "SELECT id, user_address, protocol, pool_address, token0_address, token1_address, 
             token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, chain_id,
             entry_token0_price_usd, entry_token1_price_usd, entry_timestamp, created_at, updated_at
             FROM positions WHERE user_address = $1",
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut total_value_usd = BigDecimal::from(0);
        let mut total_pnl_usd = BigDecimal::from(0);
        let mut total_fees_usd = BigDecimal::from(0);
        let mut protocol_breakdown = HashMap::new();
        let mut chain_breakdown = HashMap::new();
        let risk_aggregation = HashMap::new();
        let mut positions_summary = Vec::new();

        // TODO: fetch risk scores and fees per position if available
        for pos in &positions {
            // Calculate current position value using real-time prices
            let token0_price = match self.price_validation_service.get_validated_price(&pos.token0_address, pos.chain_id).await {
                Ok(validated_price) => validated_price.price_usd,
                Err(e) => {
                    tracing::warn!("Failed to fetch price for token0 {}: {}, using fallback", pos.token0_address, e);
                    BigDecimal::from(1) // Fallback price
                }
            };
            
            let token1_price = match self.price_validation_service.get_validated_price(&pos.token1_address, pos.chain_id).await {
                Ok(validated_price) => validated_price.price_usd,
                Err(e) => {
                    tracing::warn!("Failed to fetch price for token1 {}: {}, using fallback", pos.token1_address, e);
                    BigDecimal::from(1) // Fallback price
                }
            };
            
            let current_value = pos.calculate_position_value_usd(token0_price.clone(), token1_price.clone());
            let entry_value = pos.entry_token0_price_usd.clone().unwrap_or(BigDecimal::from(0)) + pos.entry_token1_price_usd.clone().unwrap_or(BigDecimal::from(0));
            let pnl = &current_value - &entry_value;
            let fees = BigDecimal::from(0); // Placeholder, replace with actual fees if tracked
            let protocol = pos.pool_address.clone(); // Placeholder, replace with actual protocol name
            let chain = "mainnet".to_string(); // Placeholder, replace with actual chain

            total_value_usd += &current_value;
            total_pnl_usd += &pnl;
            total_fees_usd += &fees;

            *protocol_breakdown.entry(protocol.clone()).or_insert(BigDecimal::from(0)) += &current_value;
            *chain_breakdown.entry(chain.clone()).or_insert(BigDecimal::from(0)) += &current_value;

            positions_summary.push(PositionSummary {
                id: pos.id.to_string(),
                pool_address: pos.pool_address.clone(),
                current_value_usd: current_value.clone(),
                entry_value_usd: entry_value.clone(),
                pnl_usd: pnl,
                fees_usd: fees,
                risk_score: None, // TODO: fetch risk score per position
                protocol,
                chain,
            });
        }

        // TODO: Aggregate risk scores and historical values
        let historical_values = vec![];

        Ok(PortfolioSummary {
            user_address: user_address.to_string(),
            total_value_usd,
            total_pnl_usd,
            total_fees_usd,
            positions: positions_summary,
            protocol_breakdown,
            chain_breakdown,
            risk_aggregation,
            historical_values,
        })
    }

    /// Get comprehensive portfolio performance metrics
    pub async fn get_portfolio_performance(
        &self,
        user_address: &str,
        period_days: Option<i32>,
    ) -> Result<PortfolioPerformance, AppError> {
        info!("Getting portfolio performance for user: {} over {} days", user_address, period_days.unwrap_or(30));
        
        let period_days = period_days.unwrap_or(30);
        let cutoff_date = Utc::now() - Duration::days(period_days as i64);
        
        // Get current portfolio value
        let current_positions = sqlx::query_as!(
            Position,
            "SELECT id, user_address, protocol, pool_address, token0_address, token1_address, 
             token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, chain_id,
             entry_token0_price_usd, entry_token1_price_usd, entry_timestamp, created_at, updated_at
             FROM positions WHERE user_address = $1 AND created_at >= $2",
            user_address,
            cutoff_date
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Calculate performance metrics
        let mut total_current_value = BigDecimal::from(0);
        let mut total_entry_value = BigDecimal::from(0);
        let mut best_performing_position: Option<String> = None;
        let mut worst_performing_position: Option<String> = None;
        let mut best_performance = BigDecimal::from(-100);
        let mut worst_performance = BigDecimal::from(100);

        for position in &current_positions {
            // Calculate current value (simplified - in production would use real-time prices)
            let entry_value = position.entry_token0_price_usd.clone().unwrap_or(BigDecimal::from(1000)) + 
                             position.entry_token1_price_usd.clone().unwrap_or(BigDecimal::from(1000));
            let current_value = &entry_value * "1.05".parse::<BigDecimal>().unwrap(); // Simplified 5% gain assumption
            
            let performance = ((&current_value - &entry_value) / &entry_value) * BigDecimal::from(100);
            
            if performance > best_performance {
                best_performance = performance.clone();
                best_performing_position = Some(position.id.to_string());
            }
            
            if performance < worst_performance {
                worst_performance = performance.clone();
                worst_performing_position = Some(position.id.to_string());
            }
            
            total_current_value += current_value;
            total_entry_value += entry_value;
        }

        // Calculate overall performance metrics
        let total_return_usd = &total_current_value - &total_entry_value;
        let total_return_percentage = if total_entry_value > BigDecimal::from(0) {
            (&total_return_usd / &total_entry_value) * BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };

        // Simplified calculations (in production would use historical data)
        let daily_return = &total_return_percentage / BigDecimal::from(period_days);
        let weekly_return = &daily_return * BigDecimal::from(7);
        let monthly_return = &daily_return * BigDecimal::from(30);
        
        Ok(PortfolioPerformance {
            user_address: user_address.to_string(),
            total_return_usd,
            total_return_percentage,
            daily_return_percentage: daily_return,
            weekly_return_percentage: weekly_return,
            monthly_return_percentage: monthly_return,
            sharpe_ratio: Some("1.2".parse::<BigDecimal>().unwrap()), // Placeholder
            max_drawdown: "15.5".parse::<BigDecimal>().unwrap(), // Placeholder
            volatility: "25.3".parse::<BigDecimal>().unwrap(), // Placeholder
            best_performing_position,
            worst_performing_position,
            performance_period_days: period_days,
            last_updated: Utc::now(),
        })
    }

    /// Get detailed P&L history with breakdown
    pub async fn get_pnl_history(
        &self,
        user_address: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity_hours: Option<i32>,
    ) -> Result<PnlHistory, AppError> {
        info!("Getting P&L history for user: {}", user_address);
        
        let start_date = start_date.unwrap_or(Utc::now() - Duration::days(30));
        let end_date = end_date.unwrap_or(Utc::now());
        let granularity_hours = granularity_hours.unwrap_or(24); // Daily by default
        
        // Get historical position snapshots (simplified - would need actual historical data table)
        let positions = sqlx::query_as!(
            Position,
            "SELECT id, user_address, protocol, pool_address, token0_address, token1_address, 
             token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, chain_id,
             entry_token0_price_usd, entry_token1_price_usd, entry_timestamp, created_at, updated_at
             FROM positions 
             WHERE user_address = $1 AND created_at BETWEEN $2 AND $3
             ORDER BY created_at ASC",
            user_address,
            start_date,
            end_date
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut entries = Vec::new();
        let mut total_realized_pnl = BigDecimal::from(0);
        let mut total_unrealized_pnl = BigDecimal::from(0);
        let mut total_fees_earned = BigDecimal::from(0);
        let mut total_impermanent_loss = BigDecimal::from(0);

        // Generate time series entries (simplified implementation)
        let mut current_time = start_date;
        while current_time <= end_date {
            let positions_at_time: Vec<&Position> = positions
                .iter()
                .filter(|p| p.created_at.unwrap_or(start_date) <= current_time)
                .collect();

            let mut total_value = BigDecimal::from(0);
            let mut realized_pnl = BigDecimal::from(0);
            let mut unrealized_pnl = BigDecimal::from(0);
            let mut fees_earned = BigDecimal::from(50); // Placeholder
            let mut impermanent_loss = BigDecimal::from(25); // Placeholder

            for position in &positions_at_time {
                let entry_value = position.entry_token0_price_usd.clone().unwrap_or(BigDecimal::from(1000)) + 
                                 position.entry_token1_price_usd.clone().unwrap_or(BigDecimal::from(1000));
                let current_value = &entry_value * "1.03".parse::<BigDecimal>().unwrap(); // Simplified 3% gain
                
                total_value += &current_value;
                unrealized_pnl += &current_value - &entry_value;
            }

            entries.push(PnlHistoryEntry {
                timestamp: current_time,
                total_value_usd: total_value,
                realized_pnl_usd: realized_pnl.clone(),
                unrealized_pnl_usd: unrealized_pnl.clone(),
                fees_earned_usd: fees_earned.clone(),
                impermanent_loss_usd: impermanent_loss.clone(),
                position_count: positions_at_time.len() as i32,
            });

            total_realized_pnl += realized_pnl;
            total_unrealized_pnl += unrealized_pnl;
            total_fees_earned += fees_earned;
            total_impermanent_loss += impermanent_loss;

            current_time += Duration::hours(granularity_hours as i64);
        }

        Ok(PnlHistory {
            user_address: user_address.to_string(),
            entries,
            total_realized_pnl,
            total_unrealized_pnl,
            total_fees_earned,
            total_impermanent_loss,
            period_start: start_date,
            period_end: end_date,
        })
    }

    /// Get detailed asset allocation breakdown
    pub async fn get_asset_allocation(
        &self,
        user_address: &str,
    ) -> Result<AssetAllocationSummary, AppError> {
        info!("Getting asset allocation for user: {}", user_address);
        
        // Get all positions for the user
        let positions = sqlx::query_as!(
            Position,
            "SELECT id, user_address, protocol, pool_address, token0_address, token1_address, 
             token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, chain_id,
             entry_token0_price_usd, entry_token1_price_usd, entry_timestamp, created_at, updated_at
             FROM positions WHERE user_address = $1",
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut token_allocations: HashMap<String, AssetAllocation> = HashMap::new();
        let mut total_portfolio_value = BigDecimal::from(0);

        // Aggregate token allocations across all positions
        for position in &positions {
            // Process token0
            let token0_value = &position.token0_amount * position.entry_token0_price_usd.clone().unwrap_or(BigDecimal::from(1));
            let token0_allocation = token_allocations.entry(position.token0_address.clone()).or_insert(AssetAllocation {
                token_address: position.token0_address.clone(),
                token_symbol: format!("TOKEN0_{}", &position.token0_address[..6]), // Placeholder
                token_name: format!("Token 0 ({})", &position.token0_address[..6]), // Placeholder
                total_amount: BigDecimal::from(0),
                total_value_usd: BigDecimal::from(0),
                percentage_of_portfolio: BigDecimal::from(0),
                position_count: 0,
                protocols: Vec::new(),
                chains: Vec::new(),
            });
            
            token0_allocation.total_amount += &position.token0_amount;
            token0_allocation.total_value_usd += &token0_value;
            token0_allocation.position_count += 1;
            if !token0_allocation.protocols.contains(&position.protocol) {
                token0_allocation.protocols.push(position.protocol.clone());
            }
            if !token0_allocation.chains.contains(&position.chain_id.to_string()) {
                token0_allocation.chains.push(position.chain_id.to_string());
            }
            total_portfolio_value += &token0_value;

            // Process token1
            let token1_value = &position.token1_amount * position.entry_token1_price_usd.clone().unwrap_or(BigDecimal::from(1));
            let token1_allocation = token_allocations.entry(position.token1_address.clone()).or_insert(AssetAllocation {
                token_address: position.token1_address.clone(),
                token_symbol: format!("TOKEN1_{}", &position.token1_address[..6]), // Placeholder
                token_name: format!("Token 1 ({})", &position.token1_address[..6]), // Placeholder
                total_amount: BigDecimal::from(0),
                total_value_usd: BigDecimal::from(0),
                percentage_of_portfolio: BigDecimal::from(0),
                position_count: 0,
                protocols: Vec::new(),
                chains: Vec::new(),
            });
            
            token1_allocation.total_amount += &position.token1_amount;
            token1_allocation.total_value_usd += &token1_value;
            token1_allocation.position_count += 1;
            if !token1_allocation.protocols.contains(&position.protocol) {
                token1_allocation.protocols.push(position.protocol.clone());
            }
            if !token1_allocation.chains.contains(&position.chain_id.to_string()) {
                token1_allocation.chains.push(position.chain_id.to_string());
            }
            total_portfolio_value += &token1_value;
        }

        // Calculate percentages and sort by value
        let mut allocations: Vec<AssetAllocation> = token_allocations.into_values().collect();
        for allocation in &mut allocations {
            allocation.percentage_of_portfolio = if total_portfolio_value > BigDecimal::from(0) {
                (&allocation.total_value_usd / &total_portfolio_value) * BigDecimal::from(100)
            } else {
                BigDecimal::from(0)
            };
        }
        
        // Sort by value descending
        allocations.sort_by(|a, b| b.total_value_usd.cmp(&a.total_value_usd));
        
        // Get top 5 assets
        let top_5_assets = allocations.iter().take(5).cloned().collect();
        
        // Calculate diversification metrics
        let concentration_risk = allocations.first()
            .map(|a| a.percentage_of_portfolio.clone())
            .unwrap_or(BigDecimal::from(0));
        
        let diversification_score = if allocations.len() > 1 {
            // Simple diversification score: 100 - concentration_risk
            BigDecimal::from(100) - &concentration_risk
        } else {
            BigDecimal::from(0)
        };

        Ok(AssetAllocationSummary {
            user_address: user_address.to_string(),
            total_portfolio_value_usd: total_portfolio_value,
            allocations,
            top_5_assets,
            diversification_score,
            concentration_risk,
            last_updated: Utc::now(),
        })
    }

    /// Get protocol exposure breakdown
    pub async fn get_protocol_exposure(
        &self,
        user_address: &str,
    ) -> Result<ProtocolExposureSummary, AppError> {
        info!("Getting protocol exposure for user: {}", user_address);
        
        // Get all positions for the user
        let positions = sqlx::query_as!(
            Position,
            "SELECT id, user_address, protocol, pool_address, token0_address, token1_address, 
             token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, chain_id,
             entry_token0_price_usd, entry_token1_price_usd, entry_timestamp, created_at, updated_at
             FROM positions WHERE user_address = $1",
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut protocol_exposures: HashMap<String, ProtocolExposure> = HashMap::new();
        let mut total_portfolio_value = BigDecimal::from(0);

        // Aggregate protocol exposures
        for position in &positions {
            let position_value = (position.entry_token0_price_usd.clone().unwrap_or(BigDecimal::from(1000)) * &position.token0_amount) +
                               (position.entry_token1_price_usd.clone().unwrap_or(BigDecimal::from(1000)) * &position.token1_amount);
            
            let protocol_exposure = protocol_exposures.entry(position.protocol.clone()).or_insert(ProtocolExposure {
                protocol_name: position.protocol.clone(),
                total_value_usd: BigDecimal::from(0),
                percentage_of_portfolio: BigDecimal::from(0),
                position_count: 0,
                avg_position_size_usd: BigDecimal::from(0),
                chains: Vec::new(),
                risk_score: Some(BigDecimal::from(65)), // Placeholder
                tvl_usd: Some(BigDecimal::from(1000000000)), // Placeholder $1B TVL
                yield_apr: Some("8.5".parse::<BigDecimal>().unwrap()), // Placeholder 8.5% APR
            });
            
            protocol_exposure.total_value_usd += &position_value;
            protocol_exposure.position_count += 1;
            if !protocol_exposure.chains.contains(&position.chain_id.to_string()) {
                protocol_exposure.chains.push(position.chain_id.to_string());
            }
            
            total_portfolio_value += &position_value;
        }

        // Calculate percentages and averages
        let mut exposures: Vec<ProtocolExposure> = protocol_exposures.into_values().collect();
        for exposure in &mut exposures {
            exposure.percentage_of_portfolio = if total_portfolio_value > BigDecimal::from(0) {
                (&exposure.total_value_usd / &total_portfolio_value) * BigDecimal::from(100)
            } else {
                BigDecimal::from(0)
            };
            
            exposure.avg_position_size_usd = if exposure.position_count > 0 {
                &exposure.total_value_usd / BigDecimal::from(exposure.position_count)
            } else {
                BigDecimal::from(0)
            };
        }
        
        // Sort by value descending
        exposures.sort_by(|a, b| b.total_value_usd.cmp(&a.total_value_usd));
        
        // Get top 5 protocols
        let top_5_protocols = exposures.iter().take(5).cloned().collect();
        
        // Calculate diversification score
        let protocol_diversification_score = if exposures.len() > 1 {
            let top_protocol_percentage = exposures.first()
                .map(|e| e.percentage_of_portfolio.clone())
                .unwrap_or(BigDecimal::from(0));
            BigDecimal::from(100) - &top_protocol_percentage
        } else {
            BigDecimal::from(0)
        };
        
        // Find highest risk protocol
        let highest_risk_protocol = exposures.iter()
            .max_by(|a, b| {
                let risk_a = a.risk_score.clone().unwrap_or(BigDecimal::from(0));
                let risk_b = b.risk_score.clone().unwrap_or(BigDecimal::from(0));
                risk_a.cmp(&risk_b)
            })
            .map(|e| e.protocol_name.clone());

        Ok(ProtocolExposureSummary {
            user_address: user_address.to_string(),
            total_portfolio_value_usd: total_portfolio_value,
            exposures,
            top_5_protocols,
            protocol_diversification_score,
            highest_risk_protocol,
            last_updated: Utc::now(),
        })
    }
}
