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
        risk_assessment_service::RiskAssessmentService,
        risk_analytics_service::RiskAnalyticsService,
        mev_risk_service::MevRiskService,
        cross_chain_risk_service::CrossChainRiskService,
        protocol_risk_service::ProtocolRiskService,
        portfolio_service::{PortfolioService, PositionSummary},
        price_validation::PriceValidationService,
    },
    models::{RiskAssessment, Position},
    error::AppError,
    AppState,
};
use num_traits::Zero;

// Request/Response DTOs
#[derive(Debug, Deserialize)]
pub struct CreateRiskAssessmentRequest {
    pub entity_id: Uuid,
    pub entity_type: String,
    pub risk_type: String,
    pub risk_score: BigDecimal,
    pub severity: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRiskAssessmentRequest {
    pub risk_score: Option<BigDecimal>,
    pub severity: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RiskAssessmentResponse {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub entity_type: String,
    pub risk_type: String,
    pub risk_score: BigDecimal,
    pub severity: String,
    pub description: String,
    pub metadata: Option<serde_json::Value>,
    pub is_active: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RiskTrendsResponse {
    pub entity_id: Uuid,
    pub time_series: Vec<RiskTrendPoint>,
    pub trend_direction: String,
    pub volatility: BigDecimal,
    pub average_risk: BigDecimal,
    pub max_risk: BigDecimal,
    pub min_risk: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct RiskTrendPoint {
    pub timestamp: DateTime<Utc>,
    pub risk_score: BigDecimal,
    pub risk_type: String,
    pub severity: String,
}

#[derive(Debug, Serialize)]
pub struct RiskCorrelationResponse {
    pub correlation_matrix: Vec<Vec<BigDecimal>>,
    pub asset_labels: Vec<String>,
    pub confidence_level: BigDecimal,
    pub sample_size: i64,
}

#[derive(Debug, Serialize)]
pub struct RiskDistributionResponse {
    pub buckets: Vec<RiskDistributionBucket>,
    pub percentiles: Vec<RiskPercentile>,
    pub mean: BigDecimal,
    pub median: BigDecimal,
    pub std_deviation: BigDecimal,
    pub skewness: BigDecimal,
    pub kurtosis: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct RiskDistributionBucket {
    pub range_start: BigDecimal,
    pub range_end: BigDecimal,
    pub count: i64,
    pub percentage: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct RiskPercentile {
    pub percentile: i32,
    pub value: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct MevRiskResponse {
    pub pool_address: String,
    pub chain_id: i32,
    pub sandwich_risk_score: BigDecimal,
    pub frontrun_risk_score: BigDecimal,
    pub oracle_manipulation_risk: BigDecimal,
    pub overall_mev_risk: BigDecimal,
    pub confidence_score: BigDecimal,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CrossChainRiskResponse {
    pub user_id: Uuid,
    pub total_cross_chain_exposure: BigDecimal,
    pub bridge_risks: Vec<BridgeRiskInfo>,
    pub liquidity_fragmentation_score: BigDecimal,
    pub overall_cross_chain_risk: BigDecimal,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BridgeRiskInfo {
    pub bridge_name: String,
    pub security_score: BigDecimal,
    pub tvl_risk_score: BigDecimal,
    pub overall_risk: BigDecimal,
}

#[derive(Debug, Serialize)]
pub struct ProtocolRiskResponse {
    pub protocol_name: String,
    pub audit_score: BigDecimal,
    pub tvl_score: BigDecimal,
    pub governance_score: BigDecimal,
    pub exploit_history_score: BigDecimal,
    pub overall_risk_score: BigDecimal,
    pub risk_level: String,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct GetRiskAssessmentsQuery {
    pub entity_id: Option<Uuid>,
    pub entity_type: Option<String>,
    pub risk_type: Option<String>,
    pub severity: Option<String>,
    pub is_active: Option<bool>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct GetRiskTrendsQuery {
    pub entity_id: Uuid,
    pub risk_type: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub granularity: Option<String>, // hourly, daily, weekly
}

#[derive(Debug, Deserialize)]
pub struct GetRiskCorrelationQuery {
    pub entity_ids: Vec<Uuid>,
    pub risk_type: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

// Risk Monitor Frontend Integration Data Structures
#[derive(Debug, Serialize)]
pub struct PortfolioRiskMetrics {
    pub overall_risk: i32,
    pub liquidity_risk: i32,
    pub volatility_risk: i32,
    pub mev_risk: i32,
    pub protocol_risk: i32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LiveRiskAlert {
    pub id: String,
    pub severity: String,
    pub alert_type: String,
    pub message: String,
    pub position_id: Option<String>,
    pub protocol: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

#[derive(Debug, Serialize)]
pub struct PositionRiskHeatmap {
    pub id: String,
    pub protocol: String,
    pub pair: String,
    pub risk_score: i32,
    pub risk_factors: RiskFactors,
    pub alerts: i32,
    pub trend: String,
}

#[derive(Debug, Serialize)]
pub struct RiskFactors {
    pub liquidity: i32,
    pub volatility: i32,
    pub mev: i32,
    pub protocol: i32,
}

// Handler functions
pub async fn create_risk_assessment(
    State(state): State<AppState>,
    Json(request): Json<CreateRiskAssessmentRequest>,
) -> Result<Json<RiskAssessmentResponse>, AppError> {
    let risk_service = RiskAssessmentService::new(state.db_pool.clone());
    
    // Convert string types to enums for service call
    let entity_type = match request.entity_type.as_str() {
        "Position" => crate::models::risk_assessment::RiskEntityType::Position,
        "User" => crate::models::risk_assessment::RiskEntityType::User,
        "Protocol" => crate::models::risk_assessment::RiskEntityType::Protocol,
        _ => crate::models::risk_assessment::RiskEntityType::Position, // Default
    };
    
    let risk_type = match request.risk_type.as_str() {
        "Liquidity" => crate::models::risk_assessment::RiskType::Liquidity,
        "Impermanent" => crate::models::risk_assessment::RiskType::ImpermanentLoss,
        "Protocol" => crate::models::risk_assessment::RiskType::Protocol,
        "Market" => crate::models::risk_assessment::RiskType::Market,
        _ => crate::models::risk_assessment::RiskType::Market, // Default
    };
    
    let severity = match request.severity.as_str() {
        "Low" => crate::models::risk_assessment::RiskSeverity::Low,
        "Medium" => crate::models::risk_assessment::RiskSeverity::Medium,
        "High" => crate::models::risk_assessment::RiskSeverity::High,
        "Critical" => crate::models::risk_assessment::RiskSeverity::Critical,
        _ => crate::models::risk_assessment::RiskSeverity::Medium, // Default
    };
    
    let assessment = risk_service.update_risk_assessment(
        entity_type,
        &request.entity_id.to_string(),
        None, // user_id - not provided in request
        risk_type,
        request.risk_score,
        severity,
        None, // confidence - not provided in request
        Some(request.description),
        request.metadata,
        request.expires_at,
    ).await?;
    
    let response = RiskAssessmentResponse {
        id: assessment.id,
        entity_id: uuid::Uuid::parse_str(&assessment.entity_id).unwrap_or_default(),
        entity_type: format!("{:?}", assessment.entity_type),
        risk_type: format!("{:?}", assessment.risk_type),
        risk_score: assessment.risk_score,
        severity: format!("{:?}", assessment.severity),
        description: assessment.description.unwrap_or_default(),
        metadata: assessment.metadata,
        is_active: assessment.is_active,
        expires_at: assessment.expires_at,
        created_at: assessment.created_at,
        updated_at: assessment.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn get_risk_assessment(
    State(state): State<AppState>,
    Path(assessment_id): Path<Uuid>,
) -> Result<Json<RiskAssessmentResponse>, AppError> {
    let risk_service = RiskAssessmentService::new(state.db_pool.clone());
    
    let assessment_option = risk_service.get_risk_assessment_by_id(assessment_id).await?;
    
    let assessment = assessment_option.ok_or_else(|| AppError::NotFound("Risk assessment not found".to_string()))?;
    
    let response = RiskAssessmentResponse {
        id: assessment.id,
        entity_id: uuid::Uuid::parse_str(&assessment.entity_id).unwrap_or_default(),
        entity_type: format!("{:?}", assessment.entity_type),
        risk_type: format!("{:?}", assessment.risk_type),
        risk_score: assessment.risk_score,
        severity: format!("{:?}", assessment.severity),
        description: assessment.description.unwrap_or_default(),
        metadata: assessment.metadata,
        is_active: assessment.is_active,
        expires_at: assessment.expires_at,
        created_at: assessment.created_at,
        updated_at: assessment.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn update_risk_assessment(
    State(state): State<AppState>,
    Path(assessment_id): Path<Uuid>,
    Json(request): Json<UpdateRiskAssessmentRequest>,
) -> Result<Json<RiskAssessmentResponse>, AppError> {
    let risk_service = RiskAssessmentService::new(state.db_pool.clone());
    
    // First get the existing assessment
    let existing_option = risk_service.get_risk_assessment_by_id(assessment_id).await?;
    
    let existing = existing_option.ok_or_else(|| AppError::NotFound("Risk assessment not found".to_string()))?;
    
    // Convert string types to enums for service call
    let entity_type = existing.entity_type;
    let risk_type = existing.risk_type;
    let severity = match request.severity.as_deref().unwrap_or("Medium") {
        "Low" => crate::models::risk_assessment::RiskSeverity::Low,
        "Medium" => crate::models::risk_assessment::RiskSeverity::Medium,
        "High" => crate::models::risk_assessment::RiskSeverity::High,
        "Critical" => crate::models::risk_assessment::RiskSeverity::Critical,
        _ => existing.severity, // Keep existing if invalid
    };
    
    // Update with new values or keep existing ones
    let assessment = risk_service.update_risk_assessment(
        entity_type,
        &existing.entity_id,
        None, // user_id - not provided in request
        risk_type,
        request.risk_score.unwrap_or(existing.risk_score),
        severity,
        None, // confidence - not provided in request
        request.description.or(existing.description),
        request.metadata.or(existing.metadata),
        request.expires_at.or(existing.expires_at),
    ).await?;
    
    let response = RiskAssessmentResponse {
        id: assessment.id,
        entity_id: uuid::Uuid::parse_str(&assessment.entity_id).unwrap_or_default(),
        entity_type: format!("{:?}", assessment.entity_type),
        risk_type: format!("{:?}", assessment.risk_type),
        risk_score: assessment.risk_score,
        severity: format!("{:?}", assessment.severity),
        description: assessment.description.unwrap_or_default(),
        metadata: assessment.metadata,
        is_active: assessment.is_active,
        expires_at: assessment.expires_at,
        created_at: assessment.created_at,
        updated_at: assessment.updated_at,
    };
    
    Ok(Json(response))
}

pub async fn delete_risk_assessment(
    State(state): State<AppState>,
    Path(assessment_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let risk_service = RiskAssessmentService::new(state.db_pool.clone());
    
    risk_service.deactivate_risk_assessment(assessment_id).await?;
    
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_risk_assessments(
    State(state): State<AppState>,
    Query(query): Query<GetRiskAssessmentsQuery>,
) -> Result<Json<Vec<RiskAssessmentResponse>>, AppError> {
    let risk_service = RiskAssessmentService::new(state.db_pool.clone());
    
    // Convert entity_id to appropriate types for service call
    let entity_type = crate::models::risk_assessment::RiskEntityType::Position; // Default
    let entity_id_str = query.entity_id.map(|id| id.to_string()).unwrap_or_default();
    
    let risk_type = query.risk_type.as_deref().and_then(|rt| {
        match rt {
            "Liquidity" => Some(crate::models::risk_assessment::RiskType::Liquidity),
            "ImpermanentLoss" => Some(crate::models::risk_assessment::RiskType::ImpermanentLoss),
            "Protocol" => Some(crate::models::risk_assessment::RiskType::Protocol),
            "Market" => Some(crate::models::risk_assessment::RiskType::Market),
            _ => None,
        }
    });
    
    let assessments = risk_service.get_risk_history(
        entity_type,
        &entity_id_str,
        risk_type,
        Some(30), // days_back - default 30 days
        Some(query.limit.unwrap_or(50) as i64),
    ).await?;
    
    let responses: Vec<RiskAssessmentResponse> = assessments.into_iter().map(|assessment| {
        RiskAssessmentResponse {
            id: assessment.id,
            entity_id: uuid::Uuid::parse_str(&assessment.entity_id).unwrap_or_default(),
            entity_type: format!("{:?}", assessment.entity_type),
            risk_type: format!("{:?}", assessment.risk_type),
            risk_score: assessment.risk_score,
            severity: format!("{:?}", assessment.severity),
            description: assessment.description.unwrap_or_default(),
            metadata: assessment.metadata,
            is_active: assessment.is_active,
            expires_at: assessment.expires_at,
            created_at: assessment.created_at,
            updated_at: assessment.updated_at,
        }
    }).collect();
    
    Ok(Json(responses))
}

pub async fn get_risk_trends(
    State(state): State<AppState>,
    Query(query): Query<GetRiskTrendsQuery>,
) -> Result<Json<RiskTrendsResponse>, AppError> {
    let analytics_service = RiskAnalyticsService::new(state.db_pool.clone());
    
    // Convert query parameters to match service method signature
    let entity_type = Some("Position".to_string()); // Default entity type
    let granularity_hours = match query.granularity.as_deref().unwrap_or("daily") {
        "hourly" => Some(1),
        "daily" => Some(24),
        "weekly" => Some(168),
        _ => Some(24),
    };
    
    let trends = analytics_service.get_risk_trends(
        entity_type,
        query.risk_type.map(|rt| rt.to_string()),
        query.start_date,
        query.end_date,
        granularity_hours,
    ).await?;
    
    let response = RiskTrendsResponse {
        entity_id: query.entity_id,
        time_series: trends.trends.into_iter().map(|trend| {
            RiskTrendPoint {
                timestamp: trend.timestamp,
                risk_score: trend.risk_score,
                risk_type: trend.risk_type,
                severity: trend.severity,
            }
        }).collect(),
        trend_direction: trends.overall_trend,
        volatility: trends.risk_volatility.clone(),
        average_risk: trends.average_risk_score.clone(),
        max_risk: trends.highest_risk_period.map(|_| trends.average_risk_score.clone() * BigDecimal::from(2)).unwrap_or_default(),
        min_risk: trends.lowest_risk_period.map(|_| trends.average_risk_score.clone() / BigDecimal::from(2)).unwrap_or_default(),
    };
    
    Ok(Json(response))
}

pub async fn get_risk_correlation(
    State(state): State<AppState>,
    Query(query): Query<GetRiskCorrelationQuery>,
) -> Result<Json<RiskCorrelationResponse>, AppError> {
    let analytics_service = RiskAnalyticsService::new(state.db_pool.clone());
    
    // Convert query parameters to match service method signature
    let assets: Vec<String> = query.entity_ids.iter().map(|id| id.to_string()).collect();
    let time_period_days = Some(30); // Default 30 days
    
    let correlation = analytics_service.get_correlation_matrix(
        Some(assets),
        time_period_days,
    ).await?;
    
    // Create correlation matrix as Vec<Vec<BigDecimal>> for response
    let mut matrix_rows = Vec::new();
    for asset in &correlation.assets {
        let mut row = Vec::new();
        for other_asset in &correlation.assets {
            let corr_value = correlation.matrix_data
                .get(asset)
                .and_then(|row_data| row_data.get(other_asset))
                .cloned()
                .unwrap_or_else(|| BigDecimal::from(0));
            row.push(corr_value);
        }
        matrix_rows.push(row);
    }
    
    let response = RiskCorrelationResponse {
        correlation_matrix: matrix_rows,
        asset_labels: correlation.assets,
        confidence_level: correlation.average_correlation,
        sample_size: correlation.time_period_analyzed as i64,
    };
    
    Ok(Json(response))
}

pub async fn get_risk_distribution(
    State(state): State<AppState>,
    Query(query): Query<GetRiskAssessmentsQuery>,
) -> Result<Json<RiskDistributionResponse>, AppError> {
    let analytics_service = RiskAnalyticsService::new(state.db_pool.clone());
    
    // Convert query parameters to match service method signature
    let distribution_type = query.entity_type.unwrap_or("severity".to_string());
    let bucket_count = Some(10); // Default 10 buckets
    
    let distribution = analytics_service.get_risk_distribution(
        distribution_type,
        bucket_count,
    ).await?;
    
    let response = RiskDistributionResponse {
        buckets: distribution.buckets.into_iter().map(|bucket| {
            RiskDistributionBucket {
                range_start: bucket.risk_range_min,
                range_end: bucket.risk_range_max,
                count: bucket.count as i64,
                percentage: bucket.percentage,
            }
        }).collect(),
        percentiles: distribution.percentiles.into_iter().map(|(percentile_key, percentile_value)| {
            RiskPercentile {
                percentile: percentile_key.parse::<i32>().unwrap_or(50),
                value: percentile_value,
            }
        }).collect(),
        mean: distribution.mean_risk_score,
        median: distribution.median_risk_score,
        std_deviation: distribution.standard_deviation,
        skewness: distribution.skewness,
        kurtosis: distribution.kurtosis,
    };
    
    Ok(Json(response))
}

pub async fn get_mev_risk(
    State(state): State<AppState>,
    Path((pool_address, chain_id)): Path<(String, i32)>,
) -> Result<Json<MevRiskResponse>, AppError> {
    let mev_service = MevRiskService::new(
        state.db_pool.clone(),
        None, // config
        None, // blockchain_service - simplified for now
        None, // price_validation_service
    );
    
    // Create basic pool state for MEV risk calculation
    // TODO: Integrate real blockchain service calls when type issues are resolved
    let pool_state = crate::models::pool_state::PoolState {
        id: uuid::Uuid::new_v4(),
        pool_address: pool_address.clone(),
        chain_id,
        current_tick: 0,
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(1000000),
        token0_price_usd: Some(BigDecimal::from(1)),
        token1_price_usd: Some(BigDecimal::from(1)),
        tvl_usd: Some(BigDecimal::from(1000000)),
        volume_24h_usd: Some(BigDecimal::from(100000)),
        fees_24h_usd: Some(BigDecimal::from(1000)),
        timestamp: chrono::Utc::now(),
    };
    
    let mev_risk = mev_service.calculate_mev_risk(&pool_address, chain_id, &pool_state).await?;
    
    let response = MevRiskResponse {
        pool_address: mev_risk.pool_address,
        chain_id: mev_risk.chain_id,
        sandwich_risk_score: mev_risk.sandwich_risk_score,
        frontrun_risk_score: mev_risk.frontrun_risk_score,
        oracle_manipulation_risk: mev_risk.oracle_manipulation_risk,
        overall_mev_risk: mev_risk.overall_mev_risk,
        confidence_score: mev_risk.confidence_score,
        recommendations: vec![], // Would be generated based on risk levels
    };
    
    Ok(Json(response))
}

pub async fn get_cross_chain_risk(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<CrossChainRiskResponse>, AppError> {
    let cross_chain_service = CrossChainRiskService::new(state.db_pool.clone(), None);
    
    // For now, use mock data since we need pool states and chain IDs for the actual method
    // In a real implementation, you would fetch user positions and extract chain/pool data
    let primary_chain_id = 1; // Ethereum
    let secondary_chain_ids = vec![137, 42161]; // Polygon, Arbitrum
    let pool_states = vec![]; // Would fetch actual pool states from user positions
    
    let risk_result = cross_chain_service.calculate_cross_chain_risk(
        primary_chain_id,
        &secondary_chain_ids,
        &pool_states,
    ).await?;
    
    // Use the risk_result directly since it's already a CrossChainRiskResult
    let risk = risk_result;
    
    let response = CrossChainRiskResponse {
        user_id,
        total_cross_chain_exposure: risk.overall_cross_chain_risk.clone(),
        bridge_risks: vec![], // Empty for now since bridge_risks field doesn't exist in CrossChainRiskResult
        liquidity_fragmentation_score: risk.liquidity_fragmentation_risk,
        overall_cross_chain_risk: risk.overall_cross_chain_risk,
        recommendations: vec![], // Would be generated based on risk analysis
    };
    
    Ok(Json(response))
}

pub async fn get_protocol_risk(
    State(state): State<AppState>,
    Path(protocol_name): Path<String>,
) -> Result<Json<ProtocolRiskResponse>, AppError> {
    let protocol_service = ProtocolRiskService::new(state.db_pool.clone(), None);
    
    let risk = protocol_service.calculate_protocol_risk(&protocol_name, "0x0000000000000000000000000000000000000000", 1).await?;
    
    let response = ProtocolRiskResponse {
        protocol_name: risk.protocol_name,
        audit_score: risk.audit_score,
        tvl_score: risk.tvl_score,
        governance_score: risk.governance_score,
        exploit_history_score: risk.exploit_history_score,
        overall_risk_score: risk.overall_protocol_risk,
        risk_level: "Medium".to_string(), // Would be calculated based on overall_protocol_risk
        last_updated: risk.last_updated,
    };
    
    Ok(Json(response))
}

// New Risk Monitor specific endpoints
pub async fn get_portfolio_risk_metrics(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PortfolioRiskMetrics>, AppError> {
    let address = params.get("address")
        .ok_or_else(|| AppError::ValidationError("Missing address parameter".to_string()))?;
    
    // For now, return mock data that matches frontend expectations
    // TODO: Replace with real risk calculations from services
    let metrics = PortfolioRiskMetrics {
        overall_risk: 75,
        liquidity_risk: 65,
        volatility_risk: 72,
        mev_risk: 82,
        protocol_risk: 45,
        timestamp: chrono::Utc::now(),
    };
    
    Ok(Json(metrics))
}

pub async fn get_live_risk_alerts(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<LiveRiskAlert>>, AppError> {
    let address = params.get("address")
        .ok_or_else(|| AppError::ValidationError("Missing address parameter".to_string()))?;
    
    // For now, return mock alerts that match frontend expectations
    // TODO: Replace with real alerts from monitoring service
    let alerts = vec![
        LiveRiskAlert {
            id: "alert_1".to_string(),
            severity: "high".to_string(),
            alert_type: "MEV Risk".to_string(),
            message: "High MEV vulnerability detected in ETH/USDC pool".to_string(),
            position_id: Some("pos_1".to_string()),
            protocol: Some("Uniswap V3".to_string()),
            timestamp: chrono::Utc::now(),
            acknowledged: false,
        },
        LiveRiskAlert {
            id: "alert_2".to_string(),
            severity: "medium".to_string(),
            alert_type: "Protocol Risk".to_string(),
            message: "Protocol upgrade scheduled".to_string(),
            position_id: Some("pos_2".to_string()),
            protocol: Some("Uniswap V3".to_string()),
            timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
            acknowledged: false,
        },
    ];
    
    Ok(Json(alerts))
}

pub async fn get_position_risk_heatmap(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<Vec<PositionRiskHeatmap>>, AppError> {
    let address = params.get("address")
        .ok_or_else(|| AppError::ValidationError("Missing address parameter".to_string()))?;
    
    // Fetch user's actual positions from the database
    let price_validation_service = PriceValidationService::new(state.db_pool.clone()).await
        .map_err(|e| AppError::InternalError(format!("Failed to create price validation service: {}", e)))?;
    let mut portfolio_service = PortfolioService::new(state.db_pool.clone(), price_validation_service).await;
    
    // Get user positions (this will resolve ENS if needed)
    let positions_result = portfolio_service.get_portfolio_summary(address).await;
    
    let portfolio_summary = match positions_result {
        Ok(summary) => summary,
        Err(_) => {
            // If we can't fetch positions, return empty array instead of error
            return Ok(Json(vec![]));
        }
    };
    
    // Calculate risk scores for each position
    let mut heatmap = Vec::new();
    
    for position in portfolio_summary.positions {
        // Calculate risk factors based on position data
        let liquidity_risk = calculate_liquidity_risk(&position).await;
        let volatility_risk = calculate_volatility_risk(&position);
        let mev_risk = calculate_mev_risk(&position);
        let protocol_risk = calculate_protocol_risk(&position.protocol);
        
        // Calculate overall risk as weighted average
        let overall_risk = ((liquidity_risk * 30 + volatility_risk * 25 + mev_risk * 25 + protocol_risk * 20) / 100) as i32;
        
        // Determine trend from recent PnL
        let trend = if position.pnl_usd > BigDecimal::zero() {
            "up".to_string()
        } else if position.pnl_usd < BigDecimal::zero() {
            "down".to_string()
        } else {
            "neutral".to_string()
        };
        
        // For position summary, we'll use a simplified pair format
        // In a real implementation, you'd resolve token addresses from the pool
        let pair = format!("{}/TOKEN", position.protocol.to_uppercase());
        
        let position_risk = PositionRiskHeatmap {
            id: position.id.clone(),
            protocol: position.protocol.clone(),
            pair,
            risk_score: overall_risk,
            risk_factors: RiskFactors {
                liquidity: liquidity_risk,
                volatility: volatility_risk,
                mev: mev_risk,
                protocol: protocol_risk,
            },
            alerts: 0, // TODO: Count actual alerts for this position
            trend,
        };
        
        heatmap.push(position_risk);
    }
    
    Ok(Json(heatmap))
}

// Helper functions for risk calculations
async fn calculate_liquidity_risk(position: &PositionSummary) -> i32 {
    // Calculate liquidity risk based on position value
    let position_value = position.current_value_usd.to_string().parse::<f64>().unwrap_or(0.0);
    
    if position_value > 100000.0 {
        85 // High risk for large positions
    } else if position_value > 10000.0 {
        65 // Medium risk
    } else {
        35 // Low risk for small positions
    }
}

fn calculate_volatility_risk(position: &PositionSummary) -> i32 {
    // Calculate volatility risk based on protocol type
    let protocol = position.protocol.to_lowercase();
    
    match protocol.as_str() {
        "uniswap v3" => 70, // Medium-high volatility
        "aave" => 45,      // Lower volatility for lending
        "curve" => 40,     // Lower volatility for stable pairs
        _ => 60,           // Default medium volatility
    }
}

fn calculate_mev_risk(position: &PositionSummary) -> i32 {
    // Calculate MEV risk based on protocol type
    let protocol = position.protocol.to_lowercase();
    
    match protocol.as_str() {
        "uniswap v3" => 80, // High MEV risk for DEX
        "aave" => 25,      // Lower MEV risk for lending
        "curve" => 35,     // Medium MEV risk
        _ => 50,           // Default medium MEV risk
    }
}

fn calculate_protocol_risk(protocol: &str) -> i32 {
    // Calculate protocol risk based on maturity, TVL, and audit status
    let protocol_lower = protocol.to_lowercase();
    
    match protocol_lower.as_str() {
        "uniswap v3" => 30, // Low protocol risk - well established
        "aave" => 25,      // Very low protocol risk
        "curve" => 35,     // Low protocol risk
        "compound" => 30,  // Low protocol risk
        "lido" => 40,      // Medium protocol risk
        _ => 60,           // Higher risk for unknown protocols
    }
}

async fn get_token_symbol(token_address: &str) -> Result<String, AppError> {
    // Simple token symbol resolution - in production this would query the blockchain
    // For now, return common token symbols based on known addresses
    let address_lower = token_address.to_lowercase();
    
    let symbol = match address_lower.as_str() {
        "0xa0b86a33e6441e6c4a9b0b0c4e6c4c6c4c6c4c6c" => "USDC",
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => "WETH",
        "0x6b175474e89094c44da98b954eedeac495271d0f" => "DAI",
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => "USDT",
        _ => "TOKEN", // Default for unknown tokens
    };
    
    Ok(symbol.to_string())
}

// Create router
pub fn create_risk_routes() -> Router<AppState> {
    Router::new()
        // Risk Assessment CRUD
        .route("/risk-assessments", post(create_risk_assessment))
        .route("/risk-assessments", get(list_risk_assessments))
        .route("/risk-assessments/:assessment_id", get(get_risk_assessment))
        .route("/risk-assessments/:assessment_id", put(update_risk_assessment))
        .route("/risk-assessments/:assessment_id", delete(delete_risk_assessment))
        
        // Risk Analytics
        .route("/risk-trends", get(get_risk_trends))
        .route("/risk-correlation", get(get_risk_correlation))
        .route("/risk-distribution", get(get_risk_distribution))
        
        // Specialized Risk Services
        .route("/mev-risk/:pool_address/:chain_id", get(get_mev_risk))
        .route("/cross-chain-risk/:user_id", get(get_cross_chain_risk))
        .route("/protocol-risk/:protocol_name", get(get_protocol_risk))
        
        // Risk Monitor Frontend Integration Endpoints
        .route("/portfolio-risk-metrics", get(get_portfolio_risk_metrics))
        .route("/live-alerts", get(get_live_risk_alerts))
        .route("/position-risk-heatmap", get(get_position_risk_heatmap))
}
