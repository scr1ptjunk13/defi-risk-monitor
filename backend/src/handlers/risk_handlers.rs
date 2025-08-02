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
    },
    error::AppError,
    AppState,
};

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
        None, // blockchain_service
        None, // price_validation_service
    );
    
    // Create a mock PoolState for the MEV risk calculation
    let pool_state = crate::models::pool_state::PoolState {
        id: uuid::Uuid::new_v4(),
        pool_address: pool_address.clone(),
        chain_id,
        current_tick: 0, // Mock current tick
        sqrt_price_x96: BigDecimal::from(1000000), // Mock sqrt price
        liquidity: BigDecimal::from(1000000), // Mock liquidity
        token0_price_usd: Some(BigDecimal::from(1)), // Mock token0 price
        token1_price_usd: Some(BigDecimal::from(1)), // Mock token1 price
        tvl_usd: Some(BigDecimal::from(1000000)), // Mock TVL
        volume_24h_usd: Some(BigDecimal::from(100000)), // Mock volume
        fees_24h_usd: Some(BigDecimal::from(1000)), // Mock fees
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
}
