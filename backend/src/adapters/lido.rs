use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use reqwest;
use serde::Deserialize;
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
struct CoinGeckoToken {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, Clone)]
struct ValidatorMetrics {
    total_validators: u64,
    active_validators: u64,
    exited_validators: u64,
    slashed_validators: u64,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct LidoStakingPosition {
    token_address: Address,
    token_symbol: String,
    balance: U256,
    decimals: u8,
    underlying_asset: String,
    apy: f64,
    rewards_earned: U256,
}

// Lido contract interfaces
sol! {
    #[sol(rpc)]
    interface ILidoStETH {
        function balanceOf(address account) external view returns (uint256);
        function sharesOf(address account) external view returns (uint256);
        function getSharesByPooledEth(uint256 ethAmount) external view returns (uint256);
        function getPooledEthByShares(uint256 sharesAmount) external view returns (uint256);
        function getTotalShares() external view returns (uint256);
        function getTotalPooledEther() external view returns (uint256);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
    }
    
    #[sol(rpc)]
    interface IWstETH {
        function balanceOf(address account) external view returns (uint256);
        function stEthPerToken() external view returns (uint256);
        function getWstETHByStETH(uint256 stETHAmount) external view returns (uint256);
        function getStETHByWstETH(uint256 wstETHAmount) external view returns (uint256);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
    }
    
    #[sol(rpc)]
    interface ILidoWithdrawalQueue {
        function getWithdrawalRequests(address owner) external view returns (uint256[] memory requestIds);
        function getWithdrawalStatus(uint256[] memory requestIds) external view returns (
            WithdrawalRequestStatus[] memory statuses
        );
        
        struct WithdrawalRequestStatus {
            uint256 amountOfStETH;
            uint256 amountOfShares;
            address owner;
            uint256 timestamp;
            bool isFinalized;
            bool isClaimed;
        }
    }
}

pub struct LidoAdapter {
    #[allow(dead_code)]
    client: EthereumClient,
    steth_address: Address,
    wsteth_address: Address,
    withdrawal_queue_address: Address,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    http_client: reqwest::Client,
    coingecko_api_key: Option<String>,
}

impl LidoAdapter {
    const STETH_ADDRESS: &'static str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";
    const WSTETH_ADDRESS: &'static str = "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0";
    const WITHDRAWAL_QUEUE_ADDRESS: &'static str = "0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let steth_address = Address::from_str(Self::STETH_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid stETH address: {}", e)))?;
            
        let wsteth_address = Address::from_str(Self::WSTETH_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid wstETH address: {}", e)))?;
            
        let withdrawal_queue_address = Address::from_str(Self::WITHDRAWAL_QUEUE_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid withdrawal queue address: {}", e)))?;
        
        Ok(Self {
            client,
            steth_address,
            wsteth_address,
            withdrawal_queue_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        })
    }
    
    async fn get_user_staking_positions(&self, address: Address) -> Result<Vec<LidoStakingPosition>, AdapterError> {
        let mut positions = Vec::new();
        
        if let Some(steth_position) = self.get_steth_position(address).await? {
            positions.push(steth_position);
        }
        
        if let Some(wsteth_position) = self.get_wsteth_position(address).await? {
            positions.push(wsteth_position);
        }
        
        let withdrawal_positions = self.get_withdrawal_positions(address).await?;
        positions.extend(withdrawal_positions);
        
        Ok(positions)
    }
    
    async fn get_steth_position(&self, user_address: Address) -> Result<Option<LidoStakingPosition>, AdapterError> {
        // Placeholder for contract call
        let balance = U256::ZERO;
        let shares = U256::ZERO;
        
        if balance == U256::ZERO {
            return Ok(None);
        }
        
        let apy = self.get_lido_apy("stETH").await.unwrap_or(4.5);
        let rewards_earned = self.estimate_steth_rewards(user_address, shares).await;
        
        Ok(Some(LidoStakingPosition {
            token_address: self.steth_address,
            token_symbol: "stETH".to_string(),
            balance,
            decimals: 18,
            underlying_asset: "ETH".to_string(),
            apy,
            rewards_earned,
        }))
    }
    
    async fn get_wsteth_position(&self, user_address: Address) -> Result<Option<LidoStakingPosition>, AdapterError> {
        // Placeholder for contract call
        let wsteth_balance = U256::ZERO;
        
        if wsteth_balance == U256::ZERO {
            return Ok(None);
        }
        
        let apy = self.get_lido_apy("wstETH").await.unwrap_or(4.5);
        let rewards_earned = self.estimate_wsteth_rewards(user_address, wsteth_balance).await;
        
        Ok(Some(LidoStakingPosition {
            token_address: self.wsteth_address,
            token_symbol: "wstETH".to_string(),
            balance: wsteth_balance,
            decimals: 18,
            underlying_asset: "ETH".to_string(),
            apy,
            rewards_earned,
        }))
    }
    
    async fn get_withdrawal_positions(&self, _user_address: Address) -> Result<Vec<LidoStakingPosition>, AdapterError> {
        // Placeholder for contract call
        let request_ids: Vec<u64> = vec![];
        
        if request_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        Ok(Vec::new())
    }
    
    async fn get_steth_peg_price(&self) -> Result<f64, String> {
        match self.calculate_steth_peg_from_protocol().await {
            Ok(price) => Ok(price),
            Err(_) => Ok(0.998), // Fallback
        }
    }
    
    async fn calculate_steth_peg_from_protocol(&self) -> Result<f64, String> {
        // Placeholder for contract calls
        let total_pooled_eth = U256::from(1000000u64);
        let total_shares = U256::from(1000000u64);
        
        if total_shares == U256::ZERO {
            return Err("Total shares is zero".to_string());
        }
        
        let peg_price = total_pooled_eth.try_into().unwrap_or(0.0) / total_shares.try_into().unwrap_or(1.0);
        Ok(peg_price)
    }
    
    async fn get_validator_metrics(&self) -> Result<ValidatorMetrics, String> {
        // Placeholder calculation based on TVL
        let total_pooled_eth = U256::from(1000000u64);
        let total_eth_f64 = total_pooled_eth.try_into().unwrap_or(0.0) / 10f64.powi(18);
        
        let estimated_validators = (total_eth_f64 / 32.0) as u64;
        let active_validators = (estimated_validators as f64 * 0.98) as u64;
        let exited_validators = (estimated_validators as f64 * 0.015) as u64;
        let slashed_validators = (estimated_validators as f64 * 0.005) as u64;
        
        Ok(ValidatorMetrics {
            total_validators: estimated_validators,
            active_validators,
            exited_validators,
            slashed_validators,
        })
    }
    
    async fn estimate_withdrawal_queue_time(&self) -> Result<u64, String> {
        Ok(259200) // 3 days in seconds
    }
    
    async fn get_protocol_tvl(&self) -> Result<f64, String> {
        // Placeholder for contract call
        let total_pooled_eth = U256::from(1000000u64);
        let total_eth_f64 = total_pooled_eth.try_into().unwrap_or(0.0) / 10f64.powi(18);
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        let tvl_usd = total_eth_f64 * eth_price;
        
        Ok(tvl_usd)
    }

    async fn calculate_position_value(&self, position: &LidoStakingPosition) -> (f64, f64, f64) {
        let peg_price = self.get_steth_peg_price().await.unwrap_or(1.0);
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        
        let eth_amount = if position.token_symbol == "wstETH" {
            self.convert_wsteth_to_steth_amount(position.balance).await
                .unwrap_or(position.balance.try_into().unwrap_or(0.0) / 10f64.powi(18))
        } else {
            position.balance.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32)
        };
        
        let base_value_usd = eth_amount * eth_price;
        let peg_adjusted_value = base_value_usd * peg_price;
        let rewards_eth = position.rewards_earned.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32);
        let rewards_value_usd = rewards_eth * eth_price;
        
        let peg_deviation = ((peg_price - 1.0).abs() * 100.0).min(10.0);
        let adjusted_pnl = position.apy - peg_deviation;
        
        (peg_adjusted_value, rewards_value_usd, adjusted_pnl)
    }
    
    async fn get_lido_apy(&self, _token_type: &str) -> Result<f64, String> {
        let lido_api_url = "https://stake.lido.fi/api/sma-steth-apr";
        
        match self.call_lido_api(lido_api_url).await {
            Ok(apy) => Ok(apy),
            Err(_) => self.calculate_apy_from_onchain_data().await,
        }
    }
    
    async fn calculate_apy_from_onchain_data(&self) -> Result<f64, String> {
        // Placeholder for contract calls
        let _total_pooled_eth = U256::from(1000000u64);
        let total_shares = U256::from(1000000u64);
        
        if total_shares == U256::ZERO {
            return Err("Total shares is zero".to_string());
        }
        
        let base_apy = 4.5;
        let lido_apy = base_apy * 0.9; // Lido takes 10% fee
        
        Ok(lido_apy)
    }
    
    async fn estimate_steth_rewards(&self, _user_address: Address, user_shares: U256) -> U256 {
        let estimated_rewards_percentage = 0.02;
        let balance_f64 = user_shares.try_into().unwrap_or(0.0);
        let estimated_rewards = balance_f64 * estimated_rewards_percentage;
        
        U256::from(estimated_rewards as u64)
    }
    
    async fn estimate_wsteth_rewards(&self, _user_address: Address, wsteth_balance: U256) -> U256 {
        let balance_f64 = wsteth_balance.try_into().unwrap_or(0.0);
        let estimated_rewards_percentage = 0.045;
        let estimated_rewards = balance_f64 * estimated_rewards_percentage;
        
        U256::from(estimated_rewards as u64)
    }
    
    async fn convert_wsteth_to_steth_amount(&self, wsteth_amount: U256) -> Result<f64, String> {
        // Placeholder for contract call
        let steth_amount = wsteth_amount; // 1:1 placeholder
        Ok(steth_amount.try_into().unwrap_or(0.0) / 10f64.powi(18))
    }
    
    async fn get_eth_price_usd(&self) -> Result<f64, String> {
        let url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        };
        
        let mut request = self.http_client.get(url);
        
        if let Some(api_key) = &self.coingecko_api_key {
            request = request.header("X-Cg-Pro-Api-Key", api_key);
        }
        
        let response = request.send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("HTTP error {}", response.status()));
        }
        
        let response_text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
            
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        if let Some(ethereum) = json.get("ethereum") {
            if let Some(usd_price) = ethereum.get("usd") {
                if let Some(price) = usd_price.as_f64() {
                    return Ok(price);
                }
            }
        }
        
        Err("ETH price not found in response".to_string())
    }
    
    async fn call_lido_api(&self, url: &str) -> Result<f64, String> {
        let response = self.http_client
            .get(url)
            .header("Accept", "application/json")
            .header("User-Agent", "DeFi-Portfolio-Tracker/1.0")
            .send().await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        let response_text = response.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;
            
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        if let Some(apr) = json.as_f64() {
            return Ok(apr);
        }
        
        if let Some(data) = json.get("data") {
            if let Some(apr) = data.as_f64() {
                return Ok(apr);
            }
        }
        
        if let Some(apr) = json.get("apr") {
            if let Some(apr_val) = apr.as_f64() {
                return Ok(apr_val);
            }
        }
        
        Err("APY not found in Lido API response".to_string())
    }
    
    fn is_lido_contract(&self, address: Address) -> bool {
        address == self.steth_address || 
        address == self.wsteth_address || 
        address == self.withdrawal_queue_address
    }
    
    #[allow(dead_code)]
    fn get_lido_token_symbol(&self, address: Address) -> String {
        if address == self.steth_address {
            "stETH".to_string()
        } else if address == self.wsteth_address {
            "wstETH".to_string()
        } else if address == self.withdrawal_queue_address {
            "stETH-withdrawal".to_string()
        } else {
            "UNKNOWN-LIDO".to_string()
        }
    }
}

#[async_trait]
impl DeFiAdapter for LidoAdapter {
    fn protocol_name(&self) -> &'static str {
        "lido"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        // Check cache first (5 minute TTL)
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
        
        // Get protocol metrics once for all positions
        let peg_price = self.get_steth_peg_price().await.unwrap_or(1.0);
        let tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let validator_metrics = self.get_validator_metrics().await.unwrap_or(ValidatorMetrics {
            total_validators: 0,
            active_validators: 0,
            exited_validators: 0,
            slashed_validators: 0,
        });
        let queue_time = self.estimate_withdrawal_queue_time().await.unwrap_or(259200);
        
        for stake_pos in staking_positions {
            let (value_usd, rewards_usd, apy) = self.calculate_position_value(&stake_pos).await;
            
            let position_type = if stake_pos.token_symbol.contains("withdrawal") {
                "withdrawal"
            } else {
                "staking"
            };
            
            let position = Position {
                id: format!("lido_{}_{}", stake_pos.token_symbol.to_lowercase(), stake_pos.token_address),
                protocol: "lido".to_string(),
                position_type: position_type.to_string(),
                pair: format!("{}/ETH", stake_pos.token_symbol),
                value_usd: value_usd.max(0.01),
                pnl_usd: rewards_usd,
                pnl_percentage: apy,
                metadata: serde_json::json!({
                    "token_address": format!("{:?}", stake_pos.token_address),
                    "token_symbol": stake_pos.token_symbol,
                    "underlying_asset": stake_pos.underlying_asset,
                    "balance": stake_pos.balance.to_string(),
                    "decimals": stake_pos.decimals,
                    "current_apy": stake_pos.apy,
                    "rewards_earned": stake_pos.rewards_earned.to_string(),
                    "staking_provider": "lido",
                    "is_liquid": position_type == "staking",
                    "peg_price": peg_price,
                    "peg_deviation_percent": ((peg_price - 1.0).abs() * 100.0),
                    "protocol_tvl_usd": tvl,
                    "validator_count_total": validator_metrics.total_validators,
                    "validator_count_active": validator_metrics.active_validators,
                    "validator_count_exited": validator_metrics.exited_validators,
                    "validator_count_slashed": validator_metrics.slashed_validators,
                    "withdrawal_queue_time_seconds": queue_time,
                    "withdrawal_queue_time_days": queue_time / 86400,
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
        self.is_lido_contract(contract_address)
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        let usd_value = position.value_usd.to_string().parse::<f64>().unwrap_or(0.0);
        
        let final_usd_value = if usd_value > 0.0 {
            usd_value
        } else {
            let token_amount = 1.0;
            (token_amount / 1e18) * eth_price
        };
        
        Ok(final_usd_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_contract_addresses() {
        let addr = Address::from_str(LidoAdapter::STETH_ADDRESS);
        assert!(addr.is_ok());
        
        let addr = Address::from_str(LidoAdapter::WSTETH_ADDRESS);
        assert!(addr.is_ok());
        
        let addr = Address::from_str(LidoAdapter::WITHDRAWAL_QUEUE_ADDRESS);
        assert!(addr.is_ok());
    }
    
    #[test]
    fn test_contract_detection() {
        let client = EthereumClient { rpc_url: "https://eth.llamarpc.com".to_string() };
        let adapter = LidoAdapter::new(client).unwrap();
        
        let steth_addr = Address::from_str(LidoAdapter::STETH_ADDRESS).unwrap();
        let wsteth_addr = Address::from_str(LidoAdapter::WSTETH_ADDRESS).unwrap();
        let random_addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        
        assert!(adapter.is_lido_contract(steth_addr));
        assert!(adapter.is_lido_contract(wsteth_addr));
        assert!(!adapter.is_lido_contract(random_addr));
    }
    
    #[test]
    fn test_token_symbols() {
        let client = EthereumClient { rpc_url: "https://eth.llamarpc.com".to_string() };
        let adapter = LidoAdapter::new(client).unwrap();
        
        let steth_addr = Address::from_str(LidoAdapter::STETH_ADDRESS).unwrap();
        let wsteth_addr = Address::from_str(LidoAdapter::WSTETH_ADDRESS).unwrap();
        
        assert_eq!(adapter.get_lido_token_symbol(steth_addr), "stETH");
        assert_eq!(adapter.get_lido_token_symbol(wsteth_addr), "wstETH");
    }
}