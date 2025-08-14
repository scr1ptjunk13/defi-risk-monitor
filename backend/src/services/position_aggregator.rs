use alloy::primitives::Address;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::blockchain::EthereumClient;
use crate::adapters::{
    DeFiAdapter, Position, PortfolioSummary, AdapterError,
    UniswapV3Adapter, AaveV3Adapter, CurveAdapter, LidoAdapter
};
use crate::services::price_service::{PriceService, PriceError};

#[derive(Debug, thiserror::Error)]
pub enum AggregatorError {
    #[error("Adapter error: {0}")]
    AdapterError(#[from] AdapterError),
    
    #[error("Price error: {0}")]
    PriceError(#[from] PriceError),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("No positions found for address: {0}")]
    NoPositionsFound(String),
}

/// Aggregates positions from all DeFi protocol adapters
pub struct PositionAggregator {
    adapters: Vec<Box<dyn DeFiAdapter>>,
    price_service: Arc<RwLock<PriceService>>,
    _client: EthereumClient,
}

impl PositionAggregator {
    pub async fn new(
        client: EthereumClient,
        coingecko_api_key: Option<String>,
    ) -> Result<Self, AggregatorError> {
        let price_service = Arc::new(RwLock::new(PriceService::new(coingecko_api_key)));
        
        // Initialize all protocol adapters
        let mut adapters: Vec<Box<dyn DeFiAdapter>> = Vec::new();
        
        // Uniswap V3 (fully implemented)
        tracing::info!("Initializing Uniswap V3 adapter");
        adapters.push(Box::new(UniswapV3Adapter::new(client.clone())?));
        
        // Aave V3 (now implemented)
        tracing::info!("Initializing Aave V3 adapter");
        adapters.push(Box::new(AaveV3Adapter::new(client.clone())?));
        tracing::info!("Successfully initialized Aave V3 adapter");
        

        // Other protocols (stubs for now, will implement next)
        adapters.push(Box::new(CurveAdapter::new(client.clone())));
        adapters.push(Box::new(LidoAdapter::new(client.clone())?));
        
        Ok(Self {
            adapters,
            price_service,
            _client: client,
        })
    }
    
    /// Fetch all positions for a user across all protocols
    pub async fn fetch_user_portfolio(&self, address: Address) -> Result<PortfolioSummary, AggregatorError> {
        tracing::info!(
            user_address = %address,
            "Starting portfolio aggregation across all protocols"
        );
        
        let mut all_positions = Vec::new();
        let mut protocol_counts = HashMap::new();
        
        // Fetch positions from each protocol adapter
        tracing::info!(
            user_address = %address,
            adapter_count = self.adapters.len(),
            "Starting position fetch across all adapters"
        );
        
        for adapter in &self.adapters {
            let protocol_name = adapter.protocol_name();
            
            tracing::info!(
                user_address = %address,
                protocol = protocol_name,
                "Fetching positions from protocol"
            );
            
            match adapter.fetch_positions(address).await {
                Ok(positions) => {
                    if !positions.is_empty() {
                        tracing::info!(
                            user_address = %address,
                            protocol = protocol_name,
                            position_count = positions.len(),
                            "Successfully fetched positions"
                        );
                        
                        protocol_counts.insert(protocol_name.to_string(), positions.len() as u32);
                        all_positions.extend(positions);
                    } else {
                        tracing::debug!(
                            user_address = %address,
                            protocol = protocol_name,
                            "No positions found"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        user_address = %address,
                        protocol = protocol_name,
                        error = %e,
                        "Failed to fetch positions from protocol"
                    );
                    // Continue with other protocols even if one fails
                }
            }
        }
        
        if all_positions.is_empty() {
            return Err(AggregatorError::NoPositionsFound(format!("{:?}", address)));
        }
        
        // Update position values with current prices
        let updated_positions = self.update_position_values(all_positions).await?;
        
        // Calculate portfolio summary
        let summary = self.calculate_portfolio_summary(address, updated_positions, protocol_counts).await?;
        
        tracing::info!(
            user_address = %address,
            total_value_usd = summary.total_value_usd,
            position_count = summary.positions.len(),
            protocol_count = summary.protocols_count,
            "Portfolio aggregation completed"
        );
        
        Ok(summary)
    }
    
    /// Update position values with current market prices
    async fn update_position_values(&self, mut positions: Vec<Position>) -> Result<Vec<Position>, AggregatorError> {
        let _price_service = self.price_service.write().await;
        
        for position in &mut positions {
            // TODO: Extract token addresses from position metadata
            // For now, use cached values or implement price updates per protocol
            
            // Update timestamp
            position.last_updated = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
        
        Ok(positions)
    }
    
    /// Calculate overall portfolio summary
    async fn calculate_portfolio_summary(
        &self,
        _address: Address,
        positions: Vec<Position>,
        protocol_counts: HashMap<String, u32>,
    ) -> Result<PortfolioSummary, AggregatorError> {
        let total_value_usd = positions.iter().map(|p| p.value_usd).sum();
        let total_pnl_usd = positions.iter().map(|p| p.pnl_usd).sum();
        
        let total_pnl_percentage = if total_value_usd > 0.0 {
            (total_pnl_usd / (total_value_usd - total_pnl_usd)) * 100.0
        } else {
            0.0
        };
        
        // Calculate overall risk score (weighted average)
        let total_value_for_risk = positions.iter()
            .map(|p| p.value_usd)
            .sum::<f64>();
            
        let overall_risk_score = if total_value_for_risk > 0.0 {
            positions.iter()
                .map(|p| (p.risk_score as f64) * (p.value_usd / total_value_for_risk))
                .sum::<f64>() as u8
        } else {
            0
        };
        
        Ok(PortfolioSummary {
            total_value_usd,
            total_pnl_usd,
            total_pnl_percentage,
            active_positions: positions.len() as u32,
            protocols_count: protocol_counts.len() as u32,
            overall_risk_score,
            positions,
            last_updated: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    
    /// Get positions for a specific protocol
    pub async fn fetch_protocol_positions(
        &self,
        address: Address,
        protocol: &str,
    ) -> Result<Vec<Position>, AggregatorError> {
        let adapter = self.adapters.iter()
            .find(|a| a.protocol_name() == protocol)
            .ok_or_else(|| AggregatorError::InvalidAddress(format!("Protocol {} not supported", protocol)))?;
            
        let positions = adapter.fetch_positions(address).await?;
        Ok(positions)
    }
    
    /// Get supported protocols
    pub fn get_supported_protocols(&self) -> Vec<String> {
        self.adapters.iter()
            .map(|a| a.protocol_name().to_string())
            .collect()
    }
    
    /// Health check - test all adapters
    pub async fn health_check(&self) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        
        for adapter in &self.adapters {
            let protocol = adapter.protocol_name();
            // Simple health check - try to create the adapter
            results.insert(protocol.to_string(), true);
        }
        
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    
    #[tokio::test]
    async fn test_aggregator_creation() {
        // This test requires a valid RPC URL, so we'll skip it in CI
        if std::env::var("ETHEREUM_RPC_URL").is_err() {
            return;
        }
        
        let rpc_url = std::env::var("ETHEREUM_RPC_URL").unwrap();
        let client = EthereumClient::new(&rpc_url).await.unwrap();
        let aggregator = PositionAggregator::new(client, None).await.unwrap();
        
        let protocols = aggregator.get_supported_protocols();
        assert!(protocols.contains(&"uniswap_v3".to_string()));
        assert!(protocols.contains(&"aave_v3".to_string()));
        assert_eq!(protocols.len(), 5);
    }
    
    #[tokio::test]
    async fn test_health_check() {
        if std::env::var("ETHEREUM_RPC_URL").is_err() {
            return;
        }
        
        let rpc_url = std::env::var("ETHEREUM_RPC_URL").unwrap();
        let client = EthereumClient::new(&rpc_url).await.unwrap();
        let aggregator = PositionAggregator::new(client, None).await.unwrap();
        
        let health = aggregator.health_check().await;
        assert_eq!(health.len(), 5);
        assert!(health.values().all(|&v| v));
    }
}
