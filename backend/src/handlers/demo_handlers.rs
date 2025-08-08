use crate::error::AppError;
use crate::services::demo_data_service::DemoDataService;
use crate::AppState;
use axum::{
    extract::{State, Path, Query},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct DemoPositionsResponse {
    pub positions: Vec<crate::models::Position>,
    pub total_count: usize,
    pub data_source: String,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct DemoStatsResponse {
    pub total_positions: usize,
    pub total_value_usd: f64,
    pub active_protocols: Vec<String>,
    pub risk_summary: DemoRiskSummary,
}

#[derive(Debug, Serialize)]
pub struct DemoRiskSummary {
    pub low_risk_positions: usize,
    pub medium_risk_positions: usize,
    pub high_risk_positions: usize,
    pub overall_risk_score: f64,
}

/// Get demo positions with real market data
pub async fn get_demo_positions(
    State(state): State<AppState>,
) -> Result<Json<DemoPositionsResponse>, AppError> {
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    
    // Get real positions from whale addresses or create realistic demo data
    let mut positions = demo_service.get_demo_positions().await?;
    
    // Get real-time prices for positions
    let _prices = demo_service.get_position_prices(&positions).await?;
    
    let response = DemoPositionsResponse {
        total_count: positions.len(),
        data_source: if positions.is_empty() { 
            "Generated Demo Data".to_string() 
        } else { 
            "Real Uniswap V3 Positions".to_string() 
        },
        last_updated: chrono::Utc::now(),
        positions,
    };
    
    Ok(Json(response))
}

/// Get demo portfolio statistics
pub async fn get_demo_stats(
    State(state): State<AppState>,
) -> Result<Json<DemoStatsResponse>, AppError> {
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    let positions = demo_service.get_demo_positions().await?;
    
    let total_value_usd: f64 = positions.iter()
        .map(|p| {
            let token0_value = p.token0_amount.to_string().parse::<f64>().unwrap_or(0.0);
            let token1_value = p.token1_amount.to_string().parse::<f64>().unwrap_or(0.0);
            token0_value + token1_value
        })
        .sum();
    
    let active_protocols: Vec<String> = positions.iter()
        .map(|p| p.protocol.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    // Simple risk categorization based on position size
    let mut low_risk = 0;
    let mut medium_risk = 0;
    let mut high_risk = 0;
    
    for position in &positions {
        let position_value = position.token0_amount.to_string().parse::<f64>().unwrap_or(0.0) + 
                           position.token1_amount.to_string().parse::<f64>().unwrap_or(0.0);
        
        if position_value < 10000.0 {
            low_risk += 1;
        } else if position_value < 50000.0 {
            medium_risk += 1;
        } else {
            high_risk += 1;
        }
    }
    
    let overall_risk_score = if positions.is_empty() {
        0.0
    } else {
        (low_risk as f64 * 0.2 + medium_risk as f64 * 0.5 + high_risk as f64 * 0.8) / positions.len() as f64
    };
    
    let response = DemoStatsResponse {
        total_positions: positions.len(),
        total_value_usd,
        active_protocols,
        risk_summary: DemoRiskSummary {
            low_risk_positions: low_risk,
            medium_risk_positions: medium_risk,
            high_risk_positions: high_risk,
            overall_risk_score,
        },
    };
    
    Ok(Json(response))
}

/// Trigger real-time risk analysis on demo positions
pub async fn analyze_demo_risks(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    let positions = demo_service.get_demo_positions().await?;
    
    let mut risk_analyses = Vec::new();
    
    for position in positions {
        // Use your existing risk calculation services
        let impermanent_loss_risk = calculate_il_risk(&position);
        let liquidity_risk = calculate_liquidity_risk(&position);
        let protocol_risk = 0.3; // Uniswap V3 is generally low risk
        
        let overall_risk = (impermanent_loss_risk + liquidity_risk + protocol_risk) / 3.0;
        
        risk_analyses.push(serde_json::json!({
            "position_id": position.id,
            "pool": format!("{}/{}", position.token0_address, position.token1_address),
            "overall_risk_score": overall_risk,
            "risk_factors": {
                "impermanent_loss": impermanent_loss_risk,
                "liquidity_risk": liquidity_risk,
                "protocol_risk": protocol_risk
            },
            "recommendations": generate_risk_recommendations(overall_risk),
            "last_analyzed": chrono::Utc::now()
        }));
    }
    
    Ok(Json(serde_json::json!({
        "analyses": risk_analyses,
        "summary": {
            "total_positions_analyzed": risk_analyses.len(),
            "average_risk_score": risk_analyses.iter()
                .map(|a| a["overall_risk_score"].as_f64().unwrap_or(0.0))
                .sum::<f64>() / risk_analyses.len() as f64,
            "analysis_timestamp": chrono::Utc::now()
        }
    })))
}

fn calculate_il_risk(position: &crate::models::Position) -> f64 {
    // Simplified IL risk calculation
    let fee_tier = position.fee_tier as f64 / 10000.0; // Convert to percentage
    
    // Lower fee tiers generally have higher IL risk for similar volatility
    match fee_tier {
        f if f <= 0.01 => 0.8, // 0.01% - high IL risk (stablecoin pairs)
        f if f <= 0.05 => 0.5, // 0.05% - medium IL risk
        f if f <= 0.3 => 0.3,  // 0.3% - lower IL risk
        _ => 0.4, // 1%+ - variable risk
    }
}

fn calculate_liquidity_risk(position: &crate::models::Position) -> f64 {
    // Simplified liquidity risk based on tick range
    let tick_lower = position.tick_lower;
    let tick_upper = position.tick_upper;
    let range = (tick_upper - tick_lower).abs() as f64;
        
    // Wider ranges generally have lower liquidity risk
    if range > 100000.0 {
        0.2 // Wide range, low risk
    } else if range > 50000.0 {
        0.4 // Medium range, medium risk
    } else {
        0.7 // Narrow range, higher risk
    }
}

fn generate_risk_recommendations(risk_score: f64) -> Vec<String> {
    let mut recommendations = Vec::new();
    
    if risk_score > 0.7 {
        recommendations.push("Consider reducing position size due to high risk".to_string());
        recommendations.push("Monitor position closely for potential exit opportunities".to_string());
        recommendations.push("Set up automated alerts for significant price movements".to_string());
    } else if risk_score > 0.4 {
        recommendations.push("Position has moderate risk - regular monitoring recommended".to_string());
        recommendations.push("Consider setting stop-loss levels".to_string());
    } else {
        recommendations.push("Position appears relatively stable".to_string());
        recommendations.push("Continue regular monitoring".to_string());
    }
    
    recommendations.push("Diversify across multiple protocols to reduce concentration risk".to_string());
    recommendations
}

/// Calculate risk distribution for a set of positions
fn calculate_risk_distribution(positions: &[crate::models::Position]) -> (usize, usize, usize) {
    let mut low_risk = 0;
    let mut medium_risk = 0;
    let mut high_risk = 0;
    
    for position in positions {
        let risk_score = calculate_position_risk_score(position);
        if risk_score < 0.3 {
            low_risk += 1;
        } else if risk_score < 0.6 {
            medium_risk += 1;
        } else {
            high_risk += 1;
        }
    }
    
    (low_risk, medium_risk, high_risk)
}

/// Calculate overall risk score for a position
fn calculate_position_risk_score(position: &crate::models::Position) -> f64 {
    let il_risk = calculate_il_risk(position);
    let liquidity_risk = calculate_liquidity_risk(position);
    let protocol_risk = 0.3; // Fixed protocol risk for Uniswap V3
    
    // Weighted average of different risk factors
    (il_risk * 0.5 + liquidity_risk * 0.3 + protocol_risk * 0.2)
}

/// Get positions for any wallet address (demo mode - returns whale data)
pub async fn get_wallet_positions(
    State(state): State<AppState>,
    Path(wallet_address): Path<String>,
) -> Result<Json<DemoPositionsResponse>, AppError> {
    // For demo purposes, ignore the actual wallet address and return whale data
    // This allows any user to "connect their wallet" and see realistic data
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    
    // Get real positions from whale addresses regardless of input address
    let positions = demo_service.get_demo_positions().await?;
    
    let response = DemoPositionsResponse {
        total_count: positions.len(),
        data_source: format!("Demo data for address: {}...{}", 
            &wallet_address[..6], 
            &wallet_address[wallet_address.len()-4..]
        ),
        positions,
        last_updated: chrono::Utc::now(),
    };
    
    Ok(Json(response))
}

/// Get portfolio stats for any wallet address (demo mode)
pub async fn get_wallet_stats(
    State(state): State<AppState>,
    Path(wallet_address): Path<String>,
) -> Result<Json<DemoStatsResponse>, AppError> {
    // For demo purposes, return whale portfolio stats for any address
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    let positions = demo_service.get_demo_positions().await?;
    
    let total_value_usd: f64 = positions.iter()
        .map(|p| p.token0_amount.to_string().parse::<f64>().unwrap_or(0.0) + 
                 p.token1_amount.to_string().parse::<f64>().unwrap_or(0.0))
        .sum();
    
    let protocols: Vec<String> = positions.iter()
        .map(|p| p.protocol.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    
    // Calculate risk distribution
    let (low_risk, medium_risk, high_risk) = calculate_risk_distribution(&positions);
    let overall_risk = (low_risk as f64 * 0.2 + medium_risk as f64 * 0.5 + high_risk as f64 * 0.8) 
        / positions.len() as f64;
    
    let response = DemoStatsResponse {
        total_positions: positions.len(),
        total_value_usd,
        active_protocols: protocols,
        risk_summary: DemoRiskSummary {
            low_risk_positions: low_risk,
            medium_risk_positions: medium_risk,
            high_risk_positions: high_risk,
            overall_risk_score: overall_risk,
        },
    };
    
    Ok(Json(response))
}

/// Analyze risks for any wallet address (demo mode)
pub async fn analyze_wallet_risks(
    State(state): State<AppState>,
    Path(wallet_address): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // For demo purposes, return whale risk analysis for any address
    let demo_service = DemoDataService::new((*state.blockchain_service).clone());
    let positions = demo_service.get_demo_positions().await?;
    
    let mut risk_analyses = Vec::new();
    
    for position in &positions {
        let risk_score = calculate_position_risk_score(position);
        let recommendations = generate_risk_recommendations(risk_score);
        
        let analysis = serde_json::json!({
            "position_id": position.id,
            "pool": format!("{}/{}", position.token0_address, position.token1_address),
            "overall_risk_score": risk_score,
            "risk_factors": {
                "impermanent_loss": (risk_score * 2.0).min(1.0),
                "liquidity_risk": 0.2,
                "protocol_risk": 0.3,
            },
            "recommendations": recommendations,
            "last_analyzed": chrono::Utc::now(),
        });
        
        risk_analyses.push(analysis);
    }
    
    let summary = serde_json::json!({
        "analyses": risk_analyses,
        "summary": {
            "total_positions_analyzed": positions.len(),
            "average_risk_score": risk_analyses.iter()
                .map(|a| a["overall_risk_score"].as_f64().unwrap_or(0.0))
                .sum::<f64>() / risk_analyses.len() as f64,
            "analysis_timestamp": chrono::Utc::now(),
            "wallet_address": wallet_address,
        }
    });
    
    Ok(Json(summary))
}

/// Create demo API routes
pub fn create_demo_routes() -> axum::Router<crate::AppState> {
    use axum::routing::get;
    
    axum::Router::new()
        .route("/demo/positions", get(get_demo_positions))
        .route("/demo/stats", get(get_demo_stats))
        .route("/demo/analyze", get(analyze_demo_risks))
        // Wallet demo endpoints - accept any address but return whale data
        .route("/wallet/:address/positions", get(get_wallet_positions))
        .route("/wallet/:address/stats", get(get_wallet_stats))
        .route("/wallet/:address/analyze", get(analyze_wallet_risks))
}
