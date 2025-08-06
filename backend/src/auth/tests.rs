#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{Claims, UserRole, TokenValidation};
    use crate::auth::jwt::{JwtService, JwtConfig};
    use crate::error::AppError;
    use jsonwebtoken::Algorithm;
    use std::sync::Arc;
    use tokio;
    use uuid::Uuid;

    fn setup_jwt_service() -> JwtService {
        let config = JwtConfig {
            secret: "test-secret-key".to_string(),
            algorithm: Algorithm::HS256,
            expires_in_hours: 24,
            issuer: "test".to_string(),
            audience: "test".to_string(),
        };
        JwtService::new(config)
    }

    #[tokio::test]
    async fn test_jwt_token_generation() {
        let jwt_service = setup_jwt_service();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Viewer;

        let result = jwt_service.generate_token(user_id, username.clone(), role, Some(1)).await;
        
        assert!(result.is_ok());
        let token = result.unwrap();
        assert!(!token.is_empty());
        println!("Generated JWT token: {}", token);
    }

    #[tokio::test]
    async fn test_jwt_token_validation() {
        let jwt_service = setup_jwt_service();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Admin;

        // Generate token
        let token = jwt_service.generate_token(user_id, username.clone(), role.clone(), Some(1)).await.unwrap();
        
        // Validate token
        let result = jwt_service.validate_token(&token).await;
        assert!(result.is_ok());
        
        let validation = result.unwrap();
        if let TokenValidation::Valid(claims) = validation {
            assert_eq!(claims.user_id().unwrap(), user_id);
            assert_eq!(claims.role, role);
            println!("Token validation successful for user: {}", claims.user_id().unwrap());
        } else {
            panic!("Expected valid token validation");
        }
    }

    #[tokio::test]
    async fn test_jwt_token_revocation() {
        let jwt_service = setup_jwt_service();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Operator;

        // Generate token
        let token = jwt_service.generate_token(user_id, username, role, Some(1)).await.unwrap();
        
        // Validate token (should work)
        assert!(jwt_service.validate_token(&token).await.is_ok());
        
        // Revoke token
        jwt_service.revoke_token(&token).await.unwrap();
        
        // Validate token again (should return Revoked)
        let result = jwt_service.validate_token(&token).await;
        assert!(result.is_ok());
        
        if let Ok(TokenValidation::Revoked) = result {
            println!("Token revocation test passed");
        } else {
            panic!("Expected TokenValidation::Revoked for revoked token");
        }
    }

    #[tokio::test]
    async fn test_jwt_token_expiration() {
        let jwt_service = setup_jwt_service();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Viewer;

        // Generate token with very short expiration (1 second)
        let token = jwt_service.generate_token(user_id, username, role, Some(0)).await.unwrap();
        
        // Wait for token to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Validate expired token (should return Expired)
        let result = jwt_service.validate_token(&token).await;
        assert!(result.is_ok());
        
        if let Ok(TokenValidation::Expired) = result {
            println!("Token expiration test passed");
        } else {
            panic!("Expected TokenValidation::Expired for expired token");
        }
    }

    #[tokio::test]
    async fn test_invalid_jwt_token() {
        let jwt_service = setup_jwt_service();
        
        // Test with invalid token
        let invalid_token = "invalid.jwt.token";
        let result = jwt_service.validate_token(invalid_token).await;
        
        assert!(result.is_ok());
        if let Ok(TokenValidation::Invalid(msg)) = result {
            println!("Invalid token test passed: {}", msg);
        } else {
            panic!("Expected TokenValidation::Invalid for invalid token");
        }
    }

    #[tokio::test]
    async fn test_jwt_claims_creation() {
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Admin;
        let expires_in_hours = 24;

        let claims = Claims::new(user_id, username.clone(), role.clone(), expires_in_hours);
        
        assert_eq!(claims.user_id().unwrap(), user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.role, role);
        assert_eq!(claims.iss, "defi-risk-monitor");
        assert_eq!(claims.aud, "defi-risk-monitor-api");
        assert!(!claims.jti.is_empty());
        
        // Check expiration is set correctly (approximately)
        let expected_exp = claims.iat + (expires_in_hours * 3600);
        assert!((claims.exp as i64 - expected_exp as i64).abs() < 5); // Allow 5 second tolerance
        
        println!("JWT claims creation test passed for user: {}", claims.username);
    }

    #[test]
    fn test_user_role_permissions() {
        // Test Admin permissions
        let admin_role = UserRole::Admin;
        assert!(admin_role.is_admin());
        assert!(admin_role.can_modify_positions());
        assert!(admin_role.can_view_positions());
        assert!(admin_role.can_manage_alerts());
        assert!(admin_role.can_access_system_health());
        
        // Test Operator permissions
        let operator_role = UserRole::Operator;
        assert!(!operator_role.is_admin());
        assert!(operator_role.can_modify_positions());
        assert!(operator_role.can_view_positions());
        assert!(operator_role.can_manage_alerts());
        assert!(!operator_role.can_access_system_health());
        
        // Test Viewer permissions
        let viewer_role = UserRole::Viewer;
        assert!(!viewer_role.is_admin());
        assert!(!viewer_role.can_modify_positions());
        assert!(viewer_role.can_view_positions());
        assert!(!viewer_role.can_manage_alerts());
        assert!(!viewer_role.can_access_system_health());
        
        // Test ApiUser permissions
        let api_user_role = UserRole::ApiUser;
        assert!(!api_user_role.is_admin());
        assert!(!api_user_role.can_modify_positions());
        assert!(api_user_role.can_view_positions());
        assert!(!api_user_role.can_manage_alerts());
        assert!(!api_user_role.can_access_system_health());
        
        println!("User role permissions test passed");
    }

    #[tokio::test]
    async fn test_jwt_service_login_response() {
        let jwt_service = setup_jwt_service();
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let email = "test@example.com".to_string();
        let role = UserRole::Operator;

        let token = jwt_service.generate_token(user_id, username.clone(), role.clone(), None).await.unwrap();
        let claims = Claims::new(user_id, username.clone(), role.clone(), 24);
        let login_response = jwt_service.create_login_response(token, &claims);

        assert!(!login_response.token.is_empty());
        assert_eq!(login_response.user.id, user_id.to_string());
        assert_eq!(login_response.user.username, username);
        assert_eq!(login_response.user.role, role);

        println!("JWT login response test passed for user: {}", username);
    }

    #[tokio::test]
    async fn test_concurrent_token_operations() {
        let jwt_service = Arc::new(setup_jwt_service());
        let mut handles = vec![];

        // Test concurrent token generation and validation
        for i in 0..10 {
            let service = jwt_service.clone();
            let handle = tokio::spawn(async move {
                let user_id = Uuid::new_v4();
                let username = format!("user_{}", i);
                let role = UserRole::Viewer;

                // Generate token
                let token = service.generate_token(user_id, username.clone(), role, Some(1)).await.unwrap();
                
                // Validate token
                let validation_result = service.validate_token(&token).await.unwrap();
                if let TokenValidation::Valid(claims) = validation_result {
                    assert_eq!(claims.username, username);
                } else {
                    panic!("Expected valid token validation");
                }
                
                // Revoke token
                service.revoke_token(&token).await.unwrap();
                
                // Validate revoked token (should return Revoked)
                let result = service.validate_token(&token).await;
                assert!(matches!(result, Ok(TokenValidation::Revoked)));
                
                i
            });
            handles.push(handle);
        }

        // Wait for all concurrent operations to complete
        for handle in handles {
            let result = handle.await.unwrap();
            println!("Concurrent operation {} completed successfully", result);
        }

        println!("Concurrent token operations test passed");
    }

    #[tokio::test]
    async fn test_jwt_token_with_different_secrets() {
        let config1 = JwtConfig {
            secret: "secret1".to_string(),
            algorithm: Algorithm::HS256,
            expires_in_hours: 24,
            issuer: "test".to_string(),
            audience: "test".to_string(),
        };
        let config2 = JwtConfig {
            secret: "secret2".to_string(),
            algorithm: Algorithm::HS256,
            expires_in_hours: 24,
            issuer: "test".to_string(),
            audience: "test".to_string(),
        };
        let service1 = JwtService::new(config1);
        let service2 = JwtService::new(config2);
        
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Admin;

        // Generate token with service1
        let token = service1.generate_token(user_id, username, role, Some(1)).await.unwrap();
        
        // Try to validate with service2 (should return Invalid)
        let result = service2.validate_token(&token).await;
        assert!(result.is_ok());
        
        if let Ok(TokenValidation::Invalid(msg)) = result {
            println!("Different secrets test passed: {}", msg);
        } else {
            panic!("Expected TokenValidation::Invalid for token with different secret");
        }
    }
}
