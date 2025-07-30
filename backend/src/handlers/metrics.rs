use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tracing::{info, error};

use crate::utils::monitoring::{get_metrics, HealthChecker, HealthStatus};
use crate::error::AppError;

/// Handler for Prometheus metrics endpoint
pub async fn metrics_handler() -> Result<Response, AppError> {
    info!("Serving Prometheus metrics");
    
    match get_metrics().await {
        Ok(metrics) => {
            let metrics_text = metrics.export_metrics()?;
            Ok((
                StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                metrics_text,
            ).into_response())
        }
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            Err(e)
        }
    }
}

/// Handler for health check endpoint
pub async fn health_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<Json<HealthStatus>, AppError> {
    info!("Performing health check");
    
    let health_status = health_checker.check_health().await;
    
    if health_status.healthy {
        info!("Health check passed");
    } else {
        error!("Health check failed: {:?}", health_status.checks);
    }
    
    Ok(Json(health_status))
}

/// Handler for readiness probe (Kubernetes-style)
pub async fn readiness_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<Response, AppError> {
    let health_status = health_checker.check_health().await;
    
    if health_status.healthy {
        Ok((
            StatusCode::OK,
            Json(json!({
                "status": "ready",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        ).into_response())
    } else {
        Ok((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "not_ready",
                "failed_checks": health_status.checks.iter()
                    .filter(|(_, &healthy)| !healthy)
                    .map(|(name, _)| name)
                    .collect::<Vec<_>>(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        ).into_response())
    }
}

/// Handler for liveness probe (Kubernetes-style)
pub async fn liveness_handler() -> Result<Response, AppError> {
    // Simple liveness check - if we can respond, we're alive
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "alive",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    ).into_response())
}

/// Handler for detailed system information
pub async fn system_info_handler(
    State(health_checker): State<Arc<HealthChecker>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let health_status = health_checker.check_health().await;
    
    let system_info = json!({
        "service": "defi-risk-monitor",
        "version": health_status.version,
        "uptime_seconds": health_status.uptime.as_secs(),
        "health": health_status,
        "build_info": {
            "rust_version": std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string()),
            "build_timestamp": std::env::var("BUILD_TIMESTAMP").unwrap_or_else(|_| chrono::Utc::now().to_rfc3339()),
            "git_commit": std::env::var("GIT_COMMIT").unwrap_or_else(|_| "unknown".to_string()),
        },
        "runtime_info": {
            "tokio_version": "1.0",
            "axum_version": "0.7",
        }
    });
    
    Ok(Json(system_info))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_liveness_handler() {
        let response = liveness_handler().await;
        assert!(response.is_ok());
    }
    
    #[tokio::test]
    async fn test_health_handler() {
        let health_checker = Arc::new(HealthChecker::new("1.0.0-test"));
        let response = health_handler(State(health_checker)).await;
        assert!(response.is_ok());
    }
}
