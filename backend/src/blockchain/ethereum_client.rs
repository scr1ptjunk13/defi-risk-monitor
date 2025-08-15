use alloy::{
    primitives::{Address, U256},
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
};
use std::str::FromStr;
use tokio::time::Duration;

#[derive(Debug, Clone)]
pub struct EthereumClient {
    provider: RootProvider<Http<Client>>,
    rpc_url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EthereumError {
    #[error("RPC connection failed: {0}")]
    RpcError(String),
    
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    #[error("Contract call failed: {0}")]
    ContractError(String),
    
    #[error("Max retries exceeded: {0}")]
    MaxRetriesExceeded(u32),
    
    #[error("Network error: {0}")]
    NetworkError(String),
}

impl EthereumClient {
    /// Create a new Ethereum client with the given RPC URL
    pub async fn new(rpc_url: &str) -> Result<Self, EthereumError> {
        let provider = ProviderBuilder::new()
            .on_http(rpc_url.parse().map_err(|e| {
                EthereumError::RpcError(format!("Invalid RPC URL: {}", e))
            })?);

        // Test connection
        let client = Self {
            provider,
            rpc_url: rpc_url.to_string(),
        };
        
        client.test_connection().await?;
        
        Ok(client)
    }
    
    /// Create a new Ethereum client from an existing provider
    pub fn from_provider(provider: RootProvider<Http<Client>>) -> Self {
        Self {
            provider,
            rpc_url: "from_existing_provider".to_string(),
        }
    }
    
    /// Test the RPC connection by getting the latest block number
    pub async fn test_connection(&self) -> Result<(), EthereumError> {
        match self.provider.get_block_number().await {
            Ok(block_number) => {
                tracing::info!(
                    rpc_url = %self.rpc_url,
                    block_number = %block_number,
                    "Ethereum RPC connection established"
                );
                Ok(())
            }
            Err(e) => {
                Err(EthereumError::RpcError(format!(
                    "Failed to connect to Ethereum RPC: {}", e
                )))
            }
        }
    }
    
    /// Validate an Ethereum address
    pub fn validate_address(address: &str) -> Result<Address, EthereumError> {
        // Handle ENS names (for now, just validate format)
        if address.ends_with(".eth") {
            // TODO: Resolve ENS name to address
            return Err(EthereumError::InvalidAddress(
                "ENS resolution not yet implemented".to_string()
            ));
        }
        
        Address::from_str(address).map_err(|e| {
            EthereumError::InvalidAddress(format!("Invalid address format: {}", e))
        })
    }
    
    /// Get the ETH balance for an address
    pub async fn get_eth_balance(&self, address: Address) -> Result<U256, EthereumError> {
        self.provider
            .get_balance(address)
            .await
            .map_err(|e| EthereumError::RpcError(format!("Failed to get ETH balance: {}", e)))
    }
    
    /// Get the current block number
    pub async fn get_block_number(&self) -> Result<u64, EthereumError> {
        self.provider
            .get_block_number()
            .await
            .map_err(|e| EthereumError::RpcError(format!("Failed to get block number: {}", e)))
    }
    
    /// Get the underlying provider for contract instantiation
    pub fn provider(&self) -> &RootProvider<Http<Client>> {
        &self.provider
    }
    
    /// Make a contract call with retry logic
    pub async fn call_contract_with_retry<F, Fut, T>(
        &self,
        call_fn: F,
        max_retries: u32,
    ) -> Result<T, EthereumError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
        T: Send,
    {
        for attempt in 1..=max_retries {
            tracing::debug!(attempt, max_retries, "Attempting contract call");
            
            match call_fn().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        attempt,
                        max_retries,
                        error = %e,
                        "Contract call failed"
                    );
                    
                    if attempt < max_retries {
                        let delay = Duration::from_millis(100 * attempt as u64);
                        tokio::time::sleep(delay).await;
                    } else {
                        return Err(EthereumError::ContractError(format!(
                            "Failed after {} attempts: {}", 
                            max_retries, 
                            e
                        )));
                    }
                }
            }
        }
        
        Err(EthereumError::MaxRetriesExceeded(max_retries))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_address_validation() {
        // Valid address
        let valid_addr = "0x742d35Cc6634C0532925a3b8D8b7C8b8b8b8b8b8";
        assert!(EthereumClient::validate_address(valid_addr).is_ok());
        
        // Invalid address
        let invalid_addr = "0xinvalid";
        assert!(EthereumClient::validate_address(invalid_addr).is_err());
        
        // ENS name (should fail for now)
        let ens_name = "vitalik.eth";
        assert!(EthereumClient::validate_address(ens_name).is_err());
    }
    
    #[tokio::test]
    async fn test_client_creation_with_invalid_url() {
        let result = EthereumClient::new("invalid-url").await;
        assert!(result.is_err());
    }
}
