use alloy::{
    primitives::{Address, keccak256, FixedBytes, Bytes},
    providers::{Provider, ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
    rpc::types::TransactionRequest,
    sol_types::{SolCall, sol},
};
use std::str::FromStr;
use crate::error::AppError;

// ENS Registry contract interface
sol! {
    #[allow(missing_docs)]
    interface ENSRegistry {
        function resolver(bytes32 node) external view returns (address);
    }
    
    #[allow(missing_docs)]
    interface ENSResolver {
        function addr(bytes32 node) external view returns (address);
        function name(bytes32 node) external view returns (string);
    }
}

/// ENS (Ethereum Name Service) resolution service
pub struct EnsService {
    provider: RootProvider<Http<Client>>,
    ens_registry: Address,
}

impl EnsService {
    /// Create a new ENS service with the given provider
    pub fn new(rpc_url: &str) -> Result<Self, AppError> {
        let url = rpc_url.parse().map_err(|e| {
            AppError::ConfigError(format!("Invalid RPC URL: {}", e))
        })?;
        
        let provider = ProviderBuilder::new().on_http(url);
        
        // ENS Registry contract address on Ethereum mainnet
        let ens_registry = "0x00000000000C2E074eC69A0dFb2997BA6C7d2e1e".parse()
            .map_err(|e| AppError::ConfigError(format!("Invalid ENS registry address: {}", e)))?;

        Ok(Self {
            provider,
            ens_registry,
        })
    }

    /// Resolve ENS name to Ethereum address
    pub async fn resolve_ens(&self, ens_name: &str) -> Result<Address, AppError> {
        // Validate ENS name format
        if !ens_name.ends_with(".eth") {
            return Err(AppError::ValidationError(
                "Invalid ENS name: must end with .eth".to_string()
            ));
        }

        tracing::info!("Resolving ENS name: {}", ens_name);
        
        // Calculate the namehash for the ENS name
        let node = self.namehash(ens_name);
        tracing::debug!("Namehash for '{}': {:?}", ens_name, node);
        
        // Get the resolver address from ENS registry
        let resolver_call = ENSRegistry::resolverCall { node };
        let call_data = resolver_call.abi_encode();
        
        let tx_request = TransactionRequest::default()
            .to(self.ens_registry)
            .input(call_data.into());
            
        let result_bytes = match self.provider.call(&tx_request).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to get resolver for '{}': {}", ens_name, e);
                return Err(AppError::ExternalApiError(
                    format!("Failed to query ENS registry: {}", e)
                ));
            }
        };
        
        let resolver_result = match ENSRegistry::resolverCall::abi_decode_returns(&result_bytes, false) {
            Ok(decoded) => decoded,
            Err(e) => {
                tracing::error!("Failed to decode resolver response for '{}': {}", ens_name, e);
                return Err(AppError::ExternalApiError(
                    format!("Failed to decode ENS registry response: {}", e)
                ));
            }
        };
        
        let resolver_address = resolver_result._0;
        if resolver_address == Address::ZERO {
            tracing::warn!("No resolver found for ENS name: {}", ens_name);
            return Err(AppError::ValidationError(
                format!("ENS name '{}' has no resolver configured", ens_name)
            ));
        }
        
        tracing::debug!("Resolver address for '{}': {:?}", ens_name, resolver_address);
        
        // Query the resolver for the address
        let addr_call = ENSResolver::addrCall { node };
        let addr_call_data = addr_call.abi_encode();
        
        let addr_tx_request = TransactionRequest::default()
            .to(resolver_address)
            .input(addr_call_data.into());
            
        let addr_result_bytes = match self.provider.call(&addr_tx_request).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to resolve address for '{}': {}", ens_name, e);
                return Err(AppError::ExternalApiError(
                    format!("Failed to query ENS resolver: {}", e)
                ));
            }
        };
        
        let addr_result = match ENSResolver::addrCall::abi_decode_returns(&addr_result_bytes, false) {
            Ok(decoded) => decoded,
            Err(e) => {
                tracing::error!("Failed to decode address response for '{}': {}", ens_name, e);
                return Err(AppError::ExternalApiError(
                    format!("Failed to decode ENS resolver response: {}", e)
                ));
            }
        };
        
        let resolved_address = addr_result._0;
        if resolved_address == Address::ZERO {
            tracing::warn!("ENS name '{}' resolves to zero address", ens_name);
            Err(AppError::ValidationError(
                format!("ENS name '{}' is not configured or resolves to zero address", ens_name)
            ))
        } else {
            tracing::info!("ENS '{}' resolved to: {:?}", ens_name, resolved_address);
            Ok(resolved_address)
        }
    }

    /// Reverse resolve address to ENS name (if available)
    pub async fn reverse_resolve(&self, address: Address) -> Result<Option<String>, AppError> {
        tracing::debug!("Attempting reverse ENS lookup for: {:?}", address);
        
        // For now, reverse resolution is complex and not critical
        // TODO: Implement reverse ENS resolution using reverse registrar
        tracing::debug!("Reverse ENS lookup not yet implemented for {:?}", address);
        Ok(None)
    }

    /// Resolve address or ENS name to a valid Ethereum address
    pub async fn resolve_address_or_ens(&self, input: &str) -> Result<Address, AppError> {
        if input.ends_with(".eth") {
            // It's an ENS name - resolve it
            self.resolve_ens(input).await
        } else {
            // It should be a hex address - validate and parse it
            Address::from_str(input).map_err(|e| {
                AppError::ValidationError(format!("Invalid Ethereum address: {}", e))
            })
        }
    }

    /// Get display name for an address (ENS name if available, otherwise shortened address)
    pub async fn get_display_name(&self, address: Address) -> String {
        match self.reverse_resolve(address).await {
            Ok(Some(ens_name)) => ens_name,
            _ => {
                // Return shortened address format: 0x1234...5678
                let addr_str = format!("{:?}", address);
                if addr_str.len() > 10 {
                    format!("{}...{}", &addr_str[0..6], &addr_str[addr_str.len()-4..])
                } else {
                    addr_str
                }
            }
        }
    }

    /// Batch resolve multiple ENS names (more efficient for multiple lookups)
    pub async fn batch_resolve(&self, ens_names: Vec<&str>) -> Vec<(String, Result<Address, AppError>)> {
        let mut results = Vec::new();
        
        for ens_name in ens_names {
            let result = self.resolve_ens(ens_name).await;
            results.push((ens_name.to_string(), result));
        }
        
        results
    }

    /// Calculate ENS namehash for a given domain name
    fn namehash(&self, name: &str) -> FixedBytes<32> {
        if name.is_empty() {
            return FixedBytes::ZERO;
        }
        
        let mut node = FixedBytes::ZERO;
        let labels: Vec<&str> = name.split('.').collect();
        
        for label in labels.iter().rev() {
            let label_hash = keccak256(label.as_bytes());
            let mut combined = [0u8; 64];
            combined[..32].copy_from_slice(&node[..]);
            combined[32..].copy_from_slice(&label_hash[..]);
            node = keccak256(&combined);
        }
        
        node
    }

    /// Check if a string looks like an ENS name
    pub fn is_ens_name(input: &str) -> bool {
        input.ends_with(".eth") && input.len() > 4 && !input.contains("0x")
    }

    /// Check if a string looks like an Ethereum address
    pub fn is_ethereum_address(input: &str) -> bool {
        input.starts_with("0x") && input.len() == 42
    }

    /// Validate and normalize input (ENS name or address)
    pub fn validate_input(input: &str) -> Result<String, AppError> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(AppError::ValidationError("Address or ENS name cannot be empty".to_string()));
        }

        if Self::is_ens_name(trimmed) {
            // Normalize ENS name to lowercase
            Ok(trimmed.to_lowercase())
        } else if Self::is_ethereum_address(trimmed) {
            // Validate address format
            Address::from_str(trimmed)
                .map_err(|e| AppError::ValidationError(format!("Invalid Ethereum address: {}", e)))?;
            Ok(trimmed.to_string())
        } else {
            Err(AppError::ValidationError(
                "Input must be either an ENS name (ending with .eth) or a valid Ethereum address (0x...)".to_string()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ens_name() {
        assert!(EnsService::is_ens_name("vitalik.eth"));
        assert!(EnsService::is_ens_name("test.eth"));
        assert!(!EnsService::is_ens_name("0x1234567890123456789012345678901234567890"));
        assert!(!EnsService::is_ens_name("invalid"));
        assert!(!EnsService::is_ens_name(""));
    }

    #[test]
    fn test_is_ethereum_address() {
        assert!(EnsService::is_ethereum_address("0x1234567890123456789012345678901234567890"));
        assert!(!EnsService::is_ethereum_address("vitalik.eth"));
        assert!(!EnsService::is_ethereum_address("0x123")); // Too short
        assert!(!EnsService::is_ethereum_address("1234567890123456789012345678901234567890")); // No 0x prefix
    }

    #[test]
    fn test_validate_input() {
        assert!(EnsService::validate_input("vitalik.eth").is_ok());
        assert!(EnsService::validate_input("0x1234567890123456789012345678901234567890").is_ok());
        assert!(EnsService::validate_input("").is_err());
        assert!(EnsService::validate_input("invalid").is_err());
    }
}
