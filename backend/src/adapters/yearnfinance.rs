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
use tokio::time::timeout;

#[derive(Debug, Clone)]
pub struct EthereumClient {
    pub rpc_url: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnVault {
    address: String,
    #[serde(rename = "type")]
    vault_type: String,
    symbol: String,
    name: String,
    category: String,
    version: String,
    decimals: u8,
    chain_id: u64,
    token: YearnToken,
    tvl: YearnTVL,
    apy: YearnAPY,
    strategies: Vec<YearnStrategy>,
    details: YearnVaultDetails,
    fees: YearnFees,
    migration: Option<YearnMigration>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnToken {
    address: String,
    name: String,
    symbol: String,
    decimals: u8,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnTVL {
    total_assets: String,
    total_assets_usd: f64,
    tvl: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnAPY {
    gross_apr: f64,
    net_apy: f64,
    fees: YearnAPYFees,
    points: Option<YearnAPYPoints>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnAPYFees {
    performance: f64,
    withdrawal: f64,
    management: f64,
    keep_crv: f64,
    cvx_keep_crv: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnAPYPoints {
    week_ago: f64,
    month_ago: f64,
    inception: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnStrategy {
    address: String,
    name: String,
    details: YearnStrategyDetails,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnStrategyDetails {
    total_debt: String,
    debt_ratio: u64,
    apr: f64,
    version: String,
    protocols: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnVaultDetails {
    management: String,
    governance: String,
    deposit_limit: String,
    performance_fee: u64,
    management_fee: u64,
    deposits_disabled: bool,
    withdrawals_disabled: bool,
    retired: bool,
    hide_always: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnFees {
    performance: f64,
    withdrawal: f64,
    management: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnMigration {
    available: bool,
    address: String,
}

#[derive(Debug, Deserialize, Clone)]
struct YearnVaultEarnings {
    #[serde(flatten)]
    vault_earnings: HashMap<String, YearnEarningsData>,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
struct YearnEarningsData {
    earnings: f64,
    earnings_usd: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    vault_map: HashMap<String, YearnVault>,
    earnings: HashMap<String, YearnEarningsData>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

sol! {
    #[sol(rpc)]
    interface IYearnVault {
        function balanceOf(address account) external view returns (uint256);
        function pricePerShare() external view returns (uint256);
        function decimals() external view returns (uint8);
        function symbol() external view returns (string memory);
        function name() external view returns (string memory);
        function token() external view returns (address);
    }
    
    #[sol(rpc)]
    interface IYearnRegistry {
        function latestVault(address token) external view returns (address);
        function isRegistered(address vault) external view returns (bool);
    }
}

pub struct YearnAdapter {
    #[allow(dead_code)]
    client: EthereumClient,
    chain_id: u64,
    vault_cache: Arc<Mutex<Option<CachedYearnData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    http_client: reqwest::Client,
    #[allow(dead_code)]
    registry_address: Option<Address>,
}

impl YearnAdapter {
    const YEARN_API_BASE: &'static str = "https://api.yearn.finance";
    
    fn get_registry_address(chain_id: u64) -> Option<Address> {
        match chain_id {
            1 => Address::from_str("0x50c1a2eA0a861A967D9d0FFE2AE4012c2E053804").ok(),
            250 => Address::from_str("0x727fe1759430df13655ddb0731dE0D0FDE929b04").ok(),
            42161 => Address::from_str("0x3199437193625DCcD6F9C9e98BDf93582200Eb1f").ok(),
            _ => None,
        }
    }
    
    fn get_chain_name(chain_id: u64) -> &'static str {
        match chain_id {
            1 => "ethereum",
            250 => "fantom",
            42161 => "arbitrum",
            10 => "optimism",
            137 => "polygon",
            _ => "ethereum",
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
        })
    }
    
    async fn fetch_all_vaults_data(&self) -> Result<CachedYearnData, AdapterError> {
        // Check cache first (20-minute cache)
        {
            let cache = self.vault_cache.lock().unwrap();
            if let Some(cached_data) = cache.as_ref() {
                let cache_age = cached_data.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(1200) {
                    return Ok(cached_data.clone());
                }
            }
        }
        
        // Fetch vaults and earnings concurrently
        let (vaults_result, earnings_result) = tokio::join!(
            self.fetch_vaults(),
            self.fetch_earnings()
        );
        
        let vaults = vaults_result?;
        let earnings = earnings_result.unwrap_or_default();
        
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
        
        {
            let mut cache = self.vault_cache.lock().unwrap();
            *cache = Some(cached_data.clone());
        }
        
        Ok(cached_data)
    }
    
    async fn fetch_vaults(&self) -> Result<Vec<YearnVault>, AdapterError> {
        let chain_name = Self::get_chain_name(self.chain_id);
        let url = format!("{}/v1/chains/{}/vaults/all", Self::YEARN_API_BASE, chain_name);
        
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
        
        // Filter active vaults
        vaults.retain(|v| {
            v.chain_id == self.chain_id &&
            !v.details.retired &&
            !v.details.hide_always &&
            v.tvl.tvl > 1000.0
        });
        
        vaults.sort_by(|a, b| b.tvl.tvl.partial_cmp(&a.tvl.tvl).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(vaults)
    }
    
    async fn fetch_earnings(&self) -> Result<HashMap<String, YearnEarningsData>, AdapterError> {
        let chain_name = Self::get_chain_name(self.chain_id);
        let url = format!("{}/v1/chains/{}/vaults/earnings", Self::YEARN_API_BASE, chain_name);
        
        let response = timeout(Duration::from_secs(30), self.http_client.get(&url).send())
            .await
            .map_err(|_| AdapterError::RpcError("Request timeout".to_string()))?
            .map_err(|e| AdapterError::RpcError(format!("HTTP request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Ok(HashMap::new());
        }
        
        let earnings: YearnVaultEarnings = response
            .json()
            .await
            .map_err(|e| {
                AdapterError::ContractError(format!("Earnings JSON parse error: {}", e))
            })?;
        
        Ok(earnings.vault_earnings)
    }
    
    async fn get_user_yearn_positions(&self, address: Address) -> Result<Vec<YearnPosition>, AdapterError> {
        let cached_data = self.fetch_all_vaults_data().await?;
        let mut positions = Vec::new();
        
        for vault in &cached_data.vaults {
            if let Ok(vault_address) = Address::from_str(&vault.address) {
                match self.get_vault_position(address, vault, vault_address).await {
                    Ok(Some(position)) => positions.push(position),
                    Ok(None) => {},
                    Err(_) => {},
                }
            }
        }
        
        Ok(positions)
    }
    
    async fn get_vault_position(
        &self,
        _user_address: Address,
        vault: &YearnVault,
        vault_address: Address,
    ) -> Result<Option<YearnPosition>, AdapterError> {
        // Placeholder values - replace with actual contract calls
        let shares = U256::from(1000u64);
        
        if shares == U256::ZERO {
            return Ok(None);
        }
        
        let price_per_share_raw = U256::from(1050000000000000000u64); // 1.05 in 18 decimals
        let price_per_share = price_per_share_raw.try_into().unwrap_or(0u64) as f64 / 10f64.powi(vault.decimals as i32);
        
        let shares_f64 = shares.try_into().unwrap_or(0u64) as f64 / 10f64.powi(vault.decimals as i32);
        let underlying_balance = shares_f64 * price_per_share;
        let underlying_balance_raw = U256::from((underlying_balance * 10f64.powi(vault.token.decimals as i32)) as u64);
        
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
            chain_id: vault.chain_id,
            is_migrable: vault.migration.as_ref().map(|m| m.available).unwrap_or(false),
            migration_target: vault.migration.as_ref().map(|m| m.address.clone()),
        }))
    }
    
    async fn calculate_position_value(&self, position: &YearnPosition, cached_data: &CachedYearnData) -> (f64, f64, f64) {
        let balance_f64 = position.balance.try_into().unwrap_or(0u64) as f64 / 10f64.powi(position.token.decimals as i32);
        
        let token_price = match position.token.symbol.to_uppercase().as_str() {
            "WETH" | "ETH" => self.get_token_price("ethereum").await.unwrap_or(4000.0),
            "WBTC" | "BTC" => self.get_token_price("bitcoin").await.unwrap_or(50000.0),
            "USDC" | "USDT" | "DAI" | "FRAX" => 1.0,
            "YFI" => self.get_token_price("yearn-finance").await.unwrap_or(8000.0),
            "CRV" => self.get_token_price("curve-dao-token").await.unwrap_or(1.0),
            "CVX" => self.get_token_price("convex-finance").await.unwrap_or(5.0),
            _ => {
                // Fallback: estimate from vault TVL
                if position.tvl.total_assets_usd > 0.0 {
                    let total_assets_f64 = if let Ok(total_assets_raw) = U256::from_str(&position.tvl.total_assets) {
                        total_assets_raw.try_into().unwrap_or(0u64) as f64 / 10f64.powi(position.token.decimals as i32)
                    } else {
                        position.tvl.tvl
                    };
                    
                    if total_assets_f64 > 0.0 {
                        position.tvl.total_assets_usd / total_assets_f64
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            }
        };
        
        let base_value_usd = balance_f64 * token_price;
        
        let vault_earnings = cached_data.earnings.get(&position.vault_address.to_string().to_lowercase())
            .map(|e| e.earnings)
            .unwrap_or_else(|| {
                let estimated_yearly_earnings = base_value_usd * (position.net_apy / 100.0);
                estimated_yearly_earnings * 0.25 // Assume 3 months average
            });
        
        (base_value_usd, vault_earnings, position.net_apy)
    }
    
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
    
    async fn is_yearn_vault(&self, vault_address: Address) -> Result<bool, AdapterError> {
        let cached_data = self.fetch_all_vaults_data().await?;
        Ok(cached_data.vault_map.contains_key(&vault_address.to_string().to_lowercase()))
    }
}

#[async_trait]
impl DeFiAdapter for YearnAdapter {
    fn protocol_name(&self) -> &'static str {
        "Yearn Finance"
    }

    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        // Check position cache (5-minute cache)
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached_positions) = cache.get(&address) {
                let cache_age = cached_positions.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) {
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
            
            let strategies_desc = if yearn_pos.strategies.is_empty() {
                "Strategy information unavailable".to_string()
            } else {
                yearn_pos.strategies.iter()
                    .take(3)
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
                pnl_usd: 0.0,
                pnl_percentage: 0.0,
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
                    "tvl_usd": yearn_pos.tvl.total_assets_usd,
                    "strategy_count": yearn_pos.strategies.len(),
                    "strategies": strategies_desc,
                    "management_fee": yearn_pos.fees.management,
                    "performance_fee": yearn_pos.fees.performance,
                    "migration_target": yearn_pos.migration_target,
                    "chain_id": yearn_pos.chain_id
                }),
                last_updated: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            };
            
            positions.push(position);
        }
        
        // Cache positions
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
        self.is_yearn_vault(contract_address).await.unwrap_or(false)
    }

    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        Ok(position.value_usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_configurations() {
        let supported_chains = vec![1, 250, 42161, 10, 137];
        
        for chain_id in supported_chains {
            let chain_name = YearnAdapter::get_chain_name(chain_id);
            assert!(!chain_name.is_empty());
            
            if chain_id == 1 || chain_id == 250 || chain_id == 42161 {
                assert!(YearnAdapter::get_registry_address(chain_id).is_some());
            }
        }
    }
}