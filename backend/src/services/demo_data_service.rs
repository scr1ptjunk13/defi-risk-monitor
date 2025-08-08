use crate::error::AppError;
use crate::models::Position;
use crate::services::{GraphService, BlockchainService};
use bigdecimal::BigDecimal;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use reqwest::Client;
use std::str::FromStr;

#[derive(Clone)]
pub struct DemoDataService {
    _graph_service: GraphService,
    _blockchain_service: BlockchainService,
    client: Client,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct UniswapPosition {
    id: String,
    owner: String,
    pool: Pool,
    #[serde(rename = "tickLower")]
    tick_lower: String,
    #[serde(rename = "tickUpper")]
    tick_upper: String,
    liquidity: String,
    #[serde(rename = "depositedToken0")]
    deposited_token0: String,
    #[serde(rename = "depositedToken1")]
    deposited_token1: String,
    #[serde(rename = "withdrawnToken0")]
    withdrawn_token0: String,
    #[serde(rename = "withdrawnToken1")]
    withdrawn_token1: String,
    #[serde(rename = "collectedFeesToken0")]
    collected_fees_token0: String,
    #[serde(rename = "collectedFeesToken1")]
    collected_fees_token1: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Pool {
    id: String,
    token0: Token,
    token1: Token,
    #[serde(rename = "feeTier")]
    fee_tier: String,
    #[serde(rename = "totalValueLockedUSD")]
    tvl_usd: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Token {
    id: String,
    symbol: String,
    name: String,
    decimals: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PositionsResponse {
    positions: Vec<UniswapPosition>,
}

impl DemoDataService {
    pub fn new(blockchain_service: BlockchainService) -> Self {
        Self {
            _graph_service: GraphService::new(),
            _blockchain_service: blockchain_service,
            client: Client::new(),
        }
    }

    /// Get real Uniswap V3 positions from famous DeFi addresses
    pub async fn get_demo_positions(&self) -> Result<Vec<Position>, AppError> {
        let demo_addresses = vec![
            // Famous DeFi addresses with known Uniswap positions
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Vitalik
            "0x47ac0Fb4F2D84898e4D9E7b4DaB3C24507a6D503", // Binance Hot Wallet
            "0x8EB8a3b98659Cce290402893d0123abb75E3ab28", // Avalanche Bridge
            "0x40ec5B33f54e0E8A33A975908C5BA1c14e5BbbDf", // Polygon Bridge
            "0x1a9C8182C09F50C8318d769245beA52c32BE35BC", // Large LP provider
        ];

        let mut all_positions = Vec::new();

        for address in demo_addresses {
            match self.fetch_positions_for_address(address).await {
                Ok(positions) => {
                    println!("✅ Found {} positions for address {}", positions.len(), address);
                    all_positions.extend(positions);
                }
                Err(e) => {
                    println!("⚠️ Failed to fetch positions for {}: {}", address, e);
                    // Continue with other addresses
                }
            }

            // Limit to first 10 positions for demo
            if all_positions.len() >= 10 {
                break;
            }
        }

        // If no real positions found, create realistic mock positions
        if all_positions.is_empty() {
            println!("📝 No real positions found, creating realistic demo positions...");
            return self.create_realistic_demo_positions().await;
        }

        Ok(all_positions)
    }

    async fn fetch_positions_for_address(&self, owner_address: &str) -> Result<Vec<Position>, AppError> {
        let query = format!(
            r#"
            query GetPositions($owner: String!) {{
                positions(
                    where: {{ owner: $owner, liquidity_gt: "0" }}
                    first: 5
                    orderBy: liquidity
                    orderDirection: desc
                ) {{
                    id
                    owner
                    pool {{
                        id
                        token0 {{
                            id
                            symbol
                            name
                            decimals
                        }}
                        token1 {{
                            id
                            symbol
                            name
                            decimals
                        }}
                        feeTier
                        totalValueLockedUSD
                    }}
                    tickLower
                    tickUpper
                    liquidity
                    depositedToken0
                    depositedToken1
                    withdrawnToken0
                    withdrawnToken1
                    collectedFeesToken0
                    collectedFeesToken1
                }}
            }}
            "#
        );

        let mut variables = HashMap::new();
        variables.insert("owner".to_string(), serde_json::Value::String(owner_address.to_lowercase()));

        let graph_query = serde_json::json!({
            "query": query,
            "variables": variables
        });

        let response = self.client
            .post("https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3")
            .json(&graph_query)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Graph API request failed: {}", e)))?;

        let graph_response: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Failed to parse Graph response: {}", e)))?;

        let positions_data = graph_response
            .get("data")
            .and_then(|d| d.get("positions"))
            .and_then(|p| p.as_array())
            .ok_or_else(|| AppError::ExternalApiError("No positions data found".to_string()))?;

        let mut positions = Vec::new();

        for pos_data in positions_data {
            if let Ok(position) = self.convert_graph_position_to_model(pos_data).await {
                positions.push(position);
            }
        }

        Ok(positions)
    }

    async fn convert_graph_position_to_model(&self, pos_data: &serde_json::Value) -> Result<Position, AppError> {
        let pool = pos_data.get("pool").ok_or_else(|| AppError::ExternalApiError("Missing pool data".to_string()))?;
        let token0 = pool.get("token0").ok_or_else(|| AppError::ExternalApiError("Missing token0".to_string()))?;
        let token1 = pool.get("token1").ok_or_else(|| AppError::ExternalApiError("Missing token1".to_string()))?;

        let pool_address = pool.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let _token0_symbol = token0.get("symbol").and_then(|v| v.as_str()).unwrap_or("TOKEN0");
        let _token1_symbol = token1.get("symbol").and_then(|v| v.as_str()).unwrap_or("TOKEN1");

        // Calculate position value from liquidity and pool data
        let liquidity_str = pos_data.get("liquidity").and_then(|v| v.as_str()).unwrap_or("0");
        let liquidity = liquidity_str.parse::<f64>().unwrap_or(0.0);
        
        // Estimate USD value (simplified calculation)
        let estimated_value = if liquidity > 0.0 {
            // Use pool TVL as a basis for estimation
            let pool_tvl = pool.get("totalValueLockedUSD")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1000000.0);
            
            // Rough estimation: position value as fraction of pool TVL
            (liquidity / 1000000.0) * pool_tvl / 100.0 // Very rough approximation
        } else {
            1000.0 // Default demo value
        };

        Ok(Position {
            id: uuid::Uuid::new_v4(),
            user_address: "demo_user".to_string(),
            protocol: "uniswap_v3".to_string(),
            pool_address: pool_address.to_string(),
            token0_address: token0.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            token1_address: token1.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            token0_amount: BigDecimal::from_str(&format!("{}", estimated_value / 2.0)).unwrap_or_else(|_| BigDecimal::from(1000)),
            token1_amount: BigDecimal::from_str(&format!("{}", estimated_value / 2.0)).unwrap_or_else(|_| BigDecimal::from(1000)),
            liquidity: BigDecimal::from_str(&liquidity_str).unwrap_or_else(|_| BigDecimal::from(1000000)),
            tick_lower: pos_data.get("tickLower").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(-887220),
            tick_upper: pos_data.get("tickUpper").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(887220),
            fee_tier: pool.get("feeTier").and_then(|v| v.as_str()).and_then(|s| s.parse().ok()).unwrap_or(3000),
            chain_id: 1, // Ethereum mainnet
            entry_token0_price_usd: Some(BigDecimal::from(1)),
            entry_token1_price_usd: Some(BigDecimal::from(1)),
            entry_timestamp: Some(Utc::now()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        })
    }

    /// Create realistic demo positions if no real ones are found
    async fn create_realistic_demo_positions(&self) -> Result<Vec<Position>, AppError> {
        let demo_positions = vec![
            // USDC/WETH 0.05% - Most popular pool
            self.create_demo_position(
                "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
                "0xA0b86a33E6441E8C8C7014b5C1D2664B3c2Eb16e", // USDC
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
                "USDC", "WETH",
                50000.0, // $50k position
                500, // 0.05% fee
            ),
            // USDC/USDT 0.01% - Stablecoin pair
            self.create_demo_position(
                "0x3416cF6C708Da44DB2624D63ea0AAef7113527C6",
                "0xA0b86a33E6441E8C8C7014b5C1D2664B3c2Eb16e", // USDC
                "0xdAC17F958D2ee523a2206206994597C13D831ec7", // USDT
                "USDC", "USDT",
                25000.0, // $25k position
                100, // 0.01% fee
            ),
            // WETH/WBTC 0.3% - Blue chip pair
            self.create_demo_position(
                "0xCBCdF9626bC03E24f779434178A73a0B4bad62eD",
                "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
                "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", // WBTC
                "WETH", "WBTC",
                75000.0, // $75k position
                3000, // 0.3% fee
            ),
        ];

        Ok(demo_positions)
    }

    fn create_demo_position(
        &self,
        pool_address: &str,
        token0_address: &str,
        token1_address: &str,
        _token0_symbol: &str,
        _token1_symbol: &str,
        usd_value: f64,
        fee_tier: i32,
    ) -> Position {
        Position {
            id: uuid::Uuid::new_v4(),
            user_address: "demo_user".to_string(),
            protocol: "uniswap_v3".to_string(),
            pool_address: pool_address.to_string(),
            token0_address: token0_address.to_string(),
            token1_address: token1_address.to_string(),
            token0_amount: BigDecimal::from_str(&format!("{}", usd_value / 2.0)).unwrap_or_else(|_| BigDecimal::from(1000)),
            token1_amount: BigDecimal::from_str(&format!("{}", usd_value / 2.0)).unwrap_or_else(|_| BigDecimal::from(1000)),
            liquidity: BigDecimal::from_str(&format!("{}", usd_value * 1000.0)).unwrap_or_else(|_| BigDecimal::from(1000000)),
            tick_lower: -887220, // Wide range
            tick_upper: 887220,
            fee_tier,
            chain_id: 1, // Ethereum mainnet
            entry_token0_price_usd: Some(BigDecimal::from(1)),
            entry_token1_price_usd: Some(BigDecimal::from(1)),
            entry_timestamp: Some(Utc::now()),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }

    /// Get real-time prices for positions (prices stored separately, not in Position model)
    pub async fn get_position_prices(&self, positions: &[Position]) -> Result<Vec<(BigDecimal, BigDecimal)>, AppError> {
        let mut prices = Vec::new();
        
        for position in positions {
            // Get real-time prices for both tokens
            let token0_price = self.get_token_price(&position.token0_address).await
                .unwrap_or_else(|_| BigDecimal::from(1));
            let token1_price = self.get_token_price(&position.token1_address).await
                .unwrap_or_else(|_| BigDecimal::from(1));
            
            prices.push((token0_price, token1_price));
        }
        
        Ok(prices)
    }

    async fn get_token_price(&self, token_address: &str) -> Result<BigDecimal, AppError> {
        // Use CoinGecko API for real prices
        let coingecko_id = self.get_coingecko_id(token_address);
        
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd",
            coingecko_id
        );

        let response: serde_json::Value = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Price API failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AppError::ExternalApiError(format!("Price parsing failed: {}", e)))?;

        let price = response
            .get(&coingecko_id)
            .and_then(|p| p.get("usd"))
            .and_then(|p| p.as_f64())
            .unwrap_or(1.0);

        Ok(BigDecimal::from_str(&price.to_string()).unwrap_or_else(|_| BigDecimal::from(1)))
    }

    fn get_coingecko_id(&self, token_address: &str) -> String {
        match token_address.to_lowercase().as_str() {
            "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "ethereum".to_string(),
            "0xa0b86a33e6441e8c8c7014b5c1d2664b3c2eb16e" => "usd-coin".to_string(),
            "0xdac17f958d2ee523a2206206994597c13d831ec7" => "tether".to_string(),
            "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => "wrapped-bitcoin".to_string(),
            _ => "ethereum".to_string(), // Default fallback
        }
    }
}
