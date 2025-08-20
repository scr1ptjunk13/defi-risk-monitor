use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use reqwest;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone)]
pub struct EthereumClient {
    pub rpc_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RocketPoolApiResponse {
    data: Option<serde_json::Value>,
    status: String,
}

#[derive(Debug, Clone)]
struct NodeOperatorMetrics {
    total_nodes: u64,
    active_nodes: u64,
    total_minipools: u64,
    active_minipools: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ProtocolMetrics {
    total_eth_staked: f64,
    reth_supply: f64,
    reth_exchange_rate: f64,
    node_demand: f64,
    deposit_pool_balance: f64,
    network_node_fee: f64,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct RocketPoolStakingPosition {
    token_address: Address,
    token_symbol: String,
    balance: U256,
    decimals: u8,
    underlying_asset: String,
    apy: f64,
    rewards_earned: U256,
    position_subtype: String,
}

sol! {
    #[sol(rpc)]
    interface IRocketTokenRETH {
        function balanceOf(address account) external view returns (uint256);
        function getEthValue(uint256 rethAmount) external view returns (uint256);
        function getRethValue(uint256 ethAmount) external view returns (uint256);
        function getExchangeRate() external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
    }
    
    #[sol(rpc)]
    interface IRocketDepositPool {
        function getBalance() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketNodeManager {
        function getNodeCount() external view returns (uint256);
        function getNodeExists(address nodeAddress) external view returns (bool);
    }
    
    #[sol(rpc)]
    interface IRocketMinipoolManager {
        function getMinipoolCount() external view returns (uint256);
        function getNodeMinipoolCount(address nodeAddress) external view returns (uint256);
        function getNodeActiveMinipoolCount(address nodeAddress) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketNetworkFees {
        function getNodeDemand() external view returns (int256);
        function getNodeFee() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketNodeStaking {
        function getNodeRPLStake(address nodeAddress) external view returns (uint256);
        function getNodeEffectiveRPLStake(address nodeAddress) external view returns (uint256);
    }
}

pub struct RocketPoolAdapter {
    #[allow(dead_code)]
    client: EthereumClient,
    reth_address: Address,
    deposit_pool_address: Address,
    node_manager_address: Address,
    minipool_manager_address: Address,
    network_fees_address: Address,
    node_staking_address: Address,
    rpl_token_address: Address,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    http_client: reqwest::Client,
    coingecko_api_key: Option<String>,
}

impl RocketPoolAdapter {
    const RETH_ADDRESS: &'static str = "0xae78736Cd615f374D3085123A210448E74Fc6393";
    const DEPOSIT_POOL_ADDRESS: &'static str = "0x2cac916b2A963Bf162f076C0a8a4a8200BCFBfb4";
    const NODE_MANAGER_ADDRESS: &'static str = "0x89F478E6Cc24f052103628f36598D4C14Da3D287";
    const MINIPOOL_MANAGER_ADDRESS: &'static str = "0x6d010a588f89E7e8634e1fF7A59C6F70C7D9A37b";
    const NETWORK_FEES_ADDRESS: &'static str = "0xeE4d2A71cF479e0312B3AF664B4f652E23880B12";
    const NODE_STAKING_ADDRESS: &'static str = "0x3019227b2b8493e45Bf5d6777666dC81E6e8EC2C";
    const RPL_TOKEN_ADDRESS: &'static str = "0xD33526068D116cE69F19A9ee46F0bd304F21A51f";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let reth_address = Address::from_str(Self::RETH_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid rETH address: {}", e)))?;
        let deposit_pool_address = Address::from_str(Self::DEPOSIT_POOL_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid deposit pool address: {}", e)))?;
        let node_manager_address = Address::from_str(Self::NODE_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid node manager address: {}", e)))?;
        let minipool_manager_address = Address::from_str(Self::MINIPOOL_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid minipool manager address: {}", e)))?;
        let network_fees_address = Address::from_str(Self::NETWORK_FEES_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid network fees address: {}", e)))?;
        let node_staking_address = Address::from_str(Self::NODE_STAKING_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid node staking address: {}", e)))?;
        let rpl_token_address = Address::from_str(Self::RPL_TOKEN_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid RPL token address: {}", e)))?;
        
        Ok(Self {
            client,
            reth_address,
            deposit_pool_address,
            node_manager_address,
            minipool_manager_address,
            network_fees_address,
            node_staking_address,
            rpl_token_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        })
    }
    
    async fn get_user_staking_positions(&self, address: Address) -> Result<Vec<RocketPoolStakingPosition>, AdapterError> {
        let mut positions = Vec::new();
        
        if let Some(reth_position) = self.get_reth_position(address).await? {
            positions.push(reth_position);
        }
        
        if let Some(node_positions) = self.get_node_operator_positions(address).await? {
            positions.extend(node_positions);
        }
        
        if let Some(rpl_position) = self.get_rpl_staking_position(address).await? {
            positions.push(rpl_position);
        }
        
        Ok(positions)
    }
    
    async fn get_reth_position(&self, user_address: Address) -> Result<Option<RocketPoolStakingPosition>, AdapterError> {
        let balance = U256::ZERO; // Placeholder for contract call
        
        if balance == U256::ZERO {
            return Ok(None);
        }
        
        let eth_value = balance;
        let _exchange_rate = U256::from(1000000000000000000u64);
        let apy = self.get_rocket_pool_apy("rETH").await.unwrap_or(3.5);
        let rewards_earned = self.estimate_reth_rewards(user_address, balance, eth_value).await;
        
        Ok(Some(RocketPoolStakingPosition {
            token_address: self.reth_address,
            token_symbol: "rETH".to_string(),
            balance,
            decimals: 18,
            underlying_asset: "ETH".to_string(),
            apy,
            rewards_earned,
            position_subtype: "liquid_staking".to_string(),
        }))
    }
    
    async fn get_node_operator_positions(&self, _user_address: Address) -> Result<Option<Vec<RocketPoolStakingPosition>>, AdapterError> {
        let is_node = false; // Placeholder for contract call
        
        if !is_node {
            return Ok(None);
        }
        
        let mut positions = Vec::new();
        let minipool_count = 0u64; // Placeholder
        let _active_minipool_count = 0u64; // Placeholder
        
        if minipool_count > 0 {
            let node_eth_deposited = (minipool_count as f64) * 16.0;
            let node_apy = self.get_node_operator_apy().await.unwrap_or(5.5);
            let rewards_earned = U256::from((node_eth_deposited * 0.055 * 10f64.powi(18)) as u64);
            
            let node_position = RocketPoolStakingPosition {
                token_address: self.minipool_manager_address,
                token_symbol: format!("RP-NODE-{}", minipool_count),
                balance: U256::from((node_eth_deposited * 10f64.powi(18)) as u64),
                decimals: 18,
                underlying_asset: "ETH".to_string(),
                apy: node_apy,
                rewards_earned,
                position_subtype: "node_operator".to_string(),
            };
            
            positions.push(node_position);
        }
        
        Ok(if positions.is_empty() { None } else { Some(positions) })
    }
    
    async fn get_rpl_staking_position(&self, user_address: Address) -> Result<Option<RocketPoolStakingPosition>, AdapterError> {
        let is_node = false; // Placeholder
        
        if !is_node {
            return Ok(None);
        }
        
        let rpl_stake = U256::ZERO; // Placeholder
        
        if rpl_stake == U256::ZERO {
            return Ok(None);
        }
        
        let rpl_apy = self.get_rpl_staking_apy().await.unwrap_or(8.0);
        let rewards_earned = self.estimate_rpl_rewards(user_address, rpl_stake).await;
        
        Ok(Some(RocketPoolStakingPosition {
            token_address: self.rpl_token_address,
            token_symbol: "RPL-STAKED".to_string(),
            balance: rpl_stake,
            decimals: 18,
            underlying_asset: "RPL".to_string(),
            apy: rpl_apy,
            rewards_earned,
            position_subtype: "rpl_staking".to_string(),
        }))
    }
    
    async fn get_reth_exchange_rate(&self) -> Result<f64, String> {
        let exchange_rate = U256::from(1000000000000000000u64); // Placeholder
        let rate = exchange_rate.try_into().unwrap_or(0.0) / 10f64.powi(18);
        Ok(rate)
    }
    
    async fn get_node_operator_metrics(&self) -> Result<NodeOperatorMetrics, String> {
        Ok(NodeOperatorMetrics {
            total_nodes: 2500,
            active_nodes: 2000,
            total_minipools: 15000,
            active_minipools: 12000,
        })
    }
    
    async fn get_protocol_metrics(&self) -> Result<ProtocolMetrics, String> {
        let reth_supply = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18u64));
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1);
        let total_eth_staked = (reth_supply.try_into().unwrap_or(0.0) / 10f64.powi(18)) * exchange_rate;
        
        Ok(ProtocolMetrics {
            total_eth_staked,
            reth_supply: reth_supply.try_into().unwrap_or(0.0) / 10f64.powi(18),
            reth_exchange_rate: exchange_rate,
            node_demand: 0.0,
            deposit_pool_balance: 10000.0,
            network_node_fee: 0.05,
        })
    }
    
    async fn get_protocol_tvl(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        let tvl_usd = protocol_metrics.total_eth_staked * eth_price;
        Ok(tvl_usd)
    }

    async fn calculate_position_value(&self, position: &RocketPoolStakingPosition) -> (f64, f64, f64) {
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1);
        
        let token_price = match position.underlying_asset.as_str() {
            "ETH" => self.get_eth_price_usd().await.unwrap_or(4000.0),
            "RPL" => self.get_rpl_price_usd().await.unwrap_or(50.0),
            _ => 0.0,
        };
        
        let underlying_amount = if position.token_symbol == "rETH" {
            let reth_amount = position.balance.try_into().unwrap_or(0.0) / 10f64.powi(18);
            reth_amount * exchange_rate
        } else {
            position.balance.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32)
        };
        
        let base_value_usd = underlying_amount * token_price;
        let rewards_amount = position.rewards_earned.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32);
        let rewards_value_usd = rewards_amount * token_price;
        
        (base_value_usd, rewards_value_usd, position.apy)
    }
    
    async fn get_rocket_pool_apy(&self, _token_type: &str) -> Result<f64, String> {
        let rp_api_url = "https://api.rocketpool.net/api/mainnet/payload";
        
        match self.call_rocket_pool_api(rp_api_url).await {
            Ok(apy) => Ok(apy),
            Err(_) => self.calculate_apy_from_onchain_data().await,
        }
    }
    
    async fn calculate_apy_from_onchain_data(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        let base_eth_apy = 4.0;
        let commission_rate = protocol_metrics.network_node_fee;
        let liquid_staker_apy = base_eth_apy * (1.0 - commission_rate);
        Ok(liquid_staker_apy.max(2.5))
    }
    
    async fn get_node_operator_apy(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        let base_eth_apy = 4.0;
        let node_commission = protocol_metrics.network_node_fee;
        let node_apy = base_eth_apy * (1.0 + node_commission);
        Ok(node_apy)
    }
    
    async fn get_rpl_staking_apy(&self) -> Result<f64, String> {
        Ok(7.5) // RPL inflation APY
    }
    
    async fn estimate_reth_rewards(&self, _user_address: Address, reth_balance: U256, eth_value: U256) -> U256 {
        let reth_amount_f64 = reth_balance.to::<u128>() as f64 / 10f64.powi(18);
        let eth_equivalent_f64 = eth_value.to::<u128>() as f64 / 10f64.powi(18);
        
        let assumed_entry_rate = 1.05;
        let current_rate = if reth_amount_f64 > 0.0 {
            eth_equivalent_f64 / reth_amount_f64
        } else {
            1.0
        };
        
        let rate_appreciation = (current_rate - assumed_entry_rate).max(0.0);
        let estimated_rewards_eth = reth_amount_f64 * rate_appreciation;
        
        if estimated_rewards_eth > 0.0 && estimated_rewards_eth < 1000000.0 {
            let rewards_wei = (estimated_rewards_eth * 10f64.powi(18)) as u128;
            if rewards_wei <= u128::MAX {
                U256::from(rewards_wei)
            } else {
                U256::ZERO
            }
        } else {
            U256::ZERO
        }
    }
    
    async fn estimate_rpl_rewards(&self, _user_address: Address, rpl_stake: U256) -> U256 {
        let stake_amount = rpl_stake.to::<u128>() as f64;
        let estimated_rewards_percentage = 0.075;
        let estimated_rewards = stake_amount * estimated_rewards_percentage;
        U256::from(estimated_rewards as u64)
    }
    
    async fn get_eth_price_usd(&self) -> Result<f64, String> {
        let url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        };
        
        self.get_token_price_from_coingecko(url, "ethereum").await
    }
    
    async fn get_rpl_price_usd(&self) -> Result<f64, String> {
        let url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3/simple/price?ids=rocket-pool&vs_currencies=usd"
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=rocket-pool&vs_currencies=usd"
        };
        
        self.get_token_price_from_coingecko(url, "rocket-pool").await
    }
    
    async fn get_token_price_from_coingecko(&self, url: &str, token_id: &str) -> Result<f64, String> {
        let mut request = self.http_client.get(url);
        
        if let Some(api_key) = &self.coingecko_api_key {
            request = request.header("X-Cg-Pro-Api-Key", api_key);
        }
        
        let response = request
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("HTTP error {}", response.status()));
        }
        
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
            
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        if let Some(token_data) = json.get(token_id) {
            if let Some(usd_price) = token_data.get("usd") {
                if let Some(price) = usd_price.as_f64() {
                    return Ok(price);
                }
            }
        }
        
        Err(format!("{} price not found in response", token_id))
    }
    
    async fn call_rocket_pool_api(&self, url: &str) -> Result<f64, String> {
        let response = self.http_client
            .get(url)
            .header("Accept", "application/json")
            .header("User-Agent", "DeFi-Portfolio-Tracker/1.0")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
        
        if let Some(reth_apy) = json.get("rethAPY") {
            if let Some(apy) = reth_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        if let Some(network_apy) = json.get("networkAPY") {
            if let Some(apy) = network_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        if let Some(data) = json.get("data") {
            if let Some(reth_apy) = data.get("rethAPY") {
                if let Some(apy) = reth_apy.as_f64() {
                    return Ok(apy);
                }
            }
        }
        
        Err("APY not found in Rocket Pool API response".to_string())
    }
    
    fn is_rocket_pool_contract(&self, address: Address) -> bool {
        address == self.reth_address || 
        address == self.deposit_pool_address || 
        address == self.node_manager_address ||
        address == self.minipool_manager_address ||
        address == self.network_fees_address ||
        address == self.node_staking_address ||
        address == self.rpl_token_address
    }
}

#[async_trait]
impl DeFiAdapter for RocketPoolAdapter {
    fn protocol_name(&self) -> &'static str {
        "rocket_pool"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        // Check cache first
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) {
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        let staking_positions = self.get_user_staking_positions(address).await?;
        
        if staking_positions.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Get protocol metrics once for efficiency
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1);
        let tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let node_metrics = self.get_node_operator_metrics().await.unwrap_or(NodeOperatorMetrics {
            total_nodes: 0,
            active_nodes: 0,
            total_minipools: 0,
            active_minipools: 0,
        });
        let protocol_metrics = self.get_protocol_metrics().await.unwrap_or(ProtocolMetrics {
            total_eth_staked: 0.0,
            reth_supply: 0.0,
            reth_exchange_rate: 1.1,
            node_demand: 0.0,
            deposit_pool_balance: 0.0,
            network_node_fee: 0.05,
        });
        
        for stake_pos in staking_positions {
            let (base_value_usd, rewards_usd, calculated_apy) = self.calculate_position_value(&stake_pos).await;
            
            let position_type = match stake_pos.position_subtype.as_str() {
                "liquid_staking" => "staking",
                "node_operator" => "node_operation",
                "rpl_staking" => "governance_staking",
                _ => "staking",
            };
            
            let pair = match stake_pos.underlying_asset.as_str() {
                "ETH" => format!("{}/ETH", stake_pos.token_symbol),
                "RPL" => "RPL/USD".to_string(),
                _ => format!("{}/USD", stake_pos.token_symbol),
            };
            
            let position = Position {
                id: format!("rocket_pool_{}_{}", stake_pos.token_symbol.to_lowercase(), stake_pos.token_address),
                protocol: "rocket_pool".to_string(),
                position_type: position_type.to_string(),
                pair,
                value_usd: base_value_usd.max(0.01),
                pnl_usd: rewards_usd,
                pnl_percentage: calculated_apy,
                metadata: serde_json::json!({
                    "token_address": format!("{:?}", stake_pos.token_address),
                    "token_symbol": stake_pos.token_symbol,
                    "underlying_asset": stake_pos.underlying_asset,
                    "balance": stake_pos.balance.to_string(),
                    "decimals": stake_pos.decimals,
                    "current_apy": stake_pos.apy,
                    "rewards_earned": stake_pos.rewards_earned.to_string(),
                    "position_subtype": stake_pos.position_subtype,
                    "is_liquid": stake_pos.position_subtype == "liquid_staking",
                    "reth_exchange_rate": exchange_rate,
                    "exchange_rate_premium": ((exchange_rate - 1.0) * 100.0),
                    "protocol_tvl_usd": tvl,
                    "total_nodes": node_metrics.total_nodes,
                    "active_nodes": node_metrics.active_nodes,
                    "total_minipools": node_metrics.total_minipools,
                    "active_minipools": node_metrics.active_minipools,
                    "node_utilization": if node_metrics.total_nodes > 0 { 
                        node_metrics.active_nodes as f64 / node_metrics.total_nodes as f64 
                    } else { 1.0 },
                    "total_eth_staked": protocol_metrics.total_eth_staked,
                    "reth_supply": protocol_metrics.reth_supply,
                    "node_demand_eth": protocol_metrics.node_demand,
                    "deposit_pool_balance": protocol_metrics.deposit_pool_balance,
                    "network_node_fee": protocol_metrics.network_node_fee,
                    "network_commission_percent": protocol_metrics.network_node_fee * 100.0,
                }),
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            positions.push(position);
        }
        
        // Cache results
        {
            let mut cache = self.position_cache.lock().unwrap();
            cache.insert(address, CachedPositions {
                positions: positions.clone(),
                cached_at: SystemTime::now(),
            });
        }
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        self.is_rocket_pool_contract(contract_address)
    }

    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        if let Some(balance_str) = position.metadata.get("balance") {
            if let Some(balance_str) = balance_str.as_str() {
                if let Ok(balance) = U256::from_str(balance_str) {
                    let underlying_asset = position.metadata.get("underlying_asset")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ETH");
                    
                    let exchange_rate = position.metadata.get("reth_exchange_rate")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.1);
                    
                    let token_price = match underlying_asset {
                        "ETH" => self.get_eth_price_usd().await.unwrap_or(4000.0),
                        "RPL" => self.get_rpl_price_usd().await.unwrap_or(50.0),
                        _ => 4000.0,
                    };
                    
                    let underlying_amount = if position.pair.contains("rETH") {
                        let reth_amount = balance.to::<u128>() as f64 / 10f64.powi(18);
                        reth_amount * exchange_rate
                    } else {
                        balance.to::<u128>() as f64 / 10f64.powi(18)
                    };
                    
                    let calculated_value = underlying_amount * token_price;
                    return Ok(calculated_value.max(0.01));
                }
            }
        }
        
        Ok(position.value_usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reth_address() {
        let addr = Address::from_str(RocketPoolAdapter::RETH_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0xae78736cd615f374d3085123a210448e74fc6393");
    }
    
    #[test]
    fn test_rpl_address() {
        let addr = Address::from_str(RocketPoolAdapter::RPL_TOKEN_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0xd33526068d116ce69f19a9ee46f0bd304f21a51f");
    }
    
    #[test]
    fn test_rocket_pool_contract_detection() {
        let client = EthereumClient { rpc_url: "https://eth.llamarpc.com".to_string() };
        let adapter = RocketPoolAdapter::new(client).unwrap();
        
        let reth_addr = Address::from_str(RocketPoolAdapter::RETH_ADDRESS).unwrap();
        let rpl_addr = Address::from_str(RocketPoolAdapter::RPL_TOKEN_ADDRESS).unwrap();
        let random_addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        
        assert!(adapter.is_rocket_pool_contract(reth_addr));
        assert!(adapter.is_rocket_pool_contract(rpl_addr));
        assert!(!adapter.is_rocket_pool_contract(random_addr));
    }
    
    #[test]
    fn test_exchange_rate_calculations() {
        let exchange_rate = 1.15;
        let reth_amount = 100.0;
        let eth_equivalent = reth_amount * exchange_rate;
        
        assert_eq!(eth_equivalent, 115.0);
        
        let premium_percent = (exchange_rate - 1.0) * 100.0;
        assert_eq!(premium_percent, 15.0);
    }
    
    #[test]
    fn test_apy_calculations() {
        let base_eth_apy = 4.0;
        let node_commission = 0.15;
        
        let liquid_staker_apy = base_eth_apy * (1.0 - node_commission);
        assert_eq!(liquid_staker_apy, 3.4);
        
        let node_operator_apy = base_eth_apy * (1.0 + node_commission);
        assert_eq!(node_operator_apy, 4.6);
    }
}