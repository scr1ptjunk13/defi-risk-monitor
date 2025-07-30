use serde::{Deserialize, Serialize};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Risk factor with detailed explanation and impact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    /// Type of risk factor (e.g., "impermanent_loss", "liquidity_risk", "volatility")
    pub factor_type: String,
    /// Human-readable name of the risk factor
    pub name: String,
    /// Risk score for this specific factor (0.0 to 1.0)
    pub score: BigDecimal,
    /// Weight of this factor in overall risk calculation
    pub weight: BigDecimal,
    /// Contribution to overall risk (score * weight)
    pub contribution: BigDecimal,
    /// Detailed explanation of this risk factor
    pub explanation: String,
    /// Current value that triggered this risk
    pub current_value: Option<BigDecimal>,
    /// Threshold value for this risk factor
    pub threshold_value: Option<BigDecimal>,
    /// Severity level: "low", "medium", "high", "critical"
    pub severity: String,
    /// Trend direction: "increasing", "decreasing", "stable"
    pub trend: String,
    /// Historical context or comparison
    pub historical_context: Option<String>,
}

/// Actionable recommendation for risk mitigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskRecommendation {
    /// Priority level: "immediate", "high", "medium", "low"
    pub priority: String,
    /// Category: "exit", "hedge", "rebalance", "monitor", "diversify"
    pub category: String,
    /// Short title of the recommendation
    pub title: String,
    /// Detailed recommendation text
    pub description: String,
    /// Expected impact if recommendation is followed
    pub expected_impact: String,
    /// Estimated cost or effort to implement
    pub implementation_cost: Option<String>,
    /// Time sensitivity: "immediate", "within_hour", "within_day", "flexible"
    pub time_sensitivity: String,
    /// Links to relevant documentation or tools
    pub resources: Vec<String>,
}

/// Market condition context for risk explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketContext {
    /// Overall market sentiment: "bullish", "bearish", "neutral", "volatile"
    pub sentiment: String,
    /// Market volatility level: "low", "medium", "high", "extreme"
    pub volatility_level: String,
    /// Key market events affecting risk
    pub market_events: Vec<String>,
    /// Correlation with major assets (BTC, ETH)
    pub correlation_context: Option<String>,
    /// DeFi-specific context (TVL changes, protocol events)
    pub defi_context: Option<String>,
}

/// Comprehensive risk explanation with actionable insights
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskExplanation {
    /// Overall risk score (0.0 to 1.0)
    pub risk_score: BigDecimal,
    /// Risk level: "low", "medium", "high", "critical"
    pub risk_level: String,
    /// Primary risk factors contributing to the score
    pub primary_factors: Vec<RiskFactor>,
    /// Secondary factors with lower impact
    pub secondary_factors: Vec<RiskFactor>,
    /// Actionable recommendations for risk mitigation
    pub recommendations: Vec<RiskRecommendation>,
    /// Confidence level in the risk assessment (0.0 to 1.0)
    pub confidence_level: f64,
    /// Plain English summary of the risk situation
    pub summary: String,
    /// Key insights and takeaways
    pub key_insights: Vec<String>,
    /// Market context affecting the risk
    pub market_context: MarketContext,
    /// Position-specific context
    pub position_context: PositionContext,
    /// When this explanation was generated
    pub generated_at: DateTime<Utc>,
    /// Unique identifier for this explanation
    pub explanation_id: Uuid,
}

/// Position-specific context for risk explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionContext {
    /// Position ID
    pub position_id: Uuid,
    /// Position type: "uniswap_v3", "uniswap_v2", "balancer", etc.
    pub position_type: String,
    /// Pool information
    pub pool_info: PoolInfo,
    /// Position size and allocation
    pub size_context: SizeContext,
    /// Time-based context (how long position has been open)
    pub time_context: TimeContext,
    /// Performance context (current vs entry)
    pub performance_context: PerformanceContext,
}

/// Pool information for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    /// Pool address
    pub pool_address: String,
    /// Token pair (e.g., "ETH/USDC")
    pub token_pair: String,
    /// Fee tier (e.g., "0.3%")
    pub fee_tier: String,
    /// Current TVL
    pub tvl_usd: BigDecimal,
    /// Pool age and maturity
    pub pool_age_days: i32,
    /// Pool liquidity concentration
    pub liquidity_concentration: String,
}

/// Position size context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeContext {
    /// Position value in USD
    pub position_value_usd: BigDecimal,
    /// Percentage of user's total portfolio
    pub portfolio_percentage: Option<BigDecimal>,
    /// Size category: "small", "medium", "large", "whale"
    pub size_category: String,
    /// Risk capacity based on position size
    pub risk_capacity: String,
}

/// Time-based context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeContext {
    /// How long position has been open
    pub position_age_hours: i32,
    /// Optimal holding period for this strategy
    pub optimal_holding_period: Option<String>,
    /// Time-based risk factors
    pub time_risk_factors: Vec<String>,
}

/// Performance context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceContext {
    /// Current P&L in USD
    pub current_pnl_usd: BigDecimal,
    /// Current P&L percentage
    pub current_pnl_pct: BigDecimal,
    /// Impermanent loss percentage
    pub impermanent_loss_pct: BigDecimal,
    /// Fees earned in USD
    pub fees_earned_usd: BigDecimal,
    /// Performance vs benchmark (e.g., holding tokens)
    pub vs_holding_performance: BigDecimal,
    /// Best and worst performance periods
    pub performance_range: PerformanceRange,
}

/// Performance range information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceRange {
    /// Best P&L achieved
    pub best_pnl_pct: BigDecimal,
    /// Worst P&L experienced
    pub worst_pnl_pct: BigDecimal,
    /// Current drawdown from peak
    pub current_drawdown_pct: BigDecimal,
    /// Maximum drawdown experienced
    pub max_drawdown_pct: BigDecimal,
}

/// Risk explanation request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainRiskRequest {
    /// Position ID to explain
    pub position_id: Uuid,
    /// User address for personalized recommendations
    pub user_address: Option<String>,
    /// Detail level: "summary", "detailed", "comprehensive"
    pub detail_level: String,
    /// Include market context
    pub include_market_context: bool,
    /// Include historical analysis
    pub include_historical_analysis: bool,
    /// Language for explanations (default: "en")
    pub language: Option<String>,
}

/// Risk explanation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainRiskResponse {
    /// The comprehensive risk explanation
    pub explanation: RiskExplanation,
    /// Additional metadata
    pub metadata: ExplanationMetadata,
}

/// Metadata for risk explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplanationMetadata {
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Data sources used
    pub data_sources: Vec<String>,
    /// Model version used for explanation
    pub model_version: String,
    /// Explanation quality score
    pub quality_score: f64,
    /// Whether explanation used cached data
    pub used_cached_data: bool,
}

impl RiskExplanation {
    /// Create a new risk explanation
    pub fn new(risk_score: BigDecimal, position_id: Uuid) -> Self {
        let risk_level = Self::determine_risk_level(&risk_score);
        Self {
            risk_score,
            risk_level,
            primary_factors: Vec::new(),
            secondary_factors: Vec::new(),
            recommendations: Vec::new(),
            confidence_level: 0.0,
            summary: String::new(),
            key_insights: Vec::new(),
            market_context: MarketContext::default(),
            position_context: PositionContext::new(position_id),
            generated_at: Utc::now(),
            explanation_id: Uuid::new_v4(),
        }
    }

    /// Determine risk level from score
    fn determine_risk_level(score: &BigDecimal) -> String {
        let score_f64 = score.to_string().parse::<f64>().unwrap_or(0.0);
        match score_f64 {
            s if s < 0.25 => "low".to_string(),
            s if s < 0.5 => "medium".to_string(),
            s if s < 0.75 => "high".to_string(),
            _ => "critical".to_string(),
        }
    }

    /// Add a primary risk factor
    pub fn add_primary_factor(&mut self, factor: RiskFactor) {
        self.primary_factors.push(factor);
    }

    /// Add a secondary risk factor
    pub fn add_secondary_factor(&mut self, factor: RiskFactor) {
        self.secondary_factors.push(factor);
    }

    /// Add a recommendation
    pub fn add_recommendation(&mut self, recommendation: RiskRecommendation) {
        self.recommendations.push(recommendation);
    }

    /// Set the summary
    pub fn set_summary(&mut self, summary: String) {
        self.summary = summary;
    }

    /// Add a key insight
    pub fn add_key_insight(&mut self, insight: String) {
        self.key_insights.push(insight);
    }

    /// Get the most critical risk factors (top 3)
    pub fn get_critical_factors(&self) -> Vec<&RiskFactor> {
        let mut all_factors: Vec<&RiskFactor> = self.primary_factors.iter()
            .chain(self.secondary_factors.iter())
            .collect();
        
        all_factors.sort_by(|a, b| b.contribution.partial_cmp(&a.contribution).unwrap_or(std::cmp::Ordering::Equal));
        all_factors.into_iter().take(3).collect()
    }

    /// Get immediate action recommendations
    pub fn get_immediate_actions(&self) -> Vec<&RiskRecommendation> {
        self.recommendations.iter()
            .filter(|r| r.priority == "immediate" || r.time_sensitivity == "immediate")
            .collect()
    }
}

impl Default for MarketContext {
    fn default() -> Self {
        Self {
            sentiment: "neutral".to_string(),
            volatility_level: "medium".to_string(),
            market_events: Vec::new(),
            correlation_context: None,
            defi_context: None,
        }
    }
}

impl PositionContext {
    pub fn new(position_id: Uuid) -> Self {
        Self {
            position_id,
            position_type: "unknown".to_string(),
            pool_info: PoolInfo::default(),
            size_context: SizeContext::default(),
            time_context: TimeContext::default(),
            performance_context: PerformanceContext::default(),
        }
    }
}

impl Default for PoolInfo {
    fn default() -> Self {
        Self {
            pool_address: String::new(),
            token_pair: String::new(),
            fee_tier: String::new(),
            tvl_usd: BigDecimal::from(0),
            pool_age_days: 0,
            liquidity_concentration: "unknown".to_string(),
        }
    }
}

impl Default for SizeContext {
    fn default() -> Self {
        Self {
            position_value_usd: BigDecimal::from(0),
            portfolio_percentage: None,
            size_category: "unknown".to_string(),
            risk_capacity: "unknown".to_string(),
        }
    }
}

impl Default for TimeContext {
    fn default() -> Self {
        Self {
            position_age_hours: 0,
            optimal_holding_period: None,
            time_risk_factors: Vec::new(),
        }
    }
}

impl Default for PerformanceContext {
    fn default() -> Self {
        Self {
            current_pnl_usd: BigDecimal::from(0),
            current_pnl_pct: BigDecimal::from(0),
            impermanent_loss_pct: BigDecimal::from(0),
            fees_earned_usd: BigDecimal::from(0),
            vs_holding_performance: BigDecimal::from(0),
            performance_range: PerformanceRange::default(),
        }
    }
}

impl Default for PerformanceRange {
    fn default() -> Self {
        Self {
            best_pnl_pct: BigDecimal::from(0),
            worst_pnl_pct: BigDecimal::from(0),
            current_drawdown_pct: BigDecimal::from(0),
            max_drawdown_pct: BigDecimal::from(0),
        }
    }
}
