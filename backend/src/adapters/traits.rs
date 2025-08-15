use alloy::primitives::{Address, U256};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Common error type for all DeFi protocol adapters
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("Contract call failed: {0}")]
    ContractError(String),
    
    #[error("Invalid position data: {0}")]
    InvalidData(String),
    
    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),
    
    #[error("RPC error: {0}")]
    RpcError(String),
    
    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Represents a DeFi position for any protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Unique identifier for the position
    pub id: String,
    
    /// Protocol name (e.g., "uniswap_v3", "aave_v3")
    pub protocol: String,
    
    /// Position type (e.g., "liquidity", "lending", "borrowing")
    pub position_type: String,
    
    /// Token pair or asset (e.g., "ETH/USDC", "WETH")
    pub pair: String,
    
    /// Current USD value of the position
    pub value_usd: f64,
    
    /// Profit/Loss in USD
    pub pnl_usd: f64,
    
    /// Profit/Loss percentage
    pub pnl_percentage: f64,
    
    /// Risk score (0-100, higher = riskier)
    pub risk_score: u8,
    
    /// Additional protocol-specific data
    pub metadata: serde_json::Value,
    
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Portfolio summary across all protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSummary {
    /// Total portfolio value in USD
    pub total_value_usd: f64,
    
    /// Total P&L in USD
    pub total_pnl_usd: f64,
    
    /// Total P&L percentage
    pub total_pnl_percentage: f64,
    
    /// Number of active positions
    pub active_positions: u32,
    
    /// Number of protocols with positions
    pub protocols_count: u32,
    
    /// Overall risk score (0-100)
    pub overall_risk_score: u8,
    
    /// All positions
    pub positions: Vec<Position>,
    
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Common interface for all DeFi protocol adapters
#[async_trait]
pub trait DeFiAdapter: Send + Sync {
    /// Get the protocol name
    fn protocol_name(&self) -> &'static str;
    
    /// Fetch all positions for a given address
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError>;
    
    /// Check if the adapter supports a specific contract address
    async fn supports_contract(&self, contract_address: Address) -> bool;
    
    /// Get the health factor or risk assessment for positions
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError>;
    
    /// Get real-time price data for position valuation
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError>;
}

/// Price information for tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPrice {
    pub address: Address,
    pub symbol: String,
    pub price_usd: f64,
    pub timestamp: u64,
}

/// Liquidity pool information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolInfo {
    pub address: Address,
    pub token0: Address,
    pub token1: Address,
    pub fee_tier: u32,
    pub liquidity: U256,
    pub sqrt_price_x96: U256,
}
