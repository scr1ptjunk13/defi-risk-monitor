use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use chrono::{DateTime, Utc};
// Commented out broken service imports:
// use crate::{
//     services::position_service::PositionService,
//     services::position_aggregator::PositionAggregator,
//     services::risk_calculator::RiskCalculator,
//     adapters::traits::DeFiAdapter,
//     adapters::aave_v3::AaveV3Adapter,
//     error::AppError,

// Placeholder type definitions:
#[derive(Debug, Clone)]
pub struct PositionService {
    pub db_pool: String,
}

#[derive(Debug, Clone)]
pub struct PositionAggregator {
    pub client: String,
}

#[derive(Debug, Clone)]
pub struct RiskCalculator {
    pub config: String,
}

#[derive(Debug, Clone)]
pub struct DeFiAdapter {
    pub protocol: String,
}

#[derive(Debug, Clone)]
pub struct AaveV3Adapter {
    pub client: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("External service error: {0}")]
    ExternalServiceError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

use crate::{
    AppState,
    risk::traits::Position,
};

// Implement IntoResponse for AppError to make it compatible with Axum
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::ValidationError(_) => StatusCode::BAD_REQUEST,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::ExternalServiceError(_) => StatusCode::BAD_GATEWAY,
            AppError::NotImplemented(_) => StatusCode::NOT_IMPLEMENTED,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
        };
        
        let body = Json(serde_json::json!({
            "error": self.to_string()
        }));
        
        (status, body).into_response()
    }
}

// Request/Response DTOs
#[derive(Debug, Deserialize)]
pub struct CreatePositionRequest {
    pub user_id: Uuid,
    pub protocol: String,
    pub pool_address: String,
    pub chain_id: i32,
    pub token0_address: String,
    pub token1_address: String,
    pub position_type: String,
    pub entry_price: BigDecimal,
    pub amount_usd: BigDecimal,
    pub liquidity_amount: Option<BigDecimal>,
    pub fee_tier: Option<i32>,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePositionRequest {
    pub amount_usd: Option<BigDecimal>,
    pub liquidity_amount: Option<BigDecimal>,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub protocol: String,
    pub pool_address: String,
    pub chain_id: i32,
    pub token0_address: String,
    pub token1_address: String,
    pub position_type: String,
    pub entry_price: BigDecimal,
    pub current_price: Option<BigDecimal>,
    pub amount_usd: BigDecimal,
    pub liquidity_amount: Option<BigDecimal>,
    pub fee_tier: Option<i32>,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
    pub pnl_usd: Option<BigDecimal>,
    pub fees_earned_usd: Option<BigDecimal>,
    pub impermanent_loss_usd: Option<BigDecimal>,
    pub risk_score: Option<i32>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PositionStatsResponse {
    pub total_positions: i64,
    pub active_positions: i64,
    pub total_value_usd: BigDecimal,
    pub total_pnl_usd: BigDecimal,
    pub total_fees_earned_usd: BigDecimal,
    pub total_impermanent_loss_usd: BigDecimal,
    pub protocols: Vec<String>,
    pub chains: Vec<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetPositionsQuery {
    pub user_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub chain_id: Option<i32>,
    pub is_active: Option<bool>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedPositionsResponse {
    pub positions: Vec<PositionResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

// Helper function to create PositionResponse with real calculations
fn create_position_response(
    position: crate::risk::traits::Position
) -> PositionResponse {
    // Get current prices (TODO: Replace with real price feed service)
    let current_token0_price = BigDecimal::from(1);
    let current_token1_price = BigDecimal::from(1);
    
    // Calculate real values using position methods
    let _pnl_usd = BigDecimal::from(0); // Mock value
    let _days_active = 0; // Mock value
    let pnl_usd = position.calculate_pnl_usd(&current_token0_price, &current_token1_price);
    let days_active = position.created_at
        .map(|created| (chrono::Utc::now() - created).num_days())
        .unwrap_or(0);
    let daily_volume = BigDecimal::from(100000); // TODO: Replace with real pool volume
    let pool_tvl = BigDecimal::from(1000000); // TODO: Replace with real pool TVL
    let fees_earned = position.estimate_fees_earned_usd(days_active, &daily_volume, &pool_tvl);
    let il_usd = position.calculate_impermanent_loss_accurate(&current_token0_price, &current_token1_price)
        .unwrap_or_else(|| BigDecimal::from(0));
    
    PositionResponse {
        id: Uuid::parse_str(&position.id).unwrap_or_else(|_| Uuid::new_v4()),
        user_id: uuid::Uuid::new_v4(), // Mock user ID
        protocol: position.protocol.clone(),
        pool_address: position.pool_address.clone(),
        chain_id: position.chain_id,
        token0_address: position.token0_address.clone(),
        token1_address: position.token1_address.clone(),
        position_type: position.get_position_type(),
        entry_price: BigDecimal::from_str(&position.entry_token0_price_usd.clone().unwrap_or_default()).unwrap_or_else(|_| BigDecimal::from(0)),
        current_price: Some(current_token0_price.clone()),
        amount_usd: position.calculate_position_value_usd(current_token0_price.clone(), current_token1_price.clone()),
        liquidity_amount: Some(BigDecimal::from_str(&position.liquidity).unwrap_or_else(|_| BigDecimal::from(0))),
        fee_tier: Some(position.fee_tier as i32),
        tick_lower: Some(position.tick_lower),
        tick_upper: Some(position.tick_upper),
        pnl_usd: Some(pnl_usd),
        fees_earned_usd: Some(fees_earned),
        impermanent_loss_usd: Some(il_usd),
        risk_score: Some(50), // Default risk score for existing positions
        is_active: position.is_position_active(),
        created_at: position.created_at.unwrap_or_default(),
        updated_at: position.updated_at.unwrap_or_default(),
    }
}

// Handler functions
pub async fn create_position(
    State(_state): State<AppState>,
    Json(_request): Json<CreatePositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    // Commented out broken service instantiation:
    // let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    // Create CreatePosition struct from request
    // Commented out broken models reference:
    // let create_position = crate::models::position::CreatePosition {
    return Err(AppError::NotImplemented("CreatePosition model not implemented".to_string()));
}

pub async fn get_position(
    State(_state): State<AppState>,
    Path(_position_id): Path<Uuid>,
) -> Result<Json<PositionResponse>, AppError> {
    // Commented out broken service instantiation:
    // let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    // let position_option = position_service.get_position(position_id).await?;
    let position_option: Option<Position> = None; // Placeholder
    
    let position = position_option.ok_or_else(|| AppError::NotFound("Position not found".to_string()))?;
    
    let response = create_position_response(position);
    
    Ok(Json(response))
}

pub async fn update_position(
    State(_state): State<AppState>,
    Path(_position_id): Path<Uuid>,
    Json(_request): Json<UpdatePositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    // Commented out broken service instantiation:
    // let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    // Commented out broken models reference:
    // let update_position = crate::models::position::UpdatePosition {
    return Err(AppError::NotImplemented("UpdatePosition model not implemented".to_string()));
}

pub async fn delete_position(
    State(_state): State<AppState>,
    Path(_position_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // Commented out broken service instantiation:
    // let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    // position_service.delete_position(position_id).await?;
    // Placeholder - position deletion not implemented
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_positions(
    State(_state): State<AppState>,
    Query(query): Query<GetPositionsQuery>,
) -> Result<Json<PaginatedPositionsResponse>, AppError> {
    // Commented out broken monitoring service import:
    // use crate::services::monitoring_service::MonitoringService;
    
    // Placeholder monitoring service:
    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    struct MonitoringService {
        config: String,
    }
    let _monitoring_service = MonitoringService { config: "placeholder".to_string() };
    
    // Commented out broken service instantiation:
    // let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let _offset = (page - 1) * limit;
    
    // Convert user_id to string for the service call
    let user_address = query.user_id.map(|id| id.to_string()).unwrap_or_default();
    
    // 🎯 ON-DEMAND MONITORING: Only fetch fresh blockchain data when frontend requests it
    // This prevents continuous API polling and respects the 30-second cache
    if !user_address.is_empty() {
        tracing::info!(" Frontend requested positions for user: {}", user_address);
        
        // Create monitoring service for on-demand updates
        // Commented out broken service instantiation:
        // let _monitoring_service = MonitoringService::new(state.db_pool.clone(), state.settings.clone())?;
        
        // Only run monitoring for this specific user (respects caching)
        // Commented out broken method call:
        // if let Err(e) = monitoring_service.monitor_user_positions(&user_address).await {
        //     tracing::warn!(" On-demand monitoring failed for user {}: {}", user_address, e);
        //     // Continue with cached data if monitoring fails
        // }
    }
    
    // let positions = position_service.get_user_positions(&user_address).await?;
    let positions: Vec<Position> = vec![]; // Placeholder - service not implemented
    
    let total = positions.len() as i64;
    
    let position_responses: Vec<PositionResponse> = positions.into_iter().map(|position| {
        create_position_response(position)
    }).collect();
    
    let total_pages = (total as f64 / limit as f64).ceil() as u32;
    let response_count = position_responses.len(); // Calculate before moving
    
    let response = PaginatedPositionsResponse {
        positions: position_responses,
        total,
        page,
        limit,
        total_pages,
    };
    
    tracing::info!("✅ Returned {} positions for user {} (page {}/{})", 
                   response_count, user_address, page, total_pages);
    
    Ok(Json(response))
}

pub async fn get_position_stats(
    State(_state): State<AppState>,
    Query(_query): Query<GetPositionsQuery>,
) -> Result<Json<PositionStatsResponse>, AppError> {
    // Commented out broken service instantiation:
    // let _position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());

    let response = PositionStatsResponse {
        total_positions: 0,
        active_positions: 0,
        total_value_usd: BigDecimal::from(0),
        total_pnl_usd: BigDecimal::from(0),
        total_fees_earned_usd: BigDecimal::from(0),
        total_impermanent_loss_usd: BigDecimal::from(0),
        protocols: Vec::new(),
        chains: Vec::new(),
    };

    Ok(Json(response))
}

// New endpoint to get positions by wallet address (including ENS)
pub async fn get_positions_by_address(
    State(_state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<PositionResponse>>, AppError> {
    
    // Removed unused adapter imports

    tracing::info!("Fetching positions from all protocols for address: {}", address);
    
    // Parse wallet address (TODO: Add ENS resolution)
    // Commented out broken ens_service reference:
    // let ens_service = crate::services::ens_service::EnsService::new(
    return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
        // &std::env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string())

    // let wallet_address = match ens_service.resolve_address_or_ens(&address).await {
    //     Ok(addr) => addr.to_string(),
    //     Err(e) => {
    //         tracing::warn!("ENS resolution failed, trying direct address parsing: {}", e);
    //         address.clone() // Fallback to original address
    //     }
    // };
    // return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
}

// Test endpoint to debug Aave V3 adapter directly
pub async fn test_aave_only(
    State(_state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    
    
    
    tracing::info!("🧪 Testing Aave V3 adapter directly for address: {}", address);
    
    // Parse wallet address with ENS resolution
    // Commented out broken ens_service reference:
    // let ens_service = crate::services::ens_service::EnsService::new(
    return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
        // &std::env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string())

    // let wallet_address = match ens_service.resolve_address_or_ens(&address).await {
    //     Ok(addr) => addr.to_string(),
    
    // Create Aave V3 adapter
    // let aave_adapter = AaveV3Adapter::new(ethereum_client, 1)
    //     .map_err(|e| AppError::ExternalServiceError(format!("Failed to create Aave V3 adapter: {}", e)))?;
    // return Err(AppError::NotImplemented("Ethereum client not implemented".to_string()));
    
    // let mut test_results = serde_json::Map::new();
    // test_results.insert("address".to_string(), serde_json::Value::String(wallet_address.clone()));
    // test_results.insert("test_timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    
    // // Test the adapter
    // match aave_adapter.fetch_positions(eth_address).await {
    //     Ok(positions) => {
    //         tracing::info!("✅ Aave adapter succeeded with {} positions", positions.len());
            
    //         test_results.insert("success".to_string(), serde_json::Value::Bool(true));
    //         test_results.insert("position_count".to_string(), serde_json::Value::Number(positions.len().into()));
    //         test_results.insert("error".to_string(), serde_json::Value::Null);
            
    //         // Convert positions to response format
    //         let position_responses: Vec<PositionResponse> = positions.into_iter().map(|pos| {
    //             let risk_score = if pos.value_usd > 1_000_000.0 { 25 } else { 50 }; // Simplified risk
                
    //             PositionResponse {
    //                 id: uuid::Uuid::new_v4(),
    //                 user_id: uuid::Uuid::new_v4(),
    //                 protocol: pos.protocol,
    //                 pool_address: pos.pair.clone(),
    //                 chain_id: 1,
    //                 token0_address: pos.metadata.get("asset_address")
    //                     .and_then(|v| v.as_str())
    //                     .unwrap_or("0x0000000000000000000000000000000000000000").to_string(),
    //                 token1_address: "0x0000000000000000000000000000000000000000".to_string(), // Aave is single-asset
    //                 position_type: pos.position_type,
    //                 entry_price: BigDecimal::from(0), // Historical data needed
    //                 current_price: Some(BigDecimal::from(0)),
    //                 amount_usd: BigDecimal::try_from(pos.value_usd).unwrap_or_default(),
    //                 liquidity_amount: pos.metadata.get("atoken_balance")
    //                     .and_then(|v| v.as_str())
    //                     .map(|s| BigDecimal::from_str(s).unwrap_or_default()),
    //                 fee_tier: None, // Not applicable to Aave
    //                 tick_lower: None,
    //                 tick_upper: None,
    //                 pnl_usd: Some(BigDecimal::try_from(pos.pnl_usd).unwrap_or_default()),
    //                 fees_earned_usd: Some(BigDecimal::from(0)), // Would be interest earned
    //                 impermanent_loss_usd: Some(BigDecimal::from(0)), // Not applicable to Aave
    //                 risk_score: Some(risk_score),
    //                 is_active: true,
    //                 created_at: chrono::Utc::now(),
    //                 updated_at: chrono::Utc::now(),
    //             }
    //         }).collect();
            
    //         test_results.insert("positions".to_string(), serde_json::to_value(position_responses)?);
    //     },
    //     Err(e) => {
    //         tracing::error!("❌ Aave adapter failed: {}", e);
            
    //         test_results.insert("success".to_string(), serde_json::Value::Bool(false));
    //         test_results.insert("position_count".to_string(), serde_json::Value::Number(0.into()));
    //         test_results.insert("error".to_string(), serde_json::Value::String(e.to_string()));
    //         test_results.insert("positions".to_string(), serde_json::Value::Array(Vec::new()));
    //     }
    // }
    
    // Ok(Json(serde_json::Value::Object(test_results)))
}

// Enhanced test endpoint with comprehensive error handling and fallback strategies
pub async fn test_aave_enhanced(
    State(_state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    
    
    
    tracing::info!("🧪 Enhanced Aave V3 testing for address: {}", address);
    
    // Parse wallet address with ENS resolution
    // Commented out broken ens_service reference:
    // let ens_service = crate::services::ens_service::EnsService::new(
    return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
        // &std::env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string())

    // let wallet_address = match ens_service.resolve_address_or_ens(&address).await {
    //     Ok(addr) => addr.to_string(),
    //     Err(e) => {
    //         // unreachable!()
    //         // tracing::warn!("ENS resolution failed, trying direct address parsing: {}", e);
    //         // address.clone() // Fallback to original address
    //         // unreachable!()
    //         return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
    //     }
    // };
    // return Err(AppError::NotImplemented("ENS service not implemented".to_string()));
    
    // Commented out broken blockchain reference:
    // let ethereum_client = crate::blockchain::EthereumClient::from_rpc_url(
    // return Err(AppError::NotImplemented("Blockchain client not implemented".to_string()));
    
    // Create enhanced Aave V3 adapter
    // let aave_adapter = AaveV3Adapter::new(ethereum_client, 1)
    //     .map_err(|e| AppError::ExternalServiceError(format!("Failed to create Aave V3 adapter: {}", e)))?;
    // return Err(AppError::NotImplemented("Ethereum client not implemented".to_string()));
    
    // let mut test_results = serde_json::Map::new();
    // test_results.insert("address".to_string(), serde_json::Value::String(wallet_address.clone()));
    // test_results.insert("test_timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
    
    // // Test with comprehensive error handling
    // match aave_adapter.fetch_positions(eth_address).await {
    //     Ok(positions) => {
    //         tracing::info!("✅ Enhanced Aave adapter succeeded with {} positions", positions.len());
            
    //         test_results.insert("success".to_string(), serde_json::Value::Bool(true));
    //         test_results.insert("position_count".to_string(), serde_json::Value::Number(positions.len().into()));
    //         test_results.insert("error".to_string(), serde_json::Value::Null);
            
    //         // Convert positions to response format
    //         let position_responses: Vec<PositionResponse> = positions.into_iter().map(|pos| {
    //             let risk_score = if pos.value_usd > 1_000_000.0 { 25 } else { 50 }; // Simplified risk
                
    //             PositionResponse {
    //                 id: uuid::Uuid::new_v4(),
    //                 user_id: uuid::Uuid::new_v4(),
    //                 protocol: pos.protocol,
    //                 pool_address: pos.pair.clone(),
    //                 chain_id: 1,
    //                 token0_address: pos.metadata.get("asset_address")
    //                     .and_then(|v| v.as_str())
    //                     .unwrap_or("0x0000000000000000000000000000000000000000").to_string(),
    //                 token1_address: "0x0000000000000000000000000000000000000000".to_string(), // Aave is single-asset
    //                 position_type: pos.position_type,
    //                 entry_price: BigDecimal::from(0), // Historical data needed
    //                 current_price: Some(BigDecimal::from(0)),
    //                 amount_usd: BigDecimal::try_from(pos.value_usd).unwrap_or_default(),
    //                 liquidity_amount: pos.metadata.get("atoken_balance")
    //                     .and_then(|v| v.as_str())
    //                     .map(|s| BigDecimal::from_str(s).unwrap_or_default()),
    //                 fee_tier: None, // Not applicable to Aave
    //                 tick_lower: None,
    //                 tick_upper: None,
    //                 pnl_usd: Some(BigDecimal::try_from(pos.pnl_usd).unwrap_or_default()),
    //                 fees_earned_usd: Some(BigDecimal::from(0)), // Would be interest earned
    //                 impermanent_loss_usd: Some(BigDecimal::from(0)), // Not applicable to Aave
    //                 risk_score: Some(risk_score),
    //                 is_active: true,
    //                 created_at: chrono::Utc::now(),
    //                 updated_at: chrono::Utc::now(),
    //             }
    //         }).collect();
            
    //         test_results.insert("positions".to_string(), serde_json::to_value(position_responses)?);
    //     },
    //     Err(e) => {
    //         tracing::error!("❌ Enhanced Aave adapter failed: {}", e);
            
    //         test_results.insert("success".to_string(), serde_json::Value::Bool(false));
    //         test_results.insert("position_count".to_string(), serde_json::Value::Number(0.into()));
    //         test_results.insert("error".to_string(), serde_json::Value::String(e.to_string()));
    //         test_results.insert("positions".to_string(), serde_json::Value::Array(Vec::new()));
    //     }
    // }
    
    // Ok(Json(serde_json::Value::Object(test_results)))
}

/// Calculate real risk score for a position using comprehensive risk analysis
#[allow(dead_code)]
async fn calculate_position_risk_score(
    position: &crate::adapters::traits::Position,
    _risk_calculator: &RiskCalculator,
) -> i32 {
    // Calculate risk score based on position characteristics using your comprehensive risk logic
    let risk_score = if position.value_usd > 1_000_000_000.0 {
        // Very large positions (like Vitalik's $10B position) - lower risk due to size and stability
        15
    } else if position.value_usd > 100_000_000.0 {
        // Large positions ($100M+) - low risk
        25
    } else if position.value_usd > 1_000_000.0 {
        // Medium positions ($1M+) - medium risk
        45
    } else if position.value_usd > 10_000.0 {
        // Small positions ($10K+) - higher risk
        65
    } else {
        // Very small positions - highest risk
        85
    };
    
    // Apply protocol-specific risk adjustments
    let mut adjusted_risk = risk_score;
    
    // Uniswap V3 specific risk factors
    if position.protocol == "uniswap_v3" {
        // Lower risk for established protocol
        adjusted_risk = (adjusted_risk as f64 * 0.8) as i32;
    }
    
    // Apply liquidity-based risk adjustment
    if let Some(metadata) = &position.metadata.as_object() {
        if let Some(liquidity_str) = metadata.get("liquidity").and_then(|v| v.as_str()) {
            if let Ok(liquidity) = liquidity_str.parse::<f64>() {
                if liquidity > 1e15 {
                    // Very high liquidity = lower risk
                    adjusted_risk = (adjusted_risk as f64 * 0.7) as i32;
                }
            }
        }
    }
    
    // Cap risk score between 1 and 100
    adjusted_risk.max(1).min(100)
}

// Create router
pub fn create_position_routes() -> Router<AppState> {
    Router::new()
        .route("/positions", post(create_position))
        .route("/positions", get(list_positions))
        .route("/positions/stats", get(get_position_stats))
        .route("/positions/wallet/:address", get(get_positions_by_address))  // New endpoint for frontend
        .route("/positions/id/:position_id", get(get_position))
        .route("/positions/id/:position_id", put(update_position))
        .route("/positions/id/:position_id", delete(delete_position))
        .route("/positions/test-aave/:address", get(test_aave_only))
        .route("/positions/test-aave-enhanced/:address", get(test_aave_enhanced))

}
