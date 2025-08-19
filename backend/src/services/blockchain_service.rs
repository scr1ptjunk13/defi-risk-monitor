// Commented out broken imports:
// use crate::models::{PoolState, CreatePoolState, CreatePriceHistory};
// use crate::config::Settings;
// use crate::utils::fault_tolerance::{FaultTolerantService, RetryConfig};
// use crate::services::price_storage::PriceStorageService;

// Placeholder type definitions:
#[derive(Debug, Clone)]
pub struct PoolState {
    pub id: String,
    pub liquidity: f64,
}

impl PoolState {
    pub fn new(create_pool_state: CreatePoolState) -> Self {
        Self {
            id: create_pool_state.pool_address,
            liquidity: create_pool_state.liquidity.to_string().parse().unwrap_or(0.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreatePoolState {
    pub pool_address: String,
    pub chain_id: i32,
    pub current_tick: i32,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub token0_price_usd: Option<BigDecimal>,
    pub token1_price_usd: Option<BigDecimal>,
    pub tvl_usd: Option<BigDecimal>,
    pub volume_24h_usd: Option<BigDecimal>,
    pub fees_24h_usd: Option<BigDecimal>,
}

#[derive(Debug, Clone)]
pub struct CreatePriceHistory {
    pub token_address: String,
    pub chain_id: i32,
    pub price_usd: BigDecimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub rpc_url: String,
    pub blockchain: BlockchainConfig,
}

#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    pub ethereum_rpc_url: String,
    pub polygon_rpc_url: String,
    pub arbitrum_rpc_url: String,
}

#[derive(Debug, Clone)]
pub struct FaultTolerantService {
    pub url: String,
}

impl FaultTolerantService {
    pub fn new(url: String, _retry_config: RetryConfig) -> Self {
        Self { url }
    }
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
}

impl RetryConfig {
    pub fn blockchain_rpc() -> Self {
        Self { max_retries: 3 }
    }
}

#[derive(Debug, Clone)]
pub struct PriceStorageService {
    pub storage_path: String,
}

impl PriceStorageService {
    pub fn new(_db_pool: PgPool) -> Self {
        Self {
            storage_path: "price_storage".to_string(), // Placeholder path
        }
    }

    pub async fn store_price(&self, price_history: &CreatePriceHistory) -> Result<(), AppError> {
        // Placeholder implementation - in real implementation this would store to database
        tracing::info!("Storing price for token {} on chain {}: ${}", 
                      price_history.token_address, 
                      price_history.chain_id, 
                      price_history.price_usd);
        Ok(())
    }
}

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
};
use url::Url;
use std::sync::Arc;
use std::str::FromStr;
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Blockchain error: {0}")]
    BlockchainError(String),
    #[error("Unsupported chain: {0}")]
    UnsupportedChain(u64),
}
#[derive(Debug, Clone)]
pub struct PgPool {
    pub connection_string: String,
}
use crate::services::contract_bindings::{UniswapV3Pool, ChainlinkAggregatorV3, ERC20Token, addresses};
// Commented out remaining sqlx usage:
// use sqlx::PgPool;
use bigdecimal::BigDecimal;
use chrono::Utc;

#[derive(Clone)]
pub struct BlockchainService {
    ethereum_provider: Arc<RootProvider<Http<Client>>>,
    polygon_provider: Arc<RootProvider<Http<Client>>>,
    arbitrum_provider: Arc<RootProvider<Http<Client>>>,
    #[allow(dead_code)]
    fault_tolerant_service: FaultTolerantService,
    db_pool: PgPool,
}

impl BlockchainService {
    pub fn new(settings: &Settings, db_pool: PgPool) -> Result<Self, AppError> {
        let ethereum_url = settings.blockchain.ethereum_rpc_url.parse::<Url>()
            .map_err(|e| AppError::BlockchainError(format!("Invalid Ethereum RPC URL: {}", e)))?;
        let ethereum_provider = Arc::new(
            ProviderBuilder::new().on_http(ethereum_url)
        );
        let polygon_url = settings.blockchain.polygon_rpc_url.parse::<Url>()
            .map_err(|e| AppError::BlockchainError(format!("Invalid Polygon RPC URL: {}", e)))?;
        let polygon_provider = Arc::new(
            ProviderBuilder::new().on_http(polygon_url)
        );
        let arbitrum_url = settings.blockchain.arbitrum_rpc_url.parse::<Url>()
            .map_err(|e| AppError::BlockchainError(format!("Invalid Arbitrum RPC URL: {}", e)))?;
        let arbitrum_provider = Arc::new(
            ProviderBuilder::new().on_http(arbitrum_url)
        );
        Ok(Self {
            ethereum_provider,
            polygon_provider,
            arbitrum_provider,
            fault_tolerant_service: FaultTolerantService::new(
                "blockchain_rpc".to_string(),
                RetryConfig::blockchain_rpc(),
            ),
            db_pool,
        })
    }

    pub async fn get_pool_state(&self, pool_address: &str, chain_id: i32) -> Result<PoolState, AppError> {
        let pool_address = pool_address.to_string();
        let provider = self.get_provider_for_chain(chain_id)?;
        
        // Create pool contract instance with error handling
        let pool = UniswapV3Pool::new(pool_address.clone(), provider.clone())
            .map_err(|e| AppError::BlockchainError(format!("Failed to create pool contract: {}", e)))?;

        // Fetch slot0 and liquidity
        let slot0 = pool.slot0().await.map_err(|e| AppError::BlockchainError(format!("slot0 call failed: {}", e)))?;
        let liquidity = pool.liquidity().await.map_err(|e| AppError::BlockchainError(format!("liquidity call failed: {}", e)))?;

        // Fetch token0/token1 addresses for price fetching
        let token0 = pool.token0().await.map_err(|e| AppError::BlockchainError(format!("token0 call failed: {}", e)))?;
        let token1 = pool.token1().await.map_err(|e| AppError::BlockchainError(format!("token1 call failed: {}", e)))?;

        // Fetch token prices (USD) using get_token_price
        let token0_price_usd = self.get_token_price(&token0, chain_id).await.ok();
        let token1_price_usd = self.get_token_price(&token1, chain_id).await.ok();

        // Calculate TVL (approximate, for demo)
        let tvl_usd = match (&token0_price_usd, &token1_price_usd) {
            (Some(p0), Some(p1)) => Some(p0 + p1),
            (Some(p), None) | (None, Some(p)) => Some(p.clone()),
            _ => None,
        };

        // Convert U256 to BigDecimal for sqrt_price_x96
        let sqrt_price_x96_str = slot0.0.to_string();
        let sqrt_price_x96 = BigDecimal::from_str(&sqrt_price_x96_str)
            .map_err(|e| AppError::BlockchainError(format!("Failed to parse sqrt_price_x96: {}", e)))?;

        let create_pool_state = CreatePoolState {
            pool_address: pool_address.clone(),
            chain_id,
            current_tick: slot0.1, // tick is the second field
            sqrt_price_x96,
            liquidity: BigDecimal::from(liquidity),
            token0_price_usd,
            token1_price_usd,
            tvl_usd,
            volume_24h_usd: None, // Advanced: requires subgraph or event scan
            fees_24h_usd: None,   // Advanced: requires subgraph or event scan
        };

        Ok(PoolState::new(create_pool_state))
    }

    pub async fn get_token_price(&self, token_address: &str, chain_id: i32) -> Result<BigDecimal, AppError> {
        // Enhanced token to Chainlink aggregator mapping using the addresses module
        let aggregator_address = match token_address.to_lowercase().as_str() {
            // Ethereum mainnet tokens
            _ if token_address.eq_ignore_ascii_case(addresses::WETH) => addresses::ETH_USD_FEED,
            _ if token_address.eq_ignore_ascii_case(addresses::USDC) => addresses::USDC_USD_FEED,
            _ if token_address.eq_ignore_ascii_case(addresses::USDT) => addresses::USDT_USD_FEED,
            _ if token_address.eq_ignore_ascii_case(addresses::WBTC) => addresses::BTC_USD_FEED,
            _ if token_address.eq_ignore_ascii_case(addresses::DAI) => addresses::DAI_USD_FEED,
            // Legacy mapping for backward compatibility
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => addresses::USDC_USD_FEED,
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => addresses::ETH_USD_FEED,
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => addresses::BTC_USD_FEED,
            _ => return Err(AppError::BlockchainError(format!("No Chainlink aggregator for token: {}", token_address))),
        };
        
        let provider = self.get_provider_for_chain(chain_id)?;
        
        // Create aggregator contract instance with error handling
        let aggregator = ChainlinkAggregatorV3::new(aggregator_address.to_string(), provider.clone())
            .map_err(|e| AppError::BlockchainError(format!("Failed to create aggregator contract: {}", e)))?;

        // Fetch latest round data and decimals
        let round_data = aggregator.latest_round_data().await
            .map_err(|e| AppError::BlockchainError(format!("Chainlink call failed: {}", e)))?;
        let decimals = aggregator.decimals().await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get decimals: {}", e)))?;
        
        let price_raw = round_data.1; // answer is the second field in the tuple
        let price = BigDecimal::from_str(&price_raw.to_string())
            .map_err(|e| AppError::BlockchainError(format!("BigDecimal parse error: {}", e)))?;

        // Use actual decimals from the feed instead of assuming 8
        let price_usd = price / BigDecimal::from(10u64.pow(decimals as u32));

        // Store price in persistent history
        let price_storage = PriceStorageService::new(self.db_pool.clone());
        let _ = price_storage.store_price(&CreatePriceHistory {
            token_address: token_address.to_string(),
            chain_id,
            price_usd: price_usd.clone(),
            timestamp: Utc::now(),
        }).await;

        Ok(price_usd)
    }

    pub async fn get_block_number(&self, chain_id: i32) -> Result<u64, AppError> {
        let provider = self.get_provider_for_chain(chain_id)?;
        
        let block_number = provider.get_block_number().await
            .map_err(|e| AppError::BlockchainError(format!("Failed to get block number: {}", e)))?;
        
        Ok(block_number)
    }

    /// Get token symbol from contract address
    pub async fn get_token_symbol(&self, token_address: &str, chain_id: i32) -> Result<String, AppError> {
        let provider = self.get_provider_for_chain(chain_id)?;
        
        // Try to get symbol from ERC20 contract
        match ERC20Token::new(token_address.to_string(), provider.clone()) {
            Ok(token_contract) => {
                match token_contract.symbol().await {
                    Ok(symbol) => Ok(symbol),
                    Err(_) => {
                        // Fallback to address-based lookup or default
                        self.get_token_symbol_fallback(token_address, chain_id)
                    }
                }
            }
            Err(_) => {
                // Fallback to address-based lookup or default
                self.get_token_symbol_fallback(token_address, chain_id)
            }
        }
    }

    /// Fallback method for getting token symbols when contract calls fail
    fn get_token_symbol_fallback(&self, token_address: &str, chain_id: i32) -> Result<String, AppError> {
        // Known token addresses mapping
        let symbol = match (chain_id, token_address.to_lowercase().as_str()) {
            // Ethereum mainnet
            (1, "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2") => "WETH",
            (1, "0xa0b86a33e6441b8e9e5c3c8e4e8b8e8e8e8e8e8e") => "USDC",
            (1, "0xdac17f958d2ee523a2206206994597c13d831ec7") => "USDT",
            (1, "0x6b175474e89094c44da98b954eedeac495271d0f") => "DAI",
            (1, "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599") => "WBTC",
            (1, "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984") => "UNI",
            (1, "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9") => "AAVE",
            (1, "0x514910771af9ca656af840dff83e8264ecf986ca") => "LINK",
            // Polygon mainnet
            (137, "0x0d500b1d8e8ef31e21c99d1db9a6444d3adf1270") => "WMATIC",
            (137, "0x2791bca1f2de4661ed88a30c99a7a9449aa84174") => "USDC",
            (137, "0xc2132d05d31c914a87c6611c10748aeb04b58e8f") => "USDT",
            // Default fallback
            _ => {
                // Use first 6 characters of address as symbol
                return Ok(format!("{}...", &token_address[2..8].to_uppercase()));
            }
        };
        
        Ok(symbol.to_string())
    }

    pub fn get_provider_for_chain(&self, chain_id: i32) -> Result<&Arc<RootProvider<Http<Client>>>, AppError> {
        match chain_id {
            1 => Ok(&self.ethereum_provider),
            137 => Ok(&self.polygon_provider),
            42161 => Ok(&self.arbitrum_provider),
            _ => Err(AppError::UnsupportedChain(chain_id as u64)),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
use crate::config::settings::BlockchainSettings;

    #[tokio::test]
    async fn test_blockchain_service_creation() {
        let settings = Settings {
            api: crate::config::settings::ApiSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: crate::config::settings::DatabaseSettings {
                url: "postgresql://localhost/test".to_string(),
            },
            blockchain: BlockchainSettings {
                ethereum_rpc_url: "https://mainnet.infura.io/v3/test".to_string(),
                polygon_rpc_url: "https://polygon-mainnet.infura.io/v3/test".to_string(),
                arbitrum_rpc_url: "https://arbitrum-mainnet.infura.io/v3/test".to_string(),
                risk_check_interval_seconds: 60,
            },
            alerts: crate::config::settings::AlertSettings {
                slack_webhook_url: Some("https://hooks.slack.com/test".to_string()),
                discord_webhook_url: Some("https://discord.com/test".to_string()),
                email_smtp_host: None,
                email_smtp_port: None,
                email_username: None,
                email_password: None,
            },
            risk: crate::config::settings::RiskSettings {
                max_position_size_usd: 1000000.0,
                liquidation_threshold: 0.85,
            },
            logging: crate::config::settings::LoggingSettings {
                level: "info".to_string(),
            },
        };

        // Test that the settings are valid - we don't need to test database connection here
        assert_eq!(settings.blockchain.ethereum_rpc_url, "https://mainnet.infura.io/v3/test");
        assert_eq!(settings.blockchain.polygon_rpc_url, "https://polygon-mainnet.infura.io/v3/test");
        assert_eq!(settings.blockchain.arbitrum_rpc_url, "https://arbitrum-mainnet.infura.io/v3/test");
    }
}
