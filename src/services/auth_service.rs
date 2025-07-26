use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use crate::error::AppError;

/// User roles for role-based access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Operator,
    Viewer,
    ApiUser,
    System,
}

impl UserRole {
    /// Check if role has permission for a specific action
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self {
            UserRole::Admin => true, // Admin has all permissions
            UserRole::Operator => matches!(permission, 
                Permission::ReadPositions | Permission::ReadAlerts | Permission::ReadMetrics |
                Permission::CreateAlerts | Permission::UpdateAlerts | Permission::ReadAuditLogs
            ),
            UserRole::Viewer => matches!(permission, 
                Permission::ReadPositions | Permission::ReadAlerts | Permission::ReadMetrics
            ),
            UserRole::ApiUser => matches!(permission,
                Permission::ReadPositions | Permission::CreatePositions | Permission::UpdatePositions |
                Permission::ReadAlerts | Permission::CreateAlerts
            ),
            UserRole::System => matches!(permission,
                Permission::ReadPositions | Permission::CreatePositions | Permission::UpdatePositions |
                Permission::ReadAlerts | Permission::CreateAlerts | Permission::UpdateAlerts |
                Permission::ReadMetrics | Permission::WriteMetrics | Permission::ReadAuditLogs |
                Permission::WriteAuditLogs
            ),
        }
    }
}

/// Granular permissions for fine-grained access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    // Position management
    ReadPositions,
    CreatePositions,
    UpdatePositions,
    DeletePositions,
    
    // Alert management
    ReadAlerts,
    CreateAlerts,
    UpdateAlerts,
    DeleteAlerts,
    
    // Risk management
    ReadRiskData,
    CalculateRisk,
    
    // Metrics and monitoring
    ReadMetrics,
    WriteMetrics,
    
    // Audit and compliance
    ReadAuditLogs,
    WriteAuditLogs,
    GenerateReports,
    
    // System administration
    ManageUsers,
    ManageConfig,
    SystemControl,
}

/// User authentication and profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
    pub is_active: bool,
    pub api_key_hash: Option<String>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// JWT token claims
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,        // Subject (user ID)
    pub username: String,
    pub role: UserRole,
    pub exp: usize,         // Expiration time
    pub iat: usize,         // Issued at
    pub jti: String,        // JWT ID
}

/// Authentication request
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

/// API key authentication request
#[derive(Debug, Deserialize)]
pub struct ApiKeyRequest {
    pub api_key: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
    pub user: UserInfo,
}

/// User information for responses
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: UserRole,
}

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub burst_limit: u32,
    pub window_size: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            burst_limit: 10,
            window_size: Duration::from_secs(60),
        }
    }
}

impl RateLimitConfig {
    /// Configuration for different user roles
    pub fn for_role(role: &UserRole) -> Self {
        match role {
            UserRole::Admin => Self {
                requests_per_minute: 1000,
                requests_per_hour: 10000,
                burst_limit: 100,
                window_size: Duration::from_secs(60),
            },
            UserRole::Operator => Self {
                requests_per_minute: 300,
                requests_per_hour: 3000,
                burst_limit: 50,
                window_size: Duration::from_secs(60),
            },
            UserRole::ApiUser => Self {
                requests_per_minute: 100,
                requests_per_hour: 1500,
                burst_limit: 20,
                window_size: Duration::from_secs(60),
            },
            UserRole::System => Self {
                requests_per_minute: 500,
                requests_per_hour: 5000,
                burst_limit: 100,
                window_size: Duration::from_secs(60),
            },
            UserRole::Viewer => Self::default(),
        }
    }
}

/// Rate limiting tracker
#[derive(Debug, Clone)]
pub struct RateLimitTracker {
    pub user_id: String,
    pub requests_count: u32,
    pub window_start: SystemTime,
    pub last_request: SystemTime,
}

/// Authentication and authorization service
pub struct AuthService {
    db_pool: PgPool,
    jwt_secret: String,
    rate_limits: HashMap<String, RateLimitTracker>,
    token_expiry: Duration,
}

impl AuthService {
    pub fn new(db_pool: PgPool, jwt_secret: String) -> Self {
        Self {
            db_pool,
            jwt_secret,
            rate_limits: HashMap::new(),
            token_expiry: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Authenticate user with username/password
    pub async fn authenticate_user(&self, auth_request: AuthRequest) -> Result<AuthResponse, AppError> {
        // Hash the provided password
        let password_hash = self.hash_password(&auth_request.password);

        // Query user from database
        let user_row = sqlx::query!(
            "SELECT id, username, email, role as \"role: UserRole\", is_active, password_hash, last_login 
             FROM users WHERE username = $1 AND password_hash = $2",
            auth_request.username,
            password_hash
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to query user: {}", e)))?;

        let user_row = user_row.ok_or_else(|| AppError::AuthenticationError("Invalid credentials".to_string()))?;

        if !user_row.is_active {
            return Err(AppError::AuthenticationError("Account is inactive".to_string()));
        }

        // Update last login
        sqlx::query!(
            "UPDATE users SET last_login = NOW() WHERE id = $1",
            user_row.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update last login: {}", e)))?;

        // Generate JWT token
        let token = self.generate_token(&user_row.id.to_string(), &user_row.username, &user_row.role)?;

        info!("User {} authenticated successfully", user_row.username);

        Ok(AuthResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.token_expiry.as_secs() as usize,
            user: UserInfo {
                id: user_row.id,
                username: user_row.username,
                email: user_row.email,
                role: user_row.role,
            },
        })
    }

    /// Authenticate user with API key
    pub async fn authenticate_api_key(&self, api_key_request: ApiKeyRequest) -> Result<User, AppError> {
        let api_key_hash = self.hash_api_key(&api_key_request.api_key);

        let user_row = sqlx::query!(
            "SELECT id, username, email, role as \"role: UserRole\", is_active, last_login, created_at, updated_at
             FROM users WHERE api_key_hash = $1",
            api_key_hash
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to query user by API key: {}", e)))?;

        let user_row = user_row.ok_or_else(|| AppError::AuthenticationError("Invalid API key".to_string()))?;

        if !user_row.is_active {
            return Err(AppError::AuthenticationError("Account is inactive".to_string()));
        }

        // Update last login for API key usage
        sqlx::query!(
            "UPDATE users SET last_login = NOW() WHERE id = $1",
            user_row.id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update last login: {}", e)))?;

        info!("API key authentication successful for user {}", user_row.username);

        Ok(User {
            id: user_row.id,
            username: user_row.username,
            email: user_row.email,
            role: user_row.role,
            is_active: user_row.is_active,
            api_key_hash: Some(api_key_hash),
            last_login: user_row.last_login,
            created_at: user_row.created_at,
            updated_at: user_row.updated_at,
        })
    }

    /// Validate JWT token
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, AppError> {
        let decoding_key = DecodingKey::from_secret(self.jwt_secret.as_ref());
        let validation = Validation::new(Algorithm::HS256);

        let token_data = decode::<TokenClaims>(token, &decoding_key, &validation)
            .map_err(|e| AppError::AuthenticationError(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }

    /// Check if user has permission for a specific action
    pub fn check_permission(&self, user_role: &UserRole, permission: &Permission) -> Result<(), AppError> {
        if user_role.has_permission(permission) {
            Ok(())
        } else {
            Err(AppError::AuthorizationError(format!(
                "Insufficient permissions: {:?} required for {:?}",
                permission, user_role
            )))
        }
    }

    /// Rate limiting check
    pub fn check_rate_limit(&mut self, user_id: &str, role: &UserRole) -> Result<(), AppError> {
        let config = RateLimitConfig::for_role(role);
        let now = SystemTime::now();

        let tracker = self.rate_limits.entry(user_id.to_string()).or_insert_with(|| {
            RateLimitTracker {
                user_id: user_id.to_string(),
                requests_count: 0,
                window_start: now,
                last_request: now,
            }
        });

        // Check if we need to reset the window
        if now.duration_since(tracker.window_start).unwrap_or(Duration::ZERO) >= config.window_size {
            tracker.requests_count = 0;
            tracker.window_start = now;
        }

        // Check rate limits
        if tracker.requests_count >= config.requests_per_minute {
            warn!("Rate limit exceeded for user {}: {} requests in window", user_id, tracker.requests_count);
            return Err(AppError::RateLimitError(format!(
                "Rate limit exceeded: {} requests per minute allowed",
                config.requests_per_minute
            )));
        }

        // Check burst limit
        let time_since_last = now.duration_since(tracker.last_request).unwrap_or(Duration::ZERO);
        if time_since_last < Duration::from_secs(1) && tracker.requests_count >= config.burst_limit {
            warn!("Burst limit exceeded for user {}", user_id);
            return Err(AppError::RateLimitError("Burst limit exceeded".to_string()));
        }

        // Update tracker
        tracker.requests_count += 1;
        tracker.last_request = now;

        Ok(())
    }

    /// Generate JWT token
    fn generate_token(&self, user_id: &str, username: &str, role: &UserRole) -> Result<String, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;

        let claims = TokenClaims {
            sub: user_id.to_string(),
            username: username.to_string(),
            role: role.clone(),
            exp: now + self.token_expiry.as_secs() as usize,
            iat: now,
            jti: Uuid::new_v4().to_string(),
        };

        let encoding_key = EncodingKey::from_secret(self.jwt_secret.as_ref());
        let token = encode(&Header::default(), &claims, &encoding_key)
            .map_err(|e| AppError::InternalError(format!("Failed to generate token: {}", e)))?;

        Ok(token)
    }

    /// Hash password using SHA-256 (in production, use bcrypt or Argon2)
    fn hash_password(&self, password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(self.jwt_secret.as_bytes()); // Use JWT secret as salt
        format!("{:x}", hasher.finalize())
    }

    /// Hash API key
    fn hash_api_key(&self, api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        hasher.update(self.jwt_secret.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Create new user (admin function)
    pub async fn create_user(
        &self,
        username: String,
        email: String,
        password: String,
        role: UserRole,
    ) -> Result<User, AppError> {
        let user_id = Uuid::new_v4();
        let password_hash = self.hash_password(&password);
        let now = chrono::Utc::now();

        sqlx::query!(
            "INSERT INTO users (id, username, email, password_hash, role, is_active, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, true, $6, $7)",
            user_id,
            username,
            email,
            password_hash,
            role.clone() as UserRole,
            now,
            now
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create user: {}", e)))?;

        info!("Created new user: {} with role {:?}", username, role.clone());

        Ok(User {
            id: user_id,
            username,
            email,
            role,
            is_active: true,
            api_key_hash: None,
            last_login: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Generate API key for user
    pub async fn generate_api_key(&self, user_id: Uuid) -> Result<String, AppError> {
        let api_key = format!("drm_{}", Uuid::new_v4().simple());
        let api_key_hash = self.hash_api_key(&api_key);

        sqlx::query!(
            "UPDATE users SET api_key_hash = $1, updated_at = NOW() WHERE id = $2",
            api_key_hash,
            user_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to update API key: {}", e)))?;

        info!("Generated new API key for user {}", user_id);
        Ok(api_key)
    }

    /// Get rate limit statistics
    pub fn get_rate_limit_stats(&self) -> HashMap<String, serde_json::Value> {
        let mut stats = HashMap::new();
        
        for (user_id, tracker) in &self.rate_limits {
            stats.insert(user_id.clone(), serde_json::json!({
                "requests_count": tracker.requests_count,
                "window_start": tracker.window_start.duration_since(UNIX_EPOCH).unwrap().as_secs(),
                "last_request": tracker.last_request.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            }));
        }
        
        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        assert!(UserRole::Admin.has_permission(&Permission::ManageUsers));
        assert!(UserRole::Operator.has_permission(&Permission::ReadPositions));
        assert!(!UserRole::Viewer.has_permission(&Permission::CreatePositions));
        assert!(UserRole::ApiUser.has_permission(&Permission::CreatePositions));
    }

    #[test]
    fn test_rate_limit_config() {
        let admin_config = RateLimitConfig::for_role(&UserRole::Admin);
        let viewer_config = RateLimitConfig::for_role(&UserRole::Viewer);
        
        assert!(admin_config.requests_per_minute > viewer_config.requests_per_minute);
        assert!(admin_config.burst_limit > viewer_config.burst_limit);
    }

    #[test]
    fn test_token_claims_serialization() {
        let claims = TokenClaims {
            sub: "user123".to_string(),
            username: "testuser".to_string(),
            role: UserRole::Operator,
            exp: 1234567890,
            iat: 1234567800,
            jti: "jwt123".to_string(),
        };

        let serialized = serde_json::to_string(&claims).unwrap();
        let deserialized: TokenClaims = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(claims.sub, deserialized.sub);
        assert_eq!(claims.role, deserialized.role);
    }
}
