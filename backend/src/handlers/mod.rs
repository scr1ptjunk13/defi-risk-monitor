pub mod health;
pub mod metrics;
pub mod positions;
pub mod risk;
pub mod alerts;
pub mod alert_handlers;
pub mod user_risk_config_handlers;
pub mod webhook_handlers;
pub mod websocket_handlers;
pub mod risk_explainability_handlers;
pub mod protocol_event_handlers;
pub mod analytics_handlers;
pub mod portfolio_handlers;

// New comprehensive API handlers
pub mod auth_handlers;
pub mod position_handlers;
pub mod risk_handlers;
pub mod portfolio_handlers_complete;
pub mod system_health_handlers;
pub mod monitoring_handlers;
pub mod price_feed_handlers;

// Explicitly re-export only what's needed from each module
pub use health::health_check;
pub use metrics::metrics_handler;
// Note: positions module may not have these specific handlers
// pub use positions::{
//     create_position_handler,
//     get_position_handler,
//     update_position_handler,
//     delete_position_handler,
//     list_positions_handler,
// };
// Note: risk and alerts modules may not have these specific handlers
// pub use risk::calculate_position_risk_handler;
// pub use alerts::{
//     create_alert_handler,
//     get_alert_handler,
//     list_alerts_handler,
//     update_alert_handler,
//     delete_alert_handler,
// };
pub use alert_handlers::{
    create_threshold,
    get_thresholds,
    get_threshold,
    update_threshold,
    delete_threshold,
    get_threshold_stats,
    get_alerts,
    resolve_alert,
    create_alert_routes,
    CreateThresholdRequest,
    UpdateThresholdRequest,
    ThresholdResponse,
    AlertResponse,
    GetThresholdsQuery,
    GetAlertsQuery,
    ApiResponse,
    PaginatedResponse,
};
pub use user_risk_config_handlers::create_user_risk_config_routes;
pub use webhook_handlers::create_webhook_routes;

// Export new comprehensive API route creators
pub use auth_handlers::create_auth_routes;
pub use position_handlers::create_position_routes;
pub use risk_handlers::create_risk_routes;
pub use portfolio_handlers_complete::create_portfolio_routes;
pub use system_health_handlers::create_system_health_routes;
pub use monitoring_handlers::create_monitoring_routes;
pub use price_feed_handlers::create_price_feed_routes;
