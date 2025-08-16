// Base Compound V3 configuration
use crate::adapters::compound_v3::chain_config::ChainConfig;
use alloy::primitives::Address;
use std::str::FromStr;

pub struct BaseConfig;

impl ChainConfig for BaseConfig {
    fn chain_id(&self) -> u64 {
        8453
    }
    
    fn chain_name(&self) -> &'static str {
        "Base"
    }
    
    fn comet_addresses(&self) -> Vec<Address> {
        vec![
            Address::from_str("0x9c4ec768c28520B50860ea7a15bd7213a9fF58bf").unwrap(), // USDbC market
            Address::from_str("0x46e6b214b524310239732D51387075E0e70970bf").unwrap(), // WETH market
        ]
    }
    
    fn rewards_address(&self) -> Option<Address> {
        Address::from_str("0x123964802e6ABabBE1Bc9547D72Ef1332C8d781D").ok()
    }
    
    fn configurator_address(&self) -> Option<Address> {
        None // No configurator on Base
    }
    
    fn supported_assets(&self) -> Vec<Address> {
        vec![
            Address::from_str("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913").unwrap(), // USDbC
            Address::from_str("0x4200000000000000000000000000000000000006").unwrap(), // WETH
        ]
    }
    
    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }
    
    fn block_time_ms(&self) -> u64 {
        2000 // 2 seconds
    }
    
    fn confirmation_blocks(&self) -> u64 {
        10
    }
}
