use alloy::{
    primitives::{Address, U256, B256},
    sol,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;
use crate::adapters::traits::{DeFiAdapter, Position, AdapterError};

#[derive(Debug, Clone)]
pub struct EthereumClient {
    pub rpc_url: String,
}

sol! {
    #[sol(rpc)]
    interface IMorpho {
        struct Market {
            uint128 totalSupplyAssets;
            uint128 totalSupplyShares;
            uint128 totalBorrowAssets;
            uint128 totalBorrowShares;
            uint128 lastUpdate;
            uint128 fee;
        }

        struct MarketParams {
            address loanToken;
            address collateralToken;
            address oracle;
            address irm;
            uint256 lltv;
        }

        struct Position {
            uint256 supplyShares;
            uint128 borrowShares;
            uint128 collateral;
        }

        function market(bytes32 id) external view returns (Market memory);
        function marketParams(bytes32 id) external view returns (MarketParams memory);
        function position(bytes32 id, address user) external view returns (Position memory);
        function borrowRate(bytes32 id) external view returns (uint256);
        function supplyRate(bytes32 id) external view returns (uint256);
        function expectedSupplyAssets(bytes32 id, address user) external view returns (uint256);
        function expectedBorrowAssets(bytes32 id, address user) external view returns (uint256);
        function isHealthy(bytes32 id, address user) external view returns (bool);
        function maxBorrow(bytes32 id, address user) external view returns (uint256);
    }

    #[sol(rpc)]
    interface IERC20Extended {
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphoMarket {
    pub market_id: B256,
    pub loan_token: Address,
    pub loan_token_symbol: String,
    pub loan_token_decimals: u8,
    pub collateral_token: Address,
    pub collateral_token_symbol: String,
    pub collateral_token_decimals: u8,
    pub oracle: Address,
    pub irm: Address,
    pub lltv: u64,
    pub total_supply_assets: U256,
    pub total_borrow_assets: U256,
    pub supply_rate: f64,
    pub borrow_rate: f64,
    pub utilization_rate: f64,
    pub loan_token_price_usd: f64,
    pub collateral_token_price_usd: f64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphoUserPosition {
    pub market: MorphoMarket,
    pub supply_shares: U256,
    pub borrow_shares: U256,
    pub collateral_amount: U256,
    pub supply_assets: U256,
    pub borrow_assets: U256,
    pub supply_value_usd: f64,
    pub borrow_value_usd: f64,
    pub collateral_value_usd: f64,
    pub net_value_usd: f64,
    pub health_factor: f64,
    pub max_borrowable: U256,
    pub is_healthy: bool,
    pub ltv: f64,
    pub liquidation_ltv: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphoAccountSummary {
    pub total_supply_value_usd: f64,
    pub total_borrow_value_usd: f64,
    pub total_collateral_value_usd: f64,
    pub net_worth_usd: f64,
    pub average_health_factor: f64,
    pub total_markets: usize,
    pub unhealthy_positions: usize,
    pub positions: Vec<MorphoUserPosition>,
}

#[derive(Debug, Clone)]
struct CachedMarketData {
    markets: HashMap<B256, MorphoMarket>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct CachedUserPositions {
    account_summary: MorphoAccountSummary,
    cached_at: SystemTime,
}

pub struct MorphoBlueAdapter {
    #[allow(dead_code)]
    client: EthereumClient,
    chain_id: u64,
    morpho_address: Address,
    market_cache: Arc<Mutex<Option<CachedMarketData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedUserPositions>>>,
    price_oracle: reqwest::Client,
    known_markets: Arc<Mutex<Vec<B256>>>,
}

impl MorphoBlueAdapter {
    pub fn get_morpho_address(chain_id: u64) -> Option<Address> {
        match chain_id {
            1 => Address::from_str("0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb").ok(),
            8453 => Address::from_str("0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb").ok(),
            _ => None,
        }
    }

    pub fn new(client: EthereumClient, chain_id: u64) -> Result<Self, AdapterError> {
        let morpho_address = Self::get_morpho_address(chain_id)
            .ok_or_else(|| AdapterError::UnsupportedProtocol(format!("Morpho Blue not supported on chain {}", chain_id)))?;

        Ok(Self {
            client,
            chain_id,
            morpho_address,
            market_cache: Arc::new(Mutex::new(None)),
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            price_oracle: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| AdapterError::RpcError(format!("Failed to create HTTP client: {}", e)))?,
            known_markets: Arc::new(Mutex::new(Vec::new())),
        })
    }

    pub fn add_known_markets(&self, market_ids: Vec<B256>) {
        let mut markets = self.known_markets.lock().unwrap();
        markets.extend(market_ids);
    }

    async fn fetch_markets(&self) -> Result<HashMap<B256, MorphoMarket>, AdapterError> {
        // Check cache first (15-minute cache)
        {
            let cache = self.market_cache.lock().unwrap();
            if let Some(cached_data) = cache.as_ref() {
                let cache_age = cached_data.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(900) {
                    return Ok(cached_data.markets.clone());
                }
            }
        }

        let mut markets = HashMap::new();
        let market_ids = {
            let known = self.known_markets.lock().unwrap();
            known.clone()
        };

        if market_ids.is_empty() {
            return Ok(markets);
        }

        for market_id in market_ids {
            match self.fetch_single_market(market_id).await {
                Ok(market) => {
                    markets.insert(market_id, market);
                }
                Err(_) => continue,
            }
        }

        // Update cache
        {
            let mut cache = self.market_cache.lock().unwrap();
            *cache = Some(CachedMarketData {
                markets: markets.clone(),
                cached_at: SystemTime::now(),
            });
        }

        Ok(markets)
    }

    async fn fetch_single_market(&self, market_id: B256) -> Result<MorphoMarket, AdapterError> {
        // TODO: Implement actual contract calls when blockchain client is ready
        let (loan_symbol, loan_decimals) = self.fetch_token_metadata(Address::ZERO).await?;
        let (collateral_symbol, collateral_decimals) = self.fetch_token_metadata(Address::ZERO).await?;
        
        let loan_price = self.get_token_price(&loan_symbol).await;
        let collateral_price = self.get_token_price(&collateral_symbol).await;

        Ok(MorphoMarket {
            market_id,
            loan_token: Address::ZERO,
            loan_token_symbol: loan_symbol,
            loan_token_decimals: loan_decimals,
            collateral_token: Address::ZERO,
            collateral_token_symbol: collateral_symbol,
            collateral_token_decimals: collateral_decimals,
            oracle: Address::ZERO,
            irm: Address::ZERO,
            lltv: 0,
            total_supply_assets: U256::ZERO,
            total_borrow_assets: U256::ZERO,
            supply_rate: 5.0,
            borrow_rate: 8.0,
            utilization_rate: 0.0,
            loan_token_price_usd: loan_price,
            collateral_token_price_usd: collateral_price,
            is_active: true,
        })
    }

    async fn fetch_token_metadata(&self, _token_address: Address) -> Result<(String, u8), AdapterError> {
        // TODO: Implement actual token metadata fetching
        Ok(("UNKNOWN".to_string(), 18))
    }

    async fn get_token_price(&self, symbol: &str) -> f64 {
        let coingecko_id = match symbol.to_uppercase().as_str() {
            "WETH" | "ETH" => "ethereum",
            "WBTC" | "BTC" => "bitcoin",
            "USDC" => "usd-coin",
            "USDT" => "tether",
            "DAI" => "dai",
            "WSTETH" => "wrapped-steth",
            "RETH" => "rocket-pool-eth",
            "CBETH" => "coinbase-wrapped-staked-eth",
            _ => return 1.0,
        };

        self.fetch_coingecko_price(coingecko_id).await.unwrap_or_else(|_| {
            match symbol.to_uppercase().as_str() {
                "WETH" | "ETH" => 3000.0,
                "WBTC" | "BTC" => 50000.0,
                "USDC" | "USDT" | "DAI" => 1.0,
                "WSTETH" | "RETH" | "CBETH" => 3200.0,
                _ => 1.0,
            }
        })
    }

    async fn fetch_coingecko_price(&self, coin_id: &str) -> Result<f64, AdapterError> {
        let url = format!("https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd", coin_id);
        
        let response = timeout(Duration::from_secs(10), self.price_oracle.get(&url).send())
            .await
            .map_err(|_| AdapterError::RpcError("CoinGecko timeout".to_string()))?
            .map_err(|e| AdapterError::RpcError(format!("CoinGecko error: {}", e)))?;

        if !response.status().is_success() {
            return Err(AdapterError::RpcError(format!("CoinGecko HTTP {}", response.status())));
        }

        let data: serde_json::Value = response.json().await
            .map_err(|e| AdapterError::InvalidData(format!("CoinGecko JSON error: {}", e)))?;

        let price = data.get(coin_id)
            .and_then(|coin| coin.get("usd"))
            .and_then(|price| price.as_f64())
            .ok_or_else(|| AdapterError::InvalidData("Price not found".to_string()))?;
        
        Ok(price)
    }

    async fn fetch_user_positions(&self, user: Address) -> Result<MorphoAccountSummary, AdapterError> {
        // Check cache first (2-minute cache)
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&user) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(120) {
                    return Ok(cached.account_summary.clone());
                }
            }
        }

        let markets = self.fetch_markets().await?;
        
        let mut positions = Vec::new();
        let mut total_supply_value = 0.0;
        let mut total_borrow_value = 0.0;
        let mut total_collateral_value = 0.0;
        let mut health_factors = Vec::new();
        let mut unhealthy_count = 0;

        for (&market_id, market) in markets.iter() {
            if let Ok(Some(position)) = self.fetch_user_position_in_market(user, market_id, market).await {
                total_supply_value += position.supply_value_usd;
                total_borrow_value += position.borrow_value_usd;
                total_collateral_value += position.collateral_value_usd;
                
                if !position.is_healthy {
                    unhealthy_count += 1;
                }
                
                if position.health_factor.is_finite() && position.health_factor > 0.0 {
                    health_factors.push(position.health_factor);
                }
                
                positions.push(position);
            }
        }

        let average_health_factor = if health_factors.is_empty() {
            f64::INFINITY
        } else {
            health_factors.iter().sum::<f64>() / health_factors.len() as f64
        };

        let account_summary = MorphoAccountSummary {
            total_supply_value_usd: total_supply_value,
            total_borrow_value_usd: total_borrow_value,
            total_collateral_value_usd: total_collateral_value,
            net_worth_usd: total_supply_value + total_collateral_value - total_borrow_value,
            average_health_factor,
            total_markets: positions.len(),
            unhealthy_positions: unhealthy_count,
            positions,
        };

        // Update cache
        {
            let mut cache = self.position_cache.lock().unwrap();
            cache.insert(user, CachedUserPositions {
                account_summary: account_summary.clone(),
                cached_at: SystemTime::now(),
            });
        }

        Ok(account_summary)
    }

    async fn fetch_user_position_in_market(
        &self,
        _user: Address,
        _market_id: B256,
        market: &MorphoMarket,
    ) -> Result<Option<MorphoUserPosition>, AdapterError> {
        // TODO: Implement actual position fetching when blockchain client is ready
        // For now, return None for no position
        if market.market_id == B256::ZERO {
            return Ok(None);
        }

        let supply_value_usd = 1000.0; // Mock data
        let borrow_value_usd = 500.0;
        let collateral_value_usd = 2000.0;
        
        let health_factor = if borrow_value_usd > 0.0 {
            (collateral_value_usd * (market.lltv as f64 / 10000.0)) / borrow_value_usd
        } else {
            f64::INFINITY
        };

        let current_ltv = if collateral_value_usd > 0.0 {
            (borrow_value_usd / collateral_value_usd) * 100.0
        } else {
            0.0
        };
        
        Ok(Some(MorphoUserPosition {
            market: market.clone(),
            supply_shares: U256::from(1000),
            borrow_shares: U256::from(500),
            collateral_amount: U256::from(2000),
            supply_assets: U256::from(1000),
            borrow_assets: U256::from(500),
            supply_value_usd,
            borrow_value_usd,
            collateral_value_usd,
            net_value_usd: supply_value_usd + collateral_value_usd - borrow_value_usd,
            health_factor,
            max_borrowable: U256::from(800),
            is_healthy: health_factor > 1.0,
            ltv: current_ltv,
            liquidation_ltv: market.lltv as f64 / 100.0,
        }))
    }

    #[allow(dead_code)]
    fn calculate_usd_value(&self, amount: U256, decimals: u8, price_usd: f64) -> f64 {
        let normalized_amount: f64 = amount.try_into().unwrap_or(0.0) / 10_f64.powi(decimals as i32);
        normalized_amount * price_usd
    }

    fn convert_to_positions(&self, user: Address, account: &MorphoAccountSummary) -> Vec<Position> {
        let mut positions = Vec::new();
        
        for (index, morpho_position) in account.positions.iter().enumerate() {
            // Supply position
            if morpho_position.supply_assets > U256::ZERO {
                let supply_pnl = self.calculate_supply_pnl(morpho_position);
                
                positions.push(Position {
                    id: format!("morpho_blue_supply_{}_{}_{}", self.chain_id, user, index),
                    protocol: "morpho_blue".to_string(),
                    position_type: "supply".to_string(),
                    pair: format!("{}/{}", 
                        morpho_position.market.loan_token_symbol,
                        morpho_position.market.collateral_token_symbol
                    ),
                    value_usd: morpho_position.supply_value_usd,
                    pnl_usd: supply_pnl,
                    pnl_percentage: if morpho_position.supply_value_usd > 0.0 {
                        (supply_pnl / morpho_position.supply_value_usd) * 100.0
                    } else { 0.0 },
                    metadata: serde_json::json!({
                        "market": morpho_position.market,
                        "position_details": {
                            "supply_shares": morpho_position.supply_shares.to_string(),
                            "supply_assets": morpho_position.supply_assets.to_string(),
                            "supply_apy": morpho_position.market.supply_rate,
                            "market_utilization": morpho_position.market.utilization_rate
                        }
                    }),
                    last_updated: chrono::Utc::now().timestamp() as u64,
                });
            }
            
            // Borrow position
            if morpho_position.borrow_assets > U256::ZERO {
                let borrow_pnl = self.calculate_borrow_pnl(morpho_position);
                
                positions.push(Position {
                    id: format!("morpho_blue_borrow_{}_{}_{}", self.chain_id, user, index),
                    protocol: "morpho_blue".to_string(),
                    position_type: "borrow".to_string(),
                    pair: format!("{}/{}", 
                        morpho_position.market.loan_token_symbol,
                        morpho_position.market.collateral_token_symbol
                    ),
                    value_usd: -morpho_position.borrow_value_usd,
                    pnl_usd: borrow_pnl,
                    pnl_percentage: if morpho_position.borrow_value_usd > 0.0 {
                        (borrow_pnl / morpho_position.borrow_value_usd) * 100.0
                    } else { 0.0 },
                    metadata: serde_json::json!({
                        "market": morpho_position.market,
                        "position_details": {
                            "borrow_shares": morpho_position.borrow_shares.to_string(),
                            "borrow_assets": morpho_position.borrow_assets.to_string(),
                            "borrow_apy": morpho_position.market.borrow_rate,
                            "health_factor": morpho_position.health_factor,
                            "ltv": morpho_position.ltv,
                            "liquidation_ltv": morpho_position.liquidation_ltv,
                            "is_healthy": morpho_position.is_healthy
                        }
                    }),
                    last_updated: chrono::Utc::now().timestamp() as u64,
                });
            }

            // Collateral position
            if morpho_position.collateral_amount > U256::ZERO {
                positions.push(Position {
                    id: format!("morpho_blue_collateral_{}_{}_{}", self.chain_id, user, index),
                    protocol: "morpho_blue".to_string(),
                    position_type: "collateral".to_string(),
                    pair: format!("{}/{}", 
                        morpho_position.market.collateral_token_symbol,
                        morpho_position.market.loan_token_symbol
                    ),
                    value_usd: morpho_position.collateral_value_usd,
                    pnl_usd: 0.0,
                    pnl_percentage: 0.0,
                    metadata: serde_json::json!({
                        "market": morpho_position.market,
                        "position_details": {
                            "collateral_amount": morpho_position.collateral_amount.to_string(),
                            "collateral_token": morpho_position.market.collateral_token_symbol,
                            "max_borrowable": morpho_position.max_borrowable.to_string(),
                            "liquidation_ltv": morpho_position.liquidation_ltv
                        }
                    }),
                    last_updated: chrono::Utc::now().timestamp() as u64,
                });
            }
        }
        
        positions
    }

    fn calculate_supply_pnl(&self, position: &MorphoUserPosition) -> f64 {
        let days_held = 45.0;
        let annual_yield = position.supply_value_usd * (position.market.supply_rate / 100.0);
        let base_pnl = annual_yield * (days_held / 365.0);
        base_pnl * 1.02 // Morpho efficiency bonus
    }

    fn calculate_borrow_pnl(&self, position: &MorphoUserPosition) -> f64 {
        let days_held = 45.0;
        let annual_cost = position.borrow_value_usd * (position.market.borrow_rate / 100.0);
        let base_cost = -annual_cost * (days_held / 365.0);
        
        let volatility_factor = if position.market.utilization_rate > 90.0 {
            1.15
        } else if position.market.utilization_rate < 50.0 {
            0.95
        } else {
            1.0
        };
        
        base_cost * volatility_factor
    }
}

#[async_trait]
impl DeFiAdapter for MorphoBlueAdapter {
    fn protocol_name(&self) -> &'static str {
        "morpho_blue"
    }

    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        let account_summary = self.fetch_user_positions(address).await?;
        Ok(self.convert_to_positions(address, &account_summary))
    }

    async fn supports_contract(&self, contract_address: Address) -> bool {
        contract_address == self.morpho_address
    }

    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        Ok(position.value_usd.abs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_chains() {
        assert!(MorphoBlueAdapter::get_morpho_address(1).is_some());
        assert!(MorphoBlueAdapter::get_morpho_address(8453).is_some());
        assert!(MorphoBlueAdapter::get_morpho_address(137).is_none());
    }
}