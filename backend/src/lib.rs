pub mod config;
pub mod models;
pub mod services;
pub mod handlers;
pub mod database;
pub mod utils;
pub mod error;
pub mod security;
pub mod auth;
pub mod comprehensive_test_demo;


pub use error::types::*;

// Application state for Axum handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
    pub settings: config::Settings,
    pub production_config: config::ProductionConfig,
    pub config_manager: std::sync::Arc<tokio::sync::Mutex<config::ConfigManager>>,
    pub blockchain_service: std::sync::Arc<services::BlockchainService>,
    pub websocket_service: Option<std::sync::Arc<services::websocket_service::WebSocketService>>,
    pub real_time_service: Option<std::sync::Arc<services::real_time_risk_service::RealTimeRiskService>>,
    pub jwt_service: std::sync::Arc<auth::jwt::JwtService>,
    pub health_checker: std::sync::Arc<utils::monitoring::HealthChecker>,
}
