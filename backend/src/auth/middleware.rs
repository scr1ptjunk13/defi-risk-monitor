//! JWT Authentication Middleware for Axum

use crate::auth::claims::{AuthContext, Claims, Permission, TokenValidation};
use crate::auth::jwt::JwtService;
use crate::error::AppError;
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

/// JWT Authentication middleware
pub async fn jwt_auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract Authorization header
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Extract token from Bearer header
    let token = JwtService::extract_token_from_header(auth_header)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let validation_result = jwt_service
        .validate_token(token)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match validation_result {
        TokenValidation::Valid(claims) => {
            // Add auth context to request extensions
            let auth_context = AuthContext::new(claims, token.to_string());
            request.extensions_mut().insert(auth_context);
            Ok(next.run(request).await)
        }
        TokenValidation::Expired => Err(StatusCode::UNAUTHORIZED),
        TokenValidation::Invalid(_) => Err(StatusCode::UNAUTHORIZED),
        TokenValidation::Revoked => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Permission-based authorization middleware
pub fn require_permission(permission: Permission) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, StatusCode>> + Send>> + Clone {
    move |request: Request, next: Next| {
        let required_permission = permission.clone();
        Box::pin(async move {
            // Get auth context from request extensions
            let auth_context = request
                .extensions()
                .get::<AuthContext>()
                .ok_or(StatusCode::UNAUTHORIZED)?;

            // Check permission
            if !auth_context.has_permission(required_permission) {
                return Err(StatusCode::FORBIDDEN);
            }

            Ok(next.run(request).await)
        })
    }
}

/// Admin-only middleware
pub async fn admin_only_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let auth_context = request
        .extensions()
        .get::<AuthContext>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if !auth_context.has_permission(Permission::AdminAccess) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Optional authentication middleware (doesn't fail if no token)
pub async fn optional_jwt_auth_middleware(
    State(jwt_service): State<Arc<JwtService>>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try to extract and validate token, but don't fail if missing
    if let Some(auth_header) = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
    {
        if let Some(token) = JwtService::extract_token_from_header(auth_header) {
            if let Ok(TokenValidation::Valid(claims)) = jwt_service.validate_token(token).await {
                let auth_context = AuthContext::new(claims, token.to_string());
                request.extensions_mut().insert(auth_context);
            }
        }
    }

    next.run(request).await
}

/// Extract auth context from request (for use in handlers)
pub fn extract_auth_context(headers: &HeaderMap) -> Option<AuthContext> {
    // This would typically be called from within handlers
    // In practice, you'd get this from request extensions
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::claims::UserRole;
    use crate::auth::jwt::{JwtConfig, JwtService};
    use axum::{
        body::Body,
        http::{Method, Request},
        middleware,
        response::Response,
        routing::get,
        Router,
    };
    use std::sync::Arc;
    use tower::ServiceExt;
    use uuid::Uuid;

    async fn test_handler() -> &'static str {
        "success"
    }

    #[tokio::test]
    async fn test_jwt_middleware_with_valid_token() {
        let jwt_service = Arc::new(JwtService::new(JwtConfig::default()));
        
        // Generate a valid token
        let user_id = Uuid::new_v4();
        let token = jwt_service
            .generate_token(user_id, "test_user".to_string(), UserRole::Admin, Some(1))
            .await
            .unwrap();

        // Create router with middleware
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn_with_state(
                jwt_service.clone(),
                jwt_auth_middleware,
            ))
            .with_state(jwt_service);

        // Create request with valid token
        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap();

        // Send request
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_jwt_middleware_without_token() {
        let jwt_service = Arc::new(JwtService::new(JwtConfig::default()));

        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(middleware::from_fn_with_state(
                jwt_service.clone(),
                jwt_auth_middleware,
            ))
            .with_state(jwt_service);

        let request = Request::builder()
            .method(Method::GET)
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
