use std::collections::HashMap;
use alloy::primitives::Address;
use defi_risk_monitor::adapters::aave_v3::{AaveV3Adapter, chains, multi_chain_config};
use tracing::{info, warn};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("info")
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
    
    let mut results = HashMap::new();
    let mut total_chains_tested = 0;
    let mut successful_chains = 0;

    // Test each supported chain
    for chain_id in chains::supported_chain_ids() {
        let chain_name = chains::get_chain_name(chain_id).unwrap_or("Unknown");
        info!("🔗 Testing chain: {} (ID: {})", chain_name, chain_id);
        total_chains_tested += 1;

        // Get RPC URL for this specific chain
        let rpc_url = match rpc_config.get_rpc_url(chain_id) {
            Some(url) => url.clone(),
            None => {
                warn!("⚠️  No RPC URL configured for chain {}", chain_name);
                results.insert(chain_id, ChainTestResult {
                    chain_id,
                    chain_name: chain_name.to_string(),
                    success: false,
                    error: Some("No RPC URL configured".to_string()),
                    position_count: 0,
                    total_value: 0.0,
                });
                continue;
            }
        };

        match test_chain_positions(chain_id, test_address, &rpc_url).await {
            Ok(result) => {
                info!("✅ {}: {} positions, ${:.2} total value", 
                      chain_name, result.position_count, result.total_value);
                successful_chains += 1;
                results.insert(chain_id, result);
            }
            Err(e) => {
                warn!("❌ {}: Failed - {}", chain_name, e);
                results.insert(chain_id, ChainTestResult {
                    chain_id,
                    chain_name: chain_name.to_string(),
                    success: false,
                    error: Some(e.to_string()),
                    position_count: 0,
                    total_value: 0.0,
                });
            }
        }
    }

    // Generate comprehensive multi-chain report
    info!("");
    info!("📋 Multi-Chain Test Summary:");
    info!("🏦 Total Chains Tested: {}", total_chains_tested);
    info!("✅ Successful Chains: {}", successful_chains);
    info!("❌ Failed Chains: {}", total_chains_tested - successful_chains);
    info!("");

    let mut total_positions = 0;
    let mut total_portfolio_value = 0.0;

    for (chain_id, result) in &results {
        let chain_name = chains::get_chain_name(*chain_id).unwrap_or("Unknown");
        if result.success {
            info!("🔗 {} (Chain {}): {} positions, ${:.2}", 
                  chain_name, chain_id, result.position_count, result.total_value);
            total_positions += result.position_count;
            total_portfolio_value += result.total_value;
        } else {
            warn!("❌ {} (Chain {}): {}", 
                  chain_name, chain_id, result.error.as_ref().unwrap_or(&"Unknown error".to_string()));
        }
    }

    info!("");
    info!("🎯 Cross-Chain Portfolio Summary:");
    info!("💰 Total Portfolio Value: ${:.2}", total_portfolio_value);
    info!("📊 Total Positions: {}", total_positions);
    info!("🌐 Active Chains: {}", successful_chains);

    // Generate JSON output for frontend integration
    let multi_chain_report = MultiChainReport {
        address: format!("{:?}", test_address),
        total_chains_tested,
        successful_chains,
        total_positions,
        total_portfolio_value,
        chain_results: results,
        scan_timestamp: chrono::Utc::now(),
    };

    let json_output = serde_json::to_string_pretty(&multi_chain_report)?;
    info!("📄 Multi-Chain JSON Report (first 500 chars):");
    info!("{}", &json_output[..std::cmp::min(500, json_output.len())]);
    if json_output.len() > 500 {
        info!("... (truncated, total length: {} chars)", json_output.len());
    }

    info!("🎉 Multi-chain position discovery completed!");
    
    Ok(())
}

async fn test_chain_positions(
    chain_id: u64, 
    address: Address, 
    rpc_url: &str
) -> Result<ChainTestResult, Box<dyn std::error::Error>> {
    // Create chain-specific client
    let client = multi_chain_config::create_chain_client(chain_id, rpc_url).await?;
    
    // Create adapter for this chain
    let adapter = AaveV3Adapter::new(client, chain_id)?;

    // Fetch positions
    let account_summary = adapter.get_user_positions(address).await?;
    
    let chain_name = chains::get_chain_name(chain_id).unwrap_or("Unknown").to_string();
    
    Ok(ChainTestResult {
        chain_id,
        chain_name,
        success: true,
        error: None,
        position_count: account_summary.positions.len(),
        total_value: account_summary.net_worth_usd,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
struct ChainTestResult {
    chain_id: u64,
    chain_name: String,
    success: bool,
    error: Option<String>,
    position_count: usize,
    total_value: f64,
}

#[derive(Debug, serde::Serialize)]
struct MultiChainReport {
    address: String,
    total_chains_tested: usize,
    successful_chains: usize,
    total_positions: usize,
    total_portfolio_value: f64,
    chain_results: HashMap<u64, ChainTestResult>,
    scan_timestamp: chrono::DateTime<chrono::Utc>,
}
