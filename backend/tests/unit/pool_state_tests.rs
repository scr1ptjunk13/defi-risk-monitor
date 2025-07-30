use defi_risk_monitor::models::{PoolState, CreatePoolState};
use bigdecimal::BigDecimal;
use chrono::Utc;

#[test]
fn test_pool_state_creation() {
    let create_pool_state = CreatePoolState {
        pool_address: "0x1234567890123456789012345678901234567890".to_string(),
        chain_id: 1,
        current_tick: 100,
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(500000),
        token0_price_usd: Some(BigDecimal::from(1)),
        token1_price_usd: Some(BigDecimal::from(2000)),
        tvl_usd: Some(BigDecimal::from(10000000)),
        volume_24h_usd: Some(BigDecimal::from(1000000)),
        fees_24h_usd: Some(BigDecimal::from(10000)),
    };

    let pool_state = PoolState::new(create_pool_state);

    assert_eq!(pool_state.pool_address, "0x1234567890123456789012345678901234567890");
    assert_eq!(pool_state.chain_id, 1);
    assert_eq!(pool_state.current_tick, 100);
    assert_eq!(pool_state.sqrt_price_x96, BigDecimal::from(1000000));
    assert_eq!(pool_state.liquidity, BigDecimal::from(500000));
    assert_eq!(pool_state.token0_price_usd, Some(BigDecimal::from(1)));
    assert_eq!(pool_state.token1_price_usd, Some(BigDecimal::from(2000)));
    assert_eq!(pool_state.tvl_usd, Some(BigDecimal::from(10000000)));
    assert_eq!(pool_state.volume_24h_usd, Some(BigDecimal::from(1000000)));
    assert_eq!(pool_state.fees_24h_usd, Some(BigDecimal::from(10000)));
    
    // Timestamp should be recent
    let now = Utc::now();
    let time_diff = (now - pool_state.timestamp).num_seconds();
    assert!(time_diff < 5); // Should be created within last 5 seconds
}

#[test]
fn test_pool_state_with_none_values() {
    let create_pool_state = CreatePoolState {
        pool_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef".to_string(),
        chain_id: 137,
        current_tick: -500,
        sqrt_price_x96: BigDecimal::from(2000000),
        liquidity: BigDecimal::from(1000000),
        token0_price_usd: None,
        token1_price_usd: None,
        tvl_usd: None,
        volume_24h_usd: None,
        fees_24h_usd: None,
    };

    let pool_state = PoolState::new(create_pool_state);

    assert_eq!(pool_state.pool_address, "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef");
    assert_eq!(pool_state.chain_id, 137);
    assert_eq!(pool_state.current_tick, -500);
    assert_eq!(pool_state.token0_price_usd, None);
    assert_eq!(pool_state.token1_price_usd, None);
    assert_eq!(pool_state.tvl_usd, None);
    assert_eq!(pool_state.volume_24h_usd, None);
    assert_eq!(pool_state.fees_24h_usd, None);
}

#[test]
fn test_pool_state_serialization() {
    let create_pool_state = CreatePoolState {
        pool_address: "0x1234567890123456789012345678901234567890".to_string(),
        chain_id: 1,
        current_tick: 100,
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(500000),
        token0_price_usd: Some(BigDecimal::from(1)),
        token1_price_usd: Some(BigDecimal::from(2000)),
        tvl_usd: Some(BigDecimal::from(10000000)),
        volume_24h_usd: Some(BigDecimal::from(1000000)),
        fees_24h_usd: Some(BigDecimal::from(10000)),
    };

    let pool_state = PoolState::new(create_pool_state);

    // Test serialization to JSON
    let json = serde_json::to_string(&pool_state);
    assert!(json.is_ok());

    // Test deserialization from JSON
    let json_str = json.unwrap();
    let deserialized: Result<PoolState, _> = serde_json::from_str(&json_str);
    assert!(deserialized.is_ok());

    let deserialized_state = deserialized.unwrap();
    assert_eq!(deserialized_state.pool_address, pool_state.pool_address);
    assert_eq!(deserialized_state.chain_id, pool_state.chain_id);
    assert_eq!(deserialized_state.current_tick, pool_state.current_tick);
}
