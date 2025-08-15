// Optimism Aave V3 configuration
use crate::adapters::aave_v3::chain_config::{ChainConfig, parse_address};
use alloy::primitives::Address;

pub struct OptimismConfig;

impl ChainConfig for OptimismConfig {
    fn chain_id(&self) -> u64 {
        10
    }
    
    fn chain_name(&self) -> &'static str {
        "Optimism"
    }
    
    fn pool_address(&self) -> Address {
        parse_address("0x794a61358D6845594F94dc1DB02A252b5b4814aD")
    }
    
    fn data_provider_address(&self) -> Address {
        parse_address("0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654")
    }
    
    fn oracle_address(&self) -> Address {
        parse_address("0xD81eb3728a631871a7eBBaD631b5f424909f0c77")
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            parse_address("0x4200000000000000000000000000000000000006"), // WETH
            parse_address("0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85"), // USDC
            parse_address("0x94b008aA00579c1307B0EF2c499aD98a8ce58e58"), // USDT
            parse_address("0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"), // DAI
            parse_address("0x68f180fcCe6836688e9084f035309E29Bf0A2095"), // WBTC
            parse_address("0x76FB31fb4af56892A25e32cFC43De717950c9278"), // AAVE
            parse_address("0x350a791Bfc2C21F9Ed5d10980Dad2e2638ffa7f6"), // LINK
            parse_address("0x6fd9d7AD17242c41f7131d257212c54A0e816691"), // UNI
            parse_address("0x8c6f28f2F1A3C87F0f938b96d27520d9751ec8d9"), // sUSD
            parse_address("0x9Bcef72be871e61ED4fBbc7630889beE758eb81D"), // rETH
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }
    
    fn block_time_ms(&self) -> u64 {
        2000 // ~2 seconds
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
    fn test_optimism_config() {
        let config = OptimismConfig;
        
        assert_eq!(config.chain_id(), 10);
        assert_eq!(config.chain_name(), "Optimism");
        assert_eq!(config.native_token_symbol(), "ETH");
        assert_eq!(config.block_time_ms(), 2000);
        assert_eq!(config.confirmation_blocks(), 1);
        
        // Validate configuration
        assert!(validation::validate_config(&config).is_ok());
        
        // Test supported assets
        let assets = config.supported_assets();
        assert!(!assets.is_empty());
        assert!(assets.len() >= 10);
    }
}
