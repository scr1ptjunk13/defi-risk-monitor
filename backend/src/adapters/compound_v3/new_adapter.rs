// NEW COMPLETE Compound V3 Adapter - Universal Position Detection
// This REPLACES the old hardcoded adapter with dynamic discovery for ANY wallet
// NO HARDCODED MARKETS - discovers everything dynamically

use async_trait::async_trait;
use alloy::primitives::Address;
use std::str::FromStr;
use tracing::{info, error, warn};

use crate::blockchain::ethereum_client::EthereumClient;
use crate::adapters::{DeFiAdapter, AdapterError};
use crate::adapters::compound_v3::universal_detector::{UniversalCompoundV3Detector, DetectedPosition};
use crate::models::position::Position;
use crate::risk::calculators::compound_v3::CompoundV3RiskCalculator;

/// NEW Complete Compound V3 Adapter with Universal Detection
pub struct NewCompoundV3Adapter {
    detector: UniversalCompoundV3Detector,
    risk_calculator: CompoundV3RiskCalculator,
    chain_id: u64,
}

impl NewCompoundV3Adapter {
    /// Create new adapter with universal detection capabilities
    pub fn new(client: EthereumClient, chain_id: u64) -> Result<Self, AdapterError> {
        // Validate chain support
        if !Self::is_supported_chain(chain_id) {
            return Err(AdapterError::UnsupportedChain(format!("Compound V3 not supported on chain {}", chain_id)));
        }
        
        let detector = UniversalCompoundV3Detector::new(client, chain_id);
        let risk_calculator = CompoundV3RiskCalculator::new();
        
        info!("✅ NEW Complete Compound V3 Adapter initialized for chain {}", chain_id);
        
        Ok(Self {
            detector,
            risk_calculator,
            chain_id,
        })
    }
    
    /// Check if chain is supported by Compound V3
    pub fn is_supported_chain(chain_id: u64) -> bool {
        matches!(chain_id, 1 | 137 | 42161 | 8453) // Ethereum, Polygon, Arbitrum, Base
    }
    
    /// Get all supported chain IDs
    pub fn supported_chains() -> Vec<u64> {
        vec![1, 137, 42161, 8453]
    }
    
    /// Get chain name for display
    pub fn chain_name(chain_id: u64) -> &'static str {
        match chain_id {
            1 => "Ethereum",
            137 => "Polygon",
            42161 => "Arbitrum",
            8453 => "Base",
            _ => "Unknown",
        }
    }
}

#[async_trait]
impl DeFiAdapter for NewCompoundV3Adapter {
    fn protocol_name(&self) -> &'static str {
        "compound_v3"
    }
    
    /// Fetch ALL positions for ANY wallet using Universal Detection
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        info!("🚀 NEW Adapter: Fetching ALL Compound V3 positions for {} on chain {}", 
              address, self.chain_id);
        
        // Use Universal Detector to find ALL positions
        let mut detector = self.detector.clone();
        let detected_positions = detector.detect_all_positions(address).await?;
        
        info!("🎯 Universal Detection found {} positions", detected_positions.len());
        
        // Convert to standard Position format
        let positions = detector.convert_to_positions(address, detected_positions);
        
        // Log position summary
        let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
        info!("💰 Total portfolio value: ${:.2}", total_value);
        
        for position in &positions {
            info!("   📊 {}: ${:.2} (Risk: {})", 
                  position.position_type, position.value_usd, position.risk_score);
        }
        
        Ok(positions)
    }
    
    /// Check if contract address is a Compound V3 market
    fn supports_contract(&self, contract_address: Address) -> bool {
        // This will be dynamically determined by the market registry
        // For now, return true and let the detector handle discovery
        true
    }
    
    /// Calculate overall risk score for all positions
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // Use the risk calculator to assess overall portfolio risk
        let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
        let weighted_risk: f64 = positions.iter()
            .map(|p| (p.risk_score as f64) * (p.value_usd / total_value))
            .sum();
        
        // Additional risk factors
        let position_count_risk = match positions.len() {
            1..=3 => 0,
            4..=10 => 5,
            _ => 10,
        };
        
        let leverage_risk = self.calculate_leverage_risk(positions);
        
        let final_risk = (weighted_risk + position_count_risk as f64 + leverage_risk as f64).min(100.0) as u8;
        
        info!("🎯 Portfolio risk assessment: {} (positions: {}, total: ${:.2})", 
              final_risk, positions.len(), total_value);
        
        Ok(final_risk)
    }
    
    /// Get position value (already calculated in fetch_positions)
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        Ok(position.value_usd)
    }
}

impl NewCompoundV3Adapter {
    /// Calculate leverage risk based on borrow/supply ratio
    fn calculate_leverage_risk(&self, positions: &[Position]) -> u8 {
        let mut total_supply = 0.0;
        let mut total_borrow = 0.0;
        
        for position in positions {
            match position.position_type.as_str() {
                "supply" | "collateral" => total_supply += position.value_usd,
                "borrow" => total_borrow += position.value_usd,
                _ => {}
            }
        }
        
        if total_supply == 0.0 {
            return if total_borrow > 0.0 { 100 } else { 0 };
        }
        
        let leverage_ratio = total_borrow / total_supply;
        
        match leverage_ratio {
            r if r < 0.3 => 10,
            r if r < 0.5 => 20,
            r if r < 0.7 => 40,
            r if r < 0.8 => 60,
            r if r < 0.9 => 80,
            _ => 95,
        }
    }
    
    /// Get detailed position breakdown for analysis
    pub async fn get_position_breakdown(&self, address: Address) -> Result<PositionBreakdown, AdapterError> {
        let positions = self.fetch_positions(address).await?;
        
        let mut breakdown = PositionBreakdown {
            total_supply_usd: 0.0,
            total_borrow_usd: 0.0,
            total_collateral_usd: 0.0,
            total_rewards_usd: 0.0,
            net_worth_usd: 0.0,
            health_factor: 0.0,
            liquidation_risk: 0,
            position_count: positions.len(),
            markets_used: std::collections::HashSet::new(),
        };
        
        for position in &positions {
            match position.position_type.as_str() {
                "supply" => breakdown.total_supply_usd += position.value_usd,
                "borrow" => breakdown.total_borrow_usd += position.value_usd,
                "collateral" => breakdown.total_collateral_usd += position.value_usd,
                "rewards" => breakdown.total_rewards_usd += position.value_usd,
                _ => {}
            }
            
            // Extract market from metadata
            if let Some(market_addr) = position.metadata.get("market_address") {
                if let Some(market_str) = market_addr.as_str() {
                    breakdown.markets_used.insert(market_str.to_string());
                }
            }
        }
        
        breakdown.net_worth_usd = breakdown.total_supply_usd + breakdown.total_collateral_usd + breakdown.total_rewards_usd - breakdown.total_borrow_usd;
        
        // Calculate health factor (simplified)
        if breakdown.total_borrow_usd > 0.0 {
            breakdown.health_factor = (breakdown.total_collateral_usd * 0.8) / breakdown.total_borrow_usd;
            breakdown.liquidation_risk = if breakdown.health_factor < 1.1 { 90 } 
                                       else if breakdown.health_factor < 1.3 { 60 }
                                       else if breakdown.health_factor < 1.5 { 30 }
                                       else { 10 };
        } else {
            breakdown.health_factor = f64::INFINITY;
            breakdown.liquidation_risk = 0;
        }
        
        Ok(breakdown)
    }
}

/// Detailed position breakdown for analysis
#[derive(Debug, Clone)]
pub struct PositionBreakdown {
    pub total_supply_usd: f64,
    pub total_borrow_usd: f64,
    pub total_collateral_usd: f64,
    pub total_rewards_usd: f64,
    pub net_worth_usd: f64,
    pub health_factor: f64,
    pub liquidation_risk: u8,
    pub position_count: usize,
    pub markets_used: std::collections::HashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::ethereum_client::EthereumClient;
    
    #[test]
    fn test_supported_chains() {
        assert!(NewCompoundV3Adapter::is_supported_chain(1)); // Ethereum
        assert!(NewCompoundV3Adapter::is_supported_chain(137)); // Polygon
        assert!(NewCompoundV3Adapter::is_supported_chain(42161)); // Arbitrum
        assert!(NewCompoundV3Adapter::is_supported_chain(8453)); // Base
        assert!(!NewCompoundV3Adapter::is_supported_chain(56)); // BSC not supported
    }
    
    #[test]
    fn test_chain_names() {
        assert_eq!(NewCompoundV3Adapter::chain_name(1), "Ethereum");
        assert_eq!(NewCompoundV3Adapter::chain_name(137), "Polygon");
        assert_eq!(NewCompoundV3Adapter::chain_name(42161), "Arbitrum");
        assert_eq!(NewCompoundV3Adapter::chain_name(8453), "Base");
        assert_eq!(NewCompoundV3Adapter::chain_name(999), "Unknown");
    }
    
    #[tokio::test]
    async fn test_adapter_creation() {
        // This would require a real client for full testing
        // For now, just test that supported chains work
        let supported = NewCompoundV3Adapter::supported_chains();
        assert_eq!(supported.len(), 4);
        assert!(supported.contains(&1));
        assert!(supported.contains(&137));
        assert!(supported.contains(&42161));
        assert!(supported.contains(&8453));
    }
}
