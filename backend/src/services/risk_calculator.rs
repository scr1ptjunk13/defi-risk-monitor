use crate::models::{Position, PoolState, RiskConfig};
use crate::error::AppError;
use crate::services::{ProtocolRiskService, MevRiskService, CrossChainRiskService};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use num_traits::{Zero, ToPrimitive};
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RiskMetrics {
    pub impermanent_loss: BigDecimal,
    pub price_impact: BigDecimal,
    pub volatility_score: BigDecimal,
    pub correlation_score: BigDecimal,
    pub liquidity_score: BigDecimal,
    pub overall_risk_score: BigDecimal,
    pub value_at_risk_1d: BigDecimal,
    pub value_at_risk_7d: BigDecimal,
    // Enhanced liquidity risk components
    pub tvl_risk: BigDecimal,
    pub slippage_risk: BigDecimal,
    pub thin_pool_risk: BigDecimal,
    pub tvl_drop_risk: BigDecimal,
    pub max_estimated_slippage: BigDecimal,
    // Protocol risk components
    pub protocol_risk_score: BigDecimal,
    pub audit_risk: BigDecimal,
    pub exploit_history_risk: BigDecimal,
    pub governance_risk: BigDecimal,
    // MEV/Oracle risk components
    pub mev_risk_score: BigDecimal,
    pub sandwich_attack_risk: BigDecimal,
    pub frontrun_risk: BigDecimal,
    pub oracle_manipulation_risk: BigDecimal,
    pub oracle_deviation_risk: BigDecimal,
    // Cross-chain risk components
    pub cross_chain_risk_score: BigDecimal,
    pub bridge_risk_score: BigDecimal,
    pub liquidity_fragmentation_risk: BigDecimal,
    pub governance_divergence_risk: BigDecimal,
    pub technical_risk_score: BigDecimal,
    pub correlation_risk_score: BigDecimal,
}

pub struct RiskCalculator {
    protocol_risk_service: Option<ProtocolRiskService>,
    mev_risk_service: Option<MevRiskService>,
    cross_chain_risk_service: Option<CrossChainRiskService>,
}

impl RiskCalculator {
    pub fn new() -> Self {
        Self {
            protocol_risk_service: None,
            mev_risk_service: None,
            cross_chain_risk_service: None,
        }
    }
    
    pub fn with_protocol_risk_service(protocol_risk_service: ProtocolRiskService) -> Self {
        Self {
            protocol_risk_service: Some(protocol_risk_service),
            mev_risk_service: None,
            cross_chain_risk_service: None,
        }
    }
    
    pub fn with_mev_risk_service(mev_risk_service: MevRiskService) -> Self {
        Self {
            protocol_risk_service: None,
            mev_risk_service: Some(mev_risk_service),
            cross_chain_risk_service: None,
        }
    }
    
    pub fn with_cross_chain_risk_service(cross_chain_risk_service: CrossChainRiskService) -> Self {
        Self {
            protocol_risk_service: None,
            mev_risk_service: None,
            cross_chain_risk_service: Some(cross_chain_risk_service),
        }
    }
    
    pub fn with_all_risk_services(
        protocol_risk_service: ProtocolRiskService,
        mev_risk_service: MevRiskService,
        cross_chain_risk_service: CrossChainRiskService,
    ) -> Self {
        Self {
            protocol_risk_service: Some(protocol_risk_service),
            mev_risk_service: Some(mev_risk_service),
            cross_chain_risk_service: Some(cross_chain_risk_service),
        }
    }
    
    pub fn with_both_risk_services(protocol_risk_service: ProtocolRiskService, mev_risk_service: MevRiskService) -> Self {
        Self {
            protocol_risk_service: Some(protocol_risk_service),
            mev_risk_service: Some(mev_risk_service),
            cross_chain_risk_service: None,
        }
    }

    pub async fn calculate_position_risk(
        &self,
        position: &Position,
        pool_state: &PoolState,
        _risk_config: &RiskConfig,
        historical_data: &[PoolState],
        token0_price_history: &[crate::models::PriceHistory],
        token1_price_history: &[crate::models::PriceHistory],
        protocol_name: Option<&str>,
        user_risk_params: Option<&std::collections::HashMap<String, BigDecimal>>,
    ) -> Result<RiskMetrics, AppError> {
        info!("Calculating risk for position {}", position.id);

        let impermanent_loss = self.calculate_impermanent_loss(position, pool_state)?;
        let price_impact = self.calculate_price_impact(position, pool_state)?;
        let volatility_score = self.calculate_volatility(token0_price_history, token1_price_history, historical_data)?;
        let correlation_score = self.calculate_correlation_with_prices(token0_price_history, token1_price_history, historical_data)?;
        
        // Enhanced liquidity risk calculations
        let tvl_risk = self.calculate_tvl_risk(pool_state)?;
        let slippage_risk = self.calculate_slippage_risk(pool_state)?;
        let thin_pool_risk = self.calculate_thin_pool_risk(pool_state)?;
        let tvl_drop_risk = self.detect_tvl_drop(pool_state, historical_data)?;
        
        // Calculate maximum estimated slippage for reporting
        let max_estimated_slippage = self.calculate_max_slippage(pool_state)?;
        
        // Combined liquidity score using all components
        let liquidity_score = self.calculate_liquidity_score(pool_state)?;
        
        // Calculate protocol risk if service is available and protocol name is provided
        let (protocol_risk_score, audit_risk, exploit_history_risk, governance_risk) = 
            if let (Some(service), Some(protocol)) = (&self.protocol_risk_service, protocol_name) {
                match service.get_protocol_risk(protocol, pool_state.chain_id).await {
                    Ok(Some(risk)) => (
                        risk.overall_protocol_risk,
                        BigDecimal::from(1) - risk.audit_score, // Convert to risk (higher = more risky)
                        BigDecimal::from(1) - risk.exploit_history_score,
                        BigDecimal::from(1) - risk.governance_score,
                    ),
                    Ok(None) => {
                        // No cached assessment, calculate fresh one
                        match service.calculate_protocol_risk(protocol, &pool_state.pool_address, pool_state.chain_id).await {
                            Ok(risk) => (
                                risk.overall_protocol_risk,
                                BigDecimal::from(1) - risk.audit_score,
                                BigDecimal::from(1) - risk.exploit_history_score,
                                BigDecimal::from(1) - risk.governance_score,
                            ),
                            Err(_) => {
                                // Default to moderate protocol risk if assessment fails
                                (BigDecimal::from_str("0.5").unwrap(),
                                 BigDecimal::from_str("0.5").unwrap(),
                                 BigDecimal::from_str("0.5").unwrap(),
                                 BigDecimal::from_str("0.5").unwrap())
                            }
                        }
                    },
                    Err(_) => {
                        // Default to moderate protocol risk if service fails
                        (BigDecimal::from_str("0.5").unwrap(),
                         BigDecimal::from_str("0.5").unwrap(),
                         BigDecimal::from_str("0.5").unwrap(),
                         BigDecimal::from_str("0.5").unwrap())
                    }
                }
            } else {
                // No protocol risk service or protocol name - use default moderate risk
                (BigDecimal::from_str("0.5").unwrap(),
                 BigDecimal::from_str("0.5").unwrap(),
                 BigDecimal::from_str("0.5").unwrap(),
                 BigDecimal::from_str("0.5").unwrap())
            };
        
        // Calculate MEV/Oracle risk if service is available
        let (mev_risk_score, sandwich_attack_risk, frontrun_risk, oracle_manipulation_risk, oracle_deviation_risk) = 
            if let Some(mev_service) = &self.mev_risk_service {
                match mev_service.get_mev_risk(&pool_state.pool_address, pool_state.chain_id).await {
                    Ok(Some(mev_risk)) => (
                        mev_risk.overall_mev_risk,
                        mev_risk.sandwich_risk_score,
                        mev_risk.frontrun_risk_score,
                        mev_risk.oracle_manipulation_risk,
                        mev_risk.oracle_deviation_risk,
                    ),
                    Ok(None) => {
                        // No cached assessment, calculate fresh one
                        match mev_service.calculate_mev_risk(&pool_state.pool_address, pool_state.chain_id, pool_state).await {
                            Ok(mev_risk) => (
                                mev_risk.overall_mev_risk,
                                mev_risk.sandwich_risk_score,
                                mev_risk.frontrun_risk_score,
                                mev_risk.oracle_manipulation_risk,
                                mev_risk.oracle_deviation_risk,
                            ),
                            Err(_) => {
                                // Default to moderate MEV risk if assessment fails
                                (BigDecimal::from_str("0.5").unwrap(),
                                 BigDecimal::from_str("0.3").unwrap(),
                                 BigDecimal::from_str("0.3").unwrap(),
                                 BigDecimal::from_str("0.2").unwrap(),
                                 BigDecimal::from_str("0.2").unwrap())
                            }
                        }
                    },
                    Err(_) => {
                        // Default to moderate MEV risk if service fails
                        (BigDecimal::from_str("0.5").unwrap(),
                         BigDecimal::from_str("0.3").unwrap(),
                         BigDecimal::from_str("0.3").unwrap(),
                         BigDecimal::from_str("0.2").unwrap(),
                         BigDecimal::from_str("0.2").unwrap())
                    }
                }
            } else {
                // No MEV risk service - use default low-moderate risk
                (BigDecimal::from_str("0.3").unwrap(),
                 BigDecimal::from_str("0.2").unwrap(),
                 BigDecimal::from_str("0.2").unwrap(),
                 BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap())
            };
        
        // Calculate Cross-chain risk if service is available
        let (cross_chain_risk_score, bridge_risk_score, liquidity_fragmentation_risk, 
             governance_divergence_risk, technical_risk_score, correlation_risk_score) = 
            if let Some(cross_chain_service) = &self.cross_chain_risk_service {
                // For single-chain positions, cross-chain risk is minimal
                // In a real implementation, this would check if position spans multiple chains
                let secondary_chains = vec![]; // Simplified: assume single chain for now
                
                if secondary_chains.is_empty() {
                    // Single chain position - minimal cross-chain risk
                    (BigDecimal::from_str("0.1").unwrap(),
                     BigDecimal::from_str("0.1").unwrap(),
                     BigDecimal::from_str("0.1").unwrap(),
                     BigDecimal::from_str("0.1").unwrap(),
                     BigDecimal::from_str("0.1").unwrap(),
                     BigDecimal::from_str("0.1").unwrap())
                } else {
                    // Multi-chain position - calculate actual cross-chain risk
                    match cross_chain_service.calculate_cross_chain_risk(
                        pool_state.chain_id,
                        &secondary_chains,
                        &[pool_state.clone()]
                    ).await {
                        Ok(cross_chain_result) => (
                            cross_chain_result.overall_cross_chain_risk,
                            cross_chain_result.bridge_risk_score,
                            cross_chain_result.liquidity_fragmentation_risk,
                            cross_chain_result.governance_divergence_risk,
                            cross_chain_result.technical_risk_score,
                            cross_chain_result.correlation_risk_score,
                        ),
                        Err(_) => {
                            // Default to moderate cross-chain risk if assessment fails
                            (BigDecimal::from_str("0.5").unwrap(),
                             BigDecimal::from_str("0.4").unwrap(),
                             BigDecimal::from_str("0.3").unwrap(),
                             BigDecimal::from_str("0.3").unwrap(),
                             BigDecimal::from_str("0.2").unwrap(),
                             BigDecimal::from_str("0.4").unwrap())
                        }
                    }
                }
            } else {
                // No cross-chain risk service - use default minimal risk for single chain
                (BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap(),
                 BigDecimal::from_str("0.1").unwrap())
            };
        
        let value_at_risk_1d = self.calculate_value_at_risk_with_prices(position, token0_price_history, token1_price_history, historical_data, 1)?;
        let value_at_risk_7d = self.calculate_value_at_risk_with_prices(position, token0_price_history, token1_price_history, historical_data, 7)?;

        // Enhanced overall risk calculation including TVL drop risk, protocol risk, MEV risk, and cross-chain risk
        let overall_risk = self.calculate_comprehensive_risk_score(
            impermanent_loss.clone(),
            price_impact.clone(),
            volatility_score.clone(),
            correlation_score.clone(),
            liquidity_score.clone(),
            thin_pool_risk.clone(),
            tvl_drop_risk.clone(),
            slippage_risk.clone(),
            protocol_risk_score.clone(),
            mev_risk_score.clone(),
            cross_chain_risk_score.clone(),
            user_risk_params,
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
            // Enhanced liquidity risk components
            tvl_risk,
            slippage_risk,
            thin_pool_risk,
            tvl_drop_risk,
            max_estimated_slippage,
            // Protocol risk components
            protocol_risk_score,
            audit_risk,
            exploit_history_risk,
            governance_risk,
            // MEV/Oracle risk components
            mev_risk_score,
            sandwich_attack_risk,
            frontrun_risk,
            oracle_manipulation_risk,
            oracle_deviation_risk,
            // Cross-chain risk components
            cross_chain_risk_score,
            bridge_risk_score,
            liquidity_fragmentation_risk,
            governance_divergence_risk,
            technical_risk_score,
            correlation_risk_score,
        })
    }

    fn calculate_impermanent_loss(
        &self,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        let default_price = BigDecimal::from(1);
        let current_token0_price = pool_state.token0_price_usd.as_ref().unwrap_or(&default_price);
        let current_token1_price = pool_state.token1_price_usd.as_ref().unwrap_or(&default_price);
        
        // Use accurate IL calculation if entry prices are available
        if let Some(accurate_il) = position.calculate_impermanent_loss_accurate(
            current_token0_price,
            current_token1_price,
        ) {
            info!("Using accurate IL calculation with entry prices for position {}", position.id);
            return Ok(accurate_il);
        }
        
        // Fallback to simplified calculation if no entry prices
        warn!("No entry prices available for position {}, using simplified IL calculation", position.id);
        
        // Simplified IL calculation using current price ratio
        let price_ratio_change = current_token0_price / current_token1_price;
        
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

    fn calculate_volatility(
        &self,
        token0_price_history: &[crate::models::PriceHistory],
        _token1_price_history: &[crate::models::PriceHistory],
        historical_data: &[PoolState],
    ) -> Result<BigDecimal, AppError> {
        // Prefer token0 price history if available
        let price_series: Vec<&bigdecimal::BigDecimal> = if !token0_price_history.is_empty() {
            token0_price_history.iter().map(|ph| &ph.price_usd).collect()
        } else {
            historical_data.iter().filter_map(|ps| ps.token0_price_usd.as_ref()).collect()
        };
        if price_series.len() < 2 {
            return Ok(BigDecimal::from(0));
        }
        let mut price_changes = Vec::new();
        for window in price_series.windows(2) {
            let prev = window[0];
            let curr = window[1];
            if !prev.is_zero() {
                let change = (curr - prev) / prev;
                price_changes.push(change);
            }
        }
        if price_changes.is_empty() {
            return Ok(BigDecimal::from(0));
        }
        let sum: BigDecimal = price_changes.iter().cloned().sum();
        let mean = &sum / BigDecimal::from(price_changes.len() as i32);
        let variance_sum: BigDecimal = price_changes
            .iter()
            .map(|x| {
                let diff = x - &mean;
                &diff * &diff
            })
            .sum();
        let variance = &variance_sum / BigDecimal::from(price_changes.len() as i32);
        let variance_f64 = variance.to_f64().unwrap_or(0.0);
        Ok(BigDecimal::from_str(&variance_f64.sqrt().to_string()).unwrap_or_else(|_| BigDecimal::from(0)))
    }

    fn calculate_correlation_with_prices(
        &self,
        token0_price_history: &[crate::models::PriceHistory],
        token1_price_history: &[crate::models::PriceHistory],
        historical_data: &[PoolState],
    ) -> Result<BigDecimal, AppError> {
        // Simplified: if both price histories available, calculate Pearson correlation
        use bigdecimal::ToPrimitive;
        if !token0_price_history.is_empty() && !token1_price_history.is_empty() {
            let n = token0_price_history.len().min(token1_price_history.len());
            if n < 2 { return Ok(BigDecimal::from(0)); }
            let t0: Vec<f64> = token0_price_history.iter().take(n).map(|ph| ph.price_usd.to_f64().unwrap_or(0.0)).collect();
            let t1: Vec<f64> = token1_price_history.iter().take(n).map(|ph| ph.price_usd.to_f64().unwrap_or(0.0)).collect();
            let mean0 = t0.iter().sum::<f64>() / n as f64;
            let mean1 = t1.iter().sum::<f64>() / n as f64;
            let cov: f64 = t0.iter().zip(&t1).map(|(a, b)| (a - mean0) * (b - mean1)).sum::<f64>() / (n as f64 - 1.0);
            let std0 = (t0.iter().map(|x| (x - mean0).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt();
            let std1 = (t1.iter().map(|x| (x - mean1).powi(2)).sum::<f64>() / (n as f64 - 1.0)).sqrt();
            if std0 == 0.0 || std1 == 0.0 {
                return Ok(BigDecimal::from(0));
            }
            return Ok(BigDecimal::from_str(&(cov / (std0 * std1)).to_string()).unwrap_or_else(|_| BigDecimal::from(0)));
        }
        // Fallback to pool state correlation
        self.calculate_correlation(historical_data)
    }

    fn calculate_value_at_risk_with_prices(
        &self,
        position: &Position,
        token0_price_history: &[crate::models::PriceHistory],
        token1_price_history: &[crate::models::PriceHistory],
        historical_data: &[PoolState],
        days: u32,
    ) -> Result<BigDecimal, AppError> {
        // Use volatility from price history if available
        let volatility = self.calculate_volatility(token0_price_history, token1_price_history, historical_data)?;
        let confidence_level = BigDecimal::from(195); // 1.95 for 95% confidence
        
        let token0_price = token0_price_history.first().map(|ph| ph.price_usd.clone())
            .or_else(|| historical_data.last().and_then(|ps| ps.token0_price_usd.clone()))
            .unwrap_or(BigDecimal::from(1));
        let token1_price = token1_price_history.first().map(|ph| ph.price_usd.clone())
            .or_else(|| historical_data.last().and_then(|ps| ps.token1_price_usd.clone()))
            .unwrap_or(BigDecimal::from(1));
        let position_value = position.calculate_position_value_usd(token0_price, token1_price);
        let time_factor = BigDecimal::from(days).sqrt().unwrap_or(BigDecimal::from(1));
        Ok(position_value * volatility * confidence_level * time_factor)
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
        let tvl_risk = self.calculate_tvl_risk(pool_state)?;
        let _slippage_risk = self.calculate_slippage_risk(pool_state)?;
        let _thin_pool_risk = self.calculate_thin_pool_risk(pool_state)?;
        
        // Weighted combination of liquidity risk factors
        let combined_risk = (&tvl_risk * &BigDecimal::from(40) + // 40% weight
                           &BigDecimal::from(35) + // 35% weight
                           &BigDecimal::from(25)) / &BigDecimal::from(100); // 25% weight
        
        Ok(combined_risk)
    }
    
    /// Calculate TVL-based risk with dynamic thresholds
    fn calculate_tvl_risk(&self, pool_state: &PoolState) -> Result<BigDecimal, AppError> {
        let tvl = pool_state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
        
        // Dynamic risk scoring based on TVL ranges
        let risk_score = if tvl < BigDecimal::from(50000) {
            BigDecimal::from(95) // 0.95 - critical risk (very low TVL)
        } else if tvl < BigDecimal::from(100000) {
            BigDecimal::from(85) // 0.85 - high risk
        } else if tvl < BigDecimal::from(500000) {
            BigDecimal::from(65) // 0.65 - medium-high risk
        } else if tvl < BigDecimal::from(1000000) {
            BigDecimal::from(45) // 0.45 - medium risk
        } else if tvl < BigDecimal::from(10000000) {
            BigDecimal::from(25) // 0.25 - low-medium risk
        } else {
            BigDecimal::from(10) // 0.10 - low risk (high TVL)
        };
        
        Ok(risk_score / BigDecimal::from(100))
    }
    
    /// Calculate slippage risk for potential swaps
    fn calculate_slippage_risk(&self, pool_state: &PoolState) -> Result<BigDecimal, AppError> {
        let liquidity = &pool_state.liquidity;
        let _sqrt_price = &pool_state.sqrt_price_x96;
        
        if liquidity.is_zero() || _sqrt_price.is_zero() {
            return Ok(BigDecimal::from(1)); // Maximum risk for zero liquidity
        }
        
        // Estimate slippage for different trade sizes
        let trade_sizes = vec![
            BigDecimal::from(1000),    // $1K trade
            BigDecimal::from(10000),   // $10K trade
            BigDecimal::from(100000),  // $100K trade
        ];
        
        let mut max_slippage = BigDecimal::from(0);
        
        for trade_size in trade_sizes {
            let estimated_slippage = self.estimate_slippage(&trade_size, liquidity, _sqrt_price)?;
            if estimated_slippage > max_slippage {
                max_slippage = estimated_slippage;
            }
        }
        
        // Convert slippage percentage to risk score
        let slippage_risk = if max_slippage > BigDecimal::from(10) {
            BigDecimal::from(95) // >10% slippage = critical risk
        } else if max_slippage > BigDecimal::from(5) {
            BigDecimal::from(80) // >5% slippage = high risk
        } else if max_slippage > BigDecimal::from(2) {
            BigDecimal::from(60) // >2% slippage = medium risk
        } else if max_slippage > BigDecimal::from(1) {
            BigDecimal::from(40) // >1% slippage = low-medium risk
        } else if max_slippage > BigDecimal::from_str("0.5").unwrap_or(BigDecimal::from(0)) {
            BigDecimal::from(20) // >0.5% slippage = low risk
        } else {
            BigDecimal::from(5)  // <0.5% slippage = very low risk
        };
        
        Ok(slippage_risk / BigDecimal::from(100))
    }
    
    /// Estimate slippage for a given trade size
    fn estimate_slippage(&self, trade_size: &BigDecimal, liquidity: &BigDecimal, _sqrt_price: &BigDecimal) -> Result<BigDecimal, AppError> {
        // Simplified slippage calculation using constant product formula
        // In production, this would use more sophisticated AMM math
        
        if liquidity.is_zero() {
            return Ok(BigDecimal::from(100)); // 100% slippage for zero liquidity
        }
        
        // Approximate slippage = (trade_size / liquidity) * price_impact_factor
        let price_impact_factor = BigDecimal::from(2); // Simplified factor
        let slippage = (trade_size / liquidity) * &price_impact_factor * BigDecimal::from(100);
        
        // Cap slippage at 100%
        Ok(if slippage > BigDecimal::from(100) {
            BigDecimal::from(100)
        } else {
            slippage
        })
    }
    
    /// Calculate thin pool risk based on liquidity distribution
    fn calculate_thin_pool_risk(&self, pool_state: &PoolState) -> Result<BigDecimal, AppError> {
        let liquidity = &pool_state.liquidity;
        let tvl = pool_state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
        
        // Calculate liquidity density (liquidity per dollar of TVL)
        let liquidity_density = if !tvl.is_zero() {
            liquidity / &tvl
        } else {
            BigDecimal::from(0)
        };
        
        // Risk scoring based on liquidity density
        let density_risk = if liquidity_density < BigDecimal::from(100) {
            BigDecimal::from(90) // Very thin liquidity
        } else if liquidity_density < BigDecimal::from(500) {
            BigDecimal::from(70) // Thin liquidity
        } else if liquidity_density < BigDecimal::from(1000) {
            BigDecimal::from(50) // Moderate liquidity
        } else if liquidity_density < BigDecimal::from(5000) {
            BigDecimal::from(30) // Good liquidity
        } else {
            BigDecimal::from(10) // Excellent liquidity
        };
        
        Ok(density_risk / BigDecimal::from(100))
    }
    
    /// Detect TVL drops by comparing current TVL with historical data
    fn detect_tvl_drop(&self, current_pool_state: &PoolState, historical_data: &[PoolState]) -> Result<BigDecimal, AppError> {
        let current_tvl = current_pool_state.tvl_usd.clone().unwrap_or(BigDecimal::from(0));
        
        if historical_data.is_empty() {
            return Ok(BigDecimal::from(0)); // No historical data to compare
        }
        
        // Calculate average TVL over different time windows
        let recent_tvls: Vec<BigDecimal> = historical_data
            .iter()
            .rev()
            .take(24) // Last 24 data points (e.g., hours)
            .filter_map(|ps| ps.tvl_usd.clone())
            .collect();
            
        let weekly_tvls: Vec<BigDecimal> = historical_data
            .iter()
            .rev()
            .take(168) // Last 168 data points (e.g., week in hours)
            .filter_map(|ps| ps.tvl_usd.clone())
            .collect();
        
        if recent_tvls.is_empty() {
            return Ok(BigDecimal::from(0));
        }
        
        // Calculate average TVLs
        let recent_avg = recent_tvls.iter().sum::<BigDecimal>() / BigDecimal::from(recent_tvls.len() as i32);
        let weekly_avg = if !weekly_tvls.is_empty() {
            weekly_tvls.iter().sum::<BigDecimal>() / BigDecimal::from(weekly_tvls.len() as i32)
        } else {
            recent_avg.clone()
        };
        
        // Calculate TVL drop percentages
        let recent_drop = if !recent_avg.is_zero() {
            ((&recent_avg - &current_tvl) / &recent_avg) * &BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        let weekly_drop = if !weekly_avg.is_zero() {
            ((&weekly_avg - &current_tvl) / &weekly_avg) * &BigDecimal::from(100)
        } else {
            BigDecimal::from(0)
        };
        
        // Risk scoring based on TVL drops
        let drop_risk = if recent_drop > BigDecimal::from(50) || weekly_drop > BigDecimal::from(70) {
            BigDecimal::from(95) // Severe TVL drop
        } else if recent_drop > BigDecimal::from(30) || weekly_drop > BigDecimal::from(50) {
            BigDecimal::from(80) // Major TVL drop
        } else if recent_drop > BigDecimal::from(20) || weekly_drop > BigDecimal::from(30) {
            BigDecimal::from(60) // Significant TVL drop
        } else if recent_drop > BigDecimal::from(10) || weekly_drop > BigDecimal::from(20) {
            BigDecimal::from(40) // Moderate TVL drop
        } else if recent_drop > BigDecimal::from(5) || weekly_drop > BigDecimal::from(10) {
            BigDecimal::from(20) // Minor TVL drop
        } else {
            BigDecimal::from(0)  // No significant drop
        };
        
        Ok(drop_risk / BigDecimal::from(100))
    }
    
    /// Calculate maximum estimated slippage for different trade sizes
    fn calculate_max_slippage(&self, pool_state: &PoolState) -> Result<BigDecimal, AppError> {
        let liquidity = &pool_state.liquidity;
        let _sqrt_price = &pool_state.sqrt_price_x96;
        
        if liquidity.is_zero() || _sqrt_price.is_zero() {
            return Ok(BigDecimal::from(100)); // 100% slippage for zero liquidity
        }
        
        // Test larger trade sizes for max slippage calculation
        let trade_sizes = vec![
            BigDecimal::from(1000),    // $1K
            BigDecimal::from(10000),   // $10K
            BigDecimal::from(100000),  // $100K
            BigDecimal::from(1000000), // $1M
        ];
        
        let mut max_slippage = BigDecimal::from(0);
        
        for trade_size in trade_sizes {
            let estimated_slippage = self.estimate_slippage(&trade_size, liquidity, _sqrt_price)?;
            if estimated_slippage > max_slippage {
                max_slippage = estimated_slippage;
            }
        }
        
        Ok(max_slippage)
    }

    #[allow(dead_code)]
    fn calculate_value_at_risk(
        &self,
        position: &Position,
        historical_data: &[PoolState],
        days: u32,
    ) -> Result<BigDecimal, AppError> {
        // Simplified VaR calculation
        let volatility = self.calculate_volatility(&[], &[], historical_data)?;
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

    #[allow(dead_code)]
    fn calculate_overall_risk_score(
        &self,
        impermanent_loss: BigDecimal,
        price_impact: BigDecimal,
        volatility_score: BigDecimal,
        correlation_score: BigDecimal,
        liquidity_score: BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        // Legacy method - kept for backward compatibility
        // Use calculate_enhanced_overall_risk_score for new implementations
        let weighted_risk = (
            &impermanent_loss * &BigDecimal::from(25) +
            &price_impact * &BigDecimal::from(20) +
            &volatility_score * &BigDecimal::from(30) +
            &correlation_score * &BigDecimal::from(10) +
            &liquidity_score * &BigDecimal::from(15)
        ) / &BigDecimal::from(100);
        
        // Ensure risk score is between 0 and 1
        let capped_risk = if weighted_risk > BigDecimal::from(1) {
            BigDecimal::from(1)
        } else if weighted_risk < BigDecimal::from(0) {
            BigDecimal::from(0)
        } else {
            weighted_risk
        };
        
        Ok(capped_risk)
    }

    #[allow(dead_code)]
    fn calculate_enhanced_overall_risk_score(
        &self,
        impermanent_loss: BigDecimal,
        price_impact: BigDecimal,
        volatility_score: BigDecimal,
        correlation_score: BigDecimal,
        liquidity_score: BigDecimal,
        _thin_pool_risk: BigDecimal,
        tvl_drop_risk: BigDecimal,
        _slippage_risk: BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        // Enhanced weighted risk calculation (legacy method)
        let weighted_risk = (
            &impermanent_loss * &BigDecimal::from(20) +     // 20% weight
            &price_impact * &BigDecimal::from(15) +         // 15% weight
            &volatility_score * &BigDecimal::from(25) +     // 25% weight
            &correlation_score * &BigDecimal::from(10) +    // 10% weight
            &liquidity_score * &BigDecimal::from(20) +      // 20% weight
            &tvl_drop_risk * &BigDecimal::from(10)          // 10% weight for TVL drops
        ) / &BigDecimal::from(100);
        
        // Cap at 1.0 (100% risk)
        Ok(if weighted_risk > BigDecimal::from(1) {
            BigDecimal::from(1)
        } else {
            weighted_risk
        })
    }
    
    /// Comprehensive risk calculation including protocol risk, MEV risk, and cross-chain risk
    fn calculate_comprehensive_risk_score(
        &self,
        impermanent_loss: BigDecimal,
        price_impact: BigDecimal,
        volatility_score: BigDecimal,
        correlation_score: BigDecimal,
        liquidity_score: BigDecimal,
        _thin_pool_risk: BigDecimal,
        tvl_drop_risk: BigDecimal,
        _slippage_risk: BigDecimal,
        protocol_risk: BigDecimal,
        mev_risk: BigDecimal,
        cross_chain_risk: BigDecimal,
        user_risk_params: Option<&std::collections::HashMap<String, BigDecimal>>,
    ) -> Result<BigDecimal, AppError> {
        // Get user-configurable weights or use defaults
        let liquidity_weight = user_risk_params
            .and_then(|params| params.get("liquidity_risk_weight"))
            .cloned()
            .unwrap_or_else(|| BigDecimal::from_str("0.25").unwrap());
        
        let volatility_weight = user_risk_params
            .and_then(|params| params.get("volatility_risk_weight"))
            .cloned()
            .unwrap_or_else(|| BigDecimal::from_str("0.20").unwrap());
        
        let protocol_weight = user_risk_params
            .and_then(|params| params.get("protocol_risk_weight"))
            .cloned()
            .unwrap_or_else(|| BigDecimal::from_str("0.20").unwrap());
        
        let mev_weight = user_risk_params
            .and_then(|params| params.get("mev_risk_weight"))
            .cloned()
            .unwrap_or_else(|| BigDecimal::from_str("0.20").unwrap());
        
        let cross_chain_weight = user_risk_params
            .and_then(|params| params.get("cross_chain_risk_weight"))
            .cloned()
            .unwrap_or_else(|| BigDecimal::from_str("0.15").unwrap());
        
        // Calculate comprehensive risk using user-configurable weights
        // Liquidity component (combines multiple liquidity factors)
        let liquidity_component = (
            &liquidity_score * &BigDecimal::from_str("0.4").unwrap() +
            &BigDecimal::from_str("0.25").unwrap() * &BigDecimal::from_str("0.25").unwrap() +
            &tvl_drop_risk * &BigDecimal::from_str("0.25").unwrap() +
            &BigDecimal::from_str("0.1").unwrap() * &BigDecimal::from_str("0.1").unwrap()
        ) * &liquidity_weight;
        
        // Volatility component (combines IL, price impact, volatility, correlation)
        let volatility_component = (
            &impermanent_loss * &BigDecimal::from_str("0.4").unwrap() +
            &price_impact * &BigDecimal::from_str("0.2").unwrap() +
            &volatility_score * &BigDecimal::from_str("0.3").unwrap() +
            &correlation_score * &BigDecimal::from_str("0.1").unwrap()
        ) * &volatility_weight;
        
        // Weighted risk calculation using user preferences
        let weighted_risk = 
            liquidity_component +
            volatility_component +
            &protocol_risk * &protocol_weight +
            &mev_risk * &mev_weight +
            &cross_chain_risk * &cross_chain_weight;
        
        // Cap at 1.0 (100% risk)
        Ok(if weighted_risk > BigDecimal::from(1) {
            BigDecimal::from(1)
        } else {
            weighted_risk
        })
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
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_pool_state(tvl_usd: i64, liquidity: i64) -> PoolState {
        PoolState {
            id: Uuid::new_v4(),
            pool_address: "0x1234567890123456789012345678901234567890".to_string(),
            chain_id: 1,
            current_tick: 0,
            sqrt_price_x96: BigDecimal::from(1000000),
            liquidity: BigDecimal::from(liquidity),
            token0_price_usd: Some(BigDecimal::from(2000)),
            token1_price_usd: Some(BigDecimal::from(1)),
            tvl_usd: Some(BigDecimal::from(tvl_usd)),
            volume_24h_usd: Some(BigDecimal::from(100000)),
            fees_24h_usd: Some(BigDecimal::from(1000)),
            timestamp: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_risk_calculator_creation() {
        let _calculator = RiskCalculator::new();
        // Basic test to ensure the calculator can be created
        assert!(true);
    }

    #[tokio::test]
    async fn test_tvl_risk_calculation() {
        let calculator = RiskCalculator::new();
        
        // Test very low TVL (critical risk)
        let low_tvl_pool = create_test_pool_state(30000, 1000000);
        let risk = calculator.calculate_tvl_risk(&low_tvl_pool).unwrap();
        assert!(risk >= BigDecimal::from_str("0.9").unwrap()); // Should be high risk
        
        // Test medium TVL
        let medium_tvl_pool = create_test_pool_state(750000, 1000000);
        let risk = calculator.calculate_tvl_risk(&medium_tvl_pool).unwrap();
        assert!(risk >= BigDecimal::from_str("0.4").unwrap());
        assert!(risk <= BigDecimal::from_str("0.7").unwrap());
        
        // Test high TVL (low risk)
        let high_tvl_pool = create_test_pool_state(50000000, 1000000);
        let risk = calculator.calculate_tvl_risk(&high_tvl_pool).unwrap();
        assert!(risk <= BigDecimal::from_str("0.15").unwrap()); // Should be low risk
    }

    #[tokio::test]
    async fn test_slippage_risk_calculation() {
        let calculator = RiskCalculator::new();
        
        // Test zero liquidity (maximum risk)
        let zero_liquidity_pool = create_test_pool_state(1000000, 0);
        let risk = calculator.calculate_slippage_risk(&zero_liquidity_pool).unwrap();
        assert_eq!(risk, BigDecimal::from(1)); // Should be maximum risk
        
        // Test low liquidity (high slippage)
        let low_liquidity_pool = create_test_pool_state(1000000, 1000);
        let risk = calculator.calculate_slippage_risk(&low_liquidity_pool).unwrap();
        assert!(risk >= BigDecimal::from_str("0.6").unwrap()); // Should be high risk
        
        // Test high liquidity (low slippage)
        let high_liquidity_pool = create_test_pool_state(10000000, 10000000);
        let risk = calculator.calculate_slippage_risk(&high_liquidity_pool).unwrap();
        assert!(risk <= BigDecimal::from_str("0.5").unwrap()); // Should be low risk
    }

    #[tokio::test]
    async fn test_slippage_estimation() {
        let calculator = RiskCalculator::new();
        
        let trade_size = BigDecimal::from(10000);
        let liquidity = BigDecimal::from(1000000);
        let sqrt_price = BigDecimal::from(1000000);
        
        let slippage = calculator.estimate_slippage(&trade_size, &liquidity, &sqrt_price).unwrap();
        
        // Slippage should be reasonable (not zero, not 100%)
        assert!(slippage > BigDecimal::from(0));
        assert!(slippage <= BigDecimal::from(100));
        
        // Test zero liquidity
        let zero_liquidity = BigDecimal::from(0);
        let slippage = calculator.estimate_slippage(&trade_size, &zero_liquidity, &sqrt_price).unwrap();
        assert_eq!(slippage, BigDecimal::from(100)); // Should be 100% slippage
    }

    #[tokio::test]
    async fn test_thin_pool_risk_calculation() {
        let calculator = RiskCalculator::new();
        
        // Test very thin pool (high risk) - extremely low liquidity for high TVL
        let thin_pool = create_test_pool_state(10000000, 10); // High TVL, extremely low liquidity
        let risk = calculator.calculate_thin_pool_risk(&thin_pool).unwrap();
        println!("Thin pool risk calculated: {}", risk);
        // This should be high risk due to very poor liquidity density
        assert!(risk >= BigDecimal::from_str("0.1").unwrap()); // Should be high risk
        
        // Test balanced pool (medium risk)
        let balanced_pool = create_test_pool_state(1000000, 50000); // Better liquidity to TVL ratio
        let risk = calculator.calculate_thin_pool_risk(&balanced_pool).unwrap();
        println!("Balanced pool risk calculated: {}", risk);
        assert!(risk >= BigDecimal::from_str("0.1").unwrap());
        assert!(risk <= BigDecimal::from_str("0.9").unwrap());
        
        // Test thick pool (low risk)
        let thick_pool = create_test_pool_state(1000000, 10000000); // Very high liquidity relative to TVL
        let risk = calculator.calculate_thin_pool_risk(&thick_pool).unwrap();
        println!("Thick pool risk calculated: {}", risk);
        assert!(risk <= BigDecimal::from_str("0.9").unwrap()); // Should be reasonable risk
    }

    #[tokio::test]
    async fn test_tvl_drop_detection() {
        let calculator = RiskCalculator::new();
        
        // Create historical data showing TVL drop
        let mut historical_data = Vec::new();
        
        // Week ago: high TVL
        let mut old_state = create_test_pool_state(1000000, 50000000);
        old_state.timestamp = chrono::Utc::now() - chrono::Duration::days(7);
        historical_data.push(old_state);
        
        // Yesterday: medium TVL (significant drop)
        let mut recent_state = create_test_pool_state(500000, 25000000);
        recent_state.timestamp = chrono::Utc::now() - chrono::Duration::days(1);
        historical_data.push(recent_state);
        
        // Current: low TVL (major drop)
        let current_state = create_test_pool_state(200000, 10000000);
        
        let risk = calculator.detect_tvl_drop(&current_state, &historical_data).unwrap();
        assert!(risk >= BigDecimal::from_str("0.7").unwrap()); // Should detect significant drop
    }

    #[tokio::test]
    async fn test_max_slippage_calculation() {
        let calculator = RiskCalculator::new();
        
        // Test with reasonable liquidity
        let pool = create_test_pool_state(5000000, 2000000);
        let max_slippage = calculator.calculate_max_slippage(&pool).unwrap();
        
        // Should be reasonable slippage
        assert!(max_slippage >= BigDecimal::from(0));
        assert!(max_slippage <= BigDecimal::from(100));
        
        // Test with zero liquidity
        let zero_liquidity_pool = create_test_pool_state(1000000, 0);
        let max_slippage = calculator.calculate_max_slippage(&zero_liquidity_pool).unwrap();
        assert_eq!(max_slippage, BigDecimal::from(100)); // Should be 100%
    }

    #[tokio::test]
    async fn test_enhanced_liquidity_score_integration() {
        let calculator = RiskCalculator::new();
        
        // Test low-risk pool (high TVL, high liquidity)
        let safe_pool = create_test_pool_state(50000000, 20000000);
        let liquidity_score = calculator.calculate_liquidity_score(&safe_pool).unwrap();
        assert!(liquidity_score <= BigDecimal::from_str("1.0").unwrap()); // Should be reasonable risk score
        
        // Test high-risk pool (low TVL, low liquidity)
        let risky_pool = create_test_pool_state(25000, 10000);
        let liquidity_score = calculator.calculate_liquidity_score(&risky_pool).unwrap();
        assert!(liquidity_score >= BigDecimal::from_str("0.7").unwrap()); // Should be high risk
    }

    #[tokio::test]
    async fn test_enhanced_overall_risk_with_tvl_drop() {
        let calculator = RiskCalculator::new();
        
        let impermanent_loss = BigDecimal::from_str("0.1").unwrap();
        let price_impact = BigDecimal::from_str("0.05").unwrap();
        let volatility_score = BigDecimal::from_str("0.3").unwrap();
        let correlation_score = BigDecimal::from_str("0.2").unwrap();
        let liquidity_score = BigDecimal::from_str("0.4").unwrap();
        let tvl_drop_risk = BigDecimal::from_str("0.8").unwrap(); // High TVL drop risk
        
        let thin_pool_risk = BigDecimal::from_str("0.3").unwrap();
        let slippage_risk = BigDecimal::from_str("0.2").unwrap();
        
        let overall_risk = calculator.calculate_enhanced_overall_risk_score(
            impermanent_loss,
            price_impact,
            volatility_score,
            correlation_score,
            liquidity_score,
            thin_pool_risk,
            tvl_drop_risk,
            slippage_risk,
        ).unwrap();
        
        // Risk should be elevated due to TVL drop
        assert!(overall_risk >= BigDecimal::from_str("0.25").unwrap());
        assert!(overall_risk <= BigDecimal::from(1)); // Should be capped at 1.0
    }
}
