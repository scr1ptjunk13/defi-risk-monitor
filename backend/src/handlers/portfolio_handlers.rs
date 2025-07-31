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
    // Create a dummy price validation service for now - this should be properly injected
    let cache_manager = crate::utils::caching::CacheManager::new(None).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Cache init failed: {}", e)))?;
    let price_sources = crate::services::price_validation::create_default_price_sources();
    let config = crate::services::price_validation::PriceValidationConfig::default();
    let price_validation_service = crate::services::price_validation::PriceValidationService::new(price_sources, config, cache_manager).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Price service init failed: {}", e)))?;
    
    let mut service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    match service.get_portfolio_summary(&query.user_address).await {
        Ok(summary) => Ok(Json(summary)),
        Err(e) => {
            tracing::error!("Portfolio aggregation error: {:?}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Portfolio aggregation failed: {}", e)))
        }
    }
}
