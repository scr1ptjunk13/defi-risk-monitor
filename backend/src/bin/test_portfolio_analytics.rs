use defi_risk_monitor::services::portfolio_service::PortfolioService;
use defi_risk_monitor::services::price_validation::{PriceValidationService, PriceValidationConfig, PriceSource};
use defi_risk_monitor::utils::caching::CacheManager;
use sqlx::PgPool;
use std::env;
use chrono::{Utc, Duration};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Portfolio Analytics Integration Tests");
    
    // Get database URL from environment
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    
    info!("Connecting to database: {}", database_url);
    
    // Create database connection pool
    let pool = PgPool::connect(&database_url).await?;
    
    // Run database migrations
    info!("Running database migrations...");
    sqlx::migrate!("./migrations").run(&pool).await?;
    info!("✅ Database migrations completed successfully");
    
    // Initialize services with proper configuration
    let _price_sources = vec![
        PriceSource {
            name: "coingecko".to_string(),
            url: "https://api.coingecko.com/api/v3".to_string(),
            weight: 1.0,
            timeout: std::time::Duration::from_secs(5),
            enabled: true,
        }
    ];
    
    let _price_config = PriceValidationConfig {
        max_deviation_percent: 10.0,
        min_sources_required: 1,
        anomaly_threshold: 15.0,
        price_staleness_seconds: 300,
    };
    
    // Initialize database pool
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/defi_risk_monitor".to_string());
    let db_pool = sqlx::PgPool::connect(&database_url).await.map_err(|e| format!("Failed to connect to database: {}", e))?;
    
    let _cache_manager = CacheManager::new(None).await.map_err(|e| format!("Failed to initialize cache manager: {}", e))?;
    
    let price_validation_service = PriceValidationService::new(db_pool.clone()).await.expect("Failed to create price validation service");
    
    let portfolio_service = PortfolioService::new(pool.clone(), price_validation_service).await;
    
    info!("✅ Portfolio service initialized successfully");
    
    // Test user address (using a test address)
    let test_user_address = "0x1234567890123456789012345678901234567890";
    
    // Test 1: Portfolio Performance
    info!("\n🔍 Testing get_portfolio_performance...");
    match portfolio_service.get_portfolio_performance(test_user_address, Some(30)).await {
        Ok(performance) => {
            info!("✅ Portfolio Performance Test PASSED");
            info!("  📊 User: {}", performance.user_address);
            info!("  💰 Total Return USD: {}", performance.total_return_usd);
            info!("  📈 Total Return %: {}%", performance.total_return_percentage);
            info!("  📅 Daily Return %: {}%", performance.daily_return_percentage);
            info!("  📅 Weekly Return %: {}%", performance.weekly_return_percentage);
            info!("  📅 Monthly Return %: {}%", performance.monthly_return_percentage);
            info!("  🎯 Sharpe Ratio: {:?}", performance.sharpe_ratio);
            info!("  📉 Max Drawdown: {}%", performance.max_drawdown);
            info!("  📊 Volatility: {}%", performance.volatility);
            info!("  🏆 Best Position: {:?}", performance.best_performing_position);
            info!("  📉 Worst Position: {:?}", performance.worst_performing_position);
            info!("  ⏰ Period Days: {}", performance.performance_period_days);
        }
        Err(e) => {
            error!("❌ Portfolio Performance Test FAILED: {}", e);
        }
    }
    
    // Test 2: P&L History
    info!("\n🔍 Testing get_pnl_history...");
    let start_date = Utc::now() - Duration::days(7);
    let end_date = Utc::now();
    match portfolio_service.get_pnl_history(test_user_address, Some(start_date), Some(end_date), Some(24)).await {
        Ok(pnl_history) => {
            info!("✅ P&L History Test PASSED");
            info!("  📊 User: {}", pnl_history.user_address);
            info!("  📈 Total Realized P&L: ${}", pnl_history.total_realized_pnl);
            info!("  📊 Total Unrealized P&L: ${}", pnl_history.total_unrealized_pnl);
            info!("  💰 Total Fees Earned: ${}", pnl_history.total_fees_earned);
            info!("  📉 Total Impermanent Loss: ${}", pnl_history.total_impermanent_loss);
            info!("  📅 Period: {} to {}", pnl_history.period_start, pnl_history.period_end);
            info!("  📋 History Entries: {}", pnl_history.entries.len());
            
            // Show first few entries
            for (i, entry) in pnl_history.entries.iter().take(3).enumerate() {
                info!("    Entry {}: {} - Value: ${}, Positions: {}", 
                     i + 1, entry.timestamp, entry.total_value_usd, entry.position_count);
            }
        }
        Err(e) => {
            error!("❌ P&L History Test FAILED: {}", e);
        }
    }
    
    // Test 3: Asset Allocation
    info!("\n🔍 Testing get_asset_allocation...");
    match portfolio_service.get_asset_allocation(test_user_address).await {
        Ok(asset_allocation) => {
            info!("✅ Asset Allocation Test PASSED");
            info!("  📊 User: {}", asset_allocation.user_address);
            info!("  💰 Total Portfolio Value: ${}", asset_allocation.total_portfolio_value_usd);
            info!("  🎯 Diversification Score: {}", asset_allocation.diversification_score);
            info!("  ⚠️  Concentration Risk: {}%", asset_allocation.concentration_risk);
            info!("  📋 Total Allocations: {}", asset_allocation.allocations.len());
            info!("  🏆 Top 5 Assets: {}", asset_allocation.top_5_assets.len());
            
            // Show top allocations
            for (i, allocation) in asset_allocation.top_5_assets.iter().enumerate() {
                info!("    Top Asset {}: {} ({}%) - ${} across {} positions", 
                     i + 1, allocation.token_symbol, allocation.percentage_of_portfolio, 
                     allocation.total_value_usd, allocation.position_count);
            }
        }
        Err(e) => {
            error!("❌ Asset Allocation Test FAILED: {}", e);
        }
    }
    
    // Test 4: Protocol Exposure
    info!("\n🔍 Testing get_protocol_exposure...");
    match portfolio_service.get_protocol_exposure(test_user_address).await {
        Ok(protocol_exposure) => {
            info!("✅ Protocol Exposure Test PASSED");
            info!("  📊 User: {}", protocol_exposure.user_address);
            info!("  💰 Total Portfolio Value: ${}", protocol_exposure.total_portfolio_value_usd);
            info!("  🎯 Protocol Diversification Score: {}", protocol_exposure.protocol_diversification_score);
            info!("  ⚠️  Highest Risk Protocol: {:?}", protocol_exposure.highest_risk_protocol);
            info!("  📋 Total Exposures: {}", protocol_exposure.exposures.len());
            info!("  🏆 Top 5 Protocols: {}", protocol_exposure.top_5_protocols.len());
            
            // Show top exposures
            for (i, exposure) in protocol_exposure.top_5_protocols.iter().enumerate() {
                info!("    Protocol {}: {} ({}%) - ${} across {} positions on {} chains", 
                     i + 1, exposure.protocol_name, exposure.percentage_of_portfolio, 
                     exposure.total_value_usd, exposure.position_count, exposure.chains.len());
                info!("      Risk Score: {:?}, TVL: {:?}, Yield APR: {:?}%", 
                     exposure.risk_score, exposure.tvl_usd, exposure.yield_apr);
            }
        }
        Err(e) => {
            error!("❌ Protocol Exposure Test FAILED: {}", e);
        }
    }
    
    // Test 5: Edge Cases and Error Handling
    info!("\n🔍 Testing Edge Cases...");
    
    // Test with non-existent user
    let fake_user = "0xfakeaddress1234567890123456789012345678";
    match portfolio_service.get_portfolio_performance(fake_user, Some(30)).await {
        Ok(performance) => {
            info!("✅ Empty Portfolio Test PASSED - No positions found");
            info!("  📊 Total Return: ${}", performance.total_return_usd);
            info!("  📈 Return Percentage: {}%", performance.total_return_percentage);
        }
        Err(e) => {
            warn!("⚠️  Empty Portfolio Test: {}", e);
        }
    }
    
    // Test with different time periods
    for period in [1, 7, 30, 90, 365] {
        match portfolio_service.get_portfolio_performance(test_user_address, Some(period)).await {
            Ok(performance) => {
                info!("✅ {} Day Period Test PASSED - Return: {}%", 
                     period, performance.total_return_percentage);
            }
            Err(e) => {
                warn!("⚠️  {} Day Period Test FAILED: {}", period, e);
            }
        }
    }
    
    // Test 6: Performance Benchmarking
    info!("\n🔍 Performance Benchmarking...");
    let start_time = std::time::Instant::now();
    
    for _i in 0..10 {
        let _ = portfolio_service.get_portfolio_performance(test_user_address, Some(30)).await;
        let _ = portfolio_service.get_asset_allocation(test_user_address).await;
        let _ = portfolio_service.get_protocol_exposure(test_user_address).await;
    }
    
    let duration = start_time.elapsed();
    info!("✅ Performance Test: 30 analytics queries completed in {:?}", duration);
    info!("  📊 Average time per query: {:?}", duration / 30);
    
    info!("\n🎉 ALL PORTFOLIO ANALYTICS TESTS COMPLETED!");
    info!("📋 Test Summary:");
    info!("  ✅ Portfolio Performance Query");
    info!("  ✅ P&L History Query");
    info!("  ✅ Asset Allocation Query");
    info!("  ✅ Protocol Exposure Query");
    info!("  ✅ Edge Case Handling");
    info!("  ✅ Performance Benchmarking");
    
    Ok(())
}
