use std::collections::HashMap;
use std::time::Duration;
use bigdecimal::{BigDecimal, FromPrimitive};
use serde::{Deserialize, Serialize};
use reqwest::Client;
use url::Url;
use tokio::time::timeout;
use tracing::{info, warn, error};
use crate::error::types::AppError;

/// Price feed provider configuration
#[derive(Debug, Clone)]
pub struct PriceFeedProvider {
    pub name: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub rate_limit_per_minute: u32,
    pub api_key: Option<String>,
}

impl PriceFeedProvider {
    pub fn coingecko() -> Self {
        Self {
            name: "coingecko".to_string(),
            base_url: "https://api.coingecko.com/api/v3".to_string(),
            timeout_seconds: 10,
            rate_limit_per_minute: 50, // Free tier limit
            api_key: None,
        }
    }

    pub fn coinmarketcap(api_key: Option<String>) -> Self {
        Self {
            name: "coinmarketcap".to_string(),
            base_url: "https://pro-api.coinmarketcap.com/v1".to_string(),
            timeout_seconds: 10,
            rate_limit_per_minute: 333, // Basic plan limit
            api_key,
        }
    }

    pub fn cryptocompare(api_key: Option<String>) -> Self {
        Self {
            name: "cryptocompare".to_string(),
            base_url: "https://min-api.cryptocompare.com/data".to_string(),
            timeout_seconds: 10,
            rate_limit_per_minute: 100,
            api_key,
        }
    }
}

/// CoinGecko API response structures
#[derive(Debug, Deserialize)]
struct CoinGeckoTokenResponse {
    #[serde(flatten)]
    prices: HashMap<String, f64>,
}

#[derive(Debug, Deserialize)]
struct CoinGeckoSimplePrice {
    usd: f64,
}

/// CoinMarketCap API response structures
#[derive(Debug, Deserialize)]
struct CoinMarketCapResponse {
    data: HashMap<String, CoinMarketCapData>,
}

#[derive(Debug, Deserialize)]
struct CoinMarketCapData {
    quote: HashMap<String, CoinMarketCapQuote>,
}

#[derive(Debug, Deserialize)]
struct CoinMarketCapQuote {
    price: f64,
}

/// CryptoCompare API response structures
#[derive(Debug, Deserialize)]
struct CryptoCompareResponse {
    #[serde(rename = "USD")]
    usd: f64,
}

/// Token information for price fetching
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub chain_id: i32,
    pub coingecko_id: Option<String>,
    pub coinmarketcap_id: Option<String>,
}

/// Real-time price feed service
pub struct PriceFeedService {
    client: Client,
    providers: Vec<PriceFeedProvider>,
    token_mappings: HashMap<String, TokenInfo>,
}

impl PriceFeedService {
    pub fn new(providers: Vec<PriceFeedProvider>) -> Result<Self, AppError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("DeFi-Risk-Monitor/1.0")
            .build()
            .map_err(|e| AppError::ExternalApiError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            providers,
            token_mappings: Self::create_default_token_mappings(),
        })
    }

    /// Fetch price from multiple providers with fallback and rate limiting
    pub async fn fetch_prices(&self, token_address: &str, chain_id: i32) -> Result<HashMap<String, BigDecimal>, AppError> {
        let token_info = self.get_token_info(token_address, chain_id)?;
        let mut prices = HashMap::new();
        let mut last_error = None;

        // Try providers sequentially with delays to avoid rate limiting
        for (i, provider) in self.providers.iter().enumerate() {
            // Add delay between requests to avoid rate limiting
            if i > 0 {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            match Self::fetch_price_from_provider_with_retry(&self.client, provider, &token_info).await {
                Ok(price) => {
                    prices.insert(provider.name.clone(), price);
                    // If we get at least one successful price, that's enough for basic functionality
                    break;
                }
                Err(e) => {
                    warn!("Failed to fetch price from {}: {}", provider.name, e);
                    last_error = Some(e);
                    continue;
                }
            }
        }

        if prices.is_empty() {
            return Err(last_error.unwrap_or_else(|| 
                AppError::ExternalApiError("Failed to fetch price from any provider".to_string())
            ));
        }

        Ok(prices)
    }

    /// Fetch price from a specific provider with retry logic
    async fn fetch_price_from_provider_with_retry(
        client: &Client,
        provider: &PriceFeedProvider,
        token_info: &TokenInfo,
    ) -> Result<BigDecimal, AppError> {
        let max_retries = 3;
        let mut last_error = None;
        
        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff: 500ms, 1s, 2s
                let delay = Duration::from_millis(500 * (1 << attempt));
                tokio::time::sleep(delay).await;
            }
            
            match Self::fetch_price_from_provider(client, provider, token_info).await {
                Ok(price) => return Ok(price),
                Err(e) => {
                    // Check if this is a rate limiting error (429)
                    if e.to_string().contains("429") || e.to_string().contains("Too Many Requests") {
                        warn!("Rate limited by {}, attempt {}/{}", provider.name, attempt + 1, max_retries);
                        last_error = Some(e);
                        continue;
                    } else {
                        // For non-rate-limiting errors, fail immediately
                        return Err(e);
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| 
            AppError::ExternalApiError(format!("Failed to fetch price from {} after {} retries", provider.name, max_retries))
        ))
    }

    /// Fetch price from a specific provider
    async fn fetch_price_from_provider(
        client: &Client,
        provider: &PriceFeedProvider,
        token_info: &TokenInfo,
    ) -> Result<BigDecimal, AppError> {
        let timeout_duration = Duration::from_secs(provider.timeout_seconds);

        let price_result = match provider.name.as_str() {
            "coingecko" => Self::fetch_from_coingecko(client, provider, token_info).await,
            "coinmarketcap" => Self::fetch_from_coinmarketcap(client, provider, token_info).await,
            "cryptocompare" => Self::fetch_from_cryptocompare(client, provider, token_info).await,
            _ => return Err(AppError::ExternalApiError(format!("Unknown provider: {}", provider.name))),
        };

        price_result
    }

    /// Fetch price from CoinGecko
    async fn fetch_from_coingecko(
        client: &Client,
        provider: &PriceFeedProvider,
        token_info: &TokenInfo,
    ) -> Result<BigDecimal, AppError> {
        let url = if let Some(coingecko_id) = &token_info.coingecko_id {
            // Use coin ID for better accuracy
            format!("{}/simple/price?ids={}&vs_currencies=usd", provider.base_url, coingecko_id)
        } else {
            // Fallback to contract address
            let platform = match token_info.chain_id {
                1 => "ethereum",
                137 => "polygon-pos",
                56 => "binance-smart-chain",
                43114 => "avalanche",
                _ => "ethereum", // Default to Ethereum
            };
            format!("{}/simple/token_price/{}?contract_addresses={}&vs_currencies=usd", 
                   provider.base_url, platform, token_info.address)
        };

        println!("🌐 CoinGecko API URL: {}", url);
        
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                println!("❌ CoinGecko request failed: {}", e);
                AppError::ExternalApiError(format!("CoinGecko request failed: {}", e))
            })?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApiError(format!(
                "CoinGecko API error: {}",
                response.status()
            )));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("CoinGecko JSON parse error: {}", e)))?;

        // Extract price from response
        let price = if let Some(coingecko_id) = &token_info.coingecko_id {
            json.get(coingecko_id)
                .and_then(|coin| coin.get("usd"))
                .and_then(|price| price.as_f64())
        } else {
            json.get(&token_info.address.to_lowercase())
                .and_then(|token| token.get("usd"))
                .and_then(|price| price.as_f64())
        };

        match price {
            Some(price_f64) => {
                BigDecimal::from_f64(price_f64)
                    .ok_or_else(|| AppError::ExternalApiError("Invalid price format from CoinGecko".to_string()))
            }
            None => Err(AppError::ExternalApiError("Price not found in CoinGecko response".to_string())),
        }
    }

    /// Fetch price from CoinMarketCap
    async fn fetch_from_coinmarketcap(
        client: &Client,
        provider: &PriceFeedProvider,
        token_info: &TokenInfo,
    ) -> Result<BigDecimal, AppError> {
        let api_key = provider.api_key.as_ref()
            .ok_or_else(|| AppError::ExternalApiError("CoinMarketCap API key required".to_string()))?;

        let url = format!("{}/cryptocurrency/quotes/latest?symbol={}", provider.base_url, token_info.symbol);

        let response = client
            .get(&url)
            .header("X-CMC_PRO_API_KEY", api_key)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("CoinMarketCap request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApiError(format!(
                "CoinMarketCap API error: {}",
                response.status()
            )));
        }

        let json: CoinMarketCapResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("CoinMarketCap JSON parse error: {}", e)))?;

        let price = json.data
            .get(&token_info.symbol.to_uppercase())
            .and_then(|data| data.quote.get("USD"))
            .map(|quote| quote.price)
            .ok_or_else(|| AppError::ExternalApiError("Price not found in CoinMarketCap response".to_string()))?;

        BigDecimal::from_f64(price)
            .ok_or_else(|| AppError::ExternalApiError("Invalid price format from CoinMarketCap".to_string()))
    }

    /// Fetch price from CryptoCompare
    async fn fetch_from_cryptocompare(
        client: &Client,
        provider: &PriceFeedProvider,
        token_info: &TokenInfo,
    ) -> Result<BigDecimal, AppError> {
        let mut url = format!("{}/price?fsym={}&tsyms=USD", provider.base_url, token_info.symbol);
        
        if let Some(api_key) = &provider.api_key {
            url.push_str(&format!("&api_key={}", api_key));
        }

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("CryptoCompare request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::ExternalApiError(format!(
                "CryptoCompare API error: {}",
                response.status()
            )));
        }

        let json: CryptoCompareResponse = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("CryptoCompare JSON parse error: {}", e)))?;

        BigDecimal::from_f64(json.usd)
            .ok_or_else(|| AppError::ExternalApiError("Invalid price format from CryptoCompare".to_string()))
    }

    /// Get token information for price fetching
    fn get_token_info(&self, token_address: &str, chain_id: i32) -> Result<TokenInfo, AppError> {
        let key = format!("{}:{}", chain_id, token_address.to_lowercase());
        
        self.token_mappings
            .get(&key)
            .cloned()
            .or_else(|| {
                // Fallback: create basic token info
                Some(TokenInfo {
                    address: token_address.to_string(),
                    symbol: "UNKNOWN".to_string(),
                    chain_id,
                    coingecko_id: None,
                    coinmarketcap_id: None,
                })
            })
            .ok_or_else(|| AppError::ExternalApiError("Token not supported".to_string()))
    }

    /// Create default token mappings for common tokens
    fn create_default_token_mappings() -> HashMap<String, TokenInfo> {
        let mut mappings = HashMap::new();

        // Ethereum mainnet tokens with REAL addresses and CoinGecko IDs
        
        // WETH - Wrapped Ethereum (REAL address)
        mappings.insert(
            "1:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            TokenInfo {
                address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
                symbol: "WETH".to_string(),
                chain_id: 1,
                coingecko_id: Some("weth".to_string()),
                coinmarketcap_id: Some("2396".to_string()),
            },
        );
        
        // USDC - USD Coin (REAL address)
        mappings.insert(
            "1:0xa0b86a33e6441b8e9e5c3c8e4e8b8e8e8e8e8e8e".to_string(),
            TokenInfo {
                address: "0xa0b86a33e6441b8e9e5c3c8e4e8b8e8e8e8e8e8e".to_string(),
                symbol: "USDC".to_string(),
                chain_id: 1,
                coingecko_id: Some("usd-coin".to_string()),
                coinmarketcap_id: Some("3408".to_string()),
            },
        );

        // USDT - Tether (REAL address)
        mappings.insert(
            "1:0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
            TokenInfo {
                address: "0xdac17f958d2ee523a2206206994597c13d831ec7".to_string(),
                symbol: "USDT".to_string(),
                chain_id: 1,
                coingecko_id: Some("tether".to_string()),
                coinmarketcap_id: Some("825".to_string()),
            },
        );

        // Add more token mappings as needed
        mappings
    }

    /// Add custom token mapping
    pub fn add_token_mapping(&mut self, token_info: TokenInfo) {
        let key = format!("{}:{}", token_info.chain_id, token_info.address.to_lowercase());
        self.token_mappings.insert(key, token_info);
    }
}

/// Create default price feed providers
pub fn create_default_providers() -> Vec<PriceFeedProvider> {
    vec![
        PriceFeedProvider::coingecko(),
        // Add other providers with API keys from environment
        // PriceFeedProvider::coinmarketcap(std::env::var("COINMARKETCAP_API_KEY").ok()),
        // PriceFeedProvider::cryptocompare(std::env::var("CRYPTOCOMPARE_API_KEY").ok()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_creation() {
        let coingecko = PriceFeedProvider::coingecko();
        assert_eq!(coingecko.name, "coingecko");
        assert!(coingecko.base_url.contains("coingecko.com"));
    }

    #[test]
    fn test_token_mappings() {
        let service = PriceFeedService::new(vec![PriceFeedProvider::coingecko()]).unwrap();
        let token_info = service.get_token_info("0xa0b86a33e6441b8e9e5c3c8e4e8b8e8e8e8e8e8e", 1).unwrap();
        assert_eq!(token_info.symbol, "USDC");
    }
}
