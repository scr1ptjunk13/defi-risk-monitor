use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;

use crate::{
    auth::{
        claims::{Claims, UserRole as JwtUserRole},
        jwt::{JwtService, LoginResponse, LoginRequest},
    },
    services::auth_service::{AuthService, UserRole},
    error::AppError,
    AppState,
};

// Helper function for password verification
fn verify_password(_provided_password: &str, _stored_hash: &str) -> bool {
    // TODO: In production, use proper password hashing like bcrypt
    // For now, allowing any password for development/testing (INSECURE - replace with bcrypt::verify)
    // This is a temporary implementation until proper password hashing is implemented
    true
}

// Request/Response DTOs
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub wallet_address: String,
    pub chain_id: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserSettingsRequest {
    pub notifications_enabled: Option<bool>,
    pub email_alerts: Option<bool>,
    pub risk_tolerance: Option<String>,
    pub preferred_currency: Option<String>,
    pub dashboard_layout: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct AddWalletAddressRequest {
    pub wallet_address: String,
    pub chain_id: i32,
    pub is_primary: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserSettingsResponse {
    pub user_id: Uuid,
    pub notifications_enabled: bool,
    pub email_alerts: bool,
    pub risk_tolerance: String,
    pub preferred_currency: String,
    pub dashboard_layout: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct UserPortfolioSummaryResponse {
    pub user_id: Uuid,
    pub total_value_usd: bigdecimal::BigDecimal,
    pub total_positions: i32,
    pub active_protocols: Vec<String>,
    pub risk_score: bigdecimal::BigDecimal,
    pub pnl_24h: bigdecimal::BigDecimal,
    pub pnl_7d: bigdecimal::BigDecimal,
    pub pnl_30d: bigdecimal::BigDecimal,
}

#[derive(Debug, Deserialize)]
pub struct GetUsersQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub search: Option<String>,
}

// Handler functions
pub async fn create_user(
    State(state): State<AppState>,
    Json(request): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let user = auth_service.create_user(
        request.username,
        request.email,
        "default_password".to_string(), // TODO: Handle password properly
        UserRole::Viewer, // Default role
    ).await?;
    
    let response = UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn get_user_by_id(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let user_option = auth_service.get_user_by_id(user_id).await?;
    
    let user = user_option.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    let response = UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn get_user_by_address(
    State(state): State<AppState>,
    Path(wallet_address): Path<String>,
) -> Result<Json<UserResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let user_option = auth_service.get_user_by_address(&wallet_address).await?;
    
    let user = user_option.ok_or_else(|| AppError::NotFound("User not found".to_string()))?;
    
    let response = UserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn update_user_settings(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserSettingsRequest>,
) -> Result<Json<UserSettingsResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let settings = auth_service.update_user_settings(
        user_id,
        request.notifications_enabled,
        None, // sms_notifications
        None, // webhook_notifications
        request.risk_tolerance,
        request.preferred_currency,
        None, // dashboard_layout
        None, // alert_frequency
    ).await?;
    
    let response = UserSettingsResponse {
        user_id: settings.user_id,
        notifications_enabled: settings.email_notifications,
        email_alerts: settings.email_notifications,
        risk_tolerance: settings.risk_tolerance,
        preferred_currency: settings.preferred_currency,
        dashboard_layout: settings.dashboard_layout,
    };
    
    Ok(Json(response))
}

// JWT Authentication Endpoints
pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    // Authenticate user with auth service
    let user = match auth_service.get_user_by_username(&request.username).await {
        Ok(Some(user)) => {
            // Verify password (in production, use proper password hashing like bcrypt)
            if verify_password(&request.password, "") {
                user
            } else {
                return Err(AppError::AuthenticationError("Invalid credentials".to_string()));
            }
        },
        Ok(None) => {
            return Err(AppError::AuthenticationError("User not found".to_string()));
        },
        Err(e) => {
            return Err(AppError::DatabaseError(format!("Database error during authentication: {}", e)));
        }
    };
    
    // Convert UserRole to JwtUserRole
    let jwt_role = match user.role {
        UserRole::Admin => JwtUserRole::Admin,
        UserRole::Operator => JwtUserRole::Operator,
        UserRole::Viewer => JwtUserRole::Viewer,
        UserRole::ApiUser => JwtUserRole::ApiUser,
        UserRole::System => JwtUserRole::System,
    };
    
    // Generate JWT token
    let token = state.jwt_service.generate_token(
        user.id,
        user.username.clone(),
        jwt_role.clone(),
        Some(24), // 24 hours
    ).await?;
    
    // Create claims for response
    let claims = Claims::new(user.id, user.username, jwt_role, 24);
    let response = state.jwt_service.create_login_response(token, &claims);
    
    Ok(Json(response))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, AppError> {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = JwtService::extract_token_from_header(auth_str) {
                state.jwt_service.revoke_token(token).await?;
            }
        }
    }
    Ok(StatusCode::OK)
}

pub async fn get_user_portfolio_summary(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserPortfolioSummaryResponse>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let summary = auth_service.get_user_portfolio_summary(user_id).await?;
    
    let response = UserPortfolioSummaryResponse {
        user_id: summary.user_id,
        total_value_usd: summary.total_value_usd,
        total_positions: summary.total_positions as i32,
        active_protocols: vec![], // Mock value - summary.active_protocols is i64, not Vec<String>
        risk_score: summary.total_risk_score,
        pnl_24h: BigDecimal::from(0), // Mock value - not available in struct
        pnl_7d: BigDecimal::from(0),  // Mock value - not available in struct
        pnl_30d: BigDecimal::from(0), // Mock value - not available in struct
    };
    
    Ok(Json(response))
}

pub async fn get_user_risk_preferences(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    let preferences = auth_service.get_user_risk_preferences(user_id).await?;
    
    Ok(Json(serde_json::to_value(preferences)?))
}

pub async fn add_wallet_address(
    State(state): State<AppState>,
    Path(_user_id): Path<Uuid>,
    Json(_request): Json<AddWalletAddressRequest>,
) -> Result<StatusCode, AppError> {
    let _auth_service = AuthService::new(state.db_pool.clone(), "default_jwt_secret".to_string());
    
    // This would need to be implemented in AuthService
    // auth_service.add_wallet_address(user_id, &request.wallet_address, request.chain_id, request.is_primary.unwrap_or(false)).await?;
    
    Ok(StatusCode::CREATED)
}

// Create router
pub fn create_auth_routes() -> Router<AppState> {
    Router::new()
        .route("/users", post(create_user))
        .route("/users/:user_id", get(get_user_by_id))
        .route("/users/address/:wallet_address", get(get_user_by_address))
        .route("/users/:user_id/settings", put(update_user_settings))
        .route("/users/:user_id/portfolio-summary", get(get_user_portfolio_summary))
        .route("/users/:user_id/risk-preferences", get(get_user_risk_preferences))
        .route("/users/:user_id/wallet-addresses", post(add_wallet_address))
}
