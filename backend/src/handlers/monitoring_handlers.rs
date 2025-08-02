use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crate::{
    services::{
        monitoring_service::MonitoringService,
        alert_service::AlertService,
        threshold_service::ThresholdService,
    },
    error::AppError,
    AppState,
};

// Request/Response DTOs
#[derive(Debug, Deserialize)]
pub struct CreateThresholdRequest {
    pub user_id: Uuid,
    pub position_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub threshold_type: String,
    pub operator: String, // gt, lt, gte, lte, eq
    pub value: BigDecimal,
    pub severity: String,
    pub is_enabled: bool,
    pub notification_channels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateThresholdRequest {
    pub threshold_type: Option<String>,
    pub operator: Option<String>,
    pub value: Option<BigDecimal>,
    pub severity: Option<String>,
    pub is_enabled: Option<bool>,
    pub notification_channels: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ThresholdResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub position_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub threshold_type: String,
    pub operator: String,
    pub value: BigDecimal,
    pub severity: String,
    pub is_enabled: bool,
    pub notification_channels: Vec<String>,
    pub triggered_count: i32,
    pub last_triggered: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub position_id: Option<Uuid>,
    pub threshold_id: Option<Uuid>,
    pub alert_type: String,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub current_value: Option<BigDecimal>,
    pub threshold_value: Option<BigDecimal>,
    pub metadata: Option<serde_json::Value>,
    pub is_resolved: bool,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MonitoringStatsResponse {
    pub total_positions_monitored: i64,
    pub active_thresholds: i64,
    pub alerts_last_24h: i64,
    pub critical_alerts_active: i64,
    pub avg_response_time_ms: BigDecimal,
    pub uptime_percentage: BigDecimal,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GetThresholdsQuery {
    pub user_id: Option<Uuid>,
    pub position_id: Option<Uuid>,
    pub protocol: Option<String>,
    pub threshold_type: Option<String>,
    pub is_enabled: Option<bool>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GetAlertsQuery {
    pub user_id: Option<Uuid>,
    pub position_id: Option<Uuid>,
    pub alert_type: Option<String>,
    pub severity: Option<String>,
    pub is_resolved: Option<bool>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveAlertRequest {
    pub resolution_note: Option<String>,
}

// Handler functions
pub async fn create_threshold(
    State(state): State<AppState>,
    Json(request): Json<CreateThresholdRequest>,
) -> Result<Json<ThresholdResponse>, AppError> {
    use crate::models::{CreateAlertThreshold, ThresholdType, ThresholdOperator};
    
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    // Convert string to enum types
    let threshold_type = match request.threshold_type.as_str() {
        "impermanent_loss" => ThresholdType::ImpermanentLoss,
        "tvl_drop" => ThresholdType::TvlDrop,
        "liquidity_risk" => ThresholdType::LiquidityRisk,
        "volatility_risk" => ThresholdType::VolatilityRisk,
        "protocol_risk" => ThresholdType::ProtocolRisk,
        "mev_risk" => ThresholdType::MevRisk,
        "cross_chain_risk" => ThresholdType::CrossChainRisk,
        "overall_risk" => ThresholdType::OverallRisk,
        _ => return Err(AppError::ValidationError("Invalid threshold type".to_string())),
    };
    
    let operator = match request.operator.as_str() {
        "gt" | "greater_than" => ThresholdOperator::GreaterThan,
        "lt" | "less_than" => ThresholdOperator::LessThan,
        "gte" | "greater_than_or_equal" => ThresholdOperator::GreaterThanOrEqual,
        "lte" | "less_than_or_equal" => ThresholdOperator::LessThanOrEqual,
        _ => return Err(AppError::ValidationError("Invalid operator".to_string())),
    };
    
    let create_threshold = CreateAlertThreshold {
        user_address: request.user_id.to_string(), // Convert UUID to string for now
        position_id: request.position_id,
        threshold_type,
        operator,
        threshold_value: request.value,
        is_enabled: request.is_enabled,
    };
    
    let threshold = threshold_service.create_threshold(create_threshold).await?;
    
    let response = ThresholdResponse {
        id: threshold.id,
        user_id: request.user_id, // Use original request user_id
        position_id: threshold.position_id,
        protocol: request.protocol,
        threshold_type: threshold.threshold_type,
        operator: threshold.operator,
        value: threshold.threshold_value,
        severity: request.severity,
        is_enabled: threshold.is_enabled,
        notification_channels: request.notification_channels,
        triggered_count: 0, // Default for new threshold
        last_triggered: None,
        created_at: threshold.created_at,
        updated_at: threshold.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn get_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
) -> Result<Json<ThresholdResponse>, AppError> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    let threshold = threshold_service.get_threshold(threshold_id).await?
        .ok_or_else(|| AppError::NotFound("Threshold not found".to_string()))?;
    
    let response = ThresholdResponse {
        id: threshold.id,
        user_id: Uuid::parse_str(&threshold.user_address).unwrap_or_else(|_| Uuid::new_v4()), // Convert string to UUID
        position_id: threshold.position_id,
        protocol: None, // Not stored in AlertThreshold model
        threshold_type: threshold.threshold_type,
        operator: threshold.operator,
        value: threshold.threshold_value,
        severity: "medium".to_string(), // Default severity
        is_enabled: threshold.is_enabled,
        notification_channels: vec![], // Default empty
        triggered_count: 0, // Default
        last_triggered: None,
        created_at: threshold.created_at,
        updated_at: threshold.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn update_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
    Json(request): Json<UpdateThresholdRequest>,
) -> Result<Json<ThresholdResponse>, AppError> {
    use crate::models::{UpdateAlertThreshold, ThresholdOperator};
    
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    // Convert operator string to enum if provided
    let operator = if let Some(ref op_str) = request.operator {
        Some(match op_str.as_str() {
            "gt" | "greater_than" => ThresholdOperator::GreaterThan,
            "lt" | "less_than" => ThresholdOperator::LessThan,
            "gte" | "greater_than_or_equal" => ThresholdOperator::GreaterThanOrEqual,
            "lte" | "less_than_or_equal" => ThresholdOperator::LessThanOrEqual,
            _ => return Err(AppError::ValidationError("Invalid operator".to_string())),
        })
    } else {
        None
    };
    
    let update_threshold = UpdateAlertThreshold {
        threshold_value: request.value,
        is_enabled: request.is_enabled,
        operator,
    };
    
    let threshold = threshold_service.update_threshold(threshold_id, update_threshold).await?;
    
    let response = ThresholdResponse {
        id: threshold.id,
        user_id: Uuid::parse_str(&threshold.user_address).unwrap_or_else(|_| Uuid::new_v4()),
        position_id: threshold.position_id,
        protocol: None,
        threshold_type: threshold.threshold_type,
        operator: threshold.operator,
        value: threshold.threshold_value,
        severity: "medium".to_string(),
        is_enabled: threshold.is_enabled,
        notification_channels: vec![],
        triggered_count: 0,
        last_triggered: None,
        created_at: threshold.created_at,
        updated_at: threshold.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn delete_threshold(
    State(state): State<AppState>,
    Path(threshold_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    threshold_service.delete_threshold(threshold_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_thresholds(
    State(state): State<AppState>,
    Query(query): Query<GetThresholdsQuery>,
) -> Result<Json<Vec<ThresholdResponse>>, AppError> {
    let threshold_service = ThresholdService::new(state.db_pool.clone());
    
    // For now, just get user thresholds by user_address (convert UUID to string)
    let user_address = if let Some(user_id) = query.user_id {
        user_id.to_string()
    } else {
        return Err(AppError::ValidationError("user_id is required".to_string()));
    };
    
    let thresholds = threshold_service.get_user_thresholds(&user_address).await?;
    
    let responses: Vec<ThresholdResponse> = thresholds.into_iter().map(|threshold| {
        ThresholdResponse {
            id: threshold.id,
            user_id: Uuid::parse_str(&threshold.user_address).unwrap_or_else(|_| Uuid::new_v4()),
            position_id: threshold.position_id,
            protocol: None,
            threshold_type: threshold.threshold_type,
            operator: threshold.operator,
            value: threshold.threshold_value,
            severity: "medium".to_string(),
            is_enabled: threshold.is_enabled,
            notification_channels: vec![],
            triggered_count: 0,
            last_triggered: None,
            created_at: threshold.created_at,
            updated_at: threshold.updated_at,
        }
    }).collect();
    
    Ok(Json(responses))
}

pub async fn list_alerts(
    State(state): State<AppState>,
    Query(_query): Query<GetAlertsQuery>,
) -> Result<Json<Vec<AlertResponse>>, AppError> {
    let _alert_service = AlertService::new(&state.settings);
    
    // For now, return empty alerts list since the method doesn't exist yet
    let alerts: Vec<crate::models::Alert> = vec![];
    
    let responses: Vec<AlertResponse> = alerts.into_iter().map(|alert| {
        AlertResponse {
            id: alert.id,
            user_id: Uuid::parse_str(&alert.user_address).unwrap_or_else(|_| Uuid::new_v4()),
            position_id: alert.position_id,
            threshold_id: alert.threshold_id,
            alert_type: alert.alert_type,
            severity: alert.severity,
            title: alert.title,
            message: alert.message,
            current_value: alert.current_value,
            threshold_value: alert.threshold_value,
            metadata: alert.metadata,
            is_resolved: alert.is_resolved,
            resolved_at: alert.resolved_at,
            created_at: alert.created_at,
        }
    }).collect();
    
    Ok(Json(responses))
}

pub async fn resolve_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<Uuid>,
    Json(_request): Json<ResolveAlertRequest>,
) -> Result<Json<AlertResponse>, AppError> {
    let _alert_service = AlertService::new(&state.settings);
    
    // For now, return a mock resolved alert since the method doesn't exist yet
    use chrono::Utc;
    let alert = crate::models::Alert {
        id: alert_id,
        user_address: "mock_user".to_string(),
        position_id: None,
        threshold_id: None,
        alert_type: "mock_alert".to_string(),
        severity: "medium".to_string(),
        title: "Mock Alert".to_string(),
        message: "This is a mock alert".to_string(),
        risk_score: None,
        current_value: None,
        threshold_value: None,
        metadata: None,
        is_resolved: true,
        resolved_at: Some(Utc::now()),
        created_at: Utc::now(),
    };
    
    let response = AlertResponse {
        id: alert.id,
        user_id: Uuid::parse_str(&alert.user_address).unwrap_or_else(|_| Uuid::new_v4()),
        position_id: alert.position_id,
        threshold_id: alert.threshold_id,
        alert_type: alert.alert_type,
        severity: alert.severity,
        title: alert.title,
        message: alert.message,
        current_value: alert.current_value,
        threshold_value: alert.threshold_value,
        metadata: alert.metadata,
        is_resolved: alert.is_resolved,
        resolved_at: alert.resolved_at,
        created_at: alert.created_at,
    };
    
    Ok(Json(response))
}

pub async fn get_monitoring_stats(
    State(state): State<AppState>,
) -> Result<Json<MonitoringStatsResponse>, AppError> {
    let _monitoring_service = MonitoringService::new(state.db_pool.clone(), state.settings.clone())?;
    
    // For now, return mock stats since the method doesn't exist yet
    use bigdecimal::BigDecimal;
    use std::str::FromStr;
    use chrono::Utc;
    
    let stats = crate::services::monitoring_service::MonitoringStats {
        total_positions_monitored: 0,
        active_thresholds: 0,
        alerts_last_24h: 0,
        critical_alerts_active: 0,
        avg_response_time_ms: BigDecimal::from_str("0.0").unwrap(),
        uptime_percentage: BigDecimal::from_str("100.0").unwrap(),
        last_check: Utc::now(),
    };
    
    let response = MonitoringStatsResponse {
        total_positions_monitored: stats.total_positions_monitored,
        active_thresholds: stats.active_thresholds,
        alerts_last_24h: stats.alerts_last_24h,
        critical_alerts_active: stats.critical_alerts_active,
        avg_response_time_ms: stats.avg_response_time_ms,
        uptime_percentage: stats.uptime_percentage,
        last_check: stats.last_check,
    };
    
    Ok(Json(response))
}

pub async fn start_monitoring(
    State(state): State<AppState>,
) -> Result<StatusCode, AppError> {
    let monitoring_service = MonitoringService::new(state.db_pool.clone(), state.settings.clone())?;
    
    // Start monitoring in background (this would typically be done at startup)
    tokio::spawn(async move {
        if let Err(e) = monitoring_service.start_monitoring().await {
            tracing::error!("Monitoring service error: {}", e);
        }
    });
    
    Ok(StatusCode::ACCEPTED)
}

pub async fn stop_monitoring(
    State(_state): State<AppState>,
) -> Result<StatusCode, AppError> {
    // This would stop the monitoring service
    // Implementation depends on how you manage the monitoring lifecycle
    Ok(StatusCode::ACCEPTED)
}

// Create router
pub fn create_monitoring_routes() -> Router<AppState> {
    Router::new()
        // Threshold management
        .route("/monitoring/thresholds", post(create_threshold))
        .route("/monitoring/thresholds", get(list_thresholds))
        .route("/monitoring/thresholds/:threshold_id", get(get_threshold))
        .route("/monitoring/thresholds/:threshold_id", put(update_threshold))
        .route("/monitoring/thresholds/:threshold_id", delete(delete_threshold))
        
        // Alert management
        .route("/monitoring/alerts", get(list_alerts))
        .route("/monitoring/alerts/:alert_id/resolve", put(resolve_alert))
        
        // Monitoring control
        .route("/monitoring/stats", get(get_monitoring_stats))
        .route("/monitoring/start", post(start_monitoring))
        .route("/monitoring/stop", post(stop_monitoring))
}
