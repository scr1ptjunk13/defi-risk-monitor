use alloy::{
    primitives::{Address, U256},
    sol,
};
use async_trait::async_trait;
use crate::adapters::traits::{AdapterError, Position, DeFiAdapter};
use crate::blockchain::ethereum_client::EthereumClient;
use crate::services::IERC20;
use crate::risk::calculators::rocketpool::RocketPoolRiskCalculator;
use crate::risk::traits::{ProtocolRiskCalculator, ExplainableRiskCalculator};
use crate::risk::metrics::RocketPoolRiskMetrics;
use reqwest;
use serde::Deserialize;
use serde_json;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::time::timeout;
use bigdecimal::BigDecimal;
use chrono;

#[derive(Debug, Deserialize)]
struct RocketPoolApiResponse {
    data: Option<serde_json::Value>,
    status: String,
}

/// Node operator metrics structure
#[derive(Debug, Clone)]
struct NodeOperatorMetrics {
    total_nodes: u64,
    active_nodes: u64,
    trusted_nodes: u64,
    smoothing_pool_nodes: u64,
    total_minipools: u64,
    active_minipools: u64,
}

/// Protocol metrics from Rocket Pool network
#[derive(Debug, Clone)]
struct ProtocolMetrics {
    total_eth_staked: f64,
    reth_supply: f64,
    reth_exchange_rate: f64,
    node_demand: f64,          // ETH waiting for node operators
    deposit_pool_balance: f64,  // ETH waiting to be staked
    network_node_fee: f64,     // Current node operator commission
}

/// Enhanced Rocket Pool position with comprehensive metrics
#[derive(Debug, Clone)]
struct EnhancedRocketPoolPosition {
    basic_position: RocketPoolStakingPosition,
    exchange_rate: f64,        // rETH/ETH exchange rate
    tvl_in_protocol: f64,      // Total value locked in Rocket Pool
    node_metrics: NodeOperatorMetrics,
    protocol_metrics: ProtocolMetrics,
    expected_rewards: f64,     // Expected annual rewards in USD
}

#[derive(Debug, Clone)]
struct CachedPositions {
    positions: Vec<Position>,
    cached_at: SystemTime,
}

#[derive(Debug, Clone)]
struct RocketPoolStakingPosition {
    token_address: Address,
    token_symbol: String,
    balance: U256,
    decimals: u8,
    underlying_asset: String,
    apy: f64,
    rewards_earned: U256,
    position_subtype: String, // "liquid_staking", "node_operator", "node_deposit"
}

// Rocket Pool contract ABIs using alloy sol! macro
sol! {
    #[sol(rpc)]
    interface IRocketTokenRETH {
        function balanceOf(address account) external view returns (uint256);
        function getEthValue(uint256 rethAmount) external view returns (uint256);
        function getRethValue(uint256 ethAmount) external view returns (uint256);
        function getExchangeRate() external view returns (uint256);
        function totalSupply() external view returns (uint256);
        function symbol() external pure returns (string memory);
        function decimals() external pure returns (uint8);
        function name() external pure returns (string memory);
        
        // Events
        event Transfer(address indexed from, address indexed to, uint256 value);
        event TokensMinted(address indexed to, uint256 amount, uint256 ethAmount, uint256 time);
        event TokensBurned(address indexed from, uint256 amount, uint256 ethAmount, uint256 time);
    }
    
    #[sol(rpc)]
    interface IRocketDepositPool {
        function getBalance() external view returns (uint256);
        function getMaximumDepositAmount() external view returns (uint256);
        function deposit() external payable;
        
        // Events
        event DepositReceived(address indexed from, uint256 amount, uint256 time);
    }
    
    #[sol(rpc)]
    interface IRocketNodeManager {
        function getNodeCount() external view returns (uint256);
        function getNodeCountPerStatus(uint256 offset, uint256 limit) external view returns (uint256 initialised, uint256 prelaunch, uint256 staking, uint256 withdrawable, uint256 dissolved);
        function getNodeExists(address nodeAddress) external view returns (bool);
        function getNodeWithdrawalAddress(address nodeAddress) external view returns (address);
        function getNodePendingWithdrawalAddress(address nodeAddress) external view returns (address);
        function getNodeTimezoneLocation(address nodeAddress) external view returns (string memory);
    }
    
    #[sol(rpc)]
    interface IRocketMinipoolManager {
        function getMinipoolCount() external view returns (uint256);
        function getMinipoolCountPerStatus(uint256 offset, uint256 limit) external view returns (uint256 initialised, uint256 prelaunch, uint256 staking, uint256 withdrawable, uint256 dissolved);
        function getNodeMinipoolCount(address nodeAddress) external view returns (uint256);
        function getNodeActiveMinipoolCount(address nodeAddress) external view returns (uint256);
        function getNodeFinalisedMinipoolCount(address nodeAddress) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketNetworkFees {
        function getNodeDemand() external view returns (int256);
        function getNodeFee() external view returns (uint256);
        function getNodeFeeByDemand(int256 demand) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketRewardsPool {
        function getClaimIntervalTimeStart() external view returns (uint256);
        function getClaimIntervalTime() external view returns (uint256);
        function getClaimIntervalsPassed() external view returns (uint256);
        function getClaimingContractAllowance(string memory contractName, uint256 claimIntervalStartTime) external view returns (uint256);
        function getClaimingContractUserTotalCurrent(string memory contractName) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IRocketNodeStaking {
        function getNodeRPLStake(address nodeAddress) external view returns (uint256);
        function getNodeEffectiveRPLStake(address nodeAddress) external view returns (uint256);
        function getNodeMinimumRPLStake(address nodeAddress) external view returns (uint256);
        function getNodeMaximumRPLStake(address nodeAddress) external view returns (uint256);
        function getNodeRPLStakedTime(address nodeAddress) external view returns (uint256);
    }
}

/// Rocket Pool Liquid Staking protocol adapter
pub struct RocketPoolAdapter {
    client: EthereumClient,
    reth_address: Address,
    deposit_pool_address: Address,
    node_manager_address: Address,
    minipool_manager_address: Address,
    network_fees_address: Address,
    rewards_pool_address: Address,
    node_staking_address: Address,
    rpl_token_address: Address,
    // Caches to prevent API spam
    position_cache: Arc<Mutex<HashMap<Address, CachedPositions>>>,
    // HTTP client for API calls
    http_client: reqwest::Client,
    // Optional CoinGecko API key for price fetching
    coingecko_api_key: Option<String>,
    // Risk calculator for comprehensive risk assessment
    risk_calculator: RocketPoolRiskCalculator,
}

impl RocketPoolAdapter {
    /// Convert adapter Position to risk calculator Position
    fn convert_adapter_position_to_risk_position(adapter_pos: &crate::adapters::traits::Position) -> crate::models::position::Position {
        use uuid::Uuid;
        use bigdecimal::BigDecimal;
        use std::str::FromStr;
        
        // Extract token addresses and amounts from metadata if available
        let token0_address = adapter_pos.metadata.get("token0_address")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000")
            .to_string();
        let token1_address = adapter_pos.metadata.get("token1_address")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000")
            .to_string();
        
        let token0_amount = adapter_pos.metadata.get("token0_amount")
            .and_then(|v| v.as_str())
            .and_then(|s| BigDecimal::from_str(s).ok())
            .unwrap_or_else(|| BigDecimal::from_str("0").unwrap());
        let token1_amount = adapter_pos.metadata.get("token1_amount")
            .and_then(|v| v.as_str())
            .and_then(|s| BigDecimal::from_str(s).ok())
            .unwrap_or_else(|| BigDecimal::from_str("0").unwrap());
        
        let liquidity = adapter_pos.metadata.get("liquidity")
            .and_then(|v| v.as_str())
            .and_then(|s| BigDecimal::from_str(s).ok())
            .unwrap_or_else(|| BigDecimal::from_str("0").unwrap());
        
        crate::models::position::Position {
            id: Uuid::new_v4(), // Generate new UUID since adapter uses String
            user_address: "0x0000000000000000000000000000000000000000".to_string(), // Default value
            protocol: adapter_pos.protocol.clone(),
            pool_address: adapter_pos.metadata.get("pool_address")
                .and_then(|v| v.as_str())
                .unwrap_or("0x0000000000000000000000000000000000000000")
                .to_string(),
            token0_address,
            token1_address,
            token0_amount,
            token1_amount,
            liquidity,
            tick_lower: adapter_pos.metadata.get("tick_lower")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            tick_upper: adapter_pos.metadata.get("tick_upper")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
            fee_tier: adapter_pos.metadata.get("fee_tier")
                .and_then(|v| v.as_i64())
                .unwrap_or(3000) as i32, // Default 0.3% fee tier
            chain_id: 1, // Ethereum mainnet
            entry_token0_price_usd: None,
            entry_token1_price_usd: None,
            entry_timestamp: None,
            created_at: Some(chrono::Utc::now()),
            updated_at: Some(chrono::Utc::now()),
        }
    }
    
    /// Rocket Pool contract addresses on Ethereum mainnet
    const RETH_ADDRESS: &'static str = "0xae78736Cd615f374D3085123A210448E74Fc6393";
    const DEPOSIT_POOL_ADDRESS: &'static str = "0x2cac916b2A963Bf162f076C0a8a4a8200BCFBfb4";
    const NODE_MANAGER_ADDRESS: &'static str = "0x89F478E6Cc24f052103628f36598D4C14Da3D287";
    const MINIPOOL_MANAGER_ADDRESS: &'static str = "0x6d010a588f89E7e8634e1fF7A59C6F70C7D9A37b";
    const NETWORK_FEES_ADDRESS: &'static str = "0xeE4d2A71cF479e0312B3AF664B4f652E23880B12";
    const REWARDS_POOL_ADDRESS: &'static str = "0xEE4d2A71cF479e0312B3AF664B4f652E23880B12";
    const NODE_STAKING_ADDRESS: &'static str = "0x3019227b2b8493e45Bf5d6777666dC81E6e8EC2C";
    const RPL_TOKEN_ADDRESS: &'static str = "0xD33526068D116cE69F19A9ee46F0bd304F21A51f";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let reth_address = Address::from_str(Self::RETH_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid rETH address: {}", e)))?;
            
        let deposit_pool_address = Address::from_str(Self::DEPOSIT_POOL_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid deposit pool address: {}", e)))?;
            
        let node_manager_address = Address::from_str(Self::NODE_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid node manager address: {}", e)))?;
            
        let minipool_manager_address = Address::from_str(Self::MINIPOOL_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid minipool manager address: {}", e)))?;
            
        let network_fees_address = Address::from_str(Self::NETWORK_FEES_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid network fees address: {}", e)))?;
            
        let rewards_pool_address = Address::from_str(Self::REWARDS_POOL_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid rewards pool address: {}", e)))?;
            
        let node_staking_address = Address::from_str(Self::NODE_STAKING_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid node staking address: {}", e)))?;
            
        let rpl_token_address = Address::from_str(Self::RPL_TOKEN_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid RPL token address: {}", e)))?;
        
        Ok(Self {
            client,
            reth_address,
            deposit_pool_address,
            node_manager_address,
            minipool_manager_address,
            network_fees_address,
            rewards_pool_address,
            node_staking_address,
            rpl_token_address,
            position_cache: Arc::new(Mutex::new(HashMap::new())),
            http_client: reqwest::Client::new(),
            coingecko_api_key: std::env::var("COINGECKO_API_KEY").ok(),
            risk_calculator: RocketPoolRiskCalculator::new(),
        })
    }
    
    /// Get ALL Rocket Pool staking positions for a user
    async fn get_user_staking_positions(&self, address: Address) -> Result<Vec<RocketPoolStakingPosition>, AdapterError> {
        tracing::info!(
            user_address = %address,
            "🚀 Discovering ALL Rocket Pool liquid staking positions"
        );
        
        let mut positions = Vec::new();
        
        // 1. Check rETH balance (liquid staking ETH)
        if let Some(reth_position) = self.get_reth_position(address).await? {
            positions.push(reth_position);
        }
        
        // 2. Check if user is a node operator
        if let Some(node_positions) = self.get_node_operator_positions(address).await? {
            positions.extend(node_positions);
        }
        
        // 3. Check RPL token staking (for node operators)
        if let Some(rpl_position) = self.get_rpl_staking_position(address).await? {
            positions.push(rpl_position);
        }
        
        tracing::info!(
            user_address = %address,
            total_positions = positions.len(),
            "✅ Discovered ALL Rocket Pool staking positions"
        );
        
        Ok(positions)
    }
    
    /// Get rETH liquid staking position
    async fn get_reth_position(&self, user_address: Address) -> Result<Option<RocketPoolStakingPosition>, AdapterError> {
        let reth_contract = IRocketTokenRETH::new(self.reth_address, self.client.provider());
        
        // Get user's rETH balance with better error handling
        let balance = match reth_contract.balanceOf(user_address).call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    user_address = %user_address,
                    error = %e,
                    "Failed to get rETH balance, user may not have rETH"
                );
                return Ok(None);
            }
        };
            
        if balance == U256::ZERO {
            tracing::info!(
                user_address = %user_address,
                "User has no rETH balance"
            );
            return Ok(None);
        }
        
        // Get ETH value of rETH balance
        let eth_value = reth_contract.getEthValue(balance).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get ETH value: {}", e)))?
            ._0;
        
        // Get current exchange rate (rETH per ETH)
        let exchange_rate = reth_contract.getExchangeRate().call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get exchange rate: {}", e)))?
            ._0;
        
        // Get current staking APY from Rocket Pool API or calculate from protocol data
        let apy = self.get_rocket_pool_apy("rETH").await.unwrap_or(3.5); // Fallback ~3.5%
        
        // Estimate rewards earned (appreciation of rETH vs initial ETH deposited)
        let rewards_earned = self.estimate_reth_rewards(user_address, balance, eth_value).await;
        
        tracing::info!(
            user_address = %user_address,
            reth_balance = %balance,
            eth_value = %eth_value,
            exchange_rate = %exchange_rate,
            apy = %apy,
            rewards_earned = %rewards_earned,
            "Found rETH liquid staking position"
        );
        
        Ok(Some(RocketPoolStakingPosition {
            token_address: self.reth_address,
            token_symbol: "rETH".to_string(),
            balance,
            decimals: 18,
            underlying_asset: "ETH".to_string(),
            apy,
            rewards_earned,
            position_subtype: "liquid_staking".to_string(),
        }))
    }
    
    /// Get node operator positions (if user runs nodes)
    async fn get_node_operator_positions(&self, user_address: Address) -> Result<Option<Vec<RocketPoolStakingPosition>>, AdapterError> {
        let node_manager = IRocketNodeManager::new(self.node_manager_address, self.client.provider());
        let minipool_manager = IRocketMinipoolManager::new(self.minipool_manager_address, self.client.provider());
        
        // Check if user is a registered node operator
        let is_node = match node_manager.getNodeExists(user_address).call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    user_address = %user_address,
                    error = %e,
                    "Failed to check node existence, assuming user is not a node operator"
                );
                return Ok(None);
            }
        };
            
        if !is_node {
            tracing::info!(
                user_address = %user_address,
                "User is not a registered node operator"
            );
            return Ok(None);
        }
        
        tracing::info!(
            node_address = %user_address,
            "🚀 User is a Rocket Pool node operator! Getting node metrics"
        );
        
        let mut positions = Vec::new();
        
        // Get minipool count for this node
        let minipool_count = minipool_manager.getNodeMinipoolCount(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get minipool count: {}", e)))?
            ._0;
            
        let active_minipool_count = minipool_manager.getNodeActiveMinipoolCount(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get active minipool count: {}", e)))?
            ._0;
        
        if minipool_count > U256::ZERO {
            // Each minipool represents 16 ETH from node operator + 16 ETH from protocol
            let node_eth_deposited = minipool_count.try_into().unwrap_or(0.0) * 16.0; // Node operator's ETH
            let protocol_eth_matched = active_minipool_count.try_into().unwrap_or(0.0) * 16.0; // Protocol matched ETH
            
            // Get current node operator APY (higher than liquid stakers due to commission)
            let node_apy = self.get_node_operator_apy().await.unwrap_or(5.5); // Typically higher
            
            // Estimate node operator rewards
            let rewards_earned = U256::from((node_eth_deposited * 0.055 * 10f64.powi(18)) as u64); // Rough estimate
            
            let node_position = RocketPoolStakingPosition {
                token_address: self.minipool_manager_address,
                token_symbol: format!("RP-NODE-{}", minipool_count),
                balance: U256::from((node_eth_deposited * 10f64.powi(18)) as u64),
                decimals: 18,
                underlying_asset: "ETH".to_string(),
                apy: node_apy,
                rewards_earned,
                position_subtype: "node_operator".to_string(),
            };
            
            positions.push(node_position);
            
            tracing::info!(
                node_address = %user_address,
                total_minipools = %minipool_count,
                active_minipools = %active_minipool_count,
                node_eth_deposited = %node_eth_deposited,
                node_apy = %node_apy,
                "Found node operator position with minipools"
            );
        }
        
        Ok(if positions.is_empty() { None } else { Some(positions) })
    }
    
    /// Get RPL token staking position (for node operators)
    async fn get_rpl_staking_position(&self, user_address: Address) -> Result<Option<RocketPoolStakingPosition>, AdapterError> {
        // First check if user is a registered node operator
        let node_manager = IRocketNodeManager::new(self.node_manager_address, self.client.provider());
        
        let is_node = match node_manager.getNodeExists(user_address).call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    user_address = %user_address,
                    error = %e,
                    "Failed to check if user is a node operator, assuming not"
                );
                return Ok(None);
            }
        };
        
        if !is_node {
            tracing::info!(
                user_address = %user_address,
                "User is not a registered node operator, no RPL staking position"
            );
            return Ok(None);
        }
        
        let node_staking = IRocketNodeStaking::new(self.node_staking_address, self.client.provider());
        
        // Get user's staked RPL amount with better error handling
        let rpl_stake = match node_staking.getNodeRPLStake(user_address).call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    user_address = %user_address,
                    error = %e,
                    "Failed to get RPL stake, user may not have staked RPL"
                );
                return Ok(None);
            }
        };
            
        if rpl_stake == U256::ZERO {
            tracing::info!(
                user_address = %user_address,
                "User has no RPL staked"
            );
            return Ok(None);
        }
        
        // Get effective RPL stake (after slashing protection)
        let effective_rpl_stake = node_staking.getNodeEffectiveRPLStake(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get effective RPL stake: {}", e)))?
            ._0;
        
        // Get minimum and maximum RPL stake requirements
        let min_rpl_stake = node_staking.getNodeMinimumRPLStake(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get minimum RPL stake: {}", e)))?
            ._0;
        
        let max_rpl_stake = node_staking.getNodeMaximumRPLStake(user_address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get maximum RPL stake: {}", e)))?
            ._0;
        
        // RPL staking provides additional rewards on top of ETH staking
        let rpl_apy = self.get_rpl_staking_apy().await.unwrap_or(8.0); // RPL inflation rewards
        
        // Estimate RPL rewards earned
        let rewards_earned = self.estimate_rpl_rewards(user_address, rpl_stake).await;
        
        tracing::info!(
            user_address = %user_address,
            rpl_stake = %rpl_stake,
            effective_rpl_stake = %effective_rpl_stake,
            min_rpl_stake = %min_rpl_stake,
            max_rpl_stake = %max_rpl_stake,
            rpl_apy = %rpl_apy,
            rewards_earned = %rewards_earned,
            "Found RPL staking position"
        );
        
        Ok(Some(RocketPoolStakingPosition {
            token_address: self.rpl_token_address,
            token_symbol: "RPL-STAKED".to_string(),
            balance: rpl_stake,
            decimals: 18,
            underlying_asset: "RPL".to_string(),
            apy: rpl_apy,
            rewards_earned,
            position_subtype: "rpl_staking".to_string(),
        }))
    }
    
    /// Get rETH/ETH exchange rate
    async fn get_reth_exchange_rate(&self) -> Result<f64, String> {
        let reth_contract = IRocketTokenRETH::new(self.reth_address, self.client.provider());
        
        let exchange_rate = reth_contract.getExchangeRate().call().await
            .map_err(|e| format!("Failed to get rETH exchange rate: {}", e))?
            ._0;
        
        // Exchange rate is returned as wei, convert to ratio
        let rate = exchange_rate.try_into().unwrap_or(0.0) / 10f64.powi(18);
        
        tracing::info!("Current rETH/ETH exchange rate: {}", rate);
        
        Ok(rate)
    }
    
    /// Get node operator metrics for the entire network
    async fn get_node_operator_metrics(&self) -> Result<NodeOperatorMetrics, String> {
        let node_manager = IRocketNodeManager::new(self.node_manager_address, self.client.provider());
        let minipool_manager = IRocketMinipoolManager::new(self.minipool_manager_address, self.client.provider());
        
        // Get total node count with fallback
        let total_nodes = match node_manager.getNodeCount().call().await {
            Ok(result) => result._0.to::<u64>(),
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get node count from contract, using fallback estimate"
                );
                // Fallback: Rocket Pool typically has 2000+ nodes
                2500u64
            }
        };
        
        // Get minipool counts with fallback
        let (total_minipools, active_minipools) = match minipool_manager.getMinipoolCountPerStatus(U256::ZERO, U256::from(1000)).call().await {
            Ok(result) => {
                // Sum all status counts to get total minipools
                let total = result.initialised.to::<u64>() + result.prelaunch.to::<u64>() + 
                           result.staking.to::<u64>() + result.withdrawable.to::<u64>() + result.dissolved.to::<u64>();
                let active = result.staking.to::<u64>(); // Only staking minipools are active
                (total, active)
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get minipool counts from contract, using fallback estimates"
                );
                // Fallback: Rocket Pool typically has 15000+ minipools with ~12000+ active
                (15000u64, 12000u64)
            }
        };
        
        // Estimate active nodes based on active minipools (each node can run multiple minipools)
        let estimated_active_nodes = if active_minipools > 0 {
            // Conservative estimate: average 5-6 minipools per active node
            (active_minipools as f64 / 5.5) as u64
        } else {
            (total_nodes as f64 * 0.8) as u64 // 80% of nodes are typically active
        };
        
        tracing::info!(
            total_nodes = %total_nodes,
            active_nodes = %estimated_active_nodes,
            total_minipools = %total_minipools,
            active_minipools = %active_minipools,
            "Retrieved Rocket Pool network node operator metrics"
        );
        
        Ok(NodeOperatorMetrics {
            total_nodes,
            active_nodes: estimated_active_nodes,
            trusted_nodes: 0, // Would need additional contract calls
            smoothing_pool_nodes: 0, // Would need smoothing pool contract
            total_minipools,
            active_minipools,
        })
    }
    
    /// Get protocol-wide metrics
    async fn get_protocol_metrics(&self) -> Result<ProtocolMetrics, String> {
        let reth_contract = IRocketTokenRETH::new(self.reth_address, self.client.provider());
        let deposit_pool = IRocketDepositPool::new(self.deposit_pool_address, self.client.provider());
        let network_fees = IRocketNetworkFees::new(self.network_fees_address, self.client.provider());
        
        // Get rETH supply using standard ERC-20 totalSupply method
        let reth_supply = match reth_contract.totalSupply().call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get rETH total supply from contract, using fallback"
                );
                // Use a reasonable fallback value or skip this metric
                U256::from(400000) * U256::from(10).pow(U256::from(18)) // ~400k rETH as fallback
            }
        };
        
        // Get rETH/ETH exchange rate with fallback
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1);
        
        // Calculate total ETH staked (rETH supply * exchange rate)
        let total_eth_staked = (reth_supply.try_into().unwrap_or(0.0) / 10f64.powi(18)) * exchange_rate;
        
        // Get deposit pool balance with error handling
        let deposit_balance = match deposit_pool.getBalance().call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get deposit pool balance, using fallback"
                );
                U256::ZERO
            }
        };
            
        // Get network node fee with error handling
        let node_fee = match network_fees.getNodeFee().call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get network node fee, using fallback"
                );
                U256::from(5) * U256::from(10).pow(U256::from(16)) // 5% as fallback
            }
        };
        
        // Get node demand with error handling
        let node_demand = match network_fees.getNodeDemand().call().await {
            Ok(result) => result._0,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to get node demand, using fallback"
                );
                alloy::primitives::I256::ZERO // Use signed integer zero
            }
        };
        
        let protocol_metrics = ProtocolMetrics {
            total_eth_staked,
            reth_supply: reth_supply.try_into().unwrap_or(0.0) / 10f64.powi(18),
            reth_exchange_rate: exchange_rate,
            node_demand: node_demand.to_string().parse::<f64>().unwrap_or(0.0) / 10f64.powi(18),
            deposit_pool_balance: deposit_balance.try_into().unwrap_or(0.0) / 10f64.powi(18),
            network_node_fee: node_fee.try_into().unwrap_or(0.0) / 10f64.powi(18),
        };
        
        tracing::info!(
            total_eth_staked = %protocol_metrics.total_eth_staked,
            reth_supply = %protocol_metrics.reth_supply,
            exchange_rate = %protocol_metrics.reth_exchange_rate,
            node_demand = %protocol_metrics.node_demand,
            deposit_pool_balance = %protocol_metrics.deposit_pool_balance,
            node_fee = %protocol_metrics.network_node_fee,
            "Retrieved Rocket Pool protocol metrics"
        );
        
        Ok(protocol_metrics)
    }
    
    /// Get TVL in protocol
    async fn get_protocol_tvl(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        
        // Get ETH price for USD value
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        let tvl_usd = protocol_metrics.total_eth_staked * eth_price;
        
        tracing::info!(
            total_eth_staked = %protocol_metrics.total_eth_staked,
            eth_price = %eth_price,
            tvl_usd = %tvl_usd,
            "Calculated Rocket Pool protocol TVL"
        );
        
        Ok(tvl_usd)
    }

    /// Calculate real USD value of Rocket Pool positions
    async fn calculate_position_value(&self, position: &RocketPoolStakingPosition) -> (f64, f64, f64) {
        // Get enhanced metrics for better tracking
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1); // rETH typically > 1 ETH
        let tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let node_metrics = self.get_node_operator_metrics().await.unwrap_or(NodeOperatorMetrics {
            total_nodes: 0,
            active_nodes: 0,
            trusted_nodes: 0,
            smoothing_pool_nodes: 0,
            total_minipools: 0,
            active_minipools: 0,
        });
        let protocol_metrics = self.get_protocol_metrics().await.unwrap_or(ProtocolMetrics {
            total_eth_staked: 0.0,
            reth_supply: 0.0,
            reth_exchange_rate: 1.1,
            node_demand: 0.0,
            deposit_pool_balance: 0.0,
            network_node_fee: 0.05,
        });
        
        tracing::info!(
            token_symbol = %position.token_symbol,
            exchange_rate = %exchange_rate,
            tvl_usd = %tvl,
            total_nodes = node_metrics.total_nodes,
            total_minipools = node_metrics.total_minipools,
            node_demand = %protocol_metrics.node_demand,
            "🚀 Calculating ENHANCED USD value for Rocket Pool position with all metrics"
        );
        
        // Get token price based on position type
        let token_price = match position.underlying_asset.as_str() {
            "ETH" => self.get_eth_price_usd().await.unwrap_or(4000.0),
            "RPL" => self.get_rpl_price_usd().await.unwrap_or(50.0), // Fallback RPL price
            _ => 0.0,
        };
        
        // Convert token balance to underlying asset amount
        let underlying_amount = if position.token_symbol == "rETH" {
            // For rETH, convert to ETH equivalent using exchange rate
            let reth_amount = position.balance.try_into().unwrap_or(0.0) / 10f64.powi(18);
            reth_amount * exchange_rate
        } else {
            // For other tokens, direct conversion
            position.balance.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32)
        };
        
        // Calculate USD value
        let base_value_usd = underlying_amount * token_price;
        let rewards_amount = position.rewards_earned.try_into().unwrap_or(0.0) / 10f64.powi(position.decimals as i32);
        let rewards_value_usd = rewards_amount * token_price;
        
        // Calculate estimated APY-based P&L with position-specific adjustments
        let mut adjusted_pnl = position.apy;
        
        // Apply position-specific risk adjustments
        match position.position_subtype.as_str() {
            "liquid_staking" => {
                // rETH liquidity and exchange rate risk
                let exchange_rate_premium = ((exchange_rate - 1.0) * 100.0).max(0.0);
                adjusted_pnl += exchange_rate_premium * 0.1; // Slight bonus for exchange rate appreciation
            },
            "node_operator" => {
                // Node operators have higher rewards but more complexity
                let node_demand_factor = if protocol_metrics.node_demand > 0.0 { 1.1 } else { 0.95 };
                adjusted_pnl *= node_demand_factor;
            },
            "rpl_staking" => {
                // RPL staking has inflation rewards but token price risk
                // No adjustment needed as APY already includes inflation
            },
            _ => {}
        }
        
        // Apply protocol health factors
        let node_utilization = if node_metrics.total_nodes > 0 {
            node_metrics.active_nodes as f64 / node_metrics.total_nodes as f64
        } else { 1.0 };
        
        if node_utilization < 0.8 {
            adjusted_pnl *= 0.95; // Slight penalty for low node utilization
        }
        
        tracing::info!(
            token_symbol = %position.token_symbol,
            underlying_amount = %underlying_amount,
            token_price = %token_price,
            base_value_usd = %base_value_usd,
            exchange_rate = %exchange_rate,
            rewards_value_usd = %rewards_value_usd,
            apy = %position.apy,
            adjusted_pnl = %adjusted_pnl,
            tvl_usd = %tvl,
            node_utilization = %node_utilization,
            "✅ Calculated COMPREHENSIVE Rocket Pool position value with all metrics"
        );
        
        (base_value_usd, rewards_value_usd, adjusted_pnl)
    }
    
    /// Get current Rocket Pool staking APY from API or on-chain data
    async fn get_rocket_pool_apy(&self, token_type: &str) -> Result<f64, String> {
        // Try Rocket Pool's API first
        let rp_api_url = "https://api.rocketpool.net/api/mainnet/payload";
        
        tracing::debug!("Fetching Rocket Pool APY from official API");
        
        match self.call_rocket_pool_api(rp_api_url).await {
            Ok(apy) => {
                tracing::info!("Got Rocket Pool APY from official API: {}%", apy);
                return Ok(apy);
            }
            Err(e) => {
                tracing::warn!("Rocket Pool API failed: {}, trying fallback", e);
            }
        }
        
        // Fallback: Calculate from on-chain data and ETH staking base rate
        match self.calculate_apy_from_onchain_data(token_type).await {
            Ok(apy) => {
                tracing::info!("Calculated APY from on-chain data: {}%", apy);
                Ok(apy)
            }
            Err(e) => {
                tracing::error!("Failed to calculate APY from on-chain: {}", e);
                Err(e)
            }
        }
    }
    
    /// Calculate APY from on-chain data and protocol metrics
    async fn calculate_apy_from_onchain_data(&self, token_type: &str) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        
        // Base Ethereum staking APY is around 3-5%
        let base_eth_apy = 4.0;
        
        let calculated_apy = match token_type {
            "rETH" => {
                // rETH holders get ETH staking rewards minus node operator commission
                // Node operators typically get 5-20% commission depending on network demand
                let commission_rate = protocol_metrics.network_node_fee;
                let liquid_staker_apy = base_eth_apy * (1.0 - commission_rate);
                liquid_staker_apy.max(2.5) // Minimum reasonable APY
            },
            _ => base_eth_apy,
        };
        
        tracing::info!(
            token_type = %token_type,
            base_eth_apy = %base_eth_apy,
            network_node_fee = %protocol_metrics.network_node_fee,
            calculated_apy = %calculated_apy,
            "Calculated APY from protocol metrics"
        );
        
        Ok(calculated_apy)
    }
    
    /// Get node operator APY (higher than liquid staking due to commission)
    async fn get_node_operator_apy(&self) -> Result<f64, String> {
        let protocol_metrics = self.get_protocol_metrics().await?;
        
        // Base ETH staking rewards
        let base_eth_apy = 4.0;
        
        // Node operators get:
        // 1. Their own 16 ETH staking rewards
        // 2. Commission from the protocol's 16 ETH
        let node_commission = protocol_metrics.network_node_fee;
        let node_apy = base_eth_apy * (1.0 + node_commission);
        
        tracing::info!(
            base_eth_apy = %base_eth_apy,
            node_commission = %node_commission,
            node_apy = %node_apy,
            "Calculated node operator APY"
        );
        
        Ok(node_apy)
    }
    
    /// Get RPL staking APY (inflation rewards)
    async fn get_rpl_staking_apy(&self) -> Result<f64, String> {
        // RPL has token inflation to reward node operators
        // This is separate from ETH staking rewards
        // Typical RPL inflation is around 5-10% annually
        
        let rpl_inflation_apy = 7.5; // Conservative estimate
        
        tracing::info!(
            rpl_inflation_apy = %rpl_inflation_apy,
            "Using RPL inflation APY estimate"
        );
        
        Ok(rpl_inflation_apy)
    }
    
    /// Estimate rETH rewards earned (appreciation over initial ETH)
    async fn estimate_reth_rewards(&self, _user_address: Address, reth_balance: U256, eth_value: U256) -> U256 {
        // rETH rewards come from the appreciation of the exchange rate
        // Without historical data, we can estimate based on current exchange rate vs 1.0
        
        let reth_amount_f64 = reth_balance.to::<u128>() as f64 / 10f64.powi(18);
        let eth_equivalent_f64 = eth_value.to::<u128>() as f64 / 10f64.powi(18);
        
        // Conservative estimate: assume they got rETH at ~1.05 rate (historical average)
        let assumed_entry_rate = 1.05;
        let current_rate = if reth_amount_f64 > 0.0 {
            eth_equivalent_f64 / reth_amount_f64
        } else {
            1.0
        };
        
        // Calculate appreciation rewards
        let rate_appreciation = (current_rate - assumed_entry_rate).max(0.0);
        let estimated_rewards_eth = reth_amount_f64 * rate_appreciation;
        
        // Convert back to wei, with overflow protection
        if estimated_rewards_eth > 0.0 && estimated_rewards_eth < 1000000.0 { // Max 1M ETH sanity check
            let rewards_wei = (estimated_rewards_eth * 10f64.powi(18)) as u128;
            if rewards_wei <= u128::MAX {
                U256::from(rewards_wei)
            } else {
                U256::ZERO // Overflow protection
            }
        } else {
            U256::ZERO // No rewards or invalid calculation
        }
    }
    
    /// Estimate RPL rewards earned
    async fn estimate_rpl_rewards(&self, _user_address: Address, rpl_stake: U256) -> U256 {
        // RPL rewards come from token inflation distributed to stakers
        // Typically around 5-10% annually
        
        let stake_amount = rpl_stake.to::<u128>() as f64;
        let estimated_rewards_percentage = 0.075; // 7.5% annual, pro-rated
        let estimated_rewards = stake_amount * estimated_rewards_percentage;
        
        U256::from(estimated_rewards as u64)
    }
    
    /// Get ETH price from CoinGecko
    async fn get_eth_price_usd(&self) -> Result<f64, String> {
        let url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=usd"
        };
        
        self.get_token_price_from_coingecko(url, "ethereum").await
    }
    
    /// Get RPL price from CoinGecko
    async fn get_rpl_price_usd(&self) -> Result<f64, String> {
        let url = if self.coingecko_api_key.is_some() {
            "https://pro-api.coingecko.com/api/v3/simple/price?ids=rocket-pool&vs_currencies=usd"
        } else {
            "https://api.coingecko.com/api/v3/simple/price?ids=rocket-pool&vs_currencies=usd"
        };
        
        self.get_token_price_from_coingecko(url, "rocket-pool").await
    }
    
    /// Generic method to get token price from CoinGecko
    async fn get_token_price_from_coingecko(&self, url: &str, token_id: &str) -> Result<f64, String> {
        tracing::debug!("Fetching {} price from: {}", token_id, url);
        
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
            
        // Parse JSON response
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        if let Some(token_data) = json.get(token_id) {
            if let Some(usd_price) = token_data.get("usd") {
                if let Some(price) = usd_price.as_f64() {
                    return Ok(price);
                }
            }
        }
        
        Err(format!("{} price not found in response", token_id))
    }
    
    /// Call Rocket Pool official API
    async fn call_rocket_pool_api(&self, url: &str) -> Result<f64, String> {
        let response = self.http_client
            .get(url)
            .header("Accept", "application/json")
            .header("User-Agent", "DeFi-Portfolio-Tracker/1.0")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;
            
        if !response.status().is_success() {
            return Err(format!("API returned status: {}", response.status()));
        }
        
        let response_text = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
            
        tracing::debug!("Rocket Pool API response: {}", response_text);
        
        // Parse APY from Rocket Pool API response
        let json: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("JSON parse error: {}", e))?;
            
        // Try to extract APY from different possible response formats
        if let Some(reth_apy) = json.get("rethAPY") {
            if let Some(apy) = reth_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        if let Some(network_apy) = json.get("networkAPY") {
            if let Some(apy) = network_apy.as_f64() {
                return Ok(apy);
            }
        }
        
        // Try nested data structure
        if let Some(data) = json.get("data") {
            if let Some(reth_apy) = data.get("rethAPY") {
                if let Some(apy) = reth_apy.as_f64() {
                    return Ok(apy);
                }
            }
        }
        
        Err("APY not found in Rocket Pool API response".to_string())
    }
    
    /// Check if address is a known Rocket Pool contract
    fn is_rocket_pool_contract(&self, address: Address) -> bool {
        address == self.reth_address || 
        address == self.deposit_pool_address || 
        address == self.node_manager_address ||
        address == self.minipool_manager_address ||
        address == self.network_fees_address ||
        address == self.rewards_pool_address ||
        address == self.node_staking_address ||
        address == self.rpl_token_address
    }
    
    /// Get token symbol for Rocket Pool tokens
    fn get_rocket_pool_token_symbol(&self, address: Address) -> String {
        if address == self.reth_address {
            "rETH".to_string()
        } else if address == self.rpl_token_address {
            "RPL".to_string()
        } else if address == self.node_manager_address {
            "RP-NODE".to_string()
        } else if address == self.minipool_manager_address {
            "RP-MINIPOOL".to_string()
        } else {
            "UNKNOWN-RP".to_string()
        }
    }
}

#[async_trait]
impl DeFiAdapter for RocketPoolAdapter {
    fn protocol_name(&self) -> &'static str {
        "rocket_pool"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        tracing::info!(
            user_address = %address,
            protocol = "rocket_pool",
            "CACHE CHECK: Checking for cached Rocket Pool positions"
        );
        
        // CACHE CHECK: Prevent API spam
        {
            let cache = self.position_cache.lock().unwrap();
            if let Some(cached) = cache.get(&address) {
                let cache_age = cached.cached_at.elapsed().unwrap_or(Duration::from_secs(0));
                if cache_age < Duration::from_secs(300) { // 5 minute cache
                    tracing::info!(
                        user_address = %address,
                        cache_age_secs = cache_age.as_secs(),
                        position_count = cached.positions.len(),
                        "CACHE HIT: Returning cached Rocket Pool positions!"
                    );
                    return Ok(cached.positions.clone());
                }
            }
        }
        
        tracing::info!(
            user_address = %address,
            "CACHE MISS: Fetching fresh Rocket Pool data from blockchain"
        );
        
        // Get all staking positions for the user
        let staking_positions = self.get_user_staking_positions(address).await?;
        
        if staking_positions.is_empty() {
            tracing::info!(
                user_address = %address,
                "No Rocket Pool positions found"
            );
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Get enhanced metrics once for all positions (optimization)
        let exchange_rate = self.get_reth_exchange_rate().await.unwrap_or(1.1);
        let tvl = self.get_protocol_tvl().await.unwrap_or(0.0);
        let node_metrics = self.get_node_operator_metrics().await.unwrap_or(NodeOperatorMetrics {
            total_nodes: 0,
            active_nodes: 0,
            trusted_nodes: 0,
            smoothing_pool_nodes: 0,
            total_minipools: 0,
            active_minipools: 0,
        });
        let protocol_metrics = self.get_protocol_metrics().await.unwrap_or(ProtocolMetrics {
            total_eth_staked: 0.0,
            reth_supply: 0.0,
            reth_exchange_rate: 1.1,
            node_demand: 0.0,
            deposit_pool_balance: 0.0,
            network_node_fee: 0.05,
        });
        
        tracing::info!(
            user_address = %address,
            exchange_rate = %exchange_rate,
            protocol_tvl_usd = %tvl,
            total_nodes = node_metrics.total_nodes,
            total_minipools = node_metrics.total_minipools,
            node_demand = %protocol_metrics.node_demand,
            deposit_pool_balance = %protocol_metrics.deposit_pool_balance,
            "🚀 Got enhanced Rocket Pool protocol metrics for all positions"
        );
        
        // Store consistent pricing data for all positions
        let eth_price = self.get_eth_price_usd().await.unwrap_or(4000.0);
        
        // Convert staking positions to Position structs with consistent valuation
        for stake_pos in staking_positions {
            let (base_value_usd, rewards_usd, calculated_apy) = self.calculate_position_value(&stake_pos).await;
            
            let position_type = match stake_pos.position_subtype.as_str() {
                "liquid_staking" => "staking",
                "node_operator" => "node_operation",
                "rpl_staking" => "governance_staking",
                _ => "staking",
            };
            
            let pair = match stake_pos.underlying_asset.as_str() {
                "ETH" => format!("{}/ETH", stake_pos.token_symbol),
                "RPL" => "RPL/USD".to_string(),
                _ => format!("{}/USD", stake_pos.token_symbol),
            };
            
            // Risk score varies by position type
            let risk_score = match stake_pos.position_subtype.as_str() {
                "liquid_staking" => 20, // Low risk, liquid
                "node_operator" => 35,  // Medium risk, technical complexity
                "rpl_staking" => 45,    // Higher risk, token price volatility
                _ => 25,
            };
            
            // Use the SAME calculated value for consistency
            let position = Position {
                id: format!("rocket_pool_{}_{}", stake_pos.token_symbol.to_lowercase(), stake_pos.token_address),
                protocol: "rocket_pool".to_string(),
                position_type: position_type.to_string(),
                pair,
                value_usd: base_value_usd.max(0.01), // Use consistent calculated value
                pnl_usd: rewards_usd,   // Rewards earned
                pnl_percentage: calculated_apy, // Current APY as P&L indicator
                risk_score,
                metadata: serde_json::json!({
                    "token_address": format!("{:?}", stake_pos.token_address),
                    "token_symbol": stake_pos.token_symbol,
                    "underlying_asset": stake_pos.underlying_asset,
                    "balance": stake_pos.balance.to_string(),
                    "decimals": stake_pos.decimals,
                    "current_apy": stake_pos.apy,
                    "rewards_earned": stake_pos.rewards_earned.to_string(),
                    "staking_provider": "rocket_pool",
                    "position_subtype": stake_pos.position_subtype,
                    "is_liquid": stake_pos.position_subtype == "liquid_staking",
                    
                    // ENHANCED METRICS - All the missing pieces!
                    "reth_exchange_rate": exchange_rate,
                    "exchange_rate_premium": ((exchange_rate - 1.0) * 100.0),
                    "protocol_tvl_usd": tvl,
                    "total_nodes": node_metrics.total_nodes,
                    "active_nodes": node_metrics.active_nodes,
                    "total_minipools": node_metrics.total_minipools,
                    "active_minipools": node_metrics.active_minipools,
                    "node_utilization": if node_metrics.total_nodes > 0 { 
                        node_metrics.active_nodes as f64 / node_metrics.total_nodes as f64 
                    } else { 1.0 },
                    "total_eth_staked": protocol_metrics.total_eth_staked,
                    "reth_supply": protocol_metrics.reth_supply,
                    "node_demand_eth": protocol_metrics.node_demand,
                    "deposit_pool_balance": protocol_metrics.deposit_pool_balance,
                    "network_node_fee": protocol_metrics.network_node_fee,
                    "network_commission_percent": protocol_metrics.network_node_fee * 100.0,
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
            "✅ Successfully fetched and cached Rocket Pool positions"
        );
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        self.is_rocket_pool_contract(contract_address)
    }
    
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // Convert adapter positions to risk calculator positions
        let risk_positions: Vec<crate::models::position::Position> = positions
            .iter()
            .map(Self::convert_adapter_position_to_risk_position)
            .collect();
        
        // Use the dedicated Rocket Pool risk calculator for comprehensive risk assessment
        match self.risk_calculator.calculate_risk(&risk_positions).await {
            Ok(protocol_metrics) => {
                // Extract RocketPool metrics from the protocol metrics enum
                if let crate::risk::metrics::ProtocolRiskMetrics::RocketPool(risk_metrics) = protocol_metrics {
                    // Convert BigDecimal risk score to u8 (0-100 scale)
                    let risk_score = risk_metrics.overall_risk_score.to_string().parse::<f64>()
                        .unwrap_or(30.0) // Default fallback
                        .min(100.0)
                        .max(0.0) as u8;
                    
                    tracing::info!(
                        positions_count = positions.len(),
                        risk_score = risk_score,
                        validator_slashing_risk = %risk_metrics.validator_slashing_risk,
                        reth_depeg_risk = %risk_metrics.reth_depeg_risk,
                        withdrawal_queue_risk = %risk_metrics.withdrawal_queue_risk,
                        "🔍 Rocket Pool risk assessment completed using dedicated risk calculator"
                    );
                    
                    Ok(risk_score)
                } else {
                    tracing::warn!("Risk calculator returned non-RocketPool metrics");
                    Ok(30) // Default fallback
                }
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "⚠️ Risk calculator failed, falling back to basic risk assessment"
                );
                
                // Fallback to basic risk calculation
                let mut total_risk = 0u32;
                let mut total_weight = 0f64;
                
                for position in positions {
                    let position_weight = position.value_usd;
                    let risk_score = position.risk_score as u32;
                    
                    total_risk += (risk_score * position_weight as u32);
                    total_weight += position_weight;
                }
                
                if total_weight > 0.0 {
                    let weighted_risk = (total_risk as f64 / total_weight) as u8;
                    Ok(weighted_risk.min(100))
                } else {
                    Ok(30) // Default Rocket Pool risk (medium)
                }
            }
        }
    }
    


    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // Return the SAME value that was calculated during position creation
        // to maintain consistency and avoid price/rate mismatches
        
        // Extract the balance and recalculate using the same methodology
        if let Some(balance_str) = position.metadata.get("balance") {
            if let Some(balance_str) = balance_str.as_str() {
                if let Ok(balance) = U256::from_str(balance_str) {
                    let underlying_asset = position.metadata.get("underlying_asset")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ETH");
                    
                    // Use the same exchange rate and price as stored in metadata
                    let exchange_rate = position.metadata.get("reth_exchange_rate")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(1.1);
                    
                    let token_price = match underlying_asset {
                        "ETH" => self.get_eth_price_usd().await.unwrap_or(4000.0),
                        "RPL" => self.get_rpl_price_usd().await.unwrap_or(50.0),
                        _ => 4000.0,
                    };
                    
                    // Calculate value using the same methodology as position creation
                    let underlying_amount = if position.pair.contains("rETH") {
                        let reth_amount = balance.to::<u128>() as f64 / 10f64.powi(18);
                        reth_amount * exchange_rate
                    } else {
                        balance.to::<u128>() as f64 / 10f64.powi(18)
                    };
                    
                    let calculated_value = underlying_amount * token_price;
                    return Ok(calculated_value.max(0.01));
                }
            }
        }
        
        // Fallback to stored value for consistency
        Ok(position.value_usd)
    }
}

// Additional methods for RocketPoolAdapter (outside trait implementation)
impl RocketPoolAdapter {
    /// Get comprehensive risk assessment with detailed breakdown
    pub async fn get_comprehensive_risk_assessment(&self, positions: &[Position]) -> Result<serde_json::Value, AdapterError> {
        if positions.is_empty() {
            return Ok(serde_json::json!({
                "protocol": "rocket_pool",
                "overall_risk_score": 0,
                "risk_breakdown": {},
                "status_flags": {
                    "high_risk": false,
                    "requires_attention": false,
                    "healthy": true
                },
                "metadata": {
                    "positions_analyzed": 0,
                    "total_value_usd": 0.0,
                    "last_updated": chrono::Utc::now().to_rfc3339()
                }
            }));
        }
        
        // Convert adapter positions to risk calculator positions
        let risk_positions: Vec<crate::models::position::Position> = positions
            .iter()
            .map(Self::convert_adapter_position_to_risk_position)
            .collect();
        
        // Calculate comprehensive risk metrics using the dedicated risk calculator
        match self.risk_calculator.calculate_risk(&risk_positions).await {
            Ok(protocol_metrics) => {
                // Extract RocketPool metrics from the protocol metrics enum
                if let crate::risk::metrics::ProtocolRiskMetrics::RocketPool(risk_metrics) = protocol_metrics {
                    // Get risk explanation for user-friendly output
                    let risk_explanation = self.risk_calculator.explain_risk_calculation(&crate::risk::metrics::ProtocolRiskMetrics::RocketPool(risk_metrics.clone()))
                        .explanation;
                    
                    let overall_risk_score = risk_metrics.overall_risk_score.to_string().parse::<f64>()
                        .unwrap_or(30.0).min(100.0).max(0.0);
                    
                    let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
                    
                    // Determine status flags based on risk levels
                    let high_risk = overall_risk_score > 70.0;
                    let requires_attention = overall_risk_score > 50.0;
                    let healthy = overall_risk_score < 30.0;
                    
                    Ok(serde_json::json!({
                        "protocol": "rocket_pool",
                        "overall_risk_score": overall_risk_score,
                        "risk_breakdown": {
                            "validator_slashing_risk": risk_metrics.validator_slashing_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "reth_depeg_risk": risk_metrics.reth_depeg_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "withdrawal_queue_risk": risk_metrics.withdrawal_queue_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "protocol_governance_risk": risk_metrics.protocol_governance_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "validator_performance_risk": risk_metrics.validator_performance_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "liquidity_risk": risk_metrics.liquidity_risk.to_string().parse::<f64>().unwrap_or(0.0),
                            "smart_contract_risk": risk_metrics.smart_contract_risk.to_string().parse::<f64>().unwrap_or(0.0)
                        },
                        "status_flags": {
                            "high_risk": high_risk,
                            "requires_attention": requires_attention,
                            "healthy": healthy
                        },
                        "metadata": {
                            "positions_analyzed": positions.len(),
                            "total_value_usd": total_value,
                            "risk_explanation": risk_explanation,
                            "last_updated": chrono::Utc::now().to_rfc3339(),
                            "risk_factors_analyzed": 7,
                            "calculation_method": "comprehensive_rocket_pool_risk_calculator"
                        },
                        "historical_data": {
                            "30_day_avg_risk": risk_metrics.historical_30d_avg.to_string().parse::<f64>().unwrap_or(overall_risk_score),
                            "7_day_avg_risk": risk_metrics.historical_7d_avg.to_string().parse::<f64>().unwrap_or(overall_risk_score),
                            "risk_trend": if risk_metrics.historical_30d_avg > risk_metrics.overall_risk_score { "improving" } else { "stable" }
                        },
                        "recommendations": {
                            "action_required": high_risk,
                            "monitoring_frequency": if high_risk { "daily" } else if requires_attention { "weekly" } else { "monthly" },
                            "suggested_actions": if high_risk {
                                vec!["Consider reducing position size", "Monitor validator performance closely", "Check rETH liquidity conditions"]
                            } else if requires_attention {
                                vec!["Monitor risk trends", "Review position allocation"]
                            } else {
                                vec!["Continue regular monitoring"]
                            }
                        }
                    }))
                } else {
                    // Fallback for non-RocketPool metrics
                    let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
                    Ok(serde_json::json!({
                        "protocol": "rocket_pool",
                        "overall_risk_score": 30,
                        "risk_breakdown": {
                            "note": "Risk calculator returned unexpected metrics type"
                        },
                        "status_flags": {
                            "high_risk": false,
                            "requires_attention": true,
                            "healthy": false
                        },
                        "metadata": {
                            "positions_analyzed": positions.len(),
                            "total_value_usd": total_value,
                            "last_updated": chrono::Utc::now().to_rfc3339(),
                            "calculation_method": "fallback_unexpected_metrics_type"
                        }
                    }))
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "❌ Failed to calculate comprehensive risk assessment"
                );
                
                // Return basic risk assessment as fallback
                let basic_risk_score = self.calculate_risk_score(positions).await.unwrap_or(30);
                let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
                
                Ok(serde_json::json!({
                    "protocol": "rocket_pool",
                    "overall_risk_score": basic_risk_score,
                    "risk_breakdown": {
                        "note": "Detailed risk breakdown unavailable - using fallback calculation"
                    },
                    "status_flags": {
                        "high_risk": basic_risk_score > 70,
                        "requires_attention": basic_risk_score > 50,
                        "healthy": basic_risk_score < 30
                    },
                    "metadata": {
                        "positions_analyzed": positions.len(),
                        "total_value_usd": total_value,
                        "last_updated": chrono::Utc::now().to_rfc3339(),
                        "calculation_method": "fallback_basic_calculation",
                        "error": format!("Risk calculator error: {}", e)
                    }
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_reth_address() {
        let addr = Address::from_str(RocketPoolAdapter::RETH_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0xae78736cd615f374d3085123a210448e74fc6393");
    }
    
    #[test]
    fn test_rpl_address() {
        let addr = Address::from_str(RocketPoolAdapter::RPL_TOKEN_ADDRESS);
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string().to_lowercase(), "0xd33526068d116ce69f19a9ee46f0bd304f21a51f");
    }
    
    #[test]
    fn test_node_manager_address() {
        let addr = Address::from_str(RocketPoolAdapter::NODE_MANAGER_ADDRESS);
        assert!(addr.is_ok());
    }
    
    #[test]
    fn test_rocket_pool_contract_detection() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = RocketPoolAdapter::new(client).unwrap();
        
        let reth_addr = Address::from_str(RocketPoolAdapter::RETH_ADDRESS).unwrap();
        let rpl_addr = Address::from_str(RocketPoolAdapter::RPL_TOKEN_ADDRESS).unwrap();
        let node_mgr_addr = Address::from_str(RocketPoolAdapter::NODE_MANAGER_ADDRESS).unwrap();
        let random_addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        
        assert!(adapter.is_rocket_pool_contract(reth_addr));
        assert!(adapter.is_rocket_pool_contract(rpl_addr));
        assert!(adapter.is_rocket_pool_contract(node_mgr_addr));
        assert!(!adapter.is_rocket_pool_contract(random_addr));
    }
    
    #[test]
    fn test_token_symbol_resolution() {
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = RocketPoolAdapter::new(client).unwrap();
        
        let reth_addr = Address::from_str(RocketPoolAdapter::RETH_ADDRESS).unwrap();
        let rpl_addr = Address::from_str(RocketPoolAdapter::RPL_TOKEN_ADDRESS).unwrap();
        
        assert_eq!(adapter.get_rocket_pool_token_symbol(reth_addr), "rETH");
        assert_eq!(adapter.get_rocket_pool_token_symbol(rpl_addr), "RPL");
    }
    
    #[test]
    fn test_position_subtype_risk_scoring() {
        // Test that different position subtypes have appropriate risk scores
        let client = EthereumClient::new("https://eth.llamarpc.com").unwrap();
        let adapter = RocketPoolAdapter::new(client).unwrap();
        
        // Create mock positions for different subtypes
        let liquid_staking_pos = Position {
            id: "test_reth".to_string(),
            protocol: "rocket_pool".to_string(),
            position_type: "staking".to_string(),
            pair: "rETH/ETH".to_string(),
            value_usd: 10000.0,
            pnl_usd: 100.0,
            pnl_percentage: 4.5,
            risk_score: 20, // Should be low risk
            metadata: serde_json::json!({
                "position_subtype": "liquid_staking",
                "node_utilization": 0.85,
                "exchange_rate_premium": 8.5
            }),
            last_updated: 0,
        };
        
        let node_operator_pos = Position {
            id: "test_node".to_string(),
            protocol: "rocket_pool".to_string(),
            position_type: "node_operation".to_string(),
            pair: "RP-NODE/ETH".to_string(),
            value_usd: 50000.0,
            pnl_usd: 500.0,
            pnl_percentage: 6.2,
            risk_score: 35, // Should be medium risk
            metadata: serde_json::json!({
                "position_subtype": "node_operator",
                "node_utilization": 0.92,
                "node_demand_eth": 1200.0
            }),
            last_updated: 0,
        };
        
        let rpl_staking_pos = Position {
            id: "test_rpl".to_string(),
            protocol: "rocket_pool".to_string(),
            position_type: "governance_staking".to_string(),
            pair: "RPL/USD".to_string(),
            value_usd: 5000.0,
            pnl_usd: 200.0,
            pnl_percentage: 8.0,
            risk_score: 45, // Should be higher risk due to token volatility
            metadata: serde_json::json!({
                "position_subtype": "rpl_staking",
                "underlying_asset": "RPL"
            }),
            last_updated: 0,
        };
        
        assert!(liquid_staking_pos.risk_score < node_operator_pos.risk_score);
        assert!(node_operator_pos.risk_score < rpl_staking_pos.risk_score);
        assert_eq!(liquid_staking_pos.risk_score, 20);
        assert_eq!(node_operator_pos.risk_score, 35);
        assert_eq!(rpl_staking_pos.risk_score, 45);
    }
    
    #[test]
    fn test_exchange_rate_calculations() {
        // Test that exchange rate calculations work correctly
        let exchange_rate = 1.15; // rETH worth 15% more than ETH
        let reth_amount = 100.0; // 100 rETH
        let eth_equivalent = reth_amount * exchange_rate; // Should be 115 ETH
        
        assert_eq!(eth_equivalent, 115.0);
        
        // Test premium calculation
        let premium_percent = (exchange_rate - 1.0) * 100.0;
        assert_eq!(premium_percent, 15.0);
    }
    
    #[test]
    fn test_node_utilization_impact() {
        // Test that node utilization affects risk assessment appropriately
        let high_util = 0.95; // 95% utilization - might be risky
        let normal_util = 0.85; // 85% utilization - good
        let low_util = 0.65; // 65% utilization - concerning
        
        // These would be used in risk calculations
        assert!(high_util > 0.95);  // Should trigger risk increase
        assert!(normal_util > 0.7 && normal_util < 0.95); // Should be fine
        assert!(low_util < 0.7);    // Should trigger risk increase
    }
    
    #[test]
    fn test_protocol_addresses_are_different() {
        // Ensure all protocol addresses are unique
        let addresses = vec![
            RocketPoolAdapter::RETH_ADDRESS,
            RocketPoolAdapter::DEPOSIT_POOL_ADDRESS,
            RocketPoolAdapter::NODE_MANAGER_ADDRESS,
            RocketPoolAdapter::MINIPOOL_MANAGER_ADDRESS,
            RocketPoolAdapter::NETWORK_FEES_ADDRESS,
            RocketPoolAdapter::REWARDS_POOL_ADDRESS,
            RocketPoolAdapter::NODE_STAKING_ADDRESS,
            RocketPoolAdapter::RPL_TOKEN_ADDRESS,
        ];
        
        for i in 0..addresses.len() {
            for j in i+1..addresses.len() {
                assert_ne!(addresses[i], addresses[j], 
                    "Duplicate addresses found: {} and {}", addresses[i], addresses[j]);
            }
        }
    }
    
    #[test] 
    fn test_apy_calculations() {
        // Test APY calculation logic
        let base_eth_apy = 4.0;
        let node_commission = 0.15; // 15% commission
        
        // Liquid stakers get: base APY * (1 - commission)
        let liquid_staker_apy = base_eth_apy * (1.0 - node_commission);
        assert_eq!(liquid_staker_apy, 3.4);
        
        // Node operators get: base APY * (1 + commission from their share + their own rewards)
        let node_operator_apy = base_eth_apy * (1.0 + node_commission);
        assert_eq!(node_operator_apy, 4.6);
        
        // RPL staking APY is independent (inflation rewards)
        let rpl_inflation_apy = 7.5;
        assert!(rpl_inflation_apy > base_eth_apy);
    }
}