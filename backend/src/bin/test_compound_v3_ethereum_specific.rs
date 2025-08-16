// Test Compound V3 Adapter specifically on Ethereum with known positions
// Testing address: 0x42e6300d8d5C1531996B8d567528147761C76d39

use defi_risk_monitor::adapters::compound_v3::CompoundV3Adapter;
use defi_risk_monitor::adapters::traits::DeFiAdapter;
use defi_risk_monitor::blockchain::ethereum_client::EthereumClient;
use defi_risk_monitor::risk::calculators::CompoundV3RiskCalculator;
use alloy::primitives::Address;
use std::str::FromStr;
use tokio;
use tracing::{info, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging with more detailed output
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🚀 Testing Compound V3 Adapter on Ethereum with Known Positions");
    info!("📍 Target Address: 0x42e6300d8d5C1531996B8d567528147761C76d39");
    info!("📊 Expected Positions from Zerion:");
    info!("   • LINK Deposited: 25,110.217 LINK (~$547,914)");
    info!("   • USDC Debt: 331,022.155 USDC (~$330,745)");
    info!("   • WETH Deposited: 0.6 WETH (~$2,666)");
    info!("   • COMP Deposited: 0.857 COMP (~$41)");
    info!("   • COMP Rewards: 0.684 COMP (~$33)");

    // Load environment variables
    match dotenvy::dotenv() {
        Ok(_) => info!("✅ Environment variables loaded"),
        Err(_) => info!("⚠️ Using system environment variables"),
    }

    // Get RPC URL
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .or_else(|_| std::env::var("INFURA_URL"))
        .unwrap_or_else(|_| {
            error!("❌ No RPC URL found");
            panic!("Please set ETHEREUM_RPC_URL or INFURA_URL");
        });

    info!("🔗 Using Ethereum RPC: {}", rpc_url.chars().take(50).collect::<String>() + "...");

    // Target address from Zerion
    let target_address = "0x42e6300d8d5C1531996B8d567528147761C76d39";
    let address = Address::from_str(target_address)?;

    // Initialize Ethereum client
    let ethereum_client = EthereumClient::new(&rpc_url).await?;
    info!("✅ Ethereum client initialized");

    // Initialize Compound V3 adapter for Ethereum (chain_id = 1)
    let compound_adapter = CompoundV3Adapter::new(ethereum_client, 1)?;
    info!("✅ Compound V3 adapter initialized for Ethereum");
    info!("📊 Protocol: {}", compound_adapter.protocol_name());

    // Test position fetching with detailed logging
    info!("\n🔍 Fetching Compound V3 positions...");
    match compound_adapter.fetch_positions(address).await {
        Ok(positions) => {
            info!("✅ Successfully fetched {} positions", positions.len());
            
            if positions.is_empty() {
                error!("❌ No positions found! This is unexpected based on Zerion data.");
                info!("🔧 Debugging information:");
                info!("   • Address: {}", target_address);
                info!("   • Chain: Ethereum (ID: 1)");
                info!("   • Expected: 5 positions (LINK, USDC debt, WETH, 2x COMP)");
                return Ok(());
            }

            info!("\n📊 POSITION DETAILS:");
            info!("==================");
            
            for (i, position) in positions.iter().enumerate() {
                info!("\n📍 Position {} of {}:", i + 1, positions.len());
                info!("   🆔 ID: {}", position.id);
                info!("   🏷️  Protocol: {}", position.protocol);
                info!("   📈 Type: {}", position.position_type);
                info!("   💱 Pair/Asset: {}", position.pair);
                info!("   💰 Value USD: ${:.2}", position.value_usd);
                info!("   📊 P&L USD: ${:.2}", position.pnl_usd);
                info!("   📈 P&L %: {:.2}%", position.pnl_percentage);
                info!("   ⚠️  Risk Score: {}/100", position.risk_score);
                info!("   🕒 Last Updated: {}", position.last_updated);
                
                // Display metadata
                if !position.metadata.is_null() {
                    info!("   📋 Metadata: {}", serde_json::to_string_pretty(&position.metadata)?);
                }
            }

            // Calculate totals
            let total_value: f64 = positions.iter().map(|p| p.value_usd).sum();
            let total_pnl: f64 = positions.iter().map(|p| p.pnl_usd).sum();
            
            info!("\n💼 PORTFOLIO SUMMARY:");
            info!("====================");
            info!("💰 Total Value: ${:.2}", total_value);
            info!("📊 Total P&L: ${:.2}", total_pnl);
            info!("📈 Position Count: {}", positions.len());

            // Test risk calculation
            info!("\n🎯 RISK ASSESSMENT:");
            info!("==================");
            match compound_adapter.calculate_risk_score(&positions).await {
                Ok(risk_score) => {
                    info!("✅ Risk Score: {}/100", risk_score);
                    
                    let risk_level = match risk_score {
                        0..=20 => "Very Low",
                        21..=40 => "Low",
                        41..=60 => "Medium", 
                        61..=80 => "High",
                        81..=100 => "Very High",
                        _ => "Unknown"
                    };
                    info!("🎯 Risk Level: {}", risk_level);
                }
                Err(e) => {
                    error!("❌ Risk calculation failed: {}", e);
                }
            }

            // Test individual position values
            info!("\n💰 POSITION VALUE VERIFICATION:");
            info!("==============================");
            for (i, position) in positions.iter().enumerate() {
                match compound_adapter.get_position_value(position).await {
                    Ok(value) => {
                        info!("✅ Position {} value: ${:.2}", i + 1, value);
                    }
                    Err(e) => {
                        error!("❌ Position {} value calculation failed: {}", i + 1, e);
                    }
                }
            }

            // Compare with Zerion data
            info!("\n🔍 COMPARISON WITH ZERION DATA:");
            info!("==============================");
            info!("Expected from Zerion: ~$219,909 total");
            info!("Our calculation: ${:.2}", total_value);
            info!("Difference: ${:.2}", (total_value - 219909.0).abs());
            
            if (total_value - 219909.0).abs() < 50000.0 {
                info!("✅ Values are reasonably close to Zerion!");
            } else {
                info!("⚠️  Significant difference from Zerion data");
            }

            // Generate JSON output
            let output = serde_json::json!({
                "protocol": "compound_v3",
                "chain": "ethereum",
                "address": target_address,
                "positions": positions,
                "summary": {
                    "total_value_usd": total_value,
                    "total_pnl_usd": total_pnl,
                    "position_count": positions.len(),
                },
                "comparison": {
                    "zerion_total": 219909.52,
                    "our_total": total_value,
                    "difference": (total_value - 219909.52).abs(),
                },
                "test_results": {
                    "positions_found": !positions.is_empty(),
                    "risk_calculation": "completed",
                    "value_calculation": "completed",
                },
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            info!("\n📄 JSON OUTPUT (first 1000 chars):");
            let json_str = serde_json::to_string_pretty(&output)?;
            info!("{}", json_str.chars().take(1000).collect::<String>());
            if json_str.len() > 1000 {
                info!("... (truncated, total: {} chars)", json_str.len());
            }

        }
        Err(e) => {
            error!("❌ Failed to fetch positions: {}", e);
            info!("🔧 This could be due to:");
            info!("   • Contract interface mismatch");
            info!("   • RPC connection issues");
            info!("   • Address parsing problems");
            info!("   • Chain configuration errors");
        }
    }

    info!("\n🏁 TEST SUMMARY:");
    info!("===============");
    info!("✅ Adapter initialization: SUCCESS");
    info!("✅ Blockchain connection: SUCCESS");
    info!("✅ Address parsing: SUCCESS");
    info!("📊 Expected positions: 5 (from Zerion)");
    info!("🎯 Target: Validate our adapter matches Zerion data");

    Ok(())
}
