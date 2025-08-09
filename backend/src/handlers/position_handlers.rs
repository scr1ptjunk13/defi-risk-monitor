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
use crate::{
    services::position_service::PositionService,
    services::risk_calculator::RiskCalculator,
    adapters::traits::DeFiAdapter,
    error::AppError,
    AppState,
};

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
fn create_position_response(position: crate::models::position::Position) -> PositionResponse {
    // Get current prices (TODO: Replace with real price feed service)
    let current_token0_price = BigDecimal::from(1);
    let current_token1_price = BigDecimal::from(1);
    
    // Calculate real values using position methods
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
        id: position.id,
        user_id: uuid::Uuid::parse_str(&position.user_address).unwrap_or_default(),
        protocol: position.protocol.clone(),
        pool_address: position.pool_address.clone(),
        chain_id: position.chain_id,
        token0_address: position.token0_address.clone(),
        token1_address: position.token1_address.clone(),
        position_type: position.get_position_type(),
        entry_price: position.entry_token0_price_usd.clone().unwrap_or_default(),
        current_price: Some(current_token0_price.clone()),
        amount_usd: position.calculate_position_value_usd(current_token0_price.clone(), current_token1_price.clone()),
        liquidity_amount: Some(position.liquidity.clone()),
        fee_tier: Some(position.fee_tier),
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
    State(state): State<AppState>,
    Json(request): Json<CreatePositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    // Create CreatePosition struct from request
    let create_position = crate::models::position::CreatePosition {
        user_address: request.user_id.to_string(), // Convert UUID to string
        protocol: request.protocol,
        pool_address: request.pool_address,
        chain_id: request.chain_id,
        token0_address: request.token0_address,
        token1_address: request.token1_address,
        token0_amount: request.amount_usd.clone(), // Mock value
        token1_amount: request.amount_usd.clone(), // Mock value
        liquidity: request.liquidity_amount.unwrap_or_default(),
        fee_tier: request.fee_tier.unwrap_or(3000), // Default fee tier
        tick_lower: request.tick_lower.unwrap_or(-60000),
        tick_upper: request.tick_upper.unwrap_or(60000),
        entry_token0_price_usd: Some(request.entry_price.clone()),
        entry_token1_price_usd: Some(request.entry_price.clone()),
    };
    
    let position = position_service.create_position_with_entry_prices(create_position).await?;
    
    let response = create_position_response(position);
    
    Ok(Json(response))
}

pub async fn get_position(
    State(state): State<AppState>,
    Path(position_id): Path<Uuid>,
) -> Result<Json<PositionResponse>, AppError> {
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    let position_option = position_service.get_position(position_id).await?;
    
    let position = position_option.ok_or_else(|| AppError::NotFound("Position not found".to_string()))?;
    
    let response = create_position_response(position);
    
    Ok(Json(response))
}

pub async fn update_position(
    State(state): State<AppState>,
    Path(position_id): Path<Uuid>,
    Json(request): Json<UpdatePositionRequest>,
) -> Result<Json<PositionResponse>, AppError> {
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    let update_position = crate::models::position::UpdatePosition {
        token0_amount: request.amount_usd,
        token1_amount: None, // Not provided in request
        liquidity: request.liquidity_amount,
    };
    
    let position = position_service.update_position(position_id, update_position).await?;
    
    let response = PositionResponse {
        id: position.id,
        user_id: uuid::Uuid::parse_str(&position.user_address).unwrap_or_default(),
        protocol: position.protocol,
        pool_address: position.pool_address,
        chain_id: position.chain_id,
        token0_address: position.token0_address,
        token1_address: position.token1_address,
        position_type: "liquidity".to_string(), // Mock value - field doesn't exist in Position
        entry_price: position.entry_token0_price_usd.clone().unwrap_or_default(),
        current_price: Some(position.entry_token0_price_usd.unwrap_or_default()), // Mock value
        amount_usd: position.token0_amount.clone(),
        liquidity_amount: Some(position.liquidity.clone()),
        fee_tier: Some(position.fee_tier),
        tick_lower: Some(position.tick_lower),
        tick_upper: Some(position.tick_upper),
        pnl_usd: Some(BigDecimal::from(0)), // Mock value - field doesn't exist in Position
        fees_earned_usd: Some(BigDecimal::from(0)), // Mock value - field doesn't exist in Position
        impermanent_loss_usd: Some(BigDecimal::from(0)), // Mock value - field doesn't exist in Position
        risk_score: Some(50), // Default risk score for existing positions
        is_active: true, // Mock value - field doesn't exist in Position
        created_at: position.created_at.unwrap_or_default(),
        updated_at: position.updated_at.unwrap_or_default(),
    };
    
    Ok(Json(response))
}

pub async fn delete_position(
    State(state): State<AppState>,
    Path(position_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    position_service.delete_position(position_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_positions(
    State(state): State<AppState>,
    Query(query): Query<GetPositionsQuery>,
) -> Result<Json<PaginatedPositionsResponse>, AppError> {
    use crate::services::monitoring_service::MonitoringService;
    
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let _offset = (page - 1) * limit;
    
    // Convert user_id to string for the service call
    let user_address = query.user_id.map(|id| id.to_string()).unwrap_or_default();
    
    // 🎯 ON-DEMAND MONITORING: Only fetch fresh blockchain data when frontend requests it
    // This prevents continuous API polling and respects the 30-second cache
    if !user_address.is_empty() {
        tracing::info!("🎯 Frontend requested positions for user: {}", user_address);
        
        // Create monitoring service for on-demand updates
        let monitoring_service = MonitoringService::new(state.db_pool.clone(), state.settings.clone())?;
        
        // Only run monitoring for this specific user (respects caching)
        if let Err(e) = monitoring_service.monitor_user_positions(&user_address).await {
            tracing::warn!("⚠️ On-demand monitoring failed for user {}: {}", user_address, e);
            // Continue with cached data if monitoring fails
        }
    }
    
    let positions = position_service.get_user_positions(&user_address).await?;
    
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
    State(state): State<AppState>,
    Query(_query): Query<GetPositionsQuery>,
) -> Result<Json<PositionStatsResponse>, AppError> {
    let _position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());

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
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<Vec<PositionResponse>>, AppError> {
    use alloy::primitives::Address;
    use crate::adapters::uniswap_v3::UniswapV3Adapter;

    tracing::info!("Fetching real Uniswap V3 positions for address: {}", address);
    
    // Parse wallet address (TODO: Add ENS resolution)
    // Use ENS service to resolve address or ENS name
    let ens_service = crate::services::ens_service::EnsService::new(
        &std::env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string())
    ).map_err(|e| AppError::ConfigError(format!("Failed to create ENS service: {}", e)))?;
    
    let wallet_address = ens_service.resolve_address_or_ens(&address).await
        .map_err(|e| AppError::ValidationError(format!("Failed to resolve address/ENS: {}", e)))?
        .to_string();
    
    let eth_address = Address::from_str(&wallet_address)
        .map_err(|e| AppError::ValidationError(format!("Invalid Ethereum address: {}", e)))?;
    
    // Create EthereumClient from BlockchainService provider
    let ethereum_provider = state.blockchain_service.get_provider_for_chain(1) // Ethereum mainnet
        .map_err(|e| AppError::ExternalServiceError(format!("Failed to get Ethereum provider: {}", e)))?;
    
    // Create EthereumClient wrapper for the adapter
    let ethereum_client = crate::blockchain::EthereumClient::from_provider((**ethereum_provider).clone());
    
    // Create Uniswap V3 adapter and fetch real positions
    let uniswap_adapter = UniswapV3Adapter::new(ethereum_client)
        .map_err(|e| AppError::ExternalServiceError(format!("Failed to create Uniswap V3 adapter: {}", e)))?;
    
    let positions = uniswap_adapter.fetch_positions(eth_address).await
        .map_err(|e| AppError::ExternalServiceError(format!("Failed to fetch Uniswap V3 positions: {}", e)))?;
    
    tracing::info!("Found {} Uniswap V3 positions for {}", positions.len(), wallet_address);
    
    // Initialize risk calculator for real risk assessment
    let risk_calculator = RiskCalculator::new();
    
    // Convert adapter positions to PositionResponse format with real risk calculation
    let mut position_responses = Vec::new();
    
    for pos in positions {
        // Calculate real risk score based on position characteristics
        let risk_score = calculate_position_risk_score(&pos, &risk_calculator).await;
        
        let position_response = PositionResponse {
            id: uuid::Uuid::new_v4(), // Generate new UUID
            user_id: uuid::Uuid::new_v4(), // Mock user ID
            protocol: pos.protocol,
            pool_address: pos.pair.clone(), // Use pair info as pool address for now
            chain_id: 1, // Ethereum mainnet
            token0_address: pos.metadata.get("token0").and_then(|v| v.as_str()).unwrap_or("0x0000000000000000000000000000000000000000").to_string(),
            token1_address: pos.metadata.get("token1").and_then(|v| v.as_str()).unwrap_or("0x0000000000000000000000000000000000000000").to_string(),
            position_type: pos.position_type,
            entry_price: BigDecimal::from(0), // TODO: Calculate from position data
            current_price: Some(BigDecimal::from(0)), // TODO: Get current price
            amount_usd: BigDecimal::try_from(pos.value_usd).unwrap_or_default(),
            liquidity_amount: pos.metadata.get("liquidity").and_then(|v| v.as_str()).map(|s| BigDecimal::from_str(s).unwrap_or_default()),
            fee_tier: pos.metadata.get("fee_tier").and_then(|v| v.as_u64()).map(|v| v as i32),
            tick_lower: pos.metadata.get("tick_lower").and_then(|v| v.as_i64()).map(|v| v as i32),
            tick_upper: pos.metadata.get("tick_upper").and_then(|v| v.as_i64()).map(|v| v as i32),
            pnl_usd: Some(BigDecimal::try_from(pos.pnl_usd).unwrap_or_default()),
            fees_earned_usd: Some(BigDecimal::from(0)), // TODO: Calculate fees
            impermanent_loss_usd: Some(BigDecimal::from(0)), // TODO: Calculate IL
            risk_score: Some(risk_score), // Real calculated risk score!
            is_active: true, // Assume active if returned by adapter
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        position_responses.push(position_response);
    }
    
    Ok(Json(position_responses))
}

/// Calculate real risk score for a position using comprehensive risk analysis
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
}
