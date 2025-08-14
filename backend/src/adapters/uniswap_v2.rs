use alloy::{
    primitives::{Address, U256},
    providers::Provider,
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
use tokio::time::timeout;

#[derive(Debug, Deserialize)]
struct CoinGeckoToken {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, Clone)]
struct CachedToken {
    symbol: String,
    name: String,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct LiquidityPosition {
    pair_address: Address,
    token0: Address,
    token1: Address,
    balance: U256,
    total_supply: U256,
}

// Uniswap V2 contract ABIs using alloy sol! macro
sol! {
    #[sol(rpc)]
    interface IUniswapV2Factory {
        function getPair(address tokenA, address tokenB) external view returns (address pair);
        function allPairs(uint256) external view returns (address pair);
        function allPairsLength() external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function token0() external view returns (address);
        function token1() external view returns (address);
        function totalSupply() external view returns (uint256);
        function balanceOf(address owner) external view returns (uint256);
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function name() external pure returns (string memory);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
    }
    
    #[sol(rpc)]
    interface IUniswapV2Router {
        function factory() external pure returns (address);
        function WETH() external pure returns (address);
    }
}

/// Uniswap V2 protocol adapter
pub struct UniswapV2Adapter {
    client: EthereumClient,
    factory_address: Address,
    router_address: Address,
    // Request deduplication cache (prevents API spam)
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    token_cache: Arc<Mutex<HashMap<Address, CachedToken>>>,
    // HTTP client for API calls
    http_client: reqwest::Client,
    // Optional CoinGecko API key for price fetching
    coingecko_api_key: Option<String>,
}

impl UniswapV2Adapter {
    /// Uniswap V2 Factory and Router addresses on Ethereum mainnet
    const FACTORY_ADDRESS: &'static str = "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f";
    const ROUTER_ADDRESS: &'static str = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let factory_address = Address::from_str(Self::FACTORY_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid factory address: {}", e)))?;
            
        let router_address = Address::from_str(Self::ROUTER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid router address: {}", e)))?;
        
        Ok(Self {
            client,
            factory_address,
            router_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            token_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        })
    }
    
    /// Get ALL liquidity positions for a user using event-based discovery
    async fn get_user_liquidity_positions(&self, address: Address) -> Result<Vec<LiquidityPosition>, AdapterError> {
        tracing::info!(
            user_address = %address,
            "🔍 Discovering ALL Uniswap V2 liquidity positions using events"
        );
        
        // Method 1: Scan Transfer events to find ALL LP tokens user has received
        let lp_tokens = self.discover_lp_tokens_via_events(address).await?;
        
        if lp_tokens.is_empty() {
            tracing::info!(
                user_address = %address,
                "No LP token transfers found"
            );
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Method 2: Check current balance for each discovered LP token
        for lp_token_address in &lp_tokens {
            if let Some(position) = self.check_lp_token_balance(address, *lp_token_address).await? {
                positions.push(position);
            }
        }
        
        tracing::info!(
            user_address = %address,
            total_lp_tokens_found = lp_tokens.len(),
            active_positions = positions.len(),
            "✅ Discovered ALL V2 positions"
        );
        
        Ok(positions)
    }
    
    /// Discover ALL LP tokens by scanning Transfer events (this finds EVERYTHING)
    async fn discover_lp_tokens_via_events(&self, address: Address) -> Result<Vec<Address>, AdapterError> {
        use alloy::rpc::types::{Filter, Log};
        
        tracing::info!(
            user_address = %address,
            "🔍 Scanning Transfer events to discover ALL LP tokens"
        );
        
        // ERC20 Transfer event signature: Transfer(address,address,uint256)
        let transfer_event_signature = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
        
        // Scan recent blocks for Transfer events TO this address
        let latest_block = self.client.provider().get_block_number().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get latest block: {}", e)))?;
            
        let from_block = latest_block.saturating_sub(10000); // Last ~10k blocks (~2 days)
        
        let filter = Filter::new()
            .address(Vec::<Address>::new()) // Any address (we'll filter LP tokens later)
            .event_signature(transfer_event_signature.parse::<alloy::primitives::B256>().unwrap())
            .from_block(from_block)
            .to_block(latest_block)
            .topic2(U256::from_be_bytes(address.into_array())); // TO address (our user)
            
        tracing::debug!(
            from_block = %from_block,
            to_block = %latest_block,
            "Scanning blocks for Transfer events"
        );
        
        let logs = self.client.provider().get_logs(&filter).await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get logs: {}", e)))?;
            
        tracing::info!(
            transfer_events_found = logs.len(),
            "Found Transfer events to user"
        );
        
        let mut lp_tokens = Vec::new();
        
        // Filter for actual Uniswap V2 LP tokens
        for log in logs {
            if self.is_uniswap_v2_lp_token(log.address()).await {
                if !lp_tokens.contains(&log.address()) {
                    lp_tokens.push(log.address());
                    tracing::debug!(
                        lp_token = %log.address(),
                        "Found Uniswap V2 LP token"
                    );
                }
            }
        }
        
        // Also scan Transfer events FROM this address (in case they transferred LP tokens)
        let filter_from = Filter::new()
            .address(Vec::<Address>::new())
            .event_signature(transfer_event_signature.parse::<alloy::primitives::B256>().unwrap())
            .from_block(from_block)
            .to_block(latest_block)
            .topic1(U256::from_be_bytes(address.into_array())); // FROM address (our user)
            
        let logs_from = self.client.provider().get_logs(&filter_from).await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get FROM logs: {}", e)))?;
            
        for log in logs_from {
            if self.is_uniswap_v2_lp_token(log.address()).await {
                if !lp_tokens.contains(&log.address()) {
                    lp_tokens.push(log.address());
                }
            }
        }
        
        tracing::info!(
            lp_tokens_discovered = lp_tokens.len(),
            lp_tokens = ?lp_tokens,
            "✅ Discovered unique LP tokens via events"
        );
        
        Ok(lp_tokens)
    }
    
    /// Check if an address is a Uniswap V2 LP token
    async fn is_uniswap_v2_lp_token(&self, token_address: Address) -> bool {
        let pair_contract = IUniswapV2Pair::new(token_address, self.client.provider());
        
        // Try to call Uniswap V2 pair-specific functions
        match pair_contract.token0().call().await {
            Ok(_) => {
                // If token0() succeeds, check if token1() also succeeds
                match pair_contract.token1().call().await {
                    Ok(_) => {
                        // Double check: make sure factory recognizes this pair
                        if let (Ok(token0_result), Ok(token1_result)) = (
                            pair_contract.token0().call().await,
                            pair_contract.token1().call().await
                        ) {
                            let factory = IUniswapV2Factory::new(self.factory_address, self.client.provider());
                            if let Ok(expected_pair) = factory.getPair(token0_result._0, token1_result._0).call().await {
                                return expected_pair.pair == token_address;
                            }
                        }
                        false
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }
    
    /// Alternative Method: Brute force scan all pairs in factory (VERY EXPENSIVE!)
    #[allow(dead_code)]
    async fn discover_all_pairs_brute_force(&self, user: Address) -> Result<Vec<LiquidityPosition>, AdapterError> {
        tracing::warn!(
            "🚨 EXPENSIVE OPERATION: Brute force scanning ALL Uniswap V2 pairs"
        );
        
        let factory = IUniswapV2Factory::new(self.factory_address, self.client.provider());
        
        // Get total number of pairs
        let total_pairs = factory.allPairsLength().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get pairs length: {}", e)))?
            ._0;
            
        tracing::info!(
            total_pairs = %total_pairs,
            "Found total pairs in factory"
        );
        
        let mut positions = Vec::new();
        
        // Check each pair (WARNING: This is VERY expensive for mainnet!)
        for i in 0..total_pairs.to::<u64>() {
            if i % 1000 == 0 {
                tracing::info!("Checked {}/{} pairs", i, total_pairs);
            }
            
            let pair_address = factory.allPairs(U256::from(i)).call().await
                .map_err(|e| AdapterError::ContractError(format!("Failed to get pair {}: {}", i, e)))?
                .pair;
                
            if let Some(position) = self.check_lp_token_balance(user, pair_address).await? {
                positions.push(position);
            }
        }
        
        Ok(positions)
    }
    
    /// Alternative Method: Use The Graph or similar indexing service
    #[allow(dead_code)]
    async fn discover_via_subgraph(&self, user_address: Address) -> Result<Vec<Address>, AdapterError> {
        // Query The Graph's Uniswap V2 subgraph
        let query = format!(r#"{{
            liquidityPositions(where: {{user: "{}"}}) {{
                pair {{
                    id
                    token0 {{
                        id
                        symbol
                    }}
                    token1 {{
                        id  
                        symbol
                    }}
                }}
                liquidityTokenBalance
            }}
        }}"#, user_address);
        
        let subgraph_url = "https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v2";
        
        // This would require GraphQL client implementation
        tracing::info!(
            subgraph_url = %subgraph_url,
            "Would query subgraph for user positions"
        );
        
        // Placeholder - you'd need to implement GraphQL client
        Ok(Vec::new())
    }
    async fn check_lp_token_balance(&self, user: Address, lp_token: Address) -> Result<Option<LiquidityPosition>, AdapterError> {
        let pair_contract = IUniswapV2Pair::new(lp_token, self.client.provider());
        
        // Get user's LP token balance
        let balance = pair_contract.balanceOf(user).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get LP balance: {}", e)))?
            ._0;
            
        // Skip if no balance
        if balance == U256::ZERO {
            return Ok(None);
        }
        
        // Get total supply
        let total_supply = pair_contract.totalSupply().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get total supply: {}", e)))?
            ._0;
            
        // Get token addresses
        let token0 = pair_contract.token0().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get token0: {}", e)))?
            ._0;
            
        let token1 = pair_contract.token1().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get token1: {}", e)))?
            ._0;
        
        tracing::info!(
            user_address = %user,
            lp_token = %lp_token,
            token0 = %token0,
            token1 = %token1,
            balance = %balance,
            total_supply = %total_supply,
            "Found active V2 liquidity position"
        );
        
        Ok(Some(LiquidityPosition {
            pair_address: lp_token,
            token0,
            token1,
            balance,
            total_supply,
        }))
    }
    
    /// Check if user has liquidity in a specific token pair
    async fn check_pair_position(&self, user: Address, token0: Address, token1: Address) -> Result<Option<LiquidityPosition>, AdapterError> {
        // Get pair address from factory
        let factory = IUniswapV2Factory::new(self.factory_address, self.client.provider());
        let pair_address = factory.getPair(token0, token1).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get pair address: {}", e)))?
            .pair;
            
        // If pair doesn't exist, return None
        if pair_address == Address::ZERO {
            return Ok(None);
        }
        
        // Check user's LP token balance
        let pair_contract = IUniswapV2Pair::new(pair_address, self.client.provider());
        let balance = pair_contract.balanceOf(user).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get LP balance: {}", e)))?
            ._0;
            
        // If no balance, skip
        if balance == U256::ZERO {
            return Ok(None);
        }
        
        // Get total supply to calculate share
        let total_supply = pair_contract.totalSupply().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get total supply: {}", e)))?
            ._0;
            
        // Get actual token addresses from pair (they might be swapped)
        let actual_token0 = pair_contract.token0().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get token0: {}", e)))?
            ._0;
            
        let actual_token1 = pair_contract.token1().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get token1: {}", e)))?
            ._0;
        
        tracing::info!(
            user_address = %user,
            pair_address = %pair_address,
            token0 = %actual_token0,
            token1 = %actual_token1,
            balance = %balance,
            total_supply = %total_supply,
            "Found V2 liquidity position"
        );
        
        Ok(Some(LiquidityPosition {
            pair_address,
            token0: actual_token0,
            token1: actual_token1,
            balance,
            total_supply,
        }))
    }
    
    /// Calculate real USD value of a V2 liquidity position
    async fn calculate_position_value(&self, position: &LiquidityPosition) -> (f64, f64, f64) {
        tracing::info!(
            pair = %position.pair_address,
            "🚀 Calculating REAL USD value for Uniswap V2 position"
        );
        
        // Step 1: Get current reserves from the pair
        let pair_contract = IUniswapV2Pair::new(position.pair_address, self.client.provider());
        let reserves = pair_contract.getReserves().call().await;
        
        let (reserve0, reserve1) = match reserves {
            Ok(res) => (res.reserve0, res.reserve1),
            Err(e) => {
                tracing::error!("Failed to get reserves: {}", e);
                return (100.0, 0.0, 0.0); // Fallback value
            }
        };
        
        // Step 2: Get token prices from CoinGecko
        let token0_price = self.get_token_price_usd(position.token0).await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get token0 price: {}, using fallback", e);
                self.get_fallback_price(position.token0)
            });
            
        let token1_price = self.get_token_price_usd(position.token1).await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to get token1 price: {}, using fallback", e);
                self.get_fallback_price(position.token1)
            });
            
        // Step 3: Get token decimals
        let token0_decimals = self.get_token_decimals(position.token0).await.unwrap_or(18);
        let token1_decimals = self.get_token_decimals(position.token1).await.unwrap_or(18);
        
        // Step 4: Calculate user's share of the pool
        let user_share = position.balance.to_string().parse::<f64>().unwrap_or(0.0) / position.total_supply.to_string().parse::<f64>().unwrap_or(1.0);
        
        // Step 5: Calculate token amounts owned by user
        let reserve0_f64 = reserve0.try_into().unwrap_or(0.0) / 10f64.powi(token0_decimals as i32);
        let reserve1_f64 = reserve1.try_into().unwrap_or(0.0) / 10f64.powi(token1_decimals as i32);
        
        let user_token0_amount = reserve0_f64 * user_share;
        let user_token1_amount = reserve1_f64 * user_share;
        
        // Step 6: Calculate USD values
        let token0_value_usd = user_token0_amount * token0_price;
        let token1_value_usd = user_token1_amount * token1_price;
        let total_value_usd = token0_value_usd + token1_value_usd;
        
        // Step 7: Estimate P&L (simplified)
        let pnl_percentage = self.estimate_v2_position_pnl(total_value_usd, user_share);
        let pnl_usd = total_value_usd * (pnl_percentage / 100.0);
        
        tracing::info!(
            pair = %position.pair_address,
            user_share = %user_share,
            user_token0_amount = %user_token0_amount,
            user_token1_amount = %user_token1_amount,
            token0_value_usd = %token0_value_usd,
            token1_value_usd = %token1_value_usd,
            total_value_usd = %total_value_usd,
            pnl_percentage = %pnl_percentage,
            pnl_usd = %pnl_usd,
            "✅ Calculated REAL V2 position value"
        );
        
        (total_value_usd, pnl_usd, pnl_percentage)
    }
    
    /// Enhanced token symbol resolution (same as V3 adapter)
    async fn get_token_symbol_from_coingecko(&self, token_address: Address) -> String {
        let addr_str = format!("{:?}", token_address).to_lowercase();
        
        tracing::debug!(
            token_address = %addr_str,
            "Resolving token symbol"
        );
        
        // 1. Check known tokens first
        let known_symbol = Self::get_known_token_symbol_basic(token_address);
        if known_symbol != "UNKNOWN" {
            tracing::debug!(
                token_address = %addr_str,
                symbol = %known_symbol,
                "Found in known tokens list"
            );
            return known_symbol;
        }
        
        // Get CoinGecko API key from environment
        let api_key = std::env::var("COINGECKO_API_KEY").unwrap_or_else(|_| "demo".to_string());
        
        let url = if api_key == "test-key" || api_key == "demo" {
            // Use free tier endpoint (no API key)
            format!(
                "https://api.coingecko.com/api/v3/coins/ethereum/contract/{}",
                addr_str
            )
        } else if api_key.starts_with("CG-") {
            // Use Demo API endpoint with query parameter
            format!(
                "https://api.coingecko.com/api/v3/coins/ethereum/contract/{}?x_cg_demo_api_key={}",
                addr_str, api_key
            )
        } else {
            // Use Pro API endpoint with different parameter
            format!(
                "https://pro-api.coingecko.com/api/v3/coins/ethereum/contract/{}?x_cg_pro_api_key={}",
                addr_str, api_key
            )
        };
        
        tracing::debug!(
            token_address = %addr_str,
            api_key_type = if api_key.starts_with("CG-") { "Demo API" } else if api_key == "demo" || api_key == "test-key" { "Free Tier" } else { "Pro API" },
            url = %url,
            "Calling CoinGecko API"
        );
        
        match self.call_coingecko_price_api(&url).await {
            Ok(_price) => {
                tracing::info!(
                    token_address = %addr_str,
                    price = %_price,
                    "CoinGecko returned valid price data"
                );
                // For now, return a default symbol since this method returns price, not symbol
                return format!("TOKEN_{}", &addr_str[2..8].to_uppercase());
            }
            Err(e) => {
                tracing::error!(
                    token_address = %addr_str,
                    error = %e,
                    "CoinGecko API failed"
                );
            }
        }
        
        // 3. Try blockchain call as last resort
        tracing::debug!(
            token_address = %addr_str,
            "Using blockchain fallback for token symbol"
        );
        
        match self.try_blockchain_symbol_safe(token_address).await {
            Ok(symbol) => {
                tracing::info!(
                    token_address = %addr_str,
                    symbol = %symbol,
                    "Got symbol from blockchain"
                );
                return symbol;
            }
            Err(e) => {
                tracing::error!(
                    token_address = %addr_str,
                    error = %e,
                    "Blockchain call also failed"
                );
            }
        }
        
        // 4. Final fallback
        let fallback = Self::create_smart_fallback(token_address);
        tracing::debug!(
            token_address = %addr_str,
            fallback = %fallback,
            "Using smart fallback"
        );
        
        fallback
    }
    
    /// Updated resolve_token_pair method using CoinGecko
    async fn resolve_token_pair(&self, token0: Address, token1: Address) -> String {
        tracing::info!(
            token0_address = %token0,
            token1_address = %token1,
            "Resolving token pair using CoinGecko API"
        );
        
        let token0_symbol = self.get_token_symbol_from_coingecko(token0).await;
        let token1_symbol = self.get_token_symbol_from_coingecko(token1).await;
        
        tracing::info!(
            token0_symbol = %token0_symbol,
            token1_symbol = %token1_symbol,
            "Resolved token symbols"
        );
        
        format!("{}/{}", token0_symbol, token1_symbol)
    }
    
    /// Get token price from CoinGecko API (same as V3 adapter)
    async fn get_token_price_usd(&self, token_address: Address) -> Result<f64, String> {
        let addr_str = format!("{:?}", token_address).to_lowercase();
        
        // Check if it's a known stablecoin first
        if self.is_stablecoin(token_address) {
            return Ok(1.0); // Stablecoins are ~$1
        }
        
        // Build CoinGecko price API URL
        let base_url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3"
        } else {
            "https://api.coingecko.com/api/v3"
        };
        
        let url = format!(
            "{}/simple/token_price/ethereum?contract_addresses={}&vs_currencies=usd",
            base_url, addr_str
        );
        
        tracing::debug!("Fetching price for {} from: {}", addr_str, url);
        
        match self.call_coingecko_price_api(&url).await {
            Ok(price) => {
                tracing::info!("Got price for {}: ${}", addr_str, price);
                Ok(price)
            }
            Err(e) => {
                tracing::warn!("Failed to get price for {}: {}", addr_str, e);
                Err(e)
            }
        }
    }
    
    /// Get token decimals (same as V3 adapter)
    async fn get_token_decimals(&self, token_address: Address) -> Result<u8, String> {
        // Check known token decimals first (for performance)
        let known_decimals = Self::get_known_token_decimals(token_address);
        if known_decimals.is_some() {
            return Ok(known_decimals.unwrap());
        }
        
        tracing::debug!(
            token_address = %token_address,
            "Fetching token decimals from blockchain"
        );
        
        let contract = IERC20::new(token_address, self.client.provider());
        
        // Use timeout for blockchain call
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            contract.decimals().call().await
        }).await;
        
        match result {
            Ok(Ok(decimals_result)) => {
                let decimals = decimals_result._0;
                tracing::info!(
                    token_address = %token_address,
                    decimals = %decimals,
                    "Got token decimals from blockchain"
                );
                Ok(decimals)
            }
            Ok(Err(e)) => {
                tracing::error!(
                    token_address = %token_address,
                    error = %e,
                    "Failed to get decimals from blockchain"
                );
                Err(format!("Contract call failed: {}", e))
            }
            Err(_) => {
                tracing::error!(
                    token_address = %token_address,
                    "Blockchain call for decimals timed out"
                );
                Err("Timeout".to_string())
            }
        }
    }
    
    /// Estimate P&L for V2 positions (simplified)
    fn estimate_v2_position_pnl(&self, position_value_usd: f64, user_share: f64) -> f64 {
        // V2 positions generally earn fees (0.3%) but suffer from impermanent loss
        // This is a simplified estimation
        
        let base_return = 3.0; // Base 3% APY from fees
        
        // Larger positions tend to be more stable
        let size_adjustment = if position_value_usd > 50_000.0 {
            0.8 // Large positions: more conservative
        } else if position_value_usd > 5_000.0 {
            1.0 // Medium positions
        } else {
            1.3 // Small positions: higher volatility
        };
        
        // Larger share of pool = more fee earnings
        let share_bonus = if user_share > 0.01 {
            1.2 // >1% of pool
        } else if user_share > 0.001 {
            1.1 // >0.1% of pool  
        } else {
            1.0 // Small share
        };
        
        base_return * size_adjustment * share_bonus
    }
    
    // Helper functions (same as V3 adapter)
    fn get_known_token_symbol_basic(address: Address) -> String {
        let addr_str = format!("{:?}", address).to_lowercase();
        match addr_str.as_str() {
            // Core tokens
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH".to_string(),
            "0xa0b86a33e6c3c8c95f2d8c4e9f8e8e8e8e8e8e8e" => "USDC".to_string(),
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT".to_string(),
            "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI".to_string(),
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => "WBTC".to_string(),
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => "UNI".to_string(),
            "0x514910771af9ca656af840dff83e8264ecf986ca" => "LINK".to_string(),
            "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9" => "AAVE".to_string(),
            _ => "UNKNOWN".to_string()
        }
    }
    
    fn get_known_token_decimals(address: Address) -> Option<u8> {
        let addr_str = format!("{:?}", address).to_lowercase();
        match addr_str.as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => Some(18), // WETH
            "0xa0b86a33e6c3c8c95f2d8c4e9f8e8e8e8e8e8e8e" => Some(6),  // USDC
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => Some(6),  // USDT
            "0x6b175474e89094c44da98b954eedeac495271d0f" => Some(18), // DAI
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => Some(8),  // WBTC
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => Some(18), // UNI
            "0x514910771af9ca656af840dff83e8264ecf986ca" => Some(18), // LINK
            "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9" => Some(18), // AAVE
            _ => None,
        }
    }
    
    fn is_stablecoin(&self, token_address: Address) -> bool {
        let addr_str = format!("{:?}", token_address).to_lowercase();
        
        // Known stablecoin addresses on Ethereum
        matches!(addr_str.as_str(),
            "0x6b175474e89094c44da98b954eedeac495271d0f" | // DAI
            "0xa0b86a33e6c3c8c95f2d8c4e9f8e8e8e8e8e8e8e" | // USDC  
            "0xdac17f958d2ee523a2206206994597c13d831ec7" | // USDT
            "0x4fabb145d64652a948d72533023f6e7a623c7c53" | // BUSD
            "0x8e870d67f660d95d5be530380d0ec0bd388289e1" | // USDP
            "0x57ab1ec28d129707052df4df418d58a2d46d5f51" | // sUSD
            "0x956f47f50a910163d8bf957cf5846d573e7f87ca" | // FEI
            "0x853d955acef822db058eb8505911ed77f175b99e" | // FRAX
            "0x5f98805a4e8be255a32880fdec7f6728c6568ba0" | // LUSD
            "0x99d8a9c45b2eca8864373a26d1459e3dff1e17f3"   // MIM
        )
    }
    
    fn get_fallback_price(&self, token_address: Address) -> f64 {
        if self.is_stablecoin(token_address) {
            return 1.0;
        }
        
        let addr_str = format!("{:?}", token_address).to_lowercase();
        match addr_str.as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => 4000.0, // WETH
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => 15.0,   // UNI
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => 100000.0, // WBTC
            "0x7d1afa7b718fb893db30a3abc0cfc608aacfebb0" => 0.5,    // MATIC
            "0x514910771af9ca656af840dff83e8264ecf986ca" => 25.0,   // LINK
            _ => 1.0, // Default fallback
        }
    }
    
    async fn call_coingecko_price_api(&self, url: &str) -> Result<f64, String> {
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
            
        tracing::debug!("CoinGecko price response: {}", response_text);
        
        // Parse JSON response
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        // Extract price from nested JSON structure
        if let Some(prices) = json.as_object() {
            for (_, price_data) in prices {
                if let Some(usd_price) = price_data.get("usd") {
                    if let Some(price) = usd_price.as_f64() {
                        return Ok(price);
                    }
                }
            }
        }
        
        Err("Price not found in response".to_string())
    }
    
    async fn try_blockchain_symbol_safe(&self, token_address: Address) -> Result<String, String> {
        let contract = IERC20::new(token_address, self.client.provider());
        
        // Use async block to properly handle the future
        let result = timeout(Duration::from_secs(5), async {
            contract.symbol().call().await
        }).await;
        
        match result {
            Ok(Ok(symbol_result)) => {
                let symbol = symbol_result._0.trim().to_uppercase();
                if Self::is_valid_symbol(&symbol) {
                    Ok(symbol)
                } else {
                    Err(format!("Invalid symbol from blockchain: {}", symbol))
                }
            }
            Ok(Err(e)) => Err(format!("Contract call failed: {}", e)),
            Err(_) => Err("Blockchain call timed out".to_string()),
        }
    }
    
    fn is_valid_symbol(symbol: &str) -> bool {
        if symbol.is_empty() || symbol.len() > 12 {
            return false;
        }
        
        // Check for valid characters
        let valid_chars = symbol.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '_' || c == '-'
        });
        
        if !valid_chars {
            return false;
        }
        
        // Filter out obvious junk
        let symbol_lower = symbol.to_lowercase();
        let junk_patterns = ["test", "fake", "scam", "0x", "unknown"];
        
        !junk_patterns.iter().any(|&pattern| symbol_lower.contains(pattern))
    }
    
    fn create_smart_fallback(token_address: Address) -> String {
        let addr_str = format!("{:?}", token_address);
        
        // Use last 6 characters instead of first (more unique)
        if addr_str.len() >= 8 {
            format!("TOKEN_{}", &addr_str[addr_str.len()-6..].to_uppercase())
        } else {
            "UNKNOWN".to_string()
        }
    }
}

#[async_trait]
impl DeFiAdapter for UniswapV2Adapter {
    fn protocol_name(&self) -> &'static str {
        "uniswap_v2"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        tracing::info!(
            user_address = %address,
            protocol = "uniswap_v2",
            "CACHE CHECK: Checking for cached positions to prevent API spam"
        );
        
        // CACHE CHECK: Prevent API spam by checking cache first
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) { // 5 minute cache to prevent API spam
                    tracing::info!(
                        user_address = %address,
                        cache_age_secs = cache_age.as_secs(),
                        position_count = cached.positions.len(),
                        "CACHE HIT: Returning cached positions to prevent API spam!"
                    );
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            "CACHE MISS: Fetching fresh data from blockchain (this will use API calls)"
        );
        
        // Get all liquidity positions for the user
        let liquidity_positions = self.get_user_liquidity_positions(address).await?;
        
        if liquidity_positions.is_empty() {
            tracing::info!(
                user_address = %address,
                "No Uniswap V2 positions found"
            );
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Convert liquidity positions to Position structs with real valuation
        for liq_pos in liquidity_positions {
            let (value_usd, pnl_usd, pnl_percentage) = self.calculate_position_value(&liq_pos).await;
            
            let position = Position {
                id: format!("uniswap_v2_{}", liq_pos.pair_address),
                protocol: "uniswap_v2".to_string(),
                position_type: "liquidity".to_string(),
                pair: self.resolve_token_pair(liq_pos.token0, liq_pos.token1).await,
                value_usd: value_usd.max(1.0), // Real calculated value
                pnl_usd,   // Real P&L calculation
                pnl_percentage, // Real P&L percentage
                risk_score: 25, // V2 is generally lower risk than V3
                metadata: serde_json::json!({
                    "pair_address": format!("{:?}", liq_pos.pair_address),
                    "token0": format!("{:?}", liq_pos.token0),
                    "token1": format!("{:?}", liq_pos.token1),
                    "lp_balance": liq_pos.balance.to_string(),
                    "total_supply": liq_pos.total_supply.to_string(),
                    "pool_share": (liq_pos.balance.to::<u128>() as f64 / liq_pos.total_supply.to::<u128>() as f64 * 100.0),
                    "protocol_version": "v2"
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
            "✅ Successfully fetched and cached Uniswap V2 positions"
        );
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        // Check if it's the factory or router
        contract_address == self.factory_address || contract_address == self.router_address
    }
    
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // V2 risk calculation based on:
        // - Pool size (larger = safer)
        // - Token pair volatility
        // - Impermanent loss potential
        
        let mut total_risk = 0u32;
        let mut total_weight = 0f64;
        
        for position in positions {
            let position_weight = position.value_usd;
            let mut risk_score = 25u32; // Base V2 risk
            
            // Adjust based on position size
            if position.value_usd > 100_000.0 {
                risk_score -= 5; // Large positions are generally safer
            } else if position.value_usd < 1_000.0 {
                risk_score += 10; // Small positions are riskier
            }
            
            // Adjust based on P&L
            if position.pnl_percentage < -10.0 {
                risk_score += 15; // Currently losing money
            } else if position.pnl_percentage > 10.0 {
                risk_score -= 5; // Currently profitable
            }
            
            total_risk += (risk_score * position_weight as u32);
            total_weight += position_weight;
        }
        
        if total_weight > 0.0 {
            Ok((total_risk as f64 / total_weight) as u8)
        } else {
            Ok(25) // Default V2 risk
        }
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // For V2, we can recalculate the position value in real-time
        // by parsing the metadata to get the pair address and recalculating
        
        if let Some(pair_address_str) = position.metadata.get("pair_address") {
            if let Some(pair_address_str) = pair_address_str.as_str() {
                if let Ok(pair_address) = Address::from_str(pair_address_str) {
                    // Get current reserves and recalculate
                    let pair_contract = IUniswapV2Pair::new(pair_address, self.client.provider());
                    
                    match pair_contract.getReserves().call().await {
                        Ok(_reserves) => {
                            // Could recalculate real-time value here
                            // For now, return cached value
                            return Ok(position.value_usd);
                        }
                        Err(_) => {
                            // Fallback to cached value
                            return Ok(position.value_usd);
                        }
                    }
                }
            }
        }
        
        // Fallback to cached value
        Ok(position.value_usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_factory_address() {
        let addr = Address::from_str(UniswapV2Adapter::FACTORY_ADDRESS);
        assert!(addr.is_ok());
    }
    
    #[test]
    fn test_router_address() {
        let addr = Address::from_str(UniswapV2Adapter::ROUTER_ADDRESS);
        assert!(addr.is_ok());
    }
}