// Polygon Compound V3 configuration
use crate::adapters::compound_v3::chain_config::ChainConfig;
use alloy::primitives::Address;
use std::str::FromStr;

pub struct PolygonConfig;

impl ChainConfig for PolygonConfig {
    fn chain_id(&self) -> u64 {
        137
    }
    
    fn chain_name(&self) -> &'static str {
        "Polygon"
    }
    
    fn comet_addresses(&self) -> Vec<Address> {
        vec![
            Address::from_str("0xF25212E676D1F7F89Cd72fFEe66158f541246445").unwrap(), // USDC market
        ]
    }
    
    fn rewards_address(&self) -> Option<Address> {
        Address::from_str("0x45939657d1CA34A8FA39A924B71D28Fe8431e581").ok()
    }
    
    fn configurator_address(&self) -> Option<Address> {
        None // No configurator on Polygon
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            Address::from_str("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174").unwrap(), // USDC
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "MATIC"
    }
    
    fn block_time_ms(&self) -> u64 {
        2000 // 2 seconds
    }
    
    fn confirmation_blocks(&self) -> u64 {
        20
    }
}
