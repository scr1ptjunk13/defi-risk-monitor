// Risk Orchestrator - Routes positions to appropriate protocol-specific risk calculators
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::models::Position;
use crate::risk::{
    RiskError, 
    ProtocolRiskMetrics, 
    PortfolioRiskMetrics, 
    ProtocolRiskCalculator,
    RiskAssessmentSummary
};
use bigdecimal::BigDecimal;
use num_traits::{Zero, FromPrimitive, ToPrimitive, One};

/// Main orchestrator that routes positions to appropriate risk calculators
pub struct RiskOrchestrator {
    calculators: Arc<RwLock<HashMap<String, Box<dyn ProtocolRiskCalculator>>>>,
    config: RiskOrchestratorConfig,
}

/// Configuration for the risk orchestrator
#[derive(Debug, Clone)]
pub struct RiskOrchestratorConfig {
    pub enable_cross_protocol_analysis: bool,
    pub concentration_risk_threshold: f64,
    pub correlation_analysis_enabled: bool,
    pub max_concurrent_calculations: usize,
    pub cache_duration_minutes: u64,
}

impl Default for RiskOrchestratorConfig {
    fn default() -> Self {
        Self {
            enable_cross_protocol_analysis: true,
            concentration_risk_threshold: 0.3, // 30% concentration threshold
            correlation_analysis_enabled: true,
            max_concurrent_calculations: 10,
            cache_duration_minutes: 5,
        }
    }
}

impl RiskOrchestrator {
    /// Create a new risk orchestrator
    pub fn new() -> Self {
        Self {
            calculators: Arc::new(RwLock::new(HashMap::new())),
            config: RiskOrchestratorConfig::default(),
        }
    }
    
    /// Create with custom configuration
    pub fn with_config(config: RiskOrchestratorConfig) -> Self {
        Self {
            calculators: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Register a protocol-specific risk calculator
    pub async fn register_calculator(&self, calculator: Box<dyn ProtocolRiskCalculator>) {
        let protocol_name = calculator.protocol_name().to_string();
        let mut calculators = self.calculators.write().await;
        
        info!(
            protocol = %protocol_name,
            version = calculator.version(),
            "Registering protocol risk calculator"
        );
        
        calculators.insert(protocol_name, calculator);
    }
    
    /// Get list of supported protocols
    pub async fn get_supported_protocols(&self) -> Vec<String> {
        let calculators = self.calculators.read().await;
        calculators.keys().cloned().collect()
    }
    
    /// Check if a protocol is supported
    pub async fn is_protocol_supported(&self, protocol: &str) -> bool {
        let calculators = self.calculators.read().await;
        calculators.contains_key(&protocol.to_lowercase())
    }
    
    /// Calculate risk for a single protocol's positions
    pub async fn calculate_protocol_risk(
        &self,
        protocol: &str,
        positions: &[Position],
    ) -> Result<ProtocolRiskMetrics, RiskError> {
        let calculators = self.calculators.read().await;
        let protocol_key = protocol.to_lowercase();
        
        match calculators.get(&protocol_key) {
            Some(calculator) => {
                info!(
                    protocol = %protocol,
                    position_count = positions.len(),
                    "Calculating protocol-specific risk"
                );
                
                // Validate positions belong to this protocol
                let valid_positions: Vec<Position> = positions
                    .iter()
                    .filter(|pos| calculator.can_handle_position(pos))
                    .cloned()
                    .collect();
                
                if valid_positions.is_empty() {
                    warn!(
                        protocol = %protocol,
                        total_positions = positions.len(),
                        "No valid positions found for protocol"
                    );
                    return Err(RiskError::InvalidPosition {
                        message: format!("No valid positions found for protocol: {}", protocol)
                    });
                }
                
                if valid_positions.len() != positions.len() {
                    warn!(
                        protocol = %protocol,
                        valid_positions = valid_positions.len(),
                        total_positions = positions.len(),
                        "Some positions filtered out during validation"
                    );
                }
                
                calculator.calculate_risk(&valid_positions).await
            }
            None => {
                error!(
                    protocol = %protocol,
                    supported_protocols = ?calculators.keys().collect::<Vec<_>>(),
                    "Protocol risk calculator not found"
                );
                Err(RiskError::CalculatorNotFound {
                    protocol: protocol.to_string(),
                })
            }
        }
    }
    
    /// Calculate comprehensive portfolio risk across all protocols
    pub async fn calculate_portfolio_risk(
        &self,
        positions: &[Position],
    ) -> Result<PortfolioRiskMetrics, RiskError> {
        info!(
            total_positions = positions.len(),
            "Starting comprehensive portfolio risk calculation"
        );
        
        // Group positions by protocol
        let positions_by_protocol = self.group_positions_by_protocol(positions);
        
        info!(
            protocols_found = positions_by_protocol.len(),
            protocols = ?positions_by_protocol.keys().collect::<Vec<_>>(),
            "Grouped positions by protocol"
        );
        
        let mut portfolio_metrics = PortfolioRiskMetrics::new();
        let mut total_value = BigDecimal::zero();
        
        // Calculate risk for each protocol
        for (protocol, protocol_positions) in positions_by_protocol {
            match self.calculate_protocol_risk(&protocol, &protocol_positions).await {
                Ok(risk_metrics) => {
                    info!(
                        protocol = %protocol,
                        risk_score = %risk_metrics.overall_risk_score(),
                        position_count = protocol_positions.len(),
                        "Successfully calculated protocol risk"
                    );
                    
                    // Add to total value
                    // Calculate protocol value from token amounts (simplified calculation)
                    let protocol_value: BigDecimal = protocol_positions
                        .iter()
                        .map(|p| &p.token0_amount + &p.token1_amount) // Simplified: sum token amounts
                        .sum();
                    total_value += protocol_value;
                    
                    portfolio_metrics.protocol_risks.insert(protocol, risk_metrics);
                }
                Err(e) => {
                    error!(
                        protocol = %protocol,
                        error = %e,
                        position_count = protocol_positions.len(),
                        "Failed to calculate protocol risk"
                    );
                    
                    // Continue with other protocols, but log the error
                    // In production, you might want to use a fallback generic calculator
                }
            }
        }
        
        portfolio_metrics.total_value_usd = total_value;
        
        // Calculate cross-protocol risks if enabled
        if self.config.enable_cross_protocol_analysis {
            portfolio_metrics.concentration_risk = self.calculate_concentration_risk(positions).await?;
            
            if self.config.correlation_analysis_enabled {
                portfolio_metrics.cross_protocol_correlation_risk = 
                    self.calculate_correlation_risk(positions).await?;
            }
        }
        
        // Calculate overall portfolio risk
        portfolio_metrics.overall_portfolio_risk = self.calculate_overall_portfolio_risk(&portfolio_metrics).await?;
        
        // Generate recommendations
        portfolio_metrics.recommendations = self.generate_recommendations(&portfolio_metrics).await;
        
        // Identify top risk factors
        portfolio_metrics.top_risk_factors = self.identify_top_risk_factors(&portfolio_metrics).await;
        
        info!(
            overall_risk = %portfolio_metrics.overall_portfolio_risk,
            protocols_analyzed = portfolio_metrics.protocol_risks.len(),
            total_value_usd = %portfolio_metrics.total_value_usd,
            recommendations_count = portfolio_metrics.recommendations.len(),
            "Completed portfolio risk calculation"
        );
        
        Ok(portfolio_metrics)
    }
    
    /// Group positions by protocol
    fn group_positions_by_protocol(&self, positions: &[Position]) -> HashMap<String, Vec<Position>> {
        let mut grouped = HashMap::new();
        
        for position in positions {
            let protocol = position.protocol.to_lowercase();
            grouped.entry(protocol).or_insert_with(Vec::new).push(position.clone());
        }
        
        grouped
    }
    
    /// Calculate concentration risk (how concentrated the portfolio is)
    async fn calculate_concentration_risk(&self, positions: &[Position]) -> Result<BigDecimal, RiskError> {
        if positions.is_empty() {
            return Ok(BigDecimal::zero());
        }
        
        // Calculate total value from token amounts (simplified)
        let total_value: f64 = positions.iter()
            .map(|p| (&p.token0_amount + &p.token1_amount).to_f64().unwrap_or(0.0))
            .sum();
        
        if total_value == 0.0 {
            return Ok(BigDecimal::zero());
        }
        
        // Calculate Herfindahl-Hirschman Index (HHI) for concentration
        let mut protocol_values = HashMap::new();
        
        for position in positions {
            let protocol = position.protocol.to_lowercase();
            let position_value = (&position.token0_amount + &position.token1_amount).to_f64().unwrap_or(0.0);
            *protocol_values.entry(protocol).or_insert(0.0) += position_value;
        }
        
        let hhi: f64 = protocol_values
            .values()
            .map(|value| {
                let share = value / total_value;
                share * share
            })
            .sum();
        
        // Convert HHI to risk score (0-100)
        // HHI ranges from 1/n to 1, where n is number of protocols
        // Higher HHI = higher concentration = higher risk
        let concentration_risk = (hhi * 100.0).min(100.0);
        
        Ok(BigDecimal::from_f64(concentration_risk).unwrap_or(BigDecimal::zero()))
    }
    
    /// Calculate cross-protocol correlation risk
    async fn calculate_correlation_risk(&self, _positions: &[Position]) -> Result<BigDecimal, RiskError> {
        // Simplified correlation risk calculation
        // In production, this would analyze correlations between different protocols
        // For now, return a moderate correlation risk
        Ok(BigDecimal::from_f64(25.0).unwrap_or(BigDecimal::zero()))
    }
    
    /// Calculate overall portfolio risk score
    async fn calculate_overall_portfolio_risk(
        &self,
        portfolio_metrics: &PortfolioRiskMetrics,
    ) -> Result<BigDecimal, RiskError> {
        if portfolio_metrics.protocol_risks.is_empty() {
            return Ok(BigDecimal::zero());
        }
        
        let total_value = &portfolio_metrics.total_value_usd;
        
        if total_value.is_zero() {
            return Ok(BigDecimal::zero());
        }
        
        // Calculate weighted average risk score
        let mut weighted_risk_sum = BigDecimal::zero();
        let mut total_weight = BigDecimal::zero();
        
        for (protocol, risk_metrics) in &portfolio_metrics.protocol_risks {
            // Get protocol value (simplified - in production you'd track this properly)
            let protocol_weight = BigDecimal::from_f64(1.0).unwrap_or(BigDecimal::one()); // Equal weight for now
            let risk_score = risk_metrics.overall_risk_score();
            
            weighted_risk_sum += &risk_score * &protocol_weight;
            total_weight += protocol_weight;
        }
        
        let base_risk = if !total_weight.is_zero() {
            weighted_risk_sum / total_weight
        } else {
            BigDecimal::zero()
        };
        
        // Add concentration risk and correlation risk
        let concentration_penalty = &portfolio_metrics.concentration_risk * BigDecimal::from_f64(0.3).unwrap_or(BigDecimal::zero());
        let correlation_penalty = &portfolio_metrics.cross_protocol_correlation_risk * BigDecimal::from_f64(0.2).unwrap_or(BigDecimal::zero());
        
        let overall_risk = base_risk + concentration_penalty + correlation_penalty;
        
        // Cap at 100
        Ok(overall_risk.min(BigDecimal::from(100)))
    }
    
    /// Generate risk reduction recommendations
    async fn generate_recommendations(&self, portfolio_metrics: &PortfolioRiskMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Check concentration risk
        let concentration_score = portfolio_metrics.concentration_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if concentration_score > self.config.concentration_risk_threshold * 100.0 {
            recommendations.push("Consider diversifying across more protocols to reduce concentration risk".to_string());
        }
        
        // Check individual protocol risks
        for (protocol, risk_metrics) in &portfolio_metrics.protocol_risks {
            let risk_score = risk_metrics.overall_risk_score().to_string().parse::<f64>().unwrap_or(0.0);
            
            if risk_score > 70.0 {
                recommendations.push(format!("High risk detected in {} protocol - consider reducing exposure", protocol));
            }
        }
        
        // Add general recommendations
        if portfolio_metrics.protocol_risks.len() < 3 {
            recommendations.push("Consider diversifying across more DeFi protocols".to_string());
        }
        
        recommendations
    }
    
    /// Identify top risk factors across the portfolio
    async fn identify_top_risk_factors(&self, portfolio_metrics: &PortfolioRiskMetrics) -> Vec<String> {
        let mut risk_factors = Vec::new();
        
        // Analyze concentration
        let concentration_score = portfolio_metrics.concentration_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if concentration_score > 30.0 {
            risk_factors.push("High concentration risk".to_string());
        }
        
        // Analyze protocol-specific risks
        for (protocol, risk_metrics) in &portfolio_metrics.protocol_risks {
            let risk_score = risk_metrics.overall_risk_score().to_string().parse::<f64>().unwrap_or(0.0);
            
            if risk_score > 60.0 {
                risk_factors.push(format!("{} protocol risk", protocol));
            }
        }
        
        // Add correlation risk if significant
        let correlation_score = portfolio_metrics.cross_protocol_correlation_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if correlation_score > 40.0 {
            risk_factors.push("Cross-protocol correlation risk".to_string());
        }
        
        risk_factors
    }
    
    /// Get orchestrator statistics
    pub async fn get_statistics(&self) -> RiskOrchestratorStats {
        let calculators = self.calculators.read().await;
        
        RiskOrchestratorStats {
            registered_calculators: calculators.len(),
            supported_protocols: calculators.keys().cloned().collect(),
            configuration: self.config.clone(),
        }
    }
}

/// Statistics about the risk orchestrator
#[derive(Debug, Clone, serde::Serialize)]
pub struct RiskOrchestratorStats {
    pub registered_calculators: usize,
    pub supported_protocols: Vec<String>,
    pub configuration: RiskOrchestratorConfig,
}

impl serde::Serialize for RiskOrchestratorConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RiskOrchestratorConfig", 5)?;
        state.serialize_field("enable_cross_protocol_analysis", &self.enable_cross_protocol_analysis)?;
        state.serialize_field("concentration_risk_threshold", &self.concentration_risk_threshold)?;
        state.serialize_field("correlation_analysis_enabled", &self.correlation_analysis_enabled)?;
        state.serialize_field("max_concurrent_calculations", &self.max_concurrent_calculations)?;
        state.serialize_field("cache_duration_minutes", &self.cache_duration_minutes)?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::risk::metrics::*;
    
    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orchestrator = RiskOrchestrator::new();
        let stats = orchestrator.get_statistics().await;
        
        assert_eq!(stats.registered_calculators, 0);
        assert!(stats.supported_protocols.is_empty());
    }
    
    #[tokio::test]
    async fn test_concentration_risk_calculation() {
        let orchestrator = RiskOrchestrator::new();
        
        // Create test positions with high concentration in one protocol
        let positions = vec![
            Position {
                protocol: "lido".to_string(),
                balance_usd: 80.0,
                ..Default::default()
            },
            Position {
                protocol: "uniswap_v3".to_string(),
                balance_usd: 20.0,
                ..Default::default()
            },
        ];
        
        let concentration_risk = orchestrator.calculate_concentration_risk(&positions).await.unwrap();
        let risk_score = concentration_risk.to_string().parse::<f64>().unwrap();
        
        // Should detect high concentration (80% in one protocol)
        assert!(risk_score > 50.0);
    }
    
    #[tokio::test]
    async fn test_empty_positions() {
        let orchestrator = RiskOrchestrator::new();
        let positions = vec![];
        
        let concentration_risk = orchestrator.calculate_concentration_risk(&positions).await.unwrap();
        assert!(concentration_risk.is_zero());
    }
}
