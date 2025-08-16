// Chain registry for Compound V3 adapter

// Module declarations
pub mod ethereum;
pub mod polygon;
pub mod arbitrum;
pub mod base;

use crate::adapters::compound_v3::chain_config::ChainConfig;
use ethereum::EthereumConfig;
use polygon::PolygonConfig;
use arbitrum::ArbitrumConfig;
use base::BaseConfig;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Registry of all supported chains for Compound V3
pub struct ChainRegistry {
    chains: HashMap<u64, Box<dyn ChainConfig>>,
}

impl ChainRegistry {
    /// Create a new chain registry with all supported Compound V3 chains
    pub fn new() -> Self {
        let mut chains: HashMap<u64, Box<dyn ChainConfig>> = HashMap::new();
        
        // Add all supported Compound V3 chains (based on actual deployments)
        chains.insert(1, Box::new(EthereumConfig));      // Ethereum Mainnet
        chains.insert(137, Box::new(PolygonConfig));     // Polygon
        chains.insert(42161, Box::new(ArbitrumConfig));  // Arbitrum
        chains.insert(8453, Box::new(BaseConfig));       // Base
        
        Self { chains }
    }
    
    /// Get chain configuration by chain ID
    pub fn get_chain_config(&self, chain_id: u64) -> Option<&dyn ChainConfig> {
        self.chains.get(&chain_id).map(|config| config.as_ref())
    }
    
    /// Get all supported chain IDs
    pub fn supported_chain_ids(&self) -> Vec<u64> {
        self.chains.keys().copied().collect()
    }
    
    /// Check if a chain is supported
    pub fn is_chain_supported(&self, chain_id: u64) -> bool {
        self.chains.contains_key(&chain_id)
    }
    
    /// Get chain name by chain ID
    pub fn get_chain_name(&self, chain_id: u64) -> Option<&'static str> {
        self.get_chain_config(chain_id).map(|config| config.chain_name())
    }
}

/// Global chain registry instance
static CHAIN_REGISTRY: OnceLock<ChainRegistry> = OnceLock::new();

/// Get the global chain registry instance
pub fn get_chain_registry() -> &'static ChainRegistry {
    CHAIN_REGISTRY.get_or_init(|| ChainRegistry::new())
}

/// Get chain configuration for a specific chain ID
pub fn get_chain_config(chain_id: u64) -> Option<&'static dyn ChainConfig> {
    get_chain_registry().get_chain_config(chain_id)
}

/// Get all supported Compound V3 chain IDs
pub fn get_supported_chains() -> Vec<u64> {
    get_chain_registry().supported_chain_ids()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::compound_v3::chain_config::validation;
    
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
        assert_eq!(get_chain_name(8453), Some("Base"));
        assert_eq!(get_chain_name(99999), None);
    }
}
