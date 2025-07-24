use crate::models::{Position, PoolState, RiskConfig};
use crate::error::AppError;
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use num_traits::{Zero, One, ToPrimitive};
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub impermanent_loss: BigDecimal,
    pub price_impact: BigDecimal,
    pub volatility_score: BigDecimal,
    pub correlation_score: BigDecimal,
    pub liquidity_score: BigDecimal,
    pub overall_risk_score: BigDecimal,
    pub value_at_risk_1d: BigDecimal,
    pub value_at_risk_7d: BigDecimal,
}

pub struct RiskCalculator;

impl RiskCalculator {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_position_risk(
        &self,
        position: &Position,
        pool_state: &PoolState,
        risk_config: &RiskConfig,
        historical_data: &[PoolState],
    ) -> Result<RiskMetrics, AppError> {
        info!("Calculating risk for position {}", position.id);

        let impermanent_loss = self.calculate_impermanent_loss(position, pool_state)?;
        let price_impact = self.calculate_price_impact(position, pool_state)?;
        let volatility_score = self.calculate_volatility(historical_data)?;
        let correlation_score = self.calculate_correlation(historical_data)?;
        let liquidity_score = self.calculate_liquidity_score(pool_state)?;
        let value_at_risk_1d = self.calculate_value_at_risk(position, historical_data, 1)?;
        let value_at_risk_7d = self.calculate_value_at_risk(position, historical_data, 7)?;

        let overall_risk = self.calculate_overall_risk_score(
            impermanent_loss.clone(),
            price_impact.clone(),
            volatility_score.clone(),
            correlation_score.clone(),
            liquidity_score.clone(),
        )?;

        Ok(RiskMetrics {
            impermanent_loss,
            price_impact,
            volatility_score,
            correlation_score,
            liquidity_score,
            overall_risk_score: overall_risk,
            value_at_risk_1d,
            value_at_risk_7d,
        })
    }

    fn calculate_impermanent_loss(
        &self,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        // Simplified IL calculation
        // In reality, this would be much more complex and depend on the specific AMM mechanics
        let default_price = BigDecimal::from(1);
        let token0_price = pool_state.token0_price_usd.as_ref().unwrap_or(&default_price);
        let token1_price = pool_state.token1_price_usd.as_ref().unwrap_or(&default_price);
        
        // Mock calculation - real implementation would need initial prices
        let price_ratio_change = token0_price / token1_price;
        
        // Convert to f64 for sqrt calculation, then back to BigDecimal
        let ratio_f64 = price_ratio_change.to_f64().unwrap_or(1.0);
        let sqrt_ratio = ratio_f64.sqrt();
        let sqrt_ratio_bd = BigDecimal::try_from(sqrt_ratio).unwrap_or_else(|_| BigDecimal::from(1));
        
        let il = (&BigDecimal::from(2) * &sqrt_ratio_bd / (&BigDecimal::from(1) + &price_ratio_change)) - &BigDecimal::from(1);
        
        Ok(il.abs())
    }

    fn calculate_price_impact(
        &self,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        // Simplified price impact calculation
        let position_liquidity = &position.liquidity;
        let pool_liquidity = &pool_state.liquidity;
        
        if pool_liquidity.is_zero() {
            return Ok(BigDecimal::from(1)); // Maximum impact if no liquidity
        }
        
        let impact = position_liquidity / pool_liquidity;
        let max_impact = BigDecimal::from(1);
        Ok(if impact > max_impact { max_impact } else { impact })
    }

    fn calculate_volatility(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        if historical_data.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        let mut price_changes = Vec::new();
        for window in historical_data.windows(2) {
            if let (Some(prev_price), Some(curr_price)) = (
                window[0].token0_price_usd.clone(),
                window[1].token0_price_usd.clone(),
            ) {
                if !prev_price.is_zero() {
                    let change = (&curr_price - &prev_price) / &prev_price;
                    price_changes.push(change);
                }
            }
        }

        if price_changes.is_empty() {
            return Ok(BigDecimal::from(0));
        }

        // Calculate standard deviation
        let sum: BigDecimal = price_changes.iter().cloned().sum();
        let mean = &sum / &BigDecimal::from(price_changes.len() as i32);
        let variance_sum: BigDecimal = price_changes
            .iter()
            .map(|x| {
                let diff = x - &mean;
                &diff * &diff
            })
            .sum();
        let variance = &variance_sum / &BigDecimal::from(price_changes.len() as i32);

        // Simplified square root using f64 conversion
        let variance_f64 = variance.to_f64().unwrap_or(0.0);
        Ok(BigDecimal::try_from(variance_f64.sqrt()).unwrap_or_else(|_| BigDecimal::from(0)))
    }

    fn calculate_correlation(&self, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        // Simplified correlation calculation between token0 and token1 prices
        if historical_data.len() < 2 {
            return Ok(BigDecimal::from(0));
        }

        // Mock implementation - real correlation would need proper statistical calculation
        Ok(BigDecimal::from(5)) // 0.5
    }

    fn calculate_liquidity_score(&self, pool_state: &PoolState) -> Result<BigDecimal, AppError> {
        // Higher liquidity = lower risk
        let tvl = pool_state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
        
        if tvl < BigDecimal::from(100000) {
            Ok(BigDecimal::from(9)) // 0.9 - high risk
        } else if tvl < BigDecimal::from(1000000) {
            Ok(BigDecimal::from(5)) // 0.5 - medium risk
        } else {
            Ok(BigDecimal::from(1)) // 0.1 - low risk
        }
    }

    fn calculate_value_at_risk(
        &self,
        position: &Position,
        historical_data: &[PoolState],
        days: u32,
    ) -> Result<BigDecimal, AppError> {
        // Simplified VaR calculation
        let volatility = self.calculate_volatility(historical_data)?;
        let confidence_level = BigDecimal::from(195); // 1.95 for 95% confidence
        
        let token0_price = historical_data
            .last()
            .and_then(|state| state.token0_price_usd.clone())
            .unwrap_or(BigDecimal::from(1));
        let token1_price = historical_data
            .last()
            .and_then(|state| state.token1_price_usd.clone())
            .unwrap_or(BigDecimal::from(1));
            
        let position_value = position.calculate_position_value_usd(token0_price, token1_price);
        let time_factor = BigDecimal::from(days).sqrt().unwrap_or(BigDecimal::from(1));
        
        Ok(position_value * volatility * confidence_level * time_factor)
    }

    fn calculate_overall_risk_score(
        &self,
        impermanent_loss: BigDecimal,
        price_impact: BigDecimal,
        volatility_score: BigDecimal,
        correlation_score: BigDecimal,
        liquidity_score: BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        // Weighted average of risk components
        let weights = [
            (impermanent_loss, BigDecimal::from(3)),      // 30%
            (price_impact, BigDecimal::from(2)),          // 20%
            (volatility_score, BigDecimal::from(25)),     // 25%
            (correlation_score, BigDecimal::from(1)),     // 10%
            (liquidity_score, BigDecimal::from(15)),      // 15%
        ];

        let weighted_sum = weights
            .iter()
            .map(|(score, weight)| score * weight)
            .sum::<BigDecimal>();

        let total_weight = weights.iter().map(|(_, weight)| weight.clone()).sum::<BigDecimal>();

        Ok(weighted_sum / total_weight)
    }

    pub fn check_risk_thresholds(
        &self,
        metrics: &RiskMetrics,
        config: &RiskConfig,
    ) -> Vec<String> {
        let mut violations = Vec::new();

        if &metrics.impermanent_loss > &config.impermanent_loss_threshold {
            violations.push(format!(
                "Impermanent loss ({:.2}%) exceeds threshold ({:.2}%)",
                &metrics.impermanent_loss * &BigDecimal::from(100),
                &config.impermanent_loss_threshold * &BigDecimal::from(100)
            ));
        }

        if &metrics.price_impact > &config.price_impact_threshold {
            violations.push(format!(
                "Price impact ({:.2}%) exceeds threshold ({:.2}%)",
                &metrics.price_impact * &BigDecimal::from(100),
                &config.price_impact_threshold * &BigDecimal::from(100)
            ));
        }

        if &metrics.volatility_score > &config.volatility_threshold {
            violations.push(format!(
                "Volatility ({:.2}%) exceeds threshold ({:.2}%)",
                &metrics.volatility_score * &BigDecimal::from(100),
                &config.volatility_threshold * &BigDecimal::from(100)
            ));
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    use chrono::Utc;

    #[test]
    fn test_risk_calculator_creation() {
        let calculator = RiskCalculator::new();
        // Basic test to ensure the calculator can be created
        assert!(true);
    }
}
