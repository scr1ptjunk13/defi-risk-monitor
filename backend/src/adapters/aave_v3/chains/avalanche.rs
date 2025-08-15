// Avalanche Aave V3 configuration
use crate::adapters::aave_v3::chain_config::{ChainConfig, parse_address};
use alloy::primitives::Address;

pub struct AvalancheConfig;

impl ChainConfig for AvalancheConfig {
    fn chain_id(&self) -> u64 {
        43114
    }
    
    fn chain_name(&self) -> &'static str {
        "Avalanche"
    }
    
    fn pool_address(&self) -> Address {
        parse_address("0x794a61358D6845594F94dc1DB02A252b5b4814aD")
    }
    
    fn data_provider_address(&self) -> Address {
        parse_address("0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654")
    }
    
    fn oracle_address(&self) -> Address {
        parse_address("0xEBd36016B3eD09D4693Ed4251c67Bd858c3c7C9C")
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            parse_address("0xB31f66AA3C1e785363F0875A1B74E27b85FD66c7"), // WAVAX
            parse_address("0xB97EF9Ef8734C71904D8002F8b6Bc66Dd9c48a6E"), // USDC
            parse_address("0x9702230A8Ea53601f5cD2dc00fDBc13d4dF4A8c7"), // USDT
            parse_address("0xd586E7F844cEa2F87f50152665BCbc2C279D8d70"), // DAI.e
            parse_address("0x50b7545627a5162F82A992c33b87aDc75187B218"), // WBTC.e
            parse_address("0x49D5c2BdFfac6CE2BFdB6640F4F80f226bc10bAB"), // WETH.e
            parse_address("0x63a72806098Bd3D9520cC43356dD78afe5D386D9"), // AAVE.e
            parse_address("0x5947BB275c521040051D82396192181b413227A3"), // LINK.e
            parse_address("0x8eBAf22B6F053dFFeaf46f4Dd9eFA95D89ba8580"), // UNI.e
            parse_address("0xA7D7079b0FEaD91F3e65f86E8915Cb59c1a4C664"), // USDC.e
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "AVAX"
    }
    
    fn block_time_ms(&self) -> u64 {
        2000 // ~2 seconds
    }
    
    fn confirmation_blocks(&self) -> u64 {
        1 // Fast finality
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::chain_config::validation;
    
    #[test]
    fn test_avalanche_config() {
        let config = AvalancheConfig;
        
        assert_eq!(config.chain_id(), 43114);
        assert_eq!(config.chain_name(), "Avalanche");
        assert_eq!(config.native_token_symbol(), "AVAX");
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
