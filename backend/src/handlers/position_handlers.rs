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
use chrono::{DateTime, Utc};
use crate::{
    services::position_service::PositionService,
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
    let position_service = PositionService::new(state.db_pool.clone(), (*state.blockchain_service).clone());
    
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let _offset = (page - 1) * limit;
    
    // Convert user_id to string for the service call
    let user_address = query.user_id.map(|id| id.to_string()).unwrap_or_default();
    let positions = position_service.get_user_positions(&user_address).await?;
    
    let total = positions.len() as i64;
    
    let position_responses: Vec<PositionResponse> = positions.into_iter().map(|position| {
        create_position_response(position)
    }).collect();
    
    let total_pages = (total as f64 / limit as f64).ceil() as u32;
    
    let response = PaginatedPositionsResponse {
        positions: position_responses,
        total,
        page,
        limit,
        total_pages,
    };
    
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

// Create router
pub fn create_position_routes() -> Router<AppState> {
    Router::new()
        .route("/positions", post(create_position))
        .route("/positions", get(list_positions))
        .route("/positions/stats", get(get_position_stats))
        .route("/positions/:position_id", get(get_position))
        .route("/positions/:position_id", put(update_position))
        .route("/positions/:position_id", delete(delete_position))
}
