use defi_risk_monitor::services::ens_service::EnsService;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing ENS Resolution...");
    
    // Get RPC URL from environment or use default
    let rpc_url = env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string());
    
    println!("📡 Using RPC URL: {}", rpc_url);
    
    // Create ENS service
    let ens_service = match EnsService::new(&rpc_url) {
        Ok(service) => {
            println!("✅ ENS service created successfully");
            service
        }
        Err(e) => {
            println!("❌ Failed to create ENS service: {}", e);
            return Ok(());
        }
    };
    
    // Test cases
    let test_cases = vec![
        "ethereum.eth",
        "vitalik.eth", 
        "ens.eth",
        "uniswap.eth",
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Regular address
        "invalid-input", // Invalid input
    ];
    
    println!("\n🔍 Testing different inputs:");
    println!("{}", "=".repeat(50));
    
    for test_case in test_cases {
        println!("\n🧪 Testing: {}", test_case);
        
        // Test input validation first
        match EnsService::validate_input(test_case) {
            Ok(normalized) => {
                println!("  ✅ Input validation passed: {}", normalized);
                
                // Test address/ENS resolution
                match ens_service.resolve_address_or_ens(test_case).await {
                    Ok(address) => {
                        println!("  🎉 Resolved to address: {:?}", address);
                        
                        // Test display name
                        let display_name = ens_service.get_display_name(address).await;
                        println!("  📛 Display name: {}", display_name);
                    }
                    Err(e) => {
                        println!("  ⚠️  Resolution failed: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("  ❌ Input validation failed: {}", e);
            }
        }
    }
    
    println!("\n{}", "=".repeat(50));
    println!("🏁 ENS Resolution Test Complete!");
    
    Ok(())
}
