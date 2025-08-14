// Protocol Risk Calculator Traits and Core Interfaces
use async_trait::async_trait;
use crate::models::Position;
use crate::risk::{RiskError, ProtocolRiskMetrics};

/// Core trait that all protocol-specific risk calculators must implement
#[async_trait]
pub trait ProtocolRiskCalculator: Send + Sync {
    /// Calculate risk metrics for positions from this protocol
    async fn calculate_risk(&self, positions: &[Position]) -> Result<ProtocolRiskMetrics, RiskError>;
    
    /// Get the protocol name this calculator handles
    fn protocol_name(&self) -> &'static str;
    
    /// Get supported position types for this protocol
    fn supported_position_types(&self) -> Vec<&'static str>;
    
    /// Validate that a position belongs to this protocol and is valid
    async fn validate_position(&self, position: &Position) -> Result<bool, RiskError>;
    
    /// Get the risk factors this calculator considers
    fn risk_factors(&self) -> Vec<&'static str>;
    
    /// Get the version of this risk calculator (for tracking changes)
    fn version(&self) -> &'static str {
        "1.0.0"
    }
    
    /// Check if this calculator can handle the given position
    fn can_handle_position(&self, position: &Position) -> bool {
        position.protocol.to_lowercase() == self.protocol_name().to_lowercase()
    }
    
    /// Get configuration parameters for this calculator
    fn get_config(&self) -> serde_json::Value {
        serde_json::json!({
            "protocol": self.protocol_name(),
            "version": self.version(),
            "supported_types": self.supported_position_types(),
            "risk_factors": self.risk_factors()
        })
    }
}

/// Trait for calculators that need real-time data updates
#[async_trait]
pub trait RealTimeRiskCalculator: ProtocolRiskCalculator {
    /// Update real-time data (prices, protocol metrics, etc.)
    async fn update_real_time_data(&self) -> Result<(), RiskError>;
    
    /// Get the last update timestamp
    fn last_updated(&self) -> Option<chrono::DateTime<chrono::Utc>>;
    
    /// Check if real-time data is stale and needs updating
    fn is_data_stale(&self) -> bool {
        match self.last_updated() {
            Some(last_update) => {
                let now = chrono::Utc::now();
                let stale_threshold = chrono::Duration::minutes(5); // 5 minutes
                now - last_update > stale_threshold
            }
            None => true, // No data is considered stale
        }
    }
}

/// Trait for calculators that support historical risk analysis
#[async_trait]
pub trait HistoricalRiskCalculator: ProtocolRiskCalculator {
    /// Calculate historical risk metrics for a time period
    async fn calculate_historical_risk(
        &self,
        positions: &[Position],
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<ProtocolRiskMetrics>, RiskError>;
    
    /// Get risk trend analysis
    async fn get_risk_trends(
        &self,
        positions: &[Position],
        days: u32,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, ProtocolRiskMetrics)>, RiskError>;
}

/// Trait for calculators that can explain their risk calculations
pub trait ExplainableRiskCalculator: ProtocolRiskCalculator {
    /// Get detailed explanation of how risk score was calculated
    fn explain_risk_calculation(&self, metrics: &ProtocolRiskMetrics) -> RiskExplanation;
    
    /// Get risk factor contributions (which factors contributed most to the risk score)
    fn get_risk_factor_contributions(&self, metrics: &ProtocolRiskMetrics) -> Vec<RiskFactorContribution>;
    
    /// Get recommendations to reduce risk
    fn get_risk_reduction_recommendations(&self, metrics: &ProtocolRiskMetrics) -> Vec<String>;
}

/// Risk explanation structure for explainable risk calculators
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskExplanation {
    pub overall_risk_score: f64,
    pub risk_level: String, // "Low", "Medium", "High", "Critical"
    pub primary_risk_factors: Vec<String>,
    pub explanation: String,
    pub methodology: String,
    pub confidence_score: f64, // 0.0 to 1.0
    pub data_quality: String, // "High", "Medium", "Low"
}

/// Risk factor contribution for detailed analysis
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskFactorContribution {
    pub factor_name: String,
    pub contribution_score: f64, // 0.0 to 100.0 (percentage)
    pub impact_level: String, // "Low", "Medium", "High", "Critical"
    pub description: String,
    pub current_value: Option<f64>,
    pub threshold_value: Option<f64>,
}

/// Trait for calculators that support risk simulation
#[async_trait]
pub trait RiskSimulationCalculator: ProtocolRiskCalculator {
    /// Simulate risk under different market conditions
    async fn simulate_risk_scenarios(
        &self,
        positions: &[Position],
        scenarios: &[MarketScenario],
    ) -> Result<Vec<ScenarioRiskResult>, RiskError>;
    
    /// Calculate Value at Risk (VaR) for different confidence levels
    async fn calculate_value_at_risk(
        &self,
        positions: &[Position],
        confidence_levels: &[f64], // e.g., [0.95, 0.99]
        time_horizon_days: u32,
    ) -> Result<Vec<VaRResult>, RiskError>;
}

/// Market scenario for risk simulation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MarketScenario {
    pub name: String,
    pub description: String,
    pub price_changes: std::collections::HashMap<String, f64>, // token -> percentage change
    pub volatility_multiplier: f64,
    pub liquidity_impact: f64, // -1.0 to 1.0
    pub correlation_changes: Option<std::collections::HashMap<String, f64>>,
}

/// Result of a risk scenario simulation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioRiskResult {
    pub scenario_name: String,
    pub risk_metrics: ProtocolRiskMetrics,
    pub estimated_loss_usd: f64,
    pub probability: Option<f64>, // If known
}

/// Value at Risk calculation result
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VaRResult {
    pub confidence_level: f64, // e.g., 0.95 for 95%
    pub var_usd: f64, // Maximum expected loss at this confidence level
    pub time_horizon_days: u32,
    pub methodology: String, // "Historical", "Parametric", "Monte Carlo"
}
