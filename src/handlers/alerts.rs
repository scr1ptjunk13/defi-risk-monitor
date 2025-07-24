use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use crate::models::{Alert, CreateAlert};

#[derive(Serialize, Deserialize)]
pub struct AlertsResponse {
    pub alerts: Vec<Alert>,
    pub total: usize,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAlertRequest {
    pub alert: CreateAlert,
}

#[derive(Serialize, Deserialize)]
pub struct CreateAlertResponse {
    pub alert: Alert,
}

pub async fn list_alerts(
    State(pool): State<PgPool>,
) -> Result<Json<AlertsResponse>, StatusCode> {
    let alerts = sqlx::query_as::<_, Alert>(
        "SELECT * FROM alerts ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total = alerts.len();

    Ok(Json(AlertsResponse { alerts, total }))
}

pub async fn create_alert(
    State(pool): State<PgPool>,
    Json(request): Json<CreateAlertRequest>,
) -> Result<Json<CreateAlertResponse>, StatusCode> {
    let alert = Alert::new(request.alert);

    sqlx::query!(
        r#"
        INSERT INTO alerts (
            id, position_id, alert_type, severity, title, message,
            risk_score, current_value, threshold_value, is_resolved,
            resolved_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#,
        alert.id,
        alert.position_id,
        alert.alert_type,
        alert.severity,
        alert.title,
        alert.message,
        alert.risk_score,
        alert.current_value,
        alert.threshold_value,
        alert.is_resolved,
        alert.resolved_at,
        alert.created_at
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(CreateAlertResponse { alert }))
}
