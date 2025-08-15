// Chain registry and implementations
use crate::adapters::aave_v3::chain_config::ChainConfig;
use std::collections::HashMap;

pub mod ethereum;
pub mod polygon;
pub mod arbitrum;
pub mod optimism;
pub mod avalanche;

use ethereum::EthereumConfig;
use polygon::PolygonConfig;
use arbitrum::ArbitrumConfig;
use optimism::OptimismConfig;
use avalanche::AvalancheConfig;

/// Registry for all supported chains
pub struct ChainRegistry {
    configs: HashMap<u64, Box<dyn ChainConfig>>,
}

impl ChainRegistry {
    /// Create a new chain registry with all supported chains
    pub fn new() -> Self {
        let mut configs: HashMap<u64, Box<dyn ChainConfig>> = HashMap::new();
        
        // Register all supported chains
        configs.insert(1, Box::new(EthereumConfig));
        configs.insert(137, Box::new(PolygonConfig));
        configs.insert(42161, Box::new(ArbitrumConfig));
        configs.insert(10, Box::new(OptimismConfig));
        configs.insert(43114, Box::new(AvalancheConfig));
        
        Self { configs }
    }
    
    /// Get configuration for a specific chain
    pub fn get_config(&self, chain_id: u64) -> Option<&Box<dyn ChainConfig>> {
        self.configs.get(&chain_id)
    }
    
    /// Check if a chain is supported
    pub fn is_supported(&self, chain_id: u64) -> bool {
        self.configs.contains_key(&chain_id)
    }
    
    /// Get all supported chain IDs
    pub fn supported_chains(&self) -> Vec<u64> {
        self.configs.keys().cloned().collect()
    }
    
    /// Get all chain configurations
    pub fn all_configs(&self) -> &HashMap<u64, Box<dyn ChainConfig>> {
        &self.configs
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to get chain configuration
pub fn get_chain_config(chain_id: u64) -> Option<Box<dyn ChainConfig>> {
    match chain_id {
        1 => Some(Box::new(EthereumConfig)),
        137 => Some(Box::new(PolygonConfig)),
        42161 => Some(Box::new(ArbitrumConfig)),
        10 => Some(Box::new(OptimismConfig)),
        43114 => Some(Box::new(AvalancheConfig)),
        _ => None,
    }
}

/// Get all supported chain IDs
pub fn supported_chain_ids() -> Vec<u64> {
    vec![1, 137, 42161, 10, 43114]
}

/// Get chain name by ID
pub fn get_chain_name(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("Ethereum"),
        137 => Some("Polygon"),
        42161 => Some("Arbitrum"),
        10 => Some("Optimism"),
        43114 => Some("Avalanche"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::chain_config::validation;
    
    #[test]
    fn test_chain_registry() {
        let registry = ChainRegistry::new();
        
        // Test all supported chains are registered
        for chain_id in supported_chain_ids() {
            assert!(registry.is_supported(chain_id));
            assert!(registry.get_config(chain_id).is_some());
        }
        
        // Test unsupported chain
        assert!(!registry.is_supported(99999));
        assert!(registry.get_config(99999).is_none());
    }
    
    #[test]
    fn test_get_chain_config() {
        for chain_id in supported_chain_ids() {
            let config = get_chain_config(chain_id);
            assert!(config.is_some());
            
            // Validate each configuration
            if let Some(config) = config {
                assert!(validation::validate_config(config.as_ref()).is_ok());
            }
        }
    }
    
    #[test]
    fn test_chain_names() {
        assert_eq!(get_chain_name(1), Some("Ethereum"));
        assert_eq!(get_chain_name(137), Some("Polygon"));
        assert_eq!(get_chain_name(42161), Some("Arbitrum"));
        assert_eq!(get_chain_name(10), Some("Optimism"));
        assert_eq!(get_chain_name(43114), Some("Avalanche"));
        assert_eq!(get_chain_name(99999), None);
    }
}
