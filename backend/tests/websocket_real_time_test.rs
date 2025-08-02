use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use std::str::FromStr;

use defi_risk_monitor::services::{
    WebSocketService, MonitoringService, RealTimeRiskService,
    risk_calculator::RiskMetrics,
};
use defi_risk_monitor::config::Settings;
use defi_risk_monitor::models::Position;

/// Test WebSocket real-time risk updates integration
#[tokio::test]
async fn test_websocket_real_time_risk_updates() {
    println!("🧪 Testing WebSocket Real-Time Risk Updates...");

    // Initialize WebSocket service
    let websocket_service = Arc::new(WebSocketService::new());
    println!("✅ WebSocket service initialized");

    // Create mock database pool (simplified for testing)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/defi_risk_monitor_test".to_string());
    
    let db_pool = match sqlx::PgPool::connect(&database_url).await {
        Ok(pool) => pool,
        Err(_) => {
            println!("⚠️  Database not available, using mock services");
            return; // Skip test if database not available
        }
    };

    // Initialize settings
    let settings = Settings::new().expect("Failed to load settings");

    // Initialize monitoring service
    let mut monitoring_service = MonitoringService::new(db_pool.clone(), settings.clone())
        .expect("Failed to initialize monitoring service");
    monitoring_service.set_websocket_service(websocket_service.clone());
    let monitoring_service = Arc::new(monitoring_service);

    // Initialize real-time risk service
    let real_time_service = RealTimeRiskService::new(
        monitoring_service.clone(),
        websocket_service.clone(),
    );

    println!("✅ Real-time risk service initialized");

    // Test 1: Service startup and statistics
    println!("🔍 Test 1: Service startup and statistics");
    
    assert!(!real_time_service.is_running().await, "Service should not be running initially");
    
    let stats = real_time_service.get_stats().await.expect("Failed to get stats");
    println!("📊 Initial stats: {} connected clients", stats.connected_clients);
    assert_eq!(stats.connected_clients, 0, "Should have no connected clients initially");

    // Test 2: WebSocket service functionality
    println!("🔍 Test 2: WebSocket service functionality");
    
    let test_position_id = Uuid::new_v4();
    let test_metrics = RiskMetrics {
        impermanent_loss: BigDecimal::from_str("0.08").unwrap(),
        price_impact: BigDecimal::from_str("0.03").unwrap(),
        volatility_score: BigDecimal::from(6),
        correlation_score: BigDecimal::from(7),
        liquidity_score: BigDecimal::from(5),
        overall_risk_score: BigDecimal::from(7),
        value_at_risk_1d: BigDecimal::from(1500),
        value_at_risk_7d: BigDecimal::from(4500),
        tvl_risk: BigDecimal::from(3),
        slippage_risk: BigDecimal::from(2),
        thin_pool_risk: BigDecimal::from(2),
        tvl_drop_risk: BigDecimal::from(3),
        max_estimated_slippage: BigDecimal::from_str("0.02").unwrap(),
        protocol_risk_score: BigDecimal::from(3),
        audit_risk: BigDecimal::from(2),
        exploit_history_risk: BigDecimal::from(2),
        governance_risk: BigDecimal::from(3),
        mev_risk_score: BigDecimal::from(2),
        sandwich_attack_risk: BigDecimal::from(2),
        frontrun_risk: BigDecimal::from(2),
        oracle_manipulation_risk: BigDecimal::from(2),
        oracle_deviation_risk: BigDecimal::from(2),
        cross_chain_risk_score: BigDecimal::from(4),
        bridge_risk_score: BigDecimal::from(4),
        liquidity_fragmentation_risk: BigDecimal::from(2),
        governance_divergence_risk: BigDecimal::from(2),
        technical_risk_score: BigDecimal::from(3),
        correlation_risk_score: BigDecimal::from(4),
    };

    // Test sending risk update
    let result = websocket_service.send_risk_update(test_position_id, test_metrics.clone()).await;
    assert!(result.is_ok(), "Should successfully send risk update");
    println!("✅ Risk update sent successfully");

    // Test sending position update
    let result = websocket_service.send_position_update(
        test_position_id,
        BigDecimal::from(50000),
        BigDecimal::from(-2500),
        BigDecimal::from_str("0.05").unwrap(),
    ).await;
    assert!(result.is_ok(), "Should successfully send position update");
    println!("✅ Position update sent successfully");

    // Test sending market update
    let result = websocket_service.send_market_update(
        "0xA0b86a33E6441E1e0f6d8E87A4e5C7b7F0E8A4C1".to_string(),
        BigDecimal::from(2100),
        BigDecimal::from_str("5.2").unwrap(),
        BigDecimal::from_str("0.15").unwrap(),
    ).await;
    assert!(result.is_ok(), "Should successfully send market update");
    println!("✅ Market update sent successfully");

    // Test 3: Real-time service manual trigger
    println!("🔍 Test 3: Manual position update trigger");
    
    let result = real_time_service.trigger_position_update(test_position_id).await;
    assert!(result.is_ok(), "Should successfully trigger position update");
    println!("✅ Manual position update triggered successfully");

    // Test 4: Service lifecycle
    println!("🔍 Test 4: Service lifecycle management");
    
    // Start the service (but don't wait for full monitoring cycle)
    let start_result = timeout(Duration::from_secs(2), real_time_service.start()).await;
    match start_result {
        Ok(result) => {
            assert!(result.is_ok(), "Should successfully start real-time service");
            println!("✅ Real-time service started successfully");
        }
        Err(_) => {
            println!("⚠️  Service start timed out (expected for background service)");
        }
    }

    // Check if service is running
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(real_time_service.is_running().await, "Service should be running after start");

    // Stop the service
    real_time_service.stop().await;
    assert!(!real_time_service.is_running().await, "Service should not be running after stop");
    println!("✅ Service stopped successfully");

    // Test 5: WebSocket subscription statistics
    println!("🔍 Test 5: WebSocket subscription statistics");
    
    let stats = websocket_service.get_subscription_stats().await;
    println!("📊 Subscription stats: {:?}", stats);
    // Note: In a real test with connected clients, we would verify subscription counts

    println!("🎉 All WebSocket Real-Time Risk Update tests passed!");
}

/// Test WebSocket message broadcasting
#[tokio::test]
async fn test_websocket_message_broadcasting() {
    println!("🧪 Testing WebSocket Message Broadcasting...");

    let websocket_service = Arc::new(WebSocketService::new());
    
    // Test different message types
    let test_cases = vec![
        ("Risk Update", "risk_update"),
        ("Position Update", "position_update"),
        ("Market Update", "market_update"),
        ("System Status", "system_status"),
    ];

    for (name, test_type) in test_cases {
        println!("🔍 Testing {}", name);
        
        let result = match test_type {
            "risk_update" => {
                let metrics = RiskMetrics {
                    impermanent_loss: BigDecimal::from_str("0.03").unwrap(),
                    price_impact: BigDecimal::from_str("0.01").unwrap(),
                    volatility_score: BigDecimal::from(4),
                    correlation_score: BigDecimal::from(5),
                    liquidity_score: BigDecimal::from(3),
                    overall_risk_score: BigDecimal::from(5),
                    value_at_risk_1d: BigDecimal::from(1000),
                    value_at_risk_7d: BigDecimal::from(3000),
                    tvl_risk: BigDecimal::from(2),
                    slippage_risk: BigDecimal::from(1),
                    thin_pool_risk: BigDecimal::from(1),
                    tvl_drop_risk: BigDecimal::from(2),
                    max_estimated_slippage: BigDecimal::from_str("0.005").unwrap(),
                    protocol_risk_score: BigDecimal::from(2),
                    audit_risk: BigDecimal::from(1),
                    exploit_history_risk: BigDecimal::from(1),
                    governance_risk: BigDecimal::from(2),
                    mev_risk_score: BigDecimal::from(1),
                    sandwich_attack_risk: BigDecimal::from(1),
                    frontrun_risk: BigDecimal::from(1),
                    oracle_manipulation_risk: BigDecimal::from(1),
                    oracle_deviation_risk: BigDecimal::from(1),
                    cross_chain_risk_score: BigDecimal::from(2),
                    bridge_risk_score: BigDecimal::from(2),
                    liquidity_fragmentation_risk: BigDecimal::from(1),
                    governance_divergence_risk: BigDecimal::from(1),
                    technical_risk_score: BigDecimal::from(2),
                    correlation_risk_score: BigDecimal::from(3),
                };
                websocket_service.send_risk_update(Uuid::new_v4(), metrics).await
            },
            "position_update" => {
                websocket_service.send_position_update(
                    Uuid::new_v4(),
                    BigDecimal::from(25000),
                    BigDecimal::from(1250),
                    BigDecimal::from_str("0.02").unwrap(),
                ).await
            },
            "market_update" => {
                websocket_service.send_market_update(
                    "0xTest123".to_string(),
                    BigDecimal::from(1800),
                    BigDecimal::from_str("-2.1").unwrap(),
                    BigDecimal::from_str("0.12").unwrap(),
                ).await
            },
            "system_status" => {
                websocket_service.send_system_status(
                    "healthy".to_string(),
                    "All systems operational".to_string(),
                ).await
            },
            _ => panic!("Unknown test type"),
        };

        assert!(result.is_ok(), "{} should broadcast successfully", name);
        println!("✅ {} broadcast successful", name);
    }

    println!("🎉 All WebSocket message broadcasting tests passed!");
}

/// Test WebSocket service statistics and monitoring
#[tokio::test]
async fn test_websocket_service_statistics() {
    println!("🧪 Testing WebSocket Service Statistics...");

    let websocket_service = Arc::new(WebSocketService::new());
    
    // Test initial statistics
    let client_count = websocket_service.get_connected_clients_count().await;
    assert_eq!(client_count, 0, "Should have no connected clients initially");
    println!("✅ Initial client count: {}", client_count);

    let subscription_stats = websocket_service.get_subscription_stats().await;
    assert!(subscription_stats.is_empty(), "Should have no subscriptions initially");
    println!("✅ Initial subscription stats: {:?}", subscription_stats);

    // Start heartbeat task
    websocket_service.start_heartbeat_task();
    println!("✅ Heartbeat task started");

    // Wait a bit and verify service is still functional
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let client_count_after = websocket_service.get_connected_clients_count().await;
    assert_eq!(client_count_after, 0, "Client count should remain 0 without connections");
    println!("✅ Client count after heartbeat: {}", client_count_after);

    println!("🎉 WebSocket service statistics tests passed!");
}
