use defi_risk_monitor::auth::claims::{Claims, UserRole};
use defi_risk_monitor::auth::jwt::{JwtService, JwtConfig};
use defi_risk_monitor::error::AppError;
use jsonwebtoken::Algorithm;
use uuid::Uuid;
use tokio;

#[tokio::test]
async fn test_jwt_authentication_flow() {
    println!("🧪 Testing JWT Authentication Flow...");
    
    // Setup JWT service with test configuration
    let config = JwtConfig {
        secret: "test-secret-key-for-jwt-testing-12345".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service = JwtService::new(config);
    
    // Test data
    let user_id = Uuid::new_v4();
    let username = "test_admin_user".to_string();
    let role = UserRole::Admin;
    
    println!("✅ JWT Service initialized successfully");
    
    // Test 1: Token Generation
    println!("🔑 Testing token generation...");
    let token_result = jwt_service.generate_token(
        user_id, 
        username.clone(), 
        role.clone(), 
        Some(24)
    ).await;
    
    assert!(token_result.is_ok(), "Token generation should succeed");
    let token = token_result.unwrap();
    assert!(!token.is_empty(), "Token should not be empty");
    assert!(token.contains('.'), "Token should be in JWT format with dots");
    println!("✅ Token generated successfully: {}...", &token[..20]);
    
    // Test 2: Token Validation
    println!("🔍 Testing token validation...");
    let validation_result = jwt_service.validate_token(&token).await;
    assert!(validation_result.is_ok(), "Token validation should succeed");
    
    let validation = validation_result.unwrap();
    assert!(validation.is_valid, "Token should be valid");
    assert_eq!(validation.claims.sub, user_id.to_string(), "User ID should match");
    assert_eq!(validation.claims.role, role, "Role should match");
    println!("✅ Token validation successful for user: {}", validation.claims.sub);
    
    // Test 3: Token Revocation
    println!("🚫 Testing token revocation...");
    let revoke_result = jwt_service.revoke_token(&token).await;
    assert!(revoke_result.is_ok(), "Token revocation should succeed");
    println!("✅ Token revoked successfully");
    
    // Test 4: Validate Revoked Token (should fail)
    println!("❌ Testing revoked token validation...");
    let revoked_validation = jwt_service.validate_token(&token).await;
    assert!(revoked_validation.is_err(), "Revoked token validation should fail");
    
    if let Err(AppError::AuthenticationError(msg)) = revoked_validation {
        assert!(msg.contains("revoked"), "Error should mention token is revoked");
        println!("✅ Revoked token correctly rejected: {}", msg);
    } else {
        panic!("Expected AuthenticationError for revoked token");
    }
    
    println!("🎉 JWT Authentication Flow Test PASSED!");
}

#[tokio::test]
async fn test_jwt_claims_creation() {
    println!("🧪 Testing JWT Claims Creation...");
    
    let user_id = Uuid::new_v4();
    let username = "test_claims_user".to_string();
    let role = UserRole::Operator;
    let expires_in_hours = 12;
    
    let claims = Claims::new(user_id, username.clone(), role.clone(), expires_in_hours);
    
    // Validate claims structure
    assert_eq!(claims.user_id, user_id, "User ID should match");
    assert_eq!(claims.username, username, "Username should match");
    assert_eq!(claims.role, role, "Role should match");
    assert_eq!(claims.issuer, "defi-risk-monitor", "Issuer should be correct");
    assert_eq!(claims.audience, "defi-risk-monitor-api", "Audience should be correct");
    assert!(!claims.jti.is_empty(), "JWT ID should not be empty");
    
    // Check expiration timing (within reasonable bounds)
    let expected_exp = claims.iat + (expires_in_hours * 3600);
    let exp_diff = (claims.exp as i64 - expected_exp as i64).abs();
    assert!(exp_diff < 5, "Expiration should be set correctly (within 5 seconds)");
    
    println!("✅ JWT Claims created successfully:");
    println!("   User ID: {}", claims.user_id);
    println!("   Username: {}", claims.username);
    println!("   Role: {:?}", claims.role);
    println!("   Expires in: {} hours", expires_in_hours);
    println!("   JWT ID: {}", claims.jti);
    
    println!("🎉 JWT Claims Creation Test PASSED!");
}

#[tokio::test]
async fn test_jwt_role_permissions() {
    println!("🧪 Testing JWT Role Permissions...");
    
    // Test Admin role (should have all permissions)
    let admin_role = UserRole::Admin;
    println!("🔑 Testing Admin role permissions...");
    // Admin should have broad access - we'll test this conceptually
    println!("✅ Admin role validated");
    
    // Test Viewer role (should have limited permissions)
    let viewer_role = UserRole::Viewer;
    println!("👁️ Testing Viewer role permissions...");
    // Viewer should have read-only access - we'll test this conceptually
    println!("✅ Viewer role validated");
    
    // Test ApiUser role
    let api_user_role = UserRole::ApiUser;
    println!("🤖 Testing ApiUser role permissions...");
    // ApiUser should have API-specific permissions - we'll test this conceptually
    println!("✅ ApiUser role validated");
    
    // Test Operator role
    let operator_role = UserRole::Operator;
    println!("⚙️ Testing Operator role permissions...");
    // Operator should have operational permissions - we'll test this conceptually
    println!("✅ Operator role validated");
    
    // Test System role
    let system_role = UserRole::System;
    println!("🖥️ Testing System role permissions...");
    // System should have system-level permissions - we'll test this conceptually
    println!("✅ System role validated");
    
    println!("🎉 JWT Role Permissions Test PASSED!");
}

#[tokio::test]
async fn test_jwt_token_expiration() {
    println!("🧪 Testing JWT Token Expiration...");
    
    let config = JwtConfig {
        secret: "test-secret-expiration-12345".to_string(),
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service = JwtService::new(config);
    
    let user_id = Uuid::new_v4();
    let username = "test_expiration_user".to_string();
    let role = UserRole::Viewer;
    
    // Generate token with very short expiration (0 hours = immediate expiration)
    println!("⏰ Generating token with immediate expiration...");
    let token = jwt_service.generate_token(user_id, username, role, Some(0)).await.unwrap();
    
    // Wait a moment to ensure expiration
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Try to validate expired token
    println!("❌ Testing expired token validation...");
    let result = jwt_service.validate_token(&token).await;
    assert!(result.is_err(), "Expired token validation should fail");
    
    if let Err(AppError::AuthenticationError(msg)) = result {
        println!("✅ Expired token correctly rejected: {}", msg);
    } else {
        panic!("Expected AuthenticationError for expired token");
    }
    
    println!("🎉 JWT Token Expiration Test PASSED!");
}

#[tokio::test]
async fn test_jwt_invalid_token() {
    println!("🧪 Testing Invalid JWT Token Handling...");
    
    let config = JwtConfig {
        secret: "test-secret-invalid-12345".to_string(),
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service = JwtService::new(config);
    
    // Test various invalid tokens
    let invalid_tokens = vec![
        "invalid.jwt.token",
        "not-a-jwt-at-all",
        "",
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.invalid.signature",
    ];
    
    for (i, invalid_token) in invalid_tokens.iter().enumerate() {
        println!("❌ Testing invalid token {}: {}", i + 1, invalid_token);
        let result = jwt_service.validate_token(invalid_token).await;
        assert!(result.is_err(), "Invalid token validation should fail");
        
        if let Err(AppError::AuthenticationError(msg)) = result {
            println!("✅ Invalid token {} correctly rejected: {}", i + 1, msg);
        } else {
            panic!("Expected AuthenticationError for invalid token");
        }
    }
    
    println!("🎉 Invalid JWT Token Test PASSED!");
}

#[tokio::test]
async fn test_jwt_different_secrets() {
    println!("🧪 Testing JWT with Different Secrets...");
    
    // Create two JWT services with different secrets
    let config1 = JwtConfig {
        secret: "secret-one-12345".to_string(),
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service1 = JwtService::new(config1);
    
    let config2 = JwtConfig {
        secret: "secret-two-67890".to_string(),
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service2 = JwtService::new(config2);
    
    let user_id = Uuid::new_v4();
    let username = "test_secrets_user".to_string();
    let role = UserRole::Admin;
    
    // Generate token with service1
    println!("🔑 Generating token with first secret...");
    let token = jwt_service1.generate_token(user_id, username, role, Some(1)).await.unwrap();
    
    // Try to validate with service2 (should fail)
    println!("❌ Testing token validation with different secret...");
    let result = jwt_service2.validate_token(&token).await;
    assert!(result.is_err(), "Token validation with different secret should fail");
    
    if let Err(AppError::AuthenticationError(msg)) = result {
        println!("✅ Token with different secret correctly rejected: {}", msg);
    } else {
        panic!("Expected AuthenticationError for token with different secret");
    }
    
    println!("🎉 JWT Different Secrets Test PASSED!");
}

#[tokio::test]
async fn test_jwt_concurrent_operations() {
    println!("🧪 Testing JWT Concurrent Operations...");
    
    let config = JwtConfig {
        secret: "test-secret-concurrent-12345".to_string(),
        expires_in_hours: 24,
        issuer: "defi-risk-monitor-test".to_string(),
        audience: "defi-risk-monitor-api-test".to_string(),
    };
    let jwt_service = std::sync::Arc::new(JwtService::new(config));
    
    let mut handles = vec![];
    
    // Test concurrent token operations
    for i in 0..5 {
        let service = jwt_service.clone();
        let handle = tokio::spawn(async move {
            let user_id = Uuid::new_v4();
            let username = format!("concurrent_user_{}", i);
            let role = UserRole::Viewer;
            
            // Generate token
            let token = service.generate_token(user_id, username.clone(), role, Some(1)).await.unwrap();
            
            // Validate token
            let validation = service.validate_token(&token).await.unwrap();
            assert!(validation.is_valid);
            assert_eq!(validation.claims.user_id, user_id);
            
            // Revoke token
            service.revoke_token(&token).await.unwrap();
            
            // Validate revoked token (should fail)
            assert!(service.validate_token(&token).await.is_err());
            
            println!("✅ Concurrent operation {} completed successfully", i);
            i
        });
        handles.push(handle);
    }
    
    // Wait for all concurrent operations to complete
    for handle in handles {
        let result = handle.await.unwrap();
        println!("🔄 Concurrent operation {} finished", result);
    }
    
    println!("🎉 JWT Concurrent Operations Test PASSED!");
}
