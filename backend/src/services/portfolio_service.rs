use crate::models::position::Position;
use crate::error::types::AppError;
use crate::services::price_validation::PriceValidationService;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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
}
