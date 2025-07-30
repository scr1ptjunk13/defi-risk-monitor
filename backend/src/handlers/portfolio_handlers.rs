use axum::{extract::{State, Query}, Json, http::StatusCode};
use serde::Deserialize;
use crate::services::portfolio_service::{PortfolioService, PortfolioSummary};
use crate::AppState;
use crate::error::types::AppError;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct GetPortfolioQuery {
    pub user_address: String,
}

/// GET /api/v1/portfolio?user_address=0x...
pub async fn get_portfolio(
    State(state): State<AppState>,
    Query(query): Query<GetPortfolioQuery>,
) -> Result<Json<PortfolioSummary>, (StatusCode, String)> {
    let service = PortfolioService::new(state.db_pool.clone());
    match service.get_portfolio_summary(&query.user_address).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("Portfolio aggregation error: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Portfolio aggregation failed: {}", e)))
        }
    }
}
