use std::env;
use std::collections::HashMap;
use alloy::primitives::Address;
use defi_risk_monitor::adapters::aave_v3::{AaveV3Adapter, chains, multi_chain_config};
use defi_risk_monitor::adapters::DeFiAdapter;
use defi_risk_monitor::blockchain::ethereum_client::EthereumClient;
use defi_risk_monitor::services::aave_price_service::AavePriceService;
use defi_risk_monitor::risk::calculators::aave_v3::AaveV3RiskCalculator;
use defi_risk_monitor::adapters::traits::{Position, AdapterError};
use tracing::{info, warn, error};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("debug")
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .init();

    info!("🌐 Starting Aave V3 Multi-Chain Position Discovery");
    
    // Test wallet address with known Aave V3 activity
    let test_address = "0x464C71f6c2F760DdA6093dCB91C24c39e5d6e18c".parse::<Address>()?;
    
    info!("🔍 Scanning address: {}", test_address);
    info!("📊 Supported chains: {:?}", chains::supported_chain_ids());

    // Get multi-chain RPC configuration
    let rpc_config = multi_chain_config::MultiChainRpcConfig::from_env();
    
    let mut chain_results = HashMap::new();
    let mut total_positions = 0;
    let mut total_value_usd = 0.0;

    // Test each supported chain
    for chain_id in chains::supported_chain_ids() {
        let chain_name = chains::get_chain_name(chain_id).unwrap_or("Unknown");
        info!("🔗 Testing chain: {} (ID: {})", chain_name, chain_id);

        // Get RPC URL for this specific chain
        let rpc_url = match rpc_config.get_rpc_url(chain_id) {
            Some(url) => url.clone(),
            None => {
                warn!("⚠️  No RPC URL configured for chain {}", chain_name);
                continue;
            }
        };

        match test_chain_positions(chain_id, test_address, &rpc_url).await {
            Ok(result) => {
                info!("✅ {}: {} positions, ${:.2} total value", 
                      chain_name, result.position_count, result.total_value_usd);
                
                total_positions += result.position_count;
                total_value_usd += result.total_value_usd;
                chain_results.insert(chain_id, result);
            }
            Err(e) => {
                warn!("⚠️  {}: Failed to fetch positions - {}", chain_name, e);
                chain_results.insert(chain_id, ChainResult {
                    chain_id,
                    chain_name: chain_name.to_string(),
                    position_count: 0,
                    total_value_usd: 0.0,
                    positions: Vec::new(),
                    error: Some(e.to_string()),
                });
            }
        }
    }

    // Generate comprehensive multi-chain report
    info!("📋 Multi-Chain Position Summary:");
    info!("🏦 Total Positions Across All Chains: {}", total_positions);
    info!("💰 Total Portfolio Value: ${:.2}", total_value_usd);
    info!("");

    for (chain_id, result) in &chain_results {
        let chain_name = chains::get_chain_name(*chain_id).unwrap_or("Unknown");
        if result.position_count > 0 {
            info!("🔗 {}: {} positions (${:.2})", 
                  chain_name, result.position_count, result.total_value_usd);
            
            // Show top 5 positions per chain
            let mut sorted_positions = result.positions.clone();
            sorted_positions.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap());
            
            for (i, position) in sorted_positions.iter().take(5).enumerate() {
                info!("  {}. {} - ${:.2}", i + 1, position.asset_symbol, position.value_usd);
            }
        } else if result.error.is_some() {
            warn!("❌ {}: {}", chain_name, result.error.as_ref().unwrap());
        } else {
            info!("⭕ {}: No positions found", chain_name);
        }
    }

    // Generate JSON output for frontend integration
    let multi_chain_report = MultiChainReport {
        address: format!("{:?}", test_address),
        total_positions,
        total_value_usd,
        chains: chain_results,
        scan_timestamp: chrono::Utc::now(),
    };

    let json_output = serde_json::to_string_pretty(&multi_chain_report)?;
    info!("📄 Multi-Chain JSON Report:");
    info!("{}", &json_output[..std::cmp::min(1000, json_output.len())]);
    if json_output.len() > 1000 {
        info!("... (truncated, total length: {} chars)", json_output.len());
    }

    info!("🎉 Multi-chain position discovery completed!");
    
    Ok(())
}

async fn test_chain_positions(
    chain_id: u64, 
    address: Address, 
    rpc_url: &str
) -> Result<ChainResult, AdapterError> {
    // Get chain configuration
    let chain_config = chains::get_chain_config(chain_id)
        .ok_or_else(|| AdapterError::UnsupportedChain(format!("Unsupported chain: {}", chain_id)))?;

    // Create chain-specific client
    let client = multi_chain_config::create_chain_client(chain_id, rpc_url).await?;
    
    // Create adapter for this chain (it creates its own price service and risk calculator)
    let adapter = AaveV3Adapter::new(client, chain_id)?;

    // Fetch positions
    let account_summary = adapter.get_user_positions(address).await?;
    
    let chain_name = chains::get_chain_name(chain_id).unwrap_or("Unknown").to_string();
    
    Ok(ChainResult {
        chain_id,
        chain_name,
        position_count: account_summary.positions.len(),
        total_value_usd: account_summary.net_worth_usd,
        positions: account_summary.positions,
        error: None,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
struct ChainResult {
    chain_id: u64,
    chain_name: String,
    position_count: usize,
    total_value_usd: f64,
    positions: Vec<Position>,
    error: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct MultiChainReport {
    address: String,
    total_positions: usize,
    total_value_usd: f64,
    chains: HashMap<u64, ChainResult>,
    scan_timestamp: chrono::DateTime<chrono::Utc>,
}
