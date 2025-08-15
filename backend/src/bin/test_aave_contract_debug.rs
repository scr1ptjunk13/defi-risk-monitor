// Simple test to debug Aave V3 contract calls
use alloy::primitives::Address;
use std::str::FromStr;
use std::error::Error;
use defi_risk_monitor::blockchain::EthereumClient;
use defi_risk_monitor::adapters::aave_v3::contracts::IAavePoolV3;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🔍 Testing Aave V3 contract calls directly...");
    
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Initialize Ethereum client
    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .unwrap_or_else(|_| "https://mainnet.infura.io/v3/YOUR_PROJECT_ID".to_string());
    let client = EthereumClient::new(&rpc_url).await?;
    println!("✅ Ethereum client initialized");
    
    // Test wallet address
    let test_address = Address::from_str("0xa700b4eb416be35b2911fd5dee80678ff64ff6c9")?;
    println!("🎯 Testing address: {:?}", test_address);
    
    // Aave V3 Pool address on Ethereum
    let pool_address = Address::from_str("0x87870Bce3F2c42a6C99f1b5b3c37eed3ECF86D0a")?;
    println!("🏦 Pool address: {:?}", pool_address);
    
    // Create pool contract instance
    let pool = IAavePoolV3::new(pool_address, client.provider().clone());
    
    // Test 1: Check if contract exists by calling a simple method
    println!("\n📋 Test 1: Getting reserves list...");
    match pool.getReservesList().call().await {
        Ok(reserves) => {
            println!("✅ Reserves list retrieved: {} reserves", reserves._0.len());
            for (i, reserve) in reserves._0.iter().take(3).enumerate() {
                println!("   Reserve {}: {:?}", i + 1, reserve);
            }
        }
        Err(e) => {
            println!("❌ Failed to get reserves list: {}", e);
            return Err(e.into());
        }
    }
    
    // Test 2: Try getUserAccountData with a simpler approach
    println!("\n📋 Test 2: Getting user account data...");
    match pool.getUserAccountData(test_address).call().await {
        Ok(account_data) => {
            println!("✅ Account data retrieved successfully!");
            println!("   Total Collateral: {}", account_data.totalCollateralBase);
            println!("   Total Debt: {}", account_data.totalDebtBase);
            println!("   Available Borrows: {}", account_data.availableBorrowsBase);
            println!("   Health Factor: {}", account_data.healthFactor);
        }
        Err(e) => {
            println!("❌ Failed to get account data: {}", e);
            println!("   Error details: {:?}", e);
            
            // Try to get more specific error information
            println!("   Error type: {:?}", std::any::type_name_of_val(&e));
            println!("   Error source: {:?}", e.source());
        }
    }
    
    // Test 3: Try with a different known address (Aave treasury or similar)
    println!("\n📋 Test 3: Testing with zero address...");
    let zero_address = Address::ZERO;
    match pool.getUserAccountData(zero_address).call().await {
        Ok(account_data) => {
            println!("✅ Zero address account data retrieved!");
            println!("   Total Collateral: {}", account_data.totalCollateralBase);
            println!("   Total Debt: {}", account_data.totalDebtBase);
        }
        Err(e) => {
            println!("❌ Zero address test failed: {}", e);
        }
    }
    
    println!("\n🏁 Contract debugging test completed!");
    Ok(())
}
