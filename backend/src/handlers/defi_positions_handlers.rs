use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use alloy::primitives::Address;

use crate::AppState;
use crate::blockchain::EthereumClient;
use crate::services::position_aggregator::{PositionAggregator, AggregatorError};
use crate::adapters::{Position, PortfolioSummary};

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
    State(state): State<AppState>,
    Query(query): Query<FetchPositionsQuery>,
) -> Result<Json<LivePositionsResponse<LivePositionsPage<LivePosition>>>, (StatusCode, Json<LivePositionsResponse<HashMap<&'static str, &'static str>>>)> {
    let address_str = query.address.ok_or_else(|| {
        AppError::ValidationError("Address or ENS name is required. Please provide either an Ethereum address (0x...) or ENS name (name.eth)".to_string())
    })?;
    
    // Validate and parse address
    let address = match EthereumClient::validate_address(&address_str) {
        Ok(addr) => addr,
        Err(_) => {
            let error_response = LivePositionsResponse {
                success: false,
                data: HashMap::from([("error", "Invalid Ethereum address format")]),
                warnings: None,
            };
            return Err((StatusCode::BAD_REQUEST, Json(error_response)));
        }
    };
    
    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    
    // TODO: Get RPC URL from state/config
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://eth-mainnet.alchemyapi.io/v2/demo".to_string());
    
    // Create Ethereum client and position aggregator
    let client = match EthereumClient::new(&rpc_url).await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create Ethereum client");
            let error_response = LivePositionsResponse {
                success: false,
                data: HashMap::from([("error", "Failed to connect to Ethereum network")]),
                warnings: None,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    };
    
    let aggregator = match PositionAggregator::new(client, None).await {
        Ok(agg) => agg,
        Err(e) => {
            tracing::error!(error = %e, "Failed to create position aggregator");
            let error_response = LivePositionsResponse {
                success: false,
                data: HashMap::from([("error", "Failed to initialize DeFi adapters")]),
                warnings: None,
            };
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)));
        }
    };
    
    // Fetch user portfolio
    match aggregator.fetch_user_portfolio(address).await {
        Ok(portfolio) => {
            let live_positions: Vec<LivePosition> = portfolio.positions.into_iter()
                .map(|pos| LivePosition {
                    protocol: pos.protocol,
                    chain_id: 1, // Ethereum
                    pool_address: pos.id.clone(),
                    position_id: Some(pos.id),
                    token0: "TODO".to_string(), // TODO: Extract from metadata
                    token1: "TODO".to_string(), // TODO: Extract from metadata
                    amount_token0: "0".to_string(), // TODO: Calculate from position
                    amount_token1: "0".to_string(), // TODO: Calculate from position
                    value_usd: Some(pos.value_usd),
                })
                .collect();
            
            let total = live_positions.len() as u64;
            let start_idx = ((page - 1) * per_page) as usize;
            let end_idx = (start_idx + per_page as usize).min(live_positions.len());
            let page_items = live_positions[start_idx..end_idx].to_vec();
            
            let page_payload = LivePositionsPage {
                items: page_items,
                page,
                per_page,
                total,
            };
            
            Ok(Json(LivePositionsResponse {
                success: true,
                data: page_payload,
                warnings: None,
            }))
        }
        Err(AggregatorError::NoPositionsFound(_)) => {
            // Return empty result for users with no positions
            let page_payload = LivePositionsPage {
                items: Vec::new(),
                page,
                per_page,
                total: 0,
            };
            
            Ok(Json(LivePositionsResponse {
                success: true,
                data: page_payload,
                warnings: Some(vec![format!("No DeFi positions found for address {}", address_str)]),
            }))
        }
        Err(e) => {
            tracing::error!(
                user_address = %address,
                error = %e,
                "Failed to fetch user portfolio"
            );
            
            let error_response = LivePositionsResponse {
                success: false,
                data: HashMap::from([("error", "Failed to fetch DeFi positions")]),
                warnings: None,
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

pub async fn positions_summary_handler(
    State(_state): State<AppState>,
    Query(query): Query<FetchPositionsQuery>,
) -> Result<Json<LivePositionsResponse<Vec<LivePositionSummaryItem>>>, (StatusCode, Json<LivePositionsResponse<HashMap<&'static str, &'static str>>>)> {
    let _address = query.address.ok_or_else(|| {
        AppError::ValidationError("Address or ENS name is required. Please provide either an Ethereum address (0x...) or ENS name (name.eth)".to_string())
    })?;

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
