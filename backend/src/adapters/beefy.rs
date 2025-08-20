use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use crate::blockchain::ethereum_client::EthereumClient;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

#[derive(Debug, Deserialize, Clone)]
struct BeefyVault {
    id: String,
    name: String,
    token: String,
    tokenAddress: String,
    tokenDecimals: u8,
    earnedToken: String,
    earnedTokenAddress: String,
    earnContractAddress: String,
    oracle: String,
    oracleId: String,
    status: String,
    platform: String,
    assets: Vec<String>,
    risks: Vec<String>,
    strategyTypeId: String,
    strategy: String,
    chain: String,
    addLiquidityUrl: Option<String>,
    removeLiquidityUrl: Option<String>,
    isGovVault: Option<bool>,
    pricePerFullShare: Option<String>,
    tvl: Option<f64>,
    apy: Option<f64>,
    apr: Option<BeefyAPR>,
}

#[derive(Debug, Deserialize, Clone)]
struct BeefyAPR {
    #[serde(rename = "totalApy")]
    total_apy: Option<f64>,
    #[serde(rename = "vaultApr")]
    vault_apr: Option<f64>,
    #[serde(rename = "tradingApr")]
    trading_apr: Option<f64>,
    #[serde(rename = "liquidStakingApr")]
    liquid_staking_apr: Option<f64>,
    #[serde(rename = "composablePoolApr")]
    composable_pool_apr: Option<f64>,
    #[serde(rename = "beefyPerformanceFee")]
    beefy_performance_fee: Option<f64>,
    #[serde(rename = "vaultApy")]
    vault_apy: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct BeefyPrice {
    #[serde(flatten)]
    prices: HashMap<String, f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct BeefyTVL {
    #[serde(flatten)]
    tvls: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize)]
struct BeefyPosition {
    pub vault_id: String,
    pub vault_name: String,
    pub chain: String,
    pub vault_address: Address,
    pub token_address: Address,
    pub balance: U256,
    pub shares: U256,
    pub decimals: u8,
    pub underlying_assets: Vec<String>,
    pub current_apy: f64,
    pub performance_fee: f64,
    pub strategy: String,
    pub platform: String,
    pub risks: Vec<String>,
    pub price_per_full_share: f64,
}

#[derive(Debug, Clone)]
struct CachedBeefyData {
    vaults: Vec<BeefyVault>,
    prices: HashMap<String, f64>,
    tvls: HashMap<String, f64>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

// Beefy vault contract ABIs using alloy sol! macro
sol! {
    #[sol(rpc)]
    interface IBeefyVault {
        function balanceOf(address account) external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function getPricePerFullShare() external view returns (uint256);
        function decimals() external view returns (uint8);
        function symbol() external pure returns (string memory);
        function name() external pure returns (string memory);
        function want() external view returns (address);
        function strategy() external view returns (address);
        
        // Events for tracking deposits/withdrawals
        event Deposit(address indexed user, uint256 amount);
        event Withdraw(address indexed user, uint256 amount);
    }
}

/// Beefy Finance yield optimizer adapter
pub struct BeefyAdapter {
    client: EthereumClient,
    chain: String,
    // Caches to prevent API spam
    vault_cache: Arc<Mutex<Option<CachedBeefyData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    // HTTP client for API calls
    http_client: reqwest::Client,
}

impl BeefyAdapter {
    /// Beefy API endpoints
    const BEEFY_API_BASE: &'static str = "https://api.beefy.finance";
    
    /// Chain mappings for Beefy API
    fn get_chain_name(chain_id: u64) -> &'static str {
        match chain_id {
            1 => "ethereum",
            56 => "bsc",
            137 => "polygon",
            250 => "fantom",
            43114 => "avax",
            42161 => "arbitrum",
            10 => "optimism",
            1284 => "moonbeam",
            1285 => "moonriver",
            25 => "cronos",
            66 => "okex",
            128 => "heco",
            _ => "ethereum", // fallback
        }
    }
    
    pub fn new(client: EthereumClient, chain_id: Option<u64>) -> Result<Self, AdapterError> {
        let chain = Self::get_chain_name(chain_id.unwrap_or(1)).to_string();
        
        Ok(Self {
            client,
            chain,
            vault_cache: Arc::new(Mutex::new(None)),
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| AdapterError::NetworkError(format!("Failed to create HTTP client: {}", e)))?,
        })
    }
    
    /// Fetch all vaults data from Beefy API with comprehensive caching
    async fn fetch_all_vaults_data(&self) -> Result<CachedBeefyData, AdapterError> {
        // Check cache first (30-minute cache for vault data)
        {
            let cache = self.vault_cache.lock().unwrap();
            if let Some(cached_data) = cache.as_ref() {
                let cache_age = cached_data.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(1800) { // 30 minutes
                    tracing::info!(
                        cache_age_secs = cache_age.as_secs(),
                        vault_count = cached_data.vaults.len(),
                        "Using cached Beefy vault data"
                    );
                    return Ok(cached_data.clone());
                }
            }
        }
        
        tracing::info!(chain = %self.chain, "Fetching fresh Beefy vault data from API");
        
        // Fetch all data concurrently
        let (vaults_result, prices_result, tvls_result) = tokio::join!(
            self.fetch_vaults(),
            self.fetch_prices(),
            self.fetch_tvls()
        );
        
        let vaults = vaults_result?;
        let prices = prices_result.unwrap_or_default();
        let tvls = tvls_result.unwrap_or_default();
        
        let cached_data = CachedBeefyData {
            vaults,
            prices,
            tvls,
            cached_at: SystemTime::now(),
        };
        
        // Update cache
        {
            let mut cache = self.vault_cache.lock().unwrap();
            *cache = Some(cached_data.clone());
        }
        
        tracing::info!(
            vault_count = cached_data.vaults.len(),
            price_count = cached_data.prices.len(),
            tvl_count = cached_data.tvls.len(),
            "Fetched and cached all Beefy data"
        );
        
        Ok(cached_data)
    }
    
    /// Fetch vaults from Beefy API
    async fn fetch_vaults(&self) -> Result<Vec<BeefyVault>, AdapterError> {
        let url = format!("{}/vaults", Self::BEEFY_API_BASE);
        
        tracing::debug!("Fetching Beefy vaults from: {}", url);
        
        let response = timeout(Duration::from_secs(30), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::NetworkError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AdapterError::NetworkError(format!("HTTP error: {}", response.status())));
        }
        
        let mut vaults: Vec<BeefyVault> = response
            .json()
            .await
            .map_err(|e| AdapterError::DataError(format!("JSON parse error: {}", e)))?;
        
        // Filter by chain
        vaults.retain(|v| v.chain.to_lowercase() == self.chain.to_lowercase());
        
        tracing::info!(
            chain = %self.chain,
            total_vaults = vaults.len(),
            "Fetched Beefy vaults for chain"
        );
        
        Ok(vaults)
    }
    
    /// Fetch prices from Beefy API
    async fn fetch_prices(&self) -> Result<HashMap<String, f64>, AdapterError> {
        let url = format!("{}/prices", Self::BEEFY_API_BASE);
        
        tracing::debug!("Fetching Beefy prices from: {}", url);
        
        let response = timeout(Duration::from_secs(30), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::NetworkError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            tracing::warn!("Failed to fetch Beefy prices: {}", response.status());
            return Ok(HashMap::new());
        }
        
        let prices: BeefyPrice = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to parse Beefy prices: {}", e);
                AdapterError::DataError(format!("Price JSON parse error: {}", e))
            })?;
        
        tracing::info!(price_count = prices.prices.len(), "Fetched Beefy token prices");
        
        Ok(prices.prices)
    }
    
    /// Fetch TVLs from Beefy API
    async fn fetch_tvls(&self) -> Result<HashMap<String, f64>, AdapterError> {
        let url = format!("{}/tvl", Self::BEEFY_API_BASE);
        
        tracing::debug!("Fetching Beefy TVLs from: {}", url);
        
        let response = timeout(Duration::from_secs(30), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::NetworkError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            tracing::warn!("Failed to fetch Beefy TVLs: {}", response.status());
            return Ok(HashMap::new());
        }
        
        let tvls: BeefyTVL = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to parse Beefy TVLs: {}", e);
                AdapterError::DataError(format!("TVL JSON parse error: {}", e))
            })?;
        
        tracing::info!(tvl_count = tvls.tvls.len(), "Fetched Beefy vault TVLs");
        
        Ok(tvls.tvls)
    }
    
    /// Get user positions in Beefy vaults
    async fn get_user_beefy_positions(&self, address: Address) -> Result<Vec<BeefyPosition>, AdapterError> {
        tracing::info!(
            user_address = %address,
            chain = %self.chain,
            "Discovering Beefy yield farming positions"
        );
        
        let cached_data = self.fetch_all_vaults_data().await?;
        let mut positions = Vec::new();
        
        // Check each vault for user balance
        for vault in &cached_data.vaults {
            if vault.status.to_lowercase() != "active" {
                continue; // Skip inactive vaults
            }
            
            if let Ok(vault_address) = Address::from_str(&vault.earnContractAddress) {
                match self.get_vault_position(address, vault, vault_address, &cached_data).await {
                    Ok(Some(position)) => {
                        positions.push(position);
                        tracing::info!(
                            vault_id = %vault.id,
                            vault_name = %vault.name,
                            balance = %position.balance,
                            "Found Beefy vault position"
                        );
                    }
                    Ok(None) => {
                        // No position in this vault
                    }
                    Err(e) => {
                        tracing::warn!(
                            vault_id = %vault.id,
                            error = %e,
                            "Failed to check vault position"
                        );
                    }
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            total_positions = positions.len(),
            "Discovered Beefy positions"
        );
        
        Ok(positions)
    }
    
    /// Get user position in a specific vault
    async fn get_vault_position(
        &self,
        user_address: Address,
        vault: &BeefyVault,
        vault_address: Address,
        cached_data: &CachedBeefyData,
    ) -> Result<Option<BeefyPosition>, AdapterError> {
        let vault_contract = IBeefyVault::new(vault_address, self.client.provider());
        
        // Get user's vault token balance (shares)
        let shares = vault_contract.balanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get vault balance for {}: {}", vault.id, e)))?
            ._0;
        
        if shares == U256::ZERO {
            return Ok(None);
        }
        
        // Get price per full share to calculate underlying balance
        let price_per_full_share_raw = vault_contract.getPricePerFullShare().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get price per full share for {}: {}", vault.id, e)))?
            ._0;
        
        let price_per_full_share = price_per_full_share_raw.to::<f64>() / 10f64.powi(vault.tokenDecimals as i32);
        
        // Calculate underlying token balance
        let shares_f64 = shares.to::<f64>() / 10f64.powi(18); // Vault tokens are usually 18 decimals
        let underlying_balance = shares_f64 * price_per_full_share;
        let underlying_balance_raw = U256::from((underlying_balance * 10f64.powi(vault.tokenDecimals as i32)) as u64);
        
        // Get current APY from cached data or vault
        let current_apy = vault.apy.or_else(|| {
            vault.apr.as_ref().and_then(|apr| apr.total_apy)
        }).unwrap_or(0.0);
        
        // Get performance fee
        let performance_fee = vault.apr.as_ref()
            .and_then(|apr| apr.beefy_performance_fee)
            .unwrap_or(4.5); // Default Beefy fee is 4.5%
        
        Ok(Some(BeefyPosition {
            vault_id: vault.id.clone(),
            vault_name: vault.name.clone(),
            chain: vault.chain.clone(),
            vault_address,
            token_address: Address::from_str(&vault.tokenAddress).unwrap_or_default(),
            balance: underlying_balance_raw,
            shares,
            decimals: vault.tokenDecimals,
            underlying_assets: vault.assets.clone(),
            current_apy,
            performance_fee,
            strategy: vault.strategy.clone(),
            platform: vault.platform.clone(),
            risks: vault.risks.clone(),
            price_per_full_share,
        }))
    }
    
    /// Calculate USD value of position
    async fn calculate_position_value(&self, position: &BeefyPosition, cached_data: &CachedBeefyData) -> (f64, f64, f64) {
        let balance_f64 = position.balance.to::<f64>() / 10f64.powi(position.decimals as i32);
        
        // Try to get price from multiple sources
        let mut token_price = 0.0f64;
        
        // 1. Try Beefy price oracle
        for asset in &position.underlying_assets {
            if let Some(&price) = cached_data.prices.get(asset) {
                token_price = price;
                break;
            }
        }
        
        // 2. Try vault-specific price from TVL
        if token_price == 0.0 {
            if let Some(&tvl) = cached_data.tvls.get(&position.vault_id) {
                // Estimate price from TVL (rough approximation)
                token_price = tvl / 1_000_000.0; // Very rough estimate
            }
        }
        
        // 3. Fallback pricing
        if token_price == 0.0 {
            token_price = match position.underlying_assets.first().map(|s| s.as_str()) {
                Some("ETH") | Some("WETH") => self.get_fallback_eth_price().await,
                Some("BTC") | Some("WBTC") => 50000.0, // Fallback BTC price
                Some("USDC") | Some("USDT") | Some("DAI") | Some("BUSD") => 1.0,
                _ => 100.0, // Generic fallback
            };
        }
        
        let base_value_usd = balance_f64 * token_price;
        
        // Estimate rewards earned (simplified calculation)
        let estimated_yearly_rewards = base_value_usd * (position.current_apy / 100.0);
        let estimated_current_rewards = estimated_yearly_rewards * 0.25; // Assume 3 months average
        
        tracing::info!(
            vault_id = %position.vault_id,
            balance = %balance_f64,
            token_price = %token_price,
            base_value_usd = %base_value_usd,
            current_apy = %position.current_apy,
            estimated_rewards = %estimated_current_rewards,
            performance_fee = %position.performance_fee,
            "Calculated Beefy position value"
        );
        
        (base_value_usd, estimated_current_rewards, position.current_apy)
    }
    
    /// Get fallback ETH price
    async fn get_fallback_eth_price(&self) -> f64 {
        // Try CoinGecko for ETH price
        let url = "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd";
        
        if let Ok(response) = self.http_client.get(url).send().await {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                if let Some(eth_data) = data.get("ethereum") {
                    if let Some(price) = eth_data.get("usd").and_then(|p| p.as_f64()) {
                        return price;
                    }
                }
            }
        }
        
        4000.0 // Fallback ETH price
    }
    
    /// Check if address is a known Beefy vault
    async fn is_beefy_vault(&self, contract_address: Address) -> bool {
        if let Ok(cached_data) = self.fetch_all_vaults_data().await {
            for vault in &cached_data.vaults {
                if let Ok(vault_addr) = Address::from_str(&vault.earnContractAddress) {
                    if vault_addr == contract_address {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[async_trait]
impl DeFiAdapter for BeefyAdapter {
    fn protocol_name(&self) -> &'static str {
        "beefy"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        tracing::info!(
            user_address = %address,
            protocol = "beefy",
            chain = %self.chain,
            "Checking for cached Beefy positions"
        );
        
        // Check cache first (5 minute cache for positions)
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) {
                    tracing::info!(
                        user_address = %address,
                        cache_age_secs = cache_age.as_secs(),
                        position_count = cached.positions.len(),
                        "Returning cached Beefy positions"
                    );
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            chain = %self.chain,
            "Fetching fresh Beefy data from blockchain and API"
        );
        
        // Get user positions
        let beefy_positions = self.get_user_beefy_positions(address).await?;
        
        if beefy_positions.is_empty() {
            tracing::info!(
                user_address = %address,
                "No Beefy positions found"
            );
            return Ok(Vec::new());
        }
        
        // Get cached data for pricing
        let cached_data = self.fetch_all_vaults_data().await?;
        
        let mut positions = Vec::new();
        
        // Convert to Position structs with comprehensive data
        for beefy_pos in beefy_positions {
            let (value_usd, rewards_usd, current_apy) = self.calculate_position_value(&beefy_pos, &cached_data).await;
            
            let position = Position {
                id: format!("beefy_{}_{}", self.chain, beefy_pos.vault_id),
                protocol: "beefy".to_string(),
                position_type: "yield_farming".to_string(),
                pair: format!("{}/{}", 
                    beefy_pos.underlying_assets.join("-"),
                    if beefy_pos.underlying_assets.len() == 1 { "Single" } else { "LP" }
                ),
                value_usd: value_usd.max(0.01),
                pnl_usd: rewards_usd,
                pnl_percentage: current_apy,
                metadata: serde_json::json!({
                    // Vault Information
                    "vault_id": beefy_pos.vault_id,
                    "vault_name": beefy_pos.vault_name,
                    "vault_address": format!("{:?}", beefy_pos.vault_address),
                    "token_address": format!("{:?}", beefy_pos.token_address),
                    "chain": beefy_pos.chain,
                    
                    // Position Details
                    "balance": beefy_pos.balance.to_string(),
                    "shares": beefy_pos.shares.to_string(),
                    "decimals": beefy_pos.decimals,
                    "price_per_full_share": beefy_pos.price_per_full_share,
                    
                    // Performance Metrics
                    "current_apy": current_apy,
                    "performance_fee_percent": beefy_pos.performance_fee,
                    "estimated_yearly_rewards": rewards_usd * 4.0, // Annualized
                    
                    // Strategy & Risk Information
                    "strategy": beefy_pos.strategy,
                    "strategy_platform": beefy_pos.platform,
                    "underlying_assets": beefy_pos.underlying_assets,
                    "risks": beefy_pos.risks,
                    
                    // External Links
                    "beefy_vault_url": format!("https://app.beefy.finance/vault/{}", beefy_pos.vault_id),
                    "add_liquidity_url": cached_data.vaults.iter()
                        .find(|v| v.id == beefy_pos.vault_id)
                        .and_then(|v| v.addLiquidityUrl.as_ref()),
                    "remove_liquidity_url": cached_data.vaults.iter()
                        .find(|v| v.id == beefy_pos.vault_id)
                        .and_then(|v| v.removeLiquidityUrl.as_ref()),
                    
                    // TVL Data
                    "vault_tvl_usd": cached_data.tvls.get(&beefy_pos.vault_id).copied(),
                    
                    // Yield Breakdown (if available)
                    "yield_breakdown": cached_data.vaults.iter()
                        .find(|v| v.id == beefy_pos.vault_id)
                        .and_then(|v| v.apr.as_ref())
                        .map(|apr| serde_json::json!({
                            "vault_apr": apr.vault_apr,
                            "trading_apr": apr.trading_apr,
                            "liquid_staking_apr": apr.liquid_staking_apr,
                            "composable_pool_apr": apr.composable_pool_apr,
                            "total_apy": apr.total_apy,
                        })),
                    
                    // Protocol Info
                    "auto_compound": true, // Beefy always auto-compounds
                    "compound_frequency": "multiple_times_daily",
                    "protocol_type": "yield_optimizer",
                }),
                last_updated: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };
            
            positions.push(position);
        }
        
        // Save results to cache
        {
            let mut cache = self.position_cache.lock().unwrap();
            cache.insert(address, CachedPositions {
                positions: positions.clone(),
                cached_at: SystemTime::now(),
            });
        }
        
        tracing::info!(
            user_address = %address,
            position_count = positions.len(),
            total_value_usd = positions.iter().map(|p| p.value_usd).sum::<f64>(),
            "Successfully fetched and cached Beefy positions"
        );
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        self.is_beefy_vault(contract_address).await
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // For Beefy positions, we can recalculate real-time value
        // by getting fresh price data and vault exchange rates
        
        if let Some(vault_id) = position.metadata.get("vault_id") {
            if let Some(vault_id_str) = vault_id.as_str() {
                // Try to get fresh data
                if let Ok(cached_data) = self.fetch_all_vaults_data().await {
                    // Find the vault in cached data
                    if let Some(vault) = cached_data.vaults.iter().find(|v| v.id == vault_id_str) {
                        // Get fresh price from price oracle
                        let mut fresh_price = 0.0f64;
                        for asset in &vault.assets {
                            if let Some(&price) = cached_data.prices.get(asset) {
                                fresh_price = price;
                                break;
                            }
                        }
                        
                        if fresh_price > 0.0 {
                            // Calculate updated value (simplified)
                            if let Some(balance_str) = position.metadata.get("balance") {
                                if let Some(balance_str) = balance_str.as_str() {
                                    if let Ok(balance_raw) = U256::from_str(balance_str) {
                                        if let Some(decimals) = position.metadata.get("decimals") {
                                            if let Some(decimals_num) = decimals.as_u64() {
                                                let balance_f64 = balance_raw.to::<f64>() / 10f64.powi(decimals_num as i32);
                                                let fresh_value = balance_f64 * fresh_price;
                                                return Ok(fresh_value);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Fallback to cached value
        Ok(position.value_usd)
    }
}