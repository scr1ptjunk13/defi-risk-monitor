use std::collections::HashMap;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};

use crate::models::{Position, PoolState};
use crate::services::risk_calculator::RiskMetrics;
use crate::error::AppError;

/// Client for communicating with the Python AI microservice
#[derive(Clone)]
pub struct AIClient {
    client: Client,
    base_url: String,
}

/// Request payload for AI prediction
#[derive(Debug, Serialize)]
pub struct AIPredictionRequest {
    pub position: AIPositionData,
    pub pool_state: AIPoolStateData,
    pub risk_metrics: AIRiskMetricsData,
    pub historical_data: Option<Vec<AIPoolStateData>>,
}

/// Position data for AI service
#[derive(Debug, Serialize)]
pub struct AIPositionData {
    pub id: String,
    pub pool_address: String,
    pub chain_id: i32,
    pub token0_address: String,
    pub token1_address: String,
    pub liquidity: f64,
    pub entry_price0: f64,
    pub entry_price1: f64,
    pub current_value: f64,
    pub entry_value: f64,
}

/// Pool state data for AI service
#[derive(Debug, Serialize)]
pub struct AIPoolStateData {
    pub pool_address: String,
    pub chain_id: i32,
    pub current_tick: i32,
    pub sqrt_price_x96: String,
    pub liquidity: String,
    pub token0_price: f64,
    pub token1_price: f64,
    pub tvl_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub fees_24h: Option<f64>,
}

/// Risk metrics data for AI service
#[derive(Debug, Serialize)]
pub struct AIRiskMetricsData {
    pub overall_risk_score: f64,
    pub impermanent_loss: f64,
    pub liquidity_score: f64,
    pub volatility_score: f64,
    pub concentration_risk: f64,
}

/// AI prediction result from Python service
#[derive(Debug, Serialize, Deserialize)]
pub struct AIPredictionResult {
    pub overall_risk_score: f64,
    pub confidence: f64,
    pub risk_factors: Vec<AIRiskFactor>,
    pub predictions: HashMap<String, f64>,
    pub model_version: String,
    pub prediction_timestamp: DateTime<Utc>,
}

/// AI-identified risk factor
#[derive(Debug, Serialize, Deserialize)]
pub struct AIRiskFactor {
    pub factor_id: String,
    pub factor_name: String,
    pub importance_score: f64,
    pub contribution: f64,
    pub feature_values: HashMap<String, f64>,
    pub shap_values: Option<HashMap<String, f64>>,
}

/// AI-generated explanation
#[derive(Debug, Deserialize)]
pub struct AIExplanationResult {
    pub summary: String,
    pub key_insights: Vec<String>,
    pub risk_factors: Vec<AIExplainedFactor>,
    pub recommendations: Vec<AIRecommendation>,
    pub confidence: f64,
    pub explanation_method: String,
}

/// Explained risk factor from AI
#[derive(Debug, Deserialize)]
pub struct AIExplainedFactor {
    pub factor_name: String,
    pub explanation: String,
    pub importance: f64,
    pub evidence: Vec<String>,
}

/// AI-generated recommendation
#[derive(Debug, Deserialize)]
pub struct AIRecommendation {
    pub action: String,
    pub reasoning: String,
    pub confidence: f64,
    pub urgency: String,
    pub expected_impact: Option<String>,
}

impl AIClient {
    /// Create a new AI client
    pub fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
        }
    }

    /// Check if AI service is healthy
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let url = format!("{}/health", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => Ok(response.status().is_success()),
            Err(e) => {
                tracing::warn!("AI service health check failed: {}", e);
                Ok(false)
            }
        }
    }

    /// Get AI-powered risk prediction
    pub async fn predict_risk(
        &self,
        position: &Position,
        pool_state: &PoolState,
        risk_metrics: &RiskMetrics,
    ) -> Result<AIPredictionResult, AppError> {
        let request = self.build_prediction_request(position, pool_state, risk_metrics)?;
        let url = format!("{}/predict", self.base_url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("AI service request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalServiceError(format!(
                "AI service returned {}: {}",
                status, error_text
            )));
        }

        let prediction = response
            .json::<AIPredictionResult>()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse AI response: {}", e)))?;

        Ok(prediction)
    }

    /// Get AI-powered explanation for a prediction
    pub async fn explain_prediction(
        &self,
        prediction: &AIPredictionResult,
        position: &Position,
        pool_state: &PoolState,
        risk_metrics: &RiskMetrics,
    ) -> Result<AIExplanationResult, AppError> {
        let request = self.build_prediction_request(position, pool_state, risk_metrics)?;
        let url = format!("{}/explain", self.base_url);

        #[derive(Serialize)]
        struct ExplainRequest {
            prediction: AIPredictionResult,
            request: AIPredictionRequest,
        }

        let explain_request = ExplainRequest {
            prediction: prediction.clone(),
            request,
        };

        let response = self
            .client
            .post(&url)
            .json(&explain_request)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("AI explanation request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(AppError::ExternalServiceError(format!(
                "AI explanation service returned {}: {}",
                status, error_text
            )));
        }

        let explanation = response
            .json::<AIExplanationResult>()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse AI explanation: {}", e)))?;

        Ok(explanation)
    }

    /// Get model information from AI service
    pub async fn get_model_info(&self) -> Result<HashMap<String, serde_json::Value>, AppError> {
        let url = format!("{}/models/info", self.base_url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("AI model info request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExternalServiceError(
                "Failed to get AI model info".to_string(),
            ));
        }

        let model_info = response
            .json::<HashMap<String, serde_json::Value>>()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to parse model info: {}", e)))?;

        Ok(model_info)
    }

    /// Build prediction request from Rust data structures
    fn build_prediction_request(
        &self,
        position: &Position,
        pool_state: &PoolState,
        risk_metrics: &RiskMetrics,
    ) -> Result<AIPredictionRequest, AppError> {
        // Convert BigDecimal to f64 for JSON serialization
        let to_f64 = |bd: &BigDecimal| -> f64 {
            bd.to_string().parse().unwrap_or(0.0)
        };

        let ai_position = AIPositionData {
            id: position.id.to_string(),
            pool_address: position.pool_address.clone(),
            chain_id: position.chain_id,
            token0_address: position.token0_address.clone(),
            token1_address: position.token1_address.clone(),
            liquidity: to_f64(&position.liquidity),
            entry_price0: position.entry_token0_price_usd.as_ref().map(to_f64).unwrap_or(0.0),
            entry_price1: position.entry_token1_price_usd.as_ref().map(to_f64).unwrap_or(0.0),
            current_value: to_f64(&position.token0_amount) + to_f64(&position.token1_amount), // Simplified
            entry_value: to_f64(&position.liquidity), // Simplified - use liquidity as proxy
        };

        let ai_pool_state = AIPoolStateData {
            pool_address: pool_state.pool_address.clone(),
            chain_id: pool_state.chain_id,
            current_tick: pool_state.current_tick,
            sqrt_price_x96: pool_state.sqrt_price_x96.to_string(),
            liquidity: pool_state.liquidity.to_string(),
            token0_price: pool_state.token0_price_usd.as_ref().map(to_f64).unwrap_or(0.0),
            token1_price: pool_state.token1_price_usd.as_ref().map(to_f64).unwrap_or(0.0),
            tvl_usd: pool_state.tvl_usd.as_ref().map(to_f64),
            volume_24h: None, // Not available in current model
            fees_24h: None, // Not available in current model
        };

        let ai_risk_metrics = AIRiskMetricsData {
            overall_risk_score: to_f64(&risk_metrics.overall_risk_score),
            impermanent_loss: to_f64(&risk_metrics.impermanent_loss),
            liquidity_score: to_f64(&risk_metrics.liquidity_score),
            volatility_score: to_f64(&risk_metrics.volatility_score),
            concentration_risk: to_f64(&risk_metrics.correlation_score), // Use correlation_score as proxy
        };

        Ok(AIPredictionRequest {
            position: ai_position,
            pool_state: ai_pool_state,
            risk_metrics: ai_risk_metrics,
            historical_data: None, // TODO: Add historical data support
        })
    }
}

// Make AIPredictionResult cloneable for explanation requests
impl Clone for AIPredictionResult {
    fn clone(&self) -> Self {
        Self {
            overall_risk_score: self.overall_risk_score,
            confidence: self.confidence,
            risk_factors: self.risk_factors.clone(),
            predictions: self.predictions.clone(),
            model_version: self.model_version.clone(),
            prediction_timestamp: self.prediction_timestamp,
        }
    }
}

impl Clone for AIRiskFactor {
    fn clone(&self) -> Self {
        Self {
            factor_id: self.factor_id.clone(),
            factor_name: self.factor_name.clone(),
            importance_score: self.importance_score,
            contribution: self.contribution,
            feature_values: self.feature_values.clone(),
            shap_values: self.shap_values.clone(),
        }
    }
}
