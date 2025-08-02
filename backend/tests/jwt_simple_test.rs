use defi_risk_monitor::auth::claims::{Claims, UserRole, TokenValidation};
use defi_risk_monitor::auth::jwt::{JwtService, JwtConfig};
use defi_risk_monitor::error::AppError;
use jsonwebtoken::Algorithm;
use uuid::Uuid;
use tokio;

#[tokio::test]
async fn test_jwt_authentication_complete_flow() {
    println!("🧪 Testing Complete JWT Authentication Flow...");
    
    // Setup JWT service with test configuration
    let config = JwtConfig {
        secret: "test-secret-key-for-jwt-testing-12345".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
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
    match validation {
        TokenValidation::Valid(claims) => {
            assert_eq!(claims.sub, user_id.to_string(), "User ID should match");
            assert_eq!(claims.username, username, "Username should match");
            assert_eq!(claims.role, role, "Role should match");
            println!("✅ Token validation successful for user: {}", claims.username);
        }
        _ => panic!("Expected valid token validation"),
    }
    
    // Test 3: Token Revocation
    println!("🚫 Testing token revocation...");
    let revoke_result = jwt_service.revoke_token(&token).await;
    assert!(revoke_result.is_ok(), "Token revocation should succeed");
    println!("✅ Token revoked successfully");
    
    // Test 4: Validate Revoked Token (should fail)
    println!("❌ Testing revoked token validation...");
    let revoked_validation = jwt_service.validate_token(&token).await;
    assert!(revoked_validation.is_ok(), "Revoked token validation should return Ok");
    
    match revoked_validation.unwrap() {
        TokenValidation::Revoked => {
            println!("✅ Revoked token correctly identified as revoked");
        }
        _ => panic!("Expected revoked token validation"),
    }
    
    println!("🎉 Complete JWT Authentication Flow Test PASSED!");
}

#[tokio::test]
async fn test_jwt_claims_creation_and_validation() {
    println!("🧪 Testing JWT Claims Creation and Validation...");
    
    let user_id = Uuid::new_v4();
    let username = "test_claims_user".to_string();
    let role = UserRole::Operator;
    let expires_in_hours = 12;
    
    let claims = Claims::new(user_id, username.clone(), role.clone(), expires_in_hours);
    
    // Validate claims structure
    assert_eq!(claims.sub, user_id.to_string(), "User ID should match");
    assert_eq!(claims.username, username, "Username should match");
    assert_eq!(claims.role, role, "Role should match");
    assert!(!claims.jti.is_empty(), "JWT ID should not be empty");
    
    // Check expiration timing (within reasonable bounds)
    let expected_exp = claims.iat + (expires_in_hours * 3600);
    let exp_diff = (claims.exp as i64 - expected_exp as i64).abs();
    assert!(exp_diff < 5, "Expiration should be set correctly (within 5 seconds)");
    
    // Test expiration check
    assert!(!claims.is_expired(), "Claims should not be expired immediately");
    
    println!("✅ JWT Claims created and validated successfully:");
    println!("   User ID: {}", claims.sub);
    println!("   Username: {}", claims.username);
    println!("   Role: {:?}", claims.role);
    println!("   Expires in: {} hours", expires_in_hours);
    println!("   JWT ID: {}", claims.jti);
    
    println!("🎉 JWT Claims Creation and Validation Test PASSED!");
}

#[tokio::test]
async fn test_jwt_token_expiration() {
    println!("🧪 Testing JWT Token Expiration...");
    
    let config = JwtConfig {
        secret: "test-secret-expiration-12345".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
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
    assert!(result.is_ok(), "Expired token validation should return Ok");
    
    match result.unwrap() {
        TokenValidation::Expired => {
            println!("✅ Expired token correctly identified as expired");
        }
        _ => panic!("Expected expired token validation"),
    }
    
    println!("🎉 JWT Token Expiration Test PASSED!");
}

#[tokio::test]
async fn test_jwt_invalid_token() {
    println!("🧪 Testing Invalid JWT Token Handling...");
    
    let config = JwtConfig {
        secret: "test-secret-invalid-12345".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
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
        assert!(result.is_ok(), "Invalid token validation should return Ok");
        
        match result.unwrap() {
            TokenValidation::Invalid(msg) => {
                println!("✅ Invalid token {} correctly rejected: {}", i + 1, msg);
            }
            _ => panic!("Expected invalid token validation"),
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
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
    };
    let jwt_service1 = JwtService::new(config1);
    
    let config2 = JwtConfig {
        secret: "secret-two-67890".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
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
    assert!(result.is_ok(), "Token validation with different secret should return Ok");
    
    match result.unwrap() {
        TokenValidation::Invalid(msg) => {
            println!("✅ Token with different secret correctly rejected: {}", msg);
        }
        _ => panic!("Expected invalid token validation for different secret"),
    }
    
    println!("🎉 JWT Different Secrets Test PASSED!");
}

#[tokio::test]
async fn test_jwt_login_response() {
    println!("🧪 Testing JWT Login Response Generation...");
    
    let config = JwtConfig {
        secret: "test-secret-login-12345".to_string(),
        algorithm: Algorithm::HS256,
        expires_in_hours: 24,
        issuer: "defi-risk-monitor".to_string(),
        audience: "defi-risk-monitor-api".to_string(),
    };
    let jwt_service = JwtService::new(config);
    
    let user_id = Uuid::new_v4();
    let username = "test_login_user".to_string();
    let email = "test@example.com".to_string();
    let role = UserRole::Operator;
    
    // Create claims for login response
    let claims = Claims::new(user_id, username.clone(), role.clone(), 24);
    
    let login_response = jwt_service.create_login_response(username.clone(), &claims);
    
    assert!(!login_response.token.is_empty(), "Token should not be empty");
    assert!(login_response.expires_at > 0, "Expires at should be set");
    assert_eq!(login_response.user.id, user_id.to_string(), "User ID should match");
    assert_eq!(login_response.user.username, username, "Username should match");
    assert_eq!(login_response.user.role, role, "Role should match");

    println!("✅ JWT login response generated successfully:");
    let token_preview = if login_response.token.len() > 20 {
        &login_response.token[..20]
    } else {
        &login_response.token
    };
    println!("   Token: {}...", token_preview);
    println!("   Expires at: {}", login_response.expires_at);
    println!("   User: {} ({})", login_response.user.username, login_response.user.id);
    
    println!("🎉 JWT Login Response Test PASSED!");
}
