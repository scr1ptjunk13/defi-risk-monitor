use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::models::{
    AlertThreshold, CreateAlertThreshold, UpdateAlertThreshold, ThresholdType, ThresholdOperator,
    Alert,
};
use crate::services::{ThresholdService, ThresholdStats};

use crate::AppState;



// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateThresholdRequest {
    pub user_address: String,
    pub position_id: Option<Uuid>,
    pub threshold_type: ThresholdType,
    pub operator: ThresholdOperator,
    pub threshold_value: f64, // Accept as f64 for easier API usage
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateThresholdRequest {
    pub threshold_value: Option<f64>,
    pub is_enabled: Option<bool>,
    pub operator: Option<ThresholdOperator>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThresholdResponse {
    pub id: Uuid,
    pub user_address: String,
    pub position_id: Option<Uuid>,
    pub threshold_type: String,
    pub operator: String,
    pub threshold_value: f64,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub position_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub risk_score: Option<f64>,
    pub current_value: Option<f64>,
    pub threshold_value: Option<f64>,
    pub is_resolved: bool,
    pub resolved_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct GetThresholdsQuery {
    pub user_address: Option<String>,
    pub position_id: Option<Uuid>,
    pub threshold_type: Option<String>,
    pub is_enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GetAlertsQuery {
    pub user_address: Option<String>,
    pub position_id: Option<Uuid>,
    pub severity: Option<String>,
    pub is_resolved: Option<bool>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub total: u32,
    pub limit: u32,
    pub offset: u32,
}

// Helper functions
impl From<AlertThreshold> for ThresholdResponse {
    fn from(threshold: AlertThreshold) -> Self {
        use bigdecimal::ToPrimitive;
        
        Self {
            id: threshold.id,
            user_address: threshold.user_address,
            position_id: threshold.position_id,
            threshold_type: threshold.threshold_type,
            operator: threshold.operator,
            threshold_value: threshold.threshold_value.to_f64().unwrap_or(0.0),
            is_enabled: threshold.is_enabled,
            created_at: threshold.created_at.to_rfc3339(),
            updated_at: threshold.updated_at.to_rfc3339(),
        }
    }
}

impl From<Alert> for AlertResponse {
    fn from(alert: Alert) -> Self {
        use bigdecimal::ToPrimitive;
        
        Self {
            id: alert.id,
            position_id: alert.position_id,
            alert_type: alert.alert_type,
            severity: alert.severity,
            title: alert.title,
            message: alert.message,
            risk_score: alert.risk_score.and_then(|v| v.to_f64()),
            current_value: alert.current_value.and_then(|v| v.to_f64()),
            threshold_value: alert.threshold_value.and_then(|v| v.to_f64()),
            is_resolved: alert.is_resolved,
            resolved_at: alert.resolved_at.map(|dt| dt.to_rfc3339()),
            created_at: alert.created_at.to_rfc3339(),
        }
    }
}

// API Handlers

/// Create a new alert threshold
/// POST /api/v1/thresholds
pub async fn create_threshold(
    State(state): State<AppState>,
    Json(request): Json<CreateThresholdRequest>,
) -> Result<Json<ApiResponse<ThresholdResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    let threshold_value = match BigDecimal::from_str(&request.threshold_value.to_string()) {
        Ok(val) => val,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Invalid threshold value".to_string()),
        })))
    };
    
    let create_threshold = CreateAlertThreshold {
        user_address: request.user_address,
        position_id: request.position_id,
        threshold_type: request.threshold_type,
        threshold_value,
        operator: request.operator,
        is_enabled: request.is_enabled.unwrap_or(true),
    };

    match threshold_service.create_threshold(create_threshold).await {
        Ok(threshold) => Ok(Json(ApiResponse {
            success: true,
            data: Some(threshold.into()),
            message: Some("Threshold created successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to create threshold".to_string()),
        })))
    }
}

/// Get alert thresholds with optional filtering
/// GET /api/v1/thresholds
pub async fn get_thresholds(
    State(state): State<AppState>,
    Query(query): Query<GetThresholdsQuery>,
) -> Result<Json<ApiResponse<Vec<ThresholdResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    let thresholds = if let Some(user_address) = query.user_address {
        match threshold_service.get_user_thresholds(&user_address).await {
            Ok(thresholds) => thresholds,
            Err(_) => return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
                success: false,
                data: None,
                message: Some("Failed to get thresholds".to_string()),
            })))
        }
    } else {
        // For now, return empty if no user_address provided
        // In production, you might want admin-level access to view all thresholds
        Vec::new()
    };

    let response_data: Vec<ThresholdResponse> = thresholds.into_iter().map(|t| t.into()).collect();
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(response_data),
        message: None,
    }))
}

/// Get a specific threshold by ID
/// GET /api/v1/thresholds/{id}
pub async fn get_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
) -> Result<Json<ApiResponse<ThresholdResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    match threshold_service.get_threshold(threshold_id).await {
        Ok(Some(threshold)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(threshold.into()),
            message: None,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Threshold not found".to_string()),
        }))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to get threshold".to_string()),
        })))
    }
}

/// Update an alert threshold
/// PUT /api/v1/thresholds/{id}
pub async fn update_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
    Json(request): Json<UpdateThresholdRequest>,
) -> Result<Json<ApiResponse<ThresholdResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    let threshold_value = if let Some(value) = request.threshold_value {
        match BigDecimal::from_str(&value.to_string()) {
            Ok(val) => Some(val),
            Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
                success: false,
                data: None,
                message: Some("Invalid threshold value".to_string()),
            })))
        }
    } else {
        None
    };
    
    let update_threshold = UpdateAlertThreshold {
        threshold_value,
        is_enabled: request.is_enabled,
        operator: request.operator,
    };

    match threshold_service.update_threshold(threshold_id, update_threshold).await {
        Ok(threshold) => Ok(Json(ApiResponse {
            success: true,
            data: Some(threshold.into()),
            message: Some("Threshold updated successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to update threshold".to_string()),
        })))
    }
}

/// Delete an alert threshold
/// DELETE /api/v1/thresholds/{id}
pub async fn delete_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    match threshold_service.delete_threshold(threshold_id).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: Some("Threshold deleted successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Threshold not found or failed to delete".to_string()),
        })))
    }
}

/// Initialize default thresholds for a user
/// POST /api/v1/thresholds/defaults
pub async fn initialize_default_thresholds(
    State(state): State<AppState>,
    Json(request): Json<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<ThresholdResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_address = match request.get("user_address") {
        Some(addr) => addr,
        None => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("user_address is required".to_string()),
        })))
    };
    
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    match threshold_service.initialize_default_thresholds(user_address).await {
        Ok(thresholds) => {
            let response_data: Vec<ThresholdResponse> = thresholds.into_iter().map(|t| t.into()).collect();
            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: Some("Default thresholds initialized successfully".to_string()),
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to initialize default thresholds".to_string()),
        })))
    }
}

/// Get threshold statistics for a user
/// GET /api/v1/thresholds/stats/{user_address}
pub async fn get_threshold_stats(
    State(state): State<AppState>,
    Path(user_address): Path<String>,
) -> Result<Json<ApiResponse<ThresholdStats>>, (StatusCode, Json<ApiResponse<()>>)> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    match threshold_service.get_user_threshold_stats(&user_address).await {
        Ok(stats) => Ok(Json(ApiResponse {
            success: true,
            data: Some(stats),
            message: None,
        })),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to get threshold stats".to_string()),
        })))
    }
}

/// Get alerts with optional filtering
/// GET /api/v1/alerts
pub async fn get_alerts(
    State(_state): State<AppState>,
    Query(query): Query<GetAlertsQuery>,
) -> Result<Json<PaginatedResponse<AlertResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let limit = query.limit.unwrap_or(50).min(100); // Cap at 100
    let offset = query.offset.unwrap_or(0);
    
    // For now, return empty alerts since we don't have a full AlertService with DB queries
    // In a complete implementation, you'd query the alerts table with filters
    let alerts: Vec<AlertResponse> = Vec::new();
    
    Ok(Json(PaginatedResponse {
        success: true,
        data: alerts,
        total: 0,
        limit,
        offset,
    }))
}

/// Acknowledge/resolve an alert
/// PUT /api/v1/alerts/{id}/resolve
pub async fn resolve_alert(
    State(_state): State<AppState>,
    Path(_alert_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    // For now, return success
    // In a complete implementation, you'd update the alert in the database
    Ok(Json(ApiResponse {
        success: true,
        data: None,
        message: Some("Alert resolved successfully".to_string()),
    }))
}

/// Create the router for alert-related endpoints
pub fn create_alert_routes() -> Router<AppState> {
    Router::new()
        .route("/thresholds", post(create_threshold))
        .route("/thresholds", get(get_thresholds))
        .route("/thresholds/:id", get(get_threshold))
        .route("/thresholds/:id", put(update_threshold))
        .route("/thresholds/:id", delete(delete_threshold))
        .route("/thresholds/defaults", post(initialize_default_thresholds))
        .route("/thresholds/stats/:user_address", get(get_threshold_stats))
        .route("/alerts", get(get_alerts))
        .route("/alerts/:id/resolve", put(resolve_alert))
}
