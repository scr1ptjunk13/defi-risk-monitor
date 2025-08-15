use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use crate::blockchain::ethereum_client::EthereumClient;
use crate::risk::calculators::yearnfinance::{YearnFinanceRiskCalculator, YearnRiskData};
use crate::services::IERC20;
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

#[derive(Debug, Deserialize, Clone)]
struct YearnVault {
    address: String,
    #[serde(rename = "type")]
    vault_type: String,
    kind: String,
    symbol: String,
    name: String,
    category: String,
    version: String,
    decimals: u8,
    chainID: u64,
    token: YearnToken,
    tvl: YearnTVL,
    apy: YearnAPY,
    strategies: Vec<YearnStrategy>,
    details: YearnVaultDetails,
    fees: YearnFees,
    migration: Option<YearnMigration>,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnToken {
    address: String,
    name: String,
    symbol: String,
    description: String,
    decimals: u8,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnTVL {
    totalAssets: String,  // in wei
    totalAssetsUSD: f64,
    tvl: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnAPY {
    #[serde(rename = "type")]
    apy_type: String,
    gross_apr: f64,
    net_apy: f64,
    fees: YearnAPYFees,
    points: Option<YearnAPYPoints>,
    composite: Option<YearnCompositeAPY>,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnAPYFees {
    performance: f64,
    withdrawal: f64,
    management: f64,
    keep_crv: f64,
    cvx_keep_crv: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnAPYPoints {
    week_ago: f64,
    month_ago: f64,
    inception: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnCompositeAPY {
    boost: f64,
    pool_apy: f64,
    boosted_apr: f64,
    base_apr: f64,
    cvx_apr: f64,
    rewards_apr: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnStrategy {
    address: String,
    name: String,
    description: String,
    details: YearnStrategyDetails,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnStrategyDetails {
    totalDebt: String,
    totalGain: String,
    totalLoss: String,
    debtRatio: u64, // basis points (10000 = 100%)
    rateLimit: String,
    minDebtPerHarvest: String,
    maxDebtPerHarvest: String,
    estimatedTotalAssets: String,
    creditAvailable: String,
    debtOutstanding: String,
    expectedReturn: String,
    delegatedAssets: String,
    version: String,
    protocols: Vec<String>,
    apr: f64,
    performanceFee: u64,
    activation: u64,
    keeper: String,
    strategist: String,
    rewards: String,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnVaultDetails {
    management: String,
    governance: String,
    guardian: String,
    rewards: String,
    depositLimit: String,
    availableDepositLimit: String,
    comment: String,
    apyTypeOverride: String,
    apyOverride: f64,
    order: u32,
    performanceFee: u64,
    managementFee: u64,
    depositsDisabled: bool,
    withdrawalsDisabled: bool,
    allowZapIn: bool,
    allowZapOut: bool,
    retired: bool,
    hideAlways: bool,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnFees {
    performance: f64,
    withdrawal: f64,
    management: f64,
    keep_crv: f64,
    cvx_keep_crv: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnMigration {
    available: bool,
    address: String,
    contract: String,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnVaultEarnings {
    #[serde(flatten)]
    vault_earnings: HashMap<String, YearnEarningsData>,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnEarningsData {
    earnings: f64,
    earningsUSD: f64,
}

#[derive(Debug, Clone)]
struct YearnPosition {
    vault_address: Address,
    vault_name: String,
    vault_symbol: String,
    vault_version: String,
    vault_type: String,
    category: String,
    token: YearnToken,
    balance: U256,
    shares: U256,
    underlying_balance: U256,
    price_per_share: f64,
    net_apy: f64,
    gross_apr: f64,
    strategies: Vec<YearnStrategy>,
    fees: YearnFees,
    tvl: YearnTVL,
    chain_id: u64,
    is_migrable: bool,
    migration_target: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedYearnData {
    vaults: Vec<YearnVault>,
    vault_map: HashMap<String, YearnVault>, // address -> vault
    earnings: HashMap<String, YearnEarningsData>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

// Yearn vault contract ABIs using alloy sol! macro
sol! {
    #[sol(rpc)]
    interface IYearnVault {
        function balanceOf(address account) external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function pricePerShare() external view returns (uint256);
        function decimals() external view returns (uint8);
        function symbol() external view returns (string memory);
        function name() external view returns (string memory);
        function token() external view returns (address);
        function totalAssets() external view returns (uint256);
        function totalDebt() external view returns (uint256);
        function depositLimit() external view returns (uint256);
        function availableDepositLimit() external view returns (uint256);
        function performanceFee() external view returns (uint256);
        function managementFee() external view returns (uint256);
        
        // Strategy functions
        function withdrawalQueue(uint256 index) external view returns (address);
        function strategies(address strategy) external view returns (
            uint256 performanceFee,
            uint256 activation,
            uint256 debtRatio,
            uint256 rateLimit,
            uint256 lastReport,
            uint256 totalDebt,
            uint256 totalGain,
            uint256 totalLoss
        );
        
        // Events for tracking deposits/withdrawals
        event Deposit(address indexed recipient, uint256 amount, uint256 shares);
        event Withdraw(address indexed recipient, uint256 amount, uint256 shares);
        event StrategyAdded(address indexed strategy, uint256 debtRatio, uint256 rateLimit, uint256 performanceFee);
        event StrategyRevoked(address indexed strategy);
    }
    
    #[sol(rpc)]
    interface IYearnStrategy {
        function vault() external view returns (address);
        function want() external view returns (address);
        function name() external view returns (string memory);
        function estimatedTotalAssets() external view returns (uint256);
        function isActive() external view returns (bool);
        function delegatedAssets() external view returns (uint256);
        function version() external view returns (string memory);
    }
    
    #[sol(rpc)]
    interface IYearnRegistry {
        function latestVault(address token) external view returns (address);
        function vaults(address token, uint256 index) external view returns (address);
        function numVaults(address token) external view returns (uint256);
        function isRegistered(address vault) external view returns (bool);
    }
}

/// Yearn Finance yield vault adapter
pub struct YearnAdapter {
    client: EthereumClient,
    chain_id: u64,
    // Caches to prevent API spam
    vault_cache: Arc<Mutex<Option<CachedYearnData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    // HTTP client for API calls
    http_client: reqwest::Client,
    // Yearn registry contract
    registry_address: Option<Address>,
    // Risk calculator
    risk_calculator: YearnFinanceRiskCalculator,
}

impl YearnAdapter {
    /// Yearn API endpoints
    const YEARN_API_BASE: &'static str = "https://api.yearn.finance";
    
    /// Chain-specific Yearn registry addresses
    fn get_registry_address(chain_id: u64) -> Option<Address> {
        match chain_id {
            1 => Address::from_str("0x50c1a2eA0a861A967D9d0FFE2AE4012c2E053804").ok(), // Ethereum
            250 => Address::from_str("0x727fe1759430df13655ddb0731dE0D0FDE929b04").ok(), // Fantom  
            42161 => Address::from_str("0x3199437193625DCcD6F9C9e98BDf93582200Eb1f").ok(), // Arbitrum
            _ => None,
        }
    }
    
    /// Get chain name for Yearn API
    fn get_chain_name(chain_id: u64) -> &'static str {
        match chain_id {
            1 => "ethereum",
            250 => "fantom", 
            42161 => "arbitrum",
            10 => "optimism",
            137 => "polygon",
            _ => "ethereum", // fallback
        }
    }
    
    pub fn new(client: EthereumClient, chain_id: Option<u64>) -> Result<Self, AdapterError> {
        let chain_id = chain_id.unwrap_or(1);
        let registry_address = Self::get_registry_address(chain_id);
        
        Ok(Self {
            client,
            chain_id,
            vault_cache: Arc::new(Mutex::new(None)),
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(45))
                .user_agent("DeFi-Adapter/1.0")
                .build()
                .map_err(|e| AdapterError::RpcError(format!("Failed to create HTTP client: {}", e)))?,
            registry_address,
            risk_calculator: YearnFinanceRiskCalculator::new(),
        })
    }
    
    /// Fetch all vaults data from Yearn API with comprehensive caching
    async fn fetch_all_vaults_data(&self) -> Result<CachedYearnData, AdapterError> {
        // Check cache first (20-minute cache for vault data)
        {
            let cache = self.vault_cache.lock().unwrap();
            if let Some(cached_data) = cache.as_ref() {
                let cache_age = cached_data.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(1200) { // 20 minutes
                    tracing::info!(
                        cache_age_secs = cache_age.as_secs(),
                        vault_count = cached_data.vaults.len(),
                        "CACHE HIT: Using cached Yearn vault data"
                    );
                    return Ok(cached_data.clone());
                }
            }
        }
        
        let chain_name = Self::get_chain_name(self.chain_id);
        tracing::info!(chain = %chain_name, "CACHE MISS: Fetching fresh Yearn vault data from API");
        
        // Fetch vaults and earnings concurrently
        let (vaults_result, earnings_result) = tokio::join!(
            self.fetch_vaults(),
            self.fetch_earnings()
        );
        
        let vaults = vaults_result?;
        let earnings = earnings_result.unwrap_or_default();
        
        // Create vault address mapping for quick lookups
        let mut vault_map = HashMap::new();
        for vault in &vaults {
            vault_map.insert(vault.address.clone(), vault.clone());
        }
        
        let cached_data = CachedYearnData {
            vaults,
            vault_map,
            earnings,
            cached_at: SystemTime::now(),
        };
        
        // Update cache
        {
            let mut cache = self.vault_cache.lock().unwrap();
            *cache = Some(cached_data.clone());
        }
        
        tracing::info!(
            vault_count = cached_data.vaults.len(),
            earnings_count = cached_data.earnings.len(),
            "✅ Fetched and cached all Yearn data"
        );
        
        Ok(cached_data)
    }
    
    /// Fetch vaults from Yearn API
    async fn fetch_vaults(&self) -> Result<Vec<YearnVault>, AdapterError> {
        let chain_name = Self::get_chain_name(self.chain_id);
        let url = format!("{}/v1/chains/{}/vaults/all", Self::YEARN_API_BASE, chain_name);
        
        tracing::debug!("Fetching Yearn vaults from: {}", url);
        
        let response = timeout(Duration::from_secs(45), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::RpcError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::RpcError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AdapterError::RpcError(format!("HTTP error: {}", response.status())));
        }
        
        let mut vaults: Vec<YearnVault> = response
            .json()
            .await
            .map_err(|e| AdapterError::ContractError(format!("JSON parse error: {}", e)))?;
        
        // Filter active vaults and correct chain
        vaults.retain(|v| {
            v.chainID == self.chain_id &&
            !v.details.retired &&
            !v.details.hideAlways &&
            v.tvl.tvl > 1000.0 // Only vaults with reasonable TVL
        });
        
        // Sort by TVL descending
        vaults.sort_by(|a, b| b.tvl.tvl.partial_cmp(&a.tvl.tvl).unwrap_or(std::cmp::Ordering::Equal));
        
        tracing::info!(
            chain = %chain_name,
            total_vaults = vaults.len(),
            "Fetched Yearn vaults for chain"
        );
        
        Ok(vaults)
    }
    
    /// Fetch earnings from Yearn API
    async fn fetch_earnings(&self) -> Result<HashMap<String, YearnEarningsData>, AdapterError> {
        let chain_name = Self::get_chain_name(self.chain_id);
        let url = format!("{}/v1/chains/{}/vaults/earnings", Self::YEARN_API_BASE, chain_name);
        
        tracing::debug!("Fetching Yearn earnings from: {}", url);
        
        let response = timeout(Duration::from_secs(30), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::RpcError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::RpcError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            tracing::warn!("Failed to fetch Yearn earnings: {}", response.status());
            return Ok(HashMap::new());
        }
        
        let earnings: YearnVaultEarnings = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to parse Yearn earnings: {}", e);
                AdapterError::ContractError(format!("Earnings JSON parse error: {}", e))
            })?;
        
        tracing::info!(earnings_count = earnings.vault_earnings.len(), "Fetched Yearn vault earnings");
        
        Ok(earnings.vault_earnings)
    }
    
    /// Get user positions in Yearn vaults
    async fn get_user_yearn_positions(&self, address: Address) -> Result<Vec<YearnPosition>, AdapterError> {
        tracing::info!(
            user_address = %address,
            chain_id = self.chain_id,
            "🔍 Discovering ALL Yearn vault positions"
        );
        
        let cached_data = self.fetch_all_vaults_data().await?;
        let mut positions = Vec::new();
        
        // Check each vault for user balance
        for vault in &cached_data.vaults {
            if let Ok(vault_address) = Address::from_str(&vault.address) {
                match self.get_vault_position(address, vault, vault_address).await {
                    Ok(Some(position)) => {
                        let position_clone = position.clone();
                        tracing::info!(
                            vault_symbol = %vault.symbol,
                            vault_name = %vault.name,
                            balance = %position_clone.balance,
                            version = %vault.version,
                            "Found Yearn vault position"
                        );
                        positions.push(position_clone);
                    }
                    Ok(None) => {
                        // No position in this vault
                    }
                    Err(e) => {
                        tracing::warn!(
                            vault_symbol = %vault.symbol,
                            vault_address = %vault.address,
                            error = %e,
                            "Failed to check Yearn vault position"
                        );
                    }
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            total_positions = positions.len(),
            "✅ Discovered ALL Yearn positions"
        );
        
        Ok(positions)
    }
    
    /// Get user position in a specific vault
    async fn get_vault_position(
        &self,
        user_address: Address,
        vault: &YearnVault,
        vault_address: Address,
    ) -> Result<Option<YearnPosition>, AdapterError> {
        let vault_contract = IYearnVault::new(vault_address, self.client.provider());
        
        // Get user's vault token balance (shares)
        let shares = vault_contract.balanceOf(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get vault balance for {}: {}", vault.symbol, e)))?
            ._0;
        
        if shares == U256::ZERO {
            return Ok(None);
        }
        
        // Get price per share to calculate underlying balance
        let price_per_share_raw = vault_contract.pricePerShare().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get price per share for {}: {}", vault.symbol, e)))?
            ._0;
        
        let price_per_share = price_per_share_raw.try_into().unwrap_or(0u64) as f64 / 10f64.powi(vault.decimals as i32);
        
        // Calculate underlying token balance
        let shares_f64 = shares.try_into().unwrap_or(0u64) as f64 / 10f64.powi(vault.decimals as i32);
        let underlying_balance = shares_f64 * price_per_share;
        let underlying_balance_raw = U256::from((underlying_balance * 10f64.powi(vault.token.decimals as i32)) as u64);
        
        // Calculate total balance in vault token terms
        let balance_f64 = shares_f64 * price_per_share;
        let balance_raw = U256::from((balance_f64 * 10f64.powi(vault.token.decimals as i32)) as u64);
        
        Ok(Some(YearnPosition {
            vault_address,
            vault_name: vault.name.clone(),
            vault_symbol: vault.symbol.clone(),
            vault_version: vault.version.clone(),
            vault_type: vault.vault_type.clone(),
            category: vault.category.clone(),
            token: vault.token.clone(),
            balance: balance_raw,
            shares,
            underlying_balance: underlying_balance_raw,
            price_per_share,
            net_apy: vault.apy.net_apy,
            gross_apr: vault.apy.gross_apr,
            strategies: vault.strategies.clone(),
            fees: vault.fees.clone(),
            tvl: vault.tvl.clone(),
            chain_id: vault.chainID,
            is_migrable: vault.migration.as_ref().map(|m| m.available).unwrap_or(false),
            migration_target: vault.migration.as_ref().map(|m| m.address.clone()),
        }))
    }
    
    /// Calculate USD value of position with earnings
    async fn calculate_position_value(&self, position: &YearnPosition, cached_data: &CachedYearnData) -> (f64, f64, f64) {
        let balance_f64 = position.balance.try_into().unwrap_or(0u64) as f64 / 10f64.powi(position.token.decimals as i32);
        
        // Get token price (try multiple sources)
        let mut token_price = 0.0f64;
        
        // 1. Try CoinGecko for major tokens
        token_price = match position.token.symbol.to_uppercase().as_str() {
            "WETH" | "ETH" => self.get_token_price("ethereum").await.unwrap_or(0.0),
            "WBTC" | "BTC" => self.get_token_price("bitcoin").await.unwrap_or(0.0),
            "USDC" | "USDT" | "DAI" | "FRAX" => 1.0,
            "YFI" => self.get_token_price("yearn-finance").await.unwrap_or(0.0),
            "CRV" => self.get_token_price("curve-dao-token").await.unwrap_or(0.0),
            "CVX" => self.get_token_price("convex-finance").await.unwrap_or(0.0),
            _ => 0.0,
        };
        
        // 2. Fallback: estimate from vault TVL
        if token_price == 0.0 && position.tvl.totalAssetsUSD > 0.0 {
            let total_assets_f64 = if let Ok(total_assets_raw) = U256::from_str(&position.tvl.totalAssets) {
                total_assets_raw.try_into().unwrap_or(0u64) as f64 / 10f64.powi(position.token.decimals as i32)
            } else {
                position.tvl.tvl
            };
            
            if total_assets_f64 > 0.0 {
                token_price = position.tvl.totalAssetsUSD / total_assets_f64;
            }
        }
        
        // 3. Final fallback
        if token_price == 0.0 {
            token_price = match position.token.symbol.to_uppercase().as_str() {
                "WETH" | "ETH" => 4000.0,
                "WBTC" | "BTC" => 50000.0,
                "YFI" => 8000.0,
                _ => 1.0,
            };
        }
        
        let base_value_usd = balance_f64 * token_price;
        
        // Calculate earnings from vault performance
        let vault_earnings = cached_data.earnings.get(&position.vault_address.to_string().to_lowercase())
            .map(|e| e.earningsUSD)
            .unwrap_or_else(|| {
                // Estimate earnings based on APY and time
                let estimated_yearly_earnings = base_value_usd * (position.net_apy / 100.0);
                estimated_yearly_earnings * 0.25 // Assume 3 months average
            });
        
        tracing::info!(
            vault_symbol = %position.vault_symbol,
            balance = %balance_f64,
            token_price = %token_price,
            base_value_usd = %base_value_usd,
            net_apy = %position.net_apy,
            earnings_usd = %vault_earnings,
            "💰 Calculated Yearn position value"
        );
        
        (base_value_usd, vault_earnings, position.net_apy)
    }
    
    /// Get token price from CoinGecko
    async fn get_token_price(&self, coin_id: &str) -> Result<f64, AdapterError> {
        let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd", coin_id);
        
        if let Ok(response) = timeout(Duration::from_secs(10), self.http_client.get(&url).send()).await {
            if let Ok(response) = response {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if let Some(coin_data) = data.get(coin_id) {
                        if let Some(price) = coin_data.get("usd").and_then(|p| p.as_f64()) {
                            return Ok(price);
                        }
                    }
                }
            }
        }
        
        Err(AdapterError::RpcError("Failed to get token price from CoinGecko".to_string()))
    }
    
    /// Calculate comprehensive risk score for Yearn positions using dedicated risk calculator
    fn calculate_yearn_risk_score(&self, position: &YearnPosition) -> (f64, String) {
        // Convert position data to risk calculator format
        let risk_data = YearnRiskData {
            vault_version: position.vault_version.clone(),
            vault_type: position.vault_type.clone(),
            category: position.category.clone(),
            net_apy: position.net_apy,
            gross_apr: position.gross_apr,
            strategy_count: position.strategies.len(),
            strategy_types: position.strategies.iter()
                .map(|s| s.name.clone())
                .collect(),
            underlying_protocols: position.strategies.iter()
                .flat_map(|s| s.details.protocols.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
            performance_fee: position.fees.performance,
            management_fee: position.fees.management,
            withdrawal_fee: position.fees.withdrawal,
            chain_id: position.chain_id,
            tvl_usd: position.tvl.tvl,
            is_migrable: position.is_migrable,
            harvest_frequency_days: 2, // Default harvest frequency
            withdrawal_liquidity_usd: position.tvl.tvl * 0.1, // Estimate 10% withdrawal liquidity
            is_v3: position.vault_version.starts_with("0.3") || position.vault_version.starts_with("0.4"),
        };
        
        let (risk_score, _confidence, explanation) = self.risk_calculator.calculate_risk_score(&risk_data);
        
        (risk_score, explanation.explanation)
    }

    /// Check if address is a Yearn vault
    async fn is_yearn_vault(&self, vault_address: Address) -> Result<bool, AdapterError> {
        if let Some(registry_address) = self.registry_address {
            let registry = IYearnRegistry::new(registry_address, self.client.provider());
            
            match registry.isRegistered(vault_address).call().await {
                Ok(result) => Ok(result._0),
                Err(_) => {
                    // Fallback: check against cached vault list
                    let cached_data = self.fetch_all_vaults_data().await?;
                    Ok(cached_data.vault_map.contains_key(&vault_address.to_string().to_lowercase()))
                }
            }
        } else {
            // No registry for this chain, use API data
            let cached_data = self.fetch_all_vaults_data().await?;
            Ok(cached_data.vault_map.contains_key(&vault_address.to_string().to_lowercase()))
        }
    }

    /// Get vault info from contract
    async fn get_vault_info(&self, vault_address: Address) -> Result<(String, String, u8), AdapterError> {
    let vault_contract = IYearnVault::new(vault_address, self.client.provider());
    
    let name_result = vault_contract.name().call().await;
    let symbol_result = vault_contract.symbol().call().await;
    let decimals_result = vault_contract.decimals().call().await;
    
    let name = name_result
        .map_err(|e| AdapterError::ContractError(format!("Failed to get vault name: {}", e)))?
        ._0;
    let symbol = symbol_result
        .map_err(|e| AdapterError::ContractError(format!("Failed to get vault symbol: {}", e)))?
        ._0;
    let decimals = decimals_result
        .map_err(|e| AdapterError::ContractError(format!("Failed to get vault decimals: {}", e)))?
        ._0;
        
    Ok((name, symbol, decimals))
    }
}

#[async_trait]
impl DeFiAdapter for YearnAdapter {
    fn protocol_name(&self) -> &'static str {
        "Yearn Finance"
    }

    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
    tracing::info!(
        user_address = %address,
        chain_id = self.chain_id,
        adapter = self.protocol_name(),
        "🚀 Starting Yearn Finance position discovery"
    );
    
    // Check position cache first (5-minute cache for positions)
    {
        let cache = self.position_cache.lock().unwrap();
        if let Some(cached_positions) = cache.get(&address) {
            let cache_age = cached_positions.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
            if cache_age < Duration::from_secs(300) { // 5 minutes
                tracing::info!(
                    user_address = %address,
                    position_count = cached_positions.positions.len(),
                    cache_age_secs = cache_age.as_secs(),
                    "CACHE HIT: Using cached Yearn positions"
                );
                return Ok(cached_positions.positions.clone());
            }
        }
    }
    
    let yearn_positions = self.get_user_yearn_positions(address).await?;
    let cached_data = self.fetch_all_vaults_data().await?;
    
    let mut positions = Vec::new();
    
    for yearn_pos in yearn_positions {
        let (base_value_usd, earnings_usd, apy) = self.calculate_position_value(&yearn_pos, &cached_data).await;
        let total_value_usd = base_value_usd + earnings_usd;
        
        // Calculate risk score and explanation
        let (risk_score, risk_explanation) = self.calculate_yearn_risk_score(&yearn_pos);
        
        // Build strategy descriptions
        let strategies_desc = if yearn_pos.strategies.is_empty() {
            "Strategy information unavailable".to_string()
        } else {
            yearn_pos.strategies.iter()
                .take(3) // Limit to top 3 strategies for readability
                .map(|s| format!("{} (APR: {:.2}%)", s.name, s.details.apr))
                .collect::<Vec<_>>()
                .join(", ")
        };
        
        let position = Position {
            id: format!("yearn_{}_{}", yearn_pos.vault_address, address),
            protocol: self.protocol_name().to_string(),
            position_type: format!("Yearn {} Vault", yearn_pos.vault_type),
            pair: format!("{}/{}", yearn_pos.token.symbol, "USD"),
            value_usd: total_value_usd,
            pnl_usd: 0.0, // Would need historical data
            pnl_percentage: 0.0,
            risk_score: risk_score as u8,
            metadata: serde_json::json!({
                "vault_name": yearn_pos.vault_name,
                "vault_symbol": yearn_pos.vault_symbol,
                "vault_type": yearn_pos.vault_type,
                "vault_version": yearn_pos.vault_version,
                "vault_address": yearn_pos.vault_address.to_string(),
                "asset_symbol": yearn_pos.token.symbol,
                "asset_address": yearn_pos.token.address,
                "balance": yearn_pos.balance,
                "apy": apy,
                "tvl_usd": yearn_pos.tvl.totalAssetsUSD,
                "strategy_count": yearn_pos.strategies.len(),
                "strategies": strategies_desc,
                "is_endorsed": true, // Default for now
                "is_emergency_shutdown": false, // Default for now
                "management_fee": yearn_pos.fees.management,
                "performance_fee": yearn_pos.fees.performance,
                "migration_target": yearn_pos.migration_target,
                "chain_id": yearn_pos.chain_id,
                "risk_score": risk_score,
                "risk_level": match risk_score as u8 {
                    0..=20 => "Very Low",
                    21..=35 => "Low",
                    36..=50 => "Medium",
                    51..=70 => "High",
                    _ => "Critical"
                }
            }),
            last_updated: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        };
        
        positions.push(position);
        
        tracing::info!(
            vault_symbol = %yearn_pos.vault_symbol,
            vault_name = %yearn_pos.vault_name,
            balance_usd = %total_value_usd,
            apy = %apy,
            risk_score = %risk_score,
            earnings_usd = %earnings_usd,
            strategy_count = yearn_pos.strategies.len(),
            "📊 Created Yearn position"
        );
    }
    
    // Cache positions
    {
        let mut cache = self.position_cache.lock().unwrap();
        cache.insert(address, CachedPositions {
            positions: positions.clone(),
            cached_at: SystemTime::now(),
        });
    }
    
    tracing::info!(
        user_address = %address,
        total_positions = positions.len(),
        total_value_usd = positions.iter().map(|p| p.value_usd).sum::<f64>(),
        "✅ Completed Yearn Finance position discovery"
    );
    
    Ok(positions)
    }



    async fn supports_contract(&self, contract_address: Address) -> bool {
        // Check if the contract is a known Yearn vault
        self.is_yearn_vault(contract_address).await.unwrap_or(false)
    }

    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // Calculate weighted average risk score
        let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
        if total_value == 0.0 {
            return Ok(0);
        }
        
        let weighted_risk: f64 = positions.iter()
            .map(|p| (p.risk_score as f64) * (p.value_usd / total_value))
            .sum();
            
        Ok(weighted_risk.round() as u8)
    }

    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // For Yearn positions, the value is already calculated and stored
        Ok(position.value_usd)
    }
}

#[cfg(test)]
mod tests {
use super::*;
use std::str::FromStr;

#[tokio::test]
async fn test_yearn_adapter_creation() {
    // Mock client would be needed for actual testing
    // This is a basic structure test
    let chain_id = 1u64;
    assert!(YearnAdapter::get_registry_address(chain_id).is_some());
    assert_eq!(YearnAdapter::get_chain_name(chain_id), "ethereum");
}

#[test]
fn test_chain_configurations() {
    // Test all supported chains have proper configurations
    let supported_chains = vec![1, 250, 42161, 10, 137];
    
    for chain_id in supported_chains {
        let chain_name = YearnAdapter::get_chain_name(chain_id);
        assert!(!chain_name.is_empty());
        
        if chain_id == 1 || chain_id == 250 || chain_id == 42161 {
            // These chains should have registry addresses
            assert!(YearnAdapter::get_registry_address(chain_id).is_some());
        }
    }
}

#[test]
fn test_risk_score_bounds() {
    // Risk score should never exceed 95
    let mock_position = YearnPosition {
        vault_address: Address::ZERO,
        vault_name: "Test Vault".to_string(),
        vault_symbol: "TEST".to_string(),
        vault_version: "0.1.0".to_string(),
        vault_type: "experimental".to_string(),
        category: "volatile".to_string(),
        token: YearnToken {
            address: "0x0000000000000000000000000000000000000000".to_string(),
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            description: "Test".to_string(),
            decimals: 18,
        },
        balance: U256::ZERO,
        shares: U256::ZERO,
        underlying_balance: U256::ZERO,
        price_per_share: 1.0,
        net_apy: 200.0, // Unrealistic APY
        gross_apr: 220.0,
        strategies: vec![],
        fees: YearnFees {
            performance: 50.0, // High fees
            withdrawal: 10.0,
            management: 10.0,
            keep_crv: 0.0,
            cvx_keep_crv: 0.0,
        },
        tvl: YearnTVL {
            totalAssets: "1000".to_string(),
            totalAssetsUSD: 1000.0,
            tvl: 1000.0, // Low TVL
        },
        chain_id: 9999, // Unknown chain
        is_migrable: true,
        migration_target: Some("0x123".to_string()),
    };
    
    // This would require creating an adapter instance to test
    // Risk score calculation logic should cap at 95
}
}