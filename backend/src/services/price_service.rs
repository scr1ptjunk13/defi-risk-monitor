use alloy::primitives::Address;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use tokio::time::{sleep, Duration};

#[derive(Debug, thiserror::Error)]
pub enum PriceError {
    #[error("API request failed: {0}")]
    ApiError(String),
    
    #[error("Token not found: {0}")]
    TokenNotFound(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub address: Address,
    pub symbol: String,
    pub price_usd: f64,
    pub timestamp: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    prices: HashMap<String, CoinGeckoPrice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CoinGeckoPrice {
    usd: f64,
}

/// Price service for fetching real-time token prices
pub struct PriceService {
    client: reqwest::Client,
    coingecko_api_key: Option<String>,
    cache: HashMap<Address, TokenPrice>,
    cache_ttl: Duration,
}

impl PriceService {
    pub fn new(coingecko_api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            coingecko_api_key,
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(30), // 30 second cache
        }
    }
    
    /// Get price for a single token
    pub async fn get_token_price(&mut self, address: Address) -> Result<TokenPrice, PriceError> {
        // Check cache first
        if let Some(cached_price) = self.cache.get(&address) {
            let age = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() - cached_price.timestamp;
                
            if age < self.cache_ttl.as_secs() {
                return Ok(cached_price.clone());
            }
        }
        
        // Fetch from API
        let price = self.fetch_price_from_coingecko(address).await?;
        
        // Cache the result
        self.cache.insert(address, price.clone());
        
        Ok(price)
    }
    
    /// Get prices for multiple tokens
    pub async fn get_token_prices(&mut self, addresses: Vec<Address>) -> Result<Vec<TokenPrice>, PriceError> {
        let mut prices = Vec::new();
        
        // TODO: Implement batch fetching for better performance
        for address in addresses {
            match self.get_token_price(address).await {
                Ok(price) => prices.push(price),
                Err(e) => {
                    tracing::warn!(
                        token_address = %address,
                        error = %e,
                        "Failed to fetch price for token"
                    );
                    // Continue with other tokens
                }
            }
            
            // Rate limiting - wait between requests
            sleep(Duration::from_millis(100)).await;
        }
        
        Ok(prices)
    }
    
    /// Fetch price from CoinGecko API
    async fn fetch_price_from_coingecko(&self, address: Address) -> Result<TokenPrice, PriceError> {
        let contract_address = format!("{:?}", address).to_lowercase();
        
        let mut url = format!(
            "https://api.coingecko.com/api/v3/simple/token_price/ethereum?contract_addresses={}&vs_currencies=usd",
            contract_address
        );
        
        // Add API key if available
        if let Some(api_key) = &self.coingecko_api_key {
            url.push_str(&format!("&x_cg_demo_api_key={}", api_key));
        }
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "DeFi-Risk-Monitor/1.0")
            .send()
            .await
            .map_err(|e| PriceError::ApiError(format!("Request failed: {}", e)))?;
            
        if response.status() == 429 {
            return Err(PriceError::RateLimitExceeded);
        }
        
        if !response.status().is_success() {
            return Err(PriceError::ApiError(format!(
                "API returned status: {}", response.status()
            )));
        }
        
        let response_text = response.text().await
            .map_err(|e| PriceError::ApiError(format!("Failed to read response: {}", e)))?;
            
        let parsed: CoinGeckoResponse = serde_json::from_str(&response_text)
            .map_err(|e| PriceError::InvalidResponse(format!("JSON parse error: {}", e)))?;
        
        // Extract price for our token
        let price_data = parsed.prices.get(&contract_address)
            .ok_or_else(|| PriceError::TokenNotFound(contract_address.clone()))?;
        
        Ok(TokenPrice {
            address,
            symbol: "UNKNOWN".to_string(), // TODO: Resolve symbol
            price_usd: price_data.usd,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            source: "coingecko".to_string(),
        })
    }
    
    /// Get ETH price (commonly needed)
    pub async fn get_eth_price(&mut self) -> Result<f64, PriceError> {
        // ETH is not a contract, use special endpoint
        let url = if let Some(api_key) = &self.coingecko_api_key {
            format!("https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd&x_cg_demo_api_key={}", api_key)
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd".to_string()
        };
        
        let response = self.client
            .get(&url)
            .header("User-Agent", "DeFi-Risk-Monitor/1.0")
            .send()
            .await
            .map_err(|e| PriceError::ApiError(format!("ETH price request failed: {}", e)))?;
            
        if !response.status().is_success() {
            return Err(PriceError::ApiError(format!(
                "ETH price API returned status: {}", response.status()
            )));
        }
        
        let response_text = response.text().await
            .map_err(|e| PriceError::ApiError(format!("Failed to read ETH price response: {}", e)))?;
            
        let parsed: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| PriceError::InvalidResponse(format!("ETH price JSON parse error: {}", e)))?;
        
        let eth_price = parsed["ethereum"]["usd"].as_f64()
            .ok_or_else(|| PriceError::InvalidResponse("ETH price not found in response".to_string()))?;
            
        Ok(eth_price)
    }
    
    /// Clear the price cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, Duration) {
        (self.cache.len(), self.cache_ttl)
    }
}

/// Common token addresses on Ethereum mainnet
pub struct CommonTokens;

impl CommonTokens {
    pub const WETH: &'static str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &'static str = "0xA0b86a33E6441E6C6A4c4b8C7c8c5c8c5c8c5c8c";
    pub const USDT: &'static str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
    pub const DAI: &'static str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
    pub const STETH: &'static str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";
    
    pub fn get_address(symbol: &str) -> Option<Address> {
        let addr_str = match symbol.to_uppercase().as_str() {
            "WETH" => Self::WETH,
            "USDC" => Self::USDC,
            "USDT" => Self::USDT,
            "DAI" => Self::DAI,
            "STETH" => Self::STETH,
            _ => return None,
        };
        
        Address::from_str(addr_str).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_common_tokens() {
        assert!(CommonTokens::get_address("WETH").is_some());
        assert!(CommonTokens::get_address("USDC").is_some());
        assert!(CommonTokens::get_address("UNKNOWN").is_none());
    }
    
    #[tokio::test]
    async fn test_price_service_creation() {
        let service = PriceService::new(None);
        assert_eq!(service.cache.len(), 0);
    }
}
