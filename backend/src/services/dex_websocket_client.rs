use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use tracing::{info, warn, error, debug};
use url::Url;

use crate::error::AppError;

/// DEX-specific WebSocket endpoints and configurations
#[derive(Debug, Clone)]
pub struct DexConfig {
    pub name: String,
    pub websocket_url: String,
    pub subscription_format: SubscriptionFormat,
    pub rate_limit_ms: u64,
}

#[derive(Debug, Clone)]
pub enum SubscriptionFormat {
    TheGraph,
    Alchemy,
    Infura,
    Custom(String),
}

/// Real-time pool update from DEX WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolUpdate {
    pub pool_address: String,
    pub chain_id: u64,
    pub current_tick: Option<i32>,
    pub sqrt_price_x96: Option<BigDecimal>,
    pub liquidity: Option<BigDecimal>,
    pub token0_price_usd: Option<BigDecimal>,
    pub token1_price_usd: Option<BigDecimal>,
    pub tvl_usd: Option<BigDecimal>,
    pub fees_24h_usd: Option<BigDecimal>,
    pub volume_24h_usd: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
    pub source: String,
}

/// DEX WebSocket client for real-time data ingestion
#[derive(Clone)]
pub struct DexWebSocketClient {
    configs: Arc<RwLock<HashMap<String, DexConfig>>>,
    pool_updates: broadcast::Sender<PoolUpdate>,
    active_connections: Arc<RwLock<HashMap<String, bool>>>,
}

impl DexWebSocketClient {
    /// Create a new DEX WebSocket client
    pub fn new() -> Self {
        let (pool_updates, _) = broadcast::channel(1000);
        
        Self {
            configs: Arc::new(RwLock::new(HashMap::new())),
            pool_updates,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add DEX configuration
    pub async fn add_dex_config(&self, config: DexConfig) -> Result<(), AppError> {
        let mut configs = self.configs.write().await;
        configs.insert(config.name.clone(), config);
        Ok(())
    }

    /// Initialize default DEX configurations with real endpoints
    pub async fn initialize_default_configs(&self) -> Result<(), AppError> {
        // Get API keys from environment
        let alchemy_api_key = std::env::var("ALCHEMY_API_KEY")
            .unwrap_or_else(|_| "demo".to_string());
        let infura_api_key = std::env::var("INFURA_API_KEY")
            .unwrap_or_else(|_| "demo".to_string());
        let _thegraph_api_key = std::env::var("THEGRAPH_API_KEY")
            .unwrap_or_else(|_| "demo".to_string());
        
        // Check if we're in production mode with real API keys
        let is_production_mode = alchemy_api_key != "demo" && infura_api_key != "demo";
        
        if is_production_mode {
            tracing::info!("🚀 PRODUCTION MODE: Using real API keys for live DEX data");
        } else {
            tracing::warn!("⚠️  DEMO MODE: Using demo API keys - limited/no real data expected");
            tracing::warn!("   To enable real data: Set ALCHEMY_API_KEY and INFURA_API_KEY in .env");
        }

        // Uniswap V3 via The Graph Protocol (Production)
        self.add_dex_config(DexConfig {
            name: "uniswap_v3".to_string(),
            websocket_url: "wss://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3".to_string(),
            subscription_format: SubscriptionFormat::TheGraph,
            rate_limit_ms: 100,
        }).await?;

        // Alchemy WebSocket for Ethereum mainnet events (Production)
        self.add_dex_config(DexConfig {
            name: "alchemy_mainnet".to_string(),
            websocket_url: format!("wss://eth-mainnet.g.alchemy.com/v2/{}", alchemy_api_key),
            subscription_format: SubscriptionFormat::Alchemy,
            rate_limit_ms: 50,
        }).await?;

        // SushiSwap via The Graph (Production)
        self.add_dex_config(DexConfig {
            name: "sushiswap".to_string(),
            websocket_url: "wss://api.thegraph.com/subgraphs/name/sushiswap/exchange".to_string(),
            subscription_format: SubscriptionFormat::TheGraph,
            rate_limit_ms: 100,
        }).await?;

        // Curve Finance via The Graph (Production)
        self.add_dex_config(DexConfig {
            name: "curve_finance".to_string(),
            websocket_url: "wss://api.thegraph.com/subgraphs/name/curvefi/curve".to_string(),
            subscription_format: SubscriptionFormat::TheGraph,
            rate_limit_ms: 100,
        }).await?;

        // Infura WebSocket for additional redundancy (Production)
        self.add_dex_config(DexConfig {
            name: "infura_mainnet".to_string(),
            websocket_url: format!("wss://mainnet.infura.io/ws/v3/{}", infura_api_key),
            subscription_format: SubscriptionFormat::Infura,
            rate_limit_ms: 50,
        }).await?;

        info!("Initialized production DEX WebSocket configurations with {} endpoints", 
              self.configs.read().await.len());
        Ok(())
    }

    /// Start WebSocket connection to a specific DEX
    pub async fn start_dex_connection(&self, dex_name: &str) -> Result<(), AppError> {
        let config = {
            let configs = self.configs.read().await;
            configs.get(dex_name)
                .ok_or_else(|| AppError::ConfigError(format!("DEX config not found: {}", dex_name)))?
                .clone()
        };

        // Mark connection as active
        {
            let mut connections = self.active_connections.write().await;
            connections.insert(dex_name.to_string(), true);
        }

        let pool_updates = self.pool_updates.clone();
        let active_connections = self.active_connections.clone();
        let dex_name = dex_name.to_string();

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = Self::maintain_connection(config, pool_updates, active_connections.clone(), dex_name.clone()).await {
                error!("DEX WebSocket connection failed for {}: {}", dex_name, e);
                
                // Mark connection as inactive
                let mut connections = active_connections.write().await;
                connections.insert(dex_name, false);
            }
        });

        Ok(())
    }

    /// Maintain WebSocket connection with reconnection logic
    async fn maintain_connection(
        config: DexConfig,
        pool_updates: broadcast::Sender<PoolUpdate>,
        active_connections: Arc<RwLock<HashMap<String, bool>>>,
        dex_name: String,
    ) -> Result<(), AppError> {
        let mut reconnect_attempts = 0;
        const MAX_RECONNECT_ATTEMPTS: u32 = 10;
        const RECONNECT_DELAY: Duration = Duration::from_secs(5);

        loop {
            match Self::connect_and_stream(&config, &pool_updates).await {
                Ok(_) => {
                    info!("DEX WebSocket connection established: {}", config.name);
                    reconnect_attempts = 0;
                }
                Err(e) => {
                    error!("DEX WebSocket connection error for {}: {}", config.name, e);
                    reconnect_attempts += 1;

                    if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                        error!("Max reconnection attempts reached for {}", config.name);
                        break;
                    }

                    warn!("Reconnecting to {} in {:?} (attempt {}/{})", 
                          config.name, RECONNECT_DELAY, reconnect_attempts, MAX_RECONNECT_ATTEMPTS);
                    tokio::time::sleep(RECONNECT_DELAY).await;
                }
            }

            // Check if connection should remain active
            let should_continue = {
                let connections = active_connections.read().await;
                connections.get(&dex_name).copied().unwrap_or(false)
            };

            if !should_continue {
                info!("Stopping DEX WebSocket connection: {}", config.name);
                break;
            }
        }

        Ok(())
    }

    /// Connect to DEX WebSocket and stream data
    async fn connect_and_stream(
        config: &DexConfig,
        pool_updates: &broadcast::Sender<PoolUpdate>,
    ) -> Result<(), AppError> {
        let url = Url::parse(&config.websocket_url)
            .map_err(|e| AppError::ConfigError(format!("Invalid WebSocket URL: {}", e)))?;

        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| AppError::ExternalServiceError(format!("WebSocket connection failed: {}", e)))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        // Send subscription message based on DEX format
        let subscription_msg = Self::create_subscription_message(&config.subscription_format)?;
        ws_sender.send(Message::Text(subscription_msg)).await
            .map_err(|e| AppError::ExternalServiceError(format!("Failed to send subscription: {}", e)))?;

        info!("Subscribed to {} WebSocket feed", config.name);

        // Stream incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(pool_update) = Self::parse_pool_update(&text, &config.name) {
                        debug!("Received pool update from {}: {:?}", config.name, pool_update);
                        
                        if let Err(e) = pool_updates.send(pool_update) {
                            warn!("Failed to broadcast pool update: {}", e);
                        }
                    }
                }
                Ok(Message::Ping(data)) => {
                    // Respond to ping with pong
                    if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                        error!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed by server: {}", config.name);
                    break;
                }
                Err(e) => {
                    error!("WebSocket error for {}: {}", config.name, e);
                    break;
                }
                _ => {}
            }

            // Rate limiting
            if config.rate_limit_ms > 0 {
                tokio::time::sleep(Duration::from_millis(config.rate_limit_ms)).await;
            }
        }

        Ok(())
    }

    /// Create subscription message based on DEX format
    fn create_subscription_message(format: &SubscriptionFormat) -> Result<String, AppError> {
        match format {
            SubscriptionFormat::TheGraph => {
                // GraphQL subscription for pool updates
                Ok(serde_json::json!({
                    "id": "1",
                    "type": "start",
                    "payload": {
                        "query": r#"
                            subscription {
                                pools(first: 100, orderBy: totalValueLockedUSD, orderDirection: desc) {
                                    id
                                    tick
                                    sqrtPrice
                                    liquidity
                                    totalValueLockedUSD
                                    volumeUSD
                                    feesUSD
                                    token0 {
                                        id
                                        symbol
                                        derivedETH
                                    }
                                    token1 {
                                        id
                                        symbol
                                        derivedETH
                                    }
                                }
                            }
                        "#
                    }
                }).to_string())
            }
            SubscriptionFormat::Alchemy => {
                // Alchemy WebSocket subscription for Uniswap V3 pool events
                Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_subscribe",
                    "params": ["logs", {
                        "address": [
                            "0x1f98431c8ad98523631ae4a59f267346ea31f984", // Uniswap V3 Factory
                            "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5601", // USDC/ETH 0.05%
                            "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8", // USDC/ETH 0.3%
                            "0x4e68Ccd3E89f51C3074ca5072bbAC773960dFa36"  // ETH/USDT 0.3%
                        ],
                        "topics": [
                            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67", // Swap event
                            "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde", // Mint event
                            "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c"  // Burn event
                        ]
                    }]
                }).to_string())
            }
            SubscriptionFormat::Infura => {
                // Infura WebSocket subscription for DEX events
                Ok(serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "eth_subscribe",
                    "params": ["logs", {
                        "address": [
                            "0x1f98431c8ad98523631ae4a59f267346ea31f984", // Uniswap V3 Factory
                            "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F", // SushiSwap Router
                            "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D"  // Uniswap V2 Router
                        ],
                        "topics": [
                            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" // Swap event
                        ]
                    }]
                }).to_string())
            }
            SubscriptionFormat::Custom(msg) => Ok(msg.clone()),
        }
    }

    /// Parse incoming message into PoolUpdate
    fn parse_pool_update(message: &str, source: &str) -> Result<PoolUpdate, AppError> {
        let json: Value = serde_json::from_str(message)
            .map_err(|e| AppError::ValidationError(format!("Invalid JSON: {}", e)))?;

        // Parse based on source format
        match source {
            "uniswap_v3" | "sushiswap" | "curve_finance" => Self::parse_thegraph_update(&json, source),
            "alchemy_mainnet" | "infura_mainnet" => Self::parse_alchemy_update(&json, source),
            _ => {
                debug!("Attempting to parse unknown source format: {}", source);
                // Try to parse as generic format
                Self::parse_generic_update(&json, source)
            }
        }
    }

    /// Parse The Graph protocol message
    fn parse_thegraph_update(json: &Value, source: &str) -> Result<PoolUpdate, AppError> {
        // Extract pool data from The Graph response
        if let Some(data) = json.get("payload").and_then(|p| p.get("data")).and_then(|d| d.get("pools")) {
            if let Some(pool) = data.as_array().and_then(|pools| pools.first()) {
                return Ok(PoolUpdate {
                    pool_address: pool.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    chain_id: 1, // Ethereum mainnet
                    current_tick: pool.get("tick").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    sqrt_price_x96: pool.get("sqrtPrice").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    liquidity: pool.get("liquidity").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    token0_price_usd: None, // Will be calculated separately
                    token1_price_usd: None,
                    tvl_usd: pool.get("totalValueLockedUSD").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    fees_24h_usd: pool.get("feesUSD").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    volume_24h_usd: pool.get("volumeUSD").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()),
                    timestamp: Utc::now(),
                    source: source.to_string(),
                });
            }
        }

        Err(AppError::ValidationError("Failed to parse The Graph pool update".to_string()))
    }

    /// Parse Alchemy WebSocket message (real blockchain events)
    fn parse_alchemy_update(json: &Value, source: &str) -> Result<PoolUpdate, AppError> {
        // Handle subscription confirmation
        if json.get("method").and_then(|v| v.as_str()) == Some("eth_subscription") {
            if let Some(params) = json.get("params") {
                if let Some(result) = params.get("result") {
                    // Extract pool address from log
                    let pool_address = result.get("address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Extract transaction hash for tracking
                    let tx_hash = result.get("transactionHash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");

                    // Extract block number for ordering
                    let block_number = result.get("blockNumber")
                        .and_then(|v| v.as_str())
                        .and_then(|s| u64::from_str_radix(&s[2..], 16).ok())
                        .unwrap_or(0);

                    // Parse event topics to determine event type
                    let topics = result.get("topics")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default();

                    // Determine if this is a swap, mint, or burn event
                    let event_type = if !topics.is_empty() {
                        match topics[0] {
                            "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67" => "swap",
                            "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde" => "mint",
                            "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c" => "burn",
                            _ => "unknown"
                        }
                    } else {
                        "unknown"
                    };

                    debug!("Parsed {} event from pool {} in block {}", event_type, pool_address, block_number);

                    return Ok(PoolUpdate {
                        pool_address,
                        chain_id: 1, // Ethereum mainnet
                        current_tick: None, // Would need to decode log data for this
                        sqrt_price_x96: None, // Would need to decode log data for this
                        liquidity: None, // Would need to decode log data for this
                        token0_price_usd: None, // Calculated separately
                        token1_price_usd: None, // Calculated separately
                        tvl_usd: None, // Calculated separately
                        fees_24h_usd: None, // Calculated separately
                        volume_24h_usd: None, // Calculated separately
                        timestamp: Utc::now(),
                        source: format!("{}_{}_{}", source, event_type, tx_hash),
                    });
                }
            }
        }

        // Handle subscription result (initial response)
        if let Some(result) = json.get("result") {
            if result.is_string() {
                debug!("Alchemy subscription established: {}", result.as_str().unwrap_or(""));
                // Return a placeholder update to indicate successful subscription
                return Ok(PoolUpdate {
                    pool_address: "subscription_established".to_string(),
                    chain_id: 1,
                    current_tick: None,
                    sqrt_price_x96: None,
                    liquidity: None,
                    token0_price_usd: None,
                    token1_price_usd: None,
                    tvl_usd: None,
                    fees_24h_usd: None,
                    volume_24h_usd: None,
                    timestamp: Utc::now(),
                    source: source.to_string(),
                });
            }
        }

        Err(AppError::ValidationError("Failed to parse Alchemy pool update".to_string()))
    }

    /// Parse generic WebSocket message (fallback)
    fn parse_generic_update(json: &Value, source: &str) -> Result<PoolUpdate, AppError> {
        // Try to extract common fields from any JSON structure
        let pool_address = json.get("address")
            .or_else(|| json.get("pool"))
            .or_else(|| json.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let tvl_usd = json.get("tvl")
            .or_else(|| json.get("totalValueLockedUSD"))
            .or_else(|| json.get("liquidity_usd"))
            .and_then(|v| {
                if let Some(s) = v.as_str() {
                    s.parse().ok()
                } else if let Some(f) = v.as_f64() {
                    Some(BigDecimal::from(f as i64))
                } else {
                    None
                }
            });

        debug!("Parsed generic update from {}: pool={}, tvl={:?}", source, pool_address, tvl_usd);

        Ok(PoolUpdate {
            pool_address,
            chain_id: 1,
            current_tick: None,
            sqrt_price_x96: None,
            liquidity: None,
            token0_price_usd: None,
            token1_price_usd: None,
            tvl_usd,
            fees_24h_usd: None,
            volume_24h_usd: None,
            timestamp: Utc::now(),
            source: source.to_string(),
        })
    }

    /// Subscribe to pool updates
    pub fn subscribe_to_updates(&self) -> broadcast::Receiver<PoolUpdate> {
        self.pool_updates.subscribe()
    }

    /// Start all configured DEX connections
    pub async fn start_all_connections(&self) -> Result<(), AppError> {
        let dex_names: Vec<String> = {
            let configs = self.configs.read().await;
            configs.keys().cloned().collect()
        };

        for dex_name in dex_names {
            self.start_dex_connection(&dex_name).await?;
            info!("Started DEX connection: {}", dex_name);
        }

        Ok(())
    }

    /// Stop all DEX connections
    pub async fn stop_all_connections(&self) -> Result<(), AppError> {
        let mut connections = self.active_connections.write().await;
        for (dex_name, active) in connections.iter_mut() {
            if *active {
                *active = false;
                info!("Stopping DEX connection: {}", dex_name);
            }
        }

        Ok(())
    }

    /// Get connection status for all DEXs
    pub async fn get_connection_status(&self) -> HashMap<String, bool> {
        self.active_connections.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dex_websocket_client_creation() {
        let client = DexWebSocketClient::new();
        assert!(client.initialize_default_configs().await.is_ok());
    }

    #[tokio::test]
    async fn test_subscription_message_creation() {
        let thegraph_msg = DexWebSocketClient::create_subscription_message(&SubscriptionFormat::TheGraph);
        assert!(thegraph_msg.is_ok());

        let alchemy_msg = DexWebSocketClient::create_subscription_message(&SubscriptionFormat::Alchemy);
        assert!(alchemy_msg.is_ok());
    }
}
