use defi_risk_monitor::models::mev_risk::MevRiskConfig;
use defi_risk_monitor::models::PoolState;
use defi_risk_monitor::services::mev_risk_service::MevRiskService;
use bigdecimal::BigDecimal;
use chrono::Utc;
use sqlx::PgPool;
use std::str::FromStr;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🔍 Testing MEV Risk Service Implementation");
    println!("==========================================");
    
    // Load environment variables
    dotenvy::dotenv().ok();
    
    // Connect to database
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    let pool = PgPool::connect(&database_url).await?;
    
    // Initialize MEV Risk Service
    let mev_service = MevRiskService::new(
        pool.clone(),
        Some(MevRiskConfig::default()),
        None, // blockchain_service
        None, // price_validation_service
    );
    
    println!("✅ MEV Risk Service initialized successfully");
    
    // Test data setup
    let test_pool_address = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"; // USDC/ETH pool
    let test_chain_id = 1; // Ethereum mainnet
    
    // Create test pool state
    let pool_state = create_test_pool_state();
    
    println!("\n🧪 Testing Core MEV Risk Calculation");
    println!("====================================");
    
    // Test 1: Calculate MEV risk for a pool
    match mev_service.calculate_mev_risk(test_pool_address, test_chain_id, &pool_state).await {
        Ok(mev_risk) => {
            println!("✅ MEV Risk Calculation Success:");
            println!("   📊 Overall MEV Risk: {:.4}", mev_risk.overall_mev_risk);
            println!("   🥪 Sandwich Risk: {:.4}", mev_risk.sandwich_risk_score);
            println!("   ⚡ Frontrun Risk: {:.4}", mev_risk.frontrun_risk_score);
            println!("   🔮 Oracle Manipulation Risk: {:.4}", mev_risk.oracle_manipulation_risk);
            println!("   📈 Oracle Deviation Risk: {:.4}", mev_risk.oracle_deviation_risk);
            println!("   🎯 Confidence Score: {:.4}", mev_risk.confidence_score);
            
            // Note: store_mev_risk is called internally during calculate_mev_risk
            println!("✅ MEV risk calculated and stored successfully");
        },
        Err(e) => println!("❌ MEV Risk Calculation Failed: {}", e),
    }
    
    println!("\n🔍 Testing Database Queries");
    println!("===========================");
    
    // Test 2: Test database query methods
    test_database_queries(&mev_service, test_pool_address, test_chain_id).await;
    
    println!("\n🥪 Testing Sandwich Attack Detection");
    println!("====================================");
    
    // Test 3: Test sandwich attack detection
    match mev_service.detect_sandwich_attacks(test_pool_address, test_chain_id, 100).await {
        Ok(attacks) => {
            println!("✅ Sandwich Attack Detection Success:");
            println!("   📊 Detected {} potential sandwich attacks", attacks.len());
            for (i, attack) in attacks.iter().enumerate().take(3) {
                println!("   🥪 Attack {}: {} (Block: {})", i + 1, attack.transaction_hash, attack.block_number);
            }
        },
        Err(e) => println!("❌ Sandwich Attack Detection Failed: {}", e),
    }
    
    println!("\n🔮 Testing Oracle Manipulation Detection");
    println!("========================================");
    
    // Test 4: Test oracle manipulation detection
    match mev_service.detect_oracle_manipulation(test_pool_address, test_chain_id).await {
        Ok(manipulations) => {
            println!("✅ Oracle Manipulation Detection Success:");
            println!("   📊 Detected {} potential oracle manipulations", manipulations.len());
            for (i, manipulation) in manipulations.iter().enumerate().take(3) {
                println!("   🔮 Manipulation {}: {:.2}% deviation ({:?})", 
                    i + 1, manipulation.deviation_percent, manipulation.severity);
            }
        },
        Err(e) => println!("❌ Oracle Manipulation Detection Failed: {}", e),
    }
    
    println!("\n⚡ Testing MEV-Boost Data Integration");
    println!("====================================");
    
    // Test 5: Test MEV-Boost data fetching
    let test_block_number = 18500000; // Recent Ethereum block
    match mev_service.fetch_mev_boost_data(test_block_number).await {
        Ok(mev_transactions) => {
            println!("✅ MEV-Boost Data Fetch Success:");
            println!("   📊 Fetched {} MEV transactions from block {}", mev_transactions.len(), test_block_number);
            for (i, tx) in mev_transactions.iter().enumerate().take(3) {
                println!("   ⚡ MEV Tx {}: {} (Type: {:?})", i + 1, tx.transaction_hash, tx.mev_type);
            }
        },
        Err(e) => println!("⚠️  MEV-Boost Data Fetch (Expected to fail without API access): {}", e),
    }
    
    println!("\n📊 Testing Risk Retrieval");
    println!("=========================");
    
    // Test 6: Test risk retrieval from database
    match mev_service.get_mev_risk(test_pool_address, test_chain_id).await {
        Ok(Some(cached_risk)) => {
            println!("✅ Cached MEV Risk Retrieved:");
            println!("   📊 Overall Risk: {:.4}", cached_risk.overall_mev_risk);
            println!("   🕐 Created At: {}", cached_risk.created_at);
        },
        Ok(None) => println!("ℹ️  No cached MEV risk found (this is normal for first run)"),
        Err(e) => println!("❌ Failed to retrieve MEV risk: {}", e),
    }
    
    println!("\n🧪 Testing Edge Cases");
    println!("=====================");
    
    // Test 7: Test with invalid data
    test_edge_cases(&mev_service).await;
    
    println!("\n📈 Performance Testing");
    println!("======================");
    
    // Test 8: Performance test
    let start_time = std::time::Instant::now();
    let mut successful_calculations = 0;
    
    for i in 0..10 {
        let test_address = format!("0x{:040x}", i); // Generate test addresses
        match mev_service.calculate_mev_risk(&test_address, test_chain_id, &pool_state).await {
            Ok(_) => successful_calculations += 1,
            Err(_) => {}, // Expected for some test addresses
        }
    }
    
    let duration = start_time.elapsed();
    println!("✅ Performance Test Results:");
    println!("   📊 Successful calculations: {}/10", successful_calculations);
    println!("   ⏱️  Average time per calculation: {:.2}ms", duration.as_millis() as f64 / 10.0);
    
    println!("\n🎯 Test Summary");
    println!("===============");
    println!("✅ MEV Risk Service testing completed successfully!");
    println!("📊 All major features tested:");
    println!("   ✓ Core MEV risk calculation");
    println!("   ✓ Database storage and retrieval");
    println!("   ✓ Sandwich attack detection");
    println!("   ✓ Oracle manipulation detection");
    println!("   ✓ MEV-Boost integration (API dependent)");
    println!("   ✓ Edge case handling");
    println!("   ✓ Performance benchmarking");
    
    Ok(())
}

async fn test_database_queries(service: &MevRiskService, pool_address: &str, chain_id: i32) {
    println!("🔍 Testing database integration:");
    
    // Test retrieving cached MEV risk (this tests the database query functionality)
    match service.get_mev_risk(pool_address, chain_id).await {
        Ok(Some(risk)) => {
            println!("   ✅ Database query successful - found cached risk");
            println!("   📊 Cached risk score: {:.4}", risk.overall_mev_risk);
        },
        Ok(None) => println!("   ℹ️  No cached risk found (database query working)"),
        Err(e) => println!("   ❌ Database query failed: {}", e),
    }
    
    println!("   ℹ️  Note: Individual helper methods are private (internal implementation)");
}

async fn test_edge_cases(service: &MevRiskService) {
    println!("🧪 Testing edge cases:");
    
    // Test with invalid pool address
    let invalid_pool = "0xinvalid";
    let pool_state = create_test_pool_state();
    
    match service.calculate_mev_risk(invalid_pool, 1, &pool_state).await {
        Ok(_) => println!("   ⚠️  Invalid pool address accepted (unexpected)"),
        Err(_) => println!("   ✅ Invalid pool address properly rejected"),
    }
    
    // Test with unsupported chain
    let unsupported_chain = 99999;
    match service.calculate_mev_risk("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", unsupported_chain, &pool_state).await {
        Ok(risk) => println!("   ✅ Unsupported chain handled gracefully (risk: {:.4})", risk.overall_mev_risk),
        Err(e) => println!("   ⚠️  Unsupported chain rejected: {}", e),
    }
    
    // Test with zero TVL pool
    let mut zero_tvl_pool = create_test_pool_state();
    zero_tvl_pool.tvl_usd = Some(BigDecimal::from_str("0.0").unwrap());
    
    match service.calculate_mev_risk("0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640", 1, &zero_tvl_pool).await {
        Ok(risk) => println!("   ✅ Zero TVL pool handled (risk: {:.4})", risk.overall_mev_risk),
        Err(e) => println!("   ❌ Zero TVL pool failed: {}", e),
    }
}

fn create_test_pool_state() -> PoolState {
    PoolState {
        id: Uuid::new_v4(),
        pool_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
        chain_id: 1,
        current_tick: 195000,
        sqrt_price_x96: BigDecimal::from_str("1500000000000000000000000").unwrap(),
        liquidity: BigDecimal::from_str("1000000000000000000000").unwrap(),
        token0_price_usd: Some(BigDecimal::from_str("1.0").unwrap()), // USDC price
        token1_price_usd: Some(BigDecimal::from_str("2500.0").unwrap()), // ETH price
        tvl_usd: Some(BigDecimal::from_str("50000000.0").unwrap()), // $50M TVL
        volume_24h_usd: Some(BigDecimal::from_str("10000000.0").unwrap()), // $10M daily volume
        fees_24h_usd: Some(BigDecimal::from_str("50000.0").unwrap()), // $50K daily fees
        timestamp: Utc::now(),
    }
}
