# Production Configuration Management Guide

## Overview

The DeFi Risk Monitor now includes a comprehensive production-ready configuration management system that provides:

- **Environment-specific configurations** (development, testing, staging, production)
- **Configuration validation and type safety**
- **Hot-reloading capabilities** for production environments
- **Secrets management** with encryption support
- **Configuration monitoring and health checks**
- **Structured logging and audit trails**

## Architecture

### Core Components

1. **ProductionConfig** - Main configuration structure with all application settings
2. **ConfigValidator** - Validates configuration values and enforces security policies
3. **ConfigManager** - Manages configuration lifecycle, hot-reloading, and monitoring
4. **SecretsManager** - Handles sensitive configuration data securely

### Configuration Hierarchy

Configuration is loaded in the following order (later sources override earlier ones):

1. **Default values** - Built-in defaults based on environment
2. **Environment-specific files** - `config/config.{environment}.toml`
3. **Local overrides** - `config/local.toml` (for development)
4. **Secrets file** - `config/secrets.toml` (production only)
5. **Environment variables** - `DEFI_RISK__*` prefixed variables
6. **Standard environment variables** - Standard env vars like `DATABASE_URL`

## Configuration Files

### Environment Detection

The system automatically detects the environment using these variables (in order):
- `ENVIRONMENT`
- `ENV` 
- `NODE_ENV`

Supported environments: `development`, `testing`, `staging`, `production`

### Configuration Files Structure

```
backend/
├── config/
│   ├── config.development.toml    # Development settings
│   ├── config.testing.toml        # Test settings  
│   ├── config.staging.toml        # Staging settings
│   ├── config.production.toml     # Production settings
│   ├── local.toml                 # Local overrides (optional)
│   └── secrets.toml               # Sensitive data (production)
```

## Configuration Sections

### Database Configuration

```toml
[database]
url = "postgresql://user:pass@host:port/db"
max_connections = 50
min_connections = 10
connection_timeout_seconds = 30
idle_timeout_seconds = 300
max_lifetime_seconds = 1800
enable_logging = false
slow_query_threshold_ms = 1000
replica_urls = ["postgresql://replica1:5432/db"]
enable_read_replicas = true
```

### API Configuration

```toml
[api]
host = "0.0.0.0"
port = 8080
cors_origins = ["https://app.example.com"]
request_timeout_seconds = 30
max_request_size_mb = 16
rate_limit_requests_per_minute = 1000
enable_compression = true
enable_metrics = true

# Optional TLS configuration
[api.tls]
cert_file = "/etc/ssl/certs/app.crt"
key_file = "/etc/ssl/private/app.key"
ca_file = "/etc/ssl/certs/ca.crt"
```

### Blockchain Configuration

```toml
[blockchain]
ethereum_rpc_url = "https://mainnet.infura.io/v3/PROJECT_ID"
ethereum_ws_url = "wss://mainnet.infura.io/ws/v3/PROJECT_ID"
polygon_rpc_url = "https://polygon-mainnet.infura.io/v3/PROJECT_ID"
arbitrum_rpc_url = "https://arbitrum-mainnet.infura.io/v3/PROJECT_ID"
optimism_rpc_url = "https://optimism-mainnet.infura.io/v3/PROJECT_ID"
bsc_rpc_url = "https://bsc-dataseed.binance.org"
avalanche_rpc_url = "https://api.avax.network/ext/bc/C/rpc"
request_timeout_seconds = 30
max_retries = 3
retry_delay_ms = 2000
enable_caching = true
cache_ttl_seconds = 60
risk_check_interval_seconds = 60
block_confirmation_count = 12
```

### Risk Management Configuration

```toml
[risk]
max_position_size_usd = 10000000.0
liquidation_threshold = 0.85
impermanent_loss_threshold = 0.03
mev_risk_threshold = 0.015
enable_real_time_monitoring = true
risk_calculation_interval_seconds = 30
enable_predictive_analytics = true
confidence_threshold = 0.85

# Protocol-specific risk weights
[risk.protocol_risk_weights]
uniswap_v3 = 1.0
uniswap_v2 = 1.2
sushiswap = 1.5
curve = 1.1
balancer = 1.3
aave = 0.8
compound = 0.9
```

### Security Configuration

```toml
[security]
jwt_secret = "your-super-secure-jwt-secret-32-chars-minimum"
jwt_expiration_hours = 24
enable_rate_limiting = true
max_login_attempts = 5
lockout_duration_minutes = 15
enable_2fa = true
password_min_length = 12
session_timeout_minutes = 480
enable_audit_logging = true
trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12"]
```

### Alert Configuration

```toml
[alerts]
slack_webhook_url = "https://hooks.slack.com/services/..."
discord_webhook_url = "https://discord.com/api/webhooks/..."
telegram_bot_token = "bot_token"
telegram_chat_id = "chat_id"
webhook_endpoints = ["https://api.example.com/webhooks/alerts"]
enable_push_notifications = true
alert_cooldown_minutes = 15
severity_thresholds = { low = 0.05, medium = 0.2, high = 0.5, critical = 0.8 }

# Email configuration
[alerts.email]
smtp_host = "smtp.gmail.com"
smtp_port = 587
username = "alerts@company.com"
password = "app_password"
from_address = "DeFi Risk Monitor <alerts@company.com>"
use_tls = true
```

### Logging Configuration

```toml
[logging]
level = "info"                    # debug, info, warn, error
format = "json"                   # json, pretty, compact
output = "both"                   # stdout, file, both
file_path = "/var/log/defi-risk-monitor/app.log"
max_file_size_mb = 100
max_files = 10
enable_structured_logging = true
enable_request_logging = true
sensitive_fields = ["password", "jwt_secret", "api_key"]
```

### Monitoring Configuration

```toml
[monitoring]
enable_prometheus = true
prometheus_port = 9090
enable_jaeger = true
jaeger_endpoint = "http://jaeger:14268/api/traces"
enable_health_checks = true
health_check_interval_seconds = 30
metrics_retention_days = 90
enable_alerting = true
alert_manager_url = "http://alertmanager:9093"
```

### Performance Configuration

```toml
[performance]
worker_threads = 8
max_blocking_threads = 32
enable_connection_pooling = true
cache_size_mb = 512
enable_query_caching = true
query_cache_ttl_seconds = 300
enable_compression = true
batch_size = 5000
enable_parallel_processing = true
```

### Feature Flags

```toml
[features]
enable_websockets = true
enable_real_time_updates = true
enable_advanced_analytics = true
enable_mev_protection = true
enable_cross_chain_monitoring = true
enable_ai_predictions = true
enable_automated_rebalancing = false  # Disabled for safety
enable_portfolio_optimization = true
enable_experimental_features = false
```

## Environment Variables

### Standard Environment Variables

- `DATABASE_URL` - Database connection string
- `ETHEREUM_RPC_URL` - Ethereum RPC endpoint
- `POLYGON_RPC_URL` - Polygon RPC endpoint
- `JWT_SECRET` - JWT signing secret
- `RUST_LOG` - Rust logging configuration

### Prefixed Environment Variables

Use `DEFI_RISK__` prefix with double underscores for nested values:

```bash
# Database configuration
export DEFI_RISK__DATABASE__MAX_CONNECTIONS=100
export DEFI_RISK__DATABASE__ENABLE_LOGGING=true

# API configuration  
export DEFI_RISK__API__PORT=8080
export DEFI_RISK__API__ENABLE_COMPRESSION=true

# Security configuration
export DEFI_RISK__SECURITY__ENABLE_2FA=true
export DEFI_RISK__SECURITY__MAX_LOGIN_ATTEMPTS=3

# Feature flags
export DEFI_RISK__FEATURES__ENABLE_AI_PREDICTIONS=true
export DEFI_RISK__FEATURES__ENABLE_EXPERIMENTAL_FEATURES=false
```

## Usage

### Basic Usage

```rust
use defi_risk_monitor::config::ProductionConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ProductionConfig::load()?;
    
    // Use configuration
    println!("Environment: {}", config.environment.as_str());
    println!("Database max connections: {}", config.database.max_connections);
    
    Ok(())
}
```

### With Configuration Manager

```rust
use defi_risk_monitor::config::ConfigManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration manager
    let mut config_manager = ConfigManager::new().await?;
    
    // Start hot-reloading
    config_manager.start_monitoring().await?;
    
    // Get current configuration
    let config = config_manager.get_config().await;
    
    // Check feature flags
    if config.is_feature_enabled("websockets") {
        println!("WebSockets enabled");
    }
    
    Ok(())
}
```

### Configuration Validation

```rust
use defi_risk_monitor::config::{ProductionConfig, ConfigValidator};

let config = ProductionConfig::load()?;
let validator = ConfigValidator::new();

// Validate configuration
validator.validate_database_config(&config.database)?;
validator.validate_security_config(&config.security, config.is_production())?;
```

## Secrets Management

### Creating Secrets File

```bash
# Create secrets file (production only)
cat > config/secrets.toml << EOF
jwt_secret = "super-secure-production-jwt-secret-32-chars-minimum"
database_password = "secure-database-password"
slack_webhook_url = "https://hooks.slack.com/services/real/webhook/url"
api_keys = { coinmarketcap = "real-api-key", infura = "real-project-id" }
EOF

# Set restrictive permissions
chmod 600 config/secrets.toml
```

### Using Secrets Manager

```rust
use defi_risk_monitor::config::SecretsManager;

let secrets_manager = SecretsManager::new(None);

// Set a secret
secrets_manager.set_secret("api_key", "secret-value")?;

// Get a secret
let api_key = secrets_manager.get_secret("api_key")?;

// Remove a secret
secrets_manager.remove_secret("old_key")?;
```

## Deployment

### Development

```bash
# Set environment
export ENVIRONMENT=development

# Run with development configuration
cargo run
```

### Staging

```bash
# Set environment
export ENVIRONMENT=staging

# Override specific settings
export DEFI_RISK__DATABASE__MAX_CONNECTIONS=20
export DEFI_RISK__API__CORS_ORIGINS='["https://staging.example.com"]'

# Run application
cargo run --release
```

### Production

```bash
# Set environment
export ENVIRONMENT=production

# Set critical environment variables
export DATABASE_URL="postgresql://user:pass@prod-db:5432/defi_risk_monitor"
export DEFI_RISK__SECURITY__JWT_SECRET="production-jwt-secret-32-chars-minimum"
export DEFI_RISK__API__CORS_ORIGINS='["https://app.example.com"]'

# Run with production main
cargo run --release --bin main_production
```

### Docker Deployment

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin main_production

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/main_production /app/
COPY --from=builder /app/config /app/config
ENV ENVIRONMENT=production
EXPOSE 8080
CMD ["./main_production"]
```

## Monitoring and Health Checks

### Health Check Endpoints

- `GET /health` - Basic health check
- `GET /health/detailed` - Detailed health information
- `GET /config/summary` - Configuration summary (non-sensitive)

### Configuration Health Summary

```json
{
  "environment": "production",
  "database_connections": "10/50",
  "api_endpoint": "0.0.0.0:8080",
  "monitoring_enabled": true,
  "security_features": {
    "rate_limiting": true,
    "two_factor_auth": true,
    "audit_logging": true
  },
  "feature_flags": {
    "real_time_monitoring": true,
    "websockets": true,
    "ai_predictions": true,
    "experimental": false
  }
}
```

## Security Best Practices

### Production Security Checklist

- [ ] Change default JWT secret
- [ ] Enable rate limiting
- [ ] Enable 2FA for admin accounts
- [ ] Enable audit logging
- [ ] Configure TLS certificates
- [ ] Set restrictive CORS origins
- [ ] Use strong database passwords
- [ ] Enable connection encryption
- [ ] Set up monitoring and alerting
- [ ] Configure trusted proxy IPs
- [ ] Secure secrets file permissions (600)
- [ ] Use environment variables for sensitive data

### Configuration Validation

The system automatically validates:

- URL formats and schemes
- Numeric ranges and bounds
- Security policy compliance
- Required fields presence
- File path safety
- Network configuration validity

### Audit Logging

When `security.enable_audit_logging` is enabled, the system logs:

- Configuration changes
- Authentication events
- Administrative actions
- Security policy violations
- Access control decisions

## Troubleshooting

### Common Issues

1. **Configuration not loading**
   - Check environment variable `ENVIRONMENT`
   - Verify config file exists: `config/config.{environment}.toml`
   - Check file permissions

2. **Validation errors**
   - Review validation error messages
   - Check numeric ranges and formats
   - Verify URL schemes and formats

3. **Hot-reloading not working**
   - Ensure config file is writable
   - Check file system permissions
   - Verify notify dependency is working

4. **Secrets not loading**
   - Check `config/secrets.toml` exists
   - Verify file permissions (should be 600)
   - Check TOML syntax

### Debug Configuration

```bash
# Enable debug logging
export RUST_LOG=defi_risk_monitor=debug

# Check configuration loading
cargo run -- --validate-config

# Export current configuration
cargo run -- --export-config > current-config.toml
```

## Migration from Legacy Configuration

### Step 1: Install Dependencies

Add to `Cargo.toml`:
```toml
toml = "0.8"
notify = "6.1"
```

### Step 2: Update Application State

```rust
pub struct AppState {
    pub production_config: config::ProductionConfig,
    pub config_manager: Arc<tokio::sync::Mutex<config::ConfigManager>>,
    // ... other fields
}
```

### Step 3: Update Main Function

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ProductionConfig::load()?;
    let config_manager = ConfigManager::new().await?;
    
    // Use new configuration system
    // ...
}
```

### Step 4: Create Environment Files

Create configuration files for each environment based on your current `.env` files.

### Step 5: Update Service Initialization

Update services to use the new configuration structure instead of individual environment variables.

## Support

For questions or issues with the configuration system:

1. Check this documentation
2. Review configuration validation errors
3. Check application logs for configuration-related messages
4. Verify environment-specific configuration files
5. Test configuration loading in development environment first

The production configuration management system provides a robust, secure, and maintainable approach to application configuration that scales from development to production environments.
