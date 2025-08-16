// Ethereum mainnet Compound V3 configuration
use crate::adapters::compound_v3::chain_config::ChainConfig;
use alloy::primitives::Address;
use std::str::FromStr;

pub struct EthereumConfig;

impl ChainConfig for EthereumConfig {
    fn chain_id(&self) -> u64 {
        1
    }

    fn chain_name(&self) -> &'static str {
        "Ethereum"
    }

    fn comet_addresses(&self) -> Vec<Address> {
        vec![
            Address::from_str("0xc3d688B66703497DAA19211EEdff47f25384cdc3").unwrap(), // USDC market
            Address::from_str("0xA17581A9E3356d9A858b789D68B4d866e593aE94").unwrap(), // WETH market
        ]
    }

    fn rewards_address(&self) -> Option<Address> {
        Address::from_str("0x1B0e765F6224C21223AeA2af16c1C46E38885a40").ok()
    }

    fn configurator_address(&self) -> Option<Address> {
        Address::from_str("0x316f9708bB98af7dA9c68C1C3b5e79039cD336E3").ok()
    }

    fn supported_assets(&self) -> Vec<Address> {
        vec![
            Address::from_str("0xA0b86a33E6441c8C0c0c241c7C601b0906c0b8c").unwrap(), // USDC
            Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(), // WETH
        ]
    }

    fn native_token_symbol(&self) -> &'static str {
        "ETH"
    }

    fn block_time_ms(&self) -> u64 {
        12000
    }

    fn confirmation_blocks(&self) -> u64 {
        12
    }
}
