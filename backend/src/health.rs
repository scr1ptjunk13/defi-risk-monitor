use axum::{response::Json, http::StatusCode};
use serde_json;

/// Simple health check endpoint
pub async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "defi-risk-monitor",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": "1.0.0"
    })))
}
