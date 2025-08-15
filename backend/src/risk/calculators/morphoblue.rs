// Advanced Morpho Blue Risk Management System
use alloy::primitives::{Address, U256, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::adapters::morphoblue::{MorphoAccountSummary, MorphoUserPosition, MorphoMarket};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    VeryLow,    // 0-20
    Low,        // 21-40
    Medium,     // 41-60
    High,       // 61-80
    VeryHigh,   // 81-95
    Critical,   // 96-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertType {
    HealthFactorWarning,
    HealthFactorCritical,
    LiquidationRisk,
    InterestRateSpike,
    MarketUtilizationHigh,
    ConcentrationRisk,
    OracleDeviation,
    MarketVolatilityHigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAlert {
    pub alert_type: AlertType,
    pub severity: RiskLevel,
    pub market_id: Option<B256>,
    pub message: String,
    pub recommended_action: String,
    pub urgency_score: u8, // 0-100
    pub timestamp: u64,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskMetrics {
    pub overall_risk_score: u8,
    pub risk_level: RiskLevel,
    pub health_factor_distribution: Vec<f64>,
    pub avg_health_factor: f64,
    pub min_health_factor: f64,
    pub liquidation_buffer: f64, // Hours until liquidation at current rates
    pub concentration_risk: f64,  // 0-1 scale
    pub interest_rate_risk: f64, // Average borrow rate weighted by debt
    pub market_risk_exposure: HashMap<String, f64>, // Token -> exposure %
    pub total_leverage: f64,
    pub var_95: f64, // Value at Risk 95% confidence
    pub expected_shortfall: f64,
    pub diversification_score: f64, // 0-100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRiskAnalysis {
    pub market_id: B256,
    pub market_risk_score: u8,
    pub oracle_risk: u8,
    pub liquidity_risk: u8,
    pub volatility_risk: u8,
    pub utilization_risk: u8,
    pub interest_rate_risk: u8,
    pub collateral_quality_risk: u8,
    pub loan_asset_risk: u8,
    pub historical_liquidations: u32,
    pub time_to_liquidation_hours: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskScenario {
    pub name: String,
    pub description: String,
    pub probability: f64, // 0-1
    pub impact_score: u8, // 0-100
    pub expected_loss_usd: f64,
    pub time_horizon_hours: u32,
    pub mitigation_strategies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationRiskAssessment {
    pub time_to_liquidation: Option<f64>, // Hours
    pub liquidation_buffer_usd: f64,
    pub required_collateral_top_up: f64,
    pub safe_ltv_target: f64,
    pub current_ltv: f64,
    pub liquidation_ltv: f64,
    pub price_drop_to_liquidation: HashMap<String, f64>, // Asset -> % drop needed
}

pub struct MorphoBlueRiskManager {
    // Risk thresholds and configuration
    config: RiskConfig,
    // Historical data for trend analysis
    historical_data: Vec<HistoricalSnapshot>,
    // Risk model parameters
    volatility_models: HashMap<String, VolatilityModel>,
}

#[derive(Debug, Clone)]
struct RiskConfig {
    health_factor_warning_threshold: f64,
    health_factor_critical_threshold: f64,
    max_concentration_per_asset: f64,
    max_total_leverage: f64,
    utilization_warning_threshold: f64,
    interest_rate_spike_threshold: f64,
    oracle_deviation_threshold: f64,
}

#[derive(Debug, Clone)]
struct HistoricalSnapshot {
    timestamp: u64,
    account_summary: MorphoAccountSummary,
    market_conditions: HashMap<B256, MarketCondition>,
}

#[derive(Debug, Clone)]
struct MarketCondition {
    utilization_rate: f64,
    supply_rate: f64,
    borrow_rate: f64,
    total_supply: f64,
    total_borrow: f64,
    oracle_price: f64,
}

#[derive(Debug, Clone)]
struct VolatilityModel {
    asset_symbol: String,
    daily_volatility: f64,
    weekly_volatility: f64,
    monthly_volatility: f64,
    correlation_matrix: HashMap<String, f64>,
    var_confidence_levels: HashMap<u8, f64>, // confidence% -> VaR
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            health_factor_warning_threshold: 1.3,
            health_factor_critical_threshold: 1.15,
            max_concentration_per_asset: 0.4, // 40%
            max_total_leverage: 3.0,
            utilization_warning_threshold: 85.0,
            interest_rate_spike_threshold: 15.0, // 15% APY
            oracle_deviation_threshold: 0.05, // 5%
        }
    }
}

impl MorphoBlueRiskManager {
    pub fn new() -> Self {
        Self {
            config: RiskConfig::default(),
            historical_data: Vec::new(),
            volatility_models: Self::initialize_volatility_models(),
        }
    }

    pub fn with_config(config: RiskConfig) -> Self {
        Self {
            config,
            historical_data: Vec::new(),
            volatility_models: Self::initialize_volatility_models(),
        }
    }

    /// Initialize volatility models for common assets
    fn initialize_volatility_models() -> HashMap<String, VolatilityModel> {
        let mut models = HashMap::new();
        
        // ETH volatility model
        let mut eth_correlations = HashMap::new();
        eth_correlations.insert("WSTETH".to_string(), 0.95);
        eth_correlations.insert("RETH".to_string(), 0.93);
        eth_correlations.insert("CBETH".to_string(), 0.94);
        eth_correlations.insert("WBTC".to_string(), 0.65);
        eth_correlations.insert("USDC".to_string(), -0.1);
        
        let mut eth_var = HashMap::new();
        eth_var.insert(95, 0.08); // 8% daily VaR at 95%
        eth_var.insert(99, 0.12); // 12% daily VaR at 99%
        
        models.insert("ETH".to_string(), VolatilityModel {
            asset_symbol: "ETH".to_string(),
            daily_volatility: 0.05, // 5% daily volatility
            weekly_volatility: 0.12,
            monthly_volatility: 0.24,
            correlation_matrix: eth_correlations,
            var_confidence_levels: eth_var.clone(),
        });

        // WSTETH volatility model
        let mut wsteth_correlations = HashMap::new();
        wsteth_correlations.insert("ETH".to_string(), 0.95);
        wsteth_correlations.insert("RETH".to_string(), 0.89);
        wsteth_correlations.insert("CBETH".to_string(), 0.91);
        
        models.insert("WSTETH".to_string(), VolatilityModel {
            asset_symbol: "WSTETH".to_string(),
            daily_volatility: 0.055, // Slightly higher due to staking risk
            weekly_volatility: 0.13,
            monthly_volatility: 0.26,
            correlation_matrix: wsteth_correlations,
            var_confidence_levels: eth_var.clone(),
        });

        // WBTC volatility model
        let mut btc_correlations = HashMap::new();
        btc_correlations.insert("ETH".to_string(), 0.65);
        btc_correlations.insert("USDC".to_string(), -0.05);
        
        let mut btc_var = HashMap::new();
        btc_var.insert(95, 0.09); // 9% daily VaR at 95%
        btc_var.insert(99, 0.14); // 14% daily VaR at 99%
        
        models.insert("WBTC".to_string(), VolatilityModel {
            asset_symbol: "WBTC".to_string(),
            daily_volatility: 0.06, // 6% daily volatility
            weekly_volatility: 0.16,
            monthly_volatility: 0.32,
            correlation_matrix: btc_correlations,
            var_confidence_levels: btc_var,
        });

        // Stablecoins (low volatility)
        let stable_var = HashMap::from([(95, 0.005), (99, 0.01)]); // Very low VaR
        for symbol in ["USDC", "USDT", "DAI"] {
            models.insert(symbol.to_string(), VolatilityModel {
                asset_symbol: symbol.to_string(),
                daily_volatility: 0.002, // 0.2% daily volatility
                weekly_volatility: 0.005,
                monthly_volatility: 0.01,
                correlation_matrix: HashMap::new(),
                var_confidence_levels: stable_var.clone(),
            });
        }

        models
    }

    /// Perform comprehensive risk analysis on a Morpho Blue account
    pub fn analyze_portfolio_risk(&self, account: &MorphoAccountSummary) -> PortfolioRiskMetrics {
        let overall_risk = self.calculate_overall_risk_score(account);
        let risk_level = self.risk_score_to_level(overall_risk);
        
        // Health factor analysis
        let health_factors: Vec<f64> = account.positions.iter()
            .filter(|p| p.health_factor.is_finite() && p.health_factor > 0.0)
            .map(|p| p.health_factor)
            .collect();
        
        let avg_health_factor = if health_factors.is_empty() {
            f64::INFINITY
        } else {
            health_factors.iter().sum::<f64>() / health_factors.len() as f64
        };
        
        let min_health_factor = health_factors.iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);

        // Concentration risk
        let concentration_risk = self.calculate_concentration_risk(account);
        
        // Interest rate risk
        let interest_rate_risk = self.calculate_interest_rate_risk(account);
        
        // Market exposure
        let market_risk_exposure = self.calculate_market_exposure(account);
        
        // Leverage calculation
        let total_collateral = account.total_supply_value_usd + account.total_collateral_value_usd;
        let total_leverage = if total_collateral > 0.0 {
            account.total_borrow_value_usd / total_collateral
        } else {
            0.0
        };

        // VaR and Expected Shortfall
        let (var_95, expected_shortfall) = self.calculate_var_and_es(account);
        
        // Diversification score
        let diversification_score = self.calculate_diversification_score(account);
        
        // Liquidation buffer
        let liquidation_buffer = self.estimate_liquidation_buffer_hours(account);

        PortfolioRiskMetrics {
            overall_risk_score: overall_risk,
            risk_level,
            health_factor_distribution: health_factors,
            avg_health_factor,
            min_health_factor,
            liquidation_buffer,
            concentration_risk,
            interest_rate_risk,
            market_risk_exposure,
            total_leverage,
            var_95,
            expected_shortfall,
            diversification_score,
        }
    }

    /// Generate risk alerts for the portfolio
    pub fn generate_risk_alerts(&self, account: &MorphoAccountSummary) -> Vec<RiskAlert> {
        let mut alerts = Vec::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Health factor alerts
        for (i, position) in account.positions.iter().enumerate() {
            if position.health_factor.is_finite() && position.health_factor > 0.0 {
                if position.health_factor < self.config.health_factor_critical_threshold {
                    alerts.push(RiskAlert {
                        alert_type: AlertType::HealthFactorCritical,
                        severity: RiskLevel::Critical,
                        market_id: Some(position.market.market_id),
                        message: format!(
                            "CRITICAL: Health factor {} in {}/{} market is below critical threshold {}",
                            position.health_factor,
                            position.market.loan_token_symbol,
                            position.market.collateral_token_symbol,
                            self.config.health_factor_critical_threshold
                        ),
                        recommended_action: "Immediately add collateral or repay debt to avoid liquidation".to_string(),
                        urgency_score: 95,
                        timestamp,
                        metadata: serde_json::json!({
                            "health_factor": position.health_factor,
                            "ltv": position.ltv,
                            "liquidation_ltv": position.liquidation_ltv,
                            "borrow_value_usd": position.borrow_value_usd,
                            "collateral_value_usd": position.collateral_value_usd
                        }),
                    });
                } else if position.health_factor < self.config.health_factor_warning_threshold {
                    alerts.push(RiskAlert {
                        alert_type: AlertType::HealthFactorWarning,
                        severity: RiskLevel::High,
                        market_id: Some(position.market.market_id),
                        message: format!(
                            "WARNING: Health factor {} in {}/{} market is approaching dangerous levels",
                            position.health_factor,
                            position.market.loan_token_symbol,
                            position.market.collateral_token_symbol
                        ),
                        recommended_action: "Consider adding collateral or reducing debt".to_string(),
                        urgency_score: 70,
                        timestamp,
                        metadata: serde_json::json!({
                            "health_factor": position.health_factor,
                            "threshold": self.config.health_factor_warning_threshold
                        }),
                    });
                }
            }

            // Liquidation risk alerts
            if !position.is_healthy {
                alerts.push(RiskAlert {
                    alert_type: AlertType::LiquidationRisk,
                    severity: RiskLevel::Critical,
                    market_id: Some(position.market.market_id),
                    message: format!(
                        "LIQUIDATION RISK: Position in {}/{} market is unhealthy",
                        position.market.loan_token_symbol,
                        position.market.collateral_token_symbol
                    ),
                    recommended_action: "Take immediate action to restore position health".to_string(),
                    urgency_score: 100,
                    timestamp,
                    metadata: serde_json::json!({
                        "is_healthy": position.is_healthy,
                        "position_value": position.net_value_usd
                    }),
                });
            }

            // Interest rate spike alerts
            if position.market.borrow_rate > self.config.interest_rate_spike_threshold {
                alerts.push(RiskAlert {
                    alert_type: AlertType::InterestRateSpike,
                    severity: RiskLevel::Medium,
                    market_id: Some(position.market.market_id),
                    message: format!(
                        "High borrow rate: {}% APY in {}/{} market",
                        position.market.borrow_rate,
                        position.market.loan_token_symbol,
                        position.market.collateral_token_symbol
                    ),
                    recommended_action: "Consider repaying debt or switching to lower-rate markets".to_string(),
                    urgency_score: 50,
                    timestamp,
                    metadata: serde_json::json!({
                        "borrow_rate": position.market.borrow_rate,
                        "threshold": self.config.interest_rate_spike_threshold
                    }),
                });
            }

            // Market utilization alerts
            if position.market.utilization_rate > self.config.utilization_warning_threshold {
                alerts.push(RiskAlert {
                    alert_type: AlertType::MarketUtilizationHigh,
                    severity: RiskLevel::Medium,
                    market_id: Some(position.market.market_id),
                    message: format!(
                        "High market utilization: {}% in {}/{} market",
                        position.market.utilization_rate,
                        position.market.loan_token_symbol,
                        position.market.collateral_token_symbol
                    ),
                    recommended_action: "Monitor for potential supply shortages and rate increases".to_string(),
                    urgency_score: 40,
                    timestamp,
                    metadata: serde_json::json!({
                        "utilization_rate": position.market.utilization_rate,
                        "threshold": self.config.utilization_warning_threshold
                    }),
                });
            }
        }

        // Portfolio-level concentration risk
        let concentration_risk = self.calculate_concentration_risk(account);
        if concentration_risk > self.config.max_concentration_per_asset {
            alerts.push(RiskAlert {
                alert_type: AlertType::ConcentrationRisk,
                severity: RiskLevel::Medium,
                market_id: None,
                message: format!("High concentration risk: {:.1}% in single asset", concentration_risk * 100.0),
                recommended_action: "Diversify across more assets and markets".to_string(),
                urgency_score: 45,
                timestamp,
                metadata: serde_json::json!({
                    "concentration_risk": concentration_risk,
                    "max_allowed": self.config.max_concentration_per_asset
                }),
            });
        }

        alerts.sort_by(|a, b| b.urgency_score.cmp(&a.urgency_score));
        alerts
    }

    /// Analyze individual market risk
    pub fn analyze_market_risk(&self, position: &MorphoUserPosition) -> MarketRiskAnalysis {
        let market = &position.market;
        
        // Oracle risk assessment
        let oracle_risk = self.assess_oracle_risk(market);
        
        // Liquidity risk based on market size and utilization
        let liquidity_risk = self.assess_liquidity_risk(market);
        
        // Volatility risk based on assets involved
        let volatility_risk = self.assess_volatility_risk(market);
        
        // Utilization risk
        let utilization_risk = if market.utilization_rate > 95.0 {
            90u8
        } else if market.utilization_rate > 85.0 {
            70u8
        } else if market.utilization_rate > 75.0 {
            50u8
        } else {
            20u8
        };

        // Interest rate risk
        let interest_rate_risk = if market.borrow_rate > 20.0 {
            85u8
        } else if market.borrow_rate > 15.0 {
            65u8
        } else if market.borrow_rate > 10.0 {
            45u8
        } else {
            25u8
        };

        // Collateral quality risk
        let collateral_quality_risk = match market.collateral_token_symbol.to_uppercase().as_str() {
            "WETH" | "ETH" => 15u8,
            "WSTETH" | "RETH" | "CBETH" => 25u8, // Liquid staking derivatives
            "WBTC" => 20u8,
            _ => 60u8, // Unknown/exotic collateral
        };

        // Loan asset risk
        let loan_asset_risk = match market.loan_token_symbol.to_uppercase().as_str() {
            "USDC" | "USDT" | "DAI" => 10u8, // Stablecoins
            "WETH" | "ETH" => 20u8,
            "WBTC" => 25u8,
            _ => 50u8, // Other assets
        };

        // Overall market risk score (weighted average)
        let market_risk_score = (
            oracle_risk as u32 * 20 +
            liquidity_risk as u32 * 15 +
            volatility_risk as u32 * 20 +
            utilization_risk as u32 * 15 +
            interest_rate_risk as u32 * 10 +
            collateral_quality_risk as u32 * 10 +
            loan_asset_risk as u32 * 10
        ) / 100;

        // Time to liquidation estimation
        let time_to_liquidation_hours = self.estimate_time_to_liquidation(position);

        MarketRiskAnalysis {
            market_id: market.market_id,
            market_risk_score: market_risk_score.min(95) as u8,
            oracle_risk,
            liquidity_risk,
            volatility_risk,
            utilization_risk,
            interest_rate_risk,
            collateral_quality_risk,
            loan_asset_risk,
            historical_liquidations: 0, // Would need historical data
            time_to_liquidation_hours,
        }
    }

    /// Generate risk scenarios and stress tests
    pub fn generate_risk_scenarios(&self, account: &MorphoAccountSummary) -> Vec<RiskScenario> {
        let mut scenarios = Vec::new();

        // Scenario 1: 20% drop in collateral prices
        scenarios.push(RiskScenario {
            name: "Collateral Price Drop".to_string(),
            description: "20% drop in all collateral asset prices".to_string(),
            probability: 0.15, // 15% chance in next 30 days
            impact_score: self.calculate_price_drop_impact(account, 0.2),
            expected_loss_usd: self.estimate_loss_from_price_drop(account, 0.2),
            time_horizon_hours: 24,
            mitigation_strategies: vec![
                "Maintain health factor above 2.0".to_string(),
                "Set up automated stop-losses".to_string(),
                "Diversify collateral assets".to_string(),
            ],
        });

        // Scenario 2: Interest rate spike
        scenarios.push(RiskScenario {
            name: "Interest Rate Spike".to_string(),
            description: "Borrow rates increase by 50% due to market stress".to_string(),
            probability: 0.25,
            impact_score: 60,
            expected_loss_usd: account.total_borrow_value_usd * 0.05, // 5% of borrowed amount
            time_horizon_hours: 168, // 1 week
            mitigation_strategies: vec![
                "Monitor utilization rates closely".to_string(),
                "Have repayment funds ready".to_string(),
                "Consider fixed-rate alternatives".to_string(),
            ],
        });

        // Scenario 3: Oracle failure
        scenarios.push(RiskScenario {
            name: "Oracle Manipulation/Failure".to_string(),
            description: "Price oracle shows incorrect prices leading to unfair liquidations".to_string(),
            probability: 0.05, // 5% chance
            impact_score: 85,
            expected_loss_usd: account.total_collateral_value_usd * 0.15, // 15% of collateral
            time_horizon_hours: 1, // Very fast
            mitigation_strategies: vec![
                "Use markets with reputable oracles".to_string(),
                "Monitor oracle deviations".to_string(),
                "Maintain extra collateral buffer".to_string(),
            ],
        });

        // Scenario 4: Market liquidity crisis
        scenarios.push(RiskScenario {
            name: "Liquidity Crisis".to_string(),
            description: "Severe shortage of liquidity in lending markets".to_string(),
            probability: 0.1,
            impact_score: 70,
            expected_loss_usd: account.net_worth_usd * 0.3,
            time_horizon_hours: 72, // 3 days
            mitigation_strategies: vec![
                "Diversify across multiple protocols".to_string(),
                "Keep emergency repayment funds".to_string(),
                "Monitor market utilization rates".to_string(),
            ],
        });

        scenarios.sort_by(|a, b| {
            let a_risk = (a.probability * a.impact_score as f64) as i32;
            let b_risk = (b.probability * b.impact_score as f64) as i32;
            b_risk.cmp(&a_risk)
        });

        scenarios
    }

    /// Assess liquidation risk and provide detailed analysis
    pub fn assess_liquidation_risk(&self, position: &MorphoUserPosition) -> LiquidationRiskAssessment {
        let current_ltv = position.ltv / 100.0; // Convert percentage to decimal
        let liquidation_ltv = position.liquidation_ltv / 100.0;
        
        // Calculate buffer
        let ltv_buffer = liquidation_ltv - current_ltv;
        let liquidation_buffer_usd = position.collateral_value_usd * ltv_buffer;
        
        // Required collateral to reach safe LTV (e.g., 70% of liquidation LTV)
        let safe_ltv_target = liquidation_ltv * 0.7;
        let required_collateral_top_up = if current_ltv > safe_ltv_target {
            let target_collateral_value = position.borrow_value_usd / safe_ltv_target;
            (target_collateral_value - position.collateral_value_usd).max(0.0)
        } else {
            0.0
        };

        // Calculate price drops needed for liquidation
        let mut price_drops = HashMap::new();
        
        // Collateral price drop needed
        if position.collateral_value_usd > 0.0 && position.borrow_value_usd > 0.0 {
            let required_collateral_value = position.borrow_value_usd / liquidation_ltv;
            let price_drop_needed = 1.0 - (required_collateral_value / position.collateral_value_usd);
            price_drops.insert(
                position.market.collateral_token_symbol.clone(),
                (price_drop_needed * 100.0).max(0.0)
            );
        }

        // Time to liquidation based on interest accrual
        let time_to_liquidation = if position.market.borrow_rate > 0.0 && ltv_buffer > 0.0 {
            let daily_interest_rate = position.market.borrow_rate / 365.0 / 100.0;
            let daily_debt_increase = position.borrow_value_usd * daily_interest_rate;
            let days_to_liquidation = liquidation_buffer_usd / daily_debt_increase;
            Some(days_to_liquidation * 24.0) // Convert to hours
        } else {
            None
        };

        LiquidationRiskAssessment {
            time_to_liquidation,
            liquidation_buffer_usd,
            required_collateral_top_up,
            safe_ltv_target: safe_ltv_target * 100.0, // Convert back to percentage
            current_ltv: position.ltv,
            liquidation_ltv: position.liquidation_ltv,
            price_drop_to_liquidation: price_drops,
        }
    }

    // Private helper methods

    fn calculate_overall_risk_score(&self, account: &MorphoAccountSummary) -> u8 {
        if account.positions.is_empty() {
            return 0;
        }

        let mut total_weighted_risk = 0.0f64;
        let mut total_weight = 0.0f64;

        for position in &account.positions {
            let position_value = position.supply_value_usd + position.collateral_value_usd + position.borrow_value_usd;
            if position_value > 0.0 {
                let market_analysis = self.analyze_market_risk(position);
                let position_risk = market_analysis.market_risk_score as f64;
                
                // Weight by position size
                total_weighted_risk += position_risk * position_value;
                total_weight += position_value;
            }
        }

        let base_risk = if total_weight > 0.0 {
            total_weighted_risk / total_weight
        } else {
            30.0 // Default risk for empty portfolio
        };

        // Adjust for portfolio-level risks
        let mut portfolio_risk = base_risk;

        // Health factor adjustment
        if account.average_health_factor.is_finite() {
            if account.average_health_factor < 1.2 {
                portfolio_risk += 25.0;
            } else if account.average_health_factor < 1.5 {
                portfolio_risk += 15.0;
            } else if account.average_health_factor < 2.0 {
                portfolio_risk += 8.0;
            }
        }

        // Leverage adjustment
        let total_assets = account.total_supply_value_usd + account.total_collateral_value_usd;
        if total_assets > 0.0 {
            let leverage = account.total_borrow_value_usd / total_assets;
            if leverage > 0.8 {
                portfolio_risk += 20.0;
            } else if leverage > 0.6 {
                portfolio_risk += 12.0;
            } else if leverage > 0.4 {
                portfolio_risk += 6.0;
            }
        }

        // Unhealthy positions penalty
        if account.unhealthy_positions > 0 {
            portfolio_risk += (account.unhealthy_positions as f64 * 15.0).min(30.0);
        }

        portfolio_risk.min(95.0) as u8
    }

    fn risk_score_to_level(&self, score: u8) -> RiskLevel {
        match score {
            0..=20 => RiskLevel::VeryLow,
            21..=40 => RiskLevel::Low,
            41..=60 => RiskLevel::Medium,
            61..=80 => RiskLevel::High,
            81..=95 => RiskLevel::VeryHigh,
            _ => RiskLevel::Critical,
        }
    }

    fn calculate_concentration_risk(&self, account: &MorphoAccountSummary) -> f64 {
        let mut asset_exposures: HashMap<String, f64> = HashMap::new();
        let mut total_exposure = 0.0;

        for position in &account.positions {
            let exposure = position.supply_value_usd + position.collateral_value_usd + position.borrow_value_usd;
            total_exposure += exposure;

            *asset_exposures.entry(position.market.loan_token_symbol.clone()).or_insert(0.0) += exposure * 0.5;
            *asset_exposures.entry(position.market.collateral_token_symbol.clone()).or_insert(0.0) += exposure * 0.5;
        }

        if total_exposure > 0.0 {
            asset_exposures.values()
                .map(|exposure| exposure / total_exposure)
                .fold(0.0f64, |max, concentration| max.max(concentration))
        } else {
            0.0
        }
    }

    fn calculate_interest_rate_risk(&self, account: &MorphoAccountSummary) -> f64 {
        if account.total_borrow_value_usd == 0.0 {
            return 0.0;
        }

        let weighted_borrow_rate: f64 = account.positions.iter()
            .filter(|p| p.borrow_value_usd > 0.0)
            .map(|p| p.market.borrow_rate * p.borrow_value_usd)
            .sum::<f64>() / account.total_borrow_value_usd;

        weighted_borrow_rate / 100.0 // Convert to 0-1 scale
    }

    fn calculate_market_exposure(&self, account: &MorphoAccountSummary) -> HashMap<String, f64> {
        let mut exposures = HashMap::new();
        let total_value = account.total_supply_value_usd + account.total_collateral_value_usd + account.total_borrow_value_usd;

        if total_value > 0.0 {
            for position in &account.positions {
                let position_value = position.supply_value_usd + position.collateral_value_usd + position.borrow_value_usd;
                let exposure_pct = (position_value / total_value) * 100.0;

                exposures.insert(
                    format!("{}/{}", position.market.loan_token_symbol, position.market.collateral_token_symbol),
                    exposure_pct
                );
            }
        }

        exposures
    }

    fn calculate_var_and_es(&self, account: &MorphoAccountSummary) -> (f64, f64) {
        // Simplified VaR calculation using portfolio volatility
        let mut portfolio_variance = 0.0;
        let total_value = account.total_supply_value_usd + account.total_collateral_value_usd;

        if total_value > 0.0 {
            for position in &account.positions {
                let weight = (position.supply_value_usd + position.collateral_value_usd) / total_value;
                
                // Get asset volatilities
                let collateral_vol = self.volatility_models
                    .get(&position.market.collateral_token_symbol)
                    .map(|m| m.daily_volatility)
                    .unwrap_or(0.05); // Default 5% daily volatility

                portfolio_variance += weight.powi(2) * collateral_vol.powi(2);
            }
        }

        let portfolio_vol = portfolio_variance.sqrt();
        let var_95 = total_value * portfolio_vol * 1.645; // 95% confidence level
        let expected_shortfall = var_95 * 1.3; // Approximation

        (var_95, expected_shortfall)
    }

    fn calculate_diversification_score(&self, account: &MorphoAccountSummary) -> f64 {
        let num_positions = account.positions.len() as f64;
        let concentration_risk = self.calculate_concentration_risk(account);
        
        // Higher diversification score for more positions and lower concentration
        let base_score = (num_positions / 10.0).min(1.0) * 50.0; // Up to 50 points for position count
        let concentration_penalty = concentration_risk * 50.0; // Up to 50 point penalty for concentration
        
        (base_score - concentration_penalty).max(0.0)
    }

    fn estimate_liquidation_buffer_hours(&self, account: &MorphoAccountSummary) -> f64 {
        let mut min_buffer_hours = f64::INFINITY;

        for position in &account.positions {
            if let Some(hours) = self.estimate_time_to_liquidation(position) {
                min_buffer_hours = min_buffer_hours.min(hours);
            }
        }

        if min_buffer_hours.is_infinite() {
            720.0 // Default 30 days if no liquidation risk
        } else {
            min_buffer_hours
        }
    }

    fn assess_oracle_risk(&self, market: &MorphoMarket) -> u8 {
        // This would typically check oracle type, update frequency, deviation history, etc.
        // For now, providing reasonable defaults based on common oracle patterns
        
        // Check if it's a standard oracle address (simplified)
        let oracle_addr_str = format!("{:?}", market.oracle);
        
        if oracle_addr_str.starts_with("0x0000") {
            80u8 // Null oracle is very risky
        } else {
            // Assume reasonable oracle risk for established oracles
            25u8
        }
    }

    fn assess_liquidity_risk(&self, market: &MorphoMarket) -> u8 {
        let supply_assets: f64 = market.total_supply_assets.try_into().unwrap_or(0.0);
        let borrow_assets: f64 = market.total_borrow_assets.try_into().unwrap_or(0.0);
        let total_liquidity = supply_assets + borrow_assets;
        
        if total_liquidity < 1_000_000.0 {
            70u8 // Very low liquidity
        } else if total_liquidity < 10_000_000.0 {
            50u8 // Low liquidity
        } else if total_liquidity < 100_000_000.0 {
            30u8 // Medium liquidity
        } else {
            15u8 // High liquidity
        }
    }

    fn assess_volatility_risk(&self, market: &MorphoMarket) -> u8 {
        // Combine volatility risks from both tokens
        let loan_vol_risk = match market.loan_token_symbol.to_uppercase().as_str() {
            "USDC" | "USDT" | "DAI" => 10u8,
            "WETH" | "ETH" => 40u8,
            "WBTC" => 45u8,
            _ => 70u8,
        };

        let collateral_vol_risk = match market.collateral_token_symbol.to_uppercase().as_str() {
            "WETH" | "ETH" => 40u8,
            "WSTETH" | "RETH" | "CBETH" => 45u8, // Slightly higher due to staking risk
            "WBTC" => 45u8,
            _ => 70u8,
        };

        ((loan_vol_risk + collateral_vol_risk) / 2).min(85)
    }

    fn estimate_time_to_liquidation(&self, position: &MorphoUserPosition) -> Option<f64> {
        if position.health_factor.is_infinite() || position.health_factor <= 1.0 {
            return None;
        }

        let current_ltv = position.ltv / 100.0;
        let liquidation_ltv = position.liquidation_ltv / 100.0;
        let ltv_buffer = liquidation_ltv - current_ltv;

        if ltv_buffer <= 0.0 || position.market.borrow_rate <= 0.0 {
            return None;
        }

        // Estimate based on interest accrual
        let daily_rate = position.market.borrow_rate / 365.0 / 100.0;
        let days_to_liquidation = ltv_buffer / (current_ltv * daily_rate);
        
        Some(days_to_liquidation * 24.0) // Convert to hours
    }

    fn calculate_price_drop_impact(&self, account: &MorphoAccountSummary, drop_percentage: f64) -> u8 {
        // Estimate how many positions would become unhealthy with the price drop
        let mut affected_positions = 0;
        
        for position in &account.positions {
            let new_collateral_value = position.collateral_value_usd * (1.0 - drop_percentage);
            let new_ltv = if new_collateral_value > 0.0 {
                position.borrow_value_usd / new_collateral_value * 100.0
            } else {
                f64::INFINITY
            };
            
            if new_ltv >= position.liquidation_ltv {
                affected_positions += 1;
            }
        }

        let impact_ratio = affected_positions as f64 / account.positions.len().max(1) as f64;
        (impact_ratio * 100.0) as u8
    }

    fn estimate_loss_from_price_drop(&self, account: &MorphoAccountSummary, drop_percentage: f64) -> f64 {
        // Simplified loss estimation - would be more complex in practice
        let total_collateral = account.total_collateral_value_usd;
        let potential_liquidation_loss = total_collateral * drop_percentage * 0.1; // 10% liquidation penalty
        
        potential_liquidation_loss
    }

    /// Add historical snapshot for trend analysis
    pub fn add_historical_snapshot(&mut self, account: MorphoAccountSummary) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let snapshot = HistoricalSnapshot {
            timestamp,
            account_summary: account,
            market_conditions: HashMap::new(), // Would be populated with market data
        };

        self.historical_data.push(snapshot);
        
        // Keep only last 30 days of data
        let cutoff_time = timestamp - (30 * 24 * 60 * 60);
        self.historical_data.retain(|s| s.timestamp > cutoff_time);
    }

    /// Generate a comprehensive risk report
    pub fn generate_risk_report(&self, account: &MorphoAccountSummary) -> serde_json::Value {
        let portfolio_metrics = self.analyze_portfolio_risk(account);
        let alerts = self.generate_risk_alerts(account);
        let scenarios = self.generate_risk_scenarios(account);
        
        let market_analyses: Vec<_> = account.positions.iter()
            .map(|pos| self.analyze_market_risk(pos))
            .collect();

        let liquidation_assessments: Vec<_> = account.positions.iter()
            .map(|pos| self.assess_liquidation_risk(pos))
            .collect();

        serde_json::json!({
            "timestamp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "portfolio_metrics": portfolio_metrics,
            "risk_alerts": alerts,
            "risk_scenarios": scenarios,
            "market_analyses": market_analyses,
            "liquidation_assessments": liquidation_assessments,
            "summary": {
                "overall_risk_level": portfolio_metrics.risk_level,
                "critical_alerts": alerts.iter().filter(|a| matches!(a.severity, RiskLevel::Critical)).count(),
                "positions_at_risk": account.unhealthy_positions,
                "total_exposure_usd": account.total_supply_value_usd + account.total_collateral_value_usd + account.total_borrow_value_usd,
                "net_worth_usd": account.net_worth_usd
            }
        })
    }
}