use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use crate::services::risk_calculator::RiskMetrics;

#[derive(Deserialize)]
pub struct RiskCalculationQuery {
    pub position_id: Uuid,
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
    State(pool): State<PgPool>,
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
    .fetch_optional(&pool)
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
