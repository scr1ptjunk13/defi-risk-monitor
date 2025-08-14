// Risk Orchestration Service
// Integrates the new modular risk system with existing services

use std::sync::Arc;
use tracing::{info, error, debug};

use crate::models::{Position, RiskAssessment};
use bigdecimal::BigDecimal;
use crate::risk::{
    RiskOrchestrator, 
    RiskOrchestratorConfig,
    LidoRiskCalculator,
    BeefyRiskCalculator,
    GenericRiskCalculator,
    ProtocolRiskMetrics,
    PortfolioRiskMetrics,
    RiskError
};

/// Service that orchestrates risk calculations using the new modular system
pub struct RiskOrchestrationService {
    orchestrator: Arc<RiskOrchestrator>,
}

impl RiskOrchestrationService {
    /// Create a new risk orchestration service
    pub async fn new() -> Result<Self, RiskError> {
        info!("Initializing Risk Orchestration Service");
        
        // Create orchestrator with default config
        let config = RiskOrchestratorConfig {
            enable_cross_protocol_analysis: true,
            concentration_risk_threshold: 0.3,
            correlation_analysis_enabled: true,
            max_concurrent_calculations: 10,
            cache_duration_minutes: 5,
        };
        
        let orchestrator = Arc::new(RiskOrchestrator::with_config(config));
        
        // Register protocol-specific calculators
        Self::register_default_calculators(&orchestrator).await?;
        
        Ok(Self { orchestrator })
    }
    
    /// Register default protocol calculators
    async fn register_default_calculators(orchestrator: &RiskOrchestrator) -> Result<(), RiskError> {
        info!("Registering default protocol risk calculators");
        
        // Register Lido calculator
        let lido_calculator = Box::new(LidoRiskCalculator::new());
        orchestrator.register_calculator(lido_calculator).await;
        
        // Register Beefy calculator
        let beefy_calculator = Box::new(BeefyRiskCalculator::new());
        orchestrator.register_calculator(beefy_calculator).await;
        
        // Register generic calculators for common protocols
        let protocols = vec![
            "uniswap_v3", "uniswap_v2", "aave", "compound", "curve", 
            "yearn", "balancer", "convex", "makerdao", "eigenlayer",
            "rocket_pool", "frax", "synthetix"
        ];
        
        for protocol in &protocols {
            let generic_calculator = Box::new(GenericRiskCalculator::new(protocol.to_string()));
            orchestrator.register_calculator(generic_calculator).await;
        }
        
        info!("Successfully registered {} protocol calculators", protocols.len() + 1);
        Ok(())
    }
    
    /// Calculate risk for a single protocol
    pub async fn calculate_protocol_risk(
        &self,
        protocol: &str,
        positions: &[Position],
    ) -> Result<ProtocolRiskMetrics, RiskError> {
        debug!(
            protocol = %protocol,
            position_count = positions.len(),
            "Calculating protocol risk"
        );
        
        self.orchestrator.calculate_protocol_risk(protocol, positions).await
    }
    
    /// Calculate portfolio-wide risk across all positions
    pub async fn calculate_portfolio_risk(
        &self,
        positions: &[Position],
    ) -> Result<PortfolioRiskMetrics, RiskError> {
        info!(
            position_count = positions.len(),
            "Calculating portfolio risk"
        );
        
        self.orchestrator.calculate_portfolio_risk(positions).await
    }
    
    /// Convert new risk metrics to legacy RiskAssessment format
    pub async fn calculate_legacy_risk_assessment(
        &self,
        positions: &[Position],
        user_address: &str,
    ) -> Result<RiskAssessment, RiskError> {
        info!(
            user_address = %user_address,
            position_count = positions.len(),
            "Calculating legacy risk assessment format"
        );
        
        // Calculate portfolio risk using new system
        let portfolio_risk = self.calculate_portfolio_risk(positions).await?;
        
        // Convert to RiskAssessment format
        let risk_assessment = RiskAssessment {
            id: uuid::Uuid::new_v4(),
            entity_type: crate::models::risk_assessment::RiskEntityType::Portfolio,
            entity_id: user_address.to_string(),
            user_id: None, // Could be populated if we have user ID
            risk_type: crate::models::risk_assessment::RiskType::Market,
            risk_score: BigDecimal::from((portfolio_risk.overall_portfolio_risk.to_string().parse::<f64>().unwrap_or(50.0) / 100.0) as i32),
            severity: Self::determine_risk_severity(portfolio_risk.overall_portfolio_risk.to_string().parse().unwrap_or(50.0)),
            confidence: BigDecimal::from(85i32),
            description: Some(format!("Portfolio risk assessment for {}", user_address)),
            metadata: Some(serde_json::json!({
                "total_value_usd": portfolio_risk.total_value_usd,
                "concentration_risk": portfolio_risk.concentration_risk,
                "liquidity_risk": portfolio_risk.risk_breakdown.liquidity_risk,
                "smart_contract_risk": portfolio_risk.risk_breakdown.smart_contract_risk,
                "market_risk": portfolio_risk.risk_breakdown.market_risk,
                "correlation_risk": portfolio_risk.cross_protocol_correlation_risk
            })),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(24)),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        info!(
            risk_score = %risk_assessment.risk_score,
            severity = ?risk_assessment.severity,
            "Generated risk assessment"
        );
        
        Ok(risk_assessment)
    }
    
    /// Determine risk level from score
    fn determine_risk_level(score: f64) -> String {
        match score {
            s if s >= 80.0 => "Critical".to_string(),
            s if s >= 60.0 => "High".to_string(),
            s if s >= 40.0 => "Medium".to_string(),
            s if s >= 20.0 => "Low".to_string(),
            _ => "Very Low".to_string(),
        }
    }
    
    /// Determine risk severity from score
    fn determine_risk_severity(score: f64) -> crate::models::risk_assessment::RiskSeverity {
        match score {
            s if s >= 80.0 => crate::models::risk_assessment::RiskSeverity::Critical,
            s if s >= 60.0 => crate::models::risk_assessment::RiskSeverity::High,
            s if s >= 40.0 => crate::models::risk_assessment::RiskSeverity::Medium,
            s if s >= 20.0 => crate::models::risk_assessment::RiskSeverity::Low,
            _ => crate::models::risk_assessment::RiskSeverity::Low,
        }
    }
    
    /// Extract specific protocol risk from portfolio metrics
    fn extract_protocol_risk(_portfolio: &PortfolioRiskMetrics, risk_type: &str) -> Option<f64> {
        // This would need more sophisticated logic to extract specific risks
        // from protocol-specific metrics within the portfolio
        match risk_type {
            "impermanent_loss" => Some(25.0), // Default for now
            "slippage" => Some(20.0),
            _ => None,
        }
    }
    
    /// Extract risk factors from portfolio metrics
    fn extract_risk_factors(portfolio: &PortfolioRiskMetrics) -> Vec<String> {
        let mut factors = vec![];
        
        // Add factors based on risk levels
        if portfolio.concentration_risk.to_string().parse::<f64>().unwrap_or(0.0) > 30.0 {
            factors.push("High concentration risk".to_string());
        }
        
        if portfolio.cross_protocol_correlation_risk.to_string().parse::<f64>().unwrap_or(0.0) > 25.0 {
            factors.push("High correlation risk".to_string());
        }
        
        if portfolio.risk_breakdown.liquidity_risk.to_string().parse::<f64>().unwrap_or(0.0) > 35.0 {
            factors.push("Liquidity constraints".to_string());
        }
        
        if portfolio.risk_breakdown.smart_contract_risk.to_string().parse::<f64>().unwrap_or(0.0) > 30.0 {
            factors.push("Smart contract risks".to_string());
        }
        
        factors
    }
    
    /// Generate recommendations from portfolio metrics
    fn generate_recommendations(portfolio: &PortfolioRiskMetrics) -> Vec<String> {
        let mut recommendations = vec![];
        
        let concentration_risk = portfolio.concentration_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if concentration_risk > 40.0 {
            recommendations.push("Consider diversifying across more protocols".to_string());
        }
        
        let correlation_risk = portfolio.cross_protocol_correlation_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if correlation_risk > 30.0 {
            recommendations.push("Reduce correlation by diversifying asset types".to_string());
        }
        
        let liquidity_risk = portfolio.risk_breakdown.liquidity_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if liquidity_risk > 40.0 {
            recommendations.push("Consider more liquid positions for better exit options".to_string());
        }
        
        let overall_risk = portfolio.overall_portfolio_risk.to_string().parse::<f64>().unwrap_or(0.0);
        if overall_risk > 70.0 {
            recommendations.push("Consider reducing position sizes to lower overall risk".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("Portfolio risk profile looks reasonable".to_string());
        }
        
        recommendations
    }
    
    /// Get orchestrator statistics
    pub async fn get_statistics(&self) -> serde_json::Value {
        let stats = self.orchestrator.get_statistics().await;
        serde_json::to_value(stats).unwrap_or_else(|_| serde_json::json!({
            "error": "Failed to serialize statistics"
        }))
    }
    
    /// Health check for the risk orchestration service
    pub async fn health_check(&self) -> Result<(), RiskError> {
        debug!("Performing risk orchestration service health check");
        
        // Test with a simple position
        let test_position = Position {
            id: uuid::Uuid::new_v4(),
            user_address: "0x1234567890123456789012345678901234567890".to_string(),
            protocol: "lido".to_string(),
            pool_address: "0x0000000000000000000000000000000000000000".to_string(),
            token0_address: "0x0000000000000000000000000000000000000000".to_string(),
            token1_address: "0x0000000000000000000000000000000000000000".to_string(),
            token0_amount: BigDecimal::from(1000),
            token1_amount: BigDecimal::from(0),
            liquidity: BigDecimal::from(1000),
            tick_lower: 0,
            tick_upper: 0,
            fee_tier: 0,
            chain_id: 1,
            entry_token0_price_usd: None,
            entry_token1_price_usd: None,
            entry_timestamp: None,
            created_at: None,
            updated_at: None,
        };
        
        // Try to calculate risk
        match self.calculate_protocol_risk("lido", &[test_position]).await {
            Ok(_) => {
                info!("Risk orchestration service health check passed");
                Ok(())
            }
            Err(e) => {
                error!(error = %e, "Risk orchestration service health check failed");
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Position;
    
    fn create_test_position(protocol: &str, balance_usd: f64) -> Position {
        Position {
            protocol: protocol.to_string(),
            position_type: "test".to_string(),
            balance_usd,
            ..Default::default()
        }
    }
    
    #[tokio::test]
    async fn test_service_initialization() {
        let service = RiskOrchestrationService::new().await;
        assert!(service.is_ok());
    }
    
    #[tokio::test]
    async fn test_lido_risk_calculation() {
        let service = RiskOrchestrationService::new().await.unwrap();
        let positions = vec![create_test_position("lido", 10000.0)];
        
        let result = service.calculate_protocol_risk("lido", &positions).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_portfolio_risk_calculation() {
        let service = RiskOrchestrationService::new().await.unwrap();
        let positions = vec![
            create_test_position("lido", 5000.0),
            create_test_position("uniswap_v3", 3000.0),
            create_test_position("aave", 2000.0),
        ];
        
        let result = service.calculate_portfolio_risk(&positions).await;
        assert!(result.is_ok());
        
        if let Ok(portfolio_risk) = result {
            assert!(portfolio_risk.overall_risk_score > BigDecimal::from(0));
            assert!(portfolio_risk.protocol_count == 3);
        }
    }
    
    #[tokio::test]
    async fn test_legacy_risk_assessment_conversion() {
        let service = RiskOrchestrationService::new().await.unwrap();
        let positions = vec![create_test_position("lido", 10000.0)];
        
        let result = service.calculate_legacy_risk_assessment(&positions, "0x123...").await;
        assert!(result.is_ok());
        
        if let Ok(assessment) = result {
            assert_eq!(assessment.user_address, "0x123...");
            assert!(assessment.overall_risk_score >= 0.0);
            assert!(assessment.overall_risk_score <= 100.0);
            assert!(!assessment.risk_level.is_empty());
        }
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let service = RiskOrchestrationService::new().await.unwrap();
        let result = service.health_check().await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_statistics() {
        let service = RiskOrchestrationService::new().await.unwrap();
        let stats = service.get_statistics().await;
        
        // Should have some basic statistics
        assert!(stats.is_object());
    }
}
