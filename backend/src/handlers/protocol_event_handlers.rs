use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use tracing::info;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;

use crate::models::protocol_events::*;
use crate::error::AppError;
use crate::AppState;

/// Query parameters for protocol events
#[derive(Debug, Deserialize)]
pub struct ProtocolEventQuery {
    pub protocol_name: Option<String>,
    pub event_type: Option<String>,
    pub severity: Option<String>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

/// Query parameters for event alerts
#[derive(Debug, Deserialize)]
pub struct EventAlertQuery {
    pub user_address: String,
    pub protocol_name: Option<String>,
}

/// Request body for creating event alert
#[derive(Debug, Deserialize)]
pub struct CreateEventAlertRequest {
    pub user_address: String,
    pub protocol_name: String,
    pub event_types: Vec<String>,
    pub min_severity: String,
    pub notification_channels: Vec<String>,
}

/// Response for protocol events list
#[derive(Debug, Serialize)]
pub struct ProtocolEventsResponse {
    pub events: Vec<ProtocolEventWithDetails>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
}

/// Protocol event with additional details
#[derive(Debug, Serialize)]
pub struct ProtocolEventWithDetails {
    pub id: Uuid,
    pub protocol_name: String,
    pub event_type: EventType,
    pub severity: EventSeverity,
    pub title: String,
    pub description: String,
    pub source: String,
    pub source_url: Option<String>,
    pub impact_score: BigDecimal,
    pub affected_chains: Vec<i32>,
    pub affected_tokens: Vec<String>,
    pub event_timestamp: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
    pub exploit_details: Option<ExploitEvent>,
    pub governance_details: Option<GovernanceEvent>,
    pub audit_details: Option<AuditEvent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SeverityCount {
    pub severity: String,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtocolCount {
    pub protocol_name: String,
    pub count: i64,
}

/// Get protocol events with filtering and pagination
/// GET /api/v1/protocol-events
pub async fn get_protocol_events(
    Query(params): Query<ProtocolEventQuery>,
    State(state): State<AppState>,
) -> Result<Json<ProtocolEventsResponse>, AppError> {
    info!("Fetching protocol events with filters: {:?}", params);

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;

    // Build dynamic query based on filters
    let mut query = String::from(
        "SELECT pe.*, ee.*, ge.*, ae.* FROM protocol_events pe 
         LEFT JOIN exploit_events ee ON pe.id = ee.protocol_event_id
         LEFT JOIN governance_events ge ON pe.id = ge.protocol_event_id  
         LEFT JOIN audit_events ae ON pe.id = ae.protocol_event_id
         WHERE 1=1"
    );
    
    let mut bind_count = 0;
    
    if let Some(_protocol) = &params.protocol_name {
        bind_count += 1;
        query.push_str(&format!(" AND pe.protocol_name = ${}", bind_count));
    }
    
    if let Some(_event_type) = &params.event_type {
        bind_count += 1;
        query.push_str(&format!(" AND pe.event_type = ${}", bind_count));
    }
    
    if let Some(_severity) = &params.severity {
        bind_count += 1;
        query.push_str(&format!(" AND pe.severity = ${}", bind_count));
    }
    
    if let Some(_from_date) = &params.from_date {
        bind_count += 1;
        query.push_str(&format!(" AND pe.event_timestamp >= ${}", bind_count));
    }
    
    if let Some(_to_date) = &params.to_date {
        bind_count += 1;
        query.push_str(&format!(" AND pe.event_timestamp <= ${}", bind_count));
    }

    query.push_str(" ORDER BY pe.event_timestamp DESC");
    query.push_str(&format!(" LIMIT ${} OFFSET ${}", bind_count + 1, bind_count + 2));

    // For simplicity, using a basic query - in production you'd use sqlx::QueryBuilder
    let events_query = sqlx::query_as::<_, ProtocolEvent>(
        "SELECT * FROM protocol_events WHERE ($1::text IS NULL OR protocol_name = $1)
         AND ($2::text IS NULL OR event_type::text = $2)
         AND ($3::text IS NULL OR severity::text = $3)
         AND ($4::timestamptz IS NULL OR event_timestamp >= $4)
         AND ($5::timestamptz IS NULL OR event_timestamp <= $5)
         ORDER BY event_timestamp DESC LIMIT $6 OFFSET $7"
    )
    .bind(&params.protocol_name)
    .bind(&params.event_type)
    .bind(&params.severity)
    .bind(&params.from_date)
    .bind(&params.to_date)
    .bind(per_page as i64)
    .bind(offset as i64);

    let events = events_query.fetch_all(&state.db_pool).await?;

    // Get total count for pagination
    let total_query = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM protocol_events WHERE ($1::text IS NULL OR protocol_name = $1)
         AND ($2::text IS NULL OR event_type::text = $2)
         AND ($3::text IS NULL OR severity::text = $3)
         AND ($4::timestamptz IS NULL OR event_timestamp >= $4)
         AND ($5::timestamptz IS NULL OR event_timestamp <= $5)"
    )
    .bind(&params.protocol_name)
    .bind(&params.event_type)
    .bind(&params.severity)
    .bind(&params.from_date)
    .bind(&params.to_date);

    let total = total_query.fetch_one(&state.db_pool).await?;

    // Convert to response format with details
    let mut events_with_details = Vec::new();
    for event in events {
        let mut event_detail = ProtocolEventWithDetails {
            id: event.id,
            protocol_name: event.protocol_name,
            event_type: event.event_type.clone(),
            severity: event.severity,
            title: event.title,
            description: event.description,
            source: event.source,
            source_url: event.source_url,
            impact_score: event.impact_score,
            affected_chains: event.affected_chains,
            affected_tokens: event.affected_tokens,
            event_timestamp: event.event_timestamp,
            detected_at: event.detected_at,
            exploit_details: None,
            governance_details: None,
            audit_details: None,
        };

        // Fetch additional details based on event type
        match event.event_type {
            EventType::Exploit => {
                if let Ok(Some(exploit)) = sqlx::query_as::<_, ExploitEvent>(
                    "SELECT * FROM exploit_events WHERE protocol_event_id = $1"
                )
                .bind(event.id)
                .fetch_optional(&state.db_pool)
                .await
                {
                    event_detail.exploit_details = Some(exploit);
                }
            }
            EventType::Governance => {
                if let Ok(Some(governance)) = sqlx::query_as::<_, GovernanceEvent>(
                    "SELECT * FROM governance_events WHERE protocol_event_id = $1"
                )
                .bind(event.id)
                .fetch_optional(&state.db_pool)
                .await
                {
                    event_detail.governance_details = Some(governance);
                }
            }
            EventType::Audit => {
                if let Ok(Some(audit)) = sqlx::query_as::<_, AuditEvent>(
                    "SELECT * FROM audit_events WHERE protocol_event_id = $1"
                )
                .bind(event.id)
                .fetch_optional(&state.db_pool)
                .await
                {
                    event_detail.audit_details = Some(audit);
                }
            }
            _ => {}
        }

        events_with_details.push(event_detail);
    }

    Ok(Json(ProtocolEventsResponse {
        events: events_with_details,
        total,
        page,
        per_page,
    }))
}

/// Get specific protocol event by ID
/// GET /api/v1/protocol-events/{id}
pub async fn get_protocol_event(
    Path(event_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<ProtocolEventWithDetails>, AppError> {
    info!("Fetching protocol event: {}", event_id);

    let event = sqlx::query_as::<_, ProtocolEvent>(
        "SELECT * FROM protocol_events WHERE id = $1"
    )
    .bind(event_id)
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Protocol event not found".to_string()))?;

    let mut event_detail = ProtocolEventWithDetails {
        id: event.id,
        protocol_name: event.protocol_name,
        event_type: event.event_type.clone(),
        severity: event.severity,
        title: event.title,
        description: event.description,
        source: event.source,
        source_url: event.source_url,
        impact_score: event.impact_score,
        affected_chains: event.affected_chains,
        affected_tokens: event.affected_tokens,
        event_timestamp: event.event_timestamp,
        detected_at: event.detected_at,
        exploit_details: None,
        governance_details: None,
        audit_details: None,
    };

    // Fetch additional details based on event type
    match event.event_type {
        EventType::Exploit => {
            event_detail.exploit_details = sqlx::query_as::<_, ExploitEvent>(
                "SELECT * FROM exploit_events WHERE protocol_event_id = $1"
            )
            .bind(event.id)
            .fetch_optional(&state.db_pool)
            .await?;
        }
        EventType::Governance => {
            event_detail.governance_details = sqlx::query_as::<_, GovernanceEvent>(
                "SELECT * FROM governance_events WHERE protocol_event_id = $1"
            )
            .bind(event.id)
            .fetch_optional(&state.db_pool)
            .await?;
        }
        EventType::Audit => {
            event_detail.audit_details = sqlx::query_as::<_, AuditEvent>(
                "SELECT * FROM audit_events WHERE protocol_event_id = $1"
            )
            .bind(event.id)
            .fetch_optional(&state.db_pool)
            .await?;
        }
        _ => {}
    }

    Ok(Json(event_detail))
}

/// Get protocol event statistics
/// GET /api/v1/protocol-events/stats
pub async fn get_protocol_event_stats(
    Query(params): Query<ProtocolEventQuery>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Fetching protocol event statistics");

    // Get event type distribution
    let type_stats = sqlx::query_as::<_, (String, i64)>(
        "SELECT event_type, COUNT(*) as count FROM protocol_events 
         WHERE ($1::text IS NULL OR protocol_name = $1)
         AND ($2::timestamptz IS NULL OR event_timestamp >= $2)
         AND ($3::timestamptz IS NULL OR event_timestamp <= $3)
         GROUP BY event_type ORDER BY count DESC"
    )
    .bind(&params.protocol_name)
    .bind(params.from_date)
    .bind(params.to_date)
    .fetch_all(&state.db_pool)
    .await?;

    // Get severity distribution
    let severity_stats = sqlx::query_as::<_, (String, i64)>(
        "SELECT severity, COUNT(*) as count FROM protocol_events 
         WHERE ($1::text IS NULL OR protocol_name = $1)
         AND ($2::timestamptz IS NULL OR event_timestamp >= $2)
         AND ($3::timestamptz IS NULL OR event_timestamp <= $3)
         GROUP BY severity ORDER BY count DESC"
    )
    .bind(&params.protocol_name)
    .bind(params.from_date)
    .bind(params.to_date)
    .fetch_all(&state.db_pool)
    .await?;

    // Get protocol distribution
    let protocol_stats = sqlx::query_as::<_, (String, i64)>(
        "SELECT protocol_name, COUNT(*) as count FROM protocol_events 
         WHERE ($1::timestamptz IS NULL OR event_timestamp >= $1)
         AND ($2::timestamptz IS NULL OR event_timestamp <= $2)
         GROUP BY protocol_name ORDER BY count DESC LIMIT 10"
    )
    .bind(params.from_date)
    .bind(params.to_date)
    .fetch_all(&state.db_pool)
    .await?;

    // Get total funds lost from exploits
    let total_funds_lost: Option<bigdecimal::BigDecimal> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(ee.funds_lost_usd), 0) FROM exploit_events ee
         JOIN protocol_events pe ON ee.protocol_event_id = pe.id
         WHERE ($1::text IS NULL OR pe.protocol_name = $1)
         AND ($2::timestamptz IS NULL OR pe.event_timestamp >= $2)
         AND ($3::timestamptz IS NULL OR pe.event_timestamp <= $3)"
    )
    .bind(&params.protocol_name)
    .bind(params.from_date)
    .bind(params.to_date)
    .fetch_one(&state.db_pool)
    .await?;

    // Map query results to response structs
    let type_distribution: Vec<EventTypeCount> = type_stats
        .into_iter()
        .map(|(event_type, count)| EventTypeCount {
            event_type,
            count,
        })
        .collect();

    let severity_distribution: Vec<SeverityCount> = severity_stats
        .into_iter()
        .map(|(severity, count)| SeverityCount {
            severity,
            count,
        })
        .collect();

    let protocol_distribution: Vec<ProtocolCount> = protocol_stats
        .into_iter()
        .map(|(protocol_name, count)| ProtocolCount {
            protocol_name,
            count,
        })
        .collect();

    let stats = serde_json::json!({
        "event_types": type_distribution.into_iter().map(|r| {
            serde_json::json!({
                "type": r.event_type,
                "count": r.count
            })
        }).collect::<Vec<_>>(),
        "severity_distribution": severity_distribution.into_iter().map(|r| {
            serde_json::json!({
                "severity": r.severity,
                "count": r.count
            })
        }).collect::<Vec<_>>(),
        "top_protocols": protocol_distribution.into_iter().map(|r| {
            serde_json::json!({
                "protocol": r.protocol_name,
                "count": r.count
            })
        }).collect::<Vec<_>>(),
        "total_funds_lost_usd": total_funds_lost.unwrap_or_default()
    });

    Ok(Json(stats))
}
/// Create event alert configuration
/// POST /api/v1/protocol-events/alerts
pub async fn create_event_alert(
    State(state): State<AppState>,
    Json(request): Json<CreateEventAlertRequest>,
) -> Result<Json<EventAlert>, AppError> {
    info!("Creating event alert for user: {}", request.user_address);

    // Parse event types
    let event_types: Result<Vec<EventType>, _> = request.event_types
        .iter()
        .map(|t| serde_json::from_str(&format!("\"{}\"", t)))
        .collect();
    
    let event_types = event_types
        .map_err(|_| AppError::ValidationError("Invalid event type".to_string()))?;

    // Parse severity
    let min_severity: EventSeverity = serde_json::from_str(&format!("\"{}\"", request.min_severity))
        .map_err(|_| AppError::ValidationError("Invalid severity level".to_string()))?;

    let alert = sqlx::query_as::<_, EventAlert>(
        r#"
        INSERT INTO event_alerts (
            user_address, protocol_name, event_types, min_severity, 
            notification_channels, enabled
        ) VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(&request.user_address)
    .bind(&request.protocol_name)
    .bind(&event_types)
    .bind(&min_severity)
    .bind(&request.notification_channels)
    .bind(true)
    .fetch_one(&state.db_pool)
    .await?;

    Ok(Json(alert))
}

/// Get event alerts for user
/// GET /api/v1/protocol-events/alerts
pub async fn get_event_alerts(
    Query(params): Query<EventAlertQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<EventAlert>>, AppError> {
    info!("Fetching event alerts for user: {}", params.user_address);

    let alerts = sqlx::query_as::<_, EventAlert>(
        "SELECT * FROM event_alerts WHERE user_address = $1 
         AND ($2::text IS NULL OR protocol_name = $2)
         ORDER BY created_at DESC"
    )
    .bind(&params.user_address)
    .bind(&params.protocol_name)
    .fetch_all(&state.db_pool)
    .await?;

    Ok(Json(alerts))
}

/// Update event alert
/// PUT /api/v1/protocol-events/alerts/{id}
pub async fn update_event_alert(
    Path(alert_id): Path<Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreateEventAlertRequest>,
) -> Result<Json<EventAlert>, AppError> {
    info!("Updating event alert: {}", alert_id);

    // Parse event types and severity (same as create)
    let event_types: Result<Vec<EventType>, _> = request.event_types
        .iter()
        .map(|t| serde_json::from_str(&format!("\"{}\"", t)))
        .collect();
    
    let event_types = event_types
        .map_err(|_| AppError::ValidationError("Invalid event type".to_string()))?;

    let min_severity: EventSeverity = serde_json::from_str(&format!("\"{}\"", request.min_severity))
        .map_err(|_| AppError::ValidationError("Invalid severity level".to_string()))?;

    let alert = sqlx::query_as::<_, EventAlert>(
        r#"
        UPDATE event_alerts SET 
            protocol_name = $2, event_types = $3, min_severity = $4,
            notification_channels = $5, updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(alert_id)
    .bind(&request.protocol_name)
    .bind(&event_types)
    .bind(&min_severity)
    .bind(&request.notification_channels)
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Event alert not found".to_string()))?;

    Ok(Json(alert))
}

/// Delete event alert
/// DELETE /api/v1/protocol-events/alerts/{id}
pub async fn delete_event_alert(
    Path(alert_id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Deleting event alert: {}", alert_id);

    let result = sqlx::query!("DELETE FROM event_alerts WHERE id = $1", alert_id)
        .execute(&state.db_pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Event alert not found".to_string()));
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Event alert deleted successfully"
    })))
}
