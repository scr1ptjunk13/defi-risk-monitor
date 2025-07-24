use defi_risk_monitor::{
    config::Settings,
    database::connection::establish_connection,
    services::monitoring_service::MonitoringService,
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
    
    // Establish database connection
    let db_pool = establish_connection(&settings.database.url).await?;
    info!("Database connection established");
    
    // Initialize monitoring service
    let monitoring_service = Arc::new(MonitoringService::new(db_pool.clone(), settings.clone())?);
    
    // Start the monitoring service
    let monitoring_handle = {
        let service = monitoring_service.clone();
        tokio::spawn(async move {
            if let Err(e) = service.start_monitoring().await {
                error!("Monitoring service error: {}", e);
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
    
    let app = Router::new()
        .route("/health", get(defi_risk_monitor::handlers::health::health_check))
        .route("/positions", get(defi_risk_monitor::handlers::positions::list_positions))
        .route("/positions/:id", get(defi_risk_monitor::handlers::positions::get_position))
        .route("/risk/calculate", get(defi_risk_monitor::handlers::risk::calculate_risk))
        .route("/alerts", get(defi_risk_monitor::handlers::alerts::list_alerts))
        .route("/alerts", post(defi_risk_monitor::handlers::alerts::create_alert))
        .layer(CorsLayer::permissive())
        .with_state(db_pool);
    
    let addr = SocketAddr::from(([0, 0, 0, 0], settings.api.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;
    Ok(())
}
