// Only include modules that actually exist
pub mod adapters;
pub mod health;

// Removed missing modules (cleaned up):
// pub mod handlers; - removed, starting fresh
// pub mod services; - removed, starting fresh
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

// Simplified AppState - no services needed for direct adapter approach
#[derive(Clone)]
pub struct AppState {
    // For now, we'll use direct adapter initialization in handlers
    // No complex service layer needed
    pub rpc_url: String,
    pub coingecko_api_key: Option<String>,
    // pub real_time_service: Option<std::sync::Arc<services::real_time_risk_service::RealTimeRiskService>>,
    // pub jwt_service: std::sync::Arc<auth::jwt::JwtService>,
    // pub health_checker: std::sync::Arc<utils::monitoring::HealthChecker>,
}
