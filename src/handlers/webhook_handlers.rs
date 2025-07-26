use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;
use crate::services::webhook_service::{
    WebhookService, CreateWebhookRequest, UpdateWebhookRequest, 
    WebhookSubscription, WebhookEventType, WebhookDeliveryAttempt
};
use crate::AppState;

/// Request/Response DTOs for Webhook API

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWebhookResponse {
    pub webhook: WebhookSubscription,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetWebhooksQuery {
    pub user_address: String,
    pub event_type: Option<WebhookEventType>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookListResponse {
    pub webhooks: Vec<WebhookSubscription>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookStatsResponse {
    pub total_webhooks: i64,
    pub active_webhooks: i64,
    pub total_deliveries: i64,
    pub successful_deliveries: i64,
    pub failed_deliveries: i64,
    pub event_type_distribution: HashMap<String, i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestWebhookRequest {
    pub event_type: WebhookEventType,
    pub test_data: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

/// Create a new webhook subscription
/// POST /api/v1/webhooks
pub async fn create_webhook(
    State(state): State<AppState>,
    Json(request): Json<CreateWebhookRequest>,
) -> Result<Json<ApiResponse<CreateWebhookResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    match service.create_webhook(request).await {
        Ok(webhook) => Ok(Json(ApiResponse {
            success: true,
            data: Some(CreateWebhookResponse { webhook }),
            message: Some("Webhook created successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to create webhook".to_string()),
        })))
    }
}

/// Get webhooks for a user
/// GET /api/v1/webhooks
pub async fn get_webhooks(
    State(state): State<AppState>,
    Query(query): Query<GetWebhooksQuery>,
) -> Result<Json<ApiResponse<WebhookListResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    match service.get_user_webhooks(&query.user_address).await {
        Ok(mut webhooks) => {
            // Apply filters
            if let Some(event_type) = query.event_type {
                webhooks.retain(|w| w.event_types.contains(&event_type));
            }
            
            if let Some(is_active) = query.is_active {
                webhooks.retain(|w| w.is_active == is_active);
            }
            
            let total = webhooks.len();
            
            Ok(Json(ApiResponse {
                success: true,
                data: Some(WebhookListResponse { webhooks, total }),
                message: None,
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to fetch webhooks".to_string()),
        })))
    }
}

/// Get a specific webhook by ID
/// GET /api/v1/webhooks/{id}
pub async fn get_webhook(
    Path(webhook_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<WebhookSubscription>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    match service.get_webhook(webhook_id).await {
        Ok(webhook) => Ok(Json(ApiResponse {
            success: true,
            data: Some(webhook),
            message: None,
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Webhook not found".to_string()),
        })))
    }
}

/// Update a webhook
/// PUT /api/v1/webhooks/{id}
pub async fn update_webhook(
    Path(webhook_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateWebhookRequest>,
) -> Result<Json<ApiResponse<WebhookSubscription>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    match service.update_webhook(webhook_id, request).await {
        Ok(webhook) => Ok(Json(ApiResponse {
            success: true,
            data: Some(webhook),
            message: Some("Webhook updated successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Webhook not found or failed to update".to_string()),
        })))
    }
}

/// Delete a webhook
/// DELETE /api/v1/webhooks/{id}
pub async fn delete_webhook(
    Path(webhook_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    match service.delete_webhook(webhook_id).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: Some("Webhook deleted successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Webhook not found".to_string()),
        })))
    }
}

/// Test a webhook by sending a test payload
/// POST /api/v1/webhooks/{id}/test
pub async fn test_webhook(
    Path(webhook_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<TestWebhookRequest>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = WebhookService::new(state.db_pool.clone());
    
    // Get the webhook to get user address
    let webhook = match service.get_webhook(webhook_id).await {
        Ok(webhook) => webhook,
        Err(_) => return Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Webhook not found".to_string()),
        })))
    };
    
    // Trigger test webhook
    match service.trigger_webhooks(request.event_type, &webhook.user_address, request.test_data).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: Some("Test webhook sent successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to send test webhook".to_string()),
        })))
    }
}

/// Get webhook delivery history
/// GET /api/v1/webhooks/{id}/deliveries
pub async fn get_webhook_deliveries(
    Path(webhook_id): Path<Uuid>,
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<WebhookDeliveryAttempt>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let limit = query.get("limit")
        .and_then(|l| l.parse::<i32>().ok())
        .unwrap_or(50)
        .min(100);
    
    let offset = query.get("offset")
        .and_then(|o| o.parse::<i32>().ok())
        .unwrap_or(0);
    
    // Query delivery attempts from database
    let rows = sqlx::query(
        r#"
        SELECT id, webhook_id, event_type, payload, response_status, response_body, 
               error_message, attempt_number, delivered_at, created_at
        FROM webhook_delivery_attempts 
        WHERE webhook_id = $1 
        ORDER BY created_at DESC 
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(webhook_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.db_pool)
    .await;
    
    match rows {
        Ok(rows) => {
            let mut deliveries = Vec::new();
            for row in rows {
                let event_type: WebhookEventType = serde_json::from_str(
                    &row.get::<String, _>("event_type")
                ).unwrap_or(WebhookEventType::PositionCreated);
                
                deliveries.push(WebhookDeliveryAttempt {
                    id: row.get("id"),
                    webhook_id: row.get("webhook_id"),
                    event_type,
                    payload: row.get("payload"),
                    response_status: row.get("response_status"),
                    response_body: row.get("response_body"),
                    error_message: row.get("error_message"),
                    attempt_number: row.get("attempt_number"),
                    delivered_at: row.get("delivered_at"),
                    created_at: row.get("created_at"),
                });
            }
            
            Ok(Json(ApiResponse {
                success: true,
                data: Some(deliveries),
                message: None,
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to fetch delivery history".to_string()),
        })))
    }
}

/// Get webhook statistics
/// GET /api/v1/webhooks/stats
pub async fn get_webhook_stats(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<WebhookStatsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_address = query.get("user_address");
    
    // Get basic webhook counts
    let total_webhooks = if let Some(addr) = user_address {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhooks WHERE user_address = $1"
        )
        .bind(addr)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0)
    } else {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM webhooks")
            .fetch_one(&state.db_pool)
            .await
            .unwrap_or(0)
    };
    
    let active_webhooks = if let Some(addr) = user_address {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM webhooks WHERE user_address = $1 AND is_active = true"
        )
        .bind(addr)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0)
    } else {
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM webhooks WHERE is_active = true")
            .fetch_one(&state.db_pool)
            .await
            .unwrap_or(0)
    };
    
    // For now, return basic stats
    let stats = WebhookStatsResponse {
        total_webhooks,
        active_webhooks,
        total_deliveries: 0,      // Would query webhook_delivery_attempts table
        successful_deliveries: 0, // Would count successful deliveries
        failed_deliveries: 0,     // Would count failed deliveries
        event_type_distribution: HashMap::new(), // Would aggregate by event type
    };
    
    Ok(Json(ApiResponse {
        success: true,
        data: Some(stats),
        message: None,
    }))
}

/// Get available webhook event types
/// GET /api/v1/webhooks/event-types
pub async fn get_webhook_event_types() -> Json<ApiResponse<Vec<WebhookEventType>>> {
    let event_types = vec![
        WebhookEventType::PositionCreated,
        WebhookEventType::PositionUpdated,
        WebhookEventType::PositionDeleted,
        WebhookEventType::RiskThresholdExceeded,
        WebhookEventType::LiquidityRiskAlert,
        WebhookEventType::VolatilityAlert,
        WebhookEventType::ProtocolRiskAlert,
        WebhookEventType::MevRiskAlert,
        WebhookEventType::CrossChainRiskAlert,
        WebhookEventType::SystemHealthAlert,
        WebhookEventType::PriceAlert,
        WebhookEventType::ImpermanentLossAlert,
    ];
    
    Json(ApiResponse {
        success: true,
        data: Some(event_types),
        message: None,
    })
}

/// Create router for webhook endpoints
pub fn create_webhook_routes() -> Router<AppState> {
    Router::new()
        .route("/webhooks", post(create_webhook))
        .route("/webhooks", get(get_webhooks))
        .route("/webhooks/stats", get(get_webhook_stats))
        .route("/webhooks/event-types", get(get_webhook_event_types))
        .route("/webhooks/:id", get(get_webhook))
        .route("/webhooks/:id", put(update_webhook))
        .route("/webhooks/:id", delete(delete_webhook))
        .route("/webhooks/:id/test", post(test_webhook))
        .route("/webhooks/:id/deliveries", get(get_webhook_deliveries))
}
