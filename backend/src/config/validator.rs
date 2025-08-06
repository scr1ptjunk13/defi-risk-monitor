use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use url::Url;
use regex::Regex;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Invalid range: {field} must be between {min} and {max}, got {value}")]
    InvalidRange { field: String, min: f64, max: f64, value: f64 },
    #[error("Invalid format: {field} - {message}")]
    InvalidFormat { field: String, message: String },
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Security violation: {0}")]
    SecurityViolation(String),
    #[error("Environment mismatch: {0}")]
    EnvironmentMismatch(String),
}

pub type ValidationResult<T> = Result<T, ValidationError>;

/// Configuration validation rules and schemas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    pub database: DatabaseValidationRules,
    pub api: ApiValidationRules,
    pub blockchain: BlockchainValidationRules,
    pub risk: RiskValidationRules,
    pub security: SecurityValidationRules,
    pub performance: PerformanceValidationRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseValidationRules {
    pub max_connections_range: (u32, u32),
    pub connection_timeout_range: (u64, u64),
    pub required_url_schemes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiValidationRules {
    pub port_range: (u16, u16),
    pub timeout_range: (u64, u64),
    pub max_request_size_range: (u64, u64),
    pub allowed_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainValidationRules {
    pub required_networks: Vec<String>,
    pub timeout_range: (u64, u64),
    pub retry_range: (u32, u32),
    pub confirmation_range: (u64, u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskValidationRules {
    pub threshold_range: (f64, f64),
    pub position_size_range: (f64, f64),
    pub interval_range: (u64, u64),
    pub confidence_range: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityValidationRules {
    pub jwt_secret_min_length: usize,
    pub password_min_length: u32,
    pub session_timeout_range: (u64, u64),
    pub max_login_attempts_range: (u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceValidationRules {
    pub cache_size_range: (u64, u64),
    pub batch_size_range: (u32, u32),
    pub thread_count_range: (usize, usize),
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            database: DatabaseValidationRules {
                max_connections_range: (1, 1000),
                connection_timeout_range: (1, 300),
                required_url_schemes: vec!["postgresql".to_string(), "postgres".to_string()],
            },
            api: ApiValidationRules {
                port_range: (1024, 65535),
                timeout_range: (1, 300),
                max_request_size_range: (1, 1024),
                allowed_hosts: vec![
                    "0.0.0.0".to_string(),
                    "127.0.0.1".to_string(),
                    "localhost".to_string(),
                ],
            },
            blockchain: BlockchainValidationRules {
                required_networks: vec![
                    "ethereum".to_string(),
                    "polygon".to_string(),
                    "arbitrum".to_string(),
                ],
                timeout_range: (5, 300),
                retry_range: (0, 10),
                confirmation_range: (1, 100),
            },
            risk: RiskValidationRules {
                threshold_range: (0.01, 0.99),
                position_size_range: (1.0, 1_000_000_000.0),
                interval_range: (1, 3600),
                confidence_range: (0.1, 1.0),
            },
            security: SecurityValidationRules {
                jwt_secret_min_length: 32,
                password_min_length: 8,
                session_timeout_range: (1, 10080), // 1 minute to 1 week
                max_login_attempts_range: (1, 100),
            },
            performance: PerformanceValidationRules {
                cache_size_range: (1, 10240), // 1MB to 10GB
                batch_size_range: (1, 100000),
                thread_count_range: (1, 1000),
            },
        }
    }
}

/// Configuration validator with comprehensive validation logic
pub struct ConfigValidator {
    rules: ValidationRules,
    url_regex: Regex,
    email_regex: Regex,
}

impl ConfigValidator {
    pub fn new() -> Self {
        Self {
            rules: ValidationRules::default(),
            url_regex: Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap(),
            email_regex: Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap(),
        }
    }

    pub fn with_rules(rules: ValidationRules) -> Self {
        Self {
            rules,
            url_regex: Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap(),
            email_regex: Regex::new(r"^[^\s@]+@[^\s@]+\.[^\s@]+$").unwrap(),
        }
    }

    /// Validate database configuration
    pub fn validate_database_config(&self, config: &super::production::DatabaseConfig) -> ValidationResult<()> {
        // Validate URL format
        self.validate_url(&config.url, "database.url")?;
        
        // Check URL scheme
        let url = Url::parse(&config.url)
            .map_err(|_| ValidationError::InvalidUrl(config.url.clone()))?;
        
        if !self.rules.database.required_url_schemes.contains(&url.scheme().to_string()) {
            return Err(ValidationError::InvalidFormat {
                field: "database.url".to_string(),
                message: format!("Unsupported scheme: {}", url.scheme()),
            });
        }

        // Validate connection pool settings
        self.validate_range(
            config.max_connections as f64,
            self.rules.database.max_connections_range.0 as f64,
            self.rules.database.max_connections_range.1 as f64,
            "database.max_connections",
        )?;

        if config.max_connections < config.min_connections {
            return Err(ValidationError::InvalidFormat {
                field: "database.max_connections".to_string(),
                message: "max_connections must be >= min_connections".to_string(),
            });
        }

        self.validate_range(
            config.connection_timeout_seconds as f64,
            self.rules.database.connection_timeout_range.0 as f64,
            self.rules.database.connection_timeout_range.1 as f64,
            "database.connection_timeout_seconds",
        )?;

        // Validate replica URLs if provided
        for (i, replica_url) in config.replica_urls.iter().enumerate() {
            self.validate_url(replica_url, &format!("database.replica_urls[{}]", i))?;
        }

        Ok(())
    }

    /// Validate API configuration
    pub fn validate_api_config(&self, config: &super::production::ApiConfig) -> ValidationResult<()> {
        // Validate port range
        self.validate_range(
            config.port as f64,
            self.rules.api.port_range.0 as f64,
            self.rules.api.port_range.1 as f64,
            "api.port",
        )?;

        // Validate host
        if !self.rules.api.allowed_hosts.contains(&config.host) && !self.is_valid_ip(&config.host) {
            return Err(ValidationError::InvalidFormat {
                field: "api.host".to_string(),
                message: format!("Invalid host: {}", config.host),
            });
        }

        // Validate timeout
        self.validate_range(
            config.request_timeout_seconds as f64,
            self.rules.api.timeout_range.0 as f64,
            self.rules.api.timeout_range.1 as f64,
            "api.request_timeout_seconds",
        )?;

        // Validate CORS origins
        for (i, origin) in config.cors_origins.iter().enumerate() {
            if origin != "*" && !self.url_regex.is_match(origin) {
                return Err(ValidationError::InvalidFormat {
                    field: format!("api.cors_origins[{}]", i),
                    message: format!("Invalid CORS origin: {}", origin),
                });
            }
        }

        // Validate TLS configuration if present
        if let Some(tls) = &config.tls {
            self.validate_file_path(&tls.cert_file, "api.tls.cert_file")?;
            self.validate_file_path(&tls.key_file, "api.tls.key_file")?;
            if let Some(ca_file) = &tls.ca_file {
                self.validate_file_path(ca_file, "api.tls.ca_file")?;
            }
        }

        Ok(())
    }

    /// Validate blockchain configuration
    pub fn validate_blockchain_config(&self, config: &super::production::BlockchainConfig) -> ValidationResult<()> {
        // Validate RPC URLs
        let rpc_urls = vec![
            (&config.ethereum_rpc_url, "blockchain.ethereum_rpc_url"),
            (&config.polygon_rpc_url, "blockchain.polygon_rpc_url"),
            (&config.arbitrum_rpc_url, "blockchain.arbitrum_rpc_url"),
            (&config.optimism_rpc_url, "blockchain.optimism_rpc_url"),
            (&config.bsc_rpc_url, "blockchain.bsc_rpc_url"),
            (&config.avalanche_rpc_url, "blockchain.avalanche_rpc_url"),
        ];

        for (url, field) in rpc_urls {
            self.validate_url(url, field)?;
        }

        // Validate WebSocket URL if present
        if let Some(ws_url) = &config.ethereum_ws_url {
            if !ws_url.starts_with("ws://") && !ws_url.starts_with("wss://") {
                return Err(ValidationError::InvalidFormat {
                    field: "blockchain.ethereum_ws_url".to_string(),
                    message: "WebSocket URL must start with ws:// or wss://".to_string(),
                });
            }
        }

        // Validate timeout and retry settings
        self.validate_range(
            config.request_timeout_seconds as f64,
            self.rules.blockchain.timeout_range.0 as f64,
            self.rules.blockchain.timeout_range.1 as f64,
            "blockchain.request_timeout_seconds",
        )?;

        self.validate_range(
            config.max_retries as f64,
            self.rules.blockchain.retry_range.0 as f64,
            self.rules.blockchain.retry_range.1 as f64,
            "blockchain.max_retries",
        )?;

        self.validate_range(
            config.block_confirmation_count as f64,
            self.rules.blockchain.confirmation_range.0 as f64,
            self.rules.blockchain.confirmation_range.1 as f64,
            "blockchain.block_confirmation_count",
        )?;

        Ok(())
    }

    /// Validate risk configuration
    pub fn validate_risk_config(&self, config: &super::production::RiskConfig) -> ValidationResult<()> {
        // Validate thresholds
        self.validate_range(
            config.liquidation_threshold,
            self.rules.risk.threshold_range.0,
            self.rules.risk.threshold_range.1,
            "risk.liquidation_threshold",
        )?;

        self.validate_range(
            config.impermanent_loss_threshold,
            self.rules.risk.threshold_range.0,
            self.rules.risk.threshold_range.1,
            "risk.impermanent_loss_threshold",
        )?;

        self.validate_range(
            config.mev_risk_threshold,
            self.rules.risk.threshold_range.0,
            self.rules.risk.threshold_range.1,
            "risk.mev_risk_threshold",
        )?;

        // Validate position size
        self.validate_range(
            config.max_position_size_usd,
            self.rules.risk.position_size_range.0,
            self.rules.risk.position_size_range.1,
            "risk.max_position_size_usd",
        )?;

        // Validate intervals
        self.validate_range(
            config.risk_calculation_interval_seconds as f64,
            self.rules.risk.interval_range.0 as f64,
            self.rules.risk.interval_range.1 as f64,
            "risk.risk_calculation_interval_seconds",
        )?;

        // Validate confidence threshold
        self.validate_range(
            config.confidence_threshold,
            self.rules.risk.confidence_range.0,
            self.rules.risk.confidence_range.1,
            "risk.confidence_threshold",
        )?;

        // Validate protocol risk weights
        for (protocol, weight) in &config.protocol_risk_weights {
            if *weight < 0.0 || *weight > 10.0 {
                return Err(ValidationError::InvalidRange {
                    field: format!("risk.protocol_risk_weights.{}", protocol),
                    min: 0.0,
                    max: 10.0,
                    value: *weight,
                });
            }
        }

        Ok(())
    }

    /// Validate security configuration
    pub fn validate_security_config(&self, config: &super::production::SecurityConfig, is_production: bool) -> ValidationResult<()> {
        // Validate JWT secret
        if config.jwt_secret.len() < self.rules.security.jwt_secret_min_length {
            return Err(ValidationError::SecurityViolation(
                format!("JWT secret must be at least {} characters", self.rules.security.jwt_secret_min_length)
            ));
        }

        // In production, ensure JWT secret is not the default
        if is_production && config.jwt_secret.contains("change-in-production") {
            return Err(ValidationError::SecurityViolation(
                "JWT secret must be changed from default in production".to_string()
            ));
        }

        // Validate password requirements
        if config.password_min_length < self.rules.security.password_min_length {
            return Err(ValidationError::InvalidRange {
                field: "security.password_min_length".to_string(),
                min: self.rules.security.password_min_length as f64,
                max: 128.0,
                value: config.password_min_length as f64,
            });
        }

        // Validate session timeout
        self.validate_range(
            config.session_timeout_minutes as f64,
            self.rules.security.session_timeout_range.0 as f64,
            self.rules.security.session_timeout_range.1 as f64,
            "security.session_timeout_minutes",
        )?;

        // Validate login attempts
        self.validate_range(
            config.max_login_attempts as f64,
            self.rules.security.max_login_attempts_range.0 as f64,
            self.rules.security.max_login_attempts_range.1 as f64,
            "security.max_login_attempts",
        )?;

        // Validate trusted proxies
        for (i, proxy) in config.trusted_proxies.iter().enumerate() {
            if !self.is_valid_ip(proxy) && !self.is_valid_cidr(proxy) {
                return Err(ValidationError::InvalidFormat {
                    field: format!("security.trusted_proxies[{}]", i),
                    message: format!("Invalid IP address or CIDR: {}", proxy),
                });
            }
        }

        Ok(())
    }

    /// Validate alert configuration
    pub fn validate_alert_config(&self, config: &super::production::AlertConfig) -> ValidationResult<()> {
        // Validate webhook URLs
        if let Some(slack_url) = &config.slack_webhook_url {
            self.validate_url(slack_url, "alerts.slack_webhook_url")?;
        }

        if let Some(discord_url) = &config.discord_webhook_url {
            self.validate_url(discord_url, "alerts.discord_webhook_url")?;
        }

        // Validate webhook endpoints
        for (i, endpoint) in config.webhook_endpoints.iter().enumerate() {
            self.validate_url(endpoint, &format!("alerts.webhook_endpoints[{}]", i))?;
        }

        // Validate email configuration if present
        if let Some(email) = &config.email {
            if !self.email_regex.is_match(&email.from_address) {
                return Err(ValidationError::InvalidFormat {
                    field: "alerts.email.from_address".to_string(),
                    message: format!("Invalid email address: {}", email.from_address),
                });
            }

            if !self.email_regex.is_match(&email.username) {
                return Err(ValidationError::InvalidFormat {
                    field: "alerts.email.username".to_string(),
                    message: format!("Invalid email address: {}", email.username),
                });
            }

            if email.smtp_port == 0 || email.smtp_port > 65535 {
                return Err(ValidationError::InvalidRange {
                    field: "alerts.email.smtp_port".to_string(),
                    min: 1.0,
                    max: 65535.0,
                    value: email.smtp_port as f64,
                });
            }
        }

        // Validate severity thresholds
        for (severity, threshold) in &config.severity_thresholds {
            if *threshold < 0.0 || *threshold > 1.0 {
                return Err(ValidationError::InvalidRange {
                    field: format!("alerts.severity_thresholds.{}", severity),
                    min: 0.0,
                    max: 1.0,
                    value: *threshold,
                });
            }
        }

        Ok(())
    }

    /// Helper method to validate URL format
    fn validate_url(&self, url: &str, field: &str) -> ValidationResult<()> {
        if !self.url_regex.is_match(url) {
            return Err(ValidationError::InvalidUrl(format!("{}: {}", field, url)));
        }
        Ok(())
    }

    /// Helper method to validate numeric ranges
    fn validate_range(&self, value: f64, min: f64, max: f64, field: &str) -> ValidationResult<()> {
        if value < min || value > max {
            return Err(ValidationError::InvalidRange {
                field: field.to_string(),
                min,
                max,
                value,
            });
        }
        Ok(())
    }

    /// Helper method to validate file paths
    fn validate_file_path(&self, path: &str, field: &str) -> ValidationResult<()> {
        if path.is_empty() {
            return Err(ValidationError::MissingField(field.to_string()));
        }

        // Basic path validation - could be enhanced with actual file existence checks
        if path.contains("..") || path.contains("//") {
            return Err(ValidationError::SecurityViolation(
                format!("Potentially unsafe file path in {}: {}", field, path)
            ));
        }

        Ok(())
    }

    /// Helper method to validate IP addresses
    fn is_valid_ip(&self, ip: &str) -> bool {
        ip.parse::<std::net::IpAddr>().is_ok()
    }

    /// Helper method to validate CIDR notation
    fn is_valid_cidr(&self, cidr: &str) -> bool {
        if let Some((ip, prefix)) = cidr.split_once('/') {
            if let Ok(prefix_len) = prefix.parse::<u8>() {
                if let Ok(addr) = ip.parse::<std::net::IpAddr>() {
                    return match addr {
                        std::net::IpAddr::V4(_) => prefix_len <= 32,
                        std::net::IpAddr::V6(_) => prefix_len <= 128,
                    };
                }
            }
        }
        false
    }
}

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_validation() {
        let validator = ConfigValidator::new();
        
        assert!(validator.validate_url("https://example.com", "test").is_ok());
        assert!(validator.validate_url("http://localhost:8080", "test").is_ok());
        assert!(validator.validate_url("invalid-url", "test").is_err());
        assert!(validator.validate_url("", "test").is_err());
    }

    #[test]
    fn test_range_validation() {
        let validator = ConfigValidator::new();
        
        assert!(validator.validate_range(5.0, 1.0, 10.0, "test").is_ok());
        assert!(validator.validate_range(0.5, 1.0, 10.0, "test").is_err());
        assert!(validator.validate_range(15.0, 1.0, 10.0, "test").is_err());
    }

    #[test]
    fn test_ip_validation() {
        let validator = ConfigValidator::new();
        
        assert!(validator.is_valid_ip("192.168.1.1"));
        assert!(validator.is_valid_ip("::1"));
        assert!(!validator.is_valid_ip("invalid-ip"));
    }

    #[test]
    fn test_cidr_validation() {
        let validator = ConfigValidator::new();
        
        assert!(validator.is_valid_cidr("192.168.1.0/24"));
        assert!(validator.is_valid_cidr("::1/128"));
        assert!(!validator.is_valid_cidr("192.168.1.0/33"));
        assert!(!validator.is_valid_cidr("invalid/24"));
    }
}
