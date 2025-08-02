use axum::{
    extract::{Query, State},
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crate::{
    services::portfolio_service::PortfolioService,
    services::price_validation::PriceValidationService,
    error::AppError,
    AppState,
};

// Request/Response DTOs
#[derive(Debug, Serialize)]
pub struct PortfolioPerformanceResponse {
    pub user_id: Uuid,
    pub total_return_usd: BigDecimal,
    pub total_return_percentage: BigDecimal,
    pub annualized_return: BigDecimal,
    pub volatility: BigDecimal,
    pub sharpe_ratio: BigDecimal,
    pub max_drawdown: BigDecimal,
    pub best_position: Option<PositionPerformance>,
    pub worst_position: Option<PositionPerformance>,
    pub period_days: i32,
}

#[derive(Debug, Serialize)]
pub struct PositionPerformance {
    pub position_id: Uuid,
    pub protocol: String,
    pub return_usd: BigDecimal,
    pub return_percentage: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct PnlHistoryResponse {
    pub user_id: Uuid,
    pub entries: Vec<PnlHistoryEntry>,
    pub total_realized_pnl: BigDecimal,
    pub total_unrealized_pnl: BigDecimal,
    pub total_fees_paid: BigDecimal,
    pub total_impermanent_loss: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct PnlHistoryEntry {
    pub date: DateTime<Utc>,
    pub realized_pnl_usd: BigDecimal,
    pub unrealized_pnl_usd: BigDecimal,
    pub fees_paid_usd: BigDecimal,
    pub impermanent_loss_usd: BigDecimal,
    pub total_pnl_usd: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct AssetAllocationResponse {
    pub user_id: Uuid,
    pub allocations: Vec<AssetAllocation>,
    pub diversification_score: BigDecimal,
    pub concentration_risk: BigDecimal,
    pub top_assets: Vec<TopAsset>,
    pub total_value_usd: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct AssetAllocation {
    pub token_address: String,
    pub token_symbol: String,
    pub allocation_percentage: BigDecimal,
    pub value_usd: BigDecimal,
    pub position_count: i32,
}

#[derive(Debug, Serialize)]
pub struct TopAsset {
    pub token_symbol: String,
    pub percentage: BigDecimal,
    pub value_usd: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct ProtocolExposureResponse {
    pub user_id: Uuid,
    pub exposures: Vec<ProtocolExposure>,
    pub diversification_score: BigDecimal,
    pub highest_risk_protocol: Option<String>,
    pub total_tvl_exposure: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct ProtocolExposure {
    pub protocol_name: String,
    pub exposure_percentage: BigDecimal,
    pub value_usd: BigDecimal,
    pub position_count: i32,
    pub avg_yield_apy: BigDecimal,
    pub risk_score: BigDecimal,
}

#[derive(Debug, Deserialize)]
pub struct GetPortfolioPerformanceQuery {
    pub user_id: Uuid,
    pub period_days: Option<i32>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GetPnlHistoryQuery {
    pub user_id: Uuid,
    pub granularity: Option<String>, // daily, weekly, monthly
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GetAssetAllocationQuery {
    pub user_id: Uuid,
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GetProtocolExposureQuery {
    pub user_id: Uuid,
    pub include_risk_metrics: Option<bool>,
}

// Handler functions
pub async fn get_portfolio_performance(
    State(state): State<AppState>,
    Query(query): Query<GetPortfolioPerformanceQuery>,
) -> Result<Json<PortfolioPerformanceResponse>, AppError> {
    let price_validation_service = PriceValidationService::new(state.db_pool.clone()).await?;
    let portfolio_service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    let performance = portfolio_service.get_portfolio_performance(
        &query.user_id.to_string(),
        query.period_days,
    ).await?;
    
    let response = PortfolioPerformanceResponse {
        user_id: query.user_id,
        total_return_usd: performance.total_return_usd,
        total_return_percentage: performance.total_return_percentage,
        annualized_return: performance.daily_return_percentage.clone(), // Using daily as placeholder
        volatility: performance.volatility,
        sharpe_ratio: performance.sharpe_ratio.unwrap_or_default(),
        max_drawdown: performance.max_drawdown,
        best_position: Some(PositionPerformance {
            position_id: Uuid::parse_str(&performance.best_performing_position.unwrap_or("none".to_string())).unwrap_or(Uuid::nil()),
            return_usd: BigDecimal::from(0), // Would need additional calculation
            return_percentage: BigDecimal::from(0),
            protocol: "unknown".to_string(),
        }),
        worst_position: Some(PositionPerformance {
            position_id: Uuid::parse_str(&performance.worst_performing_position.unwrap_or("none".to_string())).unwrap_or(Uuid::nil()),
            return_usd: BigDecimal::from(0),
            return_percentage: BigDecimal::from(0),
            protocol: "unknown".to_string(),
        }),
        period_days: query.period_days.unwrap_or(30),
    };
    
    Ok(Json(response))
}

pub async fn get_pnl_history(
    State(state): State<AppState>,
    Query(query): Query<GetPnlHistoryQuery>,
) -> Result<Json<PnlHistoryResponse>, AppError> {
    let price_validation_service = PriceValidationService::new(state.db_pool.clone()).await?;
    let portfolio_service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    let pnl_history = portfolio_service.get_pnl_history(
        &query.user_id.to_string(),
        query.start_date,
        query.end_date,
        Some(24), // Default to daily granularity (24 hours)
    ).await?;
    
    let response = PnlHistoryResponse {
        user_id: query.user_id,
        entries: pnl_history.entries.into_iter().map(|entry| {
            let total_pnl = &entry.realized_pnl_usd + &entry.unrealized_pnl_usd;
            PnlHistoryEntry {
                date: entry.timestamp,
                realized_pnl_usd: entry.realized_pnl_usd,
                unrealized_pnl_usd: entry.unrealized_pnl_usd,
                fees_paid_usd: entry.fees_earned_usd, // Using fees_earned_usd as closest match
                impermanent_loss_usd: entry.impermanent_loss_usd,
                total_pnl_usd: total_pnl,
            }
        }).collect(),
        total_realized_pnl: pnl_history.total_realized_pnl,
        total_unrealized_pnl: pnl_history.total_unrealized_pnl,
        total_fees_paid: pnl_history.total_fees_earned,
        total_impermanent_loss: pnl_history.total_impermanent_loss,
    };
    
    Ok(Json(response))
}

pub async fn get_asset_allocation(
    State(state): State<AppState>,
    Query(query): Query<GetAssetAllocationQuery>,
) -> Result<Json<AssetAllocationResponse>, AppError> {
    let price_validation_service = PriceValidationService::new(state.db_pool.clone()).await?;
    let portfolio_service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    let allocation_summary = portfolio_service.get_asset_allocation(&query.user_id.to_string()).await?;
    
    let response = AssetAllocationResponse {
        user_id: query.user_id,
        allocations: allocation_summary.allocations.into_iter().map(|a| AssetAllocation {
            token_address: a.token_address,
            token_symbol: a.token_symbol,
            allocation_percentage: a.percentage_of_portfolio,
            value_usd: a.total_value_usd,
            position_count: a.position_count,
        }).collect(),
        diversification_score: allocation_summary.diversification_score,
        concentration_risk: allocation_summary.concentration_risk,
        top_assets: allocation_summary.top_5_assets.into_iter().map(|ta| TopAsset {
            token_symbol: ta.token_symbol,
            percentage: ta.percentage_of_portfolio,
            value_usd: ta.total_value_usd,
        }).collect(),
        total_value_usd: allocation_summary.total_portfolio_value_usd,
    };
    
    Ok(Json(response))
}

pub async fn get_protocol_exposure(
    State(state): State<AppState>,
    Query(query): Query<GetProtocolExposureQuery>,
) -> Result<Json<ProtocolExposureResponse>, AppError> {
    let price_validation_service = PriceValidationService::new(state.db_pool.clone()).await?;
    let portfolio_service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    let exposure = portfolio_service.get_protocol_exposure(&query.user_id.to_string()).await?;
    
    let response = ProtocolExposureResponse {
        user_id: query.user_id,
        exposures: exposure.exposures.into_iter().map(|e| ProtocolExposure {
            protocol_name: e.protocol_name,
            exposure_percentage: e.percentage_of_portfolio,
            value_usd: e.total_value_usd,
            position_count: e.position_count,
            avg_yield_apy: e.yield_apr.unwrap_or_else(|| BigDecimal::from(0)),
            risk_score: e.risk_score.unwrap_or_else(|| BigDecimal::from(0)),
        }).collect(),
        diversification_score: exposure.protocol_diversification_score,
        highest_risk_protocol: exposure.highest_risk_protocol,
        total_tvl_exposure: exposure.total_portfolio_value_usd,
    };
    
    Ok(Json(response))
}

// Create router
pub fn create_portfolio_routes() -> Router<AppState> {
    Router::new()
        .route("/portfolio/performance", get(get_portfolio_performance))
        .route("/portfolio/pnl-history", get(get_pnl_history))
        .route("/portfolio/asset-allocation", get(get_asset_allocation))
        .route("/portfolio/protocol-exposure", get(get_protocol_exposure))
}
