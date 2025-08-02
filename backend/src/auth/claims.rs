//! JWT Claims and User Authentication Types

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// JWT Claims structure containing user information and token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// User ID (UUID)
    pub sub: String,
    /// Username
    pub username: String,
    /// User role
    pub role: UserRole,
    /// Token issued at (Unix timestamp)
    pub iat: i64,
    /// Token expires at (Unix timestamp)
    pub exp: i64,
    /// Token issuer
    pub iss: String,
    /// Token audience
    pub aud: String,
    /// JWT ID for token revocation
    pub jti: String,
}

/// User roles for authorization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Operator,
    Viewer,
    ApiUser,
    System,
}

impl UserRole {
    /// Check if role has admin privileges
    pub fn is_admin(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    /// Check if role can modify positions
    pub fn can_modify_positions(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Operator)
    }

    /// Check if role can view positions
    pub fn can_view_positions(&self) -> bool {
        !matches!(self, UserRole::System)
    }

    /// Check if role can manage alerts
    pub fn can_manage_alerts(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::Operator)
    }

    /// Check if role can access system health endpoints
    pub fn can_access_system_health(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::System)
    }
}

impl Claims {
    /// Create new claims for a user
    pub fn new(
        user_id: Uuid,
        username: String,
        role: UserRole,
        expires_in_hours: i64,
    ) -> Self {
        let now = Utc::now();
        let exp = now + chrono::Duration::hours(expires_in_hours);
        
        Self {
            sub: user_id.to_string(),
            username,
            role,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "defi-risk-monitor".to_string(),
            aud: "defi-risk-monitor-api".to_string(),
            jti: Uuid::new_v4().to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp();
        now >= self.exp
    }

    /// Get user ID as UUID
    pub fn user_id(&self) -> Result<Uuid, uuid::Error> {
        Uuid::parse_str(&self.sub)
    }

    /// Get expiration time as DateTime
    pub fn expires_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.exp, 0).unwrap_or_else(Utc::now)
    }

    /// Get issued at time as DateTime
    pub fn issued_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.iat, 0).unwrap_or_else(Utc::now)
    }

    /// Check if user has permission for a specific action
    pub fn has_permission(&self, permission: Permission) -> bool {
        match permission {
            Permission::ViewPositions => self.role.can_view_positions(),
            Permission::ModifyPositions => self.role.can_modify_positions(),
            Permission::ManageAlerts => self.role.can_manage_alerts(),
            Permission::AccessSystemHealth => self.role.can_access_system_health(),
            Permission::AdminAccess => self.role.is_admin(),
        }
    }
}

/// Permissions for different API operations
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    ViewPositions,
    ModifyPositions,
    ManageAlerts,
    AccessSystemHealth,
    AdminAccess,
}

/// Token validation result
#[derive(Debug)]
pub enum TokenValidation {
    Valid(Claims),
    Expired,
    Invalid(String),
    Revoked,
}

/// Authentication context for requests
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub claims: Claims,
    pub token: String,
    pub authenticated_at: DateTime<Utc>,
}

impl AuthContext {
    pub fn new(claims: Claims, token: String) -> Self {
        Self {
            claims,
            token,
            authenticated_at: Utc::now(),
        }
    }

    /// Check if user has specific permission
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.claims.has_permission(permission)
    }

    /// Get user ID
    pub fn user_id(&self) -> Result<Uuid, uuid::Error> {
        self.claims.user_id()
    }

    /// Check if authentication is still valid (not expired)
    pub fn is_valid(&self) -> bool {
        !self.claims.is_expired()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claims_creation() {
        let user_id = Uuid::new_v4();
        let claims = Claims::new(
            user_id,
            "test_user".to_string(),
            UserRole::Operator,
            24, // 24 hours
        );

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.username, "test_user");
        assert_eq!(claims.role, UserRole::Operator);
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_user_role_permissions() {
        assert!(UserRole::Admin.is_admin());
        assert!(UserRole::Admin.can_modify_positions());
        assert!(UserRole::Admin.can_view_positions());
        assert!(UserRole::Admin.can_manage_alerts());

        assert!(!UserRole::Viewer.can_modify_positions());
        assert!(UserRole::Viewer.can_view_positions());
        assert!(!UserRole::Viewer.can_manage_alerts());

        assert!(!UserRole::System.can_view_positions());
        assert!(UserRole::System.can_access_system_health());
    }

    #[test]
    fn test_permission_checking() {
        let user_id = Uuid::new_v4();
        let admin_claims = Claims::new(
            user_id,
            "admin".to_string(),
            UserRole::Admin,
            24,
        );

        assert!(admin_claims.has_permission(Permission::ViewPositions));
        assert!(admin_claims.has_permission(Permission::ModifyPositions));
        assert!(admin_claims.has_permission(Permission::ManageAlerts));
        assert!(admin_claims.has_permission(Permission::AdminAccess));

        let viewer_claims = Claims::new(
            user_id,
            "viewer".to_string(),
            UserRole::Viewer,
            24,
        );

        assert!(viewer_claims.has_permission(Permission::ViewPositions));
        assert!(!viewer_claims.has_permission(Permission::ModifyPositions));
        assert!(!viewer_claims.has_permission(Permission::AdminAccess));
    }
}
