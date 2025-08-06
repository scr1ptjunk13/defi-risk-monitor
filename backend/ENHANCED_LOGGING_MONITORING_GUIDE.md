# Enhanced Logging and Monitoring System Guide

## Overview

The DeFi Risk Monitor includes a comprehensive enhanced logging and monitoring system designed for production-ready observability, performance tracking, and operational insights. This system provides structured logging, real-time metrics collection, health monitoring, alerting, and correlation tracking.

## Architecture

### Core Components

1. **EnhancedLogger** - Advanced structured logging with sanitization, correlation tracking, and multiple output formats
2. **EnhancedMonitoringSystem** - Metrics collection, health checks, alerting, and performance tracking
3. **LoggingMonitoringIntegration** - Unified system combining logging and monitoring with correlation tracking
4. **RequestLoggingMiddleware** - HTTP request/response logging with performance metrics

### Key Features

- **Structured Logging**: JSON, pretty, and compact formats with field sanitization
- **Correlation Tracking**: Request-to-response tracing with unique correlation IDs
- **Performance Monitoring**: Operation timing, system metrics, and performance analytics
- **Health Checks**: Automated health monitoring for database, blockchain RPC, and system resources
- **Alerting System**: Configurable alert rules with multiple severity levels
- **Metrics Collection**: Counters, gauges, histograms with Prometheus export
- **Security Logging**: Security event tracking with severity-based handling
- **Business Event Logging**: DeFi-specific business logic event tracking

## Configuration

### Logging Configuration

```rust
use crate::utils::enhanced_logging::LoggingConfig;

let logging_config = LoggingConfig {
    level: "info".to_string(),
    format: LogFormat::Json,
    output: LogOutput::Both,
    file_path: Some("/var/log/defi-risk-monitor/app.log".to_string()),
    max_file_size_mb: 100,
    max_files: 10,
    enable_structured_logging: true,
    enable_request_logging: true,
    enable_performance_logging: true,
    enable_security_logging: true,
    correlation_id_header: "x-correlation-id".to_string(),
    sensitive_fields: vec![
        "password".to_string(),
        "jwt_secret".to_string(),
        "api_key".to_string(),
        "private_key".to_string(),
    ],
    log_sampling_rate: 1.0,
    buffer_size: 8192,
};
```

### Monitoring Configuration

```rust
use crate::utils::monitoring_enhanced::MonitoringConfig;

let monitoring_config = MonitoringConfig {
    enable_metrics: true,
    enable_health_checks: true,
    enable_performance_tracking: true,
    enable_alerting: true,
    metrics_retention_hours: 24,
    health_check_interval_seconds: 30,
    performance_sampling_rate: 1.0,
    alert_cooldown_minutes: 5,
    prometheus_endpoint: Some("http://prometheus:9090".to_string()),
    jaeger_endpoint: Some("http://jaeger:14268".to_string()),
};
```

### Integration Configuration

```rust
use crate::utils::logging_integration::IntegrationConfig;

let integration_config = IntegrationConfig {
    enable_correlation_tracking: true,
    enable_performance_logging: true,
    enable_security_logging: true,
    enable_business_metrics: true,
    log_sampling_rate: 1.0,
    metrics_sampling_rate: 1.0,
    correlation_ttl_minutes: 60,
};
```

## Usage Examples

### Basic Setup

```rust
use crate::utils::logging_integration::LoggingMonitoringIntegration;
use crate::config::ProductionConfig;

// Initialize from production configuration
let config = ProductionConfig::load()?;
let logging_monitoring = LoggingMonitoringIntegration::from_production_config(&config);

// Initialize the system
logging_monitoring.init().await?;
```

### Correlation Tracking

```rust
// Start correlation for a request
let correlation_id = EnhancedLogger::create_correlation_id();
logging_monitoring.start_correlation(
    correlation_id.clone(),
    Some("user123".to_string()),
    Some("session456".to_string()),
    Some("/api/positions".to_string()),
).await;

// Add operations to correlation
logging_monitoring.add_operation(&correlation_id, "database_query".to_string()).await;
logging_monitoring.add_performance_data(&correlation_id, "database_query".to_string(), 150).await;

// End correlation (generates summary)
logging_monitoring.end_correlation(&correlation_id).await;
```

### Performance Logging

```rust
// Log performance events
logging_monitoring.log_performance_event(
    "position_risk_calculation",
    250, // duration in ms
    true, // success
    Some(correlation_id.clone()),
    None, // additional data
).await;
```

### Security Event Logging

```rust
use std::collections::HashMap;

let mut details = HashMap::new();
details.insert("ip_address".to_string(), serde_json::Value::String("192.168.1.100".to_string()));
details.insert("user_agent".to_string(), serde_json::Value::String("Mozilla/5.0...".to_string()));

logging_monitoring.log_security_event(
    "failed_login_attempt",
    "medium", // severity
    Some("user123".to_string()),
    Some("192.168.1.100".to_string()),
    details,
    Some(correlation_id.clone()),
).await;
```

### Business Event Logging

```rust
let mut event_data = HashMap::new();
event_data.insert("position_id".to_string(), serde_json::Value::String("pos_123".to_string()));
event_data.insert("protocol".to_string(), serde_json::Value::String("uniswap_v3".to_string()));
event_data.insert("token_pair".to_string(), serde_json::Value::String("USDC/WETH".to_string()));
event_data.insert("amount_usd".to_string(), serde_json::Value::Number(1000.into()));

logging_monitoring.log_business_event(
    "position_created",
    event_data,
    Some(correlation_id.clone()),
    Some("user123".to_string()),
).await;
```

### Metrics Collection

```rust
use std::collections::HashMap;

// Increment counters
let mut labels = HashMap::new();
labels.insert("protocol".to_string(), "uniswap_v3".to_string());
labels.insert("status".to_string(), "active".to_string());

let monitoring = logging_monitoring.get_monitoring();
monitoring.increment_counter("defi_positions_total", labels).await;

// Set gauges
let mut labels = HashMap::new();
labels.insert("pool".to_string(), "USDC_WETH_0.05".to_string());
monitoring.set_gauge("pool_tvl_usd", 1_000_000.0, labels).await;

// Record histograms
let mut labels = HashMap::new();
labels.insert("method".to_string(), "slot0".to_string());
labels.insert("chain".to_string(), "ethereum".to_string());
monitoring.observe_histogram("blockchain_calls_duration", 0.150, labels).await;
```

### Request Middleware

```rust
use axum::{middleware, Router};

let middleware = logging_monitoring.get_request_middleware();

let app = Router::new()
    .route("/api/positions", get(get_positions))
    .layer(middleware::from_fn(|req, next| async move {
        let correlation_id = req.headers()
            .get("x-correlation-id")
            .and_then(|h| h.to_str().ok())
            .unwrap_or(&EnhancedLogger::create_correlation_id());
        
        let start = std::time::Instant::now();
        let response = next.run(req).await;
        let duration = start.elapsed().as_millis() as u64;
        
        middleware.log_request(
            "GET",
            "/api/positions",
            response.status().as_u16(),
            duration,
            Some(correlation_id.to_string()),
            None,
        ).await;
        
        response
    }));
```

## Health Checks

The system includes automated health checks for critical components:

### Default Health Checks

1. **Database Health** - Connection pool status, query performance
2. **Blockchain RPC Health** - RPC endpoint availability, response times
3. **Memory Health** - Memory usage monitoring with thresholds
4. **System Health** - CPU, disk, network metrics

### Custom Health Checks

```rust
// Health checks run automatically every 30 seconds (configurable)
let health_status = logging_monitoring.get_monitoring().get_health_status().await;

for (name, check) in health_status {
    println!("Health Check: {} - Status: {:?}", name, check.status);
    if check.status != HealthStatus::Healthy {
        println!("  Error: {:?}", check.error_message);
        println!("  Consecutive Failures: {}", check.consecutive_failures);
    }
}
```

## Alerting System

### Default Alert Rules

1. **High Error Rate** - Triggers when error rate > 5% for 5 minutes
2. **High Memory Usage** - Triggers when memory usage > 85% for 10 minutes
3. **Database Unhealthy** - Triggers when database health check fails for 1 minute

### Custom Alert Rules

```rust
// Alert rules are evaluated every 30 seconds
let active_alerts = logging_monitoring.get_monitoring().get_active_alerts().await;

for alert in active_alerts {
    println!("Active Alert: {} - Severity: {:?}", alert.rule_name, alert.severity);
    println!("  Message: {}", alert.message);
    println!("  Fire Count: {}", alert.fire_count);
    println!("  Created: {}", alert.created_at);
}
```

## Prometheus Integration

### Metrics Export

```rust
// Export metrics in Prometheus format
let prometheus_metrics = logging_monitoring.export_prometheus_metrics().await;
println!("{}", prometheus_metrics);
```

### Example Prometheus Metrics

```
# HELP defi_positions_total Counter metric
# TYPE defi_positions_total counter
defi_positions_total{protocol="uniswap_v3",status="active"} 42

# HELP pool_tvl_usd Gauge metric
# TYPE pool_tvl_usd gauge
pool_tvl_usd{pool="USDC_WETH_0.05"} 1000000.0

# HELP blockchain_calls_duration Histogram metric
# TYPE blockchain_calls_duration histogram
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="0.001"} 0
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="0.01"} 0
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="0.1"} 5
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="1.0"} 10
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="10.0"} 10
blockchain_calls_duration_bucket{method="slot0",chain="ethereum",le="+Inf"} 10
blockchain_calls_duration_count{method="slot0",chain="ethereum"} 10
blockchain_calls_duration_sum{method="slot0",chain="ethereum"} 1.5
```

## Log Formats

### JSON Format (Production)

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "info",
  "message": "Position risk calculation completed",
  "target": "performance",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_id": "user123",
  "component": "risk_engine",
  "environment": "production",
  "duration_ms": 250,
  "fields": {
    "performance": true,
    "operation": "risk_calculation",
    "position_id": "pos_123",
    "risk_score": 0.75
  }
}
```

### Pretty Format (Development)

```
2024-01-15T10:30:45.123456Z  INFO risk_engine: Position risk calculation completed
    at src/services/risk_service.rs:145
    in risk_calculation with correlation_id: 550e8400-e29b-41d4-a716-446655440000, user_id: user123
  fields:
    performance: true
    operation: "risk_calculation"
    position_id: "pos_123"
    risk_score: 0.75
    duration_ms: 250
```

## Environment Variables

### Logging Configuration

```bash
# Logging level
DEFI_RISK__LOGGING__LEVEL=info

# Log format (json, pretty, compact)
DEFI_RISK__LOGGING__FORMAT=json

# Log file path
DEFI_RISK__LOGGING__FILE=/var/log/defi-risk-monitor/app.log

# Enable structured logging
DEFI_RISK__LOGGING__STRUCTURED=true
```

### Monitoring Configuration

```bash
# Enable monitoring features
DEFI_RISK__MONITORING__ENABLE_METRICS=true
DEFI_RISK__MONITORING__ENABLE_HEALTH_CHECKS=true
DEFI_RISK__MONITORING__ENABLE_ALERTING=true

# Health check interval
DEFI_RISK__MONITORING__HEALTH_CHECK_INTERVAL_SECONDS=30

# Metrics retention
DEFI_RISK__MONITORING__METRICS_RETENTION_HOURS=24

# External integrations
DEFI_RISK__MONITORING__PROMETHEUS_ENDPOINT=http://prometheus:9090
DEFI_RISK__MONITORING__JAEGER_ENDPOINT=http://jaeger:14268
```

## Performance Considerations

### Log Sampling

For high-throughput environments, configure log sampling:

```rust
let config = LoggingConfig {
    log_sampling_rate: 0.1, // Sample 10% of logs
    metrics_sampling_rate: 1.0, // Keep all metrics
    ..Default::default()
};
```

### Async Processing

All logging and monitoring operations are asynchronous and non-blocking:

```rust
// Non-blocking logging
tokio::spawn(async move {
    logging_monitoring.log_performance_event(
        "background_task",
        duration_ms,
        success,
        correlation_id,
        None,
    ).await;
});
```

### Memory Management

- Automatic log rotation based on file size
- Configurable metrics retention periods
- Correlation context cleanup with TTL
- Background cleanup tasks for old data

## Security Features

### Sensitive Data Sanitization

Automatically redacts sensitive fields in logs:

```rust
// Before sanitization
let log_entry = LogEntry {
    message: "User login with password=secret123".to_string(),
    fields: {
        "password": "secret123",
        "jwt_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
    },
    ..
};

// After sanitization
let sanitized = logger.sanitize_log_entry(log_entry).await;
// message: "User login with password=[REDACTED]"
// fields: { "password": "[REDACTED]", "jwt_token": "[REDACTED]" }
```

### Security Event Tracking

```rust
// High-severity security events trigger immediate alerts
logging_monitoring.log_security_event(
    "brute_force_attack",
    "critical", // Triggers immediate alert
    Some("attacker_ip".to_string()),
    Some("192.168.1.100".to_string()),
    security_details,
    correlation_id,
).await;
```

## Troubleshooting

### Common Issues

1. **High Memory Usage**
   - Reduce metrics retention period
   - Increase log sampling rate
   - Check for correlation context leaks

2. **Log File Rotation Not Working**
   - Verify file permissions
   - Check disk space
   - Ensure log directory exists

3. **Missing Correlation IDs**
   - Verify correlation_id_header configuration
   - Check middleware integration
   - Ensure correlation tracking is enabled

4. **Metrics Not Appearing**
   - Verify metrics are enabled in configuration
   - Check sampling rates
   - Ensure proper label formatting

### Debug Mode

Enable debug logging for troubleshooting:

```bash
RUST_LOG=defi_risk_monitor=debug,enhanced_logging=debug,monitoring_enhanced=debug
```

### Health Check Debugging

```rust
let health_status = monitoring.get_health_status().await;
for (name, check) in health_status {
    if check.status != HealthStatus::Healthy {
        eprintln!("Unhealthy component: {}", name);
        eprintln!("  Error: {:?}", check.error_message);
        eprintln!("  Duration: {}ms", check.check_duration_ms);
        eprintln!("  Failures: {}", check.consecutive_failures);
        eprintln!("  Success Rate: {:.2}%", check.success_rate * 100.0);
    }
}
```

## Best Practices

### Correlation ID Management

1. **Generate correlation IDs early** in request processing
2. **Propagate correlation IDs** through all service calls
3. **Include correlation IDs** in external API calls
4. **Use correlation IDs** for distributed tracing

### Performance Logging

1. **Log critical operations** with timing data
2. **Use appropriate log levels** (debug for detailed, info for important)
3. **Include context** in performance logs
4. **Monitor performance trends** over time

### Security Logging

1. **Log all authentication events** (success and failure)
2. **Track authorization failures** with context
3. **Monitor for suspicious patterns** (brute force, unusual access)
4. **Include IP addresses and user agents** in security logs

### Business Event Logging

1. **Log key business operations** (position creation, risk assessments)
2. **Include relevant business context** (amounts, protocols, tokens)
3. **Use consistent event naming** conventions
4. **Track business metrics** for analytics

## Integration with Existing Systems

### Database Integration

The logging and monitoring system integrates with the existing database services:

```rust
// Example: Log database operations with performance tracking
let timer_id = logging_monitoring.get_monitoring()
    .start_timer("database_query", labels).await;

let result = position_service.get_position(position_id).await;

logging_monitoring.get_monitoring().stop_timer(&timer_id).await;

if result.is_ok() {
    logging_monitoring.log_performance_event(
        "get_position",
        timer_duration_ms,
        true,
        correlation_id,
        None,
    ).await;
}
```

### Blockchain Service Integration

```rust
// Example: Monitor blockchain calls with error tracking
let start = std::time::Instant::now();
let result = blockchain_service.get_pool_state(pool_address).await;
let duration = start.elapsed().as_millis() as u64;

match result {
    Ok(pool_state) => {
        logging_monitoring.log_performance_event(
            "blockchain_get_pool_state",
            duration,
            true,
            correlation_id,
            Some({
                let mut data = HashMap::new();
                data.insert("pool_address".to_string(), 
                    serde_json::Value::String(pool_address.to_string()));
                data
            }),
        ).await;
    }
    Err(error) => {
        logging_monitoring.log_performance_event(
            "blockchain_get_pool_state",
            duration,
            false,
            correlation_id,
            None,
        ).await;
        
        if let Some(correlation_id) = &correlation_id {
            logging_monitoring.add_error(
                correlation_id, 
                format!("Blockchain call failed: {}", error)
            ).await;
        }
    }
}
```

## Conclusion

The Enhanced Logging and Monitoring System provides comprehensive observability for the DeFi Risk Monitor, enabling:

- **Production-ready logging** with structured formats and sanitization
- **Real-time monitoring** with metrics collection and health checks
- **Performance tracking** with detailed timing and operation analytics
- **Security monitoring** with event tracking and alerting
- **Business intelligence** with DeFi-specific event logging
- **Correlation tracking** for distributed request tracing
- **Prometheus integration** for metrics export and visualization

This system ensures operational visibility, performance optimization, security monitoring, and business insights necessary for a production DeFi risk monitoring platform.
