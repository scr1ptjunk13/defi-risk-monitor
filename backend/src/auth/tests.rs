#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::{Claims, UserRole};
    use crate::auth::jwt::{JwtService, JwtConfig};
    use crate::error::AppError;
    use std::sync::Arc;
    use tokio;
    use uuid::Uuid;

    fn setup_jwt_service() -> JwtService {
        let config = JwtConfig {
            secret: "test-secret-key-for-jwt-testing".to_string(),
            expires_in_hours: 24,
            issuer: "test-issuer".to_string(),
            audience: "test-audience".to_string(),
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
        assert!(validation.is_valid);
        assert_eq!(validation.claims.user_id, user_id);
        assert_eq!(validation.claims.role, role);
        println!("Token validation successful for user: {}", validation.claims.user_id);
    }

    #[tokio::test]
    async fn test_jwt_token_revocation() {
        let jwt_service = setup_jwt_service().await;
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Operator;

        // Generate token
        let token = jwt_service.generate_token(user_id, username, role, Some(1)).await.unwrap();
        
        // Validate token (should work)
        assert!(jwt_service.validate_token(&token).await.is_ok());
        
        // Revoke token
        jwt_service.revoke_token(&token).await.unwrap();
        
        // Validate token again (should fail)
        let result = jwt_service.validate_token(&token).await;
        assert!(result.is_err());
        
        if let Err(AppError::AuthenticationError(msg)) = result {
            assert!(msg.contains("revoked"));
            println!("Token revocation test passed: {}", msg);
        } else {
            panic!("Expected AuthenticationError for revoked token");
        }
    }

    #[tokio::test]
    async fn test_jwt_token_expiration() {
        let jwt_service = setup_jwt_service().await;
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Viewer;

        // Generate token with very short expiration (1 second)
        let token = jwt_service.generate_token(user_id, username, role, Some(0)).await.unwrap();
        
        // Wait for token to expire
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Validate expired token (should fail)
        let result = jwt_service.validate_token(&token).await;
        assert!(result.is_err());
        
        if let Err(AppError::AuthenticationError(msg)) = result {
            assert!(msg.contains("expired") || msg.contains("Expired"));
            println!("Token expiration test passed: {}", msg);
        } else {
            panic!("Expected AuthenticationError for expired token");
        }
    }

    #[tokio::test]
    async fn test_invalid_jwt_token() {
        let jwt_service = setup_jwt_service().await;
        
        // Test with invalid token
        let invalid_token = "invalid.jwt.token";
        let result = jwt_service.validate_token(invalid_token).await;
        
        assert!(result.is_err());
        if let Err(AppError::AuthenticationError(msg)) = result {
            println!("Invalid token test passed: {}", msg);
        } else {
            panic!("Expected AuthenticationError for invalid token");
        }
    }

    #[tokio::test]
    async fn test_jwt_claims_creation() {
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Admin;
        let expires_in_hours = 24;

        let claims = Claims::new(user_id, username.clone(), role.clone(), expires_in_hours);
        
        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.role, role);
        assert_eq!(claims.issuer, "defi-risk-monitor");
        assert_eq!(claims.audience, "defi-risk-monitor-api");
        assert!(!claims.jti.is_empty());
        
        // Check expiration is set correctly (approximately)
        let expected_exp = claims.iat + (expires_in_hours * 3600);
        assert!((claims.exp as i64 - expected_exp as i64).abs() < 5); // Allow 5 second tolerance
        
        println!("JWT claims creation test passed for user: {}", claims.username);
    }

    #[tokio::test]
    async fn test_user_role_permissions() {
        // Test Admin permissions
        let admin_role = UserRole::Admin;
        assert!(admin_role.has_permission(&crate::auth::claims::Permission::ReadPositions));
        assert!(admin_role.has_permission(&crate::auth::claims::Permission::CreatePositions));
        assert!(admin_role.has_permission(&crate::auth::claims::Permission::DeletePositions));
        assert!(admin_role.has_permission(&crate::auth::claims::Permission::ManageUsers));
        
        // Test Viewer permissions
        let viewer_role = UserRole::Viewer;
        assert!(viewer_role.has_permission(&crate::auth::claims::Permission::ReadPositions));
        assert!(viewer_role.has_permission(&crate::auth::claims::Permission::ReadAlerts));
        assert!(!viewer_role.has_permission(&crate::auth::claims::Permission::CreatePositions));
        assert!(!viewer_role.has_permission(&crate::auth::claims::Permission::ManageUsers));
        
        // Test ApiUser permissions
        let api_user_role = UserRole::ApiUser;
        assert!(api_user_role.has_permission(&crate::auth::claims::Permission::ReadPositions));
        assert!(api_user_role.has_permission(&crate::auth::claims::Permission::CreatePositions));
        assert!(api_user_role.has_permission(&crate::auth::claims::Permission::UpdatePositions));
        assert!(!api_user_role.has_permission(&crate::auth::claims::Permission::ManageUsers));
        
        println!("User role permissions test passed");
    }

    #[tokio::test]
    async fn test_jwt_service_login_response() {
        let jwt_service = setup_jwt_service().await;
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let email = "test@example.com".to_string();
        let role = UserRole::Operator;

        let login_response = jwt_service.create_login_response(
            user_id, 
            username.clone(), 
            email.clone(), 
            role.clone()
        ).await.unwrap();

        assert!(!login_response.access_token.is_empty());
        assert_eq!(login_response.token_type, "Bearer");
        assert_eq!(login_response.expires_in, 86400); // 24 hours in seconds
        assert_eq!(login_response.user.id, user_id);
        assert_eq!(login_response.user.username, username);
        assert_eq!(login_response.user.email, email);
        assert_eq!(login_response.user.role, role);

        println!("JWT login response test passed for user: {}", username);
    }

    #[tokio::test]
    async fn test_concurrent_token_operations() {
        let jwt_service = Arc::new(setup_jwt_service().await);
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
                let claims = service.validate_token(&token).await.unwrap();
                assert_eq!(claims.username, username);
                
                // Revoke token
                service.revoke_token(&token).await.unwrap();
                
                // Validate revoked token (should fail)
                assert!(service.validate_token(&token).await.is_err());
                
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
        let jwt_service1 = JwtService::new("secret1".to_string()).await;
        let jwt_service2 = JwtService::new("secret2".to_string()).await;
        
        let user_id = Uuid::new_v4();
        let username = "test_user".to_string();
        let role = UserRole::Admin;

        // Generate token with service1
        let token = jwt_service1.generate_token(user_id, username, role, Some(1)).await.unwrap();
        
        // Try to validate with service2 (should fail)
        let result = jwt_service2.validate_token(&token).await;
        assert!(result.is_err());
        
        if let Err(AppError::AuthenticationError(msg)) = result {
            println!("Different secrets test passed: {}", msg);
        } else {
            panic!("Expected AuthenticationError for token with different secret");
        }
    }
}
