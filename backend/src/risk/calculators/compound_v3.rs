// Compound V3 Risk Calculator - Comprehensive risk assessment for Compound positions
use crate::adapters::compound_v3::contracts::CompoundAccountSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundRiskAssessment {
    pub overall_risk_score: u8,
    pub risk_factors: HashMap<String, RiskFactor>,
    pub health_status: HealthStatus,
    pub liquidation_risk: LiquidationRisk,
    pub recommendations: Vec<String>,
    pub confidence_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub score: u8,
    pub weight: f64,
    pub description: String,
    pub severity: RiskSeverity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Danger,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationRisk {
    pub probability: f64,
    pub price_drop_threshold: f64,
    pub time_to_liquidation_hours: Option<f64>,
    pub liquidation_penalty: f64,
}

pub struct CompoundV3RiskCalculator;

impl CompoundV3RiskCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate comprehensive risk score for Compound V3 positions
    pub fn calculate_risk(&self, account: &CompoundAccountSummary) -> CompoundRiskAssessment {
        let mut risk_factors = HashMap::new();
        
        // 1. Health Factor Risk (35% weight)
        let health_factor_risk = self.calculate_health_factor_risk(account.overall_health_factor);
        risk_factors.insert("health_factor".to_string(), RiskFactor {
            score: health_factor_risk.0,
            weight: 0.35,
            description: health_factor_risk.1,
            severity: self.score_to_severity(health_factor_risk.0),
        });

        // 2. Utilization Risk (30% weight)
        let utilization_risk = self.calculate_utilization_risk(account);
        risk_factors.insert("utilization".to_string(), RiskFactor {
            score: utilization_risk.0,
            weight: 0.30,
            description: utilization_risk.1,
            severity: self.score_to_severity(utilization_risk.0),
        });

        // 3. Collateral Concentration Risk (15% weight)
        let concentration_risk = self.calculate_concentration_risk(account);
        risk_factors.insert("collateral_concentration".to_string(), RiskFactor {
            score: concentration_risk.0,
            weight: 0.15,
            description: concentration_risk.1,
            severity: self.score_to_severity(concentration_risk.0),
        });

        // 4. Interest Rate Risk (10% weight)
        let interest_rate_risk = self.calculate_interest_rate_risk(account);
        risk_factors.insert("interest_rate".to_string(), RiskFactor {
            score: interest_rate_risk.0,
            weight: 0.10,
            description: interest_rate_risk.1,
            severity: self.score_to_severity(interest_rate_risk.0),
        });

        // 5. Market Risk (10% weight)
        let market_risk = self.calculate_market_risk(account);
        risk_factors.insert("market_risk".to_string(), RiskFactor {
            score: market_risk.0,
            weight: 0.10,
            description: market_risk.1,
            severity: self.score_to_severity(market_risk.0),
        });

        // Calculate weighted overall score
        let overall_score = self.calculate_weighted_score(&risk_factors);
        
        // Determine health status
        let health_status = self.determine_health_status(overall_score, account.overall_health_factor);
        
        // Calculate liquidation risk
        let liquidation_risk = self.calculate_liquidation_risk(account);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(account, &risk_factors);
        
        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(account);

        CompoundRiskAssessment {
            overall_risk_score: overall_score,
            risk_factors,
            health_status,
            liquidation_risk,
            recommendations,
            confidence_score,
        }
    }

    /// Calculate health factor risk (0-100 scale)
    fn calculate_health_factor_risk(&self, health_factor: f64) -> (u8, String) {
        if health_factor == f64::INFINITY {
            (15, "No borrowing - minimal liquidation risk".to_string())
        } else if health_factor >= 2.0 {
            (20, "Very safe position with high health factor".to_string())
        } else if health_factor >= 1.5 {
            (35, "Safe position but monitor for market changes".to_string())
        } else if health_factor >= 1.25 {
            (50, "Moderate risk - consider reducing leverage".to_string())
        } else if health_factor >= 1.1 {
            (70, "High risk - close to liquidation threshold".to_string())
        } else if health_factor >= 1.0 {
            (90, "Critical risk - liquidation imminent".to_string())
        } else {
            (100, "Position is liquidatable".to_string())
        }
    }

    /// Calculate utilization risk
    fn calculate_utilization_risk(&self, account: &CompoundAccountSummary) -> (u8, String) {
        let utilization = account.utilization_percentage;
        
        if utilization <= 20.0 {
            (10, "Low utilization - conservative borrowing".to_string())
        } else if utilization <= 40.0 {
            (25, "Moderate utilization - reasonable leverage".to_string())
        } else if utilization <= 60.0 {
            (45, "High utilization - monitor closely".to_string())
        } else if utilization <= 80.0 {
            (70, "Very high utilization - consider deleveraging".to_string())
        } else if utilization <= 95.0 {
            (85, "Extreme utilization - high liquidation risk".to_string())
        } else {
            (95, "Maximum utilization - liquidation imminent".to_string())
        }
    }

    /// Calculate collateral concentration risk
    fn calculate_concentration_risk(&self, account: &CompoundAccountSummary) -> (u8, String) {
        if account.positions.is_empty() {
            return (0, "No positions to analyze".to_string());
        }

        // Calculate concentration in largest collateral position
        let mut max_collateral_percentage: f64 = 0.0;
        let mut collateral_count = 0;
        
        for position in &account.positions {
            if position.total_collateral_value_usd > 0.0 {
                collateral_count += 1;
                let percentage = (position.total_collateral_value_usd / account.total_collateral_usd) * 100.0;
                max_collateral_percentage = max_collateral_percentage.max(percentage);
            }
        }

        if collateral_count >= 4 {
            (15, "Well diversified collateral across multiple assets".to_string())
        } else if collateral_count >= 3 {
            (25, "Moderately diversified collateral".to_string())
        } else if collateral_count == 2 {
            (40, "Limited diversification - consider adding more collateral types".to_string())
        } else if max_collateral_percentage >= 90.0 {
            (80, "Highly concentrated in single collateral asset".to_string())
        } else {
            (60, "Concentrated collateral position".to_string())
        }
    }

    /// Calculate interest rate risk
    fn calculate_interest_rate_risk(&self, account: &CompoundAccountSummary) -> (u8, String) {
        if account.total_borrowed_usd == 0.0 {
            return (5, "No borrowing - minimal interest rate risk".to_string());
        }

        // Analyze average borrow rates across positions
        let mut weighted_borrow_rate = 0.0;
        let mut total_borrowed = 0.0;
        
        for position in &account.positions {
            if position.base_balance < 0 {
                let borrowed_amount = (-position.base_balance as f64).abs();
                weighted_borrow_rate += position.market.borrow_apy * borrowed_amount;
                total_borrowed += borrowed_amount;
            }
        }
        
        if total_borrowed > 0.0 {
            weighted_borrow_rate /= total_borrowed;
        }

        if weighted_borrow_rate <= 3.0 {
            (15, "Low interest rates - favorable borrowing conditions".to_string())
        } else if weighted_borrow_rate <= 6.0 {
            (25, "Moderate interest rates".to_string())
        } else if weighted_borrow_rate <= 10.0 {
            (45, "High interest rates - monitor for rate changes".to_string())
        } else if weighted_borrow_rate <= 15.0 {
            (65, "Very high interest rates - consider refinancing".to_string())
        } else {
            (85, "Extremely high interest rates - urgent action needed".to_string())
        }
    }

    /// Calculate market risk
    fn calculate_market_risk(&self, account: &CompoundAccountSummary) -> (u8, String) {
        let mut volatile_asset_exposure = 0.0;
        let mut stablecoin_exposure = 0.0;
        
        for position in &account.positions {
            let base_symbol = &position.market.base_token_symbol;
            let exposure_usd = position.total_collateral_value_usd + position.base_balance_usd.abs();
            
            if self.is_stablecoin(base_symbol) {
                stablecoin_exposure += exposure_usd;
            } else if self.is_high_volatility(base_symbol) {
                volatile_asset_exposure += exposure_usd;
            }
        }
        
        let total_exposure = account.total_collateral_usd + account.total_borrowed_usd;
        if total_exposure == 0.0 {
            return (0, "No market exposure".to_string());
        }
        
        let volatile_percentage = (volatile_asset_exposure / total_exposure) * 100.0;
        let stable_percentage = (stablecoin_exposure / total_exposure) * 100.0;
        
        if stable_percentage >= 80.0 {
            (20, "Low market risk - mostly stablecoin exposure".to_string())
        } else if volatile_percentage <= 30.0 {
            (35, "Moderate market risk - balanced asset exposure".to_string())
        } else if volatile_percentage <= 60.0 {
            (55, "High market risk - significant volatile asset exposure".to_string())
        } else {
            (75, "Very high market risk - concentrated in volatile assets".to_string())
        }
    }

    /// Calculate liquidation risk details
    fn calculate_liquidation_risk(&self, account: &CompoundAccountSummary) -> LiquidationRisk {
        if account.is_liquidatable {
            return LiquidationRisk {
                probability: 1.0,
                price_drop_threshold: 0.0,
                time_to_liquidation_hours: Some(0.0),
                liquidation_penalty: 0.08, // 8% typical Compound V3 penalty
            };
        }
        
        if account.overall_health_factor == f64::INFINITY {
            return LiquidationRisk {
                probability: 0.0,
                price_drop_threshold: 100.0,
                time_to_liquidation_hours: None,
                liquidation_penalty: 0.08,
            };
        }
        
        // Calculate price drop needed for liquidation
        let price_drop_threshold = ((account.overall_health_factor - 1.0) / account.overall_health_factor) * 100.0;
        
        // Estimate liquidation probability based on health factor and market volatility
        let probability = if account.overall_health_factor >= 2.0 {
            0.01 // 1% chance
        } else if account.overall_health_factor >= 1.5 {
            0.05 // 5% chance
        } else if account.overall_health_factor >= 1.25 {
            0.15 // 15% chance
        } else if account.overall_health_factor >= 1.1 {
            0.35 // 35% chance
        } else {
            0.75 // 75% chance
        };
        
        // Estimate time to liquidation based on current market conditions
        let time_to_liquidation = if account.overall_health_factor <= 1.1 {
            Some(24.0) // 24 hours
        } else if account.overall_health_factor <= 1.25 {
            Some(72.0) // 3 days
        } else {
            None // More than a week
        };
        
        LiquidationRisk {
            probability,
            price_drop_threshold: price_drop_threshold.max(0.0),
            time_to_liquidation_hours: time_to_liquidation,
            liquidation_penalty: 0.08,
        }
    }

    /// Generate actionable recommendations
    fn generate_recommendations(&self, account: &CompoundAccountSummary, risk_factors: &HashMap<String, RiskFactor>) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Health factor recommendations
        if let Some(health_risk) = risk_factors.get("health_factor") {
            if health_risk.score >= 70 {
                recommendations.push("URGENT: Add more collateral or repay debt to improve health factor".to_string());
            } else if health_risk.score >= 50 {
                recommendations.push("Consider adding collateral or reducing debt exposure".to_string());
            }
        }
        
        // Utilization recommendations
        if let Some(util_risk) = risk_factors.get("utilization") {
            if util_risk.score >= 70 {
                recommendations.push("Reduce borrowing utilization to lower liquidation risk".to_string());
            }
        }
        
        // Concentration recommendations
        if let Some(conc_risk) = risk_factors.get("collateral_concentration") {
            if conc_risk.score >= 60 {
                recommendations.push("Diversify collateral across multiple asset types".to_string());
            }
        }
        
        // Interest rate recommendations
        if let Some(rate_risk) = risk_factors.get("interest_rate") {
            if rate_risk.score >= 65 {
                recommendations.push("Monitor interest rates and consider refinancing if rates decrease".to_string());
            }
        }
        
        // Market risk recommendations
        if let Some(market_risk) = risk_factors.get("market_risk") {
            if market_risk.score >= 55 {
                recommendations.push("Consider hedging volatile asset exposure or rebalancing to stablecoins".to_string());
            }
        }
        
        // General recommendations
        if account.is_liquidatable {
            recommendations.push("CRITICAL: Position is liquidatable - take immediate action".to_string());
        }
        
        if recommendations.is_empty() {
            recommendations.push("Position appears healthy - continue monitoring market conditions".to_string());
        }
        
        recommendations
    }

    /// Calculate weighted overall risk score
    fn calculate_weighted_score(&self, risk_factors: &HashMap<String, RiskFactor>) -> u8 {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;
        
        for factor in risk_factors.values() {
            weighted_sum += factor.score as f64 * factor.weight;
            total_weight += factor.weight;
        }
        
        if total_weight > 0.0 {
            (weighted_sum / total_weight).round() as u8
        } else {
            0
        }
    }

    /// Determine health status from score and health factor
    fn determine_health_status(&self, score: u8, health_factor: f64) -> HealthStatus {
        if health_factor < 1.0 || score >= 90 {
            HealthStatus::Critical
        } else if health_factor < 1.25 || score >= 70 {
            HealthStatus::Danger
        } else if health_factor < 1.5 || score >= 50 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// Convert risk score to severity level
    fn score_to_severity(&self, score: u8) -> RiskSeverity {
        match score {
            0..=25 => RiskSeverity::Low,
            26..=50 => RiskSeverity::Medium,
            51..=75 => RiskSeverity::High,
            _ => RiskSeverity::Critical,
        }
    }

    /// Calculate confidence score for the risk assessment
    fn calculate_confidence_score(&self, account: &CompoundAccountSummary) -> f64 {
        let mut confidence: f64 = 1.0;

        // Reduce confidence for very small positions
        if account.net_worth_usd < 100.0 {
            confidence *= 0.7;
        }

        // Reduce confidence for positions with no debt (limited risk factors)
        if account.total_borrowed_usd == 0.0 {
            confidence *= 0.8;
        }

        // Reduce confidence for very few positions
        if account.positions.len() < 2 {
            confidence *= 0.9;
        }

        confidence.max(0.5) // Minimum 50% confidence
    }

    /// Check if asset is a stablecoin
    fn is_stablecoin(&self, symbol: &str) -> bool {
        matches!(symbol.to_uppercase().as_str(), "USDC" | "USDT" | "DAI" | "BUSD" | "FRAX" | "LUSD" | "USDB")
    }

    /// Check if asset is high volatility
    fn is_high_volatility(&self, symbol: &str) -> bool {
        matches!(symbol.to_uppercase().as_str(), "WETH" | "ETH" | "WBTC" | "BTC" | "COMP" | "UNI" | "LINK")
    }
}

impl Default for CompoundV3RiskCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::compound_v3::contracts::{CompoundUserPosition, CompoundMarketInfo};
    use alloy::primitives::Address;
    use std::str::FromStr;

    fn create_mock_account() -> CompoundAccountSummary {
        CompoundAccountSummary {
            positions: vec![],
            total_supplied_usd: 10000.0,
            total_borrowed_usd: 5000.0,
            total_collateral_usd: 8000.0,
            net_worth_usd: 5000.0,
            total_borrow_capacity_usd: 6000.0,
            utilization_percentage: 83.33, // 5000/6000
            overall_health_factor: 1.6,
            is_liquidatable: false,
            total_pending_rewards_usd: 50.0,
        }
    }

    #[test]
    fn test_risk_calculation() {
        let calculator = CompoundV3RiskCalculator::new();
        let account = create_mock_account();
        
        let assessment = calculator.calculate_risk(&account);
        
        assert!(assessment.overall_risk_score <= 100);
        assert!(assessment.confidence_score >= 0.5);
        assert!(assessment.confidence_score <= 1.0);
        assert!(!assessment.recommendations.is_empty());
        assert_eq!(assessment.risk_factors.len(), 5);
    }

    #[test]
    fn test_health_factor_risk() {
        let calculator = CompoundV3RiskCalculator::new();
        
        let (score_healthy, _) = calculator.calculate_health_factor_risk(2.0);
        let (score_danger, _) = calculator.calculate_health_factor_risk(1.1);
        let (score_liquidatable, _) = calculator.calculate_health_factor_risk(0.9);
        
        assert!(score_healthy < score_danger);
        assert!(score_danger < score_liquidatable);
        assert_eq!(score_liquidatable, 100);
    }

    #[test]
    fn test_liquidation_risk() {
        let calculator = CompoundV3RiskCalculator::new();
        let account = create_mock_account();
        
        let liquidation_risk = calculator.calculate_liquidation_risk(&account);
        
        assert!(liquidation_risk.probability >= 0.0);
        assert!(liquidation_risk.probability <= 1.0);
        assert!(liquidation_risk.price_drop_threshold >= 0.0);
        assert_eq!(liquidation_risk.liquidation_penalty, 0.08);
    }
}
