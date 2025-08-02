use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use crate::{
    services::system_health_service::SystemHealthService,
    error::AppError,
    AppState,
};

// Request/Response DTOs
#[derive(Debug, Serialize)]
pub struct DatabaseMetricsResponse {
    pub total_connections: i32,
    pub active_connections: i32,
    pub idle_connections: i32,
    pub max_connections: i32,
    pub cache_hit_ratio: BigDecimal,
    pub total_queries: i64,
    pub slow_queries: i64,
    pub avg_query_time_ms: BigDecimal,
    pub database_size_mb: BigDecimal,
    pub table_count: i32,
    pub index_count: i32,
    pub replication_lag_ms: Option<BigDecimal>,
}

#[derive(Debug, Serialize)]
pub struct QueryPerformanceResponse {
    pub total_queries: i64,
    pub queries_per_second: BigDecimal,
    pub avg_execution_time_ms: BigDecimal,
    pub slow_query_count: i64,
    pub slow_query_threshold_ms: i32,
    pub cache_hit_ratio: BigDecimal,
    pub index_usage_ratio: BigDecimal,
    pub sequential_scan_ratio: BigDecimal,
    pub most_expensive_queries: Vec<ExpensiveQuery>,
}

#[derive(Debug, Serialize)]
pub struct ExpensiveQuery {
    pub query_hash: String,
    pub avg_time_ms: BigDecimal,
    pub call_count: i64,
    pub total_time_ms: BigDecimal,
    pub query_sample: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectionPoolHealthResponse {
    pub pool_name: String,
    pub total_connections: i32,
    pub active_connections: i32,
    pub idle_connections: i32,
    pub max_connections: i32,
    pub min_connections: i32,
    pub connection_utilization: BigDecimal,
    pub avg_connection_lifetime_ms: BigDecimal,
    pub connection_errors: i64,
    pub connection_timeouts: i64,
    pub health_score: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct TableSizeResponse {
    pub table_name: String,
    pub schema_name: String,
    pub table_size_mb: BigDecimal,
    pub index_size_mb: BigDecimal,
    pub total_size_mb: BigDecimal,
    pub row_count: i64,
    pub bloat_ratio: BigDecimal,
    pub last_vacuum: Option<DateTime<Utc>>,
    pub last_analyze: Option<DateTime<Utc>>,
    pub maintenance_recommended: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemHealthOverviewResponse {
    pub overall_health_score: BigDecimal,
    pub database_health: DatabaseHealthStatus,
    pub connection_pool_health: ConnectionPoolHealthStatus,
    pub query_performance_health: QueryPerformanceStatus,
    pub disk_usage_health: DiskUsageStatus,
    pub alerts: Vec<SystemAlert>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealthStatus {
    pub status: String, // healthy, warning, critical
    pub score: BigDecimal,
    pub issues: Vec<String>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionPoolHealthStatus {
    pub status: String,
    pub score: BigDecimal,
    pub utilization: BigDecimal,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryPerformanceStatus {
    pub status: String,
    pub score: BigDecimal,
    pub avg_response_time_ms: BigDecimal,
    pub slow_query_percentage: BigDecimal,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DiskUsageStatus {
    pub status: String,
    pub score: BigDecimal,
    pub usage_percentage: BigDecimal,
    pub available_space_gb: BigDecimal,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SystemAlert {
    pub id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct GetMetricsQuery {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub include_historical: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct GetTableSizesQuery {
    pub schema_name: Option<String>,
    pub min_size_mb: Option<i32>,
    pub include_indexes: Option<bool>,
    pub sort_by: Option<String>, // size, rows, bloat
}

// Handler functions
pub async fn get_database_metrics(
    State(state): State<AppState>,
    Query(_query): Query<GetMetricsQuery>,
) -> Result<Json<DatabaseMetricsResponse>, AppError> {
    let health_service = SystemHealthService::new(state.db_pool.clone());
    
    let metrics = health_service.get_database_metrics().await?;
    
    let response = DatabaseMetricsResponse {
        total_connections: metrics.max_connections,
        active_connections: metrics.active_connections,
        idle_connections: metrics.max_connections - metrics.active_connections,
        max_connections: metrics.max_connections,
        cache_hit_ratio: BigDecimal::try_from(metrics.cache_hit_ratio).unwrap_or_default(),
        total_queries: metrics.total_queries,
        slow_queries: metrics.slow_queries,
        avg_query_time_ms: BigDecimal::from(0), // Not available in current metrics
        database_size_mb: BigDecimal::from(metrics.database_size_mb),
        table_count: 0, // Not available in current metrics
        index_count: 0, // Not available in current metrics
        replication_lag_ms: metrics.replication_lag_ms.map(BigDecimal::from),
    };
    
    Ok(Json(response))
}

pub async fn get_query_performance(
    State(state): State<AppState>,
    Query(_query): Query<GetMetricsQuery>,
) -> Result<Json<QueryPerformanceResponse>, AppError> {
    let health_service = SystemHealthService::new(state.db_pool.clone());
    
    let performance = health_service.get_query_performance_stats().await?;
    
    let response = QueryPerformanceResponse {
        total_queries: performance.total_queries,
        queries_per_second: BigDecimal::try_from(performance.queries_per_second).unwrap_or_default(),
        avg_execution_time_ms: BigDecimal::try_from(performance.avg_query_time_ms).unwrap_or_default(),
        slow_query_count: performance.slow_queries_count,
        slow_query_threshold_ms: performance.slow_query_threshold_ms as i32,
        cache_hit_ratio: BigDecimal::try_from(performance.query_cache_stats.cache_hit_ratio).unwrap_or_default(),
        index_usage_ratio: BigDecimal::from(0), // Calculate from index_usage_stats
        sequential_scan_ratio: BigDecimal::from(0), // Calculate from table_scan_stats
        most_expensive_queries: performance.top_slow_queries.into_iter().map(|q| ExpensiveQuery {
            query_hash: q.query_hash,
            avg_time_ms: BigDecimal::try_from(q.avg_time_ms).unwrap_or_default(),
            call_count: q.calls,
            total_time_ms: BigDecimal::try_from(q.total_time_ms).unwrap_or_default(),
            query_sample: q.query_text,
        }).collect(),
    };
    
    Ok(Json(response))
}

pub async fn get_connection_pool_health(
    State(state): State<AppState>,
) -> Result<Json<ConnectionPoolHealthResponse>, AppError> {
    let health_service = SystemHealthService::new(state.db_pool.clone());
    
    let pool_health = health_service.get_connection_pool_health().await?;
    
    let response = ConnectionPoolHealthResponse {
        pool_name: "default".to_string(),
        total_connections: pool_health.pool_stats.size as i32,
        active_connections: pool_health.pool_stats.active as i32,
        idle_connections: pool_health.pool_stats.idle as i32,
        max_connections: pool_health.pool_stats.max_connections as i32,
        min_connections: pool_health.pool_stats.min_connections as i32,
        connection_utilization: BigDecimal::try_from(pool_health.pool_utilization_percent).unwrap_or_default(),
        avg_connection_lifetime_ms: BigDecimal::try_from(pool_health.avg_connection_time_ms).unwrap_or_default(),
        connection_errors: pool_health.connection_errors,
        connection_timeouts: pool_health.connection_timeouts,
        health_score: BigDecimal::from(pool_health.health_score as i32),
    };
    
    Ok(Json(response))
}

pub async fn get_table_sizes(
    State(state): State<AppState>,
    Query(query): Query<GetTableSizesQuery>,
) -> Result<Json<Vec<TableSizeResponse>>, AppError> {
    let health_service = SystemHealthService::new(state.db_pool.clone());
    
    let table_sizes = health_service.get_table_sizes().await?;
    
    let mut responses: Vec<TableSizeResponse> = table_sizes.largest_tables.into_iter().map(|table| {
        TableSizeResponse {
            table_name: table.table_name,
            schema_name: table.schema_name,
            table_size_mb: BigDecimal::from(table.table_size_mb),
            index_size_mb: BigDecimal::from(table.index_size_mb),
            total_size_mb: BigDecimal::from(table.total_size_mb),
            row_count: table.row_count,
            bloat_ratio: table.bloat_ratio.map(|ratio| BigDecimal::try_from(ratio).unwrap_or_default()).unwrap_or_default(),
            last_vacuum: table.last_vacuum,
            last_analyze: table.last_analyze,
            maintenance_recommended: table.bloat_ratio.map_or(false, |ratio| ratio > 0.2), // Recommend maintenance if bloat > 20%
        }
    }).collect();
    
    // Apply filters
    if let Some(schema) = &query.schema_name {
        responses.retain(|t| t.schema_name == *schema);
    }
    
    if let Some(min_size) = query.min_size_mb {
        responses.retain(|t| t.total_size_mb >= BigDecimal::from(min_size));
    }
    
    // Apply sorting
    match query.sort_by.as_deref() {
        Some("size") => responses.sort_by(|a, b| b.total_size_mb.cmp(&a.total_size_mb)),
        Some("rows") => responses.sort_by(|a, b| b.row_count.cmp(&a.row_count)),
        Some("bloat") => responses.sort_by(|a, b| b.bloat_ratio.cmp(&a.bloat_ratio)),
        _ => responses.sort_by(|a, b| a.table_name.cmp(&b.table_name)),
    }
    
    Ok(Json(responses))
}

pub async fn get_system_health_overview(
    State(state): State<AppState>,
) -> Result<Json<SystemHealthOverviewResponse>, AppError> {
    let health_service = SystemHealthService::new(state.db_pool.clone());
    
    // Get all health metrics
    let db_metrics = health_service.get_database_metrics().await?;
    let query_performance = health_service.get_query_performance_stats().await?;
    let pool_health = health_service.get_connection_pool_health().await?;
    
    // Calculate overall health score (simplified)
    let db_score = if db_metrics.cache_hit_ratio > 0.95 { 100 } else { 80 };
    let query_score = if query_performance.avg_query_time_ms < 100.0 { 100 } else { 70 };
    let pool_score = BigDecimal::from(pool_health.health_score as i32);
    
    let overall_score = (BigDecimal::from(db_score) + BigDecimal::from(query_score) + pool_score) / BigDecimal::from(3);
    
    // Generate status based on scores
    let db_status = if db_score >= 90 { "healthy" } else if db_score >= 70 { "warning" } else { "critical" };
    let query_status = if query_score >= 90 { "healthy" } else if query_score >= 70 { "warning" } else { "critical" };
    let pool_status = if pool_health.health_score >= 90.0 { "healthy" } else if pool_health.health_score >= 70.0 { "warning" } else { "critical" };
    
    let response = SystemHealthOverviewResponse {
        overall_health_score: overall_score,
        database_health: DatabaseHealthStatus {
            status: db_status.to_string(),
            score: BigDecimal::from(db_score),
            issues: vec![],
            recommendations: vec![],
        },
        connection_pool_health: ConnectionPoolHealthStatus {
            status: pool_status.to_string(),
            score: BigDecimal::from(pool_health.health_score as i32),
            utilization: BigDecimal::from(pool_health.pool_utilization_percent as i32),
            issues: vec![],
        },
        query_performance_health: QueryPerformanceStatus {
            status: query_status.to_string(),
            score: BigDecimal::from(query_score),
            avg_response_time_ms: BigDecimal::from(query_performance.avg_query_time_ms as i32),
            slow_query_percentage: if query_performance.total_queries > 0 {
                (BigDecimal::from(query_performance.slow_queries_count) / BigDecimal::from(query_performance.total_queries)) * BigDecimal::from(100)
            } else {
                BigDecimal::from(0)
            },
            issues: vec![],
        },
        disk_usage_health: DiskUsageStatus {
            status: "healthy".to_string(),
            score: BigDecimal::from(95),
            usage_percentage: BigDecimal::from(45),
            available_space_gb: BigDecimal::from(100),
            issues: vec![],
        },
        alerts: vec![], // Would be populated from actual alert system
        last_updated: Utc::now(),
    };
    
    Ok(Json(response))
}

pub async fn trigger_maintenance(
    State(_state): State<AppState>,
    Path(_table_name): Path<String>,
) -> Result<StatusCode, AppError> {
    // This would trigger VACUUM/ANALYZE operations
    // Implementation would depend on your maintenance strategy
    Ok(StatusCode::ACCEPTED)
}

pub async fn get_health_alerts(
    State(state): State<AppState>,
    Query(_query): Query<GetMetricsQuery>,
) -> Result<Json<Vec<SystemAlert>>, AppError> {
    // This would fetch actual system alerts from the database
    // For now, return empty array
    Ok(Json(vec![]))
}

// Create router
pub fn create_system_health_routes() -> Router<AppState> {
    Router::new()
        .route("/system/health", get(get_system_health_overview))
        .route("/system/health/database", get(get_database_metrics))
        .route("/system/health/query-performance", get(get_query_performance))
        .route("/system/health/connection-pool", get(get_connection_pool_health))
        .route("/system/health/table-sizes", get(get_table_sizes))
        .route("/system/health/alerts", get(get_health_alerts))
        .route("/system/maintenance/:table_name", post(trigger_maintenance))
}
