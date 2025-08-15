// Polygon Aave V3 configuration
use crate::adapters::aave_v3::chain_config::{ChainConfig, parse_address};
use alloy::primitives::Address;

pub struct PolygonConfig;

impl ChainConfig for PolygonConfig {
    fn chain_id(&self) -> u64 {
        137
    }
    
    fn chain_name(&self) -> &'static str {
        "Polygon"
    }
    
    fn pool_address(&self) -> Address {
        parse_address("0x794a61358D6845594F94dc1DB02A252b5b4814aD")
    }
    
    fn data_provider_address(&self) -> Address {
        parse_address("0x69FA688f1Dc47d4B5d8029D5a35FB7a548310654")
    }
    
    fn oracle_address(&self) -> Address {
        parse_address("0xb023e699F5a33916Ea823A16485e259257cA8Bd1")
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            parse_address("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"), // WMATIC
            parse_address("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"), // USDC
            parse_address("0xc2132D05D31c914a87C6611C10748AEb04B58e8F"), // USDT
            parse_address("0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"), // DAI
            parse_address("0x1BFD67037B42Cf73acF2047067bd4F2C47D9BfD6"), // WBTC
            parse_address("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619"), // WETH
            parse_address("0xD6DF932A45C0f255f85145f286eA0b292B21C90B"), // AAVE
            parse_address("0x53E0bca35eC356BD5ddDFebbD1Fc0fD03FaBad39"), // LINK
            parse_address("0xb33EaAd8d922B1083446DC23f610c2567fB5180f"), // UNI
            parse_address("0x4e3Decbb3645551B8A19f0eA1678079FCB33fB4c"), // jEUR
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "MATIC"
    }
    
    fn block_time_ms(&self) -> u64 {
        2000 // 2 seconds
    }
    
    fn confirmation_blocks(&self) -> u64 {
        20 // ~40 seconds for finality
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::aave_v3::chain_config::validation;
    
    #[test]
    fn test_polygon_config() {
        let config = PolygonConfig;
        
        assert_eq!(config.chain_id(), 137);
        assert_eq!(config.chain_name(), "Polygon");
        assert_eq!(config.native_token_symbol(), "MATIC");
        assert_eq!(config.block_time_ms(), 2000);
        assert_eq!(config.confirmation_blocks(), 20);
        
        // Validate configuration
        assert!(validation::validate_config(&config).is_ok());
        
        // Test supported assets
        let assets = config.supported_assets();
        assert!(!assets.is_empty());
        assert!(assets.len() >= 10);
    }
}
