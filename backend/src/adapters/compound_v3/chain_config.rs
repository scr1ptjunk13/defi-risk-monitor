// Chain-specific configuration trait and utilities for Compound V3
use alloy::primitives::Address;
use std::str::FromStr;

/// Chain-specific configuration trait for Compound V3
pub trait ChainConfig: Send + Sync {
    fn chain_id(&self) -> u64;
    fn chain_name(&self) -> &'static str;
    fn comet_addresses(&self) -> Vec<Address>;
    fn rewards_address(&self) -> Option<Address>;
    fn configurator_address(&self) -> Option<Address>;
    fn supported_assets(&self) -> Vec<Address>;
    fn native_token_symbol(&self) -> &'static str;
    fn block_time_ms(&self) -> u64;
    fn confirmation_blocks(&self) -> u64;
}

/// Common asset addresses used across multiple chains
pub mod common_assets {
    // Ethereum mainnet addresses (used as reference)
    pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &str = "0xA0b86a33E6441E0B9B8B273c81F6C5b6d0e8F7b0";
    pub const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
    pub const WBTC: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
    pub const COMP: &str = "0xc00e94Cb662C3520282E6f5717214004A7f26888";
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
        
        if config.comet_addresses().is_empty() {
            return Err("Must have at least one Comet address".to_string());
        }
        
        for comet_addr in config.comet_addresses() {
            if comet_addr == zero_addr {
                return Err("Comet address cannot be zero".to_string());
            }
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
        fn comet_addresses(&self) -> Vec<Address> { 
            vec![parse_address("0xc3d688B66703497DAA19211EEdff47f25384cdc3")] // cUSDCv3
        }
        fn rewards_address(&self) -> Option<Address> { 
            Some(parse_address("0x1B0e765F6224C21223AeA2af16c1C46E38885a40"))
        }
        fn configurator_address(&self) -> Option<Address> { 
            Some(parse_address("0x316f9708bB98af7dA9c68C1C3b5e79039cD336E3"))
        }
        fn supported_base_tokens(&self) -> Vec<Address> { 
            vec![parse_address("0xA0b86a33E6441E0B9B8B273c81F6C5b6d0e8F7b0")] // USDC
        }
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
        let addr = parse_address("0xc3d688B66703497DAA19211EEdff47f25384cdc3");
        assert_ne!(addr, Address::from_str("0x0000000000000000000000000000000000000000").unwrap());
    }
}
