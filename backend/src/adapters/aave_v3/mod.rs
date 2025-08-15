// Modular Aave V3 Adapter - Main implementation
use async_trait::async_trait;
use alloy::primitives::{Address, U256};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;

// Internal modules
pub mod contracts;
pub mod chain_config;
pub mod chains;
pub mod multi_chain_config;
use contracts::*;
use chain_config::ChainConfig;
use chains::get_chain_config;
use multi_chain_config::*;

// External dependencies
use crate::adapters::traits::{DeFiAdapter, Position, AdapterError};
use crate::blockchain::ethereum_client::EthereumClient;
use crate::services::aave_price_service::{AavePriceService, PriceData};
use crate::risk::calculators::aave_v3::{AaveV3RiskCalculator, AaveRiskAssessment};



/// Main Aave V3 Adapter with modular architecture
pub struct AaveV3Adapter {
    client: EthereumClient,
    chain_config: Box<dyn ChainConfig>,
    price_service: AavePriceService,
    risk_calculator: AaveV3RiskCalculator,
    reserve_cache: Arc<Mutex<Option<CachedAaveData>>>,
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    cache_duration: Duration,
}

impl AaveV3Adapter {
    /// Create a new Aave V3 adapter for the specified chain
    pub fn new(client: EthereumClient, chain_id: u64) -> Result<Self, AdapterError> {
        let chain_config = get_chain_config(chain_id)
            .ok_or_else(|| AdapterError::UnsupportedChain(format!("Chain {} not supported", chain_id)))?;

        let price_service = AavePriceService::new(
            client.clone(),
            chain_config.oracle_address(),
        );

        let risk_calculator = AaveV3RiskCalculator::new();

        Ok(Self {
            client,
            chain_config,
            price_service,
            risk_calculator,
            reserve_cache: Arc::new(Mutex::new(None)),
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            cache_duration: Duration::from_secs(1800), // 30 minutes
        })
    }

    /// Get the chain configuration
    pub fn chain_config(&self) -> &dyn ChainConfig {
        self.chain_config.as_ref()
    }

    /// Fetch all reserve data with caching
    pub async fn fetch_all_reserves(&self) -> Result<HashMap<Address, AaveReserveData>, AdapterError> {
        // Check cache first
        {
            let cache = self.reserve_cache.lock().unwrap();
            if let Some(cached) = cache.as_ref() {
                if cached.timestamp.elapsed().unwrap_or(Duration::MAX) < self.cache_duration {
                    return Ok(cached.data.clone());
                }
            }
        }

        tracing::info!("Fetching fresh reserve data for chain {}", self.chain_config.chain_id());

        // Get all reserves
        let pool_address = self.chain_config.pool_address();
        let provider = self.client.provider().clone();
        let reserves_result = {
            let pool = contracts::IAavePoolV3::new(pool_address, provider);
            pool.getReservesList().call().await
        };

        let reserves = match reserves_result {
            Ok(reserves) => reserves,
            Err(e) => return Err(AdapterError::ContractError(format!("Reserves fetch failed: {}", e))),
        };

        let mut reserve_data = HashMap::new();
        let mut price_requests = Vec::new();

        // Collect all assets for batch price fetching
        for &reserve in &reserves._0 {
            price_requests.push(reserve);
        }

        // Batch fetch prices
        let prices = self.price_service.get_prices(&price_requests).await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to batch fetch prices: {}", e);
                HashMap::new()
            });

        // Process each reserve
        for &reserve in &reserves._0 {
            match self.fetch_reserve_data(reserve, &prices).await {
                Ok(data) => {
                    reserve_data.insert(reserve, data);
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch data for reserve {:?}: {}", reserve, e);
                }
            }
        }

        // Update cache
        {
            let mut cache = self.reserve_cache.lock().unwrap();
            *cache = Some(CachedAaveData {
                data: reserve_data.clone(),
                timestamp: SystemTime::now(),
            });
        }

        tracing::info!("Successfully cached {} reserves for chain {}", reserve_data.len(), self.chain_config.chain_id());
        Ok(reserve_data)
    }

    /// Fetch individual reserve data
    async fn fetch_reserve_data(
        &self,
        asset: Address,
        prices: &HashMap<Address, PriceData>,
    ) -> Result<AaveReserveData, AdapterError> {
        // Get reserve configuration
        let data_provider_address = self.chain_config.data_provider_address();
        let provider = self.client.provider().clone();
        let config_result = {
            let data_provider = contracts::IAaveProtocolDataProvider::new(data_provider_address, provider.clone());
            data_provider.getReserveConfigurationData(asset).call().await
        };

        let config = match config_result {
            Ok(config) => config,
            Err(e) => return Err(AdapterError::ContractError(format!("Config fetch failed: {}", e))),
        };

        // Get token addresses
        let addresses_result = {
            let data_provider = contracts::IAaveProtocolDataProvider::new(data_provider_address, provider.clone());
            data_provider.getReserveTokensAddresses(asset).call().await
        };

        let addresses = match addresses_result {
            Ok(addresses) => addresses,
            Err(e) => return Err(AdapterError::ContractError(format!("Addresses fetch failed: {}", e))),
        };

        // Get token metadata
        let token = IERC20Metadata::new(asset, self.client.provider());
        let (symbol, name, decimals) = match timeout(
            Duration::from_secs(10),
            async {
                let symbol_result = token.symbol().call().await?;
                let name_result = token.name().call().await?;
                let decimals_result = token.decimals().call().await?;
                Ok::<_, alloy::contract::Error>((
                    symbol_result._0,
                    name_result._0,
                    decimals_result._0
                ))
            }
        ).await {
            Ok(Ok((symbol, name, decimals))) => (symbol, name, decimals),
            Ok(Err(e)) => {
                tracing::warn!("Failed to fetch token metadata for {:?}: {}", asset, e);
                (format!("{:?}", asset), format!("Token {:?}", asset), 18)
            }
            Err(_) => {
                tracing::warn!("Token metadata fetch timeout for {:?}", asset);
                (format!("{:?}", asset), format!("Token {:?}", asset), 18)
            }
        };

        // Get current rates from pool
        let pool_address = self.chain_config.pool_address();
        let provider_clone = provider.clone();
        let reserve_data_result = {
            let pool = contracts::IAavePoolV3::new(pool_address, provider_clone);
            pool.getReserveData(asset).call().await
        };

        let (liquidity_rate, variable_borrow_rate, stable_borrow_rate, liquidity_index, variable_borrow_index) = 
            match reserve_data_result {
                Ok(data) => (
                    data.currentLiquidityRate,
                    data.currentVariableBorrowRate,
                    data.currentStableBorrowRate,
                    data.liquidityIndex,
                    data.variableBorrowIndex,
                ),
                Err(e) => {
                    tracing::warn!("Failed to fetch reserve data for {:?}: {}", asset, e);
                    (0u128, 0u128, 0u128, 0u128, 0u128)
                }
            };

        // Get price
        let price_usd = prices.get(&asset)
            .map(|p| p.price_usd)
            .unwrap_or(1.0);

        Ok(AaveReserveData {
            asset_address: asset,
            symbol,
            name,
            decimals,
            a_token_address: addresses.aTokenAddress,
            stable_debt_token_address: addresses.stableDebtTokenAddress,
            variable_debt_token_address: addresses.variableDebtTokenAddress,
            current_liquidity_rate: U256::from(liquidity_rate),
            current_variable_borrow_rate: U256::from(variable_borrow_rate),
            current_stable_borrow_rate: U256::from(stable_borrow_rate),
            liquidity_index: U256::from(liquidity_index),
            variable_borrow_index: U256::from(variable_borrow_index),
            ltv: config.ltv.to::<u64>(),
            liquidation_threshold: config.liquidationThreshold.to::<u64>(),
            liquidation_bonus: config.liquidationBonus.to::<u64>(),
            reserve_factor: config.reserveFactor.to::<u64>(),
            usage_as_collateral_enabled: config.usageAsCollateralEnabled,
            borrowing_enabled: config.borrowingEnabled,
            stable_borrow_rate_enabled: config.stableBorrowRateEnabled,
            is_active: config.isActive,
            is_frozen: config.isFrozen,
            price_usd,
            last_updated: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        })
    }

    /// Get user positions with caching
    pub async fn get_user_positions(&self, user: Address) -> Result<AaveAccountSummary, AdapterError> {
        // Check cache first
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&user) {
                if cached.timestamp.elapsed().unwrap_or(Duration::MAX) < Duration::from_secs(300) {
                    return Ok(cached.positions.clone());
                }
            }
        }

        tracing::info!("Fetching fresh position data for user {:?} on chain {}", user, self.chain_config.chain_id());

        // Get user account data using Protocol Data Provider (more reliable)
        let data_provider_address = self.chain_config.data_provider_address();
        let provider = self.client.provider().clone();
        
        // First, get all reserves to iterate through
        let reserves_result = {
            let data_provider = contracts::IAaveProtocolDataProvider::new(data_provider_address, provider.clone());
            data_provider.getAllReservesTokens().call().await
        };
        
        let reserves = match reserves_result {
            Ok(data) => data._0,
            Err(e) => return Err(AdapterError::ContractError(format!("Failed to get reserves: {}", e))),
        };
        
        // Initialize account summary values
        let mut total_collateral_usd = 0.0;
        let mut total_debt_usd = 0.0;
        let mut positions = Vec::new();

        // Get price data for USD calculations
        let price_data = self.price_service.get_prices(&reserves.iter().map(|(_, addr)| *addr).collect::<Vec<_>>()).await
            .unwrap_or_default();
        
        // Iterate through all reserves to check user positions
        let data_provider = contracts::IAaveProtocolDataProvider::new(data_provider_address, provider.clone());
        
        for (symbol, asset_address) in &reserves {
            // Get user reserve data for this asset
            let user_reserve_result = data_provider.getUserReserveData(*asset_address, user).call().await;
            
            if let Ok(user_reserve) = user_reserve_result {
                // Check if user has any position in this reserve
                let has_supply = !user_reserve.currentATokenBalance.is_zero();
                let has_stable_debt = !user_reserve.currentStableDebt.is_zero();
                let has_variable_debt = !user_reserve.currentVariableDebt.is_zero();
                
                if has_supply || has_stable_debt || has_variable_debt {
                    // Get asset price
                    let price_usd = price_data.get(asset_address)
                        .map(|p| p.price_usd)
                        .unwrap_or(0.0);
                    
                    // Calculate USD values
                    let supply_balance_usd = if has_supply {
                        let balance = user_reserve.currentATokenBalance.to::<u128>() as f64 / 1e18;
                        balance * price_usd
                    } else { 0.0 };
                    
                    let stable_debt_usd = if has_stable_debt {
                        let debt = user_reserve.currentStableDebt.to::<u128>() as f64 / 1e18;
                        debt * price_usd
                    } else { 0.0 };
                    
                    let variable_debt_usd = if has_variable_debt {
                        let debt = user_reserve.currentVariableDebt.to::<u128>() as f64 / 1e18;
                        debt * price_usd
                    } else { 0.0 };
                    
                    // Add to totals
                    total_collateral_usd += supply_balance_usd;
                    total_debt_usd += stable_debt_usd + variable_debt_usd;
                    
                    // Create position object
                    let position = AaveUserPosition {
                        asset_address: *asset_address,
                        symbol: symbol.clone(),
                        a_token_balance: user_reserve.currentATokenBalance,
                        stable_debt: user_reserve.currentStableDebt,
                        variable_debt: user_reserve.currentVariableDebt,
                        usage_as_collateral_enabled: user_reserve.usageAsCollateralEnabled,
                        supply_apy: self.calculate_apy(user_reserve.liquidityRate),
                        variable_borrow_apy: 0.0, // Would need additional call to get this
                        stable_borrow_apy: self.calculate_apy(user_reserve.stableBorrowRate),
                        supply_balance_usd,
                        debt_balance_usd: stable_debt_usd + variable_debt_usd,
                        net_balance_usd: supply_balance_usd - (stable_debt_usd + variable_debt_usd),
                    };
                    
                    positions.push(position);
                }
            }
        }
        
        // Calculate derived values
        let available_borrows_usd = total_collateral_usd * 0.8 - total_debt_usd; // Simplified calculation
        let current_liquidation_threshold = 85.0; // Default value, would need Pool contract for exact value
        let loan_to_value = if total_collateral_usd > 0.0 { (total_debt_usd / total_collateral_usd) * 100.0 } else { 0.0 };
        let health_factor = if total_debt_usd > 0.0 { 
            (total_collateral_usd * current_liquidation_threshold / 100.0) / total_debt_usd 
        } else { 
            f64::INFINITY 
        };

        // The positions have already been collected in the loop above
        // Remove this duplicate loop that references old methods

        let account_summary = AaveAccountSummary {
            total_collateral_usd,
            total_debt_usd,
            available_borrows_usd,
            current_liquidation_threshold,
            loan_to_value,
            health_factor,
            net_worth_usd: total_collateral_usd - total_debt_usd,
            positions,
        };

        // Update cache
        {
            let mut cache = self.position_cache.lock().unwrap();
            cache.insert(user, CachedPositions {
                positions: account_summary.clone(),
                timestamp: SystemTime::now(),
            });
        }

        Ok(account_summary)
    }

    /// Fetch user data for a specific reserve
    async fn fetch_user_reserve_data(
        &self,
        asset: Address,
        user: Address,
        reserve_data: &AaveReserveData,
    ) -> Result<Option<AaveUserPosition>, AdapterError> {
        // Create data provider instance within the function
        let data_provider_address = self.chain_config.data_provider_address();
        let provider = self.client.provider().clone();
        let user_data_result = {
            let data_provider = contracts::IAaveProtocolDataProvider::new(data_provider_address, provider);
            data_provider.getUserReserveData(asset, user).call().await
        };

        let user_data = match user_data_result {
            Ok(data) => data,
            Err(e) => return Err(AdapterError::ContractError(format!("User reserve data fetch failed: {}", e))),
        };

        // Check if user has any position in this reserve
        if user_data.currentATokenBalance.is_zero() && 
           user_data.currentStableDebt.is_zero() && 
           user_data.currentVariableDebt.is_zero() {
            return Ok(None);
        }

        // Calculate USD values
        let token_decimals = 10_u128.pow(reserve_data.decimals as u32);
        let supply_balance = user_data.currentATokenBalance.to::<u128>() as f64 / token_decimals as f64;
        let stable_debt_balance = user_data.currentStableDebt.to::<u128>() as f64 / token_decimals as f64;
        let variable_debt_balance = user_data.currentVariableDebt.to::<u128>() as f64 / token_decimals as f64;

        let supply_balance_usd = supply_balance * reserve_data.price_usd;
        let stable_debt_usd = stable_debt_balance * reserve_data.price_usd;
        let variable_debt_usd = variable_debt_balance * reserve_data.price_usd;
        let debt_balance_usd = stable_debt_usd + variable_debt_usd;
        let net_balance_usd = supply_balance_usd - debt_balance_usd;

        // Convert rates to APY
        let supply_apy = self.calculate_apy(reserve_data.current_liquidity_rate);
        let variable_borrow_apy = self.calculate_apy(reserve_data.current_variable_borrow_rate);
        let stable_borrow_apy = self.calculate_apy(reserve_data.current_stable_borrow_rate);

        Ok(Some(AaveUserPosition {
            asset_address: asset,
            symbol: reserve_data.symbol.clone(),
            a_token_balance: user_data.currentATokenBalance,
            stable_debt: user_data.currentStableDebt,
            variable_debt: user_data.currentVariableDebt,
            usage_as_collateral_enabled: user_data.usageAsCollateralEnabled,
            supply_apy,
            variable_borrow_apy,
            stable_borrow_apy,
            supply_balance_usd,
            debt_balance_usd,
            net_balance_usd,
        }))
    }

    /// Convert Aave interest rate to APY
    fn calculate_apy(&self, rate: U256) -> f64 {
        // Aave rates are in ray format (27 decimals)
        let rate_decimal = rate.to::<u128>() as f64 / 1e27;
        
        // Convert to APY using compound interest formula
        // APY = (1 + rate/seconds_per_year)^seconds_per_year - 1
        let seconds_per_year = 365.25 * 24.0 * 3600.0;
        let apy = (1.0 + rate_decimal / seconds_per_year).powf(seconds_per_year) - 1.0;
        
        apy * 100.0 // Convert to percentage
    }

    /// Convert AaveAccountSummary to Position objects
    fn convert_to_positions(&self, user: Address, account: &AaveAccountSummary) -> Vec<Position> {
        let mut positions = Vec::new();
        
        for aave_position in &account.positions {
            // Create supply position if user has supplied tokens
            if !aave_position.a_token_balance.is_zero() {
                positions.push(Position {
                    id: format!("aave_v3_supply_{}_{:?}_{}", 
                        self.chain_config.chain_id(), 
                        aave_position.asset_address, 
                        user
                    ),
                    protocol: "aave_v3".to_string(),
                    position_type: "supply".to_string(),
                    pair: format!("{}/USD", aave_position.symbol),
                    value_usd: aave_position.supply_balance_usd,
                    pnl_usd: 0.0, // TODO: Calculate actual P&L
                    pnl_percentage: 0.0, // TODO: Calculate actual P&L percentage
                    risk_score: 20, // TODO: Calculate actual risk score
                    metadata: serde_json::json!({
                        "usage_as_collateral_enabled": aave_position.usage_as_collateral_enabled,
                        "a_token_address": aave_position.asset_address,
                        "supply_apy": aave_position.supply_apy,
                        "token_symbol": aave_position.symbol,
                        "balance": aave_position.a_token_balance.to_string(),
                        "balance_usd": aave_position.supply_balance_usd,
                    }),
                    last_updated: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }

            // Create borrow positions if user has debt
            if !aave_position.variable_debt.is_zero() {
                let debt_ratio = aave_position.variable_debt.to::<u128>() as f64 / 
                    (aave_position.variable_debt.to::<u128>() as f64 + aave_position.stable_debt.to::<u128>() as f64).max(1.0);
                let debt_value_usd = aave_position.debt_balance_usd * debt_ratio;
                
                positions.push(Position {
                    id: format!("aave_v3_borrow_variable_{}_{:?}_{}", 
                        self.chain_config.chain_id(), 
                        aave_position.asset_address, 
                        user
                    ),
                    protocol: "aave_v3".to_string(),
                    position_type: "borrow".to_string(),
                    pair: format!("{}/USD", aave_position.symbol),
                    value_usd: debt_value_usd,
                    pnl_usd: 0.0, // TODO: Calculate actual P&L
                    pnl_percentage: 0.0, // TODO: Calculate actual P&L percentage
                    risk_score: 40, // TODO: Calculate actual risk score
                    metadata: serde_json::json!({
                        "debt_type": "variable",
                        "borrow_apy": aave_position.variable_borrow_apy,
                        "token_symbol": aave_position.symbol,
                        "balance": aave_position.variable_debt.to_string(),
                        "balance_usd": debt_value_usd,
                    }),
                    last_updated: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }

            if !aave_position.stable_debt.is_zero() {
                let debt_ratio = aave_position.stable_debt.to::<u128>() as f64 / 
                    (aave_position.variable_debt.to::<u128>() as f64 + aave_position.stable_debt.to::<u128>() as f64).max(1.0);
                let debt_value_usd = aave_position.debt_balance_usd * debt_ratio;
                
                positions.push(Position {
                    id: format!("aave_v3_borrow_stable_{}_{:?}_{}", 
                        self.chain_config.chain_id(), 
                        aave_position.asset_address, 
                        user
                    ),
                    protocol: "aave_v3".to_string(),
                    position_type: "borrow".to_string(),
                    pair: format!("{}/USD", aave_position.symbol),
                    value_usd: debt_value_usd,
                    pnl_usd: 0.0, // TODO: Calculate actual P&L
                    pnl_percentage: 0.0, // TODO: Calculate actual P&L percentage
                    risk_score: 35, // TODO: Calculate actual risk score
                    metadata: serde_json::json!({
                        "debt_type": "stable",
                        "borrow_apy": aave_position.stable_borrow_apy,
                        "token_symbol": aave_position.symbol,
                        "balance": aave_position.stable_debt.to_string(),
                        "balance_usd": debt_value_usd,
                    }),
                    last_updated: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                });
            }
        }

        positions
    }

    /// Calculate risk assessment for account
    pub fn calculate_risk_assessment(&self, account: &AaveAccountSummary) -> AaveRiskAssessment {
        self.risk_calculator.calculate_risk(account)
    }
}

#[async_trait]
impl DeFiAdapter for AaveV3Adapter {
    fn protocol_name(&self) -> &'static str {
        "aave_v3"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        let account = self.get_user_positions(address).await?;
        Ok(self.convert_to_positions(address, &account))
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        // Check if the contract address matches any of our known Aave V3 contracts
        contract_address == self.chain_config.pool_address() ||
        contract_address == self.chain_config.data_provider_address() ||
        contract_address == self.chain_config.oracle_address()
    }
    
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // Calculate average risk score from all positions
        let total_risk: u32 = positions.iter().map(|p| p.risk_score as u32).sum();
        let avg_risk = (total_risk / positions.len() as u32) as u8;
        
        Ok(avg_risk)
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // Return the current USD value of the position
        Ok(position.value_usd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_supported_chains() {
        let supported_chains = vec![1, 137, 43114, 42161, 10];
        
        for chain_id in supported_chains {
            let config = get_chain_config(chain_id);
            assert!(config.is_some(), "Chain {} should be supported", chain_id);
        }
        
        // Test unsupported chain
        assert!(get_chain_config(99999).is_none());
    }
    
    #[test]
    fn test_apy_calculation() {
        let adapter = AaveV3Adapter::new(
            todo!("Mock EthereumClient"), 
            1
        ).unwrap();
        
        // Test APY calculation with known values
        let rate_5_percent = U256::from_str("1585489599188229325").unwrap(); // ~5% APY in ray format
        let apy = adapter.calculate_apy(rate_5_percent);
        
        // APY should be close to 5%
        assert!((apy - 5.0).abs() < 0.1);
    }
}
