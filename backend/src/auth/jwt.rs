//! JWT Token Management Service

use crate::auth::claims::{Claims, TokenValidation, UserRole};
use crate::error::AppError;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// JWT Service for token creation and validation
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
    revoked_tokens: Arc<RwLock<HashSet<String>>>,
}

/// JWT Configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    pub secret: String,
    pub algorithm: Algorithm,
    pub expires_in_hours: i64,
    pub issuer: String,
    pub audience: String,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string()),
            algorithm: Algorithm::HS256,
            expires_in_hours: 24,
            issuer: "defi-risk-monitor".to_string(),
            audience: "defi-risk-monitor-api".to_string(),
        }
    }
}

/// Login request structure
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response structure
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub expires_at: i64,
    pub user: UserInfo,
}

/// User information for response
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub role: UserRole,
}

impl JwtService {
    /// Create new JWT service with configuration
    pub fn new(config: JwtConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        Self {
            encoding_key,
            decoding_key,
            algorithm: config.algorithm,
            revoked_tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Generate JWT token for user
    pub async fn generate_token(
        &self,
        user_id: Uuid,
        username: String,
        role: UserRole,
        expires_in_hours: Option<i64>,
    ) -> Result<String, AppError> {
        let expires_in = expires_in_hours.unwrap_or(24);
        let claims = Claims::new(user_id, username, role, expires_in);

        let header = Header::new(self.algorithm);
        
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| AppError::AuthenticationError(format!("Failed to generate token: {}", e)))
    }

    /// Validate JWT token
    pub async fn validate_token(&self, token: &str) -> Result<TokenValidation, AppError> {
        // Check if token is revoked
        let revoked_tokens = self.revoked_tokens.read().await;
        if revoked_tokens.contains(token) {
            return Ok(TokenValidation::Revoked);
        }
        drop(revoked_tokens);

        // Decode and validate token
        let mut validation = Validation::new(self.algorithm);
        validation.set_issuer(&["defi-risk-monitor"]);
        validation.set_audience(&["defi-risk-monitor-api"]);

        match decode::<Claims>(token, &self.decoding_key, &validation) {
            Ok(token_data) => {
                let claims = token_data.claims;
                
                if claims.is_expired() {
                    Ok(TokenValidation::Expired)
                } else {
                    Ok(TokenValidation::Valid(claims))
                }
            }
            Err(e) => Ok(TokenValidation::Invalid(e.to_string())),
        }
    }

    /// Revoke a token (add to blacklist)
    pub async fn revoke_token(&self, token: &str) -> Result<(), AppError> {
        let mut revoked_tokens = self.revoked_tokens.write().await;
        revoked_tokens.insert(token.to_string());
        Ok(())
    }

    /// Create login response
    pub fn create_login_response(
        &self,
        token: String,
        claims: &Claims,
    ) -> LoginResponse {
        LoginResponse {
            token,
            expires_at: claims.exp,
            user: UserInfo {
                id: claims.sub.clone(),
                username: claims.username.clone(),
                role: claims.role.clone(),
            },
        }
    }

    /// Extract token from Authorization header
    pub fn extract_token_from_header(auth_header: &str) -> Option<&str> {
        if auth_header.starts_with("Bearer ") {
            Some(&auth_header[7..])
        } else {
            None
        }
    }

    /// Clean up expired revoked tokens (should be called periodically)
    pub async fn cleanup_revoked_tokens(&self) {
        let mut revoked_tokens = self.revoked_tokens.write().await;
        // In a real implementation, you'd decode each token and check expiration
        // For now, we'll just clear all (tokens expire naturally anyway)
        revoked_tokens.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_jwt_token_generation_and_validation() {
        let config = JwtConfig::default();
        let jwt_service = JwtService::new(config);
        
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Operator;

        // Generate token
        let token = jwt_service
            .generate_token(user_id, username.clone(), role.clone(), Some(1))
            .await
            .expect("Failed to generate token");

        // Validate token
        let validation_result = jwt_service
            .validate_token(&token)
            .await
            .expect("Failed to validate token");

        match validation_result {
            TokenValidation::Valid(claims) => {
                assert_eq!(claims.username, username);
                assert_eq!(claims.role, role);
                assert_eq!(claims.user_id().unwrap(), user_id);
            }
            _ => panic!("Token should be valid"),
        }
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let config = JwtConfig::default();
        let jwt_service = JwtService::new(config);
        
        let user_id = Uuid::new_v4();
        let token = jwt_service
            .generate_token(user_id, "test".to_string(), UserRole::Viewer, Some(1))
            .await
            .expect("Failed to generate token");

        // Token should be valid initially
        let result = jwt_service.validate_token(&token).await.unwrap();
        assert!(matches!(result, TokenValidation::Valid(_)));

        // Revoke token
        jwt_service.revoke_token(&token).await.unwrap();

        // Token should now be revoked
        let result = jwt_service.validate_token(&token).await.unwrap();
        assert!(matches!(result, TokenValidation::Revoked));
    }

    #[test]
    fn test_extract_token_from_header() {
        let header = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let token = JwtService::extract_token_from_header(header);
        assert_eq!(token, Some("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));

        let invalid_header = "InvalidHeader";
        let token = JwtService::extract_token_from_header(invalid_header);
        assert_eq!(token, None);
    }
}
