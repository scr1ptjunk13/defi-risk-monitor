// Aave Price Service - Handles price fetching from multiple sources
use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;
use crate::adapters::traits::AdapterError;
use crate::blockchain::EthereumClient;
use crate::adapters::aave_v3::contracts::IAaveOracle;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub price_usd: f64,
    pub timestamp: SystemTime,
    pub source: PriceSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSource {
    AaveOracle,
    CoinGecko,
    Fallback,
}

#[derive(Debug, Clone)]
struct CachedPrice {
    price: f64,
    timestamp: SystemTime,
    source: PriceSource,
}

pub struct AavePriceService {
    client: EthereumClient,
    oracle_address: Address,
    price_cache: Arc<Mutex<HashMap<Address, CachedPrice>>>,
    cache_duration: Duration,
    coingecko_api_key: Option<String>,
}

impl AavePriceService {
    pub fn new(client: EthereumClient, oracle_address: Address) -> Self {
        Self {
            client,
            oracle_address,
            price_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_duration: Duration::from_secs(300), // 5 minutes
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
        }
    }

    /// Get price for a single asset with caching
    pub async fn get_price(&self, asset: Address) -> Result<PriceData, AdapterError> {
        // Check cache first
        if let Some(cached) = self.get_cached_price(asset) {
            return Ok(PriceData {
                price_usd: cached.price,
                timestamp: cached.timestamp,
                source: cached.source,
            });
        }

        // Try Aave Oracle first
        match self.fetch_oracle_price(asset).await {
            Ok(price) => {
                self.cache_price(asset, price, PriceSource::AaveOracle);
                return Ok(PriceData {
                    price_usd: price,
                    timestamp: SystemTime::now(),
                    source: PriceSource::AaveOracle,
                });
            }
            Err(e) => {
                tracing::warn!("Failed to fetch price from Aave Oracle for {:?}: {}", asset, e);
            }
        }

        // Fallback to CoinGecko
        if let Some(symbol) = self.get_token_symbol(asset) {
            match self.fetch_coingecko_price(&symbol).await {
                Ok(price) => {
                    self.cache_price(asset, price, PriceSource::CoinGecko);
                    return Ok(PriceData {
                        price_usd: price,
                        timestamp: SystemTime::now(),
                        source: PriceSource::CoinGecko,
                    });
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch price from CoinGecko for {}: {}", symbol, e);
                }
            }
        }

        // Final fallback to hardcoded prices
        let fallback_price = self.get_fallback_price(asset);
        self.cache_price(asset, fallback_price, PriceSource::Fallback);
        
        Ok(PriceData {
            price_usd: fallback_price,
            timestamp: SystemTime::now(),
            source: PriceSource::Fallback,
        })
    }

    /// Get prices for multiple assets
    pub async fn get_prices(&self, assets: &[Address]) -> Result<HashMap<Address, PriceData>, AdapterError> {
        let mut prices = HashMap::new();
        
        // Try to batch fetch from oracle first
        match self.fetch_oracle_prices(assets).await {
            Ok(oracle_prices) => {
                for (asset, price) in oracle_prices {
                    self.cache_price(asset, price, PriceSource::AaveOracle);
                    prices.insert(asset, PriceData {
                        price_usd: price,
                        timestamp: SystemTime::now(),
                        source: PriceSource::AaveOracle,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Failed to batch fetch prices from Aave Oracle: {}", e);
                
                // Fallback to individual fetches
                for &asset in assets {
                    match self.get_price(asset).await {
                        Ok(price_data) => {
                            prices.insert(asset, price_data);
                        }
                        Err(e) => {
                            tracing::error!("Failed to fetch price for {:?}: {}", asset, e);
                            // Insert fallback price
                            let fallback_price = self.get_fallback_price(asset);
                            prices.insert(asset, PriceData {
                                price_usd: fallback_price,
                                timestamp: SystemTime::now(),
                                source: PriceSource::Fallback,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(prices)
    }

    /// Fetch price from Aave Oracle
    async fn fetch_oracle_price(&self, asset: Address) -> Result<f64, AdapterError> {
        let oracle_address = self.oracle_address;
        let provider = self.client.provider().clone();
        let price_result = {
            let oracle = IAaveOracle::new(oracle_address, provider);
            oracle.getAssetPrice(asset).call().await
        };

        match price_result {
            Ok(price) => {
                // Aave oracle returns price in 8 decimals (USD)
                let price_f64 = price._0.to::<u128>() as f64 / 1e8;
                Ok(price_f64)
            }
            Err(e) => Err(AdapterError::ContractError(format!("Oracle call failed: {}", e))),
        }
    }

    /// Batch fetch prices from Aave Oracle
    async fn fetch_oracle_prices(&self, assets: &[Address]) -> Result<HashMap<Address, f64>, AdapterError> {
        let oracle_address = self.oracle_address;
        let provider = self.client.provider().clone();
        let assets_vec: Vec<Address> = assets.to_vec();
        let prices_result = {
            let oracle = IAaveOracle::new(oracle_address, provider);
            oracle.getAssetsPrices(assets_vec.clone()).call().await
        };

        match prices_result {
            Ok(prices) => {
                let mut price_map = HashMap::new();
                for (i, price) in prices._0.iter().enumerate() {
                    if let Some(&asset) = assets_vec.get(i) {
                        let price_f64 = price.to::<u128>() as f64 / 1e8;
                        price_map.insert(asset, price_f64);
                    }
                }
                Ok(price_map)
            }
            Err(e) => Err(AdapterError::ContractError(format!("Batch oracle call failed: {}", e))),
        }
    }

    /// Fetch price from CoinGecko API
    async fn fetch_coingecko_price(&self, symbol: &str) -> Result<f64, AdapterError> {
        let coin_id = self.symbol_to_coingecko_id(symbol);
        let url = if let Some(api_key) = &self.coingecko_api_key {
            format!(
                "https://pro-api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&x_cg_pro_api_key={}",
                coin_id, api_key
            )
        } else {
            format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
                coin_id
            )
        };

        let client = reqwest::Client::new();
        let response = timeout(Duration::from_secs(10), client.get(&url).send()).await
            .map_err(|_| AdapterError::Timeout("CoinGecko API timeout".to_string()))?
            .map_err(|e| AdapterError::NetworkError(format!("CoinGecko request failed: {}", e)))?;

        let json: serde_json::Value = response.json().await
            .map_err(|e| AdapterError::InvalidData(format!("Failed to parse CoinGecko response: {}", e)))?;

        json.get(&coin_id)
            .and_then(|coin| coin.get("usd"))
            .and_then(|price| price.as_f64())
            .ok_or_else(|| AdapterError::InvalidData(format!("Price not found for {}", symbol)))
    }

    /// Get cached price if valid
    fn get_cached_price(&self, asset: Address) -> Option<CachedPrice> {
        let cache = self.price_cache.lock().unwrap();
        if let Some(cached) = cache.get(&asset) {
            if cached.timestamp.elapsed().unwrap_or(Duration::MAX) < self.cache_duration {
                return Some(cached.clone());
            }
        }
        None
    }

    /// Cache a price
    fn cache_price(&self, asset: Address, price: f64, source: PriceSource) {
        let mut cache = self.price_cache.lock().unwrap();
        cache.insert(asset, CachedPrice {
            price,
            timestamp: SystemTime::now(),
            source,
        });
    }

    /// Get token symbol for address (simplified mapping)
    fn get_token_symbol(&self, asset: Address) -> Option<String> {
        let addr_str = format!("{:?}", asset).to_lowercase();
        match addr_str.as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => Some("ethereum".to_string()),
            "0xa0b86a33e6441e0b9b8b273c81f6c5b6d0e8f7b0" => Some("usd-coin".to_string()),
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => Some("tether".to_string()),
            "0x6b175474e89094c44da98b954eedeac495271d0f" => Some("dai".to_string()),
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => Some("wrapped-bitcoin".to_string()),
            "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9" => Some("aave".to_string()),
            "0x514910771af9ca656af840dff83e8264ecf986ca" => Some("chainlink".to_string()),
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => Some("uniswap".to_string()),
            _ => None,
        }
    }

    /// Convert symbol to CoinGecko ID
    fn symbol_to_coingecko_id(&self, symbol: &str) -> String {
        match symbol.to_lowercase().as_str() {
            "eth" | "weth" => "ethereum".to_string(),
            "usdc" => "usd-coin".to_string(),
            "usdt" => "tether".to_string(),
            "dai" => "dai".to_string(),
            "wbtc" => "wrapped-bitcoin".to_string(),
            "aave" => "aave".to_string(),
            "link" => "chainlink".to_string(),
            "uni" => "uniswap".to_string(),
            "matic" => "matic-network".to_string(),
            "avax" => "avalanche-2".to_string(),
            _ => symbol.to_lowercase(),
        }
    }

    /// Fallback price for when all other sources fail
    fn get_fallback_price(&self, asset: Address) -> f64 {
        let addr_str = format!("{:?}", asset).to_lowercase();
        match addr_str.as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => 2000.0, // ETH
            "0xa0b86a33e6441e0b9b8b273c81f6c5b6d0e8f7b0" => 1.0,    // USDC
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => 1.0,    // USDT
            "0x6b175474e89094c44da98b954eedeac495271d0f" => 1.0,    // DAI
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => 40000.0, // WBTC
            "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9" => 80.0,   // AAVE
            "0x514910771af9ca656af840dff83e8264ecf986ca" => 12.0,   // LINK
            "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => 6.0,    // UNI
            _ => 1.0, // Default fallback
        }
    }

    /// Clear price cache
    pub fn clear_cache(&self) {
        let mut cache = self.price_cache.lock().unwrap();
        cache.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let cache = self.price_cache.lock().unwrap();
        let total = cache.len();
        let valid = cache.values()
            .filter(|cached| cached.timestamp.elapsed().unwrap_or(Duration::MAX) < self.cache_duration)
            .count();
        (valid, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_mapping() {
        let service = AavePriceService::new(
            todo!("Mock client"),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap()
        );
        
        assert_eq!(service.symbol_to_coingecko_id("ETH"), "ethereum");
        assert_eq!(service.symbol_to_coingecko_id("USDC"), "usd-coin");
        assert_eq!(service.symbol_to_coingecko_id("WBTC"), "wrapped-bitcoin");
    }
    
    #[test]
    fn test_fallback_prices() {
        let service = AavePriceService::new(
            todo!("Mock client"),
            Address::from_str("0x0000000000000000000000000000000000000000").unwrap()
        );
        
        let eth_addr = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap();
        let usdc_addr = Address::from_str("0xA0b86a33E6441E0B9B8B273c81F6C5b6d0e8F7b0").unwrap();
        
        assert_eq!(service.get_fallback_price(eth_addr), 2000.0);
        assert_eq!(service.get_fallback_price(usdc_addr), 1.0);
    }
}
