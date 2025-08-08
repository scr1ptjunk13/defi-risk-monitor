use alloy::{
    primitives::{Address, U256},
    sol,
    sol_types::SolCall,
};
use async_trait::async_trait;
use std::str::FromStr;

use crate::blockchain::EthereumClient;
use super::traits::{DeFiAdapter, Position, AdapterError};

// Uniswap V3 contract ABIs using alloy sol! macro
sol! {
    #[sol(rpc)]
    interface INonfungiblePositionManager {
        struct Position {
            uint96 nonce;
            address operator;
            address token0;
            address token1;
            uint24 fee;
            int24 tickLower;
            int24 tickUpper;
            uint128 liquidity;
            uint256 feeGrowthInside0LastX128;
            uint256 feeGrowthInside1LastX128;
            uint128 tokensOwed0;
            uint128 tokensOwed1;
        }
        
        function positions(uint256 tokenId) external view returns (Position memory);
        function balanceOf(address owner) external view returns (uint256);
        function tokenOfOwnerByIndex(address owner, uint256 index) external view returns (uint256);
    }
    
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
        
        function liquidity() external view returns (uint128);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
    }
}

/// Uniswap V3 protocol adapter
pub struct UniswapV3Adapter {
    client: EthereumClient,
    position_manager_address: Address,
}

impl UniswapV3Adapter {
    /// Uniswap V3 NonfungiblePositionManager on Ethereum mainnet
    const POSITION_MANAGER_ADDRESS: &'static str = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
    
    pub fn new(client: EthereumClient) -> Result<Self, AdapterError> {
        let position_manager_address = Address::from_str(Self::POSITION_MANAGER_ADDRESS)
            .map_err(|e| AdapterError::InvalidData(format!("Invalid position manager address: {}", e)))?;
            
        Ok(Self {
            client,
            position_manager_address,
        })
    }
    
    /// Get all NFT token IDs owned by an address
    async fn get_user_token_ids(&self, address: Address) -> Result<Vec<U256>, AdapterError> {
        let contract = INonfungiblePositionManager::new(self.position_manager_address, self.client.provider());
        
        // Get balance of NFTs
        let balance = contract.balanceOf(address).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get NFT balance: {}", e)))?
            ._0;
        
        let mut token_ids = Vec::new();
        
        // Get each token ID
        for i in 0..balance.to::<u64>() {
            let token_id = contract.tokenOfOwnerByIndex(address, U256::from(i)).call().await
                .map_err(|e| AdapterError::ContractError(format!("Failed to get token ID at index {}: {}", i, e)))?
                ._0;
            
            token_ids.push(token_id);
        }
        
        Ok(token_ids)
    }
    
    /// Get position details for a specific NFT token ID
    async fn get_position_details(&self, token_id: U256) -> Result<Option<Position>, AdapterError> {
        let contract = INonfungiblePositionManager::new(self.position_manager_address, self.client.provider());
        
        let position_data = contract.positions(token_id).call().await
            .map_err(|e| AdapterError::ContractError(format!("Failed to get position for token ID {}: {}", token_id, e)))?
            ._0;
        
        // Skip positions with zero liquidity
        if position_data.liquidity == 0 {
            return Ok(None);
        }
        
        // Create position struct
        let position = Position {
            id: format!("uniswap_v3_{}", token_id),
            protocol: "uniswap_v3".to_string(),
            position_type: "liquidity".to_string(),
            pair: format!("{:?}/{:?}", position_data.token0, position_data.token1), // TODO: Resolve to symbols
            value_usd: 0.0, // TODO: Calculate actual USD value
            pnl_usd: 0.0,   // TODO: Calculate P&L
            pnl_percentage: 0.0,
            risk_score: 50, // TODO: Calculate based on price range and volatility
            metadata: serde_json::json!({
                "token_id": token_id.to_string(),
                "token0": format!("{:?}", position_data.token0),
                "token1": format!("{:?}", position_data.token1),
                "fee_tier": position_data.fee,
                "tick_lower": position_data.tickLower,
                "tick_upper": position_data.tickUpper,
                "liquidity": position_data.liquidity.to_string(),
                "tokens_owed_0": position_data.tokensOwed0.to_string(),
                "tokens_owed_1": position_data.tokensOwed1.to_string(),
            }),
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        Ok(Some(position))
    }
}

#[async_trait]
impl DeFiAdapter for UniswapV3Adapter {
    fn protocol_name(&self) -> &'static str {
        "uniswap_v3"
    }
    
    async fn fetch_positions(&self, address: Address) -> Result<Vec<Position>, AdapterError> {
        tracing::info!(
            user_address = %address,
            protocol = "uniswap_v3",
            "Fetching Uniswap V3 positions"
        );
        
        // Get all NFT token IDs for the user
        let token_ids = self.get_user_token_ids(address).await?;
        
        if token_ids.is_empty() {
            tracing::info!(
                user_address = %address,
                "No Uniswap V3 positions found"
            );
            return Ok(Vec::new());
        }
        
        let mut positions = Vec::new();
        
        // Fetch details for each position
        for token_id in token_ids {
            match self.get_position_details(token_id).await? {
                Some(position) => positions.push(position),
                None => continue, // Skip empty positions
            }
        }
        
        tracing::info!(
            user_address = %address,
            position_count = positions.len(),
            "Successfully fetched Uniswap V3 positions"
        );
        
        Ok(positions)
    }
    
    async fn supports_contract(&self, contract_address: Address) -> bool {
        contract_address == self.position_manager_address
    }
    
    async fn calculate_risk_score(&self, positions: &[Position]) -> Result<u8, AdapterError> {
        if positions.is_empty() {
            return Ok(0);
        }
        
        // Simple risk calculation based on number of positions
        // TODO: Implement proper risk calculation based on:
        // - Price range (in-range vs out-of-range)
        // - Impermanent loss potential
        // - Pool volatility
        // - Liquidity depth
        
        let avg_risk = positions.iter()
            .map(|p| p.risk_score as u32)
            .sum::<u32>() / positions.len() as u32;
            
        Ok(avg_risk as u8)
    }
    
    async fn get_position_value(&self, position: &Position) -> Result<f64, AdapterError> {
        // TODO: Implement real-time position valuation
        // This requires:
        // 1. Getting current pool price from slot0()
        // 2. Calculating token amounts based on liquidity and price range
        // 3. Converting to USD using price feeds
        
        Ok(position.value_usd) // Return cached value for now
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_manager_address() {
        let addr = Address::from_str(UniswapV3Adapter::POSITION_MANAGER_ADDRESS);
        assert!(addr.is_ok());
    }
}
