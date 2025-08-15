// Ethereum mainnet Aave V3 configuration
use crate::adapters::aave_v3::chain_config::{ChainConfig, parse_address};
use alloy::primitives::Address;

pub struct EthereumConfig;

impl ChainConfig for EthereumConfig {
    fn chain_id(&self) -> u64 {
        1
    }
    
    fn chain_name(&self) -> &'static str {
        "Ethereum"
    }
    
    fn pool_address(&self) -> Address {
        parse_address("0x87870Bce3F2c42a6C99f1b5b3c37eed3ECF86D0a")
    }
    
    fn data_provider_address(&self) -> Address {
        parse_address("0x7B4EB56E7CD4b454BA8ff71E4518426369a138a3")
    }
    
    fn oracle_address(&self) -> Address {
        parse_address("0x54586bE62E3c3580375aE3723C145253060Ca0C2")
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            parse_address("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"), // WETH
            parse_address("0xA0b86a33E6441E0B9B8B273c81F6C5b6d0e8F7b0"), // USDC
            parse_address("0xdAC17F958D2ee523a2206206994597C13D831ec7"), // USDT
            parse_address("0x6B175474E89094C44Da98b954EedeAC495271d0F"), // DAI
            parse_address("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"), // WBTC
            parse_address("0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"), // AAVE
            parse_address("0x514910771AF9Ca656af840dff83E8264EcF986CA"), // LINK
            parse_address("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984"), // UNI
            parse_address("0x7D1AfA7B718fb893dB30A3aBc0Cfc608AaCfeBB0"), // MATIC
            parse_address("0x4Fabb145d64652a948d72533023f6E7A623C7C53"), // BUSD
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }
    
    fn block_time_ms(&self) -> u64 {
        12000 // 12 seconds
    }
    
    fn confirmation_blocks(&self) -> u64 {
        12 // ~2.4 minutes for finality
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::chain_config::validation;
    
    #[test]
    fn test_ethereum_config() {
        let config = EthereumConfig;
        
        assert_eq!(config.chain_id(), 1);
        assert_eq!(config.chain_name(), "Ethereum");
        assert_eq!(config.native_token_symbol(), "ETH");
        assert_eq!(config.block_time_ms(), 12000);
        assert_eq!(config.confirmation_blocks(), 12);
        
        // Validate configuration
        assert!(validation::validate_config(&config).is_ok());
        
        // Test supported assets
        let assets = config.supported_assets();
        assert!(!assets.is_empty());
        assert!(assets.len() >= 10); // Should have at least 10 major assets
    }
}
