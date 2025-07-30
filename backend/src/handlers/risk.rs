use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use crate::services::risk_calculator::RiskMetrics;
use crate::AppState;
use crate::services::user_risk_config_service::UserRiskConfigService;
use crate::services::risk_calculator::RiskCalculator;
use crate::services::blockchain_service::BlockchainService;
use crate::services::monitoring_service::MonitoringService;


#[derive(Deserialize)]
pub struct RiskCalculationQuery {
    pub position_id: Uuid,
}

#[derive(Deserialize)]
pub struct RealTimeRiskQuery {
    pub position_id: Uuid,
    pub user_address: String,
    pub pool_address: String,
    pub chain_id: i32,
}

#[derive(Serialize, Deserialize)]
pub struct RiskCalculationResponse {
    pub position_id: Uuid,
    pub risk_metrics: RiskMetricsResponse,
}

#[derive(Serialize, Deserialize)]
pub struct RiskMetricsResponse {
    pub impermanent_loss: BigDecimal,
    pub price_impact: BigDecimal,
    pub volatility_score: BigDecimal,
    pub correlation_score: BigDecimal,
    pub liquidity_score: BigDecimal,
    pub overall_risk_score: BigDecimal,
    pub value_at_risk_1d: BigDecimal,
    pub value_at_risk_7d: BigDecimal,
}

impl From<RiskMetrics> for RiskMetricsResponse {
    fn from(metrics: RiskMetrics) -> Self {
        Self {
            impermanent_loss: metrics.impermanent_loss,
            price_impact: metrics.price_impact,
            volatility_score: metrics.volatility_score,
            correlation_score: metrics.correlation_score,
            liquidity_score: metrics.liquidity_score,
            overall_risk_score: metrics.overall_risk_score,
            value_at_risk_1d: metrics.value_at_risk_1d,
            value_at_risk_7d: metrics.value_at_risk_7d,
        }
    }
}

pub async fn calculate_risk(
    Query(params): Query<RiskCalculationQuery>,
    State(state): State<AppState>,
) -> Result<Json<RiskCalculationResponse>, StatusCode> {
    // Fetch the latest risk metrics for the position
    let risk_metrics = sqlx::query!(
        r#"
        SELECT impermanent_loss, price_impact, volatility_score, correlation_score,
               liquidity_score, overall_risk_score, value_at_risk_1d, value_at_risk_7d
        FROM risk_metrics 
        WHERE position_id = $1 
        ORDER BY calculated_at DESC 
        LIMIT 1
        "#,
        params.position_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match risk_metrics {
        Some(metrics) => {
            let risk_metrics_response = RiskMetricsResponse {
                impermanent_loss: metrics.impermanent_loss.unwrap_or_default(),
                price_impact: metrics.price_impact.unwrap_or_default(),
                volatility_score: metrics.volatility_score.unwrap_or_default(),
                correlation_score: metrics.correlation_score.unwrap_or_default(),
                liquidity_score: metrics.liquidity_score.unwrap_or_default(),
                overall_risk_score: metrics.overall_risk_score,
                value_at_risk_1d: metrics.value_at_risk_1d.unwrap_or_default(),
                value_at_risk_7d: metrics.value_at_risk_7d.unwrap_or_default(),
            };

            Ok(Json(RiskCalculationResponse {
                position_id: params.position_id,
                risk_metrics: risk_metrics_response,
            }))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Calculate risk in real-time using user risk configuration
pub async fn calculate_real_time_risk(
    Query(params): Query<RealTimeRiskQuery>,
    State(state): State<AppState>,
) -> Result<Json<RiskCalculationResponse>, StatusCode> {
    // Get user risk configuration
    let user_risk_service = UserRiskConfigService::new(state.db_pool.clone());
    let user_risk_params = match user_risk_service.get_risk_params(&params.user_address).await {
        Ok(params) => Some(params),
        Err(_) => None, // Use defaults if no user config found
    };
    
    // Fetch position from database using manual query to handle DateTime conversion
    let position_row = sqlx::query!(
        r#"
        SELECT id, user_address, protocol, pool_address, token0_address, token1_address,
               token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier, 
               chain_id, entry_token0_price_usd, entry_token1_price_usd, 
               entry_timestamp, created_at, updated_at
        FROM positions 
        WHERE id = $1
        "#,
        params.position_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let position = match position_row {
        Some(row) => crate::models::Position {
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
        },
        None => return Err(StatusCode::NOT_FOUND),
    };
    
    // Create services for risk calculation
    let blockchain_service = BlockchainService::new(&state.settings, state.db_pool.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _monitoring_service = MonitoringService::new(state.db_pool.clone(), state.settings.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // Get current pool state
    let pool_state = match blockchain_service.get_pool_state(&params.pool_address, params.chain_id).await {
        Ok(state) => state,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    
    // Get historical data for risk calculation - use empty vec for now
    // In production, you would implement a public method to fetch historical data
    let historical_data: Vec<crate::models::PoolState> = vec![];
    
    // Get price history (simplified - in real implementation would fetch actual token addresses)
    let token0_price_history = vec![]; // Placeholder
    let token1_price_history = vec![]; // Placeholder
    
    // Create risk calculator and calculate risk
    let risk_calculator = RiskCalculator::new();
    
    let risk_config = crate::models::RiskConfig::default(); // Use default config
    
    let risk_metrics = match risk_calculator.calculate_position_risk(
        &position,
        &pool_state,
        &risk_config,
        &historical_data,
        &token0_price_history,
        &token1_price_history,
        None, // protocol_name
        user_risk_params.as_ref(),
    ).await {
        Ok(metrics) => metrics,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    
    let risk_metrics_response = RiskMetricsResponse {
        impermanent_loss: risk_metrics.impermanent_loss,
        price_impact: risk_metrics.price_impact,
        volatility_score: risk_metrics.volatility_score,
        correlation_score: risk_metrics.correlation_score,
        liquidity_score: risk_metrics.liquidity_score,
        overall_risk_score: risk_metrics.overall_risk_score,
        value_at_risk_1d: risk_metrics.value_at_risk_1d,
        value_at_risk_7d: risk_metrics.value_at_risk_7d,
    };
    
    Ok(Json(RiskCalculationResponse {
        position_id: params.position_id,
        risk_metrics: risk_metrics_response,
    }))
}
