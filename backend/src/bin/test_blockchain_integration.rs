//! Test real blockchain integration with Uniswap V3 and Chainlink
//! This binary tests the real contract bindings without requiring database setup

use std::sync::Arc;
use alloy::providers::ProviderBuilder;
use defi_risk_monitor::services::contract_bindings::{UniswapV3Pool, ChainlinkAggregatorV3, addresses};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🚀 Testing Real Blockchain Integration with Alloy");
    println!("================================================");

    // Create provider using a free public RPC endpoint
    let provider = Arc::new(
        ProviderBuilder::new()
            .on_http("https://eth.llamarpc.com".parse()?)
    );

    println!("✅ Provider created successfully");

    // Test 1: Uniswap V3 Pool Integration
    println!("\n📊 Testing Uniswap V3 Pool Integration...");
    test_uniswap_pool(provider.clone()).await?;

    // Test 2: Chainlink Price Feed Integration
    println!("\n💰 Testing Chainlink Price Feed Integration...");
    test_chainlink_feed(provider.clone()).await?;

    println!("\n🎉 All blockchain integration tests passed!");
    println!("✅ Real contract bindings are working correctly");
    println!("✅ No more mock implementations!");

    Ok(())
}

async fn test_uniswap_pool(provider: Arc<alloy::providers::RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Test with USDC/WETH 0.05% pool (high liquidity)
    let pool_address = addresses::USDC_WETH_POOL_500;
    println!("  📍 Pool Address: {}", pool_address);

    let pool = UniswapV3Pool::new(pool_address.to_string(), provider)?;

    // Test slot0 call
    let slot0 = pool.slot0().await?;
    println!("  📈 Current Price (sqrtPriceX96): {}", slot0.0);
    println!("  📊 Current Tick: {}", slot0.1);
    println!("  🔓 Pool Unlocked: {}", slot0.6);

    // Test liquidity call
    let liquidity = pool.liquidity().await?;
    println!("  💧 Total Liquidity: {}", liquidity);

    // Test token addresses
    let token0 = pool.token0().await?;
    let token1 = pool.token1().await?;
    println!("  🪙 Token0 (USDC): {}", token0);
    println!("  🪙 Token1 (WETH): {}", token1);

    // Test fee
    let fee = pool.fee().await?;
    println!("  💸 Pool Fee: {} ({}%)", fee, fee as f64 / 10000.0);

    // Validate results
    assert!(slot0.0 > alloy::primitives::U256::ZERO, "sqrtPriceX96 should be > 0");
    assert!(liquidity > 0, "Liquidity should be > 0");
    assert!(slot0.6, "Pool should be unlocked");
    assert_eq!(fee, 500, "Should be 0.05% fee pool");

    println!("  ✅ Uniswap V3 Pool test passed!");
    Ok(())
}

async fn test_chainlink_feed(provider: Arc<alloy::providers::RootProvider<alloy::transports::http::Http<alloy::transports::http::Client>>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Test with ETH/USD price feed
    let feed_address = addresses::ETH_USD_FEED;
    println!("  📍 Feed Address: {}", feed_address);

    let feed = ChainlinkAggregatorV3::new(feed_address.to_string(), provider)?;

    // Test latest round data
    let round_data = feed.latest_round_data().await?;
    println!("  🔢 Round ID: {}", round_data.0);
    println!("  💵 ETH Price: ${}", round_data.1 as f64 / 1e8); // 8 decimals
    println!("  ⏰ Updated At: {}", round_data.3);

    // Test decimals
    let decimals = feed.decimals().await?;
    println!("  📏 Decimals: {}", decimals);

    // Test description
    let description = feed.description().await?;
    println!("  📝 Description: {}", description);

    // Validate results
    assert!(round_data.1 > 0, "Price should be positive");
    assert_eq!(decimals, 8, "ETH/USD feed should have 8 decimals");
    assert!(description.contains("ETH"), "Description should contain ETH");

    println!("  ✅ Chainlink price feed test passed!");
    Ok(())
}
