#!/usr/bin/env rust

use std::str::FromStr;
use bigdecimal::BigDecimal;
use chrono::Utc;
use tokio;

use defi_risk_monitor::config::Settings;
use defi_risk_monitor::services::AIRiskService;
use defi_risk_monitor::services::risk_calculator::RiskMetrics;
use defi_risk_monitor::models::{Position, PoolState};

/// Test the AI service integration
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("🤖 Testing AI Service Integration");
    println!("================================");
    
    // Load configuration
    let settings = Settings::default();
    println!("✅ AI Service URL: {}", settings.ai_service.url);
    println!("✅ Fallback Enabled: {}", settings.ai_service.fallback_enabled);
    
    // Create AI service
    let ai_service = AIRiskService::new(
        settings.ai_service.url.clone(),
        settings.ai_service.fallback_enabled,
    );
    
    // Test 1: Health Check
    println!("\n🔍 Test 1: AI Service Health Check");
    let is_healthy = ai_service.is_ai_service_healthy().await;
    println!("   Health Status: {}", if is_healthy { "✅ HEALTHY" } else { "❌ UNHEALTHY" });
    
    // Test 2: Model Info (if service is healthy)
    if is_healthy {
        println!("\n🔍 Test 2: AI Model Information");
        match ai_service.get_model_info().await {
            Ok(model_info) => {
                println!("   📊 Available Models:");
                for (name, info) in model_info {
                    println!("      - {}: {:?}", name, info);
                }
            }
            Err(e) => {
                println!("   ❌ Failed to get model info: {}", e);
            }
        }
    }
    
    // Test 3: AI Risk Explanation
    println!("\n🔍 Test 3: AI Risk Explanation");
    
    // Create test data
    let position = create_test_position();
    let pool_state = create_test_pool_state();
    let risk_metrics = create_test_risk_metrics();
    
    println!("   📍 Test Position: {}", position.id);
    println!("   🏊 Pool Address: {}", position.pool_address);
    println!("   ⚠️  Risk Score: {}%", risk_metrics.overall_risk_score);
    
    // Get AI explanation
    match ai_service.explain_risk_ai(&position, &risk_metrics, &pool_state).await {
        Ok(explanation) => {
            println!("   ✅ AI Explanation Generated Successfully!");
            println!("   📈 Overall Risk Score: {}", explanation.overall_risk_score);
            println!("   🎯 Confidence: {:.2}%", explanation.confidence * 100.0);
            println!("   🧠 Model Version: {}", explanation.model_version);
            println!("   📝 Method: {}", explanation.explanation_method);
            
            println!("\n   📋 Summary:");
            println!("      {}", explanation.summary);
            
            println!("\n   💡 Key Insights:");
            for (i, insight) in explanation.key_insights.iter().enumerate() {
                println!("      {}. {}", i + 1, insight);
            }
            
            println!("\n   ⚠️  Risk Factors ({}):", explanation.risk_factors.len());
            for factor in &explanation.risk_factors {
                println!("      - {}: {:.1}% importance", factor.factor_name, factor.importance_score * 100.0);
                println!("        {}", factor.explanation);
                if !factor.evidence.is_empty() {
                    println!("        Evidence: {}", factor.evidence.join(", "));
                }
            }
            
            println!("\n   🎯 Recommendations ({}):", explanation.recommendations.len());
            for rec in &explanation.recommendations {
                println!("      - {}: {} ({}% confidence)", rec.action, rec.urgency, rec.confidence * 100.0);
                println!("        {}", rec.reasoning);
                if let Some(impact) = &rec.expected_impact {
                    println!("        Expected Impact: {}", impact);
                }
            }
        }
        Err(e) => {
            println!("   ❌ AI Explanation Failed: {}", e);
            println!("   🔄 This might be expected if AI service is not running");
            
            if settings.ai_service.fallback_enabled {
                println!("   ℹ️  Fallback should have been used - check logs for details");
            }
        }
    }
    
    // Test 4: Performance Test
    println!("\n🔍 Test 4: Performance Test");
    let start_time = std::time::Instant::now();
    
    let mut successful_calls = 0;
    let mut _failed_calls = 0;
    let test_iterations = 5;
    
    for i in 1..=test_iterations {
        print!("   Testing iteration {}/{}... ", i, test_iterations);
        
        match ai_service.explain_risk_ai(&position, &risk_metrics, &pool_state).await {
            Ok(_) => {
                successful_calls += 1;
                println!("✅");
            }
            Err(_) => {
                _failed_calls += 1;
                println!("❌");
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    let avg_time = elapsed.as_millis() as f64 / test_iterations as f64;
    
    println!("\n   📊 Performance Results:");
    println!("      Total Time: {:?}", elapsed);
    println!("      Average Time: {:.2}ms per call", avg_time);
    println!("      Success Rate: {}/{} ({:.1}%)", 
             successful_calls, test_iterations, 
             (successful_calls as f64 / test_iterations as f64) * 100.0);
    
    // Summary
    println!("\n🎯 Integration Test Summary");
    println!("==========================");
    println!("✅ Configuration: Loaded successfully");
    println!("{} Health Check: {}", 
             if is_healthy { "✅" } else { "❌" }, 
             if is_healthy { "PASSED" } else { "FAILED" });
    println!("{} AI Explanation: {}", 
             if successful_calls > 0 { "✅" } else { "❌" },
             if successful_calls > 0 { "WORKING" } else { "FAILED" });
    println!("📈 Performance: {:.2}ms average", avg_time);
    
    if is_healthy && successful_calls > 0 {
        println!("\n🎉 AI SERVICE INTEGRATION: FULLY OPERATIONAL");
        println!("   The old rule-based system has been successfully replaced!");
        println!("   AI-powered risk explanations are now active.");
    } else if successful_calls > 0 {
        println!("\n⚠️  AI SERVICE INTEGRATION: PARTIALLY WORKING");
        println!("   Fallback system is operational when AI service is unavailable.");
    } else {
        println!("\n❌ AI SERVICE INTEGRATION: NEEDS ATTENTION");
        println!("   Check if Python AI service is running on {}", settings.ai_service.url);
        println!("   Run: cd ai-service && python main.py");
    }
    
    Ok(())
}

fn create_test_position() -> Position {
    use uuid::Uuid;
    Position {
        id: Uuid::new_v4(),
        user_address: "0x742d35Cc6634C0532925a3b8D8c5A4b8C8c8c8c8".to_string(),
        protocol: "uniswap_v3".to_string(),
        pool_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(), // USDC/ETH pool
        token0_address: "0xA0b86a33E6441e6C7D7b0b0b5C5D5E5F5f5f5f5f".to_string(), // USDC
        token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(), // WETH
        token0_amount: BigDecimal::from(25000), // 25k USDC
        token1_amount: BigDecimal::from(12), // 12 ETH
        liquidity: BigDecimal::from(50000), // $50k position
        tick_lower: 195000,
        tick_upper: 205000,
        fee_tier: 3000, // 0.3%
        chain_id: 1, // Ethereum
        entry_token0_price_usd: Some(BigDecimal::from(1)), // USDC = $1
        entry_token1_price_usd: Some(BigDecimal::from(2000)), // ETH = $2000
        entry_timestamp: Some(Utc::now()),
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    }
}

fn create_test_pool_state() -> PoolState {
    use uuid::Uuid;
    PoolState {
        id: Uuid::new_v4(),
        pool_address: "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640".to_string(),
        chain_id: 1,
        current_tick: 201000, // Current price tick
        sqrt_price_x96: BigDecimal::from_str("1771845812700903892492222464").unwrap(), // ~$2100 ETH
        liquidity: BigDecimal::from_str("25000000000000000000000").unwrap(), // Pool liquidity
        token0_price_usd: Some(BigDecimal::from(1)), // USDC
        token1_price_usd: Some(BigDecimal::from(2100)), // ETH price moved up
        tvl_usd: Some(BigDecimal::from(15000000)), // $15M TVL
        fees_24h_usd: Some(BigDecimal::from(50000)), // $50K daily fees
        volume_24h_usd: Some(BigDecimal::from(5000000)), // $5M daily volume
        timestamp: Utc::now(),
    }
}

fn create_test_risk_metrics() -> RiskMetrics {
    RiskMetrics {
        impermanent_loss: BigDecimal::from(3), // 3% IL from ETH price increase
        price_impact: BigDecimal::from(2), // 2% price impact
        volatility_score: BigDecimal::from(45), // Moderate volatility
        correlation_score: BigDecimal::from(35), // Moderate correlation
        liquidity_score: BigDecimal::from(25), // Good liquidity
        overall_risk_score: BigDecimal::from(55), // Medium risk
        value_at_risk_1d: BigDecimal::from(8), // 8% VaR 1 day
        value_at_risk_7d: BigDecimal::from(15), // 15% VaR 7 days
        tvl_risk: BigDecimal::from(20), // TVL risk
        slippage_risk: BigDecimal::from(10), // Slippage risk
        thin_pool_risk: BigDecimal::from(15), // Thin pool risk
        tvl_drop_risk: BigDecimal::from(12), // TVL drop risk
        max_estimated_slippage: BigDecimal::from(5), // Max slippage
        protocol_risk_score: BigDecimal::from(25), // Protocol risk
        audit_risk: BigDecimal::from(20), // Audit risk
        exploit_history_risk: BigDecimal::from(10), // Exploit history
        governance_risk: BigDecimal::from(15), // Governance risk
        mev_risk_score: BigDecimal::from(30), // MEV risk
        sandwich_attack_risk: BigDecimal::from(25), // Sandwich risk
        frontrun_risk: BigDecimal::from(20), // Frontrun risk
        oracle_manipulation_risk: BigDecimal::from(15), // Oracle manipulation
        oracle_deviation_risk: BigDecimal::from(10), // Oracle deviation
        cross_chain_risk_score: BigDecimal::from(35), // Cross-chain risk
        bridge_risk_score: BigDecimal::from(30), // Bridge risk
        liquidity_fragmentation_risk: BigDecimal::from(20), // Liquidity fragmentation
        governance_divergence_risk: BigDecimal::from(15), // Governance divergence
        technical_risk_score: BigDecimal::from(25), // Technical risk
        correlation_risk_score: BigDecimal::from(20), // Correlation risk
    }
}
