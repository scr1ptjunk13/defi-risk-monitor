// Aave V3 Risk Calculator - Comprehensive risk assessment for Aave positions
use crate::adapters::aave_v3::contracts::AaveAccountSummary;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AaveRiskAssessment {
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

pub struct AaveV3RiskCalculator;

impl AaveV3RiskCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate comprehensive risk score for Aave positions
    pub fn calculate_risk(&self, account: &AaveAccountSummary) -> AaveRiskAssessment {
        let mut risk_factors = HashMap::new();
        
        // 1. Health Factor Risk (40% weight)
        let health_factor_risk = self.calculate_health_factor_risk(account.health_factor);
        risk_factors.insert("health_factor".to_string(), RiskFactor {
            score: health_factor_risk.0,
            weight: 0.40,
            description: health_factor_risk.1,
            severity: self.score_to_severity(health_factor_risk.0),
        });

        // 2. Debt Utilization Risk (25% weight)
        let utilization_risk = self.calculate_utilization_risk(account);
        risk_factors.insert("debt_utilization".to_string(), RiskFactor {
            score: utilization_risk.0,
            weight: 0.25,
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
        let health_status = self.determine_health_status(overall_score, account.health_factor);
        
        // Calculate liquidation risk
        let liquidation_risk = self.calculate_liquidation_risk(account);
        
        // Generate recommendations
        let recommendations = self.generate_recommendations(account, &risk_factors);
        
        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(account);

        AaveRiskAssessment {
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
        let score = if health_factor >= 2.0 {
            // Very healthy
            10
        } else if health_factor >= 1.5 {
            // Healthy
            25
        } else if health_factor >= 1.2 {
            // Warning zone
            50
        } else if health_factor >= 1.05 {
            // Danger zone
            75
        } else if health_factor > 1.0 {
            // Critical zone
            90
        } else {
            // Liquidatable
            100
        };

        let description = match score {
            0..=20 => format!("Excellent health factor ({:.2}). Very low liquidation risk.", health_factor),
            21..=40 => format!("Good health factor ({:.2}). Low liquidation risk.", health_factor),
            41..=60 => format!("Moderate health factor ({:.2}). Monitor position closely.", health_factor),
            61..=80 => format!("Low health factor ({:.2}). High liquidation risk.", health_factor),
            81..=95 => format!("Critical health factor ({:.2}). Immediate action required.", health_factor),
            _ => format!("Position is liquidatable (HF: {:.2}). Urgent intervention needed.", health_factor),
        };

        (score, description)
    }

    /// Calculate debt utilization risk
    fn calculate_utilization_risk(&self, account: &AaveAccountSummary) -> (u8, String) {
        if account.total_collateral_usd == 0.0 {
            return (0, "No collateral supplied.".to_string());
        }

        let utilization_ratio = account.total_debt_usd / account.available_borrows_usd.max(1.0);
        
        let score = if utilization_ratio <= 0.3 {
            10 // Conservative utilization
        } else if utilization_ratio <= 0.5 {
            25 // Moderate utilization
        } else if utilization_ratio <= 0.7 {
            50 // High utilization
        } else if utilization_ratio <= 0.85 {
            75 // Very high utilization
        } else {
            90 // Extreme utilization
        };

        let description = format!(
            "Debt utilization at {:.1}% of available borrowing capacity. Current debt: ${:.2}, Available: ${:.2}",
            utilization_ratio * 100.0,
            account.total_debt_usd,
            account.available_borrows_usd
        );

        (score, description)
    }

    /// Calculate collateral concentration risk
    fn calculate_concentration_risk(&self, account: &AaveAccountSummary) -> (u8, String) {
        if account.positions.is_empty() {
            return (0, "No positions found.".to_string());
        }

        // Calculate Herfindahl-Hirschman Index for concentration
        let total_collateral = account.total_collateral_usd;
        if total_collateral == 0.0 {
            return (0, "No collateral supplied.".to_string());
        }

        let mut hhi = 0.0;
        let mut largest_position_pct: f64 = 0.0;
        let mut position_count = 0;

        for position in &account.positions {
            if position.supply_balance_usd > 0.0 {
                let share = position.supply_balance_usd / total_collateral;
                hhi += share * share;
                largest_position_pct = largest_position_pct.max(share);
                position_count += 1;
            }
        }

        let score = if hhi <= 0.25 && position_count >= 4 {
            10 // Well diversified
        } else if hhi <= 0.5 && position_count >= 3 {
            25 // Moderately diversified
        } else if hhi <= 0.75 {
            50 // Concentrated
        } else if largest_position_pct >= 0.8 {
            75 // Highly concentrated
        } else {
            90 // Extremely concentrated
        };

        let description = format!(
            "Portfolio concentration: {:.1}% in largest position across {} assets. HHI: {:.3}",
            largest_position_pct * 100.0,
            position_count,
            hhi
        );

        (score, description)
    }

    /// Calculate interest rate risk
    fn calculate_interest_rate_risk(&self, account: &AaveAccountSummary) -> (u8, String) {
        if account.total_debt_usd == 0.0 {
            return (0, "No debt positions.".to_string());
        }

        let mut weighted_borrow_rate = 0.0;
        let mut stable_debt_ratio = 0.0;
        let mut variable_debt_count = 0;

        for position in &account.positions {
            if position.variable_debt.is_zero() && position.stable_debt.is_zero() {
                continue;
            }

            let debt_weight = position.debt_balance_usd / account.total_debt_usd;
            
            if !position.variable_debt.is_zero() {
                weighted_borrow_rate += position.variable_borrow_apy * debt_weight;
                variable_debt_count += 1;
            }
            
            if !position.stable_debt.is_zero() {
                weighted_borrow_rate += position.stable_borrow_apy * debt_weight;
                stable_debt_ratio += debt_weight;
            }
        }

        let score = if stable_debt_ratio >= 0.8 {
            15 // Mostly stable rate debt
        } else if weighted_borrow_rate <= 5.0 {
            20 // Low interest rates
        } else if weighted_borrow_rate <= 10.0 {
            40 // Moderate interest rates
        } else if weighted_borrow_rate <= 20.0 {
            70 // High interest rates
        } else {
            90 // Very high interest rates
        };

        let description = format!(
            "Weighted average borrow rate: {:.2}%. Stable debt ratio: {:.1}%. Variable positions: {}",
            weighted_borrow_rate,
            stable_debt_ratio * 100.0,
            variable_debt_count
        );

        (score, description)
    }

    /// Calculate market risk
    fn calculate_market_risk(&self, account: &AaveAccountSummary) -> (u8, String) {
        // Simplified market risk based on asset volatility and correlation
        let mut high_volatility_exposure = 0.0;
        let mut stablecoin_exposure = 0.0;
        let total_value = account.total_collateral_usd + account.total_debt_usd;

        if total_value == 0.0 {
            return (0, "No market exposure.".to_string());
        }

        for position in &account.positions {
            let position_value = position.supply_balance_usd + position.debt_balance_usd;
            let exposure_ratio = position_value / total_value;

            // Classify assets by volatility (simplified)
            if self.is_stablecoin(&position.symbol) {
                stablecoin_exposure += exposure_ratio;
            } else if self.is_high_volatility(&position.symbol) {
                high_volatility_exposure += exposure_ratio;
            }
        }

        let score = if stablecoin_exposure >= 0.8 {
            10 // Mostly stablecoins
        } else if high_volatility_exposure <= 0.2 {
            25 // Low volatility exposure
        } else if high_volatility_exposure <= 0.5 {
            50 // Moderate volatility exposure
        } else if high_volatility_exposure <= 0.8 {
            75 // High volatility exposure
        } else {
            90 // Very high volatility exposure
        };

        let description = format!(
            "Market risk exposure: {:.1}% high volatility assets, {:.1}% stablecoins",
            high_volatility_exposure * 100.0,
            stablecoin_exposure * 100.0
        );

        (score, description)
    }

    /// Calculate liquidation risk details
    fn calculate_liquidation_risk(&self, account: &AaveAccountSummary) -> LiquidationRisk {
        if account.health_factor <= 1.0 {
            return LiquidationRisk {
                probability: 1.0,
                price_drop_threshold: 0.0,
                time_to_liquidation_hours: Some(0.0),
                liquidation_penalty: 0.05, // 5% typical penalty
            };
        }

        // Calculate price drop needed for liquidation
        let price_drop_threshold = 1.0 - (1.0 / account.health_factor);
        
        // Estimate probability based on health factor
        let probability = if account.health_factor >= 2.0 {
            0.01 // 1% chance
        } else if account.health_factor >= 1.5 {
            0.05 // 5% chance
        } else if account.health_factor >= 1.2 {
            0.15 // 15% chance
        } else if account.health_factor >= 1.1 {
            0.35 // 35% chance
        } else {
            0.65 // 65% chance
        };

        // Estimate time to liquidation (simplified)
        let time_to_liquidation_hours = if account.health_factor <= 1.05 {
            Some(1.0) // Very soon
        } else if account.health_factor <= 1.2 {
            Some(24.0) // Within a day
        } else {
            None // Not imminent
        };

        LiquidationRisk {
            probability,
            price_drop_threshold: price_drop_threshold * 100.0,
            time_to_liquidation_hours,
            liquidation_penalty: 0.05,
        }
    }

    /// Generate actionable recommendations
    fn generate_recommendations(&self, account: &AaveAccountSummary, risk_factors: &HashMap<String, RiskFactor>) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Health factor recommendations
        if let Some(hf_risk) = risk_factors.get("health_factor") {
            if hf_risk.score >= 75 {
                recommendations.push("URGENT: Add more collateral or repay debt to improve health factor".to_string());
            } else if hf_risk.score >= 50 {
                recommendations.push("Consider adding collateral or reducing debt exposure".to_string());
            }
        }

        // Utilization recommendations
        if let Some(util_risk) = risk_factors.get("debt_utilization") {
            if util_risk.score >= 75 {
                recommendations.push("Reduce debt utilization to improve position safety".to_string());
            }
        }

        // Concentration recommendations
        if let Some(conc_risk) = risk_factors.get("collateral_concentration") {
            if conc_risk.score >= 50 {
                recommendations.push("Diversify collateral across multiple assets to reduce concentration risk".to_string());
            }
        }

        // Interest rate recommendations
        if let Some(ir_risk) = risk_factors.get("interest_rate") {
            if ir_risk.score >= 70 {
                recommendations.push("Consider switching to stable rate debt or refinancing to lower rates".to_string());
            }
        }

        // Market risk recommendations
        if let Some(market_risk) = risk_factors.get("market_risk") {
            if market_risk.score >= 75 {
                recommendations.push("Consider reducing exposure to high-volatility assets".to_string());
            }
        }

        // General recommendations
        if account.health_factor < 1.5 && account.total_debt_usd > 0.0 {
            recommendations.push("Set up automated monitoring for liquidation alerts".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("Position appears healthy. Continue monitoring market conditions".to_string());
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
        if health_factor <= 1.0 {
            HealthStatus::Critical
        } else if health_factor <= 1.1 || score >= 80 {
            HealthStatus::Danger
        } else if health_factor <= 1.5 || score >= 50 {
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
    fn calculate_confidence_score(&self, account: &AaveAccountSummary) -> f64 {
        let mut confidence: f64 = 1.0;

        // Reduce confidence for very small positions
        if account.total_collateral_usd < 100.0 {
            confidence *= 0.7;
        }

        // Reduce confidence for positions with no debt (limited risk factors)
        if account.total_debt_usd == 0.0 {
            confidence *= 0.8;
        }

        // Reduce confidence for very few positions
        if account.positions.len() < 2 {
            confidence *= 0.9;
        }

        confidence.max(0.5_f64) // Minimum 50% confidence
    }

    /// Check if asset is a stablecoin
    fn is_stablecoin(&self, symbol: &str) -> bool {
        matches!(symbol.to_uppercase().as_str(), "USDC" | "USDT" | "DAI" | "BUSD" | "FRAX" | "LUSD")
    }

    /// Check if asset is high volatility
    fn is_high_volatility(&self, symbol: &str) -> bool {
        matches!(symbol.to_uppercase().as_str(), "UNI" | "AAVE" | "LINK" | "MATIC" | "AVAX")
    }
}

impl Default for AaveV3RiskCalculator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::contracts::AaveUserPosition;
    use alloy::primitives::U256;

    fn create_mock_account() -> AaveAccountSummary {
        AaveAccountSummary {
            total_collateral_usd: 10000.0,
            total_debt_usd: 5000.0,
            available_borrows_usd: 3000.0,
            current_liquidation_threshold: 80.0,
            loan_to_value: 75.0,
            health_factor: 1.6,
            net_worth_usd: 5000.0,
            positions: vec![
                AaveUserPosition {
                    asset_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".parse().unwrap(),
                    symbol: "WETH".to_string(),
                    a_token_balance: U256::from(5000000000000000000u64), // 5 ETH
                    stable_debt: U256::ZERO,
                    variable_debt: U256::from(2000000000000000000u64), // 2 ETH
                    usage_as_collateral_enabled: true,
                    supply_apy: 3.5,
                    variable_borrow_apy: 4.2,
                    stable_borrow_apy: 5.1,
                    supply_balance_usd: 8000.0,
                    debt_balance_usd: 3200.0,
                    net_balance_usd: 4800.0,
                },
            ],
        }
    }

    #[test]
    fn test_risk_calculation() {
        let calculator = AaveV3RiskCalculator::new();
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
        let calculator = AaveV3RiskCalculator::new();
        
        let (score_healthy, _) = calculator.calculate_health_factor_risk(2.0);
        let (score_danger, _) = calculator.calculate_health_factor_risk(1.1);
        let (score_liquidatable, _) = calculator.calculate_health_factor_risk(0.9);
        
        assert!(score_healthy < score_danger);
        assert!(score_danger < score_liquidatable);
        assert_eq!(score_liquidatable, 100);
    }

    #[test]
    fn test_liquidation_risk() {
        let calculator = AaveV3RiskCalculator::new();
        let account = create_mock_account();
        
        let liquidation_risk = calculator.calculate_liquidation_risk(&account);
        
        assert!(liquidation_risk.probability >= 0.0);
        assert!(liquidation_risk.probability <= 1.0);
        assert!(liquidation_risk.price_drop_threshold >= 0.0);
        assert_eq!(liquidation_risk.liquidation_penalty, 0.05);
    }
}
