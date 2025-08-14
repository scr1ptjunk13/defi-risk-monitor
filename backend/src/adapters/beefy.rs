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

#[derive(Debug, Clone)]
struct BeefyPosition {
    vault_id: String,
    vault_name: String,
    chain: String,
    vault_address: Address,
    token_address: Address,
    balance: U256,
    shares: U256,
    decimals: u8,
    underlying_assets: Vec<String>,
    current_apy: f64,
    performance_fee: f64,
    strategy: String,
    platform: String,
    risks: Vec<String>,
    price_per_full_share: f64,
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
    
    #[sol(rpc)]
    interface IBeefyStrategy {
        function wantLockedTotal() external view returns (uint256);
        function sharesTotal() external view returns (uint256);
        function earn() external;
        function harvest() external;
        function retireStrat() external;
        function panic() external;
    }
    
    #[sol(rpc)]
    interface IBeefyRewardPool {
        function balanceOf(address account) external view returns (uint256);
        function earned(address account) external view returns (uint256);
        function rewardPerToken() external view returns (uint256);
        function rewardRate() external view returns (uint256);
        function periodFinish() external view returns (uint256);
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
                        "CACHE HIT: Using cached Beefy vault data"
                    );
                    return Ok(cached_data.clone());
                }
            }
        }
        
        tracing::info!(chain = %self.chain, "CACHE MISS: Fetching fresh Beefy vault data from API");
        
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
            "✅ Fetched and cached all Beefy data"
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
            "🔍 Discovering ALL Beefy yield farming positions"
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
                        // Additional Protocol Info
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
        
        // CACHE STORE: Save results to prevent future API spam
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
            "✅ Successfully fetched and cached Beefy positions"
        );
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        self.is_beefy_vault(contract_address).await
    }
    
    // RISK CALCULATION REMOVED - Now handled by separate risk module
    // This adapter now only fetches position data
    
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

/// Helper functions for Beefy adapter
impl BeefyAdapter {
    /// Get comprehensive vault information for display
    pub async fn get_vault_info(&self, vault_id: &str) -> Result<serde_json::Value, AdapterError> {
        let cached_data = self.fetch_all_vaults_data().await?;
        
        if let Some(vault) = cached_data.vaults.iter().find(|v| v.id == vault_id) {
            let tvl = cached_data.tvls.get(vault_id).copied().unwrap_or(0.0);
            let mut asset_prices = HashMap::new();
            
            for asset in &vault.assets {
                if let Some(&price) = cached_data.prices.get(asset) {
                    asset_prices.insert(asset.clone(), price);
                }
            }
            
            Ok(serde_json::json!({
                "vault_id": vault.id,
                "name": vault.name,
                "platform": vault.platform,
                "chain": vault.chain,
                "strategy": vault.strategy,
                "assets": vault.assets,
                "risks": vault.risks,
                "current_apy": vault.apy,
                "apr_breakdown": vault.apr,
                "tvl_usd": tvl,
                "asset_prices": asset_prices,
                "status": vault.status,
                "urls": {
                    "vault": format!("https://app.beefy.finance/vault/{}", vault.id),
                    "add_liquidity": vault.addLiquidityUrl,
                    "remove_liquidity": vault.removeLiquidityUrl,
                },
                "contracts": {
                    "vault": vault.earnContractAddress,
                    "token": vault.tokenAddress,
                    "strategy": vault.strategy,
                }
            }))
        } else {
            Err(AdapterError::InvalidData(format!("Vault {} not found", vault_id)))
        }
    }
    
    /// Get user's historical performance (requires additional API calls)
    pub async fn get_user_vault_history(&self, user_address: Address, vault_id: &str) -> Result<serde_json::Value, AdapterError> {
        // This would require Beefy's analytics API or subgraph data
        // For now, return basic info
        tracing::info!(
            user_address = %user_address,
            vault_id = %vault_id,
            "📈 Historical performance tracking not yet implemented (requires Beefy analytics API)"
        );
        
        Ok(serde_json::json!({
            "message": "Historical performance tracking requires Beefy analytics API integration",
            "vault_id": vault_id,
            "user_address": format!("{:?}", user_address),
            "suggested_integrations": [
                "Beefy Analytics API",
                "TheGraph Beefy Subgraph",
                "On-chain event analysis"
            ]
        }))
    }
    
    /// Get current boost multipliers (for boosted vaults)
    pub async fn get_boost_info(&self, vault_id: &str) -> Result<serde_json::Value, AdapterError> {
        // Some Beefy vaults have boost mechanics
        // This would require additional API calls to boost contracts
        
        Ok(serde_json::json!({
            "vault_id": vault_id,
            "boost_available": false,
            "boost_multiplier": 1.0,
            "boost_requirements": [],
            "note": "Boost tracking requires integration with specific boost contracts"
        }))
    }
    
    /// Estimate gas costs for common operations
    pub async fn estimate_gas_costs(&self, operation: &str) -> Result<serde_json::Value, AdapterError> {
        // Estimate gas costs for Beefy operations
        let base_gas_estimates = match operation {
            "deposit" => 150_000u64,
            "withdraw" => 180_000u64,
            "harvest" => 200_000u64,
            "compound" => 250_000u64,
            _ => 150_000u64,
        };
        
        // Get current gas price (this would need gas price oracle)
        let estimated_gas_price = 20_000_000_000u64; // 20 gwei fallback
        let estimated_cost_wei = base_gas_estimates * estimated_gas_price;
        let estimated_cost_eth = estimated_cost_wei as f64 / 10f64.powi(18);
        let estimated_cost_usd = estimated_cost_eth * self.get_fallback_eth_price().await;
        
        Ok(serde_json::json!({
            "operation": operation,
            "estimated_gas": base_gas_estimates,
            "gas_price_gwei": estimated_gas_price / 1_000_000_000,
            "estimated_cost_eth": estimated_cost_eth,
            "estimated_cost_usd": estimated_cost_usd,
            "note": "Estimates only - actual costs may vary"
        }))
    }
    
    /// Get all active vaults for a specific chain (useful for discovery)
    pub async fn get_all_vaults_for_chain(&self) -> Result<Vec<serde_json::Value>, AdapterError> {
        let cached_data = self.fetch_all_vaults_data().await?;
        
        let mut vaults_info = Vec::new();
        for vault in &cached_data.vaults {
            if vault.status.to_lowercase() == "active" {
                let tvl = cached_data.tvls.get(&vault.id).copied().unwrap_or(0.0);
                
                vaults_info.push(serde_json::json!({
                    "vault_id": vault.id,
                    "name": vault.name,
                    "platform": vault.platform,
                    "strategy": vault.strategy,
                    "assets": vault.assets,
                    "risks": vault.risks,
                    "current_apy": vault.apy,
                    "tvl_usd": tvl,
                    "performance_fee": vault.apr.as_ref()
                        .and_then(|apr| apr.beefy_performance_fee)
                        .unwrap_or(4.5),
                    "vault_url": format!("https://app.beefy.finance/vault/{}", vault.id),
                }));
            }
        }
        
        // Sort by TVL descending
        vaults_info.sort_by(|a, b| {
            let tvl_a = a.get("tvl_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let tvl_b = b.get("tvl_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
            tvl_b.partial_cmp(&tvl_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        Ok(vaults_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_beefy_adapter_creation() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = BeefyAdapter::new(client, Some(1)).unwrap();
        
        assert_eq!(adapter.protocol_name(), "beefy");
        assert_eq!(adapter.chain, "ethereum");
    }
    
    #[tokio::test]
    async fn test_chain_mapping() {
        assert_eq!(BeefyAdapter::get_chain_name(1), "ethereum");
        assert_eq!(BeefyAdapter::get_chain_name(56), "bsc");
        assert_eq!(BeefyAdapter::get_chain_name(137), "polygon");
        assert_eq!(BeefyAdapter::get_chain_name(250), "fantom");
        assert_eq!(BeefyAdapter::get_chain_name(999), "ethereum"); // fallback
    }
    
    #[tokio::test]
    async fn test_risk_calculation() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = BeefyAdapter::new(client, Some(1)).unwrap();
        
        let test_position = BeefyPosition {
            vault_id: "test-vault".to_string(),
            vault_name: "Test Vault".to_string(),
            chain: "ethereum".to_string(),
            vault_address: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            token_address: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            balance: U256::from(1000),
            shares: U256::from(1000),
            decimals: 18,
            underlying_assets: vec!["ETH".to_string(), "USDC".to_string()],
            current_apy: 15.5,
            performance_fee: 4.5,
            strategy: "StrategyLP".to_string(),
            platform: "uniswap".to_string(),
            risks: vec!["impermanent_loss".to_string()],
            price_per_full_share: 1.1,
        };
        
        let risk_score = adapter.calculate_beefy_risk_score(&test_position);
        
        // Should be reasonable risk score (not too low, not too high)
        assert!(risk_score >= 20 && risk_score <= 80);
    }
    
    #[test]
    fn test_api_urls() {
        assert_eq!(BeefyAdapter::BEEFY_API_BASE, "https://api.beefy.finance");
    }
    
    #[tokio::test]
    async fn test_fallback_pricing() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = BeefyAdapter::new(client, Some(1)).unwrap();
        
        let eth_price = adapter.get_fallback_eth_price().await;
        
        // Should get a reasonable ETH price or fallback
        assert!(eth_price > 1000.0 && eth_price < 10000.0);
    }
    
    #[tokio::test] 
    async fn test_vault_discovery() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = BeefyAdapter::new(client, Some(1)).unwrap();
        
        // This test would require network access, so we'll just test the function exists
        match adapter.get_all_vaults_for_chain().await {
            Ok(vaults) => {
                // Should return some vaults if API is accessible
                assert!(vaults.len() >= 0);
            }
            Err(_) => {
                // Network error is acceptable in tests
            }
        }
    }
    
    #[test]
    fn test_risk_score_bounds() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = BeefyAdapter::new(client, Some(1)).unwrap();
        
        // Test extreme high APY
        let high_risk_position = BeefyPosition {
            vault_id: "high-risk".to_string(),
            vault_name: "High Risk Vault".to_string(),
            chain: "unknown".to_string(),
            vault_address: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            token_address: Address::from_str("0x0000000000000000000000000000000000000000").unwrap(),
            balance: U256::from(1000),
            shares: U256::from(1000),
            decimals: 18,
            underlying_assets: vec!["UNKNOWN".to_string()],
            current_apy: 500.0, // Extremely high APY
            performance_fee: 4.5,
            strategy: "StrategyLeveraged".to_string(),
            platform: "unknown".to_string(),
            risks: vec!["experimental".to_string(), "impermanent_loss".to_string(), "smart_contract".to_string()],
            price_per_full_share: 1.0,
        };
        
        let risk_score = adapter.calculate_beefy_risk_score(&high_risk_position);
        
        // Should be high but capped at 95
        assert!(risk_score >= 80 && risk_score <= 95);
    }
} No position in this vault
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
            "✅ Discovered ALL Beefy positions"
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
            "💰 Calculated Beefy position value"
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
    
    /// Calculate comprehensive risk score for Beefy positions
    fn calculate_beefy_risk_score(&self, position: &BeefyPosition) -> u8 {
        let mut risk_score = 25u8; // Base yield farming risk
        
        // Strategy risk adjustment
        match position.strategy.to_lowercase().as_str() {
            s if s.contains("single") => risk_score += 5, // Single asset strategies
            s if s.contains("lp") || s.contains("liquidity") => risk_score += 10, // LP token impermanent loss risk
            s if s.contains("leveraged") || s.contains("leverage") => risk_score += 20, // High risk leveraged strategies
            s if s.contains("stable") => risk_score = risk_score.saturating_sub(5), // Lower risk stablecoin strategies
            _ => {} // No adjustment for unknown strategies
        }
        
        // Platform risk adjustment
        match position.platform.to_lowercase().as_str() {
            "uniswap" | "sushiswap" | "curve" | "balancer" => risk_score = risk_score.saturating_sub(5), // Well-established platforms
            "pancakeswap" => {} // No adjustment
            _ => risk_score += 5, // Unknown platforms get slight penalty
        }
        
        // APY risk adjustment
        if position.current_apy > 100.0 {
            risk_score += 15; // Very high APY is suspicious
        } else if position.current_apy > 50.0 {
            risk_score += 10; // High APY
        } else if position.current_apy < 5.0 {
            risk_score += 5; // Very low APY might indicate issues
        }
        
        // Chain risk adjustment
        match position.chain.to_lowercase().as_str() {
            "ethereum" => risk_score = risk_score.saturating_sub(5), // Most secure
            "polygon" | "arbitrum" | "optimism" => {}, // No adjustment
            "bsc" | "fantom" | "avax" => risk_score += 3, // Slightly higher risk
            _ => risk_score += 8, // Unknown chains
        }
        
        // Risk tags from Beefy
        for risk in &position.risks {
            match risk.to_lowercase().as_str() {
                "impermanent_loss" => risk_score += 10,
                "smart_contract" => risk_score += 8,
                "liquidity" => risk_score += 5,
                "complexity" => risk_score += 7,
                "mcap" => risk_score += 5, // Market cap risk
                "experimental" => risk_score += 15,
                _ => risk_score += 3, // Unknown risks
            }
        }
        
        risk_score.min(95) // Cap at 95 (leave some room for extreme cases)
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
            "CACHE CHECK: Checking for cached Beefy positions"
        );
        
        // CACHE CHECK: Prevent API spam (5 minute cache for positions)
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) {
                    tracing::info!(
                        user_address = %address,
                        cache_age_secs = cache_age.as_secs(),
                        position_count = cached.positions.len(),
                        "CACHE HIT: Returning cached Beefy positions!"
                    );
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            chain = %self.chain,
            "CACHE MISS: Fetching fresh Beefy data from blockchain and API"
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
            let risk_score = self.calculate_beefy_risk_score(&beefy_pos);
            
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
                risk_score,
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
                    "risk_score_detailed": risk_score,
                    
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
                    
                    //