use axum::{extract::{State, Query}, Json, http::StatusCode};
use serde::Deserialize;
use crate::services::portfolio_service::{PortfolioService, PortfolioSummary};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct GetPortfolioQuery {
    pub user_address: String,
}

/// GET /api/v1/portfolio?user_address=0x...
pub async fn get_portfolio(
    State(state): State<AppState>,
    Query(query): Query<GetPortfolioQuery>,
) -> Result<Json<PortfolioSummary>, (StatusCode, String)> {
    // Create a dummy price validation service for now - this should be properly injected
    let price_validation_service = match crate::services::price_validation::PriceValidationService::new(state.db_pool.clone()).await {
        Ok(service) => service,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create price validation service: {}", e)))
    };
    
    let mut service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    match service.get_portfolio_summary(&query.user_address).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("Portfolio aggregation error: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Portfolio aggregation failed: {}", e)))
        }
    }
}
