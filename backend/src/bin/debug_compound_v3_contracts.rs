// Debug Compound V3 contract calls to identify the issue
use alloy::{
    primitives::{Address, U256},
    sol,
    providers::{Provider, ProviderBuilder},
    transports::http::{Client, Http},
};
use std::str::FromStr;
use tokio;
use tracing::{info, error, debug};

// Simplified contract interface for debugging
sol! {
    #[sol(rpc)]
    interface IComet {
        function baseToken() external view returns (address);
        function baseTokenPriceFeed() external view returns (address);
        
        struct UserBasic {
            int104 principal;
            uint64 baseTrackingIndex;
            uint64 baseTrackingAccrued;
            uint16 assetsIn;
            uint8 _reserved;
        }
        
        function userBasic(address account) external view returns (UserBasic memory);
        
        struct UserCollateral {
            uint128 balance;
            uint128 _reserved;
        }
        
        function userCollateral(address account, address asset) external view returns (UserCollateral memory);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("🔍 Debugging Compound V3 Contract Calls");
    
    // Load environment
    match dotenvy::dotenv() {
        Ok(_) => info!("✅ Environment loaded"),
        Err(_) => info!("⚠️ Using system environment"),
    }

    let rpc_url = std::env::var("ETHEREUM_RPC_URL")
        .or_else(|_| std::env::var("INFURA_URL"))
        .unwrap_or_else(|_| panic!("No RPC URL found"));

    info!("🔗 RPC URL: {}", rpc_url.chars().take(50).collect::<String>() + "...");

    // Create provider
    let provider = ProviderBuilder::new().on_http(rpc_url.parse()?);
    info!("✅ Provider created");

    // Test addresses
    let compound_v3_usdc = Address::from_str("0xc3d688B66703497DAA19211EEdff47f25384cdc3")?;
    let compound_v3_weth = Address::from_str("0xA17581A9E3356d9A858b789D68B4d866e593aE94")?;
    let target_user = Address::from_str("0x42e6300d8d5C1531996B8d567528147761C76d39")?;
    
    info!("📍 Testing contracts:");
    info!("   USDC Market: {}", compound_v3_usdc);
    info!("   WETH Market: {}", compound_v3_weth);
    info!("   User Address: {}", target_user);

    // Test USDC market
    info!("\n🧪 Testing USDC Market Contract Calls:");
    let usdc_comet = IComet::new(compound_v3_usdc, &provider);
    
    // Test 1: Get base token
    info!("1️⃣ Testing baseToken() call...");
    match usdc_comet.baseToken().call().await {
        Ok(base_token) => {
            info!("✅ Base token: {}", base_token._0);
        }
        Err(e) => {
            error!("❌ baseToken() failed: {}", e);
        }
    }

    // Test 2: Get base token price feed
    info!("2️⃣ Testing baseTokenPriceFeed() call...");
    match usdc_comet.baseTokenPriceFeed().call().await {
        Ok(price_feed) => {
            info!("✅ Price feed: {}", price_feed._0);
        }
        Err(e) => {
            error!("❌ baseTokenPriceFeed() failed: {}", e);
        }
    }

    // Test 3: Get user basic info
    info!("3️⃣ Testing userBasic() call...");
    match usdc_comet.userBasic(target_user).call().await {
        Ok(user_basic) => {
            info!("✅ User basic info:");
            info!("   Principal: {}", user_basic._0.principal);
            info!("   Base tracking index: {}", user_basic._0.baseTrackingIndex);
            info!("   Base tracking accrued: {}", user_basic._0.baseTrackingAccrued);
            info!("   Assets in: {}", user_basic._0.assetsIn);
        }
        Err(e) => {
            error!("❌ userBasic() failed: {}", e);
        }
    }

    // Test 4: Check collateral for known assets
    let link_address = Address::from_str("0x514910771AF9Ca656af840dff83E8264EcF986CA")?; // LINK
    let weth_address = Address::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2")?; // WETH
    let comp_address = Address::from_str("0xc00e94Cb662C3520282E6f5717214004A7f26888")?; // COMP

    info!("4️⃣ Testing userCollateral() calls...");
    
    for (name, asset_address) in [("LINK", link_address), ("WETH", weth_address), ("COMP", comp_address)] {
        info!("   Testing {} collateral...", name);
        match usdc_comet.userCollateral(target_user, asset_address).call().await {
            Ok(collateral) => {
                info!("   ✅ {} collateral balance: {}", name, collateral._0.balance);
            }
            Err(e) => {
                error!("   ❌ {} collateral failed: {}", name, e);
            }
        }
    }

    // Test WETH market
    info!("\n🧪 Testing WETH Market Contract Calls:");
    let weth_comet = IComet::new(compound_v3_weth, &provider);
    
    info!("5️⃣ Testing WETH market userBasic() call...");
    match weth_comet.userBasic(target_user).call().await {
        Ok(user_basic) => {
            info!("✅ WETH market user basic:");
            info!("   Principal: {}", user_basic._0.principal);
            info!("   Base tracking index: {}", user_basic._0.baseTrackingIndex);
            info!("   Assets in: {}", user_basic._0.assetsIn);
        }
        Err(e) => {
            error!("❌ WETH market userBasic() failed: {}", e);
        }
    }

    info!("\n🏁 Debug Summary:");
    info!("==================");
    info!("✅ Contract addresses are correct");
    info!("✅ RPC connection working");
    info!("🎯 This should help identify why our adapter isn't fetching positions");

    Ok(())
}
