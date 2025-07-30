use defi_risk_monitor::{
    services::risk_calculator::{RiskCalculator, RiskMetrics},
    models::{Position, PoolState, RiskConfig, CreatePosition, CreatePoolState, CreateRiskConfig},
};
use bigdecimal::BigDecimal;
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_risk_calculation_integration() {
    let risk_calculator = RiskCalculator::new();
    
    // Create test position
    let create_position = CreatePosition {
        user_address: "0x1234567890123456789012345678901234567890".to_string(),
        protocol: "Uniswap V3".to_string(),
        pool_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef".to_string(),
        token0_address: "0x1111111111111111111111111111111111111111".to_string(),
        token1_address: "0x2222222222222222222222222222222222222222".to_string(),
        token0_amount: BigDecimal::from(1000),
        token1_amount: BigDecimal::from(2000),
        liquidity: BigDecimal::from(50000),
        tick_lower: -1000,
        tick_upper: 1000,
        fee_tier: 3000,
        chain_id: 1,
    };
    let position = Position::new(create_position);
    
    // Create test pool state
    let create_pool_state = CreatePoolState {
        pool_address: position.pool_address.clone(),
        chain_id: position.chain_id,
        current_tick: 0,
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(1000000),
        token0_price_usd: Some(BigDecimal::from(1)),
        token1_price_usd: Some(BigDecimal::from(1)),
        tvl_usd: Some(BigDecimal::from(10000000)),
        volume_24h_usd: Some(BigDecimal::from(1000000)),
        fees_24h_usd: Some(BigDecimal::from(10000)),
    };
    let pool_state = PoolState::new(create_pool_state);
    
    // Create test risk config
    let create_risk_config = CreateRiskConfig {
        user_address: position.user_address.clone(),
        max_position_size_usd: Some(BigDecimal::from(1000000)),
        liquidation_threshold: Some(BigDecimal::new(85, 2)),
        price_impact_threshold: Some(BigDecimal::new(5, 2)),
        impermanent_loss_threshold: Some(BigDecimal::new(10, 2)),
        volatility_threshold: Some(BigDecimal::new(20, 2)),
        correlation_threshold: Some(BigDecimal::new(80, 2)),
    };
    let risk_config = RiskConfig::new(create_risk_config);
    
    // Create historical data
    let historical_data = vec![pool_state.clone()];
    
    // Calculate risk metrics
    let result = risk_calculator.calculate_position_risk(
        &position,
        &pool_state,
        &risk_config,
        &historical_data,
    );
    
    assert!(result.is_ok());
    let metrics = result.unwrap();
    
    // Verify risk metrics are within expected ranges
    assert!(metrics.overall_risk_score >= BigDecimal::ZERO);
    assert!(metrics.overall_risk_score <= BigDecimal::ONE);
    assert!(metrics.impermanent_loss >= BigDecimal::ZERO);
    assert!(metrics.price_impact >= BigDecimal::ZERO);
    assert!(metrics.volatility_score >= BigDecimal::ZERO);
}

#[tokio::test]
async fn test_risk_threshold_violations() {
    let risk_calculator = RiskCalculator::new();
    
    // Create high-risk metrics
    let high_risk_metrics = RiskMetrics {
        impermanent_loss: BigDecimal::new(15, 2), // 15% - above 10% threshold
        price_impact: BigDecimal::new(8, 2),      // 8% - above 5% threshold
        volatility_score: BigDecimal::new(25, 2), // 25% - above 20% threshold
        correlation_score: BigDecimal::new(5, 1), // 0.5
        liquidity_score: BigDecimal::new(3, 1),   // 0.3
        overall_risk_score: BigDecimal::new(9, 1), // 0.9
        value_at_risk_1d: BigDecimal::from(1000),
        value_at_risk_7d: BigDecimal::from(5000),
    };
    
    let risk_config = RiskConfig::new(CreateRiskConfig {
        user_address: "0x1234567890123456789012345678901234567890".to_string(),
        max_position_size_usd: Some(BigDecimal::from(1000000)),
        liquidation_threshold: Some(BigDecimal::new(85, 2)),
        price_impact_threshold: Some(BigDecimal::new(5, 2)),
        impermanent_loss_threshold: Some(BigDecimal::new(10, 2)),
        volatility_threshold: Some(BigDecimal::new(20, 2)),
        correlation_threshold: Some(BigDecimal::new(80, 2)),
    });
    
    let violations = risk_calculator.check_risk_thresholds(&high_risk_metrics, &risk_config);
    
    // Should have violations for IL, price impact, and volatility
    assert!(violations.len() >= 3);
    assert!(violations.iter().any(|v| v.contains("Impermanent loss")));
    assert!(violations.iter().any(|v| v.contains("Price impact")));
    assert!(violations.iter().any(|v| v.contains("Volatility")));
}

#[tokio::test]
async fn test_low_risk_scenario() {
    let risk_calculator = RiskCalculator::new();
    
    // Create low-risk metrics
    let low_risk_metrics = RiskMetrics {
        impermanent_loss: BigDecimal::new(2, 2),  // 2% - below 10% threshold
        price_impact: BigDecimal::new(1, 2),      // 1% - below 5% threshold
        volatility_score: BigDecimal::new(5, 2),  // 5% - below 20% threshold
        correlation_score: BigDecimal::new(5, 1), // 0.5
        liquidity_score: BigDecimal::new(1, 1),   // 0.1 - high liquidity
        overall_risk_score: BigDecimal::new(2, 1), // 0.2 - low risk
        value_at_risk_1d: BigDecimal::from(100),
        value_at_risk_7d: BigDecimal::from(500),
    };
    
    let risk_config = RiskConfig::new(CreateRiskConfig {
        user_address: "0x1234567890123456789012345678901234567890".to_string(),
        max_position_size_usd: Some(BigDecimal::from(1000000)),
        liquidation_threshold: Some(BigDecimal::new(85, 2)),
        price_impact_threshold: Some(BigDecimal::new(5, 2)),
        impermanent_loss_threshold: Some(BigDecimal::new(10, 2)),
        volatility_threshold: Some(BigDecimal::new(20, 2)),
        correlation_threshold: Some(BigDecimal::new(80, 2)),
    });
    
    let violations = risk_calculator.check_risk_thresholds(&low_risk_metrics, &risk_config);
    
    // Should have no violations
    assert_eq!(violations.len(), 0);
}
