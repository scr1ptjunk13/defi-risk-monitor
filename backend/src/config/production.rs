use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, warn, error};
use config::{Config, ConfigError, Environment, File};

/// Environment types for configuration management
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigEnvironment {
    Development,
    Testing,
    Staging,
    Production,
}

impl ConfigEnvironment {
    pub fn from_env() -> Self {
        match env::var("ENVIRONMENT")
            .or_else(|_| env::var("ENV"))
            .or_else(|_| env::var("NODE_ENV"))
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "prod" | "production" => ConfigEnvironment::Production,
            "stage" | "staging" => ConfigEnvironment::Staging,
            "test" | "testing" => ConfigEnvironment::Testing,
            _ => ConfigEnvironment::Development,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConfigEnvironment::Development => "development",
            ConfigEnvironment::Testing => "testing",
            ConfigEnvironment::Staging => "staging",
            ConfigEnvironment::Production => "production",
        }
    }

    pub fn config_file_name(&self) -> String {
        format!("config.{}.toml", self.as_str())
    }
}

/// Production-ready configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionConfig {
    pub environment: ConfigEnvironment,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub blockchain: BlockchainConfig,
    pub risk: RiskConfig,
    pub alerts: AlertConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub performance: PerformanceConfig,
    pub features: FeatureFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub enable_logging: bool,
    pub slow_query_threshold_ms: u64,
    pub replica_urls: Vec<String>,
    pub enable_read_replicas: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    pub cors_origins: Vec<String>,
    pub request_timeout_seconds: u64,
    pub max_request_size_mb: u64,
    pub rate_limit_requests_per_minute: u64,
    pub enable_compression: bool,
    pub enable_metrics: bool,
    pub tls: Option<TlsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub ethereum_rpc_url: String,
    pub ethereum_ws_url: Option<String>,
    pub polygon_rpc_url: String,
    pub arbitrum_rpc_url: String,
    pub optimism_rpc_url: String,
    pub bsc_rpc_url: String,
    pub avalanche_rpc_url: String,
    pub request_timeout_seconds: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
    pub risk_check_interval_seconds: u64,
    pub block_confirmation_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    pub max_position_size_usd: f64,
    pub liquidation_threshold: f64,
    pub impermanent_loss_threshold: f64,
    pub mev_risk_threshold: f64,
    pub protocol_risk_weights: HashMap<String, f64>,
    pub enable_real_time_monitoring: bool,
    pub risk_calculation_interval_seconds: u64,
    pub enable_predictive_analytics: bool,
    pub confidence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub slack_webhook_url: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub email: Option<EmailConfig>,
    pub webhook_endpoints: Vec<String>,
    pub enable_push_notifications: bool,
    pub alert_cooldown_minutes: u64,
    pub severity_thresholds: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub use_tls: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // json, pretty, compact
    pub output: String, // stdout, file, both
    pub file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub enable_structured_logging: bool,
    pub enable_request_logging: bool,
    pub sensitive_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: String,
    pub jwt_expiration_hours: u64,
    pub enable_rate_limiting: bool,
    pub max_login_attempts: u32,
    pub lockout_duration_minutes: u64,
    pub enable_2fa: bool,
    pub password_min_length: u32,
    pub session_timeout_minutes: u64,
    pub enable_audit_logging: bool,
    pub trusted_proxies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enable_prometheus: bool,
    pub prometheus_port: u16,
    pub enable_jaeger: bool,
    pub jaeger_endpoint: Option<String>,
    pub enable_health_checks: bool,
    pub health_check_interval_seconds: u64,
    pub metrics_retention_days: u32,
    pub enable_alerting: bool,
    pub alert_manager_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub worker_threads: Option<usize>,
    pub max_blocking_threads: Option<usize>,
    pub enable_connection_pooling: bool,
    pub cache_size_mb: u64,
    pub enable_query_caching: bool,
    pub query_cache_ttl_seconds: u64,
    pub enable_compression: bool,
    pub batch_size: u32,
    pub enable_parallel_processing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub enable_websockets: bool,
    pub enable_real_time_updates: bool,
    pub enable_advanced_analytics: bool,
    pub enable_mev_protection: bool,
    pub enable_cross_chain_monitoring: bool,
    pub enable_ai_predictions: bool,
    pub enable_automated_rebalancing: bool,
    pub enable_portfolio_optimization: bool,
    pub enable_experimental_features: bool,
}

impl ProductionConfig {
    /// Load configuration from multiple sources with proper precedence
    pub fn load() -> Result<Self, ConfigError> {
        let environment = ConfigEnvironment::from_env();
        info!("Loading configuration for environment: {}", environment.as_str());

        let mut builder = Config::builder()
            // Start with default configuration
            .add_source(Self::default_config(&environment))
            // Add environment-specific configuration file
            .add_source(File::with_name(&format!("config/{}", environment.config_file_name())).required(false))
            // Add local configuration file (for development overrides)
            .add_source(File::with_name("config/local.toml").required(false))
            // Add environment variables with prefix
            .add_source(Environment::with_prefix("DEFI_RISK").separator("__"))
            // Add standard environment variables
            .add_source(Environment::default());

        // Add secrets file if it exists (for production)
        if environment == ConfigEnvironment::Production {
            if Path::new("config/secrets.toml").exists() {
                builder = builder.add_source(File::with_name("config/secrets.toml"));
            }
        }

        let config = builder.build()?;
        let mut production_config: ProductionConfig = config.try_deserialize()?;
        production_config.environment = environment;

        // Validate configuration
        production_config.validate()?;

        info!("Configuration loaded and validated successfully");
        Ok(production_config)
    }

    /// Create default configuration based on environment
    fn default_config(environment: &ConfigEnvironment) -> Config {
        let mut defaults = HashMap::new();

        // Database defaults
        defaults.insert("database.max_connections".to_string(), match environment {
            ConfigEnvironment::Production => "50".to_string(),
            ConfigEnvironment::Staging => "20".to_string(),
            _ => "10".to_string(),
        });
        defaults.insert("database.min_connections".to_string(), "5".to_string());
        defaults.insert("database.connection_timeout_seconds".to_string(), "30".to_string());
        defaults.insert("database.idle_timeout_seconds".to_string(), "600".to_string());
        defaults.insert("database.max_lifetime_seconds".to_string(), "3600".to_string());
        defaults.insert("database.enable_logging".to_string(), match environment {
            ConfigEnvironment::Production => "false".to_string(),
            _ => "true".to_string(),
        });
        defaults.insert("database.slow_query_threshold_ms".to_string(), "1000".to_string());
        defaults.insert("database.enable_read_replicas".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });

        // API defaults
        defaults.insert("api.host".to_string(), match environment {
            ConfigEnvironment::Production => "0.0.0.0".to_string(),
            _ => "127.0.0.1".to_string(),
        });
        defaults.insert("api.port".to_string(), "8080".to_string());
        defaults.insert("api.request_timeout_seconds".to_string(), "30".to_string());
        defaults.insert("api.max_request_size_mb".to_string(), "16".to_string());
        defaults.insert("api.rate_limit_requests_per_minute".to_string(), match environment {
            ConfigEnvironment::Production => "1000".to_string(),
            _ => "10000".to_string(),
        });
        defaults.insert("api.enable_compression".to_string(), "true".to_string());
        defaults.insert("api.enable_metrics".to_string(), "true".to_string());

        // Blockchain defaults
        defaults.insert("blockchain.request_timeout_seconds".to_string(), "30".to_string());
        defaults.insert("blockchain.max_retries".to_string(), "3".to_string());
        defaults.insert("blockchain.retry_delay_ms".to_string(), "1000".to_string());
        defaults.insert("blockchain.enable_caching".to_string(), "true".to_string());
        defaults.insert("blockchain.cache_ttl_seconds".to_string(), "300".to_string());
        defaults.insert("blockchain.risk_check_interval_seconds".to_string(), match environment {
            ConfigEnvironment::Production => "60".to_string(),
            _ => "300".to_string(),
        });
        defaults.insert("blockchain.block_confirmation_count".to_string(), match environment {
            ConfigEnvironment::Production => "12".to_string(),
            _ => "3".to_string(),
        });

        // Risk defaults
        defaults.insert("risk.max_position_size_usd".to_string(), match environment {
            ConfigEnvironment::Production => "10000000".to_string(),
            _ => "1000000".to_string(),
        });
        defaults.insert("risk.liquidation_threshold".to_string(), "0.85".to_string());
        defaults.insert("risk.impermanent_loss_threshold".to_string(), "0.05".to_string());
        defaults.insert("risk.mev_risk_threshold".to_string(), "0.02".to_string());
        defaults.insert("risk.enable_real_time_monitoring".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("risk.risk_calculation_interval_seconds".to_string(), "30".to_string());
        defaults.insert("risk.enable_predictive_analytics".to_string(), "true".to_string());
        defaults.insert("risk.confidence_threshold".to_string(), "0.8".to_string());

        // Logging defaults
        defaults.insert("logging.level".to_string(), match environment {
            ConfigEnvironment::Production => "info".to_string(),
            ConfigEnvironment::Staging => "debug".to_string(),
            _ => "debug".to_string(),
        });
        defaults.insert("logging.format".to_string(), match environment {
            ConfigEnvironment::Production => "json".to_string(),
            _ => "pretty".to_string(),
        });
        defaults.insert("logging.output".to_string(), match environment {
            ConfigEnvironment::Production => "both".to_string(),
            _ => "stdout".to_string(),
        });
        defaults.insert("logging.max_file_size_mb".to_string(), "100".to_string());
        defaults.insert("logging.max_files".to_string(), "10".to_string());
        defaults.insert("logging.enable_structured_logging".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("logging.enable_request_logging".to_string(), "true".to_string());

        // Security defaults
        defaults.insert("security.jwt_expiration_hours".to_string(), match environment {
            ConfigEnvironment::Production => "24".to_string(),
            _ => "168".to_string(), // 1 week for development
        });
        defaults.insert("security.enable_rate_limiting".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("security.max_login_attempts".to_string(), "5".to_string());
        defaults.insert("security.lockout_duration_minutes".to_string(), "15".to_string());
        defaults.insert("security.enable_2fa".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("security.password_min_length".to_string(), "12".to_string());
        defaults.insert("security.session_timeout_minutes".to_string(), "480".to_string()); // 8 hours
        defaults.insert("security.enable_audit_logging".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });

        // Monitoring defaults
        defaults.insert("monitoring.enable_prometheus".to_string(), "true".to_string());
        defaults.insert("monitoring.prometheus_port".to_string(), "9090".to_string());
        defaults.insert("monitoring.enable_jaeger".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("monitoring.enable_health_checks".to_string(), "true".to_string());
        defaults.insert("monitoring.health_check_interval_seconds".to_string(), "30".to_string());
        defaults.insert("monitoring.metrics_retention_days".to_string(), match environment {
            ConfigEnvironment::Production => "90".to_string(),
            _ => "7".to_string(),
        });
        defaults.insert("monitoring.enable_alerting".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });

        // Performance defaults
        defaults.insert("performance.enable_connection_pooling".to_string(), "true".to_string());
        defaults.insert("performance.cache_size_mb".to_string(), match environment {
            ConfigEnvironment::Production => "512".to_string(),
            _ => "128".to_string(),
        });
        defaults.insert("performance.enable_query_caching".to_string(), "true".to_string());
        defaults.insert("performance.query_cache_ttl_seconds".to_string(), "300".to_string());
        defaults.insert("performance.enable_compression".to_string(), "true".to_string());
        defaults.insert("performance.batch_size".to_string(), "1000".to_string());
        defaults.insert("performance.enable_parallel_processing".to_string(), "true".to_string());

        // Feature flags defaults
        defaults.insert("features.enable_websockets".to_string(), "true".to_string());
        defaults.insert("features.enable_real_time_updates".to_string(), "true".to_string());
        defaults.insert("features.enable_advanced_analytics".to_string(), "true".to_string());
        defaults.insert("features.enable_mev_protection".to_string(), "true".to_string());
        defaults.insert("features.enable_cross_chain_monitoring".to_string(), "true".to_string());
        defaults.insert("features.enable_ai_predictions".to_string(), match environment {
            ConfigEnvironment::Production => "true".to_string(),
            _ => "false".to_string(),
        });
        defaults.insert("features.enable_automated_rebalancing".to_string(), match environment {
            ConfigEnvironment::Production => "false".to_string(), // Disabled by default for safety
            _ => "false".to_string(),
        });
        defaults.insert("features.enable_portfolio_optimization".to_string(), "true".to_string());
        defaults.insert("features.enable_experimental_features".to_string(), match environment {
            ConfigEnvironment::Production => "false".to_string(),
            _ => "true".to_string(),
        });

        // Create a simple config with defaults using set_default
        let mut builder = Config::builder();
        for (key, value) in defaults {
            builder = builder.set_default(&key, value).expect(&format!("Failed to set default for {}", key));
        }
        
        builder
            .build()
            .expect("Failed to create default configuration")
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate database configuration
        if self.database.max_connections < self.database.min_connections {
            return Err(ConfigError::Message(
                "database.max_connections must be >= database.min_connections".to_string()
            ));
        }

        // Validate API configuration
        if self.api.port < 1024 && self.environment == ConfigEnvironment::Production {
            warn!("Using privileged port {} in production", self.api.port);
        }

        // Validate security configuration
        if self.environment == ConfigEnvironment::Production {
            if self.security.jwt_secret.len() < 32 {
                return Err(ConfigError::Message(
                    "JWT secret must be at least 32 characters in production".to_string()
                ));
            }

            if self.security.password_min_length < 8 {
                return Err(ConfigError::Message(
                    "Password minimum length must be at least 8 in production".to_string()
                ));
            }
        }

        // Validate risk configuration
        if self.risk.liquidation_threshold <= 0.0 || self.risk.liquidation_threshold >= 1.0 {
            return Err(ConfigError::Message(
                "liquidation_threshold must be between 0 and 1".to_string()
            ));
        }

        // Validate blockchain URLs
        let rpc_urls = vec![
            &self.blockchain.ethereum_rpc_url,
            &self.blockchain.polygon_rpc_url,
            &self.blockchain.arbitrum_rpc_url,
        ];

        for url in rpc_urls {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ConfigError::Message(
                    format!("Invalid RPC URL format: {}", url)
                ));
            }
        }

        info!("Configuration validation completed successfully");
        Ok(())
    }

    /// Get configuration summary for logging (without sensitive data)
    pub fn summary(&self) -> HashMap<String, String> {
        let mut summary = HashMap::new();
        
        summary.insert("environment".to_string(), self.environment.as_str().to_string());
        summary.insert("api_port".to_string(), self.api.port.to_string());
        summary.insert("database_max_connections".to_string(), self.database.max_connections.to_string());
        summary.insert("logging_level".to_string(), self.logging.level.clone());
        summary.insert("monitoring_enabled".to_string(), self.monitoring.enable_prometheus.to_string());
        summary.insert("real_time_monitoring".to_string(), self.risk.enable_real_time_monitoring.to_string());
        
        summary
    }

    /// Check if running in production environment
    pub fn is_production(&self) -> bool {
        self.environment == ConfigEnvironment::Production
    }

    /// Check if running in development environment
    pub fn is_development(&self) -> bool {
        self.environment == ConfigEnvironment::Development
    }

    /// Get feature flag value
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "websockets" => self.features.enable_websockets,
            "real_time_updates" => self.features.enable_real_time_updates,
            "advanced_analytics" => self.features.enable_advanced_analytics,
            "mev_protection" => self.features.enable_mev_protection,
            "cross_chain_monitoring" => self.features.enable_cross_chain_monitoring,
            "ai_predictions" => self.features.enable_ai_predictions,
            "automated_rebalancing" => self.features.enable_automated_rebalancing,
            "portfolio_optimization" => self.features.enable_portfolio_optimization,
            "experimental_features" => self.features.enable_experimental_features,
            _ => false,
        }
    }
}

/// Configuration hot-reloading support
pub struct ConfigWatcher {
    config: Arc<tokio::sync::RwLock<ProductionConfig>>,
    _watcher: notify::RecommendedWatcher,
}

impl ConfigWatcher {
    pub fn new(config: ProductionConfig) -> Result<Self, Box<dyn std::error::Error>> {
        use notify::{Watcher, RecursiveMode, Event, EventKind};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        let config = Arc::new(RwLock::new(config));
        let config_clone = config.clone();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            match res {
                Ok(event) => {
                    if let EventKind::Modify(_) = event.kind {
                        info!("Configuration file changed, reloading...");
                        let config_clone = config_clone.clone();
                        tokio::spawn(async move {
                            match ProductionConfig::load() {
                                Ok(new_config) => {
                                    let mut config_guard = config_clone.write().await;
                                    *config_guard = new_config;
                                    info!("Configuration reloaded successfully");
                                }
                                Err(e) => {
                                    error!("Failed to reload configuration: {}", e);
                                }
                            }
                        });
                    }
                }
                Err(e) => error!("Configuration watcher error: {:?}", e),
            }
        })?;

        watcher.watch(Path::new("config"), RecursiveMode::NonRecursive)?;

        Ok(ConfigWatcher {
            config,
            _watcher: watcher,
        })
    }

    pub async fn get_config(&self) -> ProductionConfig {
        self.config.read().await.clone()
    }
}
