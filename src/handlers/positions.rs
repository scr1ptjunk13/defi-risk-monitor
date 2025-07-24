use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::Position;
use crate::error::AppError;

#[derive(Serialize, Deserialize)]
pub struct PositionResponse {
    pub positions: Vec<Position>,
    pub total: usize,
}

#[derive(Serialize, Deserialize)]
pub struct SinglePositionResponse {
    pub position: Position,
}

pub async fn list_positions(
    State(pool): State<PgPool>,
) -> Result<Json<PositionResponse>, StatusCode> {
    let positions = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = positions.len();

    Ok(Json(PositionResponse { positions, total }))
}

pub async fn get_position(
    Path(id): Path<Uuid>,
    State(pool): State<PgPool>,
) -> Result<Json<SinglePositionResponse>, StatusCode> {
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match position {
        Some(position) => Ok(Json(SinglePositionResponse { position })),
        None => Err(StatusCode::NOT_FOUND),
    }
}
