use defi_risk_monitor::{
    config::Settings,
    database::connection::establish_connection,
    services::{monitoring_service::MonitoringService, SystemHealthIntegration, websocket_service::WebSocketService, real_time_risk_service::RealTimeRiskService},
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
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
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
    
    // TEMPORARILY DISABLED: Start monitoring service in background
    // This prevents Infura rate limiting while testing wallet input
    let monitoring_handle = {
        tokio::spawn(async move {
            info!("Background monitoring disabled for testing");
            // let monitoring_service = MonitoringService::new(db_pool.clone(), settings.clone())?;
            // if let Err(e) = monitoring_service.start_monitoring().await {
            //     error!("Monitoring service failed: {}", e);
            // }
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
    
    // Wait for web server (monitoring disabled for testing)
    tokio::select! {
        _ = server_handle => {
            error!("Web server stopped unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
    }
    
    // Clean up the monitoring handle
    monitoring_handle.abort();
    
    info!("Shutting down DeFi Risk Monitor");
    Ok(())
}

async fn start_web_server(
    db_pool: sqlx::PgPool,
    settings: Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    use axum::{
        routing::{get, post, put, delete},
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
    
    // Initialize monitoring service with WebSocket integration
    let mut monitoring_service = MonitoringService::new(db_pool.clone(), settings.clone())
        .expect("Failed to initialize monitoring service");
    monitoring_service.set_websocket_service(websocket_service.clone());
    let monitoring_service = std::sync::Arc::new(monitoring_service);
    
    // Initialize real-time risk service
    let real_time_risk_service = RealTimeRiskService::new(
        monitoring_service.clone(),
        websocket_service.clone(),
    );
    
    // Start real-time risk monitoring in background
    let real_time_service_clone = real_time_risk_service.clone();
    tokio::spawn(async move {
        if let Err(e) = real_time_service_clone.start().await {
            eprintln!("Failed to start real-time risk service: {}", e);
        }
    });
    
    // Create JWT service
    let jwt_config = defi_risk_monitor::auth::jwt::JwtConfig::default();
    let jwt_service = std::sync::Arc::new(defi_risk_monitor::auth::jwt::JwtService::new(jwt_config));
    
    // TODO: Temporarily using basic initialization to focus on core functionality
    // The production config system will be properly integrated later
    println!("Warning: Using simplified config initialization for development");
    
    // Use default production config for now
    let production_config = defi_risk_monitor::config::ProductionConfig::default();
    
    // Create a simple config manager without complex initialization
    let config_manager = std::sync::Arc::new(tokio::sync::Mutex::new(
        defi_risk_monitor::config::ConfigManager::default()
    ));
    
    let health_checker = std::sync::Arc::new(defi_risk_monitor::utils::monitoring::HealthChecker::new("v1.0.0"));
    let real_time_service = None; // Optional service
    
    // Create application state
    let app_state = AppState {
        db_pool: db_pool.clone(),
        settings: settings.clone(),
        production_config,
        config_manager,
        blockchain_service,
        websocket_service: Some(websocket_service),
        real_time_service,
        jwt_service,
        health_checker,
    };
    
    use defi_risk_monitor::handlers::{
        create_auth_routes,
        create_position_routes,
        create_risk_routes,
        create_portfolio_routes,
        create_system_health_routes,
        create_monitoring_routes,
        create_price_feed_routes,
        create_demo_routes,
        portfolio_handlers::get_portfolio,
    };
    
    let app = Router::new()
        // Health and basic endpoints
        .route("/health", get(defi_risk_monitor::handlers::health::health_check))
        .route("/metrics", get(defi_risk_monitor::handlers::metrics::metrics_handler))
        
        // Legacy endpoints (keep for backward compatibility)
        .route("/api/v1/portfolio", get(get_portfolio))
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
        // Protocol Event Monitoring API
        .route("/api/v1/protocol-events", get(defi_risk_monitor::handlers::protocol_event_handlers::get_protocol_events))
        .route("/api/v1/protocol-events/stats", get(defi_risk_monitor::handlers::protocol_event_handlers::get_protocol_event_stats))
        .route("/api/v1/protocol-events/alerts", get(defi_risk_monitor::handlers::protocol_event_handlers::get_event_alerts))
        .route("/api/v1/protocol-events/alerts", post(defi_risk_monitor::handlers::protocol_event_handlers::create_event_alert))
        .route("/api/v1/protocol-events/alerts/:id", put(defi_risk_monitor::handlers::protocol_event_handlers::update_event_alert))
        .route("/api/v1/protocol-events/alerts/:id", delete(defi_risk_monitor::handlers::protocol_event_handlers::delete_event_alert))
        .route("/api/v1/protocol-events/:id", get(defi_risk_monitor::handlers::protocol_event_handlers::get_protocol_event))
        // Analytics API - LP Returns & Performance
        .route("/api/v1/analytics/lp-returns/:position_id", get(defi_risk_monitor::handlers::analytics_handlers::get_lp_returns))
        .route("/api/v1/analytics/pool-performance", get(defi_risk_monitor::handlers::analytics_handlers::get_pool_performance))
        .route("/api/v1/analytics/yield-farming", get(defi_risk_monitor::handlers::analytics_handlers::get_yield_farming_metrics))
        .route("/api/v1/analytics/farming-strategies", get(defi_risk_monitor::handlers::analytics_handlers::get_farming_strategies))
        .route("/api/v1/analytics/optimal-allocation", get(defi_risk_monitor::handlers::analytics_handlers::get_optimal_allocation))
        .route("/api/v1/analytics/compare-pools", get(defi_risk_monitor::handlers::analytics_handlers::compare_pools))
        .route("/api/v1/analytics/benchmark", get(defi_risk_monitor::handlers::analytics_handlers::get_benchmark_metrics))
        .route("/api/v1/analytics/rankings", get(defi_risk_monitor::handlers::analytics_handlers::get_performance_rankings))
        .route("/api/v1/analytics/lp-benchmark/:position_id", get(defi_risk_monitor::handlers::analytics_handlers::get_lp_benchmark))
        // Comprehensive REST API v1 endpoints
        .nest("/api/v1", create_auth_routes())
        .nest("/api/v1", create_position_routes())
        .nest("/api/v1", create_risk_routes())
        .nest("/api/v1", create_portfolio_routes())
        .nest("/api/v1", create_system_health_routes())
        .nest("/api/v1", create_monitoring_routes())
        .nest("/api/v1", create_price_feed_routes())
        .nest("/api/v1", create_demo_routes())
        
        // Existing specialized endpoints
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
    info!("Analytics & LP Performance:");
    info!("  GET    /api/v1/analytics/lp-returns/{{position_id}} - Get LP returns for position");
    info!("  GET    /api/v1/analytics/pool-performance?pool_address={{addr}}&chain_id={{id}} - Get pool performance metrics");
    info!("  GET    /api/v1/analytics/yield-farming?pool_address={{addr}}&chain_id={{id}} - Get yield farming metrics");
    info!("  GET    /api/v1/analytics/farming-strategies?investment_amount={{amt}}&risk_tolerance={{risk}} - Get farming strategies");
    info!("  GET    /api/v1/analytics/optimal-allocation?pools={{addrs}}&investment_amount={{amt}}&risk_tolerance={{risk}} - Get optimal allocation");
    info!("  GET    /api/v1/analytics/compare-pools?pools={{addrs}}&chain_id={{id}} - Compare multiple pools");
    info!("  GET    /api/v1/analytics/benchmark?pool_address={{addr}}&chain_id={{id}}&benchmark_type={{type}} - Get benchmark metrics");
    info!("  GET    /api/v1/analytics/rankings?chain_id={{id}} - Get performance rankings");
    info!("  GET    /api/v1/analytics/lp-benchmark/{{position_id}}?pools={{addrs}} - Benchmark LP performance");
    
    axum::serve(listener, app).await?;
    Ok(())
}
