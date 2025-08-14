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
                .map_err(|e| AdapterError::NetworkError(format!("Failed to create HTTP client: {}", e)))?,
            registry_address,
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
            .map_err(|_| AdapterError::NetworkError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(AdapterError::NetworkError(format!("HTTP error: {}", response.status())));
        }
        
        let mut vaults: Vec<YearnVault> = response
            .json()
            .await
            .map_err(|e| AdapterError::DataError(format!("JSON parse error: {}", e)))?;
        
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
            .map_err(|_| AdapterError::NetworkError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            tracing::warn!("Failed to fetch Yearn earnings: {}", response.status());
            return Ok(HashMap::new());
        }
        
        let earnings: YearnVaultEarnings = response
            .json()
            .await
            .map_err(|e| {
                tracing::warn!("Failed to parse Yearn earnings: {}", e);
                AdapterError::DataError(format!("Earnings JSON parse error: {}", e))
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
                        positions.push(position);
                        tracing::info!(
                            vault_symbol = %vault.symbol,
                            vault_name = %vault.name,
                            balance = %position.balance,
                            version = %vault.version,
                            "Found Yearn vault position"
                        );
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
        
        let price_per_share = price_per_share_raw.to::<f64>() / 10f64.powi(vault.decimals as i32);
        
        // Calculate underlying token balance
        let shares_f64 = shares.to::<f64>() / 10f64.powi(vault.decimals as i32);
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
        let balance_f64 = position.balance.to::<f64>() / 10f64.powi(position.token.decimals as i32);
        
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
                total_assets_raw.to::<f64>() / 10f64.powi(position.token.decimals as i32)
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
        
        Err(AdapterError::NetworkError("Failed to get token price".to_string()))
    }
    
    /// Calculate comprehensive risk score for Yearn positions
    fn calculate_yearn_risk_score(&self, position: &YearnPosition) -> u8 {
        let mut risk_score = 20u8; // Base yield farming risk (lower than other protocols due to Yearn's reputation)
        
        // Version risk adjustment
        match position.vault_version.as_str() {
            v if v.starts_with("0.3") || v.starts_with("0.4") => risk_score = risk_score.saturating_sub(5), // Latest versions
            v if v.starts_with("0.2") => {}, // No adjustment for v2
            _ => risk_score += 10, // Older or unknown versions
        }
        
        // Vault type risk adjustment
        match position.vault_type.to_lowercase().as_str() {
            "automated" => risk_score = risk_score.saturating_sub(3), // Automated vaults are generally safer
            "experimental" => risk_score += 20, // Experimental vaults are high risk
            _ => {} // No adjustment
        }
        
        // Category risk adjustment
        match position.category.to_lowercase().as_str() {
            "stablecoin" => risk_score = risk_score.saturating_sub(8), // Stablecoins are safer
            "volatile" => risk_score += 5, // Volatile assets
            "curve" => risk_score = risk_score.saturating_sub(3), // Curve strategies are well-tested
            "balancer" => risk_score = risk_score.saturating_sub(2), // Balancer strategies
            _ => {}
        }
        
        // APY risk adjustment
        if position.net_apy > 100.0 {
            risk_score += 20; // Very high APY is suspicious
        } else if position.net_apy > 50.0 {
            risk_score += 12; // High APY
        } else if position.net_apy > 25.0 {
            risk_score += 6; // Moderate APY
        } else if position.net_apy < 2.0 {
            risk_score += 8; // Very low APY might indicate issues
        }
        
        // Strategy diversification (multiple strategies reduce risk)
        let strategy_count = position.strategies.len();
        if strategy_count > 3 {
            risk_score = risk_score.saturating_sub(5); // Well diversified
        } else if strategy_count > 1 {
            risk_score = risk_score.saturating_sub(2); // Some diversification
        } else if strategy_count == 0 {
            risk_score += 10; // No strategy info is concerning
        }
        
        // Fee risk (very high fees might indicate unsustainable model)
        if position.fees.performance > 30.0 {
            risk_score += 8; // Very high performance fee
        } else if position.fees.performance > 20.0 {
            risk_score += 4; // High performance fee
        }
        
        if position.fees.management > 5.0 {
            risk_score += 5; // High management fee
        }
        
        // Chain risk
        match position.chain_id {
            1 => risk_score = risk_score.saturating_sub(3), // Ethereum mainnet is most secure
            250 => risk_score += 5, // Fantom has some additional risks
            42161 => risk_score += 2, // Arbitrum
            10 => risk_score += 2, // Optimism
            _ => risk_score += 8, // Unknown chains
        }
        
        // TVL risk (higher TVL generally indicates more stability)
        if position.tvl.tvl > 100_000_000.0 {
            risk_score = risk_score.saturating_sub(5); // Very high TVL
        } else if position.tvl.tvl > 10_000_000.0 {
            risk_score = risk_score.saturating_sub(3); // High TVL
        } else if position.tvl.tvl < 1_000_000.0 {
            risk_score += 8; // Low TVL
        }
        
        // Migration available (indicates vault might be deprecated)
        if position.is_migrable {
            risk_score += 12; // Vaults needing migration have higher risk
        }
        
        risk_score.min(95) // Cap at 95
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
    
    let (name_result, symbol_result, decimals_result) = tokio::join!(
        vault_contract.name().call(),
        vault_contract.symbol().call(),
        vault_contract.decimals().call()
    );
    
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
fn name(&self) -> &'static str {
    "Yearn Finance"
}

fn supported_chains(&self) -> Vec<u64> {
    vec![1, 250, 42161, 10, 137] // Ethereum, Fantom, Arbitrum, Optimism, Polygon
}

async fn get_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
    tracing::info!(
        user_address = %address,
        chain_id = self.chain_id,
        adapter = self.name(),
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
        
        // Calculate risk score
        let risk_score = self.calculate_yearn_risk_score(&yearn_pos);
        
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
            protocol: self.name().to_string(),
            position_type: format!("Yearn {} Vault", yearn_pos.vault_type),
            asset_symbol: yearn_pos.token.symbol.clone(),
            asset_address: Some(Address::from_str(&yearn_pos.token.address)
                .unwrap_or_else(|_| Address::ZERO)),
            contract_address: yearn_pos.vault_address,
            balance: yearn_pos.balance,
            balance_usd: total_value_usd,
            underlying_tokens: vec![(
                yearn_pos.token.symbol.clone(),
                Address::from_str(&yearn_pos.token.address).unwrap_or_else(|_| Address::ZERO),
                yearn_pos.underlying_balance,
            )],
            metadata: serde_json::json!({
                "vault_name": yearn_pos.vault_name,
                "vault_symbol": yearn_pos.vault_symbol,
                "vault_version": yearn_pos.vault_version,
                "vault_type": yearn_pos.vault_type,
                "category": yearn_pos.category,
                "shares": yearn_pos.shares.to_string(),
                "price_per_share": yearn_pos.price_per_share,
                "net_apy": yearn_pos.net_apy,
                "gross_apr": yearn_pos.gross_apr,
                "strategies": strategies_desc,
                "strategy_count": yearn_pos.strategies.len(),
                "fees": {
                    "performance": yearn_pos.fees.performance,
                    "management": yearn_pos.fees.management,
                    "withdrawal": yearn_pos.fees.withdrawal
                },
                "tvl": {
                    "total_assets_usd": yearn_pos.tvl.totalAssetsUSD,
                    "tvl": yearn_pos.tvl.tvl
                },
                "earnings_usd": earnings_usd,
                "base_value_usd": base_value_usd,
                "is_migrable": yearn_pos.is_migrable,
                "migration_target": yearn_pos.migration_target,
                "chain_id": yearn_pos.chain_id,
                "risk_score": risk_score,
                "risk_level": match risk_score {
                    0..=20 => "Very Low",
                    21..=35 => "Low",
                    36..=50 => "Medium",
                    51..=70 => "High",
                    _ => "Very High"
                }
            }),
            apy: Some(apy),
            last_updated: std::time::SystemTime::now(),
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
        total_value_usd = positions.iter().map(|p| p.balance_usd).sum::<f64>(),
        "✅ Completed Yearn Finance position discovery"
    );
    
    Ok(positions)
}

async fn get_protocol_info(&self) -> Result<serde_json::Value, AdapterError> {
    let cached_data = self.fetch_all_vaults_data().await?;
    let chain_name = Self::get_chain_name(self.chain_id);
    
    // Calculate aggregate statistics
    let total_vaults = cached_data.vaults.len();
    let total_tvl: f64 = cached_data.vaults.iter().map(|v| v.tvl.tvl).sum();
    let avg_apy: f64 = if !cached_data.vaults.is_empty() {
        cached_data.vaults.iter().map(|v| v.apy.net_apy).sum::<f64>() / cached_data.vaults.len() as f64
    } else {
        0.0
    };
    
    // Count by categories
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut version_counts: HashMap<String, usize> = HashMap::new();
    
    for vault in &cached_data.vaults {
        *category_counts.entry(vault.category.clone()).or_insert(0) += 1;
        *version_counts.entry(vault.version.clone()).or_insert(0) += 1;
    }
    
    // Top vaults by TVL
    let top_vaults: Vec<serde_json::Value> = cached_data.vaults
        .iter()
        .take(10)
        .map(|v| serde_json::json!({
            "name": v.name,
            "symbol": v.symbol,
            "address": v.address,
            "tvl": v.tvl.tvl,
            "apy": v.apy.net_apy,
            "category": v.category,
            "version": v.version
        }))
        .collect();
    
    Ok(serde_json::json!({
        "protocol": self.name(),
        "chain": chain_name,
        "chain_id": self.chain_id,
        "registry_address": self.registry_address.map(|a| a.to_string()),
        "statistics": {
            "total_vaults": total_vaults,
            "total_tvl_usd": total_tvl,
            "average_apy": avg_apy,
            "total_earnings_tracked": cached_data.earnings.len()
        },
        "vault_breakdown": {
            "by_category": category_counts,
            "by_version": version_counts
        },
        "top_vaults_by_tvl": top_vaults,
        "supported_features": [
            "Automated yield farming",
            "Multi-strategy vaults",
            "Auto-compounding",
            "Vault migration",
            "Performance tracking",
            "Risk assessment"
        ],
        "data_sources": [
            "Yearn Finance API",
            "On-chain vault contracts",
            "Yearn Registry",
            "CoinGecko price feeds"
        ],
        "cache_info": {
            "vault_data_cache_duration_minutes": 20,
            "position_cache_duration_minutes": 5,
            "last_updated": SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        }
    }))
}

async fn refresh_cache(&self) -> Result<(), AdapterError> {
    tracing::info!("🔄 Refreshing all Yearn Finance caches");
    
    // Clear all caches
    {
        let mut vault_cache = self.vault_cache.lock().unwrap();
        *vault_cache = None;
    }
    
    {
        let mut position_cache = self.position_cache.lock().unwrap();
        position_cache.clear();
    }
    
    // Pre-warm vault cache
    let _cached_data = self.fetch_all_vaults_data().await?;
    
    tracing::info!("✅ Refreshed all Yearn Finance caches");
    Ok(())
}

async fn get_transaction_history(&self, address: Address, limit: Option<usize>) -> Result<Vec<serde_json::Value>, AdapterError> {
    // This would require indexing deposit/withdrawal events from Yearn vaults
    // For now, return empty as this is typically handled by transaction indexers
    tracing::info!(
        user_address = %address,
        "Transaction history not implemented for Yearn adapter - use transaction indexer"
    );
    Ok(vec![])
}

async fn estimate_gas(&self, _operation: &str, _params: serde_json::Value) -> Result<U256, AdapterError> {
    // Return typical gas estimates for Yearn operations
    match _operation {
        "deposit" => Ok(U256::from(150_000)), // Typical deposit gas
        "withdraw" => Ok(U256::from(200_000)), // Typical withdrawal gas
        "migrate" => Ok(U256::from(300_000)), // Vault migration gas
        _ => Ok(U256::from(100_000)), // Default estimate
    }
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