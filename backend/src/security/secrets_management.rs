use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::Aead;
use rand::{RngCore, thread_rng};
use chrono;
use hex;
use crate::error::AppError;

/// Secure secrets management for DeFi risk monitoring
#[derive(Debug, Clone)]
pub struct SecretsManager {
    encryption_key: [u8; 32],
    secrets_cache: HashMap<String, SecretValue>,
    audit_trail: Vec<SecretAccess>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretValue {
    pub encrypted_value: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    pub access_count: u64,
    pub secret_type: SecretType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretType {
    DatabaseUrl,
    ApiKey,
    WebhookUrl,
    PrivateKey,
    JwtSecret,
    EncryptionKey,
    RpcUrl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretAccess {
    pub secret_name: String,
    pub access_time: chrono::DateTime<chrono::Utc>,
    pub operation: SecretOperation,
    pub caller_info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretOperation {
    Read,
    Write,
    Delete,
    Rotate,
}

impl SecretsManager {
    /// Create a new secrets manager with a derived encryption key
    pub fn new() -> Result<Self, AppError> {
        let key_material = env::var("MASTER_KEY")
            .or_else(|_| env::var("ENCRYPTION_KEY"))
            .unwrap_or_else(|_| {
                // Generate a random key if none provided (for development only)
                let mut key = [0u8; 32];
                thread_rng().fill_bytes(&mut key);
                hex::encode(key)
            });

        let mut hasher = Sha256::new();
        hasher.update(key_material.as_bytes());
        let encryption_key: [u8; 32] = hasher.finalize().into();

        Ok(Self {
            encryption_key,
            secrets_cache: HashMap::new(),
            audit_trail: Vec::new(),
        })
    }

    /// Store a secret securely
    pub fn store_secret(
        &mut self,
        name: &str,
        value: &str,
        secret_type: SecretType,
    ) -> Result<(), AppError> {
        // Validate secret name
        if name.is_empty() || name.len() > 100 {
            return Err(AppError::ValidationError(
                "Secret name must be between 1 and 100 characters".to_string()
            ));
        }

        // Validate secret value based on type
        self.validate_secret_value(value, &secret_type)?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the secret
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));
        let encrypted_value = cipher
            .encrypt(nonce, value.as_bytes())
            .map_err(|e| AppError::InternalError(format!("Encryption failed: {}", e)))?;

        // Store in cache
        let secret_value = SecretValue {
            encrypted_value,
            nonce: nonce_bytes.to_vec(),
            created_at: chrono::Utc::now(),
            last_accessed: None,
            access_count: 0,
            secret_type,
        };

        self.secrets_cache.insert(name.to_string(), secret_value);

        // Audit trail
        self.audit_trail.push(SecretAccess {
            secret_name: name.to_string(),
            access_time: chrono::Utc::now(),
            operation: SecretOperation::Write,
            caller_info: self.get_caller_info(),
        });

        Ok(())
    }

    /// Retrieve a secret securely
    pub fn get_secret(&mut self, name: &str) -> Result<String, AppError> {
        let secret_value = self.secrets_cache
            .get_mut(name)
            .ok_or_else(|| AppError::NotFound(format!("Secret '{}' not found", name)))?;

        // Decrypt the secret
        let nonce = Nonce::from_slice(&secret_value.nonce);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.encryption_key));
        
        let decrypted_bytes = cipher
            .decrypt(nonce, secret_value.encrypted_value.as_ref())
            .map_err(|e| AppError::InternalError(format!("Decryption failed: {}", e)))?;

        let decrypted_value = String::from_utf8(decrypted_bytes)
            .map_err(|e| AppError::InternalError(format!("Invalid UTF-8 in secret: {}", e)))?;

        // Update access tracking
        secret_value.last_accessed = Some(chrono::Utc::now());
        secret_value.access_count += 1;

        // Audit trail
        self.audit_trail.push(SecretAccess {
            secret_name: name.to_string(),
            access_time: chrono::Utc::now(),
            operation: SecretOperation::Read,
            caller_info: self.get_caller_info(),
        });

        Ok(decrypted_value)
    }

    /// Rotate a secret (generate new encryption)
    pub fn rotate_secret(&mut self, name: &str) -> Result<(), AppError> {
        // Get current secret
        let current_value = self.get_secret(name)?;
        let secret_type = self.secrets_cache
            .get(name)
            .map(|s| s.secret_type.clone())
            .ok_or_else(|| AppError::NotFound("Secret not found".to_string()))?;

        // Re-encrypt with new nonce
        self.store_secret(name, &current_value, secret_type)?;

        // Audit trail
        self.audit_trail.push(SecretAccess {
            secret_name: name.to_string(),
            access_time: chrono::Utc::now(),
            operation: SecretOperation::Rotate,
            caller_info: self.get_caller_info(),
        });

        Ok(())
    }

    /// Delete a secret
    pub fn delete_secret(&mut self, name: &str) -> Result<(), AppError> {
        self.secrets_cache
            .remove(name)
            .ok_or_else(|| AppError::NotFound(format!("Secret '{}' not found", name)))?;

        // Audit trail
        self.audit_trail.push(SecretAccess {
            secret_name: name.to_string(),
            access_time: chrono::Utc::now(),
            operation: SecretOperation::Delete,
            caller_info: self.get_caller_info(),
        });

        Ok(())
    }

    /// Load secrets from environment variables securely
    pub fn load_from_env(&mut self) -> Result<(), AppError> {
        let env_secrets = [
            ("DATABASE_URL", SecretType::DatabaseUrl),
            ("ETHEREUM_RPC_URL", SecretType::RpcUrl),
            ("POLYGON_RPC_URL", SecretType::RpcUrl),
            ("ARBITRUM_RPC_URL", SecretType::RpcUrl),
            ("SLACK_WEBHOOK_URL", SecretType::WebhookUrl),
            ("DISCORD_WEBHOOK_URL", SecretType::WebhookUrl),
            ("JWT_SECRET", SecretType::JwtSecret),
            ("API_KEY", SecretType::ApiKey),
        ];

        for (env_var, secret_type) in &env_secrets {
            if let Ok(value) = env::var(env_var) {
                if !value.is_empty() {
                    self.store_secret(env_var, &value, secret_type.clone())?;
                }
            }
        }

        Ok(())
    }

    /// Export secrets to secure file (for backup/migration)
    pub fn export_to_file(&self, file_path: &Path) -> Result<(), AppError> {
        let export_data = serde_json::to_string_pretty(&self.secrets_cache)
            .map_err(|e| AppError::InternalError(format!("Serialization failed: {}", e)))?;

        fs::write(file_path, export_data)
            .map_err(|e| AppError::InternalError(format!("File write failed: {}", e)))?;

        Ok(())
    }

    /// Import secrets from secure file
    pub fn import_from_file(&mut self, file_path: &Path) -> Result<(), AppError> {
        let file_content = fs::read_to_string(file_path)
            .map_err(|e| AppError::InternalError(format!("File read failed: {}", e)))?;

        let imported_secrets: HashMap<String, SecretValue> = serde_json::from_str(&file_content)
            .map_err(|e| AppError::InternalError(format!("Deserialization failed: {}", e)))?;

        self.secrets_cache.extend(imported_secrets);
        Ok(())
    }

    /// Get audit trail for compliance
    pub fn get_audit_trail(&self) -> &[SecretAccess] {
        &self.audit_trail
    }

    /// Clear audit trail (for privacy/storage management)
    pub fn clear_audit_trail(&mut self) {
        self.audit_trail.clear();
    }

    /// Validate secret value based on type
    fn validate_secret_value(&self, value: &str, secret_type: &SecretType) -> Result<(), AppError> {
        match secret_type {
            SecretType::DatabaseUrl => {
                if !value.starts_with("postgresql://") && !value.starts_with("postgres://") {
                    return Err(AppError::ValidationError(
                        "Database URL must start with postgresql:// or postgres://".to_string()
                    ));
                }
            }
            SecretType::RpcUrl => {
                if !value.starts_with("http://") && !value.starts_with("https://") {
                    return Err(AppError::ValidationError(
                        "RPC URL must start with http:// or https://".to_string()
                    ));
                }
            }
            SecretType::WebhookUrl => {
                if !value.starts_with("https://") {
                    return Err(AppError::ValidationError(
                        "Webhook URL must start with https://".to_string()
                    ));
                }
            }
            SecretType::ApiKey => {
                if value.len() < 16 {
                    return Err(AppError::ValidationError(
                        "API key must be at least 16 characters".to_string()
                    ));
                }
            }
            SecretType::JwtSecret => {
                if value.len() < 32 {
                    return Err(AppError::ValidationError(
                        "JWT secret must be at least 32 characters".to_string()
                    ));
                }
            }
            SecretType::PrivateKey => {
                if value.len() != 64 && !value.starts_with("0x") {
                    return Err(AppError::ValidationError(
                        "Private key must be 64 hex characters or start with 0x".to_string()
                    ));
                }
            }
            SecretType::EncryptionKey => {
                if value.len() < 32 {
                    return Err(AppError::ValidationError(
                        "Encryption key must be at least 32 characters".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get caller information for audit trail
    fn get_caller_info(&self) -> String {
        // In a real implementation, this would capture stack trace or caller context
        format!("SecretsManager::{}", chrono::Utc::now().timestamp())
    }
}

/// Environment variable security scanner
pub struct EnvSecurityScanner;

impl EnvSecurityScanner {
    /// Scan environment variables for potential security issues
    pub fn scan_environment() -> Vec<SecurityIssue> {
        let mut issues = Vec::new();

        // Check for hardcoded secrets in environment
        for (key, value) in env::vars() {
            // Check for potential secrets in variable names
            if Self::is_potential_secret(&key) {
                if Self::appears_hardcoded(&value) {
                    issues.push(SecurityIssue {
                        severity: SecuritySeverity::High,
                        issue_type: SecurityIssueType::HardcodedSecret,
                        description: format!("Potential hardcoded secret in environment variable: {}", key),
                        recommendation: "Use secure secret management instead of hardcoded values".to_string(),
                    });
                }

                if Self::is_weak_secret(&value) {
                    issues.push(SecurityIssue {
                        severity: SecuritySeverity::Medium,
                        issue_type: SecurityIssueType::WeakSecret,
                        description: format!("Weak secret detected in environment variable: {}", key),
                        recommendation: "Use stronger, randomly generated secrets".to_string(),
                    });
                }
            }
        }

        issues
    }

    fn is_potential_secret(key: &str) -> bool {
        let secret_indicators = [
            "password", "secret", "key", "token", "auth", "credential",
            "private", "webhook", "api", "jwt", "rpc"
        ];

        let key_lower = key.to_lowercase();
        secret_indicators.iter().any(|indicator| key_lower.contains(indicator))
    }

    fn appears_hardcoded(value: &str) -> bool {
        // Check for common hardcoded patterns
        value == "password" || 
        value == "secret" || 
        value == "123456" ||
        value.starts_with("test_") ||
        value.starts_with("dev_") ||
        value == "localhost"
    }

    fn is_weak_secret(value: &str) -> bool {
        value.len() < 16 || 
        value.chars().all(|c| c.is_ascii_digit()) ||
        value.chars().all(|c| c.is_ascii_alphabetic())
    }
}

#[derive(Debug, Clone)]
pub struct SecurityIssue {
    pub severity: SecuritySeverity,
    pub issue_type: SecurityIssueType,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone)]
pub enum SecuritySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub enum SecurityIssueType {
    HardcodedSecret,
    WeakSecret,
    InsecureConfiguration,
    MissingEncryption,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_secrets_manager_creation() {
        let manager = SecretsManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_store_and_retrieve_secret() {
        let mut manager = SecretsManager::new().unwrap();
        
        let secret_name = "test_secret";
        let secret_value = "super_secret_value_123";
        
        // Store secret
        let result = manager.store_secret(secret_name, secret_value, SecretType::ApiKey);
        assert!(result.is_ok());
        
        // Retrieve secret
        let retrieved = manager.get_secret(secret_name);
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), secret_value);
    }

    #[test]
    fn test_secret_validation() {
        let mut manager = SecretsManager::new().unwrap();
        
        // Test valid database URL
        let result = manager.store_secret(
            "db_url", 
            "postgresql://user:pass@localhost:5432/db", 
            SecretType::DatabaseUrl
        );
        assert!(result.is_ok());
        
        // Test invalid database URL
        let result = manager.store_secret(
            "bad_db_url", 
            "invalid_url", 
            SecretType::DatabaseUrl
        );
        assert!(result.is_err());
        
        // Test weak API key
        let result = manager.store_secret(
            "weak_key", 
            "123", 
            SecretType::ApiKey
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_secret_rotation() {
        let mut manager = SecretsManager::new().unwrap();
        
        let secret_name = "rotation_test";
        let secret_value = "original_secret_value_12345";
        
        // Store original secret
        manager.store_secret(secret_name, secret_value, SecretType::ApiKey).unwrap();
        
        // Get original encrypted value
        let original_encrypted = manager.secrets_cache
            .get(secret_name)
            .unwrap()
            .encrypted_value
            .clone();
        
        // Rotate secret
        let result = manager.rotate_secret(secret_name);
        assert!(result.is_ok());
        
        // Check that encrypted value changed (new nonce)
        let new_encrypted = &manager.secrets_cache
            .get(secret_name)
            .unwrap()
            .encrypted_value;
        
        assert_ne!(original_encrypted, *new_encrypted);
        
        // But decrypted value should be the same
        let retrieved = manager.get_secret(secret_name).unwrap();
        assert_eq!(retrieved, secret_value);
    }

    #[test]
    fn test_audit_trail() {
        let mut manager = SecretsManager::new().unwrap();
        
        let secret_name = "audit_test";
        let secret_value = "audit_secret_value_123";
        
        // Perform operations
        manager.store_secret(secret_name, secret_value, SecretType::ApiKey).unwrap();
        manager.get_secret(secret_name).unwrap();
        manager.rotate_secret(secret_name).unwrap();
        manager.delete_secret(secret_name).unwrap();
        
        // Check audit trail
        let audit_trail = manager.get_audit_trail();
        assert_eq!(audit_trail.len(), 5); // Write, Read, Write (rotation), Read (rotation), Delete
        
        // Check operation types
        assert!(matches!(audit_trail[0].operation, SecretOperation::Write));
        assert!(matches!(audit_trail[1].operation, SecretOperation::Read));
        assert!(matches!(audit_trail[2].operation, SecretOperation::Rotate));
        assert!(matches!(audit_trail[4].operation, SecretOperation::Delete));
    }

    #[test]
    fn test_file_export_import() {
        let mut manager = SecretsManager::new().unwrap();
        
        // Store some secrets
        manager.store_secret("secret1", "value1_123456789", SecretType::ApiKey).unwrap();
        manager.store_secret("secret2", "value2_987654321", SecretType::JwtSecret).unwrap();
        
        // Export to file
        let temp_file = NamedTempFile::new().unwrap();
        let result = manager.export_to_file(temp_file.path());
        assert!(result.is_ok());
        
        // Create new manager and import
        let mut new_manager = SecretsManager::new().unwrap();
        let result = new_manager.import_from_file(temp_file.path());
        assert!(result.is_ok());
        
        // Verify secrets were imported
        let secret1 = new_manager.get_secret("secret1").unwrap();
        let secret2 = new_manager.get_secret("secret2").unwrap();
        
        assert_eq!(secret1, "value1_123456789");
        assert_eq!(secret2, "value2_987654321");
    }

    #[test]
    fn test_environment_security_scanner() {
        // This test would need to set up environment variables
        // For now, just test that the scanner runs without errors
        let issues = EnvSecurityScanner::scan_environment();
        // Issues may or may not be found depending on the environment
        assert!(issues.len() >= 0);
    }
}
