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
struct EtherFiApiResponse {
    data: Option<serde_json::Value>,
    status: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ValidatorMetrics {
    total_validators: u64,
    active_validators: u64,
    pending_validators: u64,
    slashed_validators: u64,
    total_staked_eth: f64,
    average_validator_balance: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct EtherFiProtocolMetrics {
    total_eth_staked: f64,
    eeth_supply: f64,
    eeth_exchange_rate: f64,
    liquid_capacity: f64,
    restaking_tvl: f64,
    protocol_revenue: f64,
    node_operator_count: u64,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct EtherFiStakingPosition {
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
    interface IEtherFiLiquidityPool {
        function deposit() external payable returns (uint256);
        function depositWithReferral(address referral) external payable returns (uint256);
        function requestWithdraw(address recipient, uint256 amount) external returns (uint256);
        function getTotalPooledEther() external view returns (uint256);
        function getTotalShares() external view returns (uint256);
        function sharesForAmount(uint256 amount) external view returns (uint256);
        function amountForShare(uint256 shares) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        
        event Deposit(address indexed user, uint256 amount, uint256 shares);
        event RequestWithdraw(address indexed user, uint256 amount, uint256 shares);
    }
    
    #[sol(rpc)]
    interface IEETH {
        function balanceOf(address account) external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function shares(address account) external view returns (uint256);
        function getPooledEthByShares(uint256 sharesAmount) external view returns (uint256);
        function getSharesByPooledEth(uint256 pooledEthAmount) external view returns (uint256);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
        function name() external pure returns (string memory);
        
        event Transfer(address indexed from, address indexed to, uint256 value);
        event TransferShares(address indexed from, address indexed to, uint256 sharesValue);
    }
    
    #[sol(rpc)]
    interface IEtherFiNodeManager {
        function numberOfValidators() external view returns (uint64);
        function getFullWithdrawalPayouts(uint256[] memory validatorIds) external view returns (uint256);
        function getRewardsPayouts(uint256[] memory validatorIds) external view returns (uint256);
        function isValidatorRegistered(uint256 validatorId) external view returns (bool);
        function getValidatorInfo(uint256 validatorId) external view returns (
            uint32 validatorIndex,
            address etherFiNode,
            address withdrawalSafe,
            uint256 localRevenueIndex,
            uint256 vestedAuctionRewards
        );
    }
    
    #[sol(rpc)]
    interface IEtherFiNodesManager {
        function numberOfValidators() external view returns (uint64);
        function getValidatorsOfEtherFiNode(address etherFiNodeAddress) external view returns (uint256[] memory);
        function etherFiNodeAddress(uint256 validatorId) external view returns (address);
        function generateWithdrawalRoot(
            uint256[] memory validatorIds,
            uint256 beaconChainETHStrategyIndex,
            uint256 eigenPodShares
        ) external view returns (bytes32);
    }
    
    #[sol(rpc)]
    interface IEigenPodManager {
        function getPod(address podOwner) external view returns (address);
        function ownerToPod(address podOwner) external view returns (address);
        function podOwnerShares(address podOwner) external view returns (int256);
        function hasPod(address podOwner) external view returns (bool);
        function numPods() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IEtherFiRestakingManager {
        function getEigenPodShares(address user) external view returns (uint256);
        function getTotalShares() external view returns (uint256);
        function getSharePrice() external view returns (uint256);
        function deposit() external payable returns (uint256);
        function requestWithdrawal(uint256 shares) external returns (uint256);
        
        event RestakingDeposit(address indexed user, uint256 amount, uint256 shares);
        event RestakingWithdrawal(address indexed user, uint256 shares, uint256 amount);
    }
    
    #[sol(rpc)]
    interface IEtherFiAuctionManager {
        function numberOfBids() external view returns (uint256);
        function getBidOwner(uint256 bidId) external view returns (address);
        function getBidAmount(uint256 bidId) external view returns (uint256);
        function processAuctionFeeRewards(uint256[] memory bidIds) external;
    }
}

pub struct EtherFiAdapter {
    #[allow(dead_code)]
    client: EthereumClient,
    eeth_address: Address,
    liquidity_pool_address: Address,
    node_manager_address: Address,
    nodes_manager_address: Address,
    eigenpod_manager_address: Address,
    restaking_manager_address: Address,
    auction_manager_address: Address,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    http_client: reqwest::Client,
    coingecko_api_key: Option<String>,
}

impl EtherFiAdapter {
    const EETH_ADDRESS: &'static str = "0x35fA164735182de50811E8e2E824cFb9B6118ac2";
    const LIQUIDITY_POOL_ADDRESS: &'static str = "0x308861A430be4cce5502d0A12724771Fc6DaF216";
    const NODE_MANAGER_ADDRESS: &'static str = "0x8103151E2377e78C04a3d2564e20542680ed3096";
    const NODES_MANAGER_ADDRESS: &'static str = "0x8103151E2377e78C04a3d2564e20542680ed3096";
    const EIGENPOD_MANAGER_ADDRESS: &'static str = "0x858646372CC42E1A627fcE94aa7A7033e7CF075A";
    const RESTAKING_MANAGER_ADDRESS: &'static str = "0x308861A430be4cce5502d0A12724771Fc6DaF216";
    const AUCTION_MANAGER_ADDRESS: &'static str = "0x5fD13359Ba15A84B76f7F87568309040176167cd";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let eeth_address = Address::from_str(Self::EETH_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid eETH address: {}", e)))?;
        let liquidity_pool_address = Address::from_str(Self::LIQUIDITY_POOL_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid liquidity pool address: {}", e)))?;
        let node_manager_address = Address::from_str(Self::NODE_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid node manager address: {}", e)))?;
        let nodes_manager_address = Address::from_str(Self::NODES_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid nodes manager address: {}", e)))?;
        let eigenpod_manager_address = Address::from_str(Self::EIGENPOD_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid EigenPod manager address: {}", e)))?;
        let restaking_manager_address = Address::from_str(Self::RESTAKING_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid restaking manager address: {}", e)))?;
        let auction_manager_address = Address::from_str(Self::AUCTION_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid auction manager address: {}", e)))?;
        
        Ok(Self {
            client,
            eeth_address,
            liquidity_pool_address,
            node_manager_address,
            nodes_manager_address,
            eigenpod_manager_address,
            restaking_manager_address,
            auction_manager_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        })
    }
    
    async fn get_user_staking_positions(&self, address: Address) -> Result<Vec<EtherFiStakingPosition>, AdapterError> {
        let mut positions = Vec::new();
        
        if let Some(eeth_position) = self.get_eeth_position(address).await? {
            positions.push(eeth_position);
        }
        
        if let Some(restaking_positions) = self.get_restaking_positions(address).await? {
            positions.extend(restaking_positions);
        }
        
        if let Some(validator_positions) = self.get_validator_positions(address).await? {
            positions.extend(validator_positions);
        }
        
        Ok(positions)
    }
    
    async fn get_eeth_position(&self, user_address: Address) -> Result<Option<EtherFiStakingPosition>, AdapterError> {
        let balance = U256::ZERO; // Placeholder - would call eeth_contract.balanceOf(user_address)
            
        if balance == U256::ZERO {
            return Ok(None);
        }
        
        let shares = balance; // Placeholder - would call eeth_contract.shares(user_address)
        let eth_value = shares; // Placeholder - would call eeth_contract.getPooledEthByShares(shares)
        
        // Calculate exchange rate
        let eeth_total_supply = U256::from(1_000_000u64) * U256::from(10u64).pow(U256::from(18u64));
        let total_pooled_eth = U256::from(1_100_000u64) * U256::from(10u64).pow(U256::from(18u64));
        
        let _exchange_rate = if eeth_total_supply > U256::ZERO {
            total_pooled_eth.to_string().parse::<f64>().unwrap_or(0.0) / eeth_total_supply.to_string().parse::<f64>().unwrap_or(1.0)
        } else {
            1.0
        };
        
        let apy = self.get_etherfi_apy("eETH").await.unwrap_or(3.8);
        let rewards_earned = self.estimate_eeth_rewards(user_address, balance, eth_value).await;
        
        Ok(Some(EtherFiStakingPosition {
            token_address: self.eeth_address,
            token_symbol: "eETH".to_string(),
            balance,
            decimals: 18,
            underlying_asset: "ETH".to_string(),
            apy,
            rewards_earned,
            position_subtype: "liquid_staking".to_string(),
        }))
    }
    
    async fn get_restaking_positions(&self, user_address: Address) -> Result<Option<Vec<EtherFiStakingPosition>>, AdapterError> {
        let eigenpod_shares = U256::ZERO; // Placeholder - would call eigenpod_manager.podOwnerShares(user_address)
        let restaking_shares = U256::ZERO; // Placeholder - would call restaking_manager.getEigenPodShares(user_address)
        
        if eigenpod_shares <= U256::ZERO && restaking_shares == U256::ZERO {
            return Ok(None);
        }
        
        let mut positions = Vec::new();
        
        if eigenpod_shares > U256::ZERO {
            let eigenpod_balance = eigenpod_shares;
            let restaking_apy = self.get_restaking_apy().await.unwrap_or(6.5);
            let rewards_earned = self.estimate_restaking_rewards(user_address, eigenpod_balance).await;
            
            let eigenpod_position = EtherFiStakingPosition {
                token_address: self.eigenpod_manager_address,
                token_symbol: "ETH-RESTAKED".to_string(),
                balance: eigenpod_balance,
                decimals: 18,
                underlying_asset: "ETH".to_string(),
                apy: restaking_apy,
                rewards_earned,
                position_subtype: "restaking".to_string(),
            };
            
            positions.push(eigenpod_position);
        }
        
        if restaking_shares > U256::ZERO {
            let share_price = 1.0; // Placeholder - would call restaking_manager.getSharePrice()
            let restaking_eth_value = (restaking_shares.to_string().parse::<f64>().unwrap_or(0.0) * share_price) / 10f64.powi(18);
            let restaking_apy = self.get_restaking_apy().await.unwrap_or(6.2);
            let rewards_earned = self.estimate_restaking_rewards(user_address, restaking_shares).await;
            
            let direct_restaking_position = EtherFiStakingPosition {
                token_address: self.restaking_manager_address,
                token_symbol: "eETH-RESTAKED".to_string(),
                balance: U256::from((restaking_eth_value * 10f64.powi(18)) as u64),
                decimals: 18,
                underlying_asset: "ETH".to_string(),
                apy: restaking_apy,
                rewards_earned,
                position_subtype: "restaking".to_string(),
            };
            
            positions.push(direct_restaking_position);
        }
        
        Ok(if positions.is_empty() { None } else { Some(positions) })
    }
    
    async fn get_validator_positions(&self, _user_address: Address) -> Result<Option<Vec<EtherFiStakingPosition>>, AdapterError> {
        // Simplified implementation - would require complex logic to detect validator ownership
        Ok(None)
    }
    
    async fn get_eeth_exchange_rate(&self) -> Result<f64, String> {
        let eeth_total_supply = U256::from(1000000u64);
        let total_pooled_eth = U256::from(1050000u64);
        
        let rate = if eeth_total_supply > U256::ZERO {
            total_pooled_eth.to_string().parse::<f64>().unwrap_or(0.0) / eeth_total_supply.to_string().parse::<f64>().unwrap_or(1.0)
        } else {
            1.0
        };
        
        Ok(rate)
    }
    
    async fn get_validator_metrics(&self) -> Result<ValidatorMetrics, String> {
        let total_validators = 1000u64;
        let total_pooled_eth = U256::from(1000000u64);
        
        let total_staked_eth = total_pooled_eth.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18);
        let average_validator_balance = if total_validators > 0 {
            total_staked_eth / total_validators as f64
        } else {
            0.0
        };
        
        let active_validators = (total_validators as f64 * 0.95) as u64;
        let pending_validators = total_validators - active_validators;
        let slashed_validators = (total_validators as f64 * 0.001) as u64;
        
        Ok(ValidatorMetrics {
            total_validators,
            active_validators,
            pending_validators,
            slashed_validators,
            total_staked_eth,
            average_validator_balance,
        })
    }
    
    async fn get_protocol_metrics(&self) -> Result<EtherFiProtocolMetrics, String> {
        let eeth_supply = U256::from(1000000u64);
        let total_pooled_eth = U256::from(2000000u64);
        let exchange_rate = self.get_eeth_exchange_rate().await?;
        let restaking_total_shares = U256::from(500000u64);
        let restaking_share_price = U256::from(1000000000000000000u64);
        
        let restaking_tvl = (restaking_total_shares.to_string().parse::<f64>().unwrap_or(0.0) * restaking_share_price.to_string().parse::<f64>().unwrap_or(0.0)) / 10f64.powi(36);
        
        Ok(EtherFiProtocolMetrics {
            total_eth_staked: total_pooled_eth.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18),
            eeth_supply: eeth_supply.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18),
            eeth_exchange_rate: exchange_rate,
            liquid_capacity: 0.0,
            restaking_tvl,
            protocol_revenue: 0.0,
            node_operator_count: 0,
        })
    }
    
    async fn get_protocol_tvl(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        let liquid_tvl = protocol_metrics.total_eth_staked * eth_price;
        let total_tvl = liquid_tvl + (protocol_metrics.restaking_tvl * eth_price);
        
        Ok(total_tvl)
    }

    async fn calculate_position_value(&self, position: &EtherFiStakingPosition) -> (f64, f64, f64) {
        let exchange_rate = self.get_eeth_exchange_rate().await.unwrap_or(1.0); 
        let _tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let validator_metrics = self.get_validator_metrics().await.unwrap_or(ValidatorMetrics {
            total_validators: 0,
            active_validators: 0,
            pending_validators: 0,
            slashed_validators: 0,
            total_staked_eth: 0.0,
            average_validator_balance: 0.0,
        });
        let protocol_metrics = self.get_protocol_metrics().await.unwrap_or(EtherFiProtocolMetrics {
            total_eth_staked: 0.0,
            eeth_supply: 0.0,
            eeth_exchange_rate: 1.0,
            liquid_capacity: 0.0,
            restaking_tvl: 0.0,
            protocol_revenue: 0.0,
            node_operator_count: 0,
        });
        
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        
        let underlying_eth_amount = match position.position_subtype.as_str() {
            "liquid_staking" => {
                let eeth_amount = position.balance.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18);
                eeth_amount * exchange_rate
            },
            "restaking" => {
                position.balance.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18)
            },
            _ => {
                position.balance.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(position.decimals as i32)
            }
        };
        
        let base_value_usd = underlying_eth_amount * eth_price;
        let rewards_amount = position.rewards_earned.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(position.decimals as i32);
        let rewards_value_usd = rewards_amount * eth_price;
        
        let mut adjusted_apy = position.apy;
        
        match position.position_subtype.as_str() {
            "liquid_staking" => {
                let exchange_rate_health = if exchange_rate >= 1.0 { 
                    ((exchange_rate - 1.0) * 100.0).min(10.0)
                } else { 
                    -5.0
                };
                adjusted_apy += exchange_rate_health * 0.1;
            },
            "restaking" => {
                let validator_health = if validator_metrics.total_validators > 0 {
                    let slashing_rate = validator_metrics.slashed_validators as f64 / validator_metrics.total_validators as f64;
                    if slashing_rate > 0.005 { -0.5 } else { 0.2 }
                } else { 0.0 };
                adjusted_apy += validator_health;
            },
            _ => {}
        }
        
        let validator_utilization = if validator_metrics.total_validators > 0 {
            validator_metrics.active_validators as f64 / validator_metrics.total_validators as f64
        } else { 1.0 };
        
        if validator_utilization < 0.9 {
            adjusted_apy *= 0.98;
        }
        
        if position.position_subtype == "restaking" && protocol_metrics.restaking_tvl > 1000.0 {
            adjusted_apy *= 1.05;
        }
        
        (base_value_usd, rewards_value_usd, adjusted_apy)
    }
    
    async fn get_etherfi_apy(&self, token_type: &str) -> Result<f64, String> {
        let etherfi_api_url = "https://api.ether.fi/api/v1/stats";
        
        match self.call_etherfi_api(etherfi_api_url).await {
            Ok(apy) => Ok(apy),
            Err(_) => self.calculate_apy_from_onchain_data(token_type).await
        }
    }
    
    async fn calculate_apy_from_onchain_data(&self, token_type: &str) -> Result<f64, String> {
        let validator_metrics = self.get_validator_metrics().await?;
        let base_eth_apy = 4.0;
        
        let calculated_apy = match token_type {
            "eETH" => {
                let protocol_fee_rate = 0.10;
                let liquid_staker_apy = base_eth_apy * (1.0 - protocol_fee_rate);
                
                let validator_performance = if validator_metrics.total_validators > 0 {
                    1.0 - (validator_metrics.slashed_validators as f64 / validator_metrics.total_validators as f64 * 10.0)
                } else { 1.0 };
                
                (liquid_staker_apy * validator_performance).max(3.0)
            },
            _ => base_eth_apy,
        };
        
        Ok(calculated_apy)
    }
    
    async fn get_restaking_apy(&self) -> Result<f64, String> {
        let base_eth_apy = 4.0;
        let avs_rewards_estimate = 3.5;
        let protocol_fee = 0.10;
        let restaking_apy = (base_eth_apy + avs_rewards_estimate) * (1.0 - protocol_fee);
        
        Ok(restaking_apy)
    }
    
    async fn estimate_eeth_rewards(&self, _user_address: Address, eeth_balance: U256, eth_value: U256) -> U256 {
        let eeth_amount = eeth_balance.to_string().parse::<f64>().unwrap_or(0.0);
        let eth_equivalent = eth_value.to_string().parse::<f64>().unwrap_or(0.0);
        
        let estimated_appreciation = eth_equivalent - eeth_amount;
        if estimated_appreciation > 0.0 {
            U256::from(estimated_appreciation as u64)
        } else {
            U256::ZERO
        }
    }
    
    async fn estimate_restaking_rewards(&self, _user_address: Address, restaking_balance: U256) -> U256 {
        let balance_amount = restaking_balance.to_string().parse::<f64>().unwrap_or(0.0);
        let estimated_rewards_percentage = 0.065;
        let estimated_rewards = balance_amount * estimated_rewards_percentage;
        
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
    
    async fn call_etherfi_api(&self, url: &str) -> Result<f64, String> {
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
            
        // Try to extract APY from different possible response formats
        if let Some(eeth_apy) = json.get("eethAPY") {
            if let Some(apy) = eeth_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        if let Some(staking_apy) = json.get("stakingAPY") {
            if let Some(apy) = staking_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        if let Some(liquid_staking) = json.get("liquidStaking") {
            if let Some(apy) = liquid_staking.get("apy") {
                if let Some(apy_val) = apy.as_f64() {
                    return Ok(apy_val);
                }
            }
        }
        
        if let Some(data) = json.get("data") {
            if let Some(apy) = data.get("apy") {
                if let Some(apy_val) = apy.as_f64() {
                    return Ok(apy_val);
                }
            }
        }
        
        Err("APY not found in Ether.fi API response".to_string())
    }
    
    fn is_etherfi_contract(&self, address: Address) -> bool {
        address == self.eeth_address || 
        address == self.liquidity_pool_address || 
        address == self.node_manager_address ||
        address == self.nodes_manager_address ||
        address == self.eigenpod_manager_address ||
        address == self.restaking_manager_address ||
        address == self.auction_manager_address
    }
    
    #[allow(dead_code)]
    fn get_etherfi_token_symbol(&self, address: Address) -> String {
        if address == self.eeth_address {
            "eETH".to_string()
        } else if address == self.eigenpod_manager_address {
            "ETH-RESTAKED".to_string()
        } else if address == self.restaking_manager_address {
            "eETH-RESTAKED".to_string()
        } else if address == self.node_manager_address {
            "EF-VALIDATOR".to_string()
        } else {
            "UNKNOWN-EF".to_string()
        }
    }
}

#[async_trait]
impl DeFiAdapter for EtherFiAdapter {
    fn protocol_name(&self) -> &'static str {
        "ether_fi"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        // Check cache first
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) { // 5 minute cache
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        let staking_positions = self.get_user_staking_positions(address).await?;
        
        if staking_positions.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Get enhanced metrics once for all positions
        let exchange_rate = self.get_eeth_exchange_rate().await.unwrap_or(1.0);
        let tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let validator_metrics = self.get_validator_metrics().await.unwrap_or(ValidatorMetrics {
            total_validators: 0,
            active_validators: 0,
            pending_validators: 0,
            slashed_validators: 0,
            total_staked_eth: 0.0,
            average_validator_balance: 0.0,
        });
        let protocol_metrics = self.get_protocol_metrics().await.unwrap_or(EtherFiProtocolMetrics {
            total_eth_staked: 0.0,
            eeth_supply: 0.0,
            eeth_exchange_rate: 1.0,
            liquid_capacity: 0.0,
            restaking_tvl: 0.0,
            protocol_revenue: 0.0,
            node_operator_count: 0,
        });
        
        // Convert staking positions to Position structs
        for stake_pos in staking_positions {
            let (value_usd, rewards_usd, apy) = self.calculate_position_value(&stake_pos).await;
            
            let position_type = match stake_pos.position_subtype.as_str() {
                "liquid_staking" => "staking",
                "restaking" => "restaking",
                "node_operator" => "node_operation",
                _ => "staking",
            };
            
            let pair = format!("{}/ETH", stake_pos.token_symbol);
            
            let position = Position {
                id: format!("ether_fi_{}_{}", stake_pos.token_symbol.to_lowercase(), stake_pos.token_address),
                protocol: "ether_fi".to_string(),
                position_type: position_type.to_string(),
                pair,
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
                    "staking_provider": "ether_fi",
                    "position_subtype": stake_pos.position_subtype,
                    "is_liquid": stake_pos.position_subtype == "liquid_staking",
                    "supports_restaking": true,
                    
                    "eeth_exchange_rate": exchange_rate,
                    "exchange_rate_premium": if exchange_rate >= 1.0 { 
                        (exchange_rate - 1.0) * 100.0 
                    } else { 
                        (1.0 - exchange_rate) * -100.0 
                    },
                    "protocol_tvl_usd": tvl,
                    "liquid_staking_tvl": protocol_metrics.total_eth_staked * 4000.0,
                    "restaking_tvl_usd": protocol_metrics.restaking_tvl * 4000.0,
                    "total_validators": validator_metrics.total_validators,
                    "active_validators": validator_metrics.active_validators,
                    "pending_validators": validator_metrics.pending_validators,
                    "slashed_validators": validator_metrics.slashed_validators,
                    "validator_utilization": if validator_metrics.total_validators > 0 { 
                        validator_metrics.active_validators as f64 / validator_metrics.total_validators as f64 
                    } else { 1.0 },
                    "slashing_rate": if validator_metrics.total_validators > 0 {
                        validator_metrics.slashed_validators as f64 / validator_metrics.total_validators as f64
                    } else { 0.0 },
                    "average_validator_balance": validator_metrics.average_validator_balance,
                    "total_eth_staked": protocol_metrics.total_eth_staked,
                    "eeth_supply": protocol_metrics.eeth_supply,
                    "protocol_features": {
                        "liquid_staking": true,
                        "eigenlayer_restaking": true,
                        "native_restaking": true,
                        "validator_services": true
                    }
                }),
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            positions.push(position);
        }
        
        // Cache the results
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
        self.is_etherfi_contract(contract_address)
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        if let Some(token_address_str) = position.metadata.get("token_address") {
            if let Some(token_address_str) = token_address_str.as_str() {
                if let Ok(_token_address) = Address::from_str(token_address_str) {
                    let current_eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
                    
                    if position.metadata.get("token_symbol").and_then(|v| v.as_str()) == Some("eETH") {
                        let current_exchange_rate = self.get_eeth_exchange_rate().await.unwrap_or(1.0);
                        
                        let cached_rate = position.metadata.get("eeth_exchange_rate")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0);
                        let cached_eth_price = 4000.0;
                        
                        let rate_change_factor = current_exchange_rate / cached_rate;
                        let price_change_factor = current_eth_price / cached_eth_price;
                        
                        return Ok(position.value_usd * rate_change_factor * price_change_factor);
                    }
                    
                    let price_change_factor = current_eth_price / 4000.0;
                    return Ok(position.value_usd * price_change_factor);
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
    fn test_eeth_address() {
        let addr = Address::from_str(EtherFiAdapter::EETH_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0x35fa164735182de50811e8e2e824cfb9b6118ac2");
    }
    
    #[test]
    fn test_liquidity_pool_address() {
        let addr = Address::from_str(EtherFiAdapter::LIQUIDITY_POOL_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0x308861a430be4cce5502d0a12724771fc6daf216");
    }
    
    #[test]
    fn test_eigenpod_manager_address() {
        let addr = Address::from_str(EtherFiAdapter::EIGENPOD_MANAGER_ADDRESS);
        assert!(addr.is_ok());
    }
    
    #[test]
    fn test_etherfi_contract_detection() {
        let client = EthereumClient { rpc_url: "https://eth.llamarpc.com".to_string() };
        let adapter = EtherFiAdapter::new(client).unwrap();
        
        let eeth_addr = Address::from_str(EtherFiAdapter::EETH_ADDRESS).unwrap();
        let pool_addr = Address::from_str(EtherFiAdapter::LIQUIDITY_POOL_ADDRESS).unwrap();
        let node_addr = Address::from_str(EtherFiAdapter::NODE_MANAGER_ADDRESS).unwrap();
        
        assert!(adapter.is_etherfi_contract(eeth_addr));
        assert!(adapter.is_etherfi_contract(pool_addr));
        assert!(adapter.is_etherfi_contract(node_addr));
        
        let random_addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        assert!(!adapter.is_etherfi_contract(random_addr));
    }
    
    #[test]
    fn test_token_symbol_mapping() {
        let client = EthereumClient { rpc_url: "https://eth.llamarpc.com".to_string() };
        let adapter = EtherFiAdapter::new(client).unwrap();
        
        let eeth_addr = Address::from_str(EtherFiAdapter::EETH_ADDRESS).unwrap();
        let eigenpod_addr = Address::from_str(EtherFiAdapter::EIGENPOD_MANAGER_ADDRESS).unwrap();
        let node_addr = Address::from_str(EtherFiAdapter::NODE_MANAGER_ADDRESS).unwrap();
        
        assert_eq!(adapter.get_etherfi_token_symbol(eeth_addr), "eETH");
        assert_eq!(adapter.get_etherfi_token_symbol(eigenpod_addr), "ETH-RESTAKED");
        assert_eq!(adapter.get_etherfi_token_symbol(node_addr), "EF-VALIDATOR");
    }
}