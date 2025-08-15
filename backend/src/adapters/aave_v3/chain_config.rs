// Chain-specific configuration trait and utilities
use alloy::primitives::Address;
use std::str::FromStr;

/// Trait for chain-specific Aave V3 configurations
pub trait ChainConfig: Send + Sync {
    fn chain_id(&self) -> u64;
    fn chain_name(&self) -> &'static str;
    fn pool_address(&self) -> Address;
    fn data_provider_address(&self) -> Address;
    fn oracle_address(&self) -> Address;
    fn supported_assets(&self) -> Vec<Address>;
    fn native_token_symbol(&self) -> &'static str;
    fn block_time_ms(&self) -> u64;
    fn confirmation_blocks(&self) -> u64;
}

/// Helper function to parse address from string with error handling
pub fn parse_address(addr_str: &str) -> Address {
    Address::from_str(addr_str)
        .unwrap_or_else(|_| panic!("Invalid address format: {}", addr_str))
}

/// Common asset addresses used across multiple chains
pub mod common_assets {
    // Ethereum mainnet addresses (used as reference)
    pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &str = "0xA0b86a33E6441E0B9B8B273c81F6C5b6d0e8F7b0";
    pub const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
    pub const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
    pub const WBTC: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
}

/// Configuration validation utilities
pub mod validation {
    use super::*;
    
    pub fn validate_config(config: &dyn ChainConfig) -> Result<(), String> {
        // Validate chain ID
        if config.chain_id() == 0 {
            return Err("Chain ID cannot be zero".to_string());
        }
        
        // Validate addresses are not zero
        let zero_addr = Address::from_str("0x0000000000000000000000000000000000000000").unwrap();
        
        if config.pool_address() == zero_addr {
            return Err("Pool address cannot be zero".to_string());
        }
        
        if config.data_provider_address() == zero_addr {
            return Err("Data provider address cannot be zero".to_string());
        }
        
        if config.oracle_address() == zero_addr {
            return Err("Oracle address cannot be zero".to_string());
        }
        
        // Validate chain name is not empty
        if config.chain_name().is_empty() {
            return Err("Chain name cannot be empty".to_string());
        }
        
        // Validate native token symbol is not empty
        if config.native_token_symbol().is_empty() {
            return Err("Native token symbol cannot be empty".to_string());
        }
        
        // Validate block time and confirmation blocks are reasonable
        if config.block_time_ms() == 0 || config.block_time_ms() > 60000 {
            return Err("Block time must be between 1ms and 60s".to_string());
        }
        
        if config.confirmation_blocks() == 0 || config.confirmation_blocks() > 100 {
            return Err("Confirmation blocks must be between 1 and 100".to_string());
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct MockChainConfig;
    
    impl ChainConfig for MockChainConfig {
        fn chain_id(&self) -> u64 { 1 }
        fn chain_name(&self) -> &'static str { "Ethereum" }
        fn pool_address(&self) -> Address { 
            parse_address("0x87870Bce3F2c42a6C99f1b5b3c37eed3ECF86D0a") 
        }
        fn data_provider_address(&self) -> Address { 
            parse_address("0x7B4EB56E7CD4b454BA8ff71E4518426369a138a3") 
        }
        fn oracle_address(&self) -> Address { 
            parse_address("0x54586bE62E3c3580375aE3723C145253060Ca0C2") 
        }
        fn supported_assets(&self) -> Vec<Address> { vec![] }
        fn native_token_symbol(&self) -> &'static str { "ETH" }
        fn block_time_ms(&self) -> u64 { 12000 }
        fn confirmation_blocks(&self) -> u64 { 12 }
    }
    
    #[test]
    fn test_config_validation() {
        let config = MockChainConfig;
        assert!(validation::validate_config(&config).is_ok());
    }
    
    #[test]
    fn test_address_parsing() {
        let addr = parse_address("0x87870Bce3F2c42a6C99f1b5b3c37eed3ECF86D0a");
        assert_ne!(addr, Address::from_str("0x0000000000000000000000000000000000000000").unwrap());
    }
}
