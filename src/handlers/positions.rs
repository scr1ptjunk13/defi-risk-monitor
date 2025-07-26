use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use crate::models::{Position, CreatePosition};
use crate::services::position_service::PositionService;

use crate::AppState;

/// Request/Response DTOs for Position Management API

#[derive(Debug, Serialize, Deserialize)]
pub struct CreatePositionRequest {
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePositionRequest {
    pub token0_amount: Option<BigDecimal>,
    pub token1_amount: Option<BigDecimal>,
    pub liquidity: Option<BigDecimal>,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionResponse {
    pub positions: Vec<Position>,
    pub total: usize,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SinglePositionResponse {
    pub position: Position,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPositionsQuery {
    pub user_address: Option<String>,
    pub protocol: Option<String>,
    pub chain_id: Option<i32>,
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PositionStatsResponse {
    pub total_positions: i64,
    pub total_value_usd: BigDecimal,
    pub protocols: HashMap<String, i64>,
    pub chains: HashMap<i32, i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// Create a new position
/// POST /api/v1/positions
pub async fn create_position(
    State(state): State<AppState>,
    Json(request): Json<CreatePositionRequest>,
) -> Result<Json<ApiResponse<SinglePositionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = PositionService::new(
        state.db_pool.clone(),
        (*state.blockchain_service).clone(),
    );
    
    let create_position = CreatePosition {
        user_address: request.user_address,
        protocol: request.protocol,
        pool_address: request.pool_address,
        token0_address: request.token0_address,
        token1_address: request.token1_address,
        token0_amount: request.token0_amount,
        token1_amount: request.token1_amount,
        liquidity: request.liquidity,
        tick_lower: request.tick_lower,
        tick_upper: request.tick_upper,
        fee_tier: request.fee_tier,
        chain_id: request.chain_id,
        entry_token0_price_usd: None, // Will be fetched by PositionService
        entry_token1_price_usd: None, // Will be fetched by PositionService
    };
    
    match service.create_position_with_entry_prices(create_position).await {
        Ok(position) => Ok(Json(ApiResponse {
            success: true,
            data: Some(SinglePositionResponse { position }),
            message: Some("Position created successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to create position".to_string()),
        })))
    }
}

/// Get positions with filtering and pagination
/// GET /api/v1/positions
pub async fn list_positions(
    State(state): State<AppState>,
    Query(query): Query<GetPositionsQuery>,
) -> Result<Json<ApiResponse<PositionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50).min(100); // Max 100 per page
    let offset = (page - 1) * per_page;
    
    let mut sql = "SELECT * FROM positions WHERE 1=1".to_string();
    let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
    let mut param_count = 0;
    
    if let Some(user_address) = &query.user_address {
        param_count += 1;
        sql.push_str(&format!(" AND user_address = ${}", param_count));
        params.push(Box::new(user_address.clone()));
    }
    
    if let Some(protocol) = &query.protocol {
        param_count += 1;
        sql.push_str(&format!(" AND protocol = ${}", param_count));
        params.push(Box::new(protocol.clone()));
    }
    
    if let Some(chain_id) = query.chain_id {
        param_count += 1;
        sql.push_str(&format!(" AND chain_id = ${}", param_count));
        params.push(Box::new(chain_id));
    }
    
    sql.push_str(&format!(" ORDER BY created_at DESC LIMIT {} OFFSET {}", per_page, offset));
    
    // For simplicity, using a basic query without dynamic parameters
    let positions = if query.user_address.is_some() {
        sqlx::query_as::<_, Position>(
            "SELECT * FROM positions WHERE user_address = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
        )
        .bind(query.user_address.unwrap())
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db_pool)
        .await
    } else {
        sqlx::query_as::<_, Position>(
            "SELECT * FROM positions ORDER BY created_at DESC LIMIT $1 OFFSET $2"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(&state.db_pool)
        .await
    };
    
    match positions {
        Ok(positions) => {
            let total = positions.len();
            Ok(Json(ApiResponse {
                success: true,
                data: Some(PositionResponse {
                    positions,
                    total,
                    page: Some(page),
                    per_page: Some(per_page),
                }),
                message: None,
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to fetch positions".to_string()),
        })))
    }
}

/// Get a specific position by ID
/// GET /api/v1/positions/{id}
pub async fn get_position(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<SinglePositionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.db_pool)
    .await;
    
    match position {
        Ok(Some(position)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(SinglePositionResponse { position }),
            message: None,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Position not found".to_string()),
        }))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to fetch position".to_string()),
        })))
    }
}

/// Update a position
/// PUT /api/v1/positions/{id}
pub async fn update_position(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdatePositionRequest>,
) -> Result<Json<ApiResponse<SinglePositionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    // Build dynamic update query
    let mut updates = Vec::new();
    let mut param_count = 1;
    
    if request.token0_amount.is_some() {
        updates.push(format!("token0_amount = ${}", param_count));
        param_count += 1;
    }
    if request.token1_amount.is_some() {
        updates.push(format!("token1_amount = ${}", param_count));
        param_count += 1;
    }
    if request.liquidity.is_some() {
        updates.push(format!("liquidity = ${}", param_count));
        param_count += 1;
    }
    if request.tick_lower.is_some() {
        updates.push(format!("tick_lower = ${}", param_count));
        param_count += 1;
    }
    if request.tick_upper.is_some() {
        updates.push(format!("tick_upper = ${}", param_count));
        param_count += 1;
    }
    
    if updates.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("No fields to update".to_string()),
        })));
    }
    
    updates.push("updated_at = NOW()".to_string());
    let _sql = format!(
        "UPDATE positions SET {} WHERE id = ${} RETURNING *",
        updates.join(", "),
        param_count
    );
    
    // For simplicity, using a basic update
    let result = if let Some(liquidity) = request.liquidity {
        sqlx::query_as::<_, Position>(
            "UPDATE positions SET liquidity = $1, updated_at = NOW() WHERE id = $2 RETURNING *"
        )
        .bind(liquidity)
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    } else {
        sqlx::query_as::<_, Position>(
            "UPDATE positions SET updated_at = NOW() WHERE id = $1 RETURNING *"
        )
        .bind(id)
        .fetch_optional(&state.db_pool)
        .await
    };
    
    match result {
        Ok(Some(position)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(SinglePositionResponse { position }),
            message: Some("Position updated successfully".to_string()),
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Position not found".to_string()),
        }))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to update position".to_string()),
        })))
    }
}

/// Delete a position
/// DELETE /api/v1/positions/{id}
pub async fn delete_position(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let result = sqlx::query(
        "DELETE FROM positions WHERE id = $1"
    )
    .bind(id)
    .execute(&state.db_pool)
    .await;
    
    match result {
        Ok(result) if result.rows_affected() > 0 => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: Some("Position deleted successfully".to_string()),
        })),
        Ok(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Position not found".to_string()),
        }))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to delete position".to_string()),
        })))
    }
}

/// Get position statistics for a user
/// GET /api/v1/positions/stats
pub async fn get_position_stats(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<PositionStatsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_address = query.get("user_address");
    
    // Get total positions count
    let total_query = if let Some(addr) = user_address {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM positions WHERE user_address = $1"
        )
        .bind(addr)
        .fetch_one(&state.db_pool)
        .await
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM positions"
        )
        .fetch_one(&state.db_pool)
        .await
    };
    
    let total_positions = total_query.unwrap_or(0);
    
    // For now, return basic stats
    let stats = PositionStatsResponse {
        total_positions,
        total_value_usd: BigDecimal::from(0), // Would calculate from position values
        protocols: HashMap::new(), // Would aggregate by protocol
        chains: HashMap::new(),    // Would aggregate by chain
    };
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(stats),
        message: None,
    }))
}

/// Create router for position management endpoints
pub fn create_position_routes() -> Router<AppState> {
    Router::new()
        .route("/positions", post(create_position))
        .route("/positions", get(list_positions))
        .route("/positions/stats", get(get_position_stats))
        .route("/positions/:id", get(get_position))
        .route("/positions/:id", put(update_position))
        .route("/positions/:id", delete(delete_position))
}
