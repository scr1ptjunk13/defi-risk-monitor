use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub api: ApiSettings,
    pub blockchain: BlockchainSettings,
    pub risk: RiskSettings,
    pub alerts: AlertSettings,
    pub logging: LoggingSettings,
    pub ai_service: AIServiceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainSettings {
    pub ethereum_rpc_url: String,
    pub polygon_rpc_url: String,
    pub arbitrum_rpc_url: String,
    pub risk_check_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSettings {
    pub max_position_size_usd: f64,
    pub liquidation_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertSettings {
    pub slack_webhook_url: Option<String>,
    pub discord_webhook_url: Option<String>,
    pub email_smtp_host: Option<String>,
    pub email_smtp_port: Option<u16>,
    pub email_username: Option<String>,
    pub email_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIServiceSettings {
    pub url: String,
    pub timeout_seconds: u64,
    pub fallback_enabled: bool,
    pub retry_attempts: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            database: DatabaseSettings::default(),
            api: ApiSettings::default(),
            blockchain: BlockchainSettings::default(),
            risk: RiskSettings::default(),
            alerts: AlertSettings::default(),
            logging: LoggingSettings::default(),
            ai_service: AIServiceSettings::default(),
        }
    }
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        DatabaseSettings {
            url: "postgresql://postgres:password@localhost:5432/defi_risk_monitor_test".to_string(),
        }
    }
}

impl Default for ApiSettings {
    fn default() -> Self {
        ApiSettings {
            host: "0.0.0.0".to_string(),
            port: 8080,
        }
    }
}

impl Default for BlockchainSettings {
    fn default() -> Self {
        BlockchainSettings {
            ethereum_rpc_url: "https://eth-mainnet.alchemyapi.io/v2/test".to_string(),
            polygon_rpc_url: "https://polygon-mainnet.alchemyapi.io/v2/test".to_string(),
            arbitrum_rpc_url: "https://arb-mainnet.alchemyapi.io/v2/test".to_string(),
            risk_check_interval_seconds: 60,
        }
    }
}

impl Default for RiskSettings {
    fn default() -> Self {
        RiskSettings {
            max_position_size_usd: 1000000.0,
            liquidation_threshold: 0.85,
        }
    }
}

impl Default for AlertSettings {
    fn default() -> Self {
        AlertSettings {
            slack_webhook_url: None,
            discord_webhook_url: None,
            email_smtp_host: None,
            email_smtp_port: None,
            email_username: None,
            email_password: None,
        }
    }
}

impl Default for LoggingSettings {
    fn default() -> Self {
        LoggingSettings {
            level: "info".to_string(),
        }
    }
}

impl Default for AIServiceSettings {
    fn default() -> Self {
        AIServiceSettings {
            url: "http://localhost:8001".to_string(),
            timeout_seconds: 30,
            fallback_enabled: true,
            retry_attempts: 3,
        }
    }
}

impl Settings {
    pub fn new() -> Result<Self, config::ConfigError> {
        let _settings = config::Config::builder()
            .add_source(config::Environment::default())
            .build()?;

        Ok(Settings {
            database: DatabaseSettings {
                url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5432/defi_risk_monitor".to_string()),
            },
            api: ApiSettings {
                host: env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("API_PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .unwrap_or(8080),
            },
            blockchain: BlockchainSettings {
                ethereum_rpc_url: env::var("ETHEREUM_RPC_URL")
                    .expect("ETHEREUM_RPC_URL must be set"),
                polygon_rpc_url: env::var("POLYGON_RPC_URL")
                    .expect("POLYGON_RPC_URL must be set"),
                arbitrum_rpc_url: env::var("ARBITRUM_RPC_URL")
                    .expect("ARBITRUM_RPC_URL must be set"),
                risk_check_interval_seconds: env::var("RISK_CHECK_INTERVAL_SECONDS")
                    .unwrap_or_else(|_| "60".to_string())
                    .parse()
                    .unwrap_or(60),
            },
            risk: RiskSettings {
                max_position_size_usd: env::var("MAX_POSITION_SIZE_USD")
                    .unwrap_or_else(|_| "1000000".to_string())
                    .parse()
                    .unwrap_or(1000000.0),
                liquidation_threshold: env::var("LIQUIDATION_THRESHOLD")
                    .unwrap_or_else(|_| "0.85".to_string())
                    .parse()
                    .unwrap_or(0.85),
            },
            alerts: AlertSettings {
                slack_webhook_url: env::var("SLACK_WEBHOOK_URL").ok(),
                discord_webhook_url: env::var("DISCORD_WEBHOOK_URL").ok(),
                email_smtp_host: env::var("EMAIL_SMTP_HOST").ok(),
                email_smtp_port: env::var("EMAIL_SMTP_PORT").ok().and_then(|s| s.parse().ok()),
                email_username: env::var("EMAIL_USERNAME").ok(),
                email_password: env::var("EMAIL_PASSWORD").ok(),
            },
            logging: LoggingSettings {
                level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            },
            ai_service: AIServiceSettings {
                url: env::var("AI_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8001".to_string()),
                timeout_seconds: env::var("AI_SERVICE_TIMEOUT")
                    .unwrap_or_else(|_| "30".to_string())
                    .parse()
                    .unwrap_or(30),
                fallback_enabled: env::var("AI_SERVICE_FALLBACK_ENABLED")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse()
                    .unwrap_or(true),
                retry_attempts: env::var("AI_SERVICE_RETRY_ATTEMPTS")
                    .unwrap_or_else(|_| "3".to_string())
                    .parse()
                    .unwrap_or(3),
            },
        })
    }
}
