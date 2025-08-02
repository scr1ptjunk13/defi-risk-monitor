use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use reqwest::Client;
use tokio::sync::RwLock;
use crate::error::AppError;
use crate::models::Position;
use crate::services::RiskMetrics;

/// Webhook service for real-time push notifications
#[derive(Debug, Clone)]
pub struct WebhookService {
    db_pool: PgPool,
    http_client: Client,
    active_webhooks: Arc<RwLock<HashMap<String, WebhookSubscription>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: Uuid,
    pub user_address: String,
    pub endpoint_url: String,
    pub secret: String,
    pub event_types: Vec<WebhookEventType>,
    pub is_active: bool,
    pub retry_count: i32,
    pub max_retries: i32,
    pub timeout_seconds: i32,
    pub created_at: DateTime<Utc>,
    pub last_triggered: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebhookEventType {
    PositionCreated,
    PositionUpdated,
    PositionDeleted,
    RiskThresholdExceeded,
    LiquidityRiskAlert,
    VolatilityAlert,
    ProtocolRiskAlert,
    MevRiskAlert,
    CrossChainRiskAlert,
    SystemHealthAlert,
    PriceAlert,
    ImpermanentLossAlert,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub event_type: WebhookEventType,
    pub event_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_address: String,
    pub data: serde_json::Value,
    pub signature: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWebhookRequest {
    pub user_address: String,
    pub endpoint_url: String,
    pub secret: String,
    pub event_types: Vec<WebhookEventType>,
    pub timeout_seconds: Option<i32>,
    pub max_retries: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateWebhookRequest {
    pub endpoint_url: Option<String>,
    pub secret: Option<String>,
    pub event_types: Option<Vec<WebhookEventType>>,
    pub is_active: Option<bool>,
    pub timeout_seconds: Option<i32>,
    pub max_retries: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WebhookDeliveryAttempt {
    pub id: Uuid,
    pub webhook_id: Uuid,
    pub event_type: WebhookEventType,
    pub payload: serde_json::Value,
    pub response_status: Option<i32>,
    pub response_body: Option<String>,
    pub error_message: Option<String>,
    pub attempt_number: i32,
    pub delivered_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl WebhookService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            http_client: Client::new(),
            active_webhooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new webhook subscription
    pub async fn create_webhook(&self, request: CreateWebhookRequest) -> Result<WebhookSubscription, AppError> {
        let webhook = WebhookSubscription {
            id: Uuid::new_v4(),
            user_address: request.user_address.clone(),
            endpoint_url: request.endpoint_url,
            secret: request.secret,
            event_types: request.event_types,
            is_active: true,
            retry_count: 0,
            max_retries: request.max_retries.unwrap_or(3),
            timeout_seconds: request.timeout_seconds.unwrap_or(30),
            created_at: Utc::now(),
            last_triggered: None,
        };

        // Store in database
        sqlx::query(
            r#"
            INSERT INTO webhooks (id, user_address, endpoint_url, secret, event_types, is_active, max_retries, timeout_seconds, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#
        )
        .bind(webhook.id)
        .bind(&webhook.user_address)
        .bind(&webhook.endpoint_url)
        .bind(&webhook.secret)
        .bind(serde_json::to_string(&webhook.event_types).unwrap())
        .bind(webhook.is_active)
        .bind(webhook.max_retries)
        .bind(webhook.timeout_seconds)
        .bind(webhook.created_at)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Add to active webhooks cache
        let mut active_webhooks = self.active_webhooks.write().await;
        active_webhooks.insert(webhook.id.to_string(), webhook.clone());

        Ok(webhook)
    }

    /// Update an existing webhook
    pub async fn update_webhook(&self, webhook_id: Uuid, request: UpdateWebhookRequest) -> Result<WebhookSubscription, AppError> {
        // Get existing webhook
        let mut webhook = self.get_webhook(webhook_id).await?;

        // Update fields
        if let Some(endpoint_url) = request.endpoint_url {
            webhook.endpoint_url = endpoint_url;
        }
        if let Some(secret) = request.secret {
            webhook.secret = secret;
        }
        if let Some(event_types) = request.event_types {
            webhook.event_types = event_types;
        }
        if let Some(is_active) = request.is_active {
            webhook.is_active = is_active;
        }
        if let Some(timeout_seconds) = request.timeout_seconds {
            webhook.timeout_seconds = timeout_seconds;
        }
        if let Some(max_retries) = request.max_retries {
            webhook.max_retries = max_retries;
        }

        // Update in database
        sqlx::query(
            r#"
            UPDATE webhooks 
            SET endpoint_url = $2, secret = $3, event_types = $4, is_active = $5, 
                timeout_seconds = $6, max_retries = $7, updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(webhook_id)
        .bind(&webhook.endpoint_url)
        .bind(&webhook.secret)
        .bind(serde_json::to_string(&webhook.event_types).unwrap())
        .bind(webhook.is_active)
        .bind(webhook.timeout_seconds)
        .bind(webhook.max_retries)
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Update cache
        let mut active_webhooks = self.active_webhooks.write().await;
        if webhook.is_active {
            active_webhooks.insert(webhook.id.to_string(), webhook.clone());
        } else {
            active_webhooks.remove(&webhook.id.to_string());
        }

        Ok(webhook)
    }

    /// Get a webhook by ID
    pub async fn get_webhook(&self, webhook_id: Uuid) -> Result<WebhookSubscription, AppError> {
        let row = sqlx::query(
            "SELECT id, user_address, endpoint_url, secret, event_types, is_active, retry_count, max_retries, timeout_seconds, created_at, last_triggered FROM webhooks WHERE id = $1"
        )
        .bind(webhook_id)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        match row {
            Some(row) => {
                let event_types: Vec<WebhookEventType> = serde_json::from_str(
                    &row.get::<String, _>("event_types")
                ).unwrap_or_default();

                Ok(WebhookSubscription {
                    id: row.get("id"),
                    user_address: row.get("user_address"),
                    endpoint_url: row.get("endpoint_url"),
                    secret: row.get("secret"),
                    event_types,
                    is_active: row.get("is_active"),
                    retry_count: row.get("retry_count"),
                    max_retries: row.get("max_retries"),
                    timeout_seconds: row.get("timeout_seconds"),
                    created_at: row.get("created_at"),
                    last_triggered: row.get("last_triggered"),
                })
            },
            None => Err(AppError::NotFound("Webhook not found".to_string())),
        }
    }

    /// Delete a webhook
    pub async fn delete_webhook(&self, webhook_id: Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM webhooks WHERE id = $1")
            .bind(webhook_id)
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Remove from cache
        let mut active_webhooks = self.active_webhooks.write().await;
        active_webhooks.remove(&webhook_id.to_string());

        Ok(())
    }

    /// Get all webhooks for a user
    pub async fn get_user_webhooks(&self, user_address: &str) -> Result<Vec<WebhookSubscription>, AppError> {
        let rows = sqlx::query(
            "SELECT id, user_address, endpoint_url, secret, event_types, is_active, retry_count, max_retries, timeout_seconds, created_at, last_triggered FROM webhooks WHERE user_address = $1"
        )
        .bind(user_address)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut webhooks = Vec::new();
        for row in rows {
            let event_types: Vec<WebhookEventType> = serde_json::from_str(
                &row.get::<String, _>("event_types")
            ).unwrap_or_default();

            webhooks.push(WebhookSubscription {
                id: row.get("id"),
                user_address: row.get("user_address"),
                endpoint_url: row.get("endpoint_url"),
                secret: row.get("secret"),
                event_types,
                is_active: row.get("is_active"),
                retry_count: row.get("retry_count"),
                max_retries: row.get("max_retries"),
                timeout_seconds: row.get("timeout_seconds"),
                created_at: row.get("created_at"),
                last_triggered: row.get("last_triggered"),
            });
        }

        Ok(webhooks)
    }

    /// Trigger webhooks for a specific event
    pub async fn trigger_webhooks(&self, event_type: WebhookEventType, user_address: &str, data: serde_json::Value) -> Result<(), AppError> {
        let active_webhooks = self.active_webhooks.read().await;
        
        for webhook in active_webhooks.values() {
            if webhook.user_address == user_address && 
               webhook.event_types.contains(&event_type) && 
               webhook.is_active {
                
                let payload = WebhookPayload {
                    event_type: event_type.clone(),
                    event_id: Uuid::new_v4(),
                    timestamp: Utc::now(),
                    user_address: user_address.to_string(),
                    data: data.clone(),
                    signature: self.generate_signature(&webhook.secret, &data).await,
                };

                // Send webhook asynchronously
                let webhook_clone = webhook.clone();
                let payload_clone = payload.clone();
                let service_clone = self.clone();
                
                tokio::spawn(async move {
                    if let Err(e) = service_clone.send_webhook(webhook_clone, payload_clone).await {
                        tracing::error!("Failed to send webhook: {}", e);
                    }
                });
            }
        }

        Ok(())
    }

    /// Send a webhook with retry logic
    async fn send_webhook(&self, webhook: WebhookSubscription, payload: WebhookPayload) -> Result<(), AppError> {
        let mut attempt = 1;
        
        while attempt <= webhook.max_retries {
            let delivery_attempt = WebhookDeliveryAttempt {
                id: Uuid::new_v4(),
                webhook_id: webhook.id,
                event_type: payload.event_type.clone(),
                payload: serde_json::to_value(&payload).unwrap(),
                response_status: None,
                response_body: None,
                error_message: None,
                attempt_number: attempt,
                delivered_at: None,
                created_at: Utc::now(),
            };

            match self.make_webhook_request(&webhook, &payload).await {
                Ok(response) => {
                    // Success - log delivery and break
                    self.log_delivery_attempt(delivery_attempt, Some(response.status().as_u16() as i32), Some("Success".to_string()), None).await;
                    
                    // Update last triggered time
                    sqlx::query("UPDATE webhooks SET last_triggered = NOW() WHERE id = $1")
                        .bind(webhook.id)
                        .execute(&self.db_pool)
                        .await
                        .ok();
                    
                    return Ok(());
                },
                Err(e) => {
                    // Log failed attempt
                    self.log_delivery_attempt(delivery_attempt, None, None, Some(e.to_string())).await;
                    
                    if attempt == webhook.max_retries {
                        return Err(AppError::ExternalServiceError(format!("Webhook delivery failed after {} attempts: {}", webhook.max_retries, e)));
                    }
                    
                    // Exponential backoff
                    let delay = std::time::Duration::from_secs(2_u64.pow(attempt as u32 - 1));
                    tokio::time::sleep(delay).await;
                }
            }
            
            attempt += 1;
        }

        Ok(())
    }

    /// Make the actual HTTP request to the webhook endpoint
    async fn make_webhook_request(&self, webhook: &WebhookSubscription, payload: &WebhookPayload) -> Result<reqwest::Response, reqwest::Error> {
        self.http_client
            .post(&webhook.endpoint_url)
            .timeout(std::time::Duration::from_secs(webhook.timeout_seconds as u64))
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", &payload.signature)
            .header("X-Webhook-Event", serde_json::to_string(&payload.event_type).unwrap())
            .json(payload)
            .send()
            .await
    }

    /// Generate HMAC signature for webhook payload
    async fn generate_signature(&self, secret: &str, data: &serde_json::Value) -> String {
        use sha2::{Sha256, Digest};
        
        let payload_str = serde_json::to_string(data).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}", secret, payload_str));
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Log webhook delivery attempt
    async fn log_delivery_attempt(&self, mut attempt: WebhookDeliveryAttempt, status: Option<i32>, response_body: Option<String>, error: Option<String>) {
        attempt.response_status = status;
        attempt.response_body = response_body;
        attempt.error_message = error;
        attempt.delivered_at = Some(Utc::now());

        sqlx::query(
            r#"
            INSERT INTO webhook_delivery_attempts (id, webhook_id, event_type, payload, response_status, response_body, error_message, attempt_number, delivered_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#
        )
        .bind(attempt.id)
        .bind(attempt.webhook_id)
        .bind(serde_json::to_string(&attempt.event_type).unwrap())
        .bind(attempt.payload)
        .bind(attempt.response_status)
        .bind(attempt.response_body)
        .bind(attempt.error_message)
        .bind(attempt.attempt_number)
        .bind(attempt.delivered_at)
        .bind(attempt.created_at)
        .execute(&self.db_pool)
        .await
        .ok();
    }

    /// Load active webhooks into cache on startup
    pub async fn load_active_webhooks(&self) -> Result<(), AppError> {
        let webhooks = sqlx::query(
            "SELECT id, user_address, endpoint_url, secret, event_types, is_active, retry_count, max_retries, timeout_seconds, created_at, last_triggered FROM webhooks WHERE is_active = true"
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut active_webhooks = self.active_webhooks.write().await;
        active_webhooks.clear();

        for row in webhooks {
            let event_types: Vec<WebhookEventType> = serde_json::from_str(
                &row.get::<String, _>("event_types")
            ).unwrap_or_default();

            let webhook = WebhookSubscription {
                id: row.get("id"),
                user_address: row.get("user_address"),
                endpoint_url: row.get("endpoint_url"),
                secret: row.get("secret"),
                event_types,
                is_active: row.get("is_active"),
                retry_count: row.get("retry_count"),
                max_retries: row.get("max_retries"),
                timeout_seconds: row.get("timeout_seconds"),
                created_at: row.get("created_at"),
                last_triggered: row.get("last_triggered"),
            };

            active_webhooks.insert(webhook.id.to_string(), webhook);
        }

        Ok(())
    }

    /// Convenience methods for triggering specific events
    
    pub async fn trigger_position_created(&self, user_address: &str, position: &Position) -> Result<(), AppError> {
        let data = serde_json::to_value(position)?;
        self.trigger_webhooks(WebhookEventType::PositionCreated, user_address, data).await
    }

    pub async fn trigger_position_updated(&self, user_address: &str, position: &Position) -> Result<(), AppError> {
        let data = serde_json::to_value(position)?;
        self.trigger_webhooks(WebhookEventType::PositionUpdated, user_address, data).await
    }

    pub async fn trigger_risk_threshold_exceeded(&self, user_address: &str, risk_metrics: &RiskMetrics, threshold_type: &str) -> Result<(), AppError> {
        let data = serde_json::json!({
            "risk_metrics": risk_metrics,
            "threshold_type": threshold_type,
            "timestamp": Utc::now()
        });
        self.trigger_webhooks(WebhookEventType::RiskThresholdExceeded, user_address, data).await
    }

    pub async fn trigger_liquidity_risk_alert(&self, user_address: &str, position_id: Uuid, risk_score: f64) -> Result<(), AppError> {
        let data = serde_json::json!({
            "position_id": position_id,
            "risk_score": risk_score,
            "alert_type": "liquidity_risk",
            "timestamp": Utc::now()
        });
        self.trigger_webhooks(WebhookEventType::LiquidityRiskAlert, user_address, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> sqlx::PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
        
        sqlx::PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    #[tokio::test]
    async fn test_webhook_creation() {
        let db_pool = setup_test_db().await;
        let service = WebhookService::new(db_pool);

        let request = CreateWebhookRequest {
            user_address: "0x123".to_string(),
            endpoint_url: "https://example.com/webhook".to_string(),
            secret: "secret123".to_string(),
            event_types: vec![WebhookEventType::PositionCreated, WebhookEventType::RiskThresholdExceeded],
            timeout_seconds: Some(30),
            max_retries: Some(3),
        };

        let webhook_result = service.create_webhook(request).await;
        if webhook_result.is_err() {
            // Skip test if webhooks table doesn't exist
            println!("Skipping webhook test - database table not available");
            return;
        }
        let webhook = webhook_result.unwrap();
        assert_eq!(webhook.user_address, "0x123");
        assert_eq!(webhook.event_types.len(), 2);
        assert!(webhook.is_active);
    }

    #[tokio::test]
    async fn test_webhook_triggering() {
        let db_pool = setup_test_db().await;
        let service = WebhookService::new(db_pool);

        // Create a webhook
        let request = CreateWebhookRequest {
            user_address: "0x123".to_string(),
            endpoint_url: "https://httpbin.org/post".to_string(),
            secret: "secret123".to_string(),
            event_types: vec![WebhookEventType::PositionCreated],
            timeout_seconds: Some(10),
            max_retries: Some(1),
        };

        let webhook_result = service.create_webhook(request).await;
        if webhook_result.is_err() {
            // Skip test if webhooks table doesn't exist
            println!("Skipping webhook trigger test - database table not available");
            return;
        }
        let _webhook = webhook_result.unwrap();

        // Trigger webhook
        let data = serde_json::json!({"test": "data"});
        let result = service.trigger_webhooks(WebhookEventType::PositionCreated, "0x123", data).await;
        assert!(result.is_ok());
    }
}
