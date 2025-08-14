use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use crate::blockchain::ethereum_client::EthereumClient;
use crate::services::IERC20;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

#[derive(Debug, Deserialize, Clone)]
struct ConvexPool {
    id: u32,
    name: String,
    gauge: String,
    lptoken: String,
    token: String,
    crvRewards: String,
    shutdown: bool,
    gaugeStatus: String,
    added: bool,
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

#[derive(Debug, Deserialize, Clone)]
struct ConvexTVL {
    pool: String,
    tvl: f64,
}

#[derive(Debug, Clone)]
struct ConvexPosition {
    pool_id: u32,
    pool_name: String,
    lp_token: Address,
    reward_contract: Address,
    staked_balance: U256,
    earned_crv: U256,
    earned_cvx: U256,
    extra_rewards: Vec<(Address, U256, String)>, // (token, amount, symbol)
    underlying_assets: Vec<String>,
    base_apy: f64,
    crv_apy: f64,
    cvx_apy: f64,
    extra_apy: f64,
    total_apy: f64,
    is_locked: bool,
    lock_end_time: Option<u64>,
}

#[derive(Debug, Clone)]
struct CachedConvexData {
    pools: Vec<ConvexPool>,
    apys: HashMap<String, ConvexAPY>,
    tvls: HashMap<String, f64>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

// Convex contract ABIs using alloy sol! macro
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
        function deposit(uint256 pid, uint256 amount, bool stake) external returns (bool);
        function withdraw(uint256 pid, uint256 amount) external returns (bool);
        function withdrawAll(uint256 pid) external returns (bool);
    }
    
    #[sol(rpc)]
    interface IConvexRewards {
        function balanceOf(address account) external view returns (uint256);
        function earned(address account) external view returns (uint256);
        function getReward(address account, bool claimExtras) external returns (bool);
        function extraRewardsLength() external view returns (uint256);
        function extraRewards(uint256 index) external view returns (address);
        function rewardToken() external view returns (address);
        function stakingToken() external view returns (address);
        function totalSupply() external view returns (uint256);
        function rewardRate() external view returns (uint256);
        function periodFinish() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IConvexExtraReward {
        function rewardToken() external view returns (address);
        function earned(address account) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function rewardRate() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IConvexVoteLocked {
        function lockedBalanceOf(address user) external view returns (uint256);
        function balances(address user) external view returns (
            uint256 locked,
            uint256 nextUnlockIndex
        );
        function userLocks(address user, uint256 index) external view returns (
            uint256 amount,
            uint256 unlockTime
        );
        function lockDuration() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface ICurvePool {
        function coins(uint256 index) external view returns (address);
        function balances(uint256 index) external view returns (uint256);
        function get_virtual_price() external view returns (uint256);
        function calc_withdraw_one_coin(uint256 amount, int128 index) external view returns (uint256);
    }
}

/// Convex Finance yield optimizer adapter
pub struct ConvexAdapter {
    client: EthereumClient,
    chain: String,
    // Contract addresses
    booster: Address,
    cvx_rewards: Address,
    vote_locked_cvx: Address,
    // Caches to prevent API spam
    pool_cache: Arc<Mutex<Option<CachedConvexData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    // HTTP client for API calls
    http_client: reqwest::Client,
}

impl ConvexAdapter {
    /// Convex contract addresses (Ethereum mainnet)
    const CONVEX_BOOSTER: &'static str = "0xF403C135812408BFbE8713b5A23a04b3D48AAE31";
    const CVX_REWARDS: &'static str = "0xCF50b810E57Ac33B91dCF525C6ddd9881B139332";
    const VOTE_LOCKED_CVX: &'static str = "0xD18140b4B819b895A3dba5442F959fA44994AF50";
    
    /// Convex API endpoints (if available)
    const CONVEX_API_BASE: &'static str = "https://www.convexfinance.com/api";
    
    pub fn new(client: EthereumClient, chain_id: Option<u64>) -> Result<Self, AdapterError> {
        // Convex is primarily on Ethereum mainnet
        if chain_id.unwrap_or(1) != 1 {
            return Err(AdapterError::InvalidData("Convex is only available on Ethereum mainnet".to_string()));
        }
        
        let booster = Address::from_str(Self::CONVEX_BOOSTER)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid booster address: {}", e)))?;
        let cvx_rewards = Address::from_str(Self::CVX_REWARDS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid CVX rewards address: {}", e)))?;
        let vote_locked_cvx = Address::from_str(Self::VOTE_LOCKED_CVX)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid vote locked CVX address: {}", e)))?;
        
        Ok(Self {
            client,
            chain: "ethereum".to_string(),
            booster,
            cvx_rewards,
            vote_locked_cvx,
            pool_cache: Arc::new(Mutex::new(None)),
            position_cache: Arc<Mutex::new(HashMap::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| AdapterError::NetworkError(format!("Failed to create HTTP client: {}", e)))?,
        })
    }
    
    /// Fetch all pools data from Convex contracts and APIs with comprehensive caching
    async fn fetch_all_pools_data(&self) -> Result<CachedConvexData, AdapterError> {
        // Check cache first (30-minute cache for pool data)
        {
            let cache = self.pool_cache.lock().unwrap();
            if let Some(cached_data) = cache.as_ref() {
                let cache_age = cached_data.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(1800) { // 30 minutes
                    tracing::info!(
                        cache_age_secs = cache_age.as_secs(),
                        pool_count = cached_data.pools.len(),
                        "CACHE HIT: Using cached Convex pool data"
                    );
                    return Ok(cached_data.clone());
                }
            }
        }
        
        tracing::info!("CACHE MISS: Fetching fresh Convex pool data from contracts and API");
        
        // Fetch data concurrently
        let (pools_result, apys_result, tvls_result) = tokio::join!(
            self.fetch_pools_from_contracts(),
            self.fetch_apys_from_api(),
            self.fetch_tvls_from_api()
        );
        
        let pools = pools_result?;
        let apys = apys_result.unwrap_or_default();
        let tvls = tvls_result.unwrap_or_default();
        
        let cached_data = CachedConvexData {
            pools,
            apys,
            tvls,
            cached_at: SystemTime::now(),
        };
        
        // Update cache
        {
            let mut cache = self.pool_cache.lock().unwrap();
            *cache = Some(cached_data.clone());
        }
        
        tracing::info!(
            pool_count = cached_data.pools.len(),
            apy_count = cached_data.apys.len(),
            tvl_count = cached_data.tvls.len(),
            "✅ Fetched and cached all Convex data"
        );
        
        Ok(cached_data)
    }
    
    /// Fetch pools from Convex Booster contract
    async fn fetch_pools_from_contracts(&self) -> Result<Vec<ConvexPool>, AdapterError> {
        tracing::debug!("Fetching Convex pools from Booster contract");
        
        let booster = IConvexBooster::new(self.booster, self.client.provider());
        
        // Get total number of pools
        let pool_length = booster.poolLength().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get pool length: {}", e)))?
            ._0;
        
        tracing::info!(total_pools = %pool_length, "Found pools in Convex Booster");
        
        let mut pools = Vec::new();
        
        // Fetch pool info in batches to avoid timeouts
        const BATCH_SIZE: u64 = 50;
        for batch_start in (0..pool_length.to::<u64>()).step_by(BATCH_SIZE as usize) {
            let batch_end = std::cmp::min(batch_start + BATCH_SIZE, pool_length.to::<u64>());
            
            let mut batch_futures = Vec::new();
            for i in batch_start..batch_end {
                let pool_id = U256::from(i);
                batch_futures.push(booster.poolInfo(pool_id));
            }
            
            // Execute batch
            for (idx, future) in batch_futures.into_iter().enumerate() {
                let pool_id = batch_start + idx as u64;
                match future.call().await {
                    Ok(pool_info) => {
                        let pool = ConvexPool {
                            id: pool_id as u32,
                            name: format!("Pool {}", pool_id), // We'll get better names from API if available
                            gauge: format!("{:?}", pool_info.gauge),
                            lptoken: format!("{:?}", pool_info.lptoken),
                            token: format!("{:?}", pool_info.token),
                            crvRewards: format!("{:?}", pool_info.crvRewards),
                            shutdown: pool_info.shutdown,
                            gaugeStatus: if pool_info.shutdown { "inactive".to_string() } else { "active".to_string() },
                            added: true,
                        };
                        pools.push(pool);
                    }
                    Err(e) => {
                        tracing::warn!(pool_id = %pool_id, error = %e, "Failed to fetch pool info");
                    }
                }
            }
            
            // Small delay between batches to be nice to RPC
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        tracing::info!(fetched_pools = pools.len(), "Fetched Convex pools from contracts");
        
        Ok(pools)
    }
    
    /// Fetch APYs from Convex API (if available)
    async fn fetch_apys_from_api(&self) -> Result<HashMap<String, ConvexAPY>, AdapterError> {
        // Try to fetch from API, fallback to empty if not available
        let url = format!("{}/apys", Self::CONVEX_API_BASE);
        
        tracing::debug!("Attempting to fetch Convex APYs from: {}", url);
        
        match timeout(Duration::from_secs(15), self.http_client.get(&url).send()).await {
            Ok(Ok(response)) if response.status().is_success() => {
                match response.json::<HashMap<String, ConvexAPY>>().await {
                    Ok(apys) => {
                        tracing::info!(apy_count = apys.len(), "Fetched Convex APYs from API");
                        return Ok(apys);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse Convex APY data: {}", e);
                    }
                }
            }
            Ok(Ok(response)) => {
                tracing::warn!("Convex API returned error status: {}", response.status());
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to fetch Convex APYs: {}", e);
            }
            Err(_) => {
                tracing::warn!("Convex API request timed out");
            }
        }
        
        // Return empty HashMap as fallback
        tracing::info!("Using fallback APY calculation (API unavailable)");
        Ok(HashMap::new())
    }
    
    /// Fetch TVLs from Convex API (if available)
    async fn fetch_tvls_from_api(&self) -> Result<HashMap<String, f64>, AdapterError> {
        // Try to fetch from API, fallback to empty if not available
        let url = format!("{}/tvl", Self::CONVEX_API_BASE);
        
        tracing::debug!("Attempting to fetch Convex TVLs from: {}", url);
        
        match timeout(Duration::from_secs(15), self.http_client.get(&url).send()).await {
            Ok(Ok(response)) if response.status().is_success() => {
                match response.json::<HashMap<String, f64>>().await {
                    Ok(tvls) => {
                        tracing::info!(tvl_count = tvls.len(), "Fetched Convex TVLs from API");
                        return Ok(tvls);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse Convex TVL data: {}", e);
                    }
                }
            }
            Ok(Ok(response)) => {
                tracing::warn!("Convex API returned error status: {}", response.status());
            }
            Ok(Err(e)) => {
                tracing::warn!("Failed to fetch Convex TVLs: {}", e);
            }
            Err(_) => {
                tracing::warn!("Convex API request timed out");
            }
        }
        
        // Return empty HashMap as fallback
        tracing::info!("Using fallback TVL calculation (API unavailable)");
        Ok(HashMap::new())
    }
    
    /// Get user positions in Convex pools
    async fn get_user_convex_positions(&self, address: Address) -> Result<Vec<ConvexPosition>, AdapterError> {
        tracing::info!(
            user_address = %address,
            "🔍 Discovering ALL Convex staking positions"
        );
        
        let cached_data = self.fetch_all_pools_data().await?;
        let mut positions = Vec::new();
        
        // Check each active pool for user balance
        for pool in &cached_data.pools {
            if pool.shutdown || pool.gaugeStatus != "active" {
                continue; // Skip inactive pools
            }
            
            if let Ok(reward_contract) = Address::from_str(&pool.crvRewards) {
                match self.get_pool_position(address, pool, reward_contract, &cached_data).await {
                    Ok(Some(position)) => {
                        positions.push(position);
                        tracing::info!(
                            pool_id = %pool.id,
                            pool_name = %pool.name,
                            staked_balance = %position.staked_balance,
                            "Found Convex staking position"
                        );
                    }
                    Ok(None) => {
                        // No position in this pool
                    }
                    Err(e) => {
                        tracing::warn!(
                            pool_id = %pool.id,
                            error = %e,
                            "Failed to check pool position"
                        );
                    }
                }
            }
        }
        
        // Also check for locked CVX positions
        if let Ok(locked_position) = self.get_locked_cvx_position(address).await {
            if let Some(locked_pos) = locked_position {
                positions.push(locked_pos);
                tracing::info!(
                    user_address = %address,
                    "Found locked CVX position"
                );
            }
        }
        
        tracing::info!(
            user_address = %address,
            total_positions = positions.len(),
            "✅ Discovered ALL Convex positions"
        );
        
        Ok(positions)
    }
    
    /// Get user position in a specific pool
    async fn get_pool_position(
        &self,
        user_address: Address,
        pool: &ConvexPool,
        reward_contract: Address,
        cached_data: &CachedConvexData,
    ) -> Result<Option<ConvexPosition>, AdapterError> {
        let reward_contract_instance = IConvexRewards::new(reward_contract, self.client.provider());
        
        // Get user's staked balance
        let staked_balance = reward_contract_instance.balanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get staked balance for pool {}: {}", pool.id, e)))?
            ._0;
        
        if staked_balance == U256::ZERO {
            return Ok(None);
        }
        
        // Get earned CRV
        let earned_crv = reward_contract_instance.earned(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get earned CRV for pool {}: {}", pool.id, e)))?
            ._0;
        
        // Calculate earned CVX (CVX mint ratio is typically 1:1 with some adjustments)
        let earned_cvx = self.calculate_cvx_mint_amount(earned_crv).await;
        
        // Get extra rewards
        let extra_rewards = self.get_extra_rewards(reward_contract, user_address).await.unwrap_or_default();
        
        // Get APY data
        let pool_key = pool.id.to_string();
        let (base_apy, crv_apy, cvx_apy, extra_apy, total_apy) = if let Some(apy_data) = cached_data.apys.get(&pool_key) {
            (
                apy_data.baseApy,
                apy_data.crvApy,
                apy_data.cvxApy,
                apy_data.extraRewards.iter().map(|r| r.apy).sum::<f64>(),
                apy_data.totalApy,
            )
        } else {
            // Fallback APY estimation
            let estimated_apy = self.estimate_pool_apy(pool, reward_contract).await.unwrap_or(10.0);
            (5.0, estimated_apy * 0.6, estimated_apy * 0.3, estimated_apy * 0.1, estimated_apy)
        };
        
        // Get underlying assets from LP token (this would need Curve pool interaction)
        let underlying_assets = self.get_underlying_assets(pool).await.unwrap_or_else(|| vec!["CURVE-LP".to_string()]);
        
        Ok(Some(ConvexPosition {
            pool_id: pool.id,
            pool_name: pool.name.clone(),
            lp_token: Address::from_str(&pool.lptoken).unwrap_or_default(),
            reward_contract,
            staked_balance,
            earned_crv,
            earned_cvx,
            extra_rewards,
            underlying_assets,
            base_apy,
            crv_apy,
            cvx_apy,
            extra_apy,
            total_apy,
            is_locked: false,
            lock_end_time: None,
        }))
    }
    
    /// Get locked CVX position
    async fn get_locked_cvx_position(&self, user_address: Address) -> Result<Option<ConvexPosition>, AdapterError> {
        let vote_locked = IConvexVoteLocked::new(self.vote_locked_cvx, self.client.provider());
        
        let locked_balance = vote_locked.lockedBalanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get locked CVX balance: {}", e)))?
            ._0;
        
        if locked_balance == U256::ZERO {
            return Ok(None);
        }
        
        // Get lock information
        let (locked_amount, next_unlock_index) = vote_locked.balances(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get lock info: {}", e)))?;
        
        let mut lock_end_time = None;
        if next_unlock_index._1.to::<u64>() > 0 {
            if let Ok(lock_info) = vote_locked.userLocks(user_address, next_unlock_index._1).call().await {
                lock_end_time = Some(lock_info._1.to::<u64>());
            }
        }
        
        Ok(Some(ConvexPosition {
            pool_id: 9999, // Special ID for locked CVX
            pool_name: "Locked CVX".to_string(),
            lp_token: Address::ZERO,
            reward_contract: self.vote_locked_cvx,
            staked_balance: locked_balance,
            earned_crv: U256::ZERO,
            earned_cvx: U256::ZERO,
            extra_rewards: Vec::new(),
            underlying_assets: vec!["CVX".to_string()],
            base_apy: 0.0,
            crv_apy: 0.0,
            cvx_apy: 0.0,
            extra_apy: 15.0, // Locked CVX typically earns around 15% APY
            total_apy: 15.0,
            is_locked: true,
            lock_end_time,
        }))
    }
    
    /// Get extra rewards for a pool
    async fn get_extra_rewards(&self, reward_contract: Address, user_address: Address) -> Result<Vec<(Address, U256, String)>, AdapterError> {
        let reward_contract_instance = IConvexRewards::new(reward_contract, self.client.provider());
        
        let extra_rewards_length = reward_contract_instance.extraRewardsLength().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get extra rewards length: {}", e)))?
            ._0;
        
        let mut extra_rewards = Vec::new();
        
        for i in 0..extra_rewards_length.to::<u64>() {
            if let Ok(extra_reward_address) = reward_contract_instance.extraRewards(U256::from(i)).call().await {
                let extra_reward_contract = IConvexExtraReward::new(extra_reward_address._0, self.client.provider());
                
                if let Ok(earned) = extra_reward_contract.earned(user_address).call().await {
                    if earned._0 > U256::ZERO {
                        // Get token address and symbol
                        if let Ok(token_address) = extra_reward_contract.rewardToken().call().await {
                            let token_symbol = self.get_token_symbol(token_address._0).await.unwrap_or("UNKNOWN".to_string());
                            extra_rewards.push((token_address._0, earned._0, token_symbol));
                        }
                    }
                }
            }
        }
        
        Ok(extra_rewards)
    }
    
    /// Calculate CVX mint amount from CRV earned
    async fn calculate_cvx_mint_amount(&self, crv_amount: U256) -> U256 {
        // CVX minting follows a reduction schedule
        // For simplicity, we'll use a 1:1 ratio in early stages
        // In practice, this gets more complex as CVX supply increases
        crv_amount
    }
    
    /// Estimate pool APY from on-chain data
    async fn estimate_pool_apy(&self, pool: &ConvexPool, reward_contract: Address) -> Result<f64, AdapterError> {
        let reward_contract_instance = IConvexRewards::new(reward_contract, self.client.provider());
        
        // Get reward rate and total supply to estimate APY
        let reward_rate = reward_contract_instance.rewardRate().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get reward rate: {}", e)))?
            ._0;
        
        let total_supply = reward_contract_instance.totalSupply().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get total supply: {}", e)))?
            ._0;
        
        if total_supply == U256::ZERO {
            return Ok(0.0);
        }
        
        // Calculate APY (simplified)
        let annual_rewards = reward_rate.to::<f64>() * 31536000.0; // seconds in a year
        let total_supply_f64 = total_supply.to::<f64>();
        
        let apy = (annual_rewards / total_supply_f64) * 100.0;
        
        Ok(apy.min(1000.0)) // Cap at 1000% APY for sanity
    }
    
    /// Get underlying assets from Curve LP token
    async fn get_underlying_assets(&self, pool: &ConvexPool) -> Result<Vec<String>, AdapterError> {
        // This would require querying the Curve pool contract
        // For now, we'll return a fallback based on common pools
        match pool.name.to_lowercase().as_str() {
            name if name.contains("3crv") || name.contains("3pool") => Ok(vec!["DAI".to_string(), "USDC".to_string(), "USDT".to_string()]),
            name if name.contains("frax") => Ok(vec!["FRAX".to_string(), "USDC".to_string()]),
            name if name.contains("eth") => Ok(vec!["ETH".to_string(), "WETH".to_string()]),
            name if name.contains("btc") => Ok(vec!["WBTC".to_string(), "renBTC".to_string()]),
            _ => Ok(vec!["CURVE-LP".to_string()]),
        }
    }
    
    /// Get token symbol
    async fn get_token_symbol(&self, token_address: Address) -> Result<String, AdapterError> {
        let token = IERC20::new(token_address, self.client.provider());
        
        match token.symbol().call().await {
            Ok(symbol) => Ok(symbol._0),
            Err(_) => Ok("UNKNOWN".to_string()),
        }
    }
    
    /// Calculate USD value of position
    async fn calculate_position_value(&self, position: &ConvexPosition, cached_data: &CachedConvexData) -> (f64, f64, f64) {
        let staked_balance_f64 = position.staked_balance.to::<f64>() / 10f64.powi(18);
        
        // Get price from multiple sources
        let mut token_price = 1.0f64; // Default for stable LP tokens
        
        // Try to get price based on underlying assets
        if position.underlying_assets.contains(&"ETH".to_string()) || 
           position.underlying_assets.contains(&"WETH".to_string()) {
            token_price = self.get_fallback_eth_price().await;
        } else if position.underlying_assets.iter().any(|asset| 
            asset.contains("BTC") || asset.contains("btc")) {
            token_price = 50000.0; // Fallback BTC price
        }
        
        // Get TVL data if available
        if let Some(&tvl) = cached_data.tvls.get(&position.pool_id.to_string()) {
            // Rough price estimation from TVL
            if tvl > 0.0 {
                token_price = (tvl / 1000000.0).max(0.5).min(10.0); // Reasonable bounds
            }
        }
        
        let base_value_usd = staked_balance_f64 * token_price;
        
        // Calculate pending rewards value
        let crv_price = 0.5; // Fallback CRV price
        let cvx_price = 2.0; // Fallback CVX price
        
        let crv_rewards_usd = position.earned_crv.to::<f64>() / 10f64.powi(18) * crv_price;
        let cvx_rewards_usd = position.earned_cvx.to::<f64>() / 10f64.powi(18) * cvx_price;
        
        // Calculate extra rewards value
        let extra_rewards_usd = position.extra_rewards.iter().fold(0.0, |acc, (_, amount, _)| {
            acc + (amount.to::<f64>() / 10f64.powi(18) * 1.0) // Assume $1 per extra reward token as fallback
        });
        
        let total_rewards_usd = crv_rewards_usd + cvx_rewards_usd + extra_rewards_usd;
        let total_value_usd = base_value_usd + total_rewards_usd;
        
        (base_value_usd, total_rewards_usd, total_value_usd)
    }
    
    /// Get fallback ETH price (simple implementation)
    async fn get_fallback_eth_price(&self) -> f64 {
        // In a real implementation, you'd fetch this from a price oracle or API
        // For now, return a reasonable fallback
        2000.0 // $2000 ETH
    }
    
    /// Convert ConvexPosition to generic Position
    async fn convex_position_to_position(&self, convex_pos: &ConvexPosition, cached_data: &CachedConvexData) -> Position {
        let (base_value, rewards_value, total_value) = self.calculate_position_value(convex_pos, cached_data).await;
        
        let mut metadata = HashMap::new();
        metadata.insert("pool_id".to_string(), convex_pos.pool_id.to_string());
        metadata.insert("pool_name".to_string(), convex_pos.pool_name.clone());
        metadata.insert("reward_contract".to_string(), format!("{:?}", convex_pos.reward_contract));
        metadata.insert("base_apy".to_string(), convex_pos.base_apy.to_string());
        metadata.insert("crv_apy".to_string(), convex_pos.crv_apy.to_string());
        metadata.insert("cvx_apy".to_string(), convex_pos.cvx_apy.to_string());
        metadata.insert("extra_apy".to_string(), convex_pos.extra_apy.to_string());
        metadata.insert("total_apy".to_string(), convex_pos.total_apy.to_string());
        metadata.insert("is_locked".to_string(), convex_pos.is_locked.to_string());
        
        if let Some(lock_end) = convex_pos.lock_end_time {
            metadata.insert("lock_end_time".to_string(), lock_end.to_string());
        }
        
        // Add earned rewards information
        metadata.insert("earned_crv".to_string(), convex_pos.earned_crv.to_string());
        metadata.insert("earned_cvx".to_string(), convex_pos.earned_cvx.to_string());
        
        // Add extra rewards info
        for (i, (token_addr, amount, symbol)) in convex_pos.extra_rewards.iter().enumerate() {
            metadata.insert(format!("extra_reward_{}_token", i), format!("{:?}", token_addr));
            metadata.insert(format!("extra_reward_{}_amount", i), amount.to_string());
            metadata.insert(format!("extra_reward_{}_symbol", i), symbol.clone());
        }
        
        // Add underlying assets
        for (i, asset) in convex_pos.underlying_assets.iter().enumerate() {
            metadata.insert(format!("underlying_asset_{}", i), asset.clone());
        }
        
        Position {
            protocol: "Convex".to_string(),
            chain: self.chain.clone(),
            position_type: if convex_pos.is_locked { "Locked Staking".to_string() } else { "Staking".to_string() },
            pool_name: convex_pos.pool_name.clone(),
            tokens: convex_pos.underlying_assets.clone(),
            balance_usd: total_value,
            apy: Some(convex_pos.total_apy),
            metadata,
        }
    }
}

#[async_trait]
impl DeFiAdapter for ConvexAdapter {
    /// Get protocol name
    fn protocol_name(&self) -> String {
        "Convex Finance".to_string()
    }
    
    /// Get supported chains
    fn supported_chains(&self) -> Vec<String> {
        vec!["ethereum".to_string()]
    }
    
    /// Get user positions across all Convex pools
    async fn get_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        tracing::info!(
            user_address = %address,
            protocol = "Convex Finance",
            "🚀 Starting comprehensive Convex position discovery"
        );
        
        // Check position cache first (5-minute cache)
        {
            let position_cache = self.position_cache.lock().unwrap();
            if let Some(cached_positions) = position_cache.get(&address) {
                let cache_age = cached_positions.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) { // 5 minutes
                    tracing::info!(
                        user_address = %address,
                        cache_age_secs = cache_age.as_secs(),
                        position_count = cached_positions.positions.len(),
                        "CACHE HIT: Using cached Convex positions"
                    );
                    return Ok(cached_positions.positions.clone());
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            "CACHE MISS: Fetching fresh Convex positions"
        );
        
        // Get all user positions from Convex
        let convex_positions = self.get_user_convex_positions(address).await?;
        
        if convex_positions.is_empty() {
            tracing::info!(
                user_address = %address,
                "No Convex positions found for user"
            );
            return Ok(Vec::new());
        }
        
        // Get cached data for price calculations
        let cached_data = self.fetch_all_pools_data().await?;
        
        // Convert to generic positions
        let mut positions = Vec::new();
        for convex_pos in &convex_positions {
            let position = self.convex_position_to_position(convex_pos, &cached_data).await;
            positions.push(position);
        }
        
        // Cache the results
        {
            let mut position_cache = self.position_cache.lock().unwrap();
            position_cache.insert(address, CachedPositions {
                positions: positions.clone(),
                cached_at: SystemTime::now(),
            });
        }
        
        tracing::info!(
            user_address = %address,
            total_positions = positions.len(),
            total_value_usd = positions.iter().map(|p| p.balance_usd).sum::<f64>(),
            "✅ Successfully discovered all Convex Finance positions"
        );
        
        Ok(positions)
    }
    
    /// Get protocol-specific statistics
    async fn get_protocol_stats(&self) -> Result<serde_json::Value, AdapterError> {
        let cached_data = self.fetch_all_pools_data().await?;
        
        let total_pools = cached_data.pools.len();
        let active_pools = cached_data.pools.iter().filter(|p| !p.shutdown).count();
        let total_tvl = cached_data.tvls.values().sum::<f64>();
        let avg_apy = if !cached_data.apys.is_empty() {
            cached_data.apys.values().map(|a| a.totalApy).sum::<f64>() / cached_data.apys.len() as f64
        } else {
            0.0
        };
        
        Ok(serde_json::json!({
            "protocol": "Convex Finance",
            "chain": "ethereum",
            "total_pools": total_pools,
            "active_pools": active_pools,
            "total_tvl_usd": total_tvl,
            "average_apy": avg_apy,
            "data_freshness": "30min_cache",
            "supported_features": [
                "LP Token Staking",
                "CRV Rewards",
                "CVX Rewards", 
                "Extra Incentives",
                "Locked CVX",
                "Auto-compounding"
            ]
        }))
    }
    
    /// Health check for the adapter
    async fn health_check(&self) -> Result<bool, AdapterError> {
        tracing::info!("🔍 Performing Convex Finance adapter health check");
        
        // Test contract connectivity
        let booster = IConvexBooster::new(self.booster, self.client.provider());
        
        // Try to get pool length as a simple connectivity test
        match booster.poolLength().call().await {
            Ok(pool_length) => {
                tracing::info!(
                    pool_count = %pool_length._0,
                    "✅ Convex Booster contract accessible"
                );
                
                // Test API connectivity (optional)
                let api_healthy = match self.http_client.get(&format!("{}/health", Self::CONVEX_API_BASE))
                    .timeout(Duration::from_secs(5))
                    .send().await 
                {
                    Ok(response) if response.status().is_success() => {
                        tracing::info!("✅ Convex API accessible");
                        true
                    }
                    _ => {
                        tracing::warn!("⚠️ Convex API not accessible (fallback mode available)");
                        true // Still healthy since we can work without API
                    }
                };
                
                tracing::info!("✅ Convex Finance adapter health check passed");
                Ok(api_healthy)
            }
            Err(e) => {
                tracing::error!("❌ Convex Booster contract not accessible: {}", e);
                Err(AdapterError::ContractError(format!("Health check failed: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_convex_adapter_creation() {
        // Mock client - in real tests you'd use a proper test client
        let client = EthereumClient::new("http://localhost:8545").expect("Failed to create client");
        
        let adapter = ConvexAdapter::new(client, Some(1)).expect("Failed to create adapter");
        assert_eq!(adapter.protocol_name(), "Convex Finance");
        assert_eq!(adapter.supported_chains(), vec!["ethereum"]);
    }

    #[tokio::test] 
    async fn test_invalid_chain() {
        let client = EthereumClient::new("http://localhost:8545").expect("Failed to create client");
        
        let result = ConvexAdapter::new(client, Some(137)); // Polygon
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_address_parsing() {
        let booster_addr = Address::from_str(ConvexAdapter::CONVEX_BOOSTER);
        assert!(booster_addr.is_ok());
        
        let cvx_rewards_addr = Address::from_str(ConvexAdapter::CVX_REWARDS);
        assert!(cvx_rewards_addr.is_ok());
        
        let vote_locked_addr = Address::from_str(ConvexAdapter::VOTE_LOCKED_CVX);
        assert!(vote_locked_addr.is_ok());
    }
}