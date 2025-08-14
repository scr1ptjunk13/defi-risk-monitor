use std::str::FromStr;
use alloy::primitives::Address;
use defi_risk_monitor::adapters::lido::LidoAdapter;
use defi_risk_monitor::adapters::traits::DeFiAdapter;
use defi_risk_monitor::risk::calculators::lido::LidoRiskCalculator;
use defi_risk_monitor::risk::traits::ProtocolRiskCalculator;
use defi_risk_monitor::blockchain::ethereum_client::EthereumClient;
use bigdecimal::{BigDecimal, ToPrimitive};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("🧪 Testing Lido Adapter with Real stETH Address");
    println!("================================================");

    // Real stETH holder address
    let test_address = Address::from_str("0x28C6c06298d514Db089934071355E5743bf21d60")?;
    println!("📍 Testing address: {}", test_address);

    // Create Ethereum client and Lido adapter
    println!("\n🔧 Initializing Ethereum client and Lido adapter...");
    
    // Initialize Ethereum client with environment variables
    let rpc_url = std::env::var("ETHEREUM_RPC_URL").unwrap_or_else(|_| "https://eth.llamarpc.com".to_string());
    let ethereum_client = EthereumClient::new(&rpc_url).await?;
    println!("✅ Ethereum client initialized successfully");
    
    let lido_adapter = match LidoAdapter::new(ethereum_client) {
        Ok(adapter) => {
            println!("✅ Lido adapter initialized successfully");
            adapter
        }
        Err(e) => {
            println!("❌ Failed to initialize Lido adapter: {}", e);
            return Err(e.into());
        }
    };

    // Test 1: Fetch positions
    println!("\n📊 Fetching Lido positions...");
    match lido_adapter.fetch_positions(test_address).await {
        Ok(positions) => {
            println!("✅ Successfully fetched {} positions", positions.len());
            
            for (i, position) in positions.iter().enumerate() {
                println!("\n📈 Position {}: {}", i + 1, position.id);
                println!("   Protocol: {}", position.protocol);
                println!("   Type: {}", position.position_type);
                println!("   Pair: {}", position.pair);
                println!("   Value USD: {}", position.value_usd);
                println!("   PnL USD: {}", position.pnl_usd);
                println!("   PnL Percentage: {}", position.pnl_percentage);
                println!("   Risk Score: {}", position.risk_score);
                println!("   Last Updated: {}", position.last_updated);
            }

            // Test 2: Calculate risk score
            if !positions.is_empty() {
                println!("\n🎯 Calculating risk scores...");
                match lido_adapter.calculate_risk_score(&positions).await {
                    Ok(risk_score) => {
                        println!("✅ Overall risk score: {}/100", risk_score);
                        
                        // Interpret risk score
                        let risk_level = match risk_score {
                            0..=20 => "🟢 Very Low Risk",
                            21..=40 => "🟡 Low Risk",
                            41..=60 => "🟠 Medium Risk",
                            61..=80 => "🔴 High Risk",
                            81..=100 => "⚫ Very High Risk",
                            _ => "❓ Unknown Risk",
                        };
                        println!("📊 Risk Level: {}", risk_level);
                    }
                    Err(e) => {
                        println!("❌ Failed to calculate risk score: {}", e);
                    }
                }

                // Test 3: Detailed risk calculation using LidoRiskCalculator
                println!("\n🔍 Performing detailed risk analysis...");
                let risk_calculator = LidoRiskCalculator::new();
                
                // Convert adapter positions to model positions (simplified conversion)
                let model_positions: Vec<defi_risk_monitor::models::Position> = positions.iter().map(|pos| {
                    use uuid::Uuid;
                    use bigdecimal::BigDecimal;
                    use chrono::Utc;
                    
                    defi_risk_monitor::models::Position {
                        id: Uuid::new_v4(),
                        user_address: test_address.to_string(),
                        protocol: pos.protocol.clone(),
                        pool_address: "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84".to_string(), // stETH contract
                        token0_address: "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84".to_string(), // stETH
                        token1_address: "0x0000000000000000000000000000000000000000".to_string(), // ETH (placeholder)
                        token0_amount: BigDecimal::from(pos.value_usd as i64),
                        token1_amount: BigDecimal::from(0),
                        liquidity: BigDecimal::from(pos.value_usd as i64),
                        tick_lower: 0,
                        tick_upper: 0,
                        fee_tier: 0,
                        chain_id: 1, // Ethereum mainnet
                        entry_token0_price_usd: None,
                        entry_token1_price_usd: None,
                        entry_timestamp: None,
                        created_at: Some(Utc::now()),
                        updated_at: Some(Utc::now()),
                    }
                }).collect();
                
                match risk_calculator.calculate_risk(&model_positions).await {
                    Ok(risk_metrics) => {
                        println!("✅ Detailed risk analysis completed!");
                        println!("\n📊 Rich Position Analysis:");
                        
                        if let defi_risk_monitor::risk::metrics::ProtocolRiskMetrics::Lido(lido_metrics) = risk_metrics {
                            for (i, position) in positions.iter().enumerate() {
                                println!("\n🎯 Position {} Analysis:", i + 1);
                                println!("{{");
                                println!("  \"id\": \"{}\",", position.id);
                                println!("  \"protocol\": \"{}\",", position.protocol);
                                println!("  \"position_type\": \"{}\",", position.position_type);
                                println!("  \"pair\": \"{}\",", position.pair);
                                println!("  \"value_usd\": {:.2},", position.value_usd);
                                println!("  \"pnl_usd\": {:.2},", position.pnl_usd);
                                println!("  \"pnl_percentage\": {:.2},", position.pnl_percentage);
                                println!("  \"risk_score\": {},", lido_metrics.overall_risk_score);
                                println!("  \"metadata\": {{");
                                
                                // Rich metadata output
                                if let Some(peg) = &lido_metrics.current_steth_peg {
                                    let peg_f64 = peg.to_f64().unwrap_or(1.0);
                                    let peg_deviation = ((1.0_f64 - peg_f64) * 100.0).abs();
                                    println!("    \"peg_price\": {:.6},", peg_f64);
                                    println!("    \"peg_deviation_percent\": {:.2},", peg_deviation);
                                }
                                
                                if let Some(total_staked) = &lido_metrics.total_staked_eth {
                                    let tvl_usd = total_staked.to_f64().unwrap_or(0.0) * 3200.0; // Approximate ETH price
                                    println!("    \"protocol_tvl_usd\": {:.0},", tvl_usd);
                                }
                                
                                if let Some(active_validators) = lido_metrics.active_validators {
                                    println!("    \"validator_count_total\": {},", active_validators);
                                }
                                
                                if let Some(queue_length) = lido_metrics.withdrawal_queue_length {
                                    let queue_time_days = (queue_length as f64 / 1000.0).max(1.0).min(14.0); // Estimate
                                    println!("    \"withdrawal_queue_time_days\": {:.0},", queue_time_days);
                                }
                                
                                // Additional risk metrics
                                println!("    \"validator_slashing_risk\": {},", lido_metrics.validator_slashing_risk);
                                println!("    \"steth_depeg_risk\": {},", lido_metrics.steth_depeg_risk);
                                println!("    \"withdrawal_queue_risk\": {},", lido_metrics.withdrawal_queue_risk);
                                println!("    \"protocol_governance_risk\": {},", lido_metrics.protocol_governance_risk);
                                println!("    \"validator_performance_risk\": {},", lido_metrics.validator_performance_risk);
                                println!("    \"liquidity_risk\": {},", lido_metrics.liquidity_risk);
                                println!("    \"smart_contract_risk\": {}", lido_metrics.smart_contract_risk);
                                
                                if let Some(apy) = &lido_metrics.apy {
                                    println!("    \"current_apy\": {:.2}", apy.to_f64().unwrap_or(0.0));
                                }
                                
                                println!("  }}");
                                println!("}}");
                            }
                        }
                    }
                    Err(e) => {
                        println!("❌ Failed to perform detailed risk analysis: {}", e);
                    }
                }

                // Test 4: Position value calculation
                println!("\n💰 Calculating position values...");
                for (i, position) in positions.iter().enumerate() {
                    match lido_adapter.get_position_value(position).await {
                        Ok(value) => {
                            println!("✅ Position {} value: ${:.2}", i + 1, value);
                        }
                        Err(e) => {
                            println!("❌ Failed to calculate position {} value: {}", i + 1, e);
                        }
                    }
                }
            } else {
                println!("ℹ️  No positions found for this address");
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch positions: {}", e);
            return Err(e.into());
        }
    }

    // Test 5: Protocol information
    println!("\n🏥 Checking Lido protocol information...");
    println!("✅ Protocol name: {}", lido_adapter.protocol_name());
    println!("📆 Adapter successfully initialized and functional");

    println!("\n🎉 Lido adapter testing completed!");
    Ok(())
}
