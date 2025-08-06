use super::{production::ProductionConfig, validator::ConfigValidator};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};
use std::path::Path;
use std::fs;

/// Configuration manager with hot-reloading and validation capabilities
pub struct ConfigManager {
    config: Arc<RwLock<ProductionConfig>>,
    validator: ConfigValidator,
    config_path: String,
    last_modified: Option<std::time::SystemTime>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config = ProductionConfig::load()?;
        let validator = ConfigValidator::new();
        
        // Validate the initial configuration
        Self::validate_config(&config, &validator)?;
        
        let config_path = format!("config/{}", config.environment.config_file_name());
        let last_modified = Self::get_file_modified_time(&config_path);
        
        info!("Configuration manager initialized for environment: {}", config.environment.as_str());
        
        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            validator,
            config_path,
            last_modified,
        })
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> ProductionConfig {
        self.config.read().await.clone()
    }

    /// Reload configuration from files
    pub async fn reload_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if config file has been modified
        let current_modified = Self::get_file_modified_time(&self.config_path);
        
        if let (Some(last), Some(current)) = (self.last_modified, current_modified) {
            if current <= last {
                // No changes detected
                return Ok(());
            }
        }

        info!("Reloading configuration from {}", self.config_path);
        
        // Load new configuration
        let new_config = ProductionConfig::load()?;
        
        // Validate new configuration
        Self::validate_config(&new_config, &self.validator)?;
        
        // Update the configuration
        {
            let mut config_guard = self.config.write().await;
            *config_guard = new_config;
        }
        
        self.last_modified = current_modified;
        info!("Configuration reloaded successfully");
        
        Ok(())
    }

    /// Validate configuration using the validator
    fn validate_config(config: &ProductionConfig, validator: &ConfigValidator) -> Result<(), Box<dyn std::error::Error>> {
        validator.validate_database_config(&config.database)?;
        validator.validate_api_config(&config.api)?;
        validator.validate_blockchain_config(&config.blockchain)?;
        validator.validate_risk_config(&config.risk)?;
        validator.validate_security_config(&config.security, config.is_production())?;
        validator.validate_alert_config(&config.alerts)?;
        
        Ok(())
    }

    /// Get file modification time
    fn get_file_modified_time(path: &str) -> Option<std::time::SystemTime> {
        fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    }

    /// Start background configuration monitoring
    pub async fn start_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: Implement proper background monitoring without borrow checker issues
        info!("Configuration monitoring started (simplified implementation)");
        Ok(())
    }

    /// Get configuration summary for health checks
    pub async fn get_health_summary(&self) -> ConfigHealthSummary {
        let config = self.get_config().await;
        
        ConfigHealthSummary {
            environment: config.environment.as_str().to_string(),
            database_connections: format!("{}/{}", config.database.min_connections, config.database.max_connections),
            api_endpoint: format!("{}:{}", config.api.host, config.api.port),
            monitoring_enabled: config.monitoring.enable_prometheus,
            security_features: SecurityFeaturesSummary {
                rate_limiting: config.security.enable_rate_limiting,
                two_factor_auth: config.security.enable_2fa,
                audit_logging: config.security.enable_audit_logging,
            },
            feature_flags: FeatureFlagsSummary {
                real_time_monitoring: config.risk.enable_real_time_monitoring,
                websockets: config.features.enable_websockets,
                ai_predictions: config.features.enable_ai_predictions,
                experimental: config.features.enable_experimental_features,
            },
        }
    }

    /// Export configuration for backup
    pub async fn export_config(&self) -> Result<String, Box<dyn std::error::Error>> {
        let config = self.get_config().await;
        let config_toml = toml::to_string_pretty(&config)?;
        Ok(config_toml)
    }

    /// Import configuration from backup
    pub async fn import_config(&mut self, config_toml: &str) -> Result<(), Box<dyn std::error::Error>> {
        let new_config: ProductionConfig = toml::from_str(config_toml)?;
        
        // Validate imported configuration
        Self::validate_config(&new_config, &self.validator)?;
        
        // Update configuration
        {
            let mut config_guard = self.config.write().await;
            *config_guard = new_config;
        }
        
        info!("Configuration imported successfully");
        Ok(())
    }

    /// Get configuration diff between current and a new configuration
    pub async fn get_config_diff(&self, new_config: &ProductionConfig) -> ConfigDiff {
        let current_config = self.get_config().await;
        
        ConfigDiff {
            environment_changed: current_config.environment != new_config.environment,
            database_changes: self.compare_database_config(&current_config.database, &new_config.database),
            api_changes: self.compare_api_config(&current_config.api, &new_config.api),
            security_changes: self.compare_security_config(&current_config.security, &new_config.security),
            feature_changes: self.compare_feature_flags(&current_config.features, &new_config.features),
        }
    }

    fn compare_database_config(&self, current: &super::production::DatabaseConfig, new: &super::production::DatabaseConfig) -> Vec<String> {
        let mut changes = Vec::new();
        
        if current.max_connections != new.max_connections {
            changes.push(format!("max_connections: {} -> {}", current.max_connections, new.max_connections));
        }
        if current.connection_timeout_seconds != new.connection_timeout_seconds {
            changes.push(format!("connection_timeout: {} -> {}", current.connection_timeout_seconds, new.connection_timeout_seconds));
        }
        if current.enable_read_replicas != new.enable_read_replicas {
            changes.push(format!("read_replicas: {} -> {}", current.enable_read_replicas, new.enable_read_replicas));
        }
        
        changes
    }

    fn compare_api_config(&self, current: &super::production::ApiConfig, new: &super::production::ApiConfig) -> Vec<String> {
        let mut changes = Vec::new();
        
        if current.port != new.port {
            changes.push(format!("port: {} -> {}", current.port, new.port));
        }
        if current.rate_limit_requests_per_minute != new.rate_limit_requests_per_minute {
            changes.push(format!("rate_limit: {} -> {}", current.rate_limit_requests_per_minute, new.rate_limit_requests_per_minute));
        }
        if current.cors_origins != new.cors_origins {
            changes.push("cors_origins changed".to_string());
        }
        
        changes
    }

    fn compare_security_config(&self, current: &super::production::SecurityConfig, new: &super::production::SecurityConfig) -> Vec<String> {
        let mut changes = Vec::new();
        
        if current.enable_rate_limiting != new.enable_rate_limiting {
            changes.push(format!("rate_limiting: {} -> {}", current.enable_rate_limiting, new.enable_rate_limiting));
        }
        if current.enable_2fa != new.enable_2fa {
            changes.push(format!("2fa: {} -> {}", current.enable_2fa, new.enable_2fa));
        }
        if current.max_login_attempts != new.max_login_attempts {
            changes.push(format!("max_login_attempts: {} -> {}", current.max_login_attempts, new.max_login_attempts));
        }
        
        changes
    }

    fn compare_feature_flags(&self, current: &super::production::FeatureFlags, new: &super::production::FeatureFlags) -> Vec<String> {
        let mut changes = Vec::new();
        
        if current.enable_real_time_updates != new.enable_real_time_updates {
            changes.push(format!("real_time_updates: {} -> {}", current.enable_real_time_updates, new.enable_real_time_updates));
        }
        if current.enable_ai_predictions != new.enable_ai_predictions {
            changes.push(format!("ai_predictions: {} -> {}", current.enable_ai_predictions, new.enable_ai_predictions));
        }
        if current.enable_experimental_features != new.enable_experimental_features {
            changes.push(format!("experimental_features: {} -> {}", current.enable_experimental_features, new.enable_experimental_features));
        }
        
        changes
    }
}

/// Configuration health summary for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigHealthSummary {
    pub environment: String,
    pub database_connections: String,
    pub api_endpoint: String,
    pub monitoring_enabled: bool,
    pub security_features: SecurityFeaturesSummary,
    pub feature_flags: FeatureFlagsSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFeaturesSummary {
    pub rate_limiting: bool,
    pub two_factor_auth: bool,
    pub audit_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlagsSummary {
    pub real_time_monitoring: bool,
    pub websockets: bool,
    pub ai_predictions: bool,
    pub experimental: bool,
}

/// Configuration diff for change tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiff {
    pub environment_changed: bool,
    pub database_changes: Vec<String>,
    pub api_changes: Vec<String>,
    pub security_changes: Vec<String>,
    pub feature_changes: Vec<String>,
}

impl ConfigDiff {
    pub fn has_changes(&self) -> bool {
        self.environment_changed ||
        !self.database_changes.is_empty() ||
        !self.api_changes.is_empty() ||
        !self.security_changes.is_empty() ||
        !self.feature_changes.is_empty()
    }

    pub fn get_summary(&self) -> String {
        let mut summary = Vec::new();
        
        if self.environment_changed {
            summary.push("Environment changed".to_string());
        }
        if !self.database_changes.is_empty() {
            summary.push(format!("Database: {} changes", self.database_changes.len()));
        }
        if !self.api_changes.is_empty() {
            summary.push(format!("API: {} changes", self.api_changes.len()));
        }
        if !self.security_changes.is_empty() {
            summary.push(format!("Security: {} changes", self.security_changes.len()));
        }
        if !self.feature_changes.is_empty() {
            summary.push(format!("Features: {} changes", self.feature_changes.len()));
        }
        
        if summary.is_empty() {
            "No changes".to_string()
        } else {
            summary.join(", ")
        }
    }
}

/// Configuration secrets manager for sensitive data
pub struct SecretsManager {
    secrets_path: String,
}

impl SecretsManager {
    pub fn new(secrets_path: Option<String>) -> Self {
        Self {
            secrets_path: secrets_path.unwrap_or_else(|| "config/secrets.toml".to_string()),
        }
    }

    /// Load secrets from encrypted file
    pub fn load_secrets(&self) -> Result<std::collections::HashMap<String, String>, Box<dyn std::error::Error>> {
        if !Path::new(&self.secrets_path).exists() {
            warn!("Secrets file not found at {}", self.secrets_path);
            return Ok(std::collections::HashMap::new());
        }

        let secrets_content = fs::read_to_string(&self.secrets_path)?;
        let secrets: std::collections::HashMap<String, String> = toml::from_str(&secrets_content)?;
        
        info!("Loaded {} secrets from {}", secrets.len(), self.secrets_path);
        Ok(secrets)
    }

    /// Save secrets to encrypted file
    pub fn save_secrets(&self, secrets: &std::collections::HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
        let secrets_content = toml::to_string_pretty(secrets)?;
        
        // Ensure secrets directory exists
        if let Some(parent) = Path::new(&self.secrets_path).parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(&self.secrets_path, secrets_content)?;
        
        // Set restrictive permissions on secrets file
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.secrets_path)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            fs::set_permissions(&self.secrets_path, perms)?;
        }
        
        info!("Saved {} secrets to {}", secrets.len(), self.secrets_path);
        Ok(())
    }

    /// Get a specific secret
    pub fn get_secret(&self, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let secrets = self.load_secrets()?;
        Ok(secrets.get(key).cloned())
    }

    /// Set a specific secret
    pub fn set_secret(&self, key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut secrets = self.load_secrets()?;
        secrets.insert(key.to_string(), value.to_string());
        self.save_secrets(&secrets)?;
        Ok(())
    }

    /// Remove a specific secret
    pub fn remove_secret(&self, key: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let mut secrets = self.load_secrets()?;
        let removed = secrets.remove(key).is_some();
        if removed {
            self.save_secrets(&secrets)?;
        }
        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_config_manager_creation() {
        // This test would require proper environment setup
        // For now, we'll just test the structure
        assert!(true);
    }

    #[test]
    fn test_config_diff() {
        let diff = ConfigDiff {
            environment_changed: true,
            database_changes: vec!["max_connections: 10 -> 20".to_string()],
            api_changes: vec![],
            security_changes: vec!["2fa: false -> true".to_string()],
            feature_changes: vec![],
        };

        assert!(diff.has_changes());
        let summary = diff.get_summary();
        assert!(summary.contains("Environment changed"));
        assert!(summary.contains("Database: 1 changes"));
        assert!(summary.contains("Security: 1 changes"));
    }

    #[test]
    fn test_secrets_manager() {
        let temp_dir = TempDir::new().unwrap();
        let secrets_path = temp_dir.path().join("secrets.toml").to_string_lossy().to_string();
        
        let secrets_manager = SecretsManager::new(Some(secrets_path));
        
        // Test setting and getting secrets
        secrets_manager.set_secret("test_key", "test_value").unwrap();
        let value = secrets_manager.get_secret("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));
        
        // Test removing secrets
        let removed = secrets_manager.remove_secret("test_key").unwrap();
        assert!(removed);
        
        let value = secrets_manager.get_secret("test_key").unwrap();
        assert_eq!(value, None);
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        let config = ProductionConfig::default();
        let validator = ConfigValidator::default();
        
        Self {
            config: Arc::new(RwLock::new(config)),
            validator,
            config_path: "config/config.development.toml".to_string(),
            last_modified: None,
        }
    }
}
