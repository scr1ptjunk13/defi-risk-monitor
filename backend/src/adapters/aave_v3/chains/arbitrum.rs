// Arbitrum Aave V3 configuration
use crate::adapters::aave_v3::chain_config::{ChainConfig, parse_address};
use alloy::primitives::Address;

pub struct ArbitrumConfig;

impl ChainConfig for ArbitrumConfig {
    fn chain_id(&self) -> u64 {
        42161
    }
    
    fn chain_name(&self) -> &'static str {
        "Arbitrum"
    }
    
    fn pool_address(&self) -> Address {
        parse_address("0x794a61358D6845594F94dc1DB02A252b5b4814aD")
    }
    
    fn data_provider_address(&self) -> Address {
        parse_address("0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654")
    }
    
    fn oracle_address(&self) -> Address {
        parse_address("0xb56c2F0B653B2e0b10C9b928C8580Ac5Df02C7C7")
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            parse_address("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"), // WETH
            parse_address("0xaf88d065e77c8cC2239327C5EDb3A432268e5831"), // USDC
            parse_address("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"), // USDT
            parse_address("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"), // DAI
            parse_address("0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"), // WBTC
            parse_address("0xba5DdD1f9d7F570dc94a51479a000E3BCE967196"), // AAVE
            parse_address("0xf97f4df75117a78c1A5a0DBb814Af92458539FB4"), // LINK
            parse_address("0xFa7F8980b0f1E64A2062791cc3b0871572f1F7f0"), // UNI
            parse_address("0x17FC002b466eEc40DaE837Fc4bE5c67993ddBd6F"), // FRAX
            parse_address("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8"), // USDC.e
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }
    
    fn block_time_ms(&self) -> u64 {
        250 // ~250ms
    }
    
    fn confirmation_blocks(&self) -> u64 {
        1 // Very fast finality on L2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::chain_config::validation;
    
    #[test]
    fn test_arbitrum_config() {
        let config = ArbitrumConfig;
        
        assert_eq!(config.chain_id(), 42161);
        assert_eq!(config.chain_name(), "Arbitrum");
        assert_eq!(config.native_token_symbol(), "ETH");
        assert_eq!(config.block_time_ms(), 250);
        assert_eq!(config.confirmation_blocks(), 1);
        
        // Validate configuration
        assert!(validation::validate_config(&config).is_ok());
        
        // Test supported assets
        let assets = config.supported_assets();
        assert!(!assets.is_empty());
        assert!(assets.len() >= 10);
    }
}
