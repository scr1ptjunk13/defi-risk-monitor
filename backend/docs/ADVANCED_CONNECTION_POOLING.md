# Advanced Connection Pooling Documentation

## Overview

The DeFi Risk Monitor implements advanced connection pooling optimization to ensure optimal database performance under varying load conditions. This system provides intelligent pool management, health monitoring, statement caching, and load-based optimization.

## Key Features

### 1. Advanced Pool Configuration
- **Dynamic Sizing**: Automatic pool scaling based on load metrics
- **Health Monitoring**: Real-time connection health checks with automatic recovery
- **Statement Caching**: Intelligent prepared statement caching for improved performance
- **Connection Lifecycle**: Proper connection validation, warmup, and recycling

### 2. Performance Monitoring
- **Real-time Metrics**: Continuous collection of pool performance data
- **Database Storage**: Persistent metrics storage for trend analysis
- **Load Testing**: Built-in load testing framework with performance grading
- **Optimization Recommendations**: Automated suggestions for pool tuning

## Configuration

### Basic Configuration

```rust
use defi_risk_monitor::database::{AdvancedConnectionPool, AdvancedPoolConfig};

// Default configuration
let config = AdvancedPoolConfig::default();

// Custom configuration
let config = AdvancedPoolConfig {
    // Pool sizing
    max_connections: 100,
    min_connections: 20,
    acquire_timeout_secs: 30,
    idle_timeout_secs: 600,
    max_lifetime_secs: 3600,
    
    // Statement caching
    statement_cache_capacity: 2000,
    enable_prepared_statements: true,
    
    // Health monitoring
    health_check_interval_secs: 30,
    health_check_timeout_secs: 5,
    max_failed_health_checks: 3,
    
    // Dynamic scaling
    enable_dynamic_sizing: true,
    load_threshold_high: 0.8,  // Scale up at 80% utilization
    load_threshold_low: 0.3,   // Scale down at 30% utilization
    scale_up_factor: 1.2,      // 20% increase
    scale_down_factor: 0.9,    // 10% decrease
    min_scale_interval_secs: 60,
    
    // Connection lifecycle
    enable_connection_validation: true,
    validation_query: "SELECT 1".to_string(),
    connection_warmup_queries: vec![
        "SET application_name = 'defi-risk-monitor'".to_string(),
        "SET statement_timeout = '30s'".to_string(),
    ],
    enable_connection_recycling: true,
    recycle_threshold_queries: 10000,
    
    ..Default::default()
};
```

### Environment-Specific Configurations

#### Production Configuration
```rust
let production_config = AdvancedPoolConfig {
    max_connections: 200,
    min_connections: 50,
    acquire_timeout_secs: 10,
    idle_timeout_secs: 300,
    max_lifetime_secs: 1800,
    statement_cache_capacity: 5000,
    enable_dynamic_sizing: true,
    load_threshold_high: 0.85,
    load_threshold_low: 0.25,
    health_check_interval_secs: 15,
    ..Default::default()
};
```

#### Development Configuration
```rust
let dev_config = AdvancedPoolConfig {
    max_connections: 20,
    min_connections: 5,
    acquire_timeout_secs: 60,
    idle_timeout_secs: 1200,
    statement_cache_capacity: 500,
    enable_dynamic_sizing: false,
    health_check_interval_secs: 60,
    ..Default::default()
};
```

## Usage

### Creating an Advanced Pool

```rust
use defi_risk_monitor::database::{AdvancedConnectionPool, AdvancedPoolConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = "postgresql://user:password@localhost:5432/database";
    let config = AdvancedPoolConfig::default();
    
    let pool = AdvancedConnectionPool::new(database_url, config).await?;
    
    // Use the pool for database operations
    let result = sqlx::query("SELECT 1 as test")
        .fetch_one(pool.get_pool())
        .await?;
    
    Ok(())
}
```

### Using the Connection Pool Service

```rust
use defi_risk_monitor::database::{ConnectionPoolService, establish_connection};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create service with main database pool
    let main_pool = establish_connection(&database_url).await?;
    let pool_service = ConnectionPoolService::new(main_pool);
    
    // Create specialized pools
    let primary_pool = pool_service.create_pool(
        "primary".to_string(),
        &database_url,
        production_config
    ).await?;
    
    let cache_pool = pool_service.create_pool(
        "cache".to_string(),
        &database_url,
        cache_config
    ).await?;
    
    // Monitor pool performance
    let performance = pool_service.get_pool_performance_summary("primary").await?;
    println!("Pool utilization: {:.2}%", performance.utilization_rate.to_f64().unwrap_or(0.0) * 100.0);
    
    Ok(())
}
```

## Monitoring and Metrics

### Real-time Pool Statistics

```rust
// Get current pool statistics
let stats = pool.get_pool_stats().await;
println!("Utilization: {:.2}%", stats.utilization_rate * 100.0);
println!("Average acquire time: {}ms", stats.avg_acquire_time_ms);

// Get health status
let health = pool.get_health_status().await;
println!("Pool healthy: {}", health.is_healthy);
println!("Response time: {}ms", health.response_time_ms);

// Get statement cache statistics
let cache_stats = pool.get_statement_cache_stats().await;
println!("Cache hit rate: {:.2}%", cache_stats.hit_rate * 100.0);
```

### Database Metrics Storage

The system automatically stores metrics in the following tables:
- `connection_pool_metrics`: Real-time pool performance data
- `connection_health_status`: Health monitoring results
- `statement_cache_metrics`: Cache performance statistics
- `pool_scaling_events`: Scaling decisions and events
- `pool_load_test_results`: Load testing results

### Performance Views

Use the pre-built views for monitoring dashboards:

```sql
-- Current pool health summary
SELECT * FROM pool_health_summary WHERE rn = 1;

-- Current pool performance summary
SELECT * FROM pool_performance_summary WHERE rn = 1;

-- Statement cache performance
SELECT * FROM statement_cache_performance WHERE rn = 1;
```

## Load Testing

### Running Load Tests

```rust
use defi_risk_monitor::database::PoolLoadTester;

// Create load tester
let load_tester = PoolLoadTester::new(Arc::clone(&pool));

// Run load test
let results = load_tester.run_load_test(
    50,  // 50 concurrent requests
    60   // for 60 seconds
).await?;

println!("Total requests: {}", results.total_requests);
println!("Error rate: {:.2}%", results.error_rate * 100.0);
println!("Requests per second: {}", results.requests_per_second);
```

### Performance Grading

The system automatically grades performance:
- **A**: < 1% errors, < 50ms response time, < 80% utilization
- **B**: < 5% errors, < 100ms response time, < 90% utilization
- **C**: < 10% errors, < 200ms response time, < 95% utilization
- **D**: < 20% errors, < 500ms response time
- **F**: Above thresholds

## Optimization Recommendations

### Automatic Optimization

```rust
// Run optimization on all pools
let report = pool_service.optimize_all_pools().await?;
println!("Optimized {}/{} pools", report.optimized_pools, report.total_pools);

for optimization in report.optimizations {
    println!("Pool '{}': {:.2}% utilization", 
             optimization.pool_name, 
             optimization.current_utilization * 100.0);
    
    for recommendation in optimization.recommendations {
        println!("  - {}", recommendation);
    }
}
```

### Manual Tuning Guidelines

1. **High Utilization (>85%)**:
   - Increase `max_connections`
   - Decrease `acquire_timeout_secs`
   - Enable dynamic scaling

2. **High Error Rate (>5%)**:
   - Increase pool size
   - Check connection validation
   - Review database capacity

3. **Slow Response Times (>100ms)**:
   - Optimize queries
   - Increase statement cache capacity
   - Review connection lifecycle settings

4. **Low Utilization (<30%)**:
   - Decrease `max_connections`
   - Increase `idle_timeout_secs`
   - Consider pool consolidation

## Best Practices

### Production Deployment

1. **Start Conservative**: Begin with moderate pool sizes and adjust based on metrics
2. **Monitor Continuously**: Use the built-in monitoring to track performance trends
3. **Load Test Regularly**: Run periodic load tests to validate configuration
4. **Set Alerts**: Monitor key metrics and set up alerts for anomalies

### Configuration Guidelines

1. **Pool Sizing**:
   - Set `min_connections` to handle baseline load
   - Set `max_connections` based on database capacity
   - Use dynamic scaling for variable workloads

2. **Health Checks**:
   - Keep intervals reasonable (15-60 seconds)
   - Set appropriate failure thresholds (3-5 failures)
   - Use simple validation queries

3. **Statement Caching**:
   - Size cache based on query diversity
   - Enable for read-heavy workloads
   - Monitor hit rates and adjust capacity

### Troubleshooting

#### High Connection Acquisition Times
- Check pool utilization
- Verify database capacity
- Review long-running queries

#### Frequent Health Check Failures
- Check network connectivity
- Verify database availability
- Review validation query complexity

#### Low Cache Hit Rates
- Increase cache capacity
- Review query patterns
- Check cache cleanup intervals

## Integration with Existing Systems

### Service Integration

```rust
// In your service layer
pub struct MyService {
    pool_service: Arc<ConnectionPoolService>,
}

impl MyService {
    pub async fn new(database_url: &str) -> Result<Self, AppError> {
        let main_pool = establish_connection(database_url).await?;
        let pool_service = Arc::new(ConnectionPoolService::new(main_pool));
        
        // Create specialized pools
        pool_service.create_pool(
            "transactions".to_string(),
            database_url,
            transaction_config
        ).await?;
        
        Ok(Self { pool_service })
    }
    
    pub async fn get_pool(&self, name: &str) -> Option<Arc<AdvancedConnectionPool>> {
        self.pool_service.get_pool(name).await
    }
}
```

### Monitoring Integration

The system provides Prometheus-compatible metrics and can be integrated with existing monitoring infrastructure.

## Conclusion

The advanced connection pooling system provides comprehensive database connection management with intelligent optimization, health monitoring, and performance tracking. It's designed to handle the demanding requirements of DeFi risk monitoring while providing the flexibility to adapt to changing load patterns.

For additional support or questions, refer to the test suite in `test_advanced_connection_pool.rs` for comprehensive usage examples.
