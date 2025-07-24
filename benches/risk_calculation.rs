use criterion::{black_box, criterion_group, criterion_main, Criterion};
use defi_risk_monitor::{
    services::risk_calculator::RiskCalculator,
    models::{Position, PoolState, RiskConfig, CreatePosition, CreatePoolState, CreateRiskConfig},
    utils::math::{standard_deviation, moving_average, correlation},
};
use rust_decimal::Decimal;

fn benchmark_risk_calculation(c: &mut Criterion) {
    let risk_calculator = RiskCalculator::new();
    
    // Create test data
    let position = Position::new(CreatePosition {
        user_address: "0x1234567890123456789012345678901234567890".to_string(),
        protocol: "Uniswap V3".to_string(),
        pool_address: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdef".to_string(),
        token0_address: "0x1111111111111111111111111111111111111111".to_string(),
        token1_address: "0x2222222222222222222222222222222222222222".to_string(),
        token0_amount: Decimal::from(1000),
        token1_amount: Decimal::from(2000),
        liquidity: Decimal::from(50000),
        tick_lower: -1000,
        tick_upper: 1000,
        fee_tier: 3000,
        chain_id: 1,
    });
    
    let pool_state = PoolState::new(CreatePoolState {
        pool_address: position.pool_address.clone(),
        chain_id: position.chain_id,
        current_tick: 0,
        sqrt_price_x96: Decimal::from(1000000),
        liquidity: Decimal::from(1000000),
        token0_price_usd: Some(Decimal::from(1)),
        token1_price_usd: Some(Decimal::from(1)),
        tvl_usd: Some(Decimal::from(10000000)),
        volume_24h_usd: Some(Decimal::from(1000000)),
        fees_24h_usd: Some(Decimal::from(10000)),
    });
    
    let risk_config = RiskConfig::new(CreateRiskConfig {
        user_address: position.user_address.clone(),
        max_position_size_usd: Some(Decimal::from(1000000)),
        liquidation_threshold: Some(Decimal::new(85, 2)),
        price_impact_threshold: Some(Decimal::new(5, 2)),
        impermanent_loss_threshold: Some(Decimal::new(10, 2)),
        volatility_threshold: Some(Decimal::new(20, 2)),
        correlation_threshold: Some(Decimal::new(80, 2)),
    });
    
    // Create historical data
    let mut historical_data = Vec::new();
    for i in 0..100 {
        let mut state = pool_state.clone();
        state.token0_price_usd = Some(Decimal::from(1) + Decimal::from(i) / Decimal::from(1000));
        state.token1_price_usd = Some(Decimal::from(1) + Decimal::from(i) / Decimal::from(2000));
        historical_data.push(state);
    }
    
    c.bench_function("risk_calculation", |b| {
        b.iter(|| {
            risk_calculator.calculate_position_risk(
                black_box(&position),
                black_box(&pool_state),
                black_box(&risk_config),
                black_box(&historical_data),
            )
        })
    });
}

fn benchmark_standard_deviation(c: &mut Criterion) {
    let values: Vec<Decimal> = (0..1000)
        .map(|i| Decimal::from(i) + Decimal::from(i) / Decimal::from(100))
        .collect();
    
    c.bench_function("standard_deviation", |b| {
        b.iter(|| standard_deviation(black_box(&values)))
    });
}

fn benchmark_moving_average(c: &mut Criterion) {
    let values: Vec<Decimal> = (0..1000)
        .map(|i| Decimal::from(i))
        .collect();
    
    c.bench_function("moving_average_10", |b| {
        b.iter(|| moving_average(black_box(&values), black_box(10)))
    });
    
    c.bench_function("moving_average_50", |b| {
        b.iter(|| moving_average(black_box(&values), black_box(50)))
    });
}

fn benchmark_correlation(c: &mut Criterion) {
    let x: Vec<Decimal> = (0..1000).map(|i| Decimal::from(i)).collect();
    let y: Vec<Decimal> = (0..1000).map(|i| Decimal::from(i * 2)).collect();
    
    c.bench_function("correlation", |b| {
        b.iter(|| correlation(black_box(&x), black_box(&y)))
    });
}

criterion_group!(
    benches,
    benchmark_risk_calculation,
    benchmark_standard_deviation,
    benchmark_moving_average,
    benchmark_correlation
);
criterion_main!(benches);
