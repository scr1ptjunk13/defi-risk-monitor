//! Simple test for real blockchain integration with Uniswap V3 and Chainlink

use std::sync::Arc;
use std::str::FromStr;
use alloy::{
    providers::{ProviderBuilder, RootProvider},
    transports::http::{Client, Http},
    sol,
    primitives::{Address, U256},
};

// Uniswap V3 Pool contract ABI definitions
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
        
        function liquidity() external view returns (uint128);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
    }
}

// Chainlink Aggregator V3 contract ABI definitions
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IAggregatorV3 {
        function latestRoundData() external view returns (
            uint80 roundId,
            int256 answer,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        );
        
        function decimals() external view returns (uint8);
        function description() external view returns (string memory);
    }
}

// Contract addresses
const USDC_WETH_POOL_500: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"; // 0.05% fee
const ETH_USD_FEED: &str = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Real Blockchain Integration with Alloy");
    println!("================================================");

    // Create provider using a free public RPC endpoint (Ethereum mainnet)
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
    println!("✅ MILESTONE 1.1 COMPLETED: Real Blockchain Integration");

    Ok(())
}

async fn test_uniswap_pool(provider: Arc<RootProvider<Http<Client>>>) -> Result<(), Box<dyn std::error::Error>> {
    // Test with USDC/WETH 0.05% pool (high liquidity)
    println!("  📍 Pool Address: {}", USDC_WETH_POOL_500);

    let pool_address = Address::from_str(USDC_WETH_POOL_500)?;
    let pool = IUniswapV3Pool::new(pool_address, provider);

    // Test slot0 call
    let slot0_result = pool.slot0().call().await?;
    let sqrt_price_x96 = U256::from(slot0_result.sqrtPriceX96);
    let tick: i32 = slot0_result.tick.try_into().map_err(|_| "Tick conversion failed")?;
    
    println!("  📈 Current Price (sqrtPriceX96): {}", sqrt_price_x96);
    println!("  📊 Current Tick: {}", tick);
    println!("  🔓 Pool Unlocked: {}", slot0_result.unlocked);

    // Test liquidity call
    let liquidity_result = pool.liquidity().call().await?;
    let liquidity = liquidity_result._0;
    println!("  💧 Total Liquidity: {}", liquidity);

    // Test token addresses
    let token0_result = pool.token0().call().await?;
    let token1_result = pool.token1().call().await?;
    println!("  🪙 Token0 (USDC): {:?}", token0_result._0);
    println!("  🪙 Token1 (WETH): {:?}", token1_result._0);

    // Test fee
    let fee_result = pool.fee().call().await?;
    let fee: u32 = fee_result._0.try_into().map_err(|_| "Fee conversion failed")?;
    println!("  💸 Pool Fee: {} ({}%)", fee, fee as f64 / 10000.0);

    // Validate results
    assert!(sqrt_price_x96 > U256::ZERO, "sqrtPriceX96 should be > 0");
    assert!(liquidity > 0, "Liquidity should be > 0");
    assert!(slot0_result.unlocked, "Pool should be unlocked");
    assert_eq!(fee, 500, "Should be 0.05% fee pool");

    println!("  ✅ Uniswap V3 Pool test passed!");
    Ok(())
}

async fn test_chainlink_feed(provider: Arc<RootProvider<Http<Client>>>) -> Result<(), Box<dyn std::error::Error>> {
    // Test with ETH/USD price feed
    println!("  📍 Feed Address: {}", ETH_USD_FEED);

    let feed_address = Address::from_str(ETH_USD_FEED)?;
    let feed = IAggregatorV3::new(feed_address, provider);

    // Test latest round data
    let round_data_result = feed.latestRoundData().call().await?;
    let round_id: u64 = round_data_result.roundId.try_into().unwrap_or(0);
    let answer: i128 = round_data_result.answer.try_into().map_err(|_| "Answer conversion failed")?;
    let updated_at: u64 = round_data_result.updatedAt.try_into().unwrap_or(0);
    
    println!("  🔢 Round ID: {}", round_id);
    println!("  💵 ETH Price: ${}", answer as f64 / 1e8); // 8 decimals
    println!("  ⏰ Updated At: {}", updated_at);

    // Test decimals
    let decimals_result = feed.decimals().call().await?;
    let decimals = decimals_result._0;
    println!("  📏 Decimals: {}", decimals);

    // Test description
    let description_result = feed.description().call().await?;
    let description = &description_result._0;
    println!("  📝 Description: {}", description);

    // Validate results
    assert!(answer > 0, "Price should be positive");
    assert_eq!(decimals, 8, "ETH/USD feed should have 8 decimals");
    assert!(description.contains("ETH"), "Description should contain ETH");

    println!("  ✅ Chainlink price feed test passed!");
    Ok(())
}
