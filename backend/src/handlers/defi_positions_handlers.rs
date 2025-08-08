use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct FetchPositionsQuery {
    pub address: Option<String>,
    pub protocols: Option<String>, // comma-separated
    // Ethereum only for now
    pub include_metrics: Option<bool>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct LivePositionSummaryItem {
    pub protocol: String,
    pub chain_id: i32,
    pub positions: u32,
    pub total_value_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct LivePositionsResponse<T> {
    pub success: bool,
    pub data: T,
    pub warnings: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct LivePositionsPage<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
}

#[derive(Debug, Serialize)]
pub struct LivePosition {
    pub protocol: String,
    pub chain_id: i32,
    pub pool_address: String,
    pub position_id: Option<String>,
    pub token0: String,
    pub token1: String,
    pub amount_token0: String,
    pub amount_token1: String,
    pub value_usd: Option<f64>,
}

pub async fn fetch_positions_handler(
    State(_state): State<AppState>,
    Query(query): Query<FetchPositionsQuery>,
) -> Result<Json<LivePositionsResponse<LivePositionsPage<LivePosition>>>, (StatusCode, Json<LivePositionsResponse<HashMap<&'static str, &'static str>>>)> {
    // Basic validation and defaults
    let address = query.address.unwrap_or_else(|| "vitalik.eth".to_string());
    let _protocols: Vec<String> = query
        .protocols
        .unwrap_or_else(|| "uniswap_v3,aave_v3,compound_v3,curve,lido".to_string())
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    // Focus on Ethereum only (chain_id = 1)
    let chain_id = 1; // Ethereum mainnet
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let _include_metrics = query.include_metrics.unwrap_or(false);

    // TODO: Wire up Ethereum DeFi adapters (Uniswap V3, Aave V3, Compound V3, Curve, Lido)
    // For now, return an empty but well-structured payload to scaffold the API
    let warnings = vec![format!(
        "Ethereum DeFi adapters not yet implemented. Returning empty result for address {} on Ethereum",
        address
    )];

    let page_payload = LivePositionsPage {
        items: Vec::new(),
        page,
        per_page,
        total: 0,
    };

    Ok(Json(LivePositionsResponse {
        success: true,
        data: page_payload,
        warnings: Some(warnings),
    }))
}

pub async fn positions_summary_handler(
    State(_state): State<AppState>,
    Query(query): Query<FetchPositionsQuery>,
) -> Result<Json<LivePositionsResponse<Vec<LivePositionSummaryItem>>>, (StatusCode, Json<LivePositionsResponse<HashMap<&'static str, &'static str>>>)> {
    let _address = query.address.unwrap_or_else(|| "vitalik.eth".to_string());

    // TODO: Compute real summary after integrating adapters and price feed
    Ok(Json(LivePositionsResponse {
        success: true,
        data: Vec::new(),
        warnings: Some(vec!["Summary not yet implemented".to_string()]),
    }))
}

pub fn create_defi_positions_routes() -> Router<AppState> {
    Router::new()
        // Intentionally use distinct paths to avoid clashing with existing CRUD routes
        .route("/positions/fetch", get(fetch_positions_handler))
        .route("/positions/summary", get(positions_summary_handler))
}
