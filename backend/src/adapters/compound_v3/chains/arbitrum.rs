// Arbitrum Compound V3 configuration
use crate::adapters::compound_v3::chain_config::ChainConfig;
use alloy::primitives::Address;
use std::str::FromStr;

pub struct ArbitrumConfig;

impl ChainConfig for ArbitrumConfig {
    fn chain_id(&self) -> u64 {
        42161
    }
    
    fn chain_name(&self) -> &'static str {
        "Arbitrum"
    }
    
    fn comet_addresses(&self) -> Vec<Address> {
        vec![
            Address::from_str("0xA5EDBDD9646f8dFF606d7448e414884C7d905dCA").unwrap(), // USDC.e market
            Address::from_str("0x9c4ec768c28520B50860ea7a15bd7213a9fF58bf").unwrap(), // USDC market
        ]
    }
    
    fn rewards_address(&self) -> Option<Address> {
        Address::from_str("0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae").ok()
    }
    
    fn configurator_address(&self) -> Option<Address> {
        None // No configurator on Arbitrum
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            Address::from_str("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8").unwrap(), // USDC.e
            Address::from_str("0xaf88d065e77c8cC2239327C5EDb3A432268e5831").unwrap(), // USDC
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }
    
    fn block_time_ms(&self) -> u64 {
        250 // ~250ms
    }
    
    fn confirmation_blocks(&self) -> u64 {
        1
    }
}
