use std::collections::HashMap;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use std::convert::TryFrom;

use crate::models::{Position, PoolState};
use crate::models::risk_explanation::{RiskExplanation, RiskFactor, RiskRecommendation, MarketContext};
use crate::services::risk_calculator::RiskMetrics;
use crate::services::ai_client::{AIClient, AIPredictionResult, AIExplanationResult};
use crate::error::AppError;

/// New AI-powered risk service that replaces the old rule-based explainable AI
pub struct AIRiskService {
    ai_client: AIClient,
    fallback_enabled: bool,
}

/// AI-powered risk explanation result
#[derive(Debug, Clone)]
pub struct AIRiskExplanation {
    pub overall_risk_score: BigDecimal,
    pub confidence: f64,
    pub summary: String,
    pub key_insights: Vec<String>,
    pub risk_factors: Vec<AIRiskFactorExplanation>,
    pub recommendations: Vec<AIActionRecommendation>,
    pub model_version: String,
    pub explanation_method: String,
    pub prediction_timestamp: DateTime<Utc>,
}

/// AI-explained risk factor
#[derive(Debug, Clone)]
pub struct AIRiskFactorExplanation {
    pub factor_id: String,
    pub factor_name: String,
    pub importance_score: f64,
    pub contribution: BigDecimal,
    pub explanation: String,
    pub evidence: Vec<String>,
    pub feature_values: HashMap<String, f64>,
    pub shap_values: Option<HashMap<String, f64>>,
}

/// AI-generated action recommendation
#[derive(Debug, Clone)]
pub struct AIActionRecommendation {
    pub action: String,
    pub reasoning: String,
    pub confidence: f64,
    pub urgency: String,
    pub expected_impact: Option<String>,
}

impl AIRiskService {
    /// Create a new AI risk service
    pub fn new(ai_service_url: String, fallback_enabled: bool) -> Self {
        Self {
            ai_client: AIClient::new(ai_service_url),
            fallback_enabled,
        }
    }

    /// Generate AI-powered risk explanation (replaces old rule-based system)
    pub async fn explain_risk_ai(
        &self,
        position: &Position,
        risk_metrics: &RiskMetrics,
        pool_state: &PoolState,
    ) -> Result<AIRiskExplanation, AppError> {
        // Check if AI service is available
        let ai_available = self.ai_client.health_check().await.unwrap_or(false);
        
        if !ai_available {
            if self.fallback_enabled {
                tracing::warn!("AI service unavailable, using fallback explanation");
                return self.generate_fallback_explanation(position, risk_metrics, pool_state).await;
            } else {
                return Err(AppError::ExternalServiceError(
                    "AI service is unavailable and fallback is disabled".to_string(),
                ));
            }
        }

        // Get AI prediction
        let prediction = self
            .ai_client
            .predict_risk(position, pool_state, risk_metrics)
            .await?;

        // Get AI explanation
        let explanation = self
            .ai_client
            .explain_prediction(&prediction, position, pool_state, risk_metrics)
            .await?;

        // Convert to our internal format
        Ok(self.convert_ai_explanation(prediction, explanation))
    }

    /// Get AI model information
    pub async fn get_model_info(&self) -> Result<HashMap<String, serde_json::Value>, AppError> {
        self.ai_client.get_model_info().await
    }

    /// Check if AI service is healthy
    pub async fn is_ai_service_healthy(&self) -> bool {
        self.ai_client.health_check().await.unwrap_or(false)
    }

    /// Convert AI service response to internal format
    fn convert_ai_explanation(
        &self,
        prediction: AIPredictionResult,
        explanation: AIExplanationResult,
    ) -> AIRiskExplanation {
        let risk_factors = prediction
            .risk_factors
            .into_iter()
            .zip(explanation.risk_factors.into_iter())
            .map(|(pred_factor, exp_factor)| AIRiskFactorExplanation {
                factor_id: pred_factor.factor_id,
                factor_name: pred_factor.factor_name,
                importance_score: pred_factor.importance_score,
                contribution: BigDecimal::try_from(pred_factor.contribution).unwrap_or_else(|_| BigDecimal::from(0)),
                explanation: exp_factor.explanation,
                evidence: exp_factor.evidence,
                feature_values: pred_factor.feature_values,
                shap_values: pred_factor.shap_values,
            })
            .collect();

        let recommendations = explanation
            .recommendations
            .into_iter()
            .map(|rec| AIActionRecommendation {
                action: rec.action,
                reasoning: rec.reasoning,
                confidence: rec.confidence,
                urgency: rec.urgency,
                expected_impact: rec.expected_impact,
            })
            .collect();

        AIRiskExplanation {
            overall_risk_score: BigDecimal::try_from(prediction.overall_risk_score).unwrap_or_else(|_| BigDecimal::from(0)),
            confidence: prediction.confidence,
            summary: explanation.summary,
            key_insights: explanation.key_insights,
            risk_factors,
            recommendations,
            model_version: prediction.model_version,
            explanation_method: explanation.explanation_method,
            prediction_timestamp: prediction.prediction_timestamp,
        }
    }

    /// Fallback explanation when AI service is unavailable
    async fn generate_fallback_explanation(
        &self,
        _position: &Position,
        risk_metrics: &RiskMetrics,
        _pool_state: &PoolState,
    ) -> Result<AIRiskExplanation, AppError> {
        tracing::info!("Generating fallback risk explanation (AI service unavailable)");

        // Simple fallback - much simpler than the old rule-based system
        let risk_score = risk_metrics.overall_risk_score.clone();
        let risk_level = if risk_score > BigDecimal::from(70) {
            "HIGH"
        } else if risk_score > BigDecimal::from(40) {
            "MEDIUM"
        } else {
            "LOW"
        };

        let summary = format!(
            "Risk assessment: {} (score: {:.1}%). AI service unavailable - using basic analysis.",
            risk_level,
            &risk_score
        );

        let mut key_insights = vec![
            "AI-powered analysis temporarily unavailable".to_string(),
            "Basic risk metrics calculated using deterministic methods".to_string(),
        ];

        let mut risk_factors = vec![];
        let mut recommendations = vec![];

        // Only add basic factors if they're significant
        if risk_metrics.impermanent_loss > BigDecimal::from(10) {
            risk_factors.push(AIRiskFactorExplanation {
                factor_id: "impermanent_loss_basic".to_string(),
                factor_name: "Impermanent Loss".to_string(),
                importance_score: 0.8,
                contribution: risk_metrics.impermanent_loss.clone() / BigDecimal::from(100),
                explanation: format!("Position experiencing {}% impermanent loss", risk_metrics.impermanent_loss),
                evidence: vec!["Price divergence detected".to_string()],
                feature_values: HashMap::new(),
                shap_values: None,
            });

            if risk_metrics.impermanent_loss > BigDecimal::from(20) {
                recommendations.push(AIActionRecommendation {
                    action: "Monitor position closely".to_string(),
                    reasoning: "High impermanent loss detected".to_string(),
                    confidence: 0.7,
                    urgency: "soon".to_string(),
                    expected_impact: Some("Prevent further losses".to_string()),
                });
            }
        }

        if risk_metrics.liquidity_score > BigDecimal::from(60) {
            key_insights.push("Liquidity risk detected - consider position size".to_string());
        }

        Ok(AIRiskExplanation {
            overall_risk_score: risk_score,
            confidence: 0.6, // Lower confidence for fallback
            summary,
            key_insights,
            risk_factors,
            recommendations,
            model_version: "fallback-1.0".to_string(),
            explanation_method: "Basic deterministic analysis (AI unavailable)".to_string(),
            prediction_timestamp: Utc::now(),
        })
    }

    /// Compatibility method for existing handlers - converts AI explanation to legacy format
    pub async fn explain_position_risk(
        &self,
        position: &Position,
        pool_state: &PoolState,
        risk_metrics: &RiskMetrics,
    ) -> Result<RiskExplanation, AppError> {
        // Get AI explanation
        let ai_explanation = self.explain_risk_ai(position, risk_metrics, pool_state).await?;
        
        // Convert to legacy RiskExplanation format
        let mut risk_explanation = RiskExplanation::new(
            risk_metrics.overall_risk_score.clone(),
            position.id,
        );
        
        // Set summary and insights
        risk_explanation.set_summary(ai_explanation.summary);
        for insight in ai_explanation.key_insights {
            risk_explanation.add_key_insight(insight);
        }
        
        // Convert AI risk factors to legacy format
        for ai_factor in ai_explanation.risk_factors {
            let risk_factor = RiskFactor {
                factor_type: ai_factor.factor_id.clone(),
                name: ai_factor.factor_name,
                score: BigDecimal::try_from(ai_factor.importance_score).unwrap_or_default(),
                weight: BigDecimal::from(1), // Default weight
                contribution: ai_factor.contribution,
                explanation: ai_factor.explanation,
                current_value: None,
                threshold_value: None,
                severity: if ai_factor.importance_score > 0.8 {
                    "critical".to_string()
                } else if ai_factor.importance_score > 0.6 {
                    "high".to_string()
                } else if ai_factor.importance_score > 0.4 {
                    "medium".to_string()
                } else {
                    "low".to_string()
                },
                trend: "stable".to_string(), // Default trend
                historical_context: None,
            };
            
            if ai_factor.importance_score > 0.5 {
                risk_explanation.add_primary_factor(risk_factor);
            } else {
                risk_explanation.add_secondary_factor(risk_factor);
            }
        }
        
        // Convert AI recommendations to legacy format
        for ai_rec in ai_explanation.recommendations {
            let recommendation = RiskRecommendation {
                priority: ai_rec.urgency.clone(),
                category: "general".to_string(), // Default category
                title: ai_rec.action.clone(),
                description: ai_rec.reasoning,
                expected_impact: ai_rec.expected_impact.unwrap_or_else(|| "Unknown impact".to_string()),
                implementation_cost: None,
                time_sensitivity: ai_rec.urgency.clone(),
                resources: vec![],
            };
            risk_explanation.add_recommendation(recommendation);
        }
        
        // Set default market context
        risk_explanation.market_context = MarketContext::default();
        
        Ok(risk_explanation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Position;
    use bigdecimal::BigDecimal;

    fn create_test_position() -> Position {
        Position {
            id: "test-position".to_string(),
            user_id: "test-user".to_string(),
            pool_address: "0x1234".to_string(),
            chain_id: 1,
            token0_address: "0xA0b86a33E6441e6C7D7b0b0b5C5D5E5F5f5f5f5f".to_string(),
            token1_address: "0xB0b86a33E6441e6C7D7b0b0b5C5D5E5F5f5f5f5f".to_string(),
            liquidity: BigDecimal::from(1000000),
            entry_price0: BigDecimal::from(1800),
            entry_price1: BigDecimal::from(1),
            current_value: BigDecimal::from(10000),
            entry_value: BigDecimal::from(10000),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_risk_metrics() -> RiskMetrics {
        RiskMetrics {
            overall_risk_score: BigDecimal::from(45),
            impermanent_loss: BigDecimal::from(15),
            liquidity_score: BigDecimal::from(30),
            volatility_score: BigDecimal::from(40),
            concentration_risk: BigDecimal::from(25),
        }
    }

    fn create_test_pool_state() -> PoolState {
        PoolState {
            id: "test-pool".to_string(),
            pool_address: "0x1234".to_string(),
            chain_id: 1,
            current_tick: 200000,
            sqrt_price_x96: "1234567890123456789012345678".to_string(),
            liquidity: "1000000000000000000".to_string(),
            token0_price: BigDecimal::from(1800),
            token1_price: BigDecimal::from(1),
            tvl_usd: Some(BigDecimal::from(5000000)),
            volume_24h: Some(BigDecimal::from(1000000)),
            fees_24h: Some(BigDecimal::from(5000)),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_fallback_explanation() {
        let ai_service = AIRiskService::new("http://localhost:8001".to_string(), true);
        let position = create_test_position();
        let risk_metrics = create_test_risk_metrics();
        let pool_state = create_test_pool_state();

        let explanation = ai_service
            .generate_fallback_explanation(&position, &risk_metrics, &pool_state)
            .await
            .unwrap();

        assert!(explanation.summary.contains("MEDIUM"));
        assert!(explanation.confidence < 1.0);
        assert_eq!(explanation.model_version, "fallback-1.0");
        assert!(!explanation.risk_factors.is_empty());
    }

    #[tokio::test]
    async fn test_ai_service_creation() {
        let ai_service = AIRiskService::new("http://localhost:8001".to_string(), true);
        
        // Test that service is created properly
        assert!(ai_service.fallback_enabled);
    }
}
