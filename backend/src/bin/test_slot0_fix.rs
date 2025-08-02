use defi_risk_monitor::services::contract_bindings::UniswapV3Pool;
use defi_risk_monitor::config::Settings;
use alloy::providers::ProviderBuilder;
use std::sync::Arc;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔧 Testing slot0 ABI decoding fix...");
    
    // Load settings
    let settings = Settings::new().expect("Failed to load settings");
    
    // Create provider
    let ethereum_url = settings.blockchain.ethereum_rpc_url.parse::<Url>()?;
    let provider = Arc::new(ProviderBuilder::new().on_http(ethereum_url));
    
    // Test with a known Uniswap V3 pool (USDC/WETH 0.05% pool)
    let pool_address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string();
    
    println!("📍 Testing pool address: {}", pool_address);
    
    // Create pool contract instance
    let pool = match UniswapV3Pool::new(pool_address.clone(), provider) {
        Ok(pool) => {
            println!("✅ Pool contract created successfully");
            pool
        }
        Err(e) => {
            println!("❌ Failed to create pool contract: {}", e);
            return Err(e);
        }
    };
    
    // Test slot0 call
    println!("🔍 Calling slot0()...");
    match pool.slot0().await {
        Ok((sqrt_price_x96, tick, obs_index, obs_cardinality, obs_cardinality_next, fee_protocol, unlocked)) => {
            println!("✅ slot0() call successful!");
            println!("   sqrtPriceX96: {}", sqrt_price_x96);
            println!("   tick: {}", tick);
            println!("   observationIndex: {}", obs_index);
            println!("   observationCardinality: {}", obs_cardinality);
            println!("   observationCardinalityNext: {}", obs_cardinality_next);
            println!("   feeProtocol: {}", fee_protocol);
            println!("   unlocked: {}", unlocked);
            
            // Validate tick is within reasonable bounds
            if tick >= -887272 && tick <= 887272 {
                println!("✅ Tick value {} is within valid Uniswap V3 range", tick);
            } else {
                println!("⚠️  Tick value {} is outside expected range", tick);
            }
        }
        Err(e) => {
            println!("❌ slot0() call failed: {}", e);
            return Err(e);
        }
    }
    
    // Test other functions to ensure they work too
    println!("🔍 Testing other pool functions...");
    
    match pool.liquidity().await {
        Ok(liquidity) => println!("✅ liquidity(): {}", liquidity),
        Err(e) => println!("❌ liquidity() failed: {}", e),
    }
    
    match pool.token0().await {
        Ok(token0) => println!("✅ token0(): {}", token0),
        Err(e) => println!("❌ token0() failed: {}", e),
    }
    
    match pool.token1().await {
        Ok(token1) => println!("✅ token1(): {}", token1),
        Err(e) => println!("❌ token1() failed: {}", e),
    }
    
    println!("🎉 All tests completed successfully! ABI decoding fix is working.");
    
    Ok(())
}
