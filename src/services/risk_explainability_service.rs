use std::collections::HashMap;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tracing::{info, warn};
use num_traits::FromPrimitive;

use crate::models::{Position, PoolState};
use crate::models::risk_explanation::*;
use crate::services::risk_calculator::RiskMetrics;
use crate::error::AppError;

/// Service for generating explainable risk analysis
pub struct RiskExplainabilityService {
    /// Cache for risk explanations
    explanation_cache: HashMap<String, (RiskExplanation, DateTime<Utc>)>,
    /// Cache TTL in seconds
    cache_ttl_seconds: i64,
}

impl RiskExplainabilityService {
    /// Create a new risk explainability service
    pub fn new() -> Self {
        Self {
            explanation_cache: HashMap::new(),
            cache_ttl_seconds: 300, // 5 minutes
        }
    }

    /// Generate comprehensive risk explanation
    pub async fn explain_risk(
        &mut self,
        position: &Position,
        risk_metrics: &RiskMetrics,
        pool_state: &PoolState,
        request: &ExplainRiskRequest,
    ) -> Result<RiskExplanation, AppError> {
        let cache_key = format!("{}_{}", position.id, risk_metrics.overall_risk_score);
        
        // Check cache first
        if let Some((cached_explanation, cached_at)) = self.explanation_cache.get(&cache_key) {
            if (Utc::now() - *cached_at).num_seconds() < self.cache_ttl_seconds {
                return Ok(cached_explanation.clone());
            }
        }

        let mut explanation = RiskExplanation::new(
            risk_metrics.overall_risk_score.clone(),
            position.id,
        );

        // Generate primary risk factors
        self.generate_primary_factors(&mut explanation, risk_metrics, position, pool_state).await?;
        
        // Generate secondary risk factors
        self.generate_secondary_factors(&mut explanation, risk_metrics, position).await?;
        
        // Generate recommendations
        self.generate_recommendations(&mut explanation, risk_metrics, position, pool_state).await?;
        
        // Generate market context
        self.generate_market_context(&mut explanation, pool_state).await?;
        
        // Generate position context
        self.generate_position_context(&mut explanation, position, pool_state).await?;
        
        // Generate summary and insights
        self.generate_summary_and_insights(&mut explanation, risk_metrics, position).await?;
        
        // Set confidence level
        explanation.confidence_level = self.calculate_confidence_level(risk_metrics, position);

        // Cache the explanation
        self.explanation_cache.insert(cache_key, (explanation.clone(), Utc::now()));

        Ok(explanation)
    }

    /// Generate primary risk factors
    async fn generate_primary_factors(
        &self,
        explanation: &mut RiskExplanation,
        risk_metrics: &RiskMetrics,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<(), AppError> {
        // Impermanent Loss Factor
        if risk_metrics.impermanent_loss > BigDecimal::from(5) {
            let il_factor = RiskFactor {
                factor_type: "impermanent_loss".to_string(),
                name: "Impermanent Loss".to_string(),
                score: risk_metrics.impermanent_loss.clone() / BigDecimal::from(100),
                weight: BigDecimal::from(30) / BigDecimal::from(100), // 0.3
                contribution: (risk_metrics.impermanent_loss.clone() / BigDecimal::from(100)) * (BigDecimal::from(30) / BigDecimal::from(100)),
                explanation: format!(
                    "Your position is experiencing {}% impermanent loss due to price divergence between {} and {}. This means you would have more value if you simply held the tokens.",
                    risk_metrics.impermanent_loss,
                    "TOKEN0", // TODO: Fetch actual token symbols
                    "TOKEN1" // TODO: Fetch actual token symbols
                ),
                current_value: Some(risk_metrics.impermanent_loss.clone()),
                threshold_value: Some(BigDecimal::from(5)),
                severity: if risk_metrics.impermanent_loss > BigDecimal::from(20) { "critical" } else if risk_metrics.impermanent_loss > BigDecimal::from(10) { "high" } else { "medium" }.to_string(),
                trend: self.determine_il_trend(&risk_metrics.impermanent_loss),
                historical_context: Some("Impermanent loss above 10% typically indicates significant price divergence".to_string()),
            };
            explanation.add_primary_factor(il_factor);
        }

        // Liquidity Risk
        if risk_metrics.liquidity_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.4 {
            let liquidity_factor = RiskFactor {
                factor_type: "liquidity_risk".to_string(),
                name: "Liquidity Risk".to_string(),
                score: risk_metrics.liquidity_score.clone(),
                weight: BigDecimal::from(25) / BigDecimal::from(100), // 0.25
                contribution: risk_metrics.liquidity_score.clone() * (BigDecimal::from(25) / BigDecimal::from(100)),
                explanation: format!(
                    "Pool liquidity is concerning with TVL of ${:.2}. Low liquidity increases slippage and makes it harder to exit your position at fair prices.",
                    pool_state.tvl_usd.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string())
                ),
                current_value: pool_state.tvl_usd.clone(),
                threshold_value: Some(BigDecimal::from(1000000)), // $1M threshold
                severity: if risk_metrics.liquidity_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.8 { "high" } else { "medium" }.to_string(),
                trend: "stable".to_string(),
                historical_context: Some("Pools with TVL below $1M are considered high-risk".to_string()),
            };
            explanation.add_primary_factor(liquidity_factor);
        }

        // Volatility Risk Factor
        if risk_metrics.volatility_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.6 {
            let volatility_factor = RiskFactor {
                factor_type: "volatility_risk".to_string(),
                name: "Price Volatility".to_string(),
                score: risk_metrics.volatility_score.clone(),
                weight: BigDecimal::from(20) / BigDecimal::from(100), // 0.2
                contribution: risk_metrics.volatility_score.clone() * (BigDecimal::from(20) / BigDecimal::from(100)),
                explanation: "High price volatility increases the risk of impermanent loss and makes position management more challenging. Consider reducing exposure during volatile periods.".to_string(),
                current_value: Some(risk_metrics.volatility_score.clone()),
                threshold_value: Some(BigDecimal::from(15) / BigDecimal::from(100)), // 0.15
                severity: if risk_metrics.volatility_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.8 { "high" } else { "medium" }.to_string(),
                trend: "increasing".to_string(),
                historical_context: Some("Volatility above 60% indicates unstable market conditions".to_string()),
            };
            explanation.add_primary_factor(volatility_factor);
        }

        Ok(())
    }

    /// Generate secondary risk factors
    async fn generate_secondary_factors(
        &self,
        explanation: &mut RiskExplanation,
        risk_metrics: &RiskMetrics,
        _position: &Position,
    ) -> Result<(), AppError> {
        // Protocol Risk
        if risk_metrics.protocol_risk_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.3 {
            let protocol_factor = RiskFactor {
                factor_type: "protocol_risk".to_string(),
                name: "Protocol Risk".to_string(),
                score: risk_metrics.protocol_risk_score.clone(),
                weight: BigDecimal::from(10) / BigDecimal::from(100), // 0.1
                contribution: risk_metrics.protocol_risk_score.clone() * (BigDecimal::from(10) / BigDecimal::from(100)),
                explanation: "Protocol has some risk factors including audit status, governance, and historical security incidents.".to_string(),
                current_value: Some(risk_metrics.protocol_risk_score.clone()),
                threshold_value: Some(BigDecimal::from_f64(0.5).unwrap_or_else(|| BigDecimal::from(50) / BigDecimal::from(100))),
                severity: "medium".to_string(),
                trend: "stable".to_string(),
                historical_context: None,
            };
            explanation.add_secondary_factor(protocol_factor);
        }

        // MEV Risk
        if risk_metrics.mev_risk_score.to_string().parse::<f64>().unwrap_or(0.0) > 0.4 {
            let mev_factor = RiskFactor {
                factor_type: "mev_risk".to_string(),
                name: "MEV Risk".to_string(),
                score: risk_metrics.mev_risk_score.clone(),
                weight: BigDecimal::from_f64(0.1).unwrap_or_else(|| BigDecimal::from(10) / BigDecimal::from(100)),
                contribution: risk_metrics.mev_risk_score.clone() * BigDecimal::from_f64(0.1).unwrap_or_else(|| BigDecimal::from(10) / BigDecimal::from(100)),
                explanation: "Position may be vulnerable to MEV attacks including sandwich attacks and frontrunning.".to_string(),
                current_value: Some(risk_metrics.mev_risk_score.clone()),
                threshold_value: Some(BigDecimal::from_f64(0.3).unwrap_or_else(|| BigDecimal::from(30) / BigDecimal::from(100))),
                severity: "medium".to_string(),
                trend: "stable".to_string(),
                historical_context: None,
            };
            explanation.add_secondary_factor(mev_factor);
        }

        Ok(())
    }

    /// Generate actionable recommendations
    async fn generate_recommendations(
        &self,
        explanation: &mut RiskExplanation,
        risk_metrics: &RiskMetrics,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<(), AppError> {
        // Critical IL recommendation
        if risk_metrics.impermanent_loss.to_string().parse::<f64>().unwrap_or(0.0) > 15.0 {
            let il_recommendation = RiskRecommendation {
                priority: "immediate".to_string(),
                category: "exit".to_string(),
                title: "Consider Exiting Position".to_string(),
                description: format!(
                    "Your position is experiencing {}% impermanent loss. Consider exiting to prevent further losses.",
                    risk_metrics.impermanent_loss
                ),
                expected_impact: "Stop further IL accumulation, realize current losses".to_string(),
                implementation_cost: Some("Gas fees for withdrawal".to_string()),
                time_sensitivity: "immediate".to_string(),
                resources: vec![
                    "https://uniswap.org/".to_string(),
                    "Position management guide".to_string(),
                ],
            };
            explanation.add_recommendation(il_recommendation);
        }

        // Rebalancing recommendation
        let score_str = risk_metrics.overall_risk_score.to_string();
        let score_f64 = score_str.parse::<f64>().unwrap_or(0.0);
        if score_f64 > 0.7 {
            let rebalance_recommendation = RiskRecommendation {
                priority: "high".to_string(),
                category: "rebalance".to_string(),
                title: "Rebalance Position Range".to_string(),
                description: "Consider adjusting your price range to better capture fees and reduce IL risk.".to_string(),
                expected_impact: "Improved fee capture, reduced impermanent loss risk".to_string(),
                implementation_cost: Some("Gas fees for position adjustment".to_string()),
                time_sensitivity: "within_day".to_string(),
                resources: vec!["Uniswap V3 range management guide".to_string()],
            };
            explanation.add_recommendation(rebalance_recommendation);
        }

        // Monitoring recommendation
        let monitor_recommendation = RiskRecommendation {
            priority: "medium".to_string(),
            category: "monitor".to_string(),
            title: "Set Up Automated Alerts".to_string(),
            description: "Configure alerts for IL thresholds, price movements, and liquidity changes.".to_string(),
            expected_impact: "Early warning system for risk management".to_string(),
            implementation_cost: None,
            time_sensitivity: "flexible".to_string(),
            resources: vec!["Alert configuration guide".to_string()],
        };
        explanation.add_recommendation(monitor_recommendation);

        Ok(())
    }

    /// Generate market context
    async fn generate_market_context(
        &self,
        explanation: &mut RiskExplanation,
        pool_state: &PoolState,
    ) -> Result<(), AppError> {
        explanation.market_context = MarketContext {
            sentiment: self.determine_market_sentiment(pool_state),
            volatility_level: self.determine_volatility_level(pool_state),
            market_events: vec!["DeFi market conditions".to_string()],
            correlation_context: Some("Correlated with broader crypto market".to_string()),
            defi_context: Some(format!("Pool TVL: ${}", pool_state.tvl_usd.as_ref().map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()))),
        };
        Ok(())
    }

    /// Generate position context
    async fn generate_position_context(
        &self,
        explanation: &mut RiskExplanation,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<(), AppError> {
        explanation.position_context = PositionContext {
            position_id: position.id,
            position_type: "uniswap_v3".to_string(),
            pool_info: PoolInfo {
                pool_address: position.pool_address.clone(),
                token_pair: format!("{}/{}", "TOKEN0", "TOKEN1"), // TODO: Fetch actual token symbols
                fee_tier: format!("{}%", position.fee_tier),
                tvl_usd: pool_state.tvl_usd.clone().unwrap_or_else(|| BigDecimal::from(0)),
                pool_age_days: 30, // Placeholder
                liquidity_concentration: "medium".to_string(),
            },
            size_context: SizeContext {
                position_value_usd: BigDecimal::from(0), // TODO: Calculate current position value
                portfolio_percentage: None,
                size_category: "medium".to_string(), // TODO: Determine size category
                risk_capacity: "medium".to_string(),
            },
            time_context: TimeContext {
                position_age_hours: (chrono::Utc::now() - position.created_at).num_hours() as i32,
                optimal_holding_period: Some("1-4 weeks for active management".to_string()),
                time_risk_factors: vec!["Short-term volatility".to_string()],
            },
            performance_context: PerformanceContext {
                current_pnl_usd: BigDecimal::from(0), // TODO: Calculate current PnL
                current_pnl_pct: BigDecimal::from(0), // TODO: Calculate current PnL percentage
                impermanent_loss_pct: BigDecimal::from(0), // TODO: Use actual IL calculation
                fees_earned_usd: BigDecimal::from(0), // Placeholder
                vs_holding_performance: BigDecimal::from(0), // Placeholder
                performance_range: PerformanceRange::default(),
            },
        };
        Ok(())
    }

    /// Generate summary and key insights
    async fn generate_summary_and_insights(
        &self,
        explanation: &mut RiskExplanation,
        risk_metrics: &RiskMetrics,
        position: &Position,
    ) -> Result<(), AppError> {
        let risk_level = &explanation.risk_level;
        let il_pct = &risk_metrics.impermanent_loss;
        
        explanation.set_summary(format!(
            "Your {}/{} position has {} risk (score: {:.2}) primarily due to {}% impermanent loss. {}",
            "TOKEN0".to_string(), // TODO: Fetch actual token symbols
            "TOKEN1".to_string(), // TODO: Fetch actual token symbols
            risk_level,
            risk_metrics.overall_risk_score,
            il_pct,
            if il_pct.to_string().parse::<f64>().unwrap_or(0.0) > 15.0 {
                "Immediate action recommended."
            } else if il_pct.to_string().parse::<f64>().unwrap_or(0.0) > 10.0 {
                "Monitor closely and consider adjustments."
            } else {
                "Position is within acceptable risk parameters."
            }
        ));

        explanation.add_key_insight("Impermanent loss is the primary risk factor for this position".to_string());
        explanation.add_key_insight("Price divergence between tokens is the main driver of IL".to_string());
        explanation.add_key_insight("Active management can help optimize risk-adjusted returns".to_string());

        Ok(())
    }

    /// Helper methods
    fn determine_il_trend(&self, _il_pct: &BigDecimal) -> String {
        "increasing".to_string() // Simplified
    }

    fn determine_market_sentiment(&self, _pool_state: &PoolState) -> String {
        "neutral".to_string() // Simplified
    }

    fn determine_volatility_level(&self, _pool_state: &PoolState) -> String {
        "medium".to_string() // Simplified
    }

    fn determine_size_category(&self, value: &BigDecimal) -> String {
        let value_f64 = value.to_string().parse::<f64>().unwrap_or(0.0);
        match value_f64 {
            v if v < 1000.0 => "small".to_string(),
            v if v < 10000.0 => "medium".to_string(),
            v if v < 100000.0 => "large".to_string(),
            _ => "whale".to_string(),
        }
    }

    fn calculate_confidence_level(&self, risk_metrics: &RiskMetrics, _position: &Position) -> f64 {
        // Simplified confidence calculation
        let score_str = risk_metrics.overall_risk_score.to_string();
        let score_f64 = score_str.parse::<f64>().unwrap_or(0.0);
        if score_f64 > 0.8 {
            0.9 // High confidence in high-risk scenarios
        } else if score_f64 > 0.5 {
            0.8 // Medium confidence
        } else {
            0.7 // Lower confidence in low-risk scenarios
        }
    }
}
