use crate::models::Position;
use crate::services::{
    lp_analytics_service::{LpAnalyticsService, LpReturns, PoolPerformanceMetrics},
    pool_performance_service::PoolPerformanceService,
    yield_farming_service::{YieldFarmingService, YieldFarmingMetrics, FarmingStrategy, OptimalAllocation},
    comparative_analytics_service::{ComparativeAnalyticsService, PoolComparison, BenchmarkMetrics, PerformanceRanking},
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub pool_address: Option<String>,
    pub chain_id: Option<i32>,
    pub user_address: Option<String>,
    pub position_id: Option<Uuid>,
    pub period: Option<String>, // "24h", "7d", "30d", "90d"
    pub limit: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct ComparisonQuery {
    pub pools: String, // Comma-separated pool addresses
    pub chain_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct AllocationQuery {
    pub pools: String, // Comma-separated pool addresses
    pub investment_amount: BigDecimal,
    pub risk_tolerance: f64, // 0.0 to 1.0
}

#[derive(Debug, Deserialize)]
pub struct BenchmarkQuery {
    pub pool_address: String,
    pub chain_id: i32,
    pub benchmark_type: String, // "DeFi_Index", "ETH_Staking", etc.
}

#[derive(Debug, Serialize)]
pub struct LpReturnsResponse {
    pub success: bool,
    pub data: LpReturns,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PoolPerformanceResponse {
    pub success: bool,
    pub data: PoolPerformanceMetrics,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct YieldFarmingResponse {
    pub success: bool,
    pub data: YieldFarmingMetrics,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FarmingStrategiesResponse {
    pub success: bool,
    pub data: Vec<FarmingStrategy>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct OptimalAllocationResponse {
    pub success: bool,
    pub data: Vec<OptimalAllocation>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PoolComparisonResponse {
    pub success: bool,
    pub data: Vec<PoolComparison>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkResponse {
    pub success: bool,
    pub data: BenchmarkMetrics,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PerformanceRankingsResponse {
    pub success: bool,
    pub data: Vec<PerformanceRanking>,
    pub message: String,
}

/// Get LP returns for a specific position
pub async fn get_lp_returns(
    State(app_state): State<AppState>,
    Path(position_id): Path<Uuid>,
) -> Result<Json<LpReturnsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let analytics_service = LpAnalyticsService::new(app_state.db_pool.clone());

    // Get position from database
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(position_id)
    .fetch_one(&app_state.db_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Position not found: {}", e)
            }))
        )
    })?;

    let lp_returns = analytics_service.calculate_lp_returns(&position).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to calculate LP returns: {}", e)
                }))
            )
        })?;

    Ok(Json(LpReturnsResponse {
        success: true,
        data: lp_returns,
        message: "LP returns calculated successfully".to_string(),
    }))
}

/// Get pool performance metrics
pub async fn get_pool_performance(
    State(app_state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<PoolPerformanceResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool_address = params.pool_address.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "pool_address is required"
            }))
        )
    })?;

    let chain_id = params.chain_id.unwrap_or(1);
    let performance_service = PoolPerformanceService::new(app_state.db_pool.clone());

    let performance_metrics = performance_service.get_pool_performance(&pool_address, chain_id).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to get pool performance: {}", e)
                }))
            )
        })?;

    Ok(Json(PoolPerformanceResponse {
        success: true,
        data: performance_metrics,
        message: "Pool performance metrics retrieved successfully".to_string(),
    }))
}

/// Get yield farming metrics
pub async fn get_yield_farming_metrics(
    State(app_state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<YieldFarmingResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool_address = params.pool_address.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "success": false,
                "error": "pool_address is required"
            }))
        )
    })?;

    let chain_id = params.chain_id.unwrap_or(1);
    let farming_service = YieldFarmingService::new(app_state.db_pool.clone());

    let farming_metrics = farming_service.calculate_farming_metrics(&pool_address, chain_id).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to get yield farming metrics: {}", e)
                }))
            )
        })?;

    Ok(Json(YieldFarmingResponse {
        success: true,
        data: farming_metrics,
        message: "Yield farming metrics retrieved successfully".to_string(),
    }))
}

/// Get farming strategies
pub async fn get_farming_strategies(
    State(app_state): State<AppState>,
    Query(params): Query<AllocationQuery>,
) -> Result<Json<FarmingStrategiesResponse>, (StatusCode, Json<serde_json::Value>)> {
    let farming_service = YieldFarmingService::new(app_state.db_pool.clone());

    let strategies = farming_service.generate_farming_strategies(&params.investment_amount, params.risk_tolerance).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to generate farming strategies: {}", e)
                }))
            )
        })?;

    Ok(Json(FarmingStrategiesResponse {
        success: true,
        data: strategies,
        message: "Farming strategies generated successfully".to_string(),
    }))
}

/// Get optimal allocation
pub async fn get_optimal_allocation(
    State(app_state): State<AppState>,
    Query(params): Query<AllocationQuery>,
) -> Result<Json<OptimalAllocationResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool_addresses: Vec<String> = params.pools.split(',').map(|s| s.trim().to_string()).collect();
    let farming_service = YieldFarmingService::new(app_state.db_pool.clone());

    let allocation = farming_service.calculate_optimal_allocation(&pool_addresses, &params.investment_amount, params.risk_tolerance).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to calculate optimal allocation: {}", e)
                }))
            )
        })?;

    Ok(Json(OptimalAllocationResponse {
        success: true,
        data: allocation,
        message: "Optimal allocation calculated successfully".to_string(),
    }))
}

/// Compare multiple pools
pub async fn compare_pools(
    State(app_state): State<AppState>,
    Query(params): Query<ComparisonQuery>,
) -> Result<Json<PoolComparisonResponse>, (StatusCode, Json<serde_json::Value>)> {
    let pool_addresses: Vec<String> = params.pools.split(',').map(|s| s.trim().to_string()).collect();
    let comparative_service = ComparativeAnalyticsService::new(app_state.db_pool.clone());

    let comparisons = comparative_service.compare_pools(&pool_addresses, params.chain_id).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to compare pools: {}", e)
                }))
            )
        })?;

    Ok(Json(PoolComparisonResponse {
        success: true,
        data: comparisons,
        message: "Pool comparison completed successfully".to_string(),
    }))
}

/// Get benchmark metrics
pub async fn get_benchmark_metrics(
    State(app_state): State<AppState>,
    Query(params): Query<BenchmarkQuery>,
) -> Result<Json<BenchmarkResponse>, (StatusCode, Json<serde_json::Value>)> {
    let comparative_service = ComparativeAnalyticsService::new(app_state.db_pool.clone());

    let benchmark_metrics = comparative_service.calculate_benchmark_metrics(&params.pool_address, params.chain_id, &params.benchmark_type).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to calculate benchmark metrics: {}", e)
                }))
            )
        })?;

    Ok(Json(BenchmarkResponse {
        success: true,
        data: benchmark_metrics,
        message: "Benchmark metrics calculated successfully".to_string(),
    }))
}

/// Get performance rankings
pub async fn get_performance_rankings(
    State(app_state): State<AppState>,
    Query(params): Query<AnalyticsQuery>,
) -> Result<Json<PerformanceRankingsResponse>, (StatusCode, Json<serde_json::Value>)> {
    let chain_id = params.chain_id.unwrap_or(1);
    let comparative_service = ComparativeAnalyticsService::new(app_state.db_pool.clone());

    let rankings = comparative_service.generate_performance_rankings(chain_id).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to generate performance rankings: {}", e)
                }))
            )
        })?;

    Ok(Json(PerformanceRankingsResponse {
        success: true,
        data: rankings,
        message: "Performance rankings generated successfully".to_string(),
    }))
}

/// Get LP performance benchmark
pub async fn get_lp_benchmark(
    State(app_state): State<AppState>,
    Path(position_id): Path<Uuid>,
    Query(params): Query<ComparisonQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(position_id)
    .fetch_one(&app_state.db_pool)
    .await
    .map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "success": false,
                "error": format!("Position not found: {}", e)
            }))
        )
    })?;

    let benchmark_pools: Vec<String> = params.pools.split(',').map(|s| s.trim().to_string()).collect();
    let comparative_service = ComparativeAnalyticsService::new(app_state.db_pool.clone());

    let benchmarks = comparative_service.benchmark_lp_performance(&position, &benchmark_pools).await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to benchmark LP performance: {}", e)
                }))
            )
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "data": benchmarks,
        "message": "LP performance benchmark completed successfully"
    })))
}
