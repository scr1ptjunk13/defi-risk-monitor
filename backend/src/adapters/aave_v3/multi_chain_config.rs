use std::collections::HashMap;
use std::env;

/// Multi-chain RPC configuration for Aave V3 adapter
#[derive(Debug, Clone)]
pub struct MultiChainRpcConfig {
    rpc_urls: HashMap<u64, String>,
}

impl MultiChainRpcConfig {
    /// Create new multi-chain RPC configuration from environment variables
    pub fn from_env() -> Self {
        let mut rpc_urls = HashMap::new();
        
        // Ethereum Mainnet (Chain 1)
        if let Ok(url) = env::var("ETHEREUM_RPC_URL") {
            rpc_urls.insert(1, url);
        } else {
            // Fallback to public RPC (rate limited)
            rpc_urls.insert(1, "https://eth-mainnet.g.alchemy.com/v2/demo".to_string());
        }
        
        // Polygon (Chain 137)
        if let Ok(url) = env::var("POLYGON_RPC_URL") {
            rpc_urls.insert(137, url);
        } else {
            rpc_urls.insert(137, "https://polygon-rpc.com".to_string());
        }
        
        // Arbitrum (Chain 42161)
        if let Ok(url) = env::var("ARBITRUM_RPC_URL") {
            rpc_urls.insert(42161, url);
        } else {
            rpc_urls.insert(42161, "https://arb1.arbitrum.io/rpc".to_string());
        }
        
        // Optimism (Chain 10)
        if let Ok(url) = env::var("OPTIMISM_RPC_URL") {
            rpc_urls.insert(10, url);
        } else {
            rpc_urls.insert(10, "https://mainnet.optimism.io".to_string());
        }
        
        // Avalanche (Chain 43114)
        if let Ok(url) = env::var("AVALANCHE_RPC_URL") {
            rpc_urls.insert(43114, url);
        } else {
            rpc_urls.insert(43114, "https://api.avax.network/ext/bc/C/rpc".to_string());
        }
        
        Self { rpc_urls }
    }
    
    /// Get RPC URL for a specific chain
    pub fn get_rpc_url(&self, chain_id: u64) -> Option<&String> {
        self.rpc_urls.get(&chain_id)
    }
    
    /// Get all configured chains
    pub fn configured_chains(&self) -> Vec<u64> {
        self.rpc_urls.keys().cloned().collect()
    }
    
    /// Check if a chain is configured
    pub fn is_configured(&self, chain_id: u64) -> bool {
        self.rpc_urls.contains_key(&chain_id)
    }
}

impl Default for MultiChainRpcConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

/// Get default RPC URLs for testing (public endpoints with rate limits)
pub fn get_default_rpc_urls() -> HashMap<u64, String> {
    let mut urls = HashMap::new();
    
    urls.insert(1, "https://eth-mainnet.g.alchemy.com/v2/demo".to_string());
    urls.insert(137, "https://polygon-rpc.com".to_string());
    urls.insert(42161, "https://arb1.arbitrum.io/rpc".to_string());
    urls.insert(10, "https://mainnet.optimism.io".to_string());
    urls.insert(43114, "https://api.avax.network/ext/bc/C/rpc".to_string());
    
    urls
}

/// Get chain-specific client creation function
pub async fn create_chain_client(_chain_id: u64, _rpc_url: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Commented out broken blockchain client creation:
    // Ok(crate::blockchain::ethereum_client::EthereumClient::from_rpc_url(&rpc_url)?)
    Err("Blockchain client not implemented".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_multi_chain_config() {
        let config = MultiChainRpcConfig::from_env();
        
        // Should have at least Ethereum configured
        assert!(config.is_configured(1));
        assert!(config.get_rpc_url(1).is_some());
        
        // Should return configured chains
        let chains = config.configured_chains();
        assert!(!chains.is_empty());
        assert!(chains.contains(&1)); // Ethereum should always be present
    }
    
    #[test]
    fn test_default_rpc_urls() {
        let urls = get_default_rpc_urls();
        
        assert_eq!(urls.len(), 5); // All 5 supported chains
        assert!(urls.contains_key(&1));   // Ethereum
        assert!(urls.contains_key(&137)); // Polygon
        assert!(urls.contains_key(&42161)); // Arbitrum
        assert!(urls.contains_key(&10));  // Optimism
        assert!(urls.contains_key(&43114)); // Avalanche
    }
}
