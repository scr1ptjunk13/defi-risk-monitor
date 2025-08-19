// Only include modules that actually exist
pub mod adapters;
pub mod handlers;
pub mod risk;
pub mod services;

// Removed missing modules:
// pub mod config;
// pub mod models;
// pub mod blockchain;
// pub mod error;
// pub mod security;
// pub mod auth;
// pub mod utils;
// pub mod database;
// pub mod comprehensive_test_demo;

// Removed broken error import:
// pub use error::types::*;

// Application state for Axum handlers - simplified to only include existing services
#[derive(Clone)]
pub struct AppState {
    pub blockchain_service: std::sync::Arc<services::BlockchainService>,
    // Removed all broken references to non-existent modules:
    // pub db_pool: sqlx::PgPool,
    // pub settings: config::Settings,
    // pub production_config: config::ProductionConfig,
    // pub config_manager: std::sync::Arc<tokio::sync::Mutex<config::ConfigManager>>,
    // pub websocket_service: Option<std::sync::Arc<services::websocket_service::WebSocketService>>,
    // pub real_time_service: Option<std::sync::Arc<services::real_time_risk_service::RealTimeRiskService>>,
    // pub jwt_service: std::sync::Arc<auth::jwt::JwtService>,
    // pub health_checker: std::sync::Arc<utils::monitoring::HealthChecker>,
}
