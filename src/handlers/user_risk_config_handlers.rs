use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use bigdecimal::BigDecimal;
use crate::models::{UserRiskConfig, CreateUserRiskConfig, UpdateUserRiskConfig, RiskToleranceLevel};
use crate::services::user_risk_config_service::UserRiskConfigService;
use crate::AppState;

/// Request/Response DTOs for User Risk Configuration API

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRiskConfigRequest {
    pub user_address: String,
    pub profile_name: String,
    pub risk_tolerance_level: RiskToleranceLevel,
    
    // Optional custom parameters
    pub custom_weights: Option<RiskWeights>,
    pub custom_thresholds: Option<RiskThresholds>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskWeights {
    pub liquidity_risk_weight: Option<BigDecimal>,
    pub volatility_risk_weight: Option<BigDecimal>,
    pub protocol_risk_weight: Option<BigDecimal>,
    pub mev_risk_weight: Option<BigDecimal>,
    pub cross_chain_risk_weight: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskThresholds {
    // Liquidity thresholds
    pub min_tvl_threshold: Option<BigDecimal>,
    pub max_slippage_tolerance: Option<BigDecimal>,
    pub thin_pool_threshold: Option<BigDecimal>,
    pub tvl_drop_threshold: Option<BigDecimal>,
    
    // Volatility thresholds
    pub volatility_lookback_days: Option<i32>,
    pub high_volatility_threshold: Option<BigDecimal>,
    pub correlation_threshold: Option<BigDecimal>,
    
    // Protocol thresholds
    pub min_audit_score: Option<BigDecimal>,
    pub max_exploit_tolerance: Option<i32>,
    pub governance_risk_weight: Option<BigDecimal>,
    
    // MEV thresholds
    pub sandwich_attack_threshold: Option<BigDecimal>,
    pub frontrun_threshold: Option<BigDecimal>,
    pub oracle_deviation_threshold: Option<BigDecimal>,
    
    // Cross-chain thresholds
    pub bridge_risk_tolerance: Option<BigDecimal>,
    pub liquidity_fragmentation_threshold: Option<BigDecimal>,
    pub governance_divergence_threshold: Option<BigDecimal>,
    
    // Overall threshold
    pub overall_risk_threshold: Option<BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRiskConfigRequest {
    pub profile_name: Option<String>,
    pub is_active: Option<bool>,
    pub risk_tolerance_level: Option<RiskToleranceLevel>,
    pub custom_weights: Option<RiskWeights>,
    pub custom_thresholds: Option<RiskThresholds>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskConfigResponse {
    pub id: Uuid,
    pub user_address: String,
    pub profile_name: String,
    pub is_active: bool,
    pub risk_tolerance_level: RiskToleranceLevel,
    
    pub weights: RiskWeights,
    pub thresholds: RiskThresholds,
    
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRiskConfigsQuery {
    pub user_address: String,
    pub include_inactive: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RiskParamsResponse {
    pub user_address: String,
    pub active_profile: Option<String>,
    pub parameters: HashMap<String, BigDecimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl From<UserRiskConfig> for RiskConfigResponse {
    fn from(config: UserRiskConfig) -> Self {
        Self {
            id: config.id,
            user_address: config.user_address,
            profile_name: config.profile_name,
            is_active: config.is_active,
            risk_tolerance_level: config.risk_tolerance_level,
            
            weights: RiskWeights {
                liquidity_risk_weight: Some(config.liquidity_risk_weight),
                volatility_risk_weight: Some(config.volatility_risk_weight),
                protocol_risk_weight: Some(config.protocol_risk_weight),
                mev_risk_weight: Some(config.mev_risk_weight),
                cross_chain_risk_weight: Some(config.cross_chain_risk_weight),
            },
            
            thresholds: RiskThresholds {
                min_tvl_threshold: Some(config.min_tvl_threshold),
                max_slippage_tolerance: Some(config.max_slippage_tolerance),
                thin_pool_threshold: Some(config.thin_pool_threshold),
                tvl_drop_threshold: Some(config.tvl_drop_threshold),
                volatility_lookback_days: Some(config.volatility_lookback_days),
                high_volatility_threshold: Some(config.high_volatility_threshold),
                correlation_threshold: Some(config.correlation_threshold),
                min_audit_score: Some(config.min_audit_score),
                max_exploit_tolerance: Some(config.max_exploit_tolerance),
                governance_risk_weight: Some(config.governance_risk_weight),
                sandwich_attack_threshold: Some(config.sandwich_attack_threshold),
                frontrun_threshold: Some(config.frontrun_threshold),
                oracle_deviation_threshold: Some(config.oracle_deviation_threshold),
                bridge_risk_tolerance: Some(config.bridge_risk_tolerance),
                liquidity_fragmentation_threshold: Some(config.liquidity_fragmentation_threshold),
                governance_divergence_threshold: Some(config.governance_divergence_threshold),
                overall_risk_threshold: Some(config.overall_risk_threshold),
            },
            
            created_at: config.created_at.to_rfc3339(),
            updated_at: config.updated_at.to_rfc3339(),
        }
    }
}

/// Create a new risk configuration
/// POST /api/v1/risk-configs
pub async fn create_risk_config(
    State(state): State<AppState>,
    Json(request): Json<CreateRiskConfigRequest>,
) -> Result<Json<ApiResponse<RiskConfigResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    let create_config = CreateUserRiskConfig {
        user_address: request.user_address,
        profile_name: request.profile_name,
        risk_tolerance_level: request.risk_tolerance_level,
        
        // Apply custom weights if provided
        liquidity_risk_weight: request.custom_weights.as_ref().and_then(|w| w.liquidity_risk_weight.clone()),
        volatility_risk_weight: request.custom_weights.as_ref().and_then(|w| w.volatility_risk_weight.clone()),
        protocol_risk_weight: request.custom_weights.as_ref().and_then(|w| w.protocol_risk_weight.clone()),
        mev_risk_weight: request.custom_weights.as_ref().and_then(|w| w.mev_risk_weight.clone()),
        cross_chain_risk_weight: request.custom_weights.as_ref().and_then(|w| w.cross_chain_risk_weight.clone()),
        
        // Apply custom thresholds if provided
        min_tvl_threshold: request.custom_thresholds.as_ref().and_then(|t| t.min_tvl_threshold.clone()),
        max_slippage_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.max_slippage_tolerance.clone()),
        thin_pool_threshold: request.custom_thresholds.as_ref().and_then(|t| t.thin_pool_threshold.clone()),
        tvl_drop_threshold: request.custom_thresholds.as_ref().and_then(|t| t.tvl_drop_threshold.clone()),
        volatility_lookback_days: request.custom_thresholds.as_ref().and_then(|t| t.volatility_lookback_days),
        high_volatility_threshold: request.custom_thresholds.as_ref().and_then(|t| t.high_volatility_threshold.clone()),
        correlation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.correlation_threshold.clone()),
        min_audit_score: request.custom_thresholds.as_ref().and_then(|t| t.min_audit_score.clone()),
        max_exploit_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.max_exploit_tolerance),
        governance_risk_weight: request.custom_thresholds.as_ref().and_then(|t| t.governance_risk_weight.clone()),
        sandwich_attack_threshold: request.custom_thresholds.as_ref().and_then(|t| t.sandwich_attack_threshold.clone()),
        frontrun_threshold: request.custom_thresholds.as_ref().and_then(|t| t.frontrun_threshold.clone()),
        oracle_deviation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.oracle_deviation_threshold.clone()),
        bridge_risk_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.bridge_risk_tolerance.clone()),
        liquidity_fragmentation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.liquidity_fragmentation_threshold.clone()),
        governance_divergence_threshold: request.custom_thresholds.as_ref().and_then(|t| t.governance_divergence_threshold.clone()),
        overall_risk_threshold: request.custom_thresholds.as_ref().and_then(|t| t.overall_risk_threshold.clone()),
    };
    
    match service.create_config(create_config).await {
        Ok(config) => Ok(Json(ApiResponse {
            success: true,
            data: Some(config.into()),
            message: Some("Risk configuration created successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to create risk configuration".to_string()),
        })))
    }
}

/// Get risk configurations for a user
/// GET /api/v1/risk-configs
pub async fn get_risk_configs(
    State(state): State<AppState>,
    Query(query): Query<GetRiskConfigsQuery>,
) -> Result<Json<ApiResponse<Vec<RiskConfigResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.get_user_configs(&query.user_address).await {
        Ok(configs) => {
            let filtered_configs: Vec<UserRiskConfig> = if query.include_inactive.unwrap_or(false) {
                configs
            } else {
                configs.into_iter().filter(|c| c.is_active).collect()
            };
            
            let response_data: Vec<RiskConfigResponse> = filtered_configs.into_iter().map(|c| c.into()).collect();
            
            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: None,
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to get risk configurations".to_string()),
        })))
    }
}

/// Get a specific risk configuration by ID
/// GET /api/v1/risk-configs/{id}
pub async fn get_risk_config(
    State(state): State<AppState>,
    Path(config_id): Path<Uuid>,
) -> Result<Json<ApiResponse<RiskConfigResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.get_config(config_id).await {
        Ok(Some(config)) => Ok(Json(ApiResponse {
            success: true,
            data: Some(config.into()),
            message: None,
        })),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Risk configuration not found".to_string()),
        }))),
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to get risk configuration".to_string()),
        })))
    }
}

/// Update a risk configuration
/// PUT /api/v1/risk-configs/{id}
pub async fn update_risk_config(
    State(state): State<AppState>,
    Path(config_id): Path<Uuid>,
    Json(request): Json<UpdateRiskConfigRequest>,
) -> Result<Json<ApiResponse<RiskConfigResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    let update_config = UpdateUserRiskConfig {
        profile_name: request.profile_name,
        is_active: request.is_active,
        risk_tolerance_level: request.risk_tolerance_level,
        
        // Apply weight updates if provided
        liquidity_risk_weight: request.custom_weights.as_ref().and_then(|w| w.liquidity_risk_weight.clone()),
        volatility_risk_weight: request.custom_weights.as_ref().and_then(|w| w.volatility_risk_weight.clone()),
        protocol_risk_weight: request.custom_weights.as_ref().and_then(|w| w.protocol_risk_weight.clone()),
        mev_risk_weight: request.custom_weights.as_ref().and_then(|w| w.mev_risk_weight.clone()),
        cross_chain_risk_weight: request.custom_weights.as_ref().and_then(|w| w.cross_chain_risk_weight.clone()),
        
        // Apply threshold updates if provided
        min_tvl_threshold: request.custom_thresholds.as_ref().and_then(|t| t.min_tvl_threshold.clone()),
        max_slippage_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.max_slippage_tolerance.clone()),
        thin_pool_threshold: request.custom_thresholds.as_ref().and_then(|t| t.thin_pool_threshold.clone()),
        tvl_drop_threshold: request.custom_thresholds.as_ref().and_then(|t| t.tvl_drop_threshold.clone()),
        volatility_lookback_days: request.custom_thresholds.as_ref().and_then(|t| t.volatility_lookback_days),
        high_volatility_threshold: request.custom_thresholds.as_ref().and_then(|t| t.high_volatility_threshold.clone()),
        correlation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.correlation_threshold.clone()),
        min_audit_score: request.custom_thresholds.as_ref().and_then(|t| t.min_audit_score.clone()),
        max_exploit_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.max_exploit_tolerance),
        governance_risk_weight: request.custom_thresholds.as_ref().and_then(|t| t.governance_risk_weight.clone()),
        sandwich_attack_threshold: request.custom_thresholds.as_ref().and_then(|t| t.sandwich_attack_threshold.clone()),
        frontrun_threshold: request.custom_thresholds.as_ref().and_then(|t| t.frontrun_threshold.clone()),
        oracle_deviation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.oracle_deviation_threshold.clone()),
        bridge_risk_tolerance: request.custom_thresholds.as_ref().and_then(|t| t.bridge_risk_tolerance.clone()),
        liquidity_fragmentation_threshold: request.custom_thresholds.as_ref().and_then(|t| t.liquidity_fragmentation_threshold.clone()),
        governance_divergence_threshold: request.custom_thresholds.as_ref().and_then(|t| t.governance_divergence_threshold.clone()),
        overall_risk_threshold: request.custom_thresholds.as_ref().and_then(|t| t.overall_risk_threshold.clone()),
    };
    
    match service.update_config(config_id, update_config).await {
        Ok(config) => Ok(Json(ApiResponse {
            success: true,
            data: Some(config.into()),
            message: Some("Risk configuration updated successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to update risk configuration".to_string()),
        })))
    }
}

/// Delete a risk configuration
/// DELETE /api/v1/risk-configs/{id}
pub async fn delete_risk_config(
    State(state): State<AppState>,
    Path(config_id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.delete_config(config_id).await {
        Ok(_) => Ok(Json(ApiResponse {
            success: true,
            data: None,
            message: Some("Risk configuration deleted successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Risk configuration not found or failed to delete".to_string()),
        })))
    }
}

/// Set a configuration as active
/// PUT /api/v1/risk-configs/{id}/activate
pub async fn activate_risk_config(
    State(state): State<AppState>,
    Path(config_id): Path<Uuid>,
) -> Result<Json<ApiResponse<RiskConfigResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.set_active_config(config_id).await {
        Ok(config) => Ok(Json(ApiResponse {
            success: true,
            data: Some(config.into()),
            message: Some("Risk configuration activated successfully".to_string()),
        })),
        Err(_) => Err((StatusCode::NOT_FOUND, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Risk configuration not found or failed to activate".to_string()),
        })))
    }
}

/// Initialize default risk configurations for a user
/// POST /api/v1/risk-configs/defaults
pub async fn initialize_default_configs(
    State(state): State<AppState>,
    Json(request): Json<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<RiskConfigResponse>>>, (StatusCode, Json<ApiResponse<()>>)> {
    let user_address = match request.get("user_address") {
        Some(addr) => addr,
        None => return Err((StatusCode::BAD_REQUEST, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("user_address is required".to_string()),
        })))
    };
    
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.initialize_default_configs(user_address).await {
        Ok(configs) => {
            let response_data: Vec<RiskConfigResponse> = configs.into_iter().map(|c| c.into()).collect();
            Ok(Json(ApiResponse {
                success: true,
                data: Some(response_data),
                message: Some("Default risk configurations initialized successfully".to_string()),
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to initialize default configurations".to_string()),
        })))
    }
}

/// Get risk parameters for calculation (active config or defaults)
/// GET /api/v1/risk-configs/{user_address}/params
pub async fn get_risk_params(
    State(state): State<AppState>,
    Path(user_address): Path<String>,
) -> Result<Json<ApiResponse<RiskParamsResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let service = UserRiskConfigService::new(state.db_pool.clone());
    
    match service.get_risk_params(&user_address).await {
        Ok(params) => {
            // Get active config name
            let active_profile = if let Ok(Some(config)) = service.get_active_config(&user_address).await {
                Some(config.profile_name)
            } else {
                None
            };
            
            Ok(Json(ApiResponse {
                success: true,
                data: Some(RiskParamsResponse {
                    user_address,
                    active_profile,
                    parameters: params,
                }),
                message: None,
            }))
        },
        Err(_) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse {
            success: false,
            data: None,
            message: Some("Failed to get risk parameters".to_string()),
        })))
    }
}

/// Create router for user risk configuration endpoints
pub fn create_user_risk_config_routes() -> Router<AppState> {
    Router::new()
        .route("/risk-configs", post(create_risk_config))
        .route("/risk-configs", get(get_risk_configs))
        .route("/risk-configs/:id", get(get_risk_config))
        .route("/risk-configs/:id", put(update_risk_config))
        .route("/risk-configs/:id", delete(delete_risk_config))
        .route("/risk-configs/:id/activate", put(activate_risk_config))
        .route("/risk-configs/defaults", post(initialize_default_configs))
        .route("/risk-configs/:user_address/params", get(get_risk_params))
}
