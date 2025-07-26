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
