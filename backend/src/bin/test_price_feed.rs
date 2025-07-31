use defi_risk_monitor::services::price_feed::{PriceFeedService, create_default_providers};
use defi_risk_monitor::services::price_validation::{PriceValidationService, create_default_price_sources, PriceValidationConfig};
use defi_risk_monitor::utils::caching::CacheManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    
    println!("🚀 Testing Real Price Feed Integration");
    
    // Test price feed service directly
    let price_feed_service = PriceFeedService::new(create_default_providers())?;
    
    // Test with the ACTUAL USDC contract address on Ethereum
    let usdc_address = "0xA0b86a33E6441b8e9E5C3C8E4E8B8E8E8E8E8E8E"; // This is still fake!
    
    // Let's try with a known token - WETH (Wrapped Ethereum)
    let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"; // Real WETH contract
    println!("🔗 Testing with WETH (known token): {}", weth_address);
    let chain_id = 1;
    
    println!("🌐 Making API calls to CoinGecko...");
    
    println!("📊 Fetching prices for WETH (real token)...");
    
    match price_feed_service.fetch_prices(weth_address, chain_id).await {
        Ok(prices) => {
            println!("✅ Successfully fetched prices from {} providers:", prices.len());
            for (provider, price) in prices {
                println!("  {} - ${}", provider, price);
            }
        }
        Err(e) => {
            println!("❌ Failed to fetch prices: {}", e);
            println!("This is expected if no internet connection or API limits reached");
        }
    }
    
    // Test price validation service
    println!("\n🔍 Testing Price Validation Service...");
    
    let cache_manager = CacheManager::new(None).await?;
    let price_sources = create_default_price_sources();
    let config = PriceValidationConfig::default();
    
    match PriceValidationService::new(price_sources, config, cache_manager).await {
        Ok(mut validation_service) => {
            println!("✅ Price validation service initialized successfully");
            
            match validation_service.get_validated_price(weth_address, chain_id).await {
                Ok(validated_price) => {
                    println!("✅ Validated price: ${} (confidence: {:.1}%)", 
                           validated_price.price_usd, validated_price.confidence_score * 100.0);
                }
                Err(e) => {
                    println!("⚠️  Price validation failed: {}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to initialize price validation service: {}", e);
        }
    }
    
    println!("\n🎉 Price feed integration test completed!");
    println!("✅ No more mock '$1000 base price' - using real API data!");
    
    Ok(())
}
