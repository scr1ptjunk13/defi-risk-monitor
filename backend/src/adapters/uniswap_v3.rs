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
// Removed unused import: use tokio::time::timeout;

// Placeholder EthereumClient
#[derive(Debug, Clone)]
pub struct EthereumClient {
    pub rpc_url: String,
}

impl EthereumClient {
    pub fn provider(&self) -> &str {
        &self.rpc_url
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct CoinGeckoToken {
    id: String,
    symbol: String,
    name: String,
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

// Uniswap V3 contract interfaces
sol! {
    #[sol(rpc)]
    interface INonfungiblePositionManager {
        struct Position {
            uint96 nonce;
            address operator;
            address token0;
            address token1;
            uint24 fee;
            int24 tickLower;
            int24 tickUpper;
            uint128 liquidity;
            uint256 feeGrowthInside0LastX128;
            uint256 feeGrowthInside1LastX128;
            uint128 tokensOwed0;
            uint128 tokensOwed1;
        }
        
        function positions(uint256 tokenId) external view returns (Position memory);
        function balanceOf(address owner) external view returns (uint256);
        function tokenOfOwnerByIndex(address owner, uint256 index) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
        
        function liquidity() external view returns (uint128);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
    }
}

pub struct UniswapV3Adapter {
    #[allow(dead_code)]
    client: EthereumClient,
    position_manager_address: Address,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    #[allow(dead_code)]
    http_client: reqwest::Client,
    #[allow(dead_code)]
    coingecko_api_key: Option<String>,
}

#[allow(dead_code)]
impl UniswapV3Adapter {
    const POSITION_MANAGER_ADDRESS: &'static str = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
    const CACHE_DURATION: Duration = Duration::from_secs(300); // 5 minutes
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let position_manager_address = Address::from_str(Self::POSITION_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid position manager address: {}", e)))?;
        
        Ok(Self {
            client,
            position_manager_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        })
    }
    
    async fn get_user_token_ids(&self, _address: Address) -> Result<Vec<U256>, AdapterError> {
        // TODO: Implement NFT balance and token ID fetching
        Ok(vec![])
    }
    
    async fn get_position_details(&self, _token_id: U256) -> Result<Option<Position>, AdapterError> {
        // TODO: Implement position details fetching from contract
        Ok(None)
    }
    
    async fn resolve_token_pair(&self, token0: Address, token1: Address) -> String {
        let token0_symbol = self.get_token_symbol(token0).await;
        let token1_symbol = self.get_token_symbol(token1).await;
        format!("{}/{}", token0_symbol, token1_symbol)
    }
    
    async fn get_token_symbol(&self, token_address: Address) -> String {
        let known_symbol = Self::get_known_token_symbol(token_address);
        if known_symbol != "UNKNOWN" {
            return known_symbol;
        }
        
        // Try CoinGecko API
        if let Ok((symbol, _)) = self.call_coingecko_api(token_address).await {
            return symbol;
        }
        
        // Try blockchain call
        if let Ok(symbol) = self.try_blockchain_symbol(token_address).await {
            return symbol;
        }
        
        Self::create_fallback_symbol(token_address)
    }
    
    async fn call_coingecko_api(&self, token_address: Address) -> Result<(String, String), String> {
        let addr_str = format!("{:?}", token_address).to_lowercase();
        
        let api_key = std::env::var("COINGECKO_API_KEY").unwrap_or_else(|_| "demo".to_string());
        
        let url = if api_key == "test-key" || api_key == "demo" {
            format!(
                "https://api.coingecko.com/api/v3/coins/ethereum/contract/{}",
                addr_str
            )
        } else if api_key.starts_with("CG-") {
            format!(
                "https://api.coingecko.com/api/v3/coins/ethereum/contract/{}?x_cg_demo_api_key={}",
                addr_str, api_key
            )
        } else {
            format!(
                "https://pro-api.coingecko.com/api/v3/coins/ethereum/contract/{}?x_cg_pro_api_key={}",
                addr_str, api_key
            )
        };
        
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;
            
        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .header("User-Agent", "DeFi-Portfolio-Tracker/1.0")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        let token_data: CoinGeckoToken = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
            
        let symbol = token_data.symbol.to_uppercase();
        if !Self::is_valid_symbol(&symbol) {
            return Err(format!("Invalid symbol returned: {}", symbol));
        }
        
        Ok((symbol, token_data.name))
    }
    
    async fn try_blockchain_symbol(&self, _token_address: Address) -> Result<String, String> {
        // TODO: Implement blockchain call for symbol
        let symbol = format!("TOKEN_{}", &_token_address.to_string()[2..6].to_uppercase());
        if Self::is_valid_symbol(&symbol) {
            Ok(symbol)
        } else {
            Err(format!("Invalid symbol: {}", symbol))
        }
    }
    
    async fn calculate_real_position_value(
        &self,
        position_data: &INonfungiblePositionManager::Position,
        _token_id: U256,
    ) -> (f64, f64, f64) {
        let token0_price = self.get_token_price_usd(position_data.token0).await
            .unwrap_or_else(|_| self.get_fallback_price(position_data.token0));
            
        let token1_price = self.get_token_price_usd(position_data.token1).await
            .unwrap_or_else(|_| self.get_fallback_price(position_data.token1));

        let (amount0, amount1) = self.calculate_token_amounts_from_liquidity(
            position_data.liquidity,
            position_data.tickLower.as_i32(),
            position_data.tickUpper.as_i32(),
            position_data.token0,
            position_data.token1,
        ).await;

        let token0_value_usd = amount0 * token0_price;
        let token1_value_usd = amount1 * token1_price;
        let total_value_usd = token0_value_usd + token1_value_usd;

        let pnl_percentage = self.estimate_position_pnl(total_value_usd, position_data.fee.to());
        let pnl_usd = total_value_usd * (pnl_percentage / 100.0);

        (total_value_usd, pnl_usd, pnl_percentage)
    }

    async fn calculate_token_amounts_from_liquidity(
        &self,
        liquidity: u128,
        tick_lower: i32,
        tick_upper: i32,
        token0: Address,
        token1: Address,
    ) -> (f64, f64) {
        let token0_decimals = self.get_token_decimals(token0).await.unwrap_or(18);
        let token1_decimals = self.get_token_decimals(token1).await.unwrap_or(18);
        
        let current_price_ratio = self.estimate_current_price_ratio(token0, token1).await;
        
        let price_lower = self.tick_to_price(tick_lower);
        let price_upper = self.tick_to_price(tick_upper);
        let current_price = current_price_ratio;
        
        let liquidity_f64 = liquidity as f64;
        
        let (amount0_wei, amount1_wei) = if current_price < price_lower {
            let amount0 = liquidity_f64 * (1.0 / price_lower.sqrt() - 1.0 / price_upper.sqrt());
            (amount0, 0.0)
        } else if current_price > price_upper {
            let amount1 = liquidity_f64 * (price_upper.sqrt() - price_lower.sqrt());
            (0.0, amount1)
        } else {
            let amount0 = liquidity_f64 * (1.0 / current_price.sqrt() - 1.0 / price_upper.sqrt());
            let amount1 = liquidity_f64 * (current_price.sqrt() - price_lower.sqrt());
            (amount0, amount1)
        };
        
        let amount0_readable = amount0_wei / 10f64.powi(token0_decimals as i32);
        let amount1_readable = amount1_wei / 10f64.powi(token1_decimals as i32);
        
        (amount0_readable, amount1_readable)
    }

    fn tick_to_price(&self, tick: i32) -> f64 {
        1.0001_f64.powi(tick)
    }

    async fn estimate_current_price_ratio(&self, token0: Address, token1: Address) -> f64 {
        let price0 = self.get_token_price_usd(token0).await.unwrap_or(1.0);
        let price1 = self.get_token_price_usd(token1).await.unwrap_or(1.0);
        
        if price1 > 0.0 {
            price0 / price1
        } else {
            1.0
        }
    }

    fn estimate_position_pnl(&self, position_value_usd: f64, fee_tier: u32) -> f64 {
        let base_return = match fee_tier {
            500 => 2.0,
            3000 => 5.0,
            10000 => 8.0,
            _ => 3.0,
        };
        
        let size_adjustment = if position_value_usd > 100_000.0 {
            0.7
        } else if position_value_usd > 10_000.0 {
            0.9
        } else {
            1.2
        };
        
        base_return * size_adjustment
    }

    async fn get_token_decimals(&self, token_address: Address) -> Result<u8, String> {
        if let Some(decimals) = Self::get_known_token_decimals(token_address) {
            return Ok(decimals);
        }
        
        // TODO: Implement blockchain call for decimals
        Ok(18) // Default fallback
    }
    
    async fn get_token_price_usd(&self, token_address: Address) -> Result<f64, String> {
        let addr_str = format!("{:?}", token_address).to_lowercase();
        
        if self.is_stablecoin(token_address) {
            return Ok(1.0);
        }
        
        let base_url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3"
        } else {
            "https://api.coingecko.com/api/v3"
        };
        
        let url = format!(
            "{}/simple/token_price/ethereum?contract_addresses={}&vs_currencies=usd",
            base_url, addr_str
        );
        
        self.call_coingecko_price_api(&url).await
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
            
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
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

    // Helper functions
    fn get_known_token_symbol(address: Address) -> String {
        let addr_str = format!("{:?}", address).to_lowercase();
        match addr_str.as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH".to_string(),
            "0xa0b86a33e6c1a8c95f686066c9c9e8c8c8c8c2" => "USDC".to_string(),
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
            "0xa0b86a33e6c1a8c95f686066c9c9e8c8c8c8c2" => Some(6),  // USDC
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
        matches!(addr_str.as_str(),
            "0x6b175474e89094c44da98b954eedeac495271d0f" | // DAI
            "0xa0b86a33e6c1a8c95f686066c9c9e8c8c8c8c2" | // USDC  
            "0xdac17f958d2ee523a2206206994597c13d831ec7" | // USDT
            "0x4fabb145d64652a948d72533023f6e7a623c7c53" | // BUSD
            "0x853d955acef822db058eb8505911ed77f175b99e" | // FRAX
            "0x5f98805a4e8be255a32880fdec7f6728c6568ba0"   // LUSD
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
            "0x514910771af9ca656af840dff83e8264ecf986ca" => 25.0,   // LINK
            _ => 1.0,
        }
    }
    
    fn is_valid_symbol(symbol: &str) -> bool {
        !symbol.is_empty() 
            && symbol.len() <= 12 
            && symbol.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            && !symbol.to_lowercase().contains("test")
            && !symbol.to_lowercase().contains("fake")
            && !symbol.to_lowercase().contains("scam")
    }
    
    fn create_fallback_symbol(token_address: Address) -> String {
        let addr_str = format!("{:?}", token_address);
        if addr_str.len() >= 8 {
            format!("TOKEN_{}", &addr_str[addr_str.len()-6..].to_uppercase())
        } else {
            "UNKNOWN".to_string()
        }
    }
}

#[async_trait]
impl DeFiAdapter for UniswapV3Adapter {
    fn protocol_name(&self) -> &'static str {
        "uniswap_v3"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        // Check cache first
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Self::CACHE_DURATION {
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        let token_ids = self.get_user_token_ids(address).await?;
        
        if token_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        for token_id in token_ids {
            match self.get_position_details(token_id).await? {
                Some(position) => positions.push(position),
                None => continue,
            }
        }
        
        // Update cache
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
        contract_address == self.position_manager_address
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        Ok(position.value_usd)
    }
}