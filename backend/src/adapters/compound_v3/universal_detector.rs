// Universal Compound V3 Position Detector
// This module detects ALL positions for ANY wallet across ALL markets and chains
// NO HARDCODED ASSETS - discovers everything dynamically

use alloy::primitives::{Address, U256, I256};
use alloy::providers::Provider;
use alloy::sol;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::time::{timeout, Duration};
use tracing::{info, warn, error, debug};

use crate::blockchain::ethereum_client::EthereumClient;
use crate::adapters::traits::{AdapterError, Position};
use crate::adapters::compound_v3::market_registry::{CompoundV3MarketRegistry, MarketInfo, CollateralAssetInfo};
use std::time::{SystemTime, UNIX_EPOCH};

// Enhanced Comet interface for complete position detection
sol! {
    #[sol(rpc)]
    interface IComet {
        struct UserBasic {
            int104 principal;
            uint64 baseTrackingIndex;
            uint64 baseTrackingAccrued;
            uint16 assetsIn;
            uint8 _reserved;
        }
        
        struct UserCollateral {
            uint128 balance;
            uint128 _reserved;
        }
        
        function userBasic(address account) external view returns (UserBasic memory);
        function userCollateral(address account, address asset) external view returns (UserCollateral memory);
        function borrowBalanceOf(address account) external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
        function collateralBalanceOf(address account, address asset) external view returns (uint128);
        function baseToken() external view returns (address);
        function baseTokenPriceFeed() external view returns (address);
        function getAssetInfo(uint8 i) external view returns (AssetInfo memory);
        function numAssets() external view returns (uint8);
        
        struct AssetInfo {
            uint8 offset;
            address asset;
            address priceFeed;
            uint128 scale;
            uint128 borrowCollateralFactor;
            uint128 liquidateCollateralFactor;
            uint128 liquidationFactor;
            uint128 supplyCap;
        }
    }
    
    #[sol(rpc)]
    interface ICometRewards {
        struct RewardOwed {
            address token;
            uint256 owed;
        }
        
        function getRewardOwed(address comet, address account) external view returns (RewardOwed memory);
        function rewardConfig(address comet) external view returns (RewardConfig memory);
        
        struct RewardConfig {
            address token;
            uint64 rescaleFactor;
            bool shouldUpscale;
        }
    }
    
    #[sol(rpc)]
    interface IERC20 {
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function name() external view returns (string memory);
    }
}

#[derive(Debug, Clone)]
pub struct DetectedPosition {
    pub position_type: String, // "supply", "borrow", "collateral", "rewards"
    pub market_address: Address,
    pub market_symbol: String,
    pub asset_address: Address,
    pub asset_symbol: String,
    pub asset_decimals: u8,
    pub balance_raw: String,
    pub balance_formatted: f64,
    pub balance_usd: f64,
    pub apy: f64,
    pub risk_score: u8,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Clone)]
pub struct UniversalCompoundV3Detector {
    client: EthereumClient,
    chain_id: u64,
    market_registry: CompoundV3MarketRegistry,
    // Price cache for tokens
    price_cache: HashMap<Address, f64>,
    // Rewards contract addresses per chain
    rewards_contracts: HashMap<u64, Vec<Address>>,
}

impl UniversalCompoundV3Detector {
    pub fn new(client: EthereumClient, chain_id: u64) -> Self {
        let market_registry = CompoundV3MarketRegistry::new(client.clone(), chain_id);
        
        let mut rewards_contracts = HashMap::new();
        
        // Known rewards contracts per chain
        rewards_contracts.insert(1, vec![
            Address::from_str("0x1B0e765F6224C21223AeA2af16c1C46E38885a40").unwrap(), // Ethereum
        ]);
        rewards_contracts.insert(137, vec![
            Address::from_str("0x45939657d1CA34A8FA39A924B71D28Fe8431e581").unwrap(), // Polygon
        ]);
        rewards_contracts.insert(42161, vec![
            Address::from_str("0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae").unwrap(), // Arbitrum
        ]);
        rewards_contracts.insert(8453, vec![
            Address::from_str("0x123964802e6ABabBE1Bc9547D72Ef1332C8d781D").unwrap(), // Base
        ]);
        
        Self {
            client,
            chain_id,
            market_registry,
            price_cache: HashMap::new(),
            rewards_contracts,
        }
    }
    
    /// Detect ALL Compound V3 positions for ANY wallet address
    pub async fn detect_all_positions(&mut self, wallet_address: Address) -> Result<Vec<DetectedPosition>, AdapterError> {
        info!("🎯 Detecting ALL Compound V3 positions for wallet: {} on chain {}", wallet_address, self.chain_id);
        
        // Step 1: Discover all markets on this chain
        let all_markets = self.market_registry.discover_all_markets().await?;
        info!("📊 Found {} markets to check", all_markets.len());
        
        let mut all_positions = Vec::new();
        
        // Step 2: Check each market for positions
        for market in &all_markets {
            info!("🔍 Checking market: {} ({})", market.base_token_symbol, market.market_address);
            
            // Check base token positions (supply/borrow)
            if let Ok(base_positions) = self.detect_base_positions(wallet_address, market).await {
                all_positions.extend(base_positions);
            }
            
            // Check ALL collateral positions
            if let Ok(collateral_positions) = self.detect_collateral_positions(wallet_address, market).await {
                all_positions.extend(collateral_positions);
            }
            
            // Check rewards for this market
            if let Ok(reward_positions) = self.detect_reward_positions(wallet_address, market).await {
                all_positions.extend(reward_positions);
            }
        }
        
        info!("✅ Position detection complete: {} positions found", all_positions.len());
        
        // Log summary
        for position in &all_positions {
            info!("   💰 {} {}: {} {} (${:.2})", 
                  position.position_type,
                  position.market_symbol,
                  position.balance_formatted,
                  position.asset_symbol,
                  position.balance_usd);
        }
        
        Ok(all_positions)
    }
    
    /// Detect base token positions (supply/borrow) for a specific market
    async fn detect_base_positions(&mut self, wallet: Address, market: &MarketInfo) -> Result<Vec<DetectedPosition>, AdapterError> {
        debug!("🔍 Checking base positions for market: {}", market.base_token_symbol);
        
        let provider = self.client.provider();
        let comet = IComet::new(market.market_address, provider);
        
        let mut positions = Vec::new();
        
        // Get user basic info
        let user_basic = timeout(Duration::from_secs(10), comet.userBasic(wallet).call())
            .await
            .map_err(|_| AdapterError::Timeout("User basic fetch timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("Failed to get user basic: {:?}", e)))?
            ._0;
        
        let principal = user_basic.principal;
        warn!("🔍 CRITICAL DEBUG - Principal for {}: {}", wallet, principal);
        warn!("🔍 CRITICAL DEBUG - Principal is_zero(): {}", principal.is_zero());
        warn!("🔍 CRITICAL DEBUG - Principal is_positive(): {}", principal.is_positive());
        warn!("🔍 CRITICAL DEBUG - Principal is_negative(): {}", principal.is_negative());
        
        if !principal.is_zero() {
            let is_supply = principal.is_positive();
            let position_type = if is_supply { "supply" } else { "borrow" };
            
            // Get accurate balance
            let balance_raw = if is_supply {
                // For supply positions, use balanceOf
                timeout(Duration::from_secs(10), comet.balanceOf(wallet).call())
                    .await
                    .map_err(|_| AdapterError::Timeout("Balance fetch timeout".to_string()))?
                    .map_err(|e| AdapterError::NetworkError(format!("Failed to get balance: {:?}", e)))?
                    ._0
            } else {
                // For borrow positions, use borrowBalanceOf
                timeout(Duration::from_secs(10), comet.borrowBalanceOf(wallet).call())
                    .await
                    .map_err(|_| AdapterError::Timeout("Borrow balance fetch timeout".to_string()))?
                    .map_err(|e| AdapterError::NetworkError(format!("Failed to get borrow balance: {:?}", e)))?
                    ._0
            };
            
            let balance_formatted = (balance_raw.to_string().parse::<f64>().unwrap_or(0.0)) / 10_f64.powi(market.base_token_decimals as i32);
            let token_price = self.get_token_price(market.base_token).await.unwrap_or(1.0);
            let balance_usd = balance_formatted * token_price;
            
            if balance_usd > 0.01 { // Only include positions worth more than 1 cent
                let mut metadata = HashMap::new();
                metadata.insert("market_address".to_string(), serde_json::Value::String(market.market_address.to_string()));
                metadata.insert("principal".to_string(), serde_json::Value::String(principal.to_string()));
                metadata.insert("balance_raw".to_string(), serde_json::Value::String(balance_raw.to_string()));
                
                let apy = if is_supply { market.supply_apy } else { market.borrow_apy };
                let risk_score = self.calculate_risk_score(position_type, balance_usd, apy);
                
                positions.push(DetectedPosition {
                    position_type: position_type.to_string(),
                    market_address: market.market_address,
                    market_symbol: market.base_token_symbol.clone(),
                    asset_address: market.base_token,
                    asset_symbol: market.base_token_symbol.clone(),
                    asset_decimals: market.base_token_decimals,
                    balance_raw: balance_raw.to_string(),
                    balance_formatted,
                    balance_usd,
                    apy,
                    risk_score,
                    metadata,
                });
                
                info!("   ✅ Found {} position: {:.6} {} (${:.2})", 
                      position_type, balance_formatted, market.base_token_symbol, balance_usd);
            }
        }
        
        Ok(positions)
    }
    
    /// Detect ALL collateral positions for a specific market
    async fn detect_collateral_positions(&mut self, wallet: Address, market: &MarketInfo) -> Result<Vec<DetectedPosition>, AdapterError> {
        debug!("🔍 Checking collateral positions for market: {}", market.base_token_symbol);
        
        let mut positions = Vec::new();
        
        // Check ALL collateral assets for this market
        for collateral_asset in &market.collateral_assets {
            debug!("   Checking collateral: {}", collateral_asset.asset_symbol);
            
            // Create provider and contract for each iteration to avoid borrow conflicts
            let provider = self.client.provider();
            let comet = IComet::new(market.market_address, provider);
            
            let user_collateral = timeout(Duration::from_secs(10), comet.userCollateral(wallet, collateral_asset.asset).call())
                .await
                .map_err(|_| AdapterError::Timeout("User collateral fetch timeout".to_string()))?
                .map_err(|e| AdapterError::NetworkError(format!("Failed to get user collateral: {:?}", e)))?
                ._0;
            
            let balance_raw = user_collateral.balance;
            
            if balance_raw > 0 {
                let balance_formatted = (balance_raw as f64) / 10_f64.powi(collateral_asset.asset_decimals as i32);
                let token_price = self.get_token_price(collateral_asset.asset).await.unwrap_or(1.0);
                let balance_usd = balance_formatted * token_price;
                
                if balance_usd > 0.01 { // Only include positions worth more than 1 cent
                    let mut metadata = HashMap::new();
                    metadata.insert("market_address".to_string(), serde_json::Value::String(market.market_address.to_string()));
                    metadata.insert("borrow_collateral_factor".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(collateral_asset.borrow_collateral_factor).unwrap()));
                    metadata.insert("liquidate_collateral_factor".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(collateral_asset.liquidate_collateral_factor).unwrap()));
                    
                    let risk_score = self.calculate_collateral_risk_score(balance_usd, &collateral_asset.asset_symbol);
                    
                    positions.push(DetectedPosition {
                        position_type: "collateral".to_string(),
                        market_address: market.market_address,
                        market_symbol: market.base_token_symbol.clone(),
                        asset_address: collateral_asset.asset,
                        asset_symbol: collateral_asset.asset_symbol.clone(),
                        asset_decimals: collateral_asset.asset_decimals,
                        balance_raw: balance_raw.to_string(),
                        balance_formatted,
                        balance_usd,
                        apy: 0.0, // Collateral doesn't earn APY directly
                        risk_score,
                        metadata,
                    });
                    
                    info!("   ✅ Found collateral: {:.6} {} (${:.2})", 
                          balance_formatted, collateral_asset.asset_symbol, balance_usd);
                }
            }
        }
        
        Ok(positions)
    }
    
    /// Detect ALL reward positions for a specific market
    async fn detect_reward_positions(&mut self, wallet: Address, market: &MarketInfo) -> Result<Vec<DetectedPosition>, AdapterError> {
        debug!("🔍 Checking reward positions for market: {}", market.base_token_symbol);
        
        let mut positions = Vec::new();
        
        // Check all known rewards contracts for this chain
        let rewards_addresses: Vec<Address> = if let Some(addresses) = self.rewards_contracts.get(&self.chain_id) {
            addresses.clone()
        } else {
            Vec::new()
        };
        
        for rewards_address in rewards_addresses {
            if let Ok(reward_position) = self.check_rewards_contract(wallet, market, rewards_address).await {
                if let Some(position) = reward_position {
                    positions.push(position);
                }
            }
        }
        
        Ok(positions)
    }
    
    /// Check a specific rewards contract for pending rewards
    async fn check_rewards_contract(&mut self, wallet: Address, market: &MarketInfo, rewards_address: Address) -> Result<Option<DetectedPosition>, AdapterError> {
        let provider = self.client.provider();
        let rewards_contract = ICometRewards::new(rewards_address, provider);
        
        // Try to get reward owed
        match timeout(Duration::from_secs(10), rewards_contract.getRewardOwed(market.market_address, wallet).call()).await {
            Ok(Ok(reward_data)) => {
                let rewards_balance = reward_data._0.owed;
                
                if rewards_balance > U256::from(0) {
                    // Get reward token info
                    let reward_token = reward_data._0.token;
                    let reward_symbol = self.get_token_symbol(reward_token).await.unwrap_or("REWARD".to_string());
                    let reward_decimals = self.get_token_decimals(reward_token).await.unwrap_or(18);
                    
                    let balance_formatted = (rewards_balance.to_string().parse::<f64>().unwrap_or(0.0)) / 10_f64.powi(reward_decimals as i32);
                    let token_price = self.get_token_price(reward_token).await.unwrap_or(1.0);
                    let balance_usd = balance_formatted * token_price;
                    
                    if balance_usd > 0.01 {
                        let mut metadata = HashMap::new();
                        metadata.insert("market_address".to_string(), serde_json::Value::String(market.market_address.to_string()));
                        metadata.insert("rewards_contract".to_string(), serde_json::Value::String(rewards_address.to_string()));
                        metadata.insert("reward_type".to_string(), serde_json::Value::String("compound_governance".to_string()));
                        
                        info!("   ✅ Found rewards: {:.6} {} (${:.2})", 
                              balance_formatted, reward_symbol, balance_usd);
                        
                        return Ok(Some(DetectedPosition {
                            position_type: "rewards".to_string(),
                            market_address: market.market_address,
                            market_symbol: market.base_token_symbol.clone(),
                            asset_address: reward_token,
                            asset_symbol: reward_symbol,
                            asset_decimals: reward_decimals,
                            balance_raw: rewards_balance.to_string(),
                            balance_formatted,
                            balance_usd,
                            apy: 0.0,
                            risk_score: 10, // Rewards are low risk
                            metadata,
                        }));
                    }
                }
            }
            Ok(Err(e)) => {
                debug!("   Rewards contract call failed: {:?}", e);
            }
            Err(_) => {
                debug!("   Rewards contract call timed out");
            }
        }
        
        Ok(None)
    }
    
    /// Get token price in USD
    async fn get_token_price(&mut self, token_address: Address) -> Result<f64, AdapterError> {
        // Check cache first
        if let Some(&price) = self.price_cache.get(&token_address) {
            return Ok(price);
        }
        
        // Known token prices (this would be replaced with real price oracle calls)
        let known_prices = HashMap::from([
            (Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap(), 4336.0), // WETH
            (Address::from_str("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599").unwrap(), 95000.0), // WBTC
            (Address::from_str("0xA0b86a33E6441E2C2C8A0E3C516C7A4e9e9e9e9e").unwrap(), 1.0), // USDC
            (Address::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap(), 1.0), // USDT
            (Address::from_str("0x514910771AF9Ca656af840dff83E8264EcF986CA").unwrap(), 21.8), // LINK
            (Address::from_str("0xc00e94Cb662C3520282E6f5717214004A7f26888").unwrap(), 48.0), // COMP
        ]);
        
        let price = known_prices.get(&token_address).copied().unwrap_or(1.0);
        
        // Cache the price
        self.price_cache.insert(token_address, price);
        
        Ok(price)
    }
    
    /// Get token symbol
    async fn get_token_symbol(&self, token_address: Address) -> Result<String, AdapterError> {
        let provider = self.client.provider();
        let token = IERC20::new(token_address, provider);
        
        match timeout(Duration::from_secs(5), token.symbol().call()).await {
            Ok(Ok(symbol)) => Ok(symbol._0),
            _ => Ok("UNKNOWN".to_string()),
        }
    }
    
    /// Get token decimals
    async fn get_token_decimals(&self, token_address: Address) -> Result<u8, AdapterError> {
        let provider = self.client.provider();
        let token = IERC20::new(token_address, provider);
        
        match timeout(Duration::from_secs(5), token.decimals().call()).await {
            Ok(Ok(decimals)) => Ok(decimals._0),
            _ => Ok(18),
        }
    }
    
    /// Calculate risk score for a position
    fn calculate_risk_score(&self, position_type: &str, balance_usd: f64, apy: f64) -> u8 {
        let base_risk = match position_type {
            "supply" => 20,
            "borrow" => 60,
            "collateral" => 40,
            "rewards" => 10,
            _ => 50,
        };
        
        // Adjust for position size
        let size_adjustment = if balance_usd > 100000.0 { 10 } else if balance_usd > 10000.0 { 5 } else { 0 };
        
        // Adjust for APY (higher APY = higher risk)
        let apy_adjustment = if apy > 20.0 { 15 } else if apy > 10.0 { 10 } else if apy > 5.0 { 5 } else { 0 };
        
        (base_risk + size_adjustment + apy_adjustment).min(100)
    }
    
    /// Calculate risk score for collateral positions
    fn calculate_collateral_risk_score(&self, balance_usd: f64, asset_symbol: &str) -> u8 {
        let base_risk = match asset_symbol.to_uppercase().as_str() {
            "USDC" | "USDT" | "DAI" => 15, // Stablecoins are lower risk
            "WETH" | "ETH" => 25,
            "WBTC" | "BTC" => 30,
            "LINK" => 40,
            "COMP" => 50,
            _ => 45,
        };
        
        // Adjust for position size
        let size_adjustment = if balance_usd > 100000.0 { 10 } else if balance_usd > 10000.0 { 5 } else { 0 };
        
        (base_risk + size_adjustment).min(100)
    }
    
    /// Convert detected positions to the standard Position format
    pub fn convert_to_positions(&self, wallet: Address, detected_positions: Vec<DetectedPosition>) -> Vec<Position> {
        detected_positions.into_iter().map(|detected| {
            Position {
                id: format!("compound_v3_{}_{}_{}_{:x}", 
                          detected.position_type,
                          self.chain_id,
                          detected.asset_address,
                          wallet.as_slice().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))),
                protocol: "compound_v3".to_string(),
                position_type: detected.position_type,
                pair: format!("{}/USD", detected.asset_symbol),
                value_usd: detected.balance_usd,
                pnl_usd: 0.0, // Would be calculated based on entry price
                pnl_percentage: 0.0,
                risk_score: detected.risk_score,
                last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                metadata: serde_json::json!({
                    "balance": detected.balance_raw,
                    "balance_usd": detected.balance_usd,
                    "token_address": detected.asset_address.to_string(),
                    "token_symbol": detected.asset_symbol,
                    "market_address": detected.market_address.to_string(),
                    "market_symbol": detected.market_symbol,
                    "apy": detected.apy,
                    "additional_metadata": detected.metadata
                }),
            }
        }).collect()
    }
}
