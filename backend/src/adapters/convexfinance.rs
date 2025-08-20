use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use crate::blockchain::ethereum_client::EthereumClient;
use crate::services::IERC20;
use reqwest;
use serde::Deserialize;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

#[derive(Debug, Deserialize, Clone)]
struct ConvexPool {
    id: u32,
    name: String,
    lptoken: String,
    crvRewards: String,
    shutdown: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct ConvexAPY {
    pool: String,
    baseApy: f64,
    crvApy: f64,
    cvxApy: f64,
    extraRewards: Vec<ExtraReward>,
    totalApy: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct ExtraReward {
    token: String,
    apy: f64,
    name: String,
}

#[derive(Debug, Clone)]
struct ConvexPosition {
    pool_id: u32,
    pool_name: String,
    lp_token: Address,
    staked_balance: U256,
    earned_crv: U256,
    earned_cvx: U256,
    extra_rewards: Vec<(Address, U256, String)>,
    underlying_assets: Vec<String>,
    total_apy: f64,
    is_locked: bool,
    lock_end_time: Option<u64>,
}

#[derive(Debug, Clone)]
struct PoolCache {
    pools: Vec<ConvexPool>,
    apys: HashMap<String, ConvexAPY>,
    cached_at: SystemTime,
}

sol! {
    #[sol(rpc)]
    interface IConvexBooster {
        function poolInfo(uint256 pid) external view returns (
            address lptoken,
            address token,
            address gauge,
            address crvRewards,
            address stash,
            bool shutdown
        );
        function poolLength() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IConvexRewards {
        function balanceOf(address account) external view returns (uint256);
        function earned(address account) external view returns (uint256);
        function extraRewardsLength() external view returns (uint256);
        function extraRewards(uint256 index) external view returns (address);
    }
    
    #[sol(rpc)]
    interface IConvexExtraReward {
        function rewardToken() external view returns (address);
        function earned(address account) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IConvexVoteLocked {
        function lockedBalanceOf(address user) external view returns (uint256);
        function userLocks(address user, uint256 index) external view returns (
            uint256 amount,
            uint256 unlockTime
        );
    }
}

pub struct ConvexAdapter {
    client: EthereumClient,
    booster: Address,
    vote_locked_cvx: Address,
    pool_cache: Arc<Mutex<Option<PoolCache>>>,
    http_client: reqwest::Client,
}

impl ConvexAdapter {
    const CONVEX_BOOSTER: &'static str = "0xF403C135812408BFbE8713b5A23a04b3D48AAE31";
    const VOTE_LOCKED_CVX: &'static str = "0xD18140b4B819b895A3dba5442F959fA44994AF50";
    const CONVEX_API_BASE: &'static str = "https://www.convexfinance.com/api";
    
    pub fn new(client: EthereumClient, chain_id: Option<u64>) -> Result<Self, AdapterError> {
        if chain_id.unwrap_or(1) != 1 {
            return Err(AdapterError::InvalidData("Convex is only available on Ethereum mainnet".to_string()));
        }
        
        let booster = Address::from_str(Self::CONVEX_BOOSTER)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid booster address: {}", e)))?;
        let vote_locked_cvx = Address::from_str(Self::VOTE_LOCKED_CVX)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid vote locked CVX address: {}", e)))?;
        
        Ok(Self {
            client,
            booster,
            vote_locked_cvx,
            pool_cache: Arc::new(Mutex::new(None)),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| AdapterError::NetworkError(format!("Failed to create HTTP client: {}", e)))?,
        })
    }
    
    async fn get_pools(&self) -> Result<PoolCache, AdapterError> {
        // Check cache (30-minute cache)
        {
            let cache = self.pool_cache.lock().unwrap();
            if let Some(cached) = cache.as_ref() {
                if cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0)) < Duration::from_secs(1800) {
                    return Ok(cached.clone());
                }
            }
        }
        
        let (pools, apys) = tokio::join!(
            self.fetch_pools_from_contracts(),
            self.fetch_apys_from_api()
        );
        
        let pool_cache = PoolCache {
            pools: pools?,
            apys: apys.unwrap_or_default(),
            cached_at: SystemTime::now(),
        };
        
        // Update cache
        {
            let mut cache = self.pool_cache.lock().unwrap();
            *cache = Some(pool_cache.clone());
        }
        
        Ok(pool_cache)
    }
    
    async fn fetch_pools_from_contracts(&self) -> Result<Vec<ConvexPool>, AdapterError> {
        let booster = IConvexBooster::new(self.booster, self.client.provider());
        let pool_length = booster.poolLength().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get pool length: {}", e)))?
            ._0;
        
        let mut pools = Vec::new();
        
        for i in 0..pool_length.to::<u64>() {
            match booster.poolInfo(U256::from(i)).call().await {
                Ok(pool_info) => {
                    if !pool_info.shutdown {
                        pools.push(ConvexPool {
                            id: i as u32,
                            name: format!("Pool {}", i),
                            lptoken: format!("{:?}", pool_info.lptoken),
                            crvRewards: format!("{:?}", pool_info.crvRewards),
                            shutdown: pool_info.shutdown,
                        });
                    }
                }
                Err(_) => continue,
            }
        }
        
        Ok(pools)
    }
    
    async fn fetch_apys_from_api(&self) -> Result<HashMap<String, ConvexAPY>, AdapterError> {
        let url = format!("{}/apys", Self::CONVEX_API_BASE);
        
        match self.http_client.get(&url).send().await {
            Ok(response) if response.status().is_success() => {
                response.json().await
                    .map_err(|e| AdapterError::NetworkError(format!("Failed to parse APY data: {}", e)))
            }
            _ => Ok(HashMap::new()), // Fallback to empty
        }
    }
    
    async fn get_user_positions(&self, address: Address) -> Result<Vec<ConvexPosition>, AdapterError> {
        let pool_cache = self.get_pools().await?;
        let mut positions = Vec::new();
        
        // Check regular staking positions
        for pool in &pool_cache.pools {
            if let Ok(reward_contract) = Address::from_str(&pool.crvRewards) {
                if let Ok(Some(position)) = self.get_pool_position(address, pool, reward_contract, &pool_cache).await {
                    positions.push(position);
                }
            }
        }
        
        // Check locked CVX position
        if let Ok(Some(locked_pos)) = self.get_locked_cvx_position(address).await {
            positions.push(locked_pos);
        }
        
        Ok(positions)
    }
    
    async fn get_pool_position(
        &self,
        user_address: Address,
        pool: &ConvexPool,
        reward_contract: Address,
        pool_cache: &PoolCache,
    ) -> Result<Option<ConvexPosition>, AdapterError> {
        let rewards = IConvexRewards::new(reward_contract, self.client.provider());
        
        let staked_balance = rewards.balanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get balance: {}", e)))?._0;
        
        if staked_balance == U256::ZERO {
            return Ok(None);
        }
        
        let earned_crv = rewards.earned(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get earned CRV: {}", e)))?._0;
        
        let earned_cvx = earned_crv; // 1:1 ratio simplified
        let extra_rewards = self.get_extra_rewards(reward_contract, user_address).await.unwrap_or_default();
        
        let total_apy = pool_cache.apys.get(&pool.id.to_string())
            .map(|apy| apy.totalApy)
            .unwrap_or(10.0); // Fallback APY
        
        let underlying_assets = self.get_underlying_assets(pool);
        
        Ok(Some(ConvexPosition {
            pool_id: pool.id,
            pool_name: pool.name.clone(),
            lp_token: Address::from_str(&pool.lptoken).unwrap_or_default(),
            staked_balance,
            earned_crv,
            earned_cvx,
            extra_rewards,
            underlying_assets,
            total_apy,
            is_locked: false,
            lock_end_time: None,
        }))
    }
    
    async fn get_locked_cvx_position(&self, user_address: Address) -> Result<Option<ConvexPosition>, AdapterError> {
        let vote_locked = IConvexVoteLocked::new(self.vote_locked_cvx, self.client.provider());
        
        let locked_balance = vote_locked.lockedBalanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get locked balance: {}", e)))?._0;
        
        if locked_balance == U256::ZERO {
            return Ok(None);
        }
        
        // Get next unlock time
        let lock_end_time = match vote_locked.userLocks(user_address, U256::ZERO).call().await {
            Ok(lock_info) => Some(lock_info._1.to::<u64>()),
            Err(_) => None,
        };
        
        Ok(Some(ConvexPosition {
            pool_id: 9999, // Special ID for locked CVX
            pool_name: "Locked CVX".to_string(),
            lp_token: Address::ZERO,
            staked_balance: locked_balance,
            earned_crv: U256::ZERO,
            earned_cvx: U256::ZERO,
            extra_rewards: Vec::new(),
            underlying_assets: vec!["CVX".to_string()],
            total_apy: 15.0, // Typical locked CVX APY
            is_locked: true,
            lock_end_time,
        }))
    }
    
    async fn get_extra_rewards(&self, reward_contract: Address, user_address: Address) -> Result<Vec<(Address, U256, String)>, AdapterError> {
        let rewards = IConvexRewards::new(reward_contract, self.client.provider());
        let extra_length = rewards.extraRewardsLength().call().await?.._0;
        
        let mut extra_rewards = Vec::new();
        
        for i in 0..extra_length.to::<u64>() {
            if let Ok(extra_addr) = rewards.extraRewards(U256::from(i)).call().await {
                let extra_reward = IConvexExtraReward::new(extra_addr._0, self.client.provider());
                
                if let Ok(earned) = extra_reward.earned(user_address).call().await {
                    if earned._0 > U256::ZERO {
                        if let Ok(token_addr) = extra_reward.rewardToken().call().await {
                            let symbol = self.get_token_symbol(token_addr._0).await.unwrap_or("UNKNOWN".to_string());
                            extra_rewards.push((token_addr._0, earned._0, symbol));
                        }
                    }
                }
            }
        }
        
        Ok(extra_rewards)
    }
    
    async fn get_token_symbol(&self, token_address: Address) -> Result<String, AdapterError> {
        let token = IERC20::new(token_address, self.client.provider());
        token.symbol().call().await
            .map(|s| s._0)
            .map_err(|e| AdapterError::ContractError(format!("Failed to get token symbol: {}", e)))
    }
    
    fn get_underlying_assets(&self, pool: &ConvexPool) -> Vec<String> {
        match pool.name.to_lowercase().as_str() {
            name if name.contains("3crv") || name.contains("3pool") => 
                vec!["DAI".to_string(), "USDC".to_string(), "USDT".to_string()],
            name if name.contains("frax") => 
                vec!["FRAX".to_string(), "USDC".to_string()],
            name if name.contains("eth") => 
                vec!["ETH".to_string(), "WETH".to_string()],
            name if name.contains("btc") => 
                vec!["WBTC".to_string(), "renBTC".to_string()],
            _ => vec!["CURVE-LP".to_string()],
        }
    }
    
    fn calculate_position_value(&self, position: &ConvexPosition) -> f64 {
        let balance_f64 = position.staked_balance.to::<f64>() / 1e18;
        
        // Simple price estimation based on asset type
        let token_price = if position.underlying_assets.contains(&"ETH".to_string()) {
            2000.0 // ETH price
        } else if position.underlying_assets.iter().any(|a| a.contains("BTC")) {
            50000.0 // BTC price
        } else {
            1.0 // Stable assets
        };
        
        balance_f64 * token_price
    }
    
    fn to_position(&self, convex_pos: &ConvexPosition) -> Position {
        let balance_usd = self.calculate_position_value(convex_pos);
        
        let mut metadata = HashMap::new();
        metadata.insert("pool_id".to_string(), convex_pos.pool_id.to_string());
        metadata.insert("reward_contract".to_string(), format!("{:?}", convex_pos.lp_token));
        metadata.insert("earned_crv".to_string(), convex_pos.earned_crv.to_string());
        metadata.insert("earned_cvx".to_string(), convex_pos.earned_cvx.to_string());
        metadata.insert("is_locked".to_string(), convex_pos.is_locked.to_string());
        
        if let Some(lock_end) = convex_pos.lock_end_time {
            metadata.insert("lock_end_time".to_string(), lock_end.to_string());
        }
        
        // Add extra rewards
        for (i, (token, amount, symbol)) in convex_pos.extra_rewards.iter().enumerate() {
            metadata.insert(format!("extra_reward_{}_token", i), format!("{:?}", token));
            metadata.insert(format!("extra_reward_{}_amount", i), amount.to_string());
            metadata.insert(format!("extra_reward_{}_symbol", i), symbol.clone());
        }
        
        Position {
            protocol: "Convex".to_string(),
            chain: "ethereum".to_string(),
            position_type: if convex_pos.is_locked { "Locked Staking" } else { "Staking" }.to_string(),
            pool_name: convex_pos.pool_name.clone(),
            tokens: convex_pos.underlying_assets.clone(),
            balance_usd,
            apy: Some(convex_pos.total_apy),
            metadata,
        }
    }
}

#[async_trait]
impl DeFiAdapter for ConvexAdapter {
    fn protocol_name(&self) -> String {
        "Convex Finance".to_string()
    }
    
    fn supported_chains(&self) -> Vec<String> {
        vec!["ethereum".to_string()]
    }
    
    async fn get_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        let convex_positions = self.get_user_positions(address).await?;
        let positions = convex_positions.iter()
            .map(|pos| self.to_position(pos))
            .collect();
        
        Ok(positions)
    }
    
    async fn get_protocol_stats(&self) -> Result<serde_json::Value, AdapterError> {
        let pool_cache = self.get_pools().await?;
        
        Ok(serde_json::json!({
            "protocol": "Convex Finance",
            "chain": "ethereum",
            "total_pools": pool_cache.pools.len(),
            "average_apy": pool_cache.apys.values()
                .map(|a| a.totalApy)
                .sum::<f64>() / pool_cache.apys.len().max(1) as f64,
            "supported_features": [
                "LP Token Staking",
                "CRV/CVX Rewards",
                "Extra Incentives",
                "Locked CVX"
            ]
        }))
    }
    
    async fn health_check(&self) -> Result<bool, AdapterError> {
        let booster = IConvexBooster::new(self.booster, self.client.provider());
        booster.poolLength().call().await
            .map(|_| true)
            .map_err(|e| AdapterError::ContractError(format!("Health check failed: {}", e)))
    }
}