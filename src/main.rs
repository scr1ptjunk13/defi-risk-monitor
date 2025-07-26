use defi_risk_monitor::{
    config::Settings,
    database::connection::establish_connection,
    services::{monitoring_service::MonitoringService, SystemHealthIntegration, websocket_service::WebSocketService},
    utils::monitoring::{init_metrics, HealthChecker},
    handlers::{create_alert_routes, create_user_risk_config_routes, create_webhook_routes},
    AppState,
};
use std::sync::Arc;
use tokio;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("Starting DeFi Risk Monitor");
    
    // Load configuration
    let settings = Settings::new()?;
    info!("Configuration loaded successfully");
    
    // Initialize metrics system
    init_metrics().await?;
    info!("Metrics system initialized");
    
    // Initialize health checker
    let _health_checker = Arc::new(HealthChecker::new("1.0.0"));
    
    // Establish database connection
    let db_pool = establish_connection(&settings.database.url).await?;
    info!("Database connection established");
    
    // Initialize monitoring service
    let _monitoring_service = Arc::new(MonitoringService::new(db_pool.clone(), settings.clone())?);
    
    // Start system health monitoring
    SystemHealthIntegration::start_background_monitoring(settings.clone()).await?;
    
    // Start monitoring service in background
    let monitoring_handle = {
        let monitoring_service = MonitoringService::new(db_pool.clone(), settings.clone())?;
        tokio::spawn(async move {
            if let Err(e) = monitoring_service.start_monitoring().await {
                error!("Monitoring service failed: {}", e);
            }
        })
    };
    
    // Start the web server
    let server_handle = {
        let pool = db_pool.clone();
        let config = settings.clone();
        tokio::spawn(async move {
            if let Err(e) = start_web_server(pool, config).await {
                error!("Web server error: {}", e);
            }
        })
    };
    
    info!("DeFi Risk Monitor started successfully");
    info!("API server running on {}:{}", settings.api.host, settings.api.port);
    
    // Wait for both services
    tokio::select! {
        _ = monitoring_handle => {
            error!("Monitoring service stopped unexpectedly");
        }
        _ = server_handle => {
            error!("Web server stopped unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }
    
    info!("Shutting down DeFi Risk Monitor");
    Ok(())
}

async fn start_web_server(
    db_pool: sqlx::PgPool,
    settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        routing::{get, post},
        Router,
    };
    use std::net::SocketAddr;
    use tower_http::cors::CorsLayer;
    
    // Initialize blockchain service
    let blockchain_service = std::sync::Arc::new(
        defi_risk_monitor::services::BlockchainService::new(&settings, db_pool.clone())
            .expect("Failed to initialize blockchain service")
    );
    
    // Initialize WebSocket service
    let websocket_service = std::sync::Arc::new(WebSocketService::new());
    
    // Start WebSocket heartbeat task
    websocket_service.start_heartbeat_task();
    
    // Create application state
    let app_state = AppState {
        db_pool: db_pool.clone(),
        settings: settings.clone(),
        blockchain_service,
        websocket_service,
    };
    
    let app = Router::new()
        // Health and basic endpoints
        .route("/health", get(defi_risk_monitor::handlers::health::health_check))
        .route("/risk/calculate", get(defi_risk_monitor::handlers::risk::calculate_risk))
        .route("/risk/calculate-realtime", get(defi_risk_monitor::handlers::risk::calculate_real_time_risk))
        .route("/alerts", get(defi_risk_monitor::handlers::alerts::list_alerts))
        .route("/alerts", post(defi_risk_monitor::handlers::alerts::create_alert))
        // WebSocket endpoints for real-time streaming
        .route("/ws/positions/:id/risk-stream", get(defi_risk_monitor::handlers::websocket_handlers::position_risk_stream))
        .route("/ws/alerts/live-feed", get(defi_risk_monitor::handlers::websocket_handlers::alerts_live_feed))
        .route("/ws/positions/:id/value-stream", get(defi_risk_monitor::handlers::websocket_handlers::position_value_stream))
        .route("/ws/market/:token_address/stream", get(defi_risk_monitor::handlers::websocket_handlers::market_data_stream))
        .route("/ws/system/status", get(defi_risk_monitor::handlers::websocket_handlers::system_status_stream))
        .route("/ws/stream", get(defi_risk_monitor::handlers::websocket_handlers::general_stream))
        // Risk Explainability endpoints
        .route("/api/v1/positions/:id/explain-risk", get(defi_risk_monitor::handlers::risk_explainability_handlers::explain_position_risk))
        .route("/api/v1/positions/:id/risk-summary", get(defi_risk_monitor::handlers::risk_explainability_handlers::get_risk_summary))
        .route("/api/v1/positions/:id/recommendations", get(defi_risk_monitor::handlers::risk_explainability_handlers::get_risk_recommendations))
        .route("/api/v1/positions/:id/market-context", get(defi_risk_monitor::handlers::risk_explainability_handlers::get_market_context))
        // API v1 endpoints
        .nest("/api/v1", create_alert_routes())
        .nest("/api/v1/user-risk-config", create_user_risk_config_routes())
        .nest("/api/v1", create_webhook_routes())
        .layer(CorsLayer::permissive())
        .with_state(app_state);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], settings.api.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    info!("API endpoints available at:");
    info!("Position Management:");
    info!("  GET    /api/v1/positions - List positions");
    info!("  POST   /api/v1/positions - Create position");
    info!("  GET    /api/v1/positions/{{id}} - Get position");
    info!("  PUT    /api/v1/positions/{{id}} - Update position");
    info!("  DELETE /api/v1/positions/{{id}} - Delete position");
    info!("  GET    /api/v1/positions/stats - Get position statistics");
    info!("Alert Thresholds:");
    info!("  POST   /api/v1/thresholds - Create threshold");
    info!("  GET    /api/v1/thresholds - List thresholds");
    info!("  GET    /api/v1/thresholds/{{id}} - Get threshold");
    info!("  PUT    /api/v1/thresholds/{{id}} - Update threshold");
    info!("  DELETE /api/v1/thresholds/{{id}} - Delete threshold");
    info!("  POST   /api/v1/thresholds/defaults - Initialize defaults");
    info!("  GET    /api/v1/thresholds/stats/{{user}} - Get stats");
    info!("  GET    /api/v1/alerts - List alerts");
    info!("  PUT    /api/v1/alerts/{{id}}/resolve - Resolve alert");
    info!("Webhooks:");
    info!("  GET    /api/v1/webhooks - List webhooks");
    info!("  POST   /api/v1/webhooks - Create webhook");
    info!("  GET    /api/v1/webhooks/{{id}} - Get webhook");
    info!("  PUT    /api/v1/webhooks/{{id}} - Update webhook");
    info!("  DELETE /api/v1/webhooks/{{id}} - Delete webhook");
    info!("  POST   /api/v1/webhooks/{{id}}/test - Test webhook");
    info!("  GET    /api/v1/webhooks/event-types - Get event types");
    info!("  GET    /api/v1/webhooks/{{id}}/deliveries - Get delivery history");
    
    axum::serve(listener, app).await?;
    Ok(())
}
