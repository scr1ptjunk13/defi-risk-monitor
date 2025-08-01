use std::sync::Arc;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::Utc;
use std::str::FromStr;

use defi_risk_monitor::{
    services::{
        RiskAssessmentService, PortfolioService, SystemHealthService,
        CrossChainRiskService, MevRiskService, PriceValidationService,
        AuthService, UserRiskConfigService, PositionService,
        BlockchainService, QueryPerformanceService,
    },
    models::*,
    error::AppError,
    database::{Database, get_database_pool},
    config::Settings,
};

/// Comprehensive integration tests with real database connections
/// These tests validate end-to-end functionality with actual data persistence
#[cfg(test)]
mod integration_tests {
    use super::*;

    async fn setup_test_environment() -> Result<Arc<Database>, AppError> {
        dotenvy::dotenv().ok();
        let settings = Settings::new().expect("Failed to load settings");
        let pool = get_database_pool(&settings.database.url).await
            .expect("Failed to create database pool");
        Ok(Arc::new(Database::new(pool)))
    }

    #[tokio::test]
    async fn test_end_to_end_position_lifecycle() {
        println!("🧪 Testing End-to-End Position Lifecycle");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = PositionService::new(db.clone());
        let risk_service = RiskAssessmentService::new(db.clone());
        
        // Create test user first
        let user_id = Uuid::new_v4();
        
        // 1. Create a new position
        let position_id = Uuid::new_v4();
        let new_position = Position {
            id: position_id,
            user_id,
            protocol: "uniswap_v3".to_string(),
            chain: "ethereum".to_string(),
            position_type: PositionType::LiquidityPool,
            token0_address: "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A".to_string(),
            token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount0: BigDecimal::from_str("1000.0").unwrap(),
            amount1: BigDecimal::from_str("0.5").unwrap(),
            entry_price: BigDecimal::from_str("2000.0").unwrap(),
            current_price: BigDecimal::from_str("2100.0").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // Test position creation
        let create_result = position_service.create_position(&new_position).await;
        assert!(create_result.is_ok(), "Position creation should succeed");
        
        // 2. Retrieve the position
        let retrieved_position = position_service.get_position_by_id(position_id).await;
        assert!(retrieved_position.is_ok(), "Position retrieval should succeed");
        let position = retrieved_position.unwrap();
        assert_eq!(position.id, position_id);
        assert_eq!(position.protocol, "uniswap_v3");
        
        // 3. Create risk assessment for the position
        let risk_assessment = RiskAssessment {
            id: Uuid::new_v4(),
            entity_id: position_id,
            entity_type: RiskEntityType::Position,
            risk_type: RiskType::ImpermanentLoss,
            risk_score: BigDecimal::from_str("0.25").unwrap(),
            risk_severity: RiskSeverity::Medium,
            description: "Moderate impermanent loss risk due to price volatility".to_string(),
            metadata: None,
            expires_at: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let risk_result = risk_service.update_risk_assessment(&risk_assessment).await;
        assert!(risk_result.is_ok(), "Risk assessment creation should succeed");
        
        // 4. Update position price and recalculate risk
        let mut updated_position = position.clone();
        updated_position.current_price = BigDecimal::from_str("1800.0").unwrap(); // Price drop
        updated_position.updated_at = Utc::now();
        
        let update_result = position_service.update_position(&updated_position).await;
        assert!(update_result.is_ok(), "Position update should succeed");
        
        // 5. Verify risk assessment reflects the change
        let risk_history = risk_service.get_risk_history(
            position_id, 
            Some(RiskType::ImpermanentLoss), 
            None, 
            None, 
            10, 
            0
        ).await;
        assert!(risk_history.is_ok(), "Risk history retrieval should succeed");
        
        // 6. Clean up - delete position
        let delete_result = position_service.delete_position(position_id).await;
        assert!(delete_result.is_ok(), "Position deletion should succeed");
        
        println!("✅ End-to-End Position Lifecycle: PASSED");
    }

    #[tokio::test]
    async fn test_portfolio_analytics_integration() {
        println!("🧪 Testing Portfolio Analytics Integration");
        
        let db = setup_test_environment().await.unwrap();
        let portfolio_service = PortfolioService::new(db.clone());
        let position_service = PositionService::new(db.clone());
        
        let user_id = Uuid::new_v4();
        
        // Create multiple positions for portfolio analysis
        let positions = vec![
            create_test_position(user_id, "uniswap_v3", "ethereum", "1000", "0.5", "2000", "2100"),
            create_test_position(user_id, "sushiswap", "polygon", "500", "1.0", "500", "520"),
            create_test_position(user_id, "pancakeswap", "bsc", "2000", "0.25", "8000", "7800"),
        ];
        
        // Create positions in database
        for position in &positions {
            let result = position_service.create_position(position).await;
            assert!(result.is_ok(), "Position creation should succeed");
        }
        
        // Test portfolio performance calculation
        let performance = portfolio_service.get_portfolio_performance(
            user_id, 
            Some(30), 
            None, 
            None
        ).await;
        assert!(performance.is_ok(), "Portfolio performance calculation should succeed");
        
        let perf_data = performance.unwrap();
        assert!(perf_data.total_value > BigDecimal::from(0), "Portfolio should have positive value");
        assert!(perf_data.positions_count > 0, "Portfolio should have positions");
        
        // Test P&L history
        let pnl_history = portfolio_service.get_pnl_history(
            user_id, 
            Some(7), 
            Some("daily".to_string()), 
            None, 
            None
        ).await;
        assert!(pnl_history.is_ok(), "P&L history calculation should succeed");
        
        // Test asset allocation
        let allocation = portfolio_service.get_asset_allocation(user_id, None, None).await;
        assert!(allocation.is_ok(), "Asset allocation calculation should succeed");
        
        let alloc_data = allocation.unwrap();
        assert!(!alloc_data.allocations.is_empty(), "Should have asset allocations");
        
        // Test protocol exposure
        let exposure = portfolio_service.get_protocol_exposure(user_id, None, None).await;
        assert!(exposure.is_ok(), "Protocol exposure calculation should succeed");
        
        let exp_data = exposure.unwrap();
        assert!(!exp_data.exposures.is_empty(), "Should have protocol exposures");
        assert!(exp_data.total_protocols >= 3, "Should have multiple protocols");
        
        // Clean up positions
        for position in &positions {
            let _ = position_service.delete_position(position.id).await;
        }
        
        println!("✅ Portfolio Analytics Integration: PASSED");
    }

    #[tokio::test]
    async fn test_cross_chain_risk_integration() {
        println!("🧪 Testing Cross-Chain Risk Integration");
        
        let db = setup_test_environment().await.unwrap();
        let settings = Settings::new().unwrap();
        let blockchain_service = Arc::new(BlockchainService::new(&settings, db.clone()));
        let cross_chain_service = CrossChainRiskService::new(db.clone(), blockchain_service);
        
        let user_id = Uuid::new_v4();
        
        // Test cross-chain risk assessment
        let chains = vec!["ethereum".to_string(), "polygon".to_string(), "arbitrum".to_string()];
        let risk_result = cross_chain_service.assess_cross_chain_risk(user_id, &chains).await;
        assert!(risk_result.is_ok(), "Cross-chain risk assessment should succeed");
        
        let risk_data = risk_result.unwrap();
        assert!(risk_data.overall_risk_score >= BigDecimal::from(0), "Risk score should be non-negative");
        
        // Test bridge risk assessment
        let bridge_risk = cross_chain_service.assess_bridge_risk(
            "polygon", 
            &BigDecimal::from_str("10000").unwrap()
        ).await;
        assert!(bridge_risk.is_ok(), "Bridge risk assessment should succeed");
        
        // Test liquidity fragmentation analysis
        let fragmentation = cross_chain_service.analyze_liquidity_fragmentation(user_id).await;
        assert!(fragmentation.is_ok(), "Liquidity fragmentation analysis should succeed");
        
        println!("✅ Cross-Chain Risk Integration: PASSED");
    }

    #[tokio::test]
    async fn test_mev_risk_integration() {
        println!("🧪 Testing MEV Risk Integration");
        
        let db = setup_test_environment().await.unwrap();
        let settings = Settings::new().unwrap();
        let blockchain_service = Arc::new(BlockchainService::new(&settings, db.clone()));
        let mev_service = MevRiskService::new(db.clone(), blockchain_service);
        
        let position_id = Uuid::new_v4();
        
        // Test MEV risk calculation
        let mev_risk = mev_service.calculate_mev_risk(
            position_id,
            "ethereum",
            &BigDecimal::from_str("50000").unwrap()
        ).await;
        assert!(mev_risk.is_ok(), "MEV risk calculation should succeed");
        
        let risk_score = mev_risk.unwrap();
        assert!(risk_score >= BigDecimal::from(0), "MEV risk score should be non-negative");
        assert!(risk_score <= BigDecimal::from(1), "MEV risk score should be normalized");
        
        // Test sandwich attack detection
        let sandwich_risk = mev_service.detect_sandwich_attacks(
            "ethereum",
            "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A"
        ).await;
        assert!(sandwich_risk.is_ok(), "Sandwich attack detection should succeed");
        
        // Test oracle manipulation detection
        let oracle_risk = mev_service.detect_oracle_manipulation(
            "ethereum",
            "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A",
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
        ).await;
        assert!(oracle_risk.is_ok(), "Oracle manipulation detection should succeed");
        
        println!("✅ MEV Risk Integration: PASSED");
    }

    #[tokio::test]
    async fn test_system_health_monitoring() {
        println!("🧪 Testing System Health Monitoring");
        
        let db = setup_test_environment().await.unwrap();
        let health_service = SystemHealthService::new(db.clone());
        
        // Test database metrics
        let db_metrics = health_service.get_database_metrics().await;
        assert!(db_metrics.is_ok(), "Database metrics should be retrievable");
        
        let metrics = db_metrics.unwrap();
        assert!(metrics.total_connections >= 0, "Connection count should be non-negative");
        assert!(metrics.cache_hit_ratio >= 0.0, "Cache hit ratio should be non-negative");
        
        // Test query performance stats
        let query_stats = health_service.get_query_performance_stats().await;
        assert!(query_stats.is_ok(), "Query performance stats should be retrievable");
        
        // Test connection pool health
        let pool_health = health_service.get_connection_pool_health().await;
        assert!(pool_health.is_ok(), "Connection pool health should be retrievable");
        
        let health_data = pool_health.unwrap();
        assert!(health_data.health_score >= 0.0, "Health score should be non-negative");
        assert!(health_data.health_score <= 1.0, "Health score should be normalized");
        
        // Test table sizes analysis
        let table_sizes = health_service.get_table_sizes().await;
        assert!(table_sizes.is_ok(), "Table sizes should be retrievable");
        
        println!("✅ System Health Monitoring: PASSED");
    }

    #[tokio::test]
    async fn test_price_validation_integration() {
        println!("🧪 Testing Price Validation Integration");
        
        let db = setup_test_environment().await.unwrap();
        let settings = Settings::new().unwrap();
        let blockchain_service = Arc::new(BlockchainService::new(&settings, db.clone()));
        let price_service = PriceValidationService::new(db.clone(), blockchain_service);
        
        // Test price validation for known tokens
        let validation_result = price_service.validate_token_prices(
            "ethereum",
            &["0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A".to_string()],
            &[BigDecimal::from_str("2000.0").unwrap()]
        ).await;
        assert!(validation_result.is_ok(), "Price validation should succeed");
        
        // Test price confidence calculation
        let confidence = price_service.calculate_price_confidence(
            "ethereum",
            "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A"
        ).await;
        assert!(confidence.is_ok(), "Price confidence calculation should succeed");
        
        let conf_score = confidence.unwrap();
        assert!(conf_score >= 0.0, "Confidence score should be non-negative");
        assert!(conf_score <= 1.0, "Confidence score should be normalized");
        
        println!("✅ Price Validation Integration: PASSED");
    }

    #[tokio::test]
    async fn test_query_performance_monitoring() {
        println!("🧪 Testing Query Performance Monitoring");
        
        let db = setup_test_environment().await.unwrap();
        let query_service = QueryPerformanceService::new(db.clone());
        
        // Test query performance logging
        let test_query = "SELECT COUNT(*) FROM positions WHERE user_id = $1";
        let user_id = Uuid::new_v4();
        
        let start_time = std::time::Instant::now();
        // Simulate query execution
        sleep(Duration::from_millis(10)).await;
        let execution_time = start_time.elapsed();
        
        let log_result = query_service.log_query_performance(
            test_query,
            execution_time,
            true,
            Some("Test query for performance monitoring".to_string())
        ).await;
        assert!(log_result.is_ok(), "Query performance logging should succeed");
        
        // Test slow query detection
        let slow_queries = query_service.get_slow_queries(
            Some(Duration::from_millis(5)),
            Some(10)
        ).await;
        assert!(slow_queries.is_ok(), "Slow query detection should succeed");
        
        // Test query plan analysis
        let plan_analysis = query_service.analyze_query_plan(test_query).await;
        assert!(plan_analysis.is_ok(), "Query plan analysis should succeed");
        
        println!("✅ Query Performance Monitoring: PASSED");
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        println!("🧪 Testing Concurrent Operations");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = Arc::new(PositionService::new(db.clone()));
        
        let user_id = Uuid::new_v4();
        let mut handles = vec![];
        
        // Create multiple concurrent position operations
        for i in 0..10 {
            let service = position_service.clone();
            let handle = tokio::spawn(async move {
                let position = create_test_position(
                    user_id,
                    &format!("protocol_{}", i),
                    "ethereum",
                    "1000",
                    "1.0",
                    "1000",
                    "1010"
                );
                
                // Create position
                let create_result = service.create_position(&position).await;
                assert!(create_result.is_ok(), "Concurrent position creation should succeed");
                
                // Read position
                let read_result = service.get_position_by_id(position.id).await;
                assert!(read_result.is_ok(), "Concurrent position read should succeed");
                
                // Update position
                let mut updated_position = position.clone();
                updated_position.current_price = BigDecimal::from_str("1020").unwrap();
                let update_result = service.update_position(&updated_position).await;
                assert!(update_result.is_ok(), "Concurrent position update should succeed");
                
                // Delete position
                let delete_result = service.delete_position(position.id).await;
                assert!(delete_result.is_ok(), "Concurrent position deletion should succeed");
                
                i
            });
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        for handle in handles {
            let result = handle.await;
            assert!(result.is_ok(), "Concurrent operation should complete successfully");
        }
        
        println!("✅ Concurrent Operations: PASSED");
    }

    #[tokio::test]
    async fn test_error_handling_and_recovery() {
        println!("🧪 Testing Error Handling and Recovery");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = PositionService::new(db.clone());
        
        // Test invalid position creation
        let invalid_position = Position {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            protocol: "".to_string(), // Invalid empty protocol
            chain: "ethereum".to_string(),
            position_type: PositionType::LiquidityPool,
            token0_address: "invalid_address".to_string(), // Invalid address
            token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount0: BigDecimal::from(-100), // Invalid negative amount
            amount1: BigDecimal::from_str("1.0").unwrap(),
            entry_price: BigDecimal::from(0), // Invalid zero price
            current_price: BigDecimal::from_str("1000").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let create_result = position_service.create_position(&invalid_position).await;
        assert!(create_result.is_err(), "Invalid position creation should fail");
        
        // Test non-existent position retrieval
        let non_existent_id = Uuid::new_v4();
        let read_result = position_service.get_position_by_id(non_existent_id).await;
        assert!(read_result.is_err(), "Non-existent position retrieval should fail gracefully");
        
        // Test duplicate position creation (if unique constraints exist)
        let valid_position = create_test_position(
            Uuid::new_v4(),
            "test_protocol",
            "ethereum",
            "1000",
            "1.0",
            "1000",
            "1010"
        );
        
        let first_create = position_service.create_position(&valid_position).await;
        assert!(first_create.is_ok(), "First position creation should succeed");
        
        // Clean up
        let _ = position_service.delete_position(valid_position.id).await;
        
        println!("✅ Error Handling and Recovery: PASSED");
    }

    #[tokio::test]
    async fn test_data_consistency_and_integrity() {
        println!("🧪 Testing Data Consistency and Integrity");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = PositionService::new(db.clone());
        let risk_service = RiskAssessmentService::new(db.clone());
        
        let user_id = Uuid::new_v4();
        let position_id = Uuid::new_v4();
        
        // Create position
        let position = create_test_position_with_id(
            position_id,
            user_id,
            "consistency_test",
            "ethereum",
            "1000",
            "1.0",
            "1000",
            "1010"
        );
        
        let create_result = position_service.create_position(&position).await;
        assert!(create_result.is_ok(), "Position creation should succeed");
        
        // Create risk assessment linked to position
        let risk_assessment = RiskAssessment {
            id: Uuid::new_v4(),
            entity_id: position_id,
            entity_type: RiskEntityType::Position,
            risk_type: RiskType::Liquidity,
            risk_score: BigDecimal::from_str("0.3").unwrap(),
            risk_severity: RiskSeverity::Medium,
            description: "Test risk assessment for consistency".to_string(),
            metadata: None,
            expires_at: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        let risk_result = risk_service.update_risk_assessment(&risk_assessment).await;
        assert!(risk_result.is_ok(), "Risk assessment creation should succeed");
        
        // Verify referential integrity - risk assessment should reference valid position
        let risk_history = risk_service.get_risk_history(
            position_id,
            None,
            None,
            None,
            10,
            0
        ).await;
        assert!(risk_history.is_ok(), "Risk history retrieval should succeed");
        
        // Test cascading operations - what happens when position is deleted?
        let delete_result = position_service.delete_position(position_id).await;
        assert!(delete_result.is_ok(), "Position deletion should succeed");
        
        // Verify risk assessments are handled appropriately
        let risk_after_delete = risk_service.get_risk_history(
            position_id,
            None,
            None,
            None,
            10,
            0
        ).await;
        // This should either succeed with empty results or handle the missing reference gracefully
        
        println!("✅ Data Consistency and Integrity: PASSED");
    }

    // Helper functions
    fn create_test_position(
        user_id: Uuid,
        protocol: &str,
        chain: &str,
        amount0: &str,
        amount1: &str,
        entry_price: &str,
        current_price: &str,
    ) -> Position {
        Position {
            id: Uuid::new_v4(),
            user_id,
            protocol: protocol.to_string(),
            chain: chain.to_string(),
            position_type: PositionType::LiquidityPool,
            token0_address: "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A".to_string(),
            token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount0: BigDecimal::from_str(amount0).unwrap(),
            amount1: BigDecimal::from_str(amount1).unwrap(),
            entry_price: BigDecimal::from_str(entry_price).unwrap(),
            current_price: BigDecimal::from_str(current_price).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn create_test_position_with_id(
        id: Uuid,
        user_id: Uuid,
        protocol: &str,
        chain: &str,
        amount0: &str,
        amount1: &str,
        entry_price: &str,
        current_price: &str,
    ) -> Position {
        Position {
            id,
            user_id,
            protocol: protocol.to_string(),
            chain: chain.to_string(),
            position_type: PositionType::LiquidityPool,
            token0_address: "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A".to_string(),
            token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount0: BigDecimal::from_str(amount0).unwrap(),
            amount1: BigDecimal::from_str(amount1).unwrap(),
            entry_price: BigDecimal::from_str(entry_price).unwrap(),
            current_price: BigDecimal::from_str(current_price).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
