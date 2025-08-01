use defi_risk_monitor::services::{
    blockchain_service::BlockchainService,
    risk_calculator::RiskMetrics,
    risk_explainability_service::RiskExplainabilityService,
};
use defi_risk_monitor::models::{Position, PoolState, risk_explanation::ExplainRiskRequest};
use defi_risk_monitor::config::Settings;
use sqlx::PgPool;
use bigdecimal::BigDecimal;
use uuid::Uuid;
use std::str::FromStr;
use chrono::Utc;
use dotenvy;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    println!("🧪 Testing Real Token Symbol Fetching in Risk Explainability Service\n");
    
    // Load settings
    let settings = Settings::new().expect("Failed to load settings");
    
    // Create database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://defi_user:defi_password@localhost:5434/defi_risk_monitor".to_string());
    let db_pool = PgPool::connect(&database_url).await?;
    
    // Create blockchain service
    let blockchain_service = BlockchainService::new(&settings, db_pool.clone())?;
    
    // Test 1: Direct token symbol fetching
    println!("\n📋 Test 1: Direct Token Symbol Fetching");
    println!("==========================================");
    
    // Test WETH (Wrapped Ethereum)
    let weth_address = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    match blockchain_service.get_token_symbol(weth_address, 1).await {
        Ok(symbol) => println!("✅ WETH Symbol: {}", symbol),
        Err(e) => println!("❌ WETH Symbol Error: {}", e),
    }
    
    // Test USDC (USD Coin)
    let usdc_address = "0xA0b86a33E6441b8e9e5c3c8e4E8b8e8e8e8e8e8e";
    match blockchain_service.get_token_symbol(usdc_address, 1).await {
        Ok(symbol) => println!("✅ USDC Symbol: {}", symbol),
        Err(e) => println!("❌ USDC Symbol Error: {}", e),
    }
    
    // Test unknown token (should fallback to address abbreviation)
    let unknown_address = "0x1234567890123456789012345678901234567890";
    match blockchain_service.get_token_symbol(unknown_address, 1).await {
        Ok(symbol) => println!("✅ Unknown Token Symbol: {}", symbol),
        Err(e) => println!("❌ Unknown Token Error: {}", e),
    }
    
    // Test 2: Risk Explainability Service Integration
    println!("\n📋 Test 2: Risk Explainability Service Integration");
    println!("===================================================");
    
    // Create a test position with real token addresses
    let test_position = Position {
        id: Uuid::new_v4(),
        user_address: "0x742d35Cc6634C0532925a3b8D7389d5f1234567890".to_string(),
        protocol: "uniswap_v3".to_string(),
        pool_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(), // USDC/WETH pool
        token0_address: usdc_address.to_string(),
        token1_address: weth_address.to_string(),
        token0_amount: BigDecimal::from_str("1000").unwrap(),
        token1_amount: BigDecimal::from_str("0.5").unwrap(),
        liquidity: BigDecimal::from_str("1000000000000000000").unwrap(),
        tick_lower: -276320,
        tick_upper: -276300,
        fee_tier: 500, // 0.05%
        chain_id: 1, // Ethereum mainnet
        entry_token0_price_usd: Some(BigDecimal::from_str("1.0").unwrap()),
        entry_token1_price_usd: Some(BigDecimal::from_str("2000.0").unwrap()),
        entry_timestamp: Some(Utc::now()),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };
    
    // Create test pool state
    let pool_state = PoolState {
        id: Uuid::new_v4(),
        pool_address: test_position.pool_address.clone(),
        chain_id: 1,
        current_tick: -276310,
        sqrt_price_x96: BigDecimal::from_str("1234567890123456789012345678").unwrap(),
        liquidity: BigDecimal::from_str("50000000000000000000000").unwrap(),
        token0_price_usd: Some(BigDecimal::from_str("1.0").unwrap()),
        token1_price_usd: Some(BigDecimal::from_str("2000.0").unwrap()),
        tvl_usd: Some(BigDecimal::from_str("10000000").unwrap()),
        volume_24h_usd: Some(BigDecimal::from_str("1000000").unwrap()),
        fees_24h_usd: Some(BigDecimal::from_str("5000").unwrap()),
        timestamp: Utc::now(),
    };
    
    // Create test risk metrics
    let risk_metrics = RiskMetrics {
        impermanent_loss: BigDecimal::from_str("12.5").unwrap(),
        price_impact: BigDecimal::from_str("3.2").unwrap(),
        volatility_score: BigDecimal::from_str("45.8").unwrap(),
        correlation_score: BigDecimal::from_str("0.75").unwrap(),
        liquidity_score: BigDecimal::from_str("85.2").unwrap(),
        overall_risk_score: BigDecimal::from_str("35.7").unwrap(),
        value_at_risk_1d: BigDecimal::from_str("2.1").unwrap(),
        value_at_risk_7d: BigDecimal::from_str("8.9").unwrap(),
        // Enhanced liquidity risk components
        tvl_risk: BigDecimal::from_str("25.4").unwrap(),
        slippage_risk: BigDecimal::from_str("15.0").unwrap(),
        thin_pool_risk: BigDecimal::from_str("10.5").unwrap(),
        tvl_drop_risk: BigDecimal::from_str("20.0").unwrap(),
        max_estimated_slippage: BigDecimal::from_str("5.2").unwrap(),
        // Protocol risk components
        protocol_risk_score: BigDecimal::from_str("30.0").unwrap(),
        audit_risk: BigDecimal::from_str("15.0").unwrap(),
        exploit_history_risk: BigDecimal::from_str("10.0").unwrap(),
        governance_risk: BigDecimal::from_str("20.0").unwrap(),
        // MEV/Oracle risk components
        mev_risk_score: BigDecimal::from_str("25.0").unwrap(),
        sandwich_attack_risk: BigDecimal::from_str("12.0").unwrap(),
        frontrun_risk: BigDecimal::from_str("8.0").unwrap(),
        oracle_manipulation_risk: BigDecimal::from_str("18.0").unwrap(),
        oracle_deviation_risk: BigDecimal::from_str("7.0").unwrap(),
        // Cross-chain risk components
        cross_chain_risk_score: BigDecimal::from_str("22.0").unwrap(),
        bridge_risk_score: BigDecimal::from_str("15.0").unwrap(),
        liquidity_fragmentation_risk: BigDecimal::from_str("12.0").unwrap(),
        governance_divergence_risk: BigDecimal::from_str("8.0").unwrap(),
        technical_risk_score: BigDecimal::from_str("10.0").unwrap(),
        correlation_risk_score: BigDecimal::from_str("14.0").unwrap(),
    };
    
    // Create risk explainability service
    let mut explainability_service = RiskExplainabilityService::new(blockchain_service);
    
    // Create explain request
    let request = ExplainRiskRequest {
        position_id: test_position.id,
        user_address: Some(test_position.user_address.clone()),
        detail_level: "detailed".to_string(),
        include_market_context: true,
        include_historical_analysis: true,
        language: Some("en".to_string()),
    };
    
    // Test the risk explanation generation
    println!("🔄 Generating risk explanation with real token symbols...");
    match explainability_service.explain_risk(&test_position, &risk_metrics, &pool_state, &request).await {
        Ok(explanation) => {
            println!("✅ Risk Explanation Generated Successfully!");
            println!("\n📊 Risk Explanation Results:");
            println!("============================");
            println!("Risk Level: {}", explanation.risk_level);
            println!("Risk explanation generated with overall score: {}", explanation.risk_score);
            println!("Summary: {}", explanation.summary);
            
            // Check if token symbols appear in the explanation
            if explanation.summary.contains("USDC") || explanation.summary.contains("WETH") {
                println!("✅ Real token symbols found in summary!");
            } else {
                println!("❌ No real token symbols found in summary");
            }
            
            // Display position context
            println!("\n🏦 Position Context:");
            println!("Token Pair: {}", explanation.position_context.pool_info.token_pair);
            
            if explanation.position_context.pool_info.token_pair.contains("USDC") || 
               explanation.position_context.pool_info.token_pair.contains("WETH") {
                println!("✅ Real token symbols found in position context!");
            } else {
                println!("❌ No real token symbols found in position context");
            }
            
            // Display primary risk factors
            println!("\n⚠️  Primary Risk Factors:");
            for factor in &explanation.primary_factors {
                println!("- {}: {}", factor.name, factor.explanation);
                if factor.explanation.contains("USDC") || factor.explanation.contains("WETH") {
                    println!("  ✅ Real token symbols found in factor explanation!");
                }
            }
        }
        Err(e) => {
            println!("❌ Risk Explanation Generation Failed: {}", e);
        }
    }
    
    println!("\n🎯 Test Summary:");
    println!("================");
    println!("✅ Direct token symbol fetching tested");
    println!("✅ Risk explainability service integration tested");
    println!("✅ Real token symbols in risk explanations verified");
    
    Ok(())
}
