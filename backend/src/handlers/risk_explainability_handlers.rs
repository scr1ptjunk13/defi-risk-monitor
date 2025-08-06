use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::info;

use crate::models::risk_explanation::*;
use crate::services::{
    AIRiskService,
    risk_calculator::RiskCalculator,
};
use crate::error::AppError;
use crate::AppState;

/// Query parameters for risk explanation
#[derive(Debug, Deserialize)]
pub struct ExplainRiskQuery {
    /// User address for personalized recommendations
    pub user_address: Option<String>,
    /// Detail level: "summary", "detailed", "comprehensive"
    pub detail_level: Option<String>,
    /// Include market context
    pub include_market_context: Option<bool>,
    /// Include historical analysis
    pub include_historical_analysis: Option<bool>,
    /// Language for explanations
    pub language: Option<String>,
}

/// Response for risk explanation endpoint
#[derive(Debug, Serialize)]
pub struct ExplainRiskApiResponse {
    pub success: bool,
    pub explanation: RiskExplanation,
    pub metadata: ExplanationMetadata,
}

/// Response for risk factors summary
#[derive(Debug, Serialize)]
pub struct RiskFactorsSummaryResponse {
    pub success: bool,
    pub position_id: Uuid,
    pub risk_score: String,
    pub risk_level: String,
    pub critical_factors: Vec<RiskFactorSummary>,
    pub immediate_actions: Vec<ActionSummary>,
    pub confidence_level: f64,
}

/// Simplified risk factor for summary
#[derive(Debug, Serialize)]
pub struct RiskFactorSummary {
    pub name: String,
    pub score: String,
    pub severity: String,
    pub explanation: String,
}

/// Simplified action summary
#[derive(Debug, Serialize)]
pub struct ActionSummary {
    pub title: String,
    pub category: String,
    pub description: String,
    pub priority: String,
}

/// Get comprehensive risk explanation for a position
/// GET /api/v1/positions/{id}/explain-risk
pub async fn explain_position_risk(
    Path(position_id): Path<Uuid>,
    Query(params): Query<ExplainRiskQuery>,
    State(state): State<AppState>,
) -> Result<Json<ExplainRiskApiResponse>, AppError> {
    info!("Explaining risk for position: {}", position_id);

    // Fetch position from database
    let row = sqlx::query!("SELECT * FROM positions WHERE id = $1", position_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::NotFound(format!("Position not found: {}", e)))?;
    
    let position = crate::models::Position {
        id: row.id,
        user_address: row.user_address,
        protocol: row.protocol,
        pool_address: row.pool_address,
        token0_address: row.token0_address,
        token1_address: row.token1_address,
        token0_amount: row.token0_amount,
        token1_amount: row.token1_amount,
        liquidity: row.liquidity,
        tick_lower: row.tick_lower,
        tick_upper: row.tick_upper,
        fee_tier: row.fee_tier,
        chain_id: row.chain_id,
        entry_token0_price_usd: row.entry_token0_price_usd,
        entry_token1_price_usd: row.entry_token1_price_usd,
        entry_timestamp: Some(row.entry_timestamp),
        created_at: Some(row.created_at.unwrap_or_else(|| chrono::Utc::now())),
        updated_at: Some(row.updated_at.unwrap_or_else(|| chrono::Utc::now())),
    };

    // Get current pool state
    let pool_state = state.blockchain_service
        .get_pool_state(&position.pool_address, position.chain_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get pool state: {}", e)))?;

    // Calculate current risk metrics
    let risk_calculator = RiskCalculator::new();
    let risk_config = crate::models::RiskConfig::default();
    let risk_metrics = risk_calculator
        .calculate_position_risk(
            &position,
            &pool_state,
            &risk_config,
            &[pool_state.clone()],
            &[],
            &[],
            None,
            None,
        )
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to calculate risk: {}", e)))?;

    // Create AI risk service and generate explanation
    let ai_risk_service = AIRiskService::new(
        state.settings.ai_service.url.clone(),
        state.settings.ai_service.fallback_enabled,
    );
    
    let start_time = std::time::Instant::now();
    let explanation = ai_risk_service
        .explain_position_risk(&position, &pool_state, &risk_metrics)
        .await?;
    let processing_time = start_time.elapsed().as_millis() as u64;

    let metadata = ExplanationMetadata {
        processing_time_ms: processing_time,
        data_sources: vec![
            "blockchain_data".to_string(),
            "risk_calculator".to_string(),
            "market_data".to_string(),
        ],
        model_version: "1.0.0".to_string(),
        quality_score: explanation.confidence_level,
        used_cached_data: false, // Simplified
    };

    let response = ExplainRiskApiResponse {
        success: true,
        explanation,
        metadata,
    };

    info!("Risk explanation generated in {}ms", processing_time);
    Ok(Json(response))
}

/// Get simplified risk factors summary
/// GET /api/v1/positions/{id}/risk-summary
pub async fn get_risk_summary(
    Path(position_id): Path<Uuid>,
    Query(params): Query<ExplainRiskQuery>,
    State(state): State<AppState>,
) -> Result<Json<RiskFactorsSummaryResponse>, AppError> {
    info!("Getting risk summary for position: {}", position_id);

    // Fetch position from database
    let row = sqlx::query!("SELECT * FROM positions WHERE id = $1", position_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::NotFound(format!("Position not found: {}", e)))?;
    
    let position = crate::models::Position {
        id: row.id,
        user_address: row.user_address,
        protocol: row.protocol,
        pool_address: row.pool_address,
        token0_address: row.token0_address,
        token1_address: row.token1_address,
        token0_amount: row.token0_amount,
        token1_amount: row.token1_amount,
        liquidity: row.liquidity,
        tick_lower: row.tick_lower,
        tick_upper: row.tick_upper,
        fee_tier: row.fee_tier,
        chain_id: row.chain_id,
        entry_token0_price_usd: row.entry_token0_price_usd,
        entry_token1_price_usd: row.entry_token1_price_usd,
        entry_timestamp: Some(row.entry_timestamp),
        created_at: Some(row.created_at.unwrap_or_else(|| chrono::Utc::now())),
        updated_at: Some(row.updated_at.unwrap_or_else(|| chrono::Utc::now())),
    };

    // Get current pool state
    let pool_state = state.blockchain_service
        .get_pool_state(&position.pool_address, position.chain_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get pool state: {}", e)))?;

    // Calculate current risk metrics
    let risk_calculator = RiskCalculator::new();
    let risk_config = crate::models::RiskConfig::default();
    let risk_metrics = risk_calculator
        .calculate_position_risk(
            &position,
            &pool_state,
            &risk_config,
            &[pool_state.clone()],
            &[],
            &[],
            None,
            None,
        )
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to calculate risk: {}", e)))?;

    // Generate explanation using AI risk service
    let ai_risk_service = AIRiskService::new(
        state.settings.ai_service.url.clone(),
        state.settings.ai_service.fallback_enabled,
    );

    let explanation = ai_risk_service
        .explain_position_risk(&position, &pool_state, &risk_metrics)
        .await?;

    // Convert to simplified format
    let critical_factors: Vec<RiskFactorSummary> = explanation
        .get_critical_factors()
        .into_iter()
        .map(|factor| RiskFactorSummary {
            name: factor.name.clone(),
            score: format!("{:.1}%", factor.score.clone() * bigdecimal::BigDecimal::from(100)),
            severity: factor.severity.clone(),
            explanation: factor.explanation.clone(),
        })
        .collect();

    let immediate_actions: Vec<ActionSummary> = explanation
        .get_immediate_actions()
        .into_iter()
        .map(|action| ActionSummary {
            title: action.title.clone(),
            category: action.category.clone(),
            description: action.description.clone(),
            priority: action.priority.clone(),
        })
        .collect();

    let response = RiskFactorsSummaryResponse {
        success: true,
        position_id,
        risk_score: format!("{:.1}%", explanation.risk_score * bigdecimal::BigDecimal::from(100)),
        risk_level: explanation.risk_level,
        critical_factors,
        immediate_actions,
        confidence_level: explanation.confidence_level,
    };

    Ok(Json(response))
}

/// Get risk recommendations for a position
/// GET /api/v1/positions/{id}/recommendations
pub async fn get_risk_recommendations(
    Path(position_id): Path<Uuid>,
    Query(params): Query<ExplainRiskQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<RiskRecommendation>>, AppError> {
    info!("Getting risk recommendations for position: {}", position_id);

    // Fetch position from database
    let row = sqlx::query!("SELECT * FROM positions WHERE id = $1", position_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::NotFound(format!("Position not found: {}", e)))?;
    
    let position = crate::models::Position {
        id: row.id,
        user_address: row.user_address,
        protocol: row.protocol,
        pool_address: row.pool_address,
        token0_address: row.token0_address,
        token1_address: row.token1_address,
        token0_amount: row.token0_amount,
        token1_amount: row.token1_amount,
        liquidity: row.liquidity,
        tick_lower: row.tick_lower,
        tick_upper: row.tick_upper,
        fee_tier: row.fee_tier,
        chain_id: row.chain_id,
        entry_token0_price_usd: row.entry_token0_price_usd,
        entry_token1_price_usd: row.entry_token1_price_usd,
        entry_timestamp: Some(row.entry_timestamp),
        created_at: Some(row.created_at.unwrap_or_else(|| chrono::Utc::now())),
        updated_at: Some(row.updated_at.unwrap_or_else(|| chrono::Utc::now())),
    };

    // Get current pool state
    let pool_state = state.blockchain_service
        .get_pool_state(&position.pool_address, position.chain_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get pool state: {}", e)))?;

    // Calculate current risk metrics
    let risk_calculator = RiskCalculator::new();
    let risk_config = crate::models::RiskConfig::default();
    let risk_metrics = risk_calculator
        .calculate_position_risk(
            &position,
            &pool_state,
            &risk_config,
            &[pool_state.clone()],
            &[],
            &[],
            None,
            None,
        )
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to calculate risk: {}", e)))?;

    // Generate explanation using AI risk service
    let ai_risk_service = AIRiskService::new(
        state.settings.ai_service.url.clone(),
        state.settings.ai_service.fallback_enabled,
    );

    let explanation = ai_risk_service
        .explain_position_risk(&position, &pool_state, &risk_metrics)
        .await?;

    Ok(Json(explanation.recommendations))
}

/// Get market context for risk analysis
/// GET /api/v1/positions/{id}/market-context
pub async fn get_market_context(
    Path(position_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<MarketContext>, AppError> {
    info!("Getting market context for position: {}", position_id);

    // Fetch position from database
    let row = sqlx::query!("SELECT * FROM positions WHERE id = $1", position_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|e| AppError::NotFound(format!("Position not found: {}", e)))?;
    
    let position = crate::models::Position {
        id: row.id,
        user_address: row.user_address,
        protocol: row.protocol,
        pool_address: row.pool_address,
        token0_address: row.token0_address,
        token1_address: row.token1_address,
        token0_amount: row.token0_amount,
        token1_amount: row.token1_amount,
        liquidity: row.liquidity,
        tick_lower: row.tick_lower,
        tick_upper: row.tick_upper,
        fee_tier: row.fee_tier,
        chain_id: row.chain_id,
        entry_token0_price_usd: row.entry_token0_price_usd,
        entry_token1_price_usd: row.entry_token1_price_usd,
        entry_timestamp: Some(row.entry_timestamp),
        created_at: Some(row.created_at.unwrap_or_else(|| chrono::Utc::now())),
        updated_at: Some(row.updated_at.unwrap_or_else(|| chrono::Utc::now())),
    };

    // Get current pool state
    let pool_state = state.blockchain_service
        .get_pool_state(&position.pool_address, position.chain_id)
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to get pool state: {}", e)))?;

    // Calculate current risk metrics
    let risk_calculator = RiskCalculator::new();
    let risk_config = crate::models::RiskConfig::default();
    let risk_metrics = risk_calculator
        .calculate_position_risk(
            &position,
            &pool_state,
            &risk_config,
            &[pool_state.clone()],
            &[],
            &[],
            None,
            None,
        )
        .await
        .map_err(|e| AppError::InternalError(format!("Failed to calculate risk: {}", e)))?;

    // Generate market context using AI risk service
    let ai_risk_service = AIRiskService::new(
        state.settings.ai_service.url.clone(),
        state.settings.ai_service.fallback_enabled,
    );

    let explanation = ai_risk_service
        .explain_position_risk(&position, &pool_state, &risk_metrics)
        .await?;

    Ok(Json(explanation.market_context))
}
