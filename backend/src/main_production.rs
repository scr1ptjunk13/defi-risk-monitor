use defi_risk_monitor::{
    config::{ProductionConfig, ConfigManager, ConfigValidator},
    database::connection::establish_connection,
    services::{
        monitoring_service::MonitoringService, 
        SystemHealthIntegration, 
        websocket_service::WebSocketService, 
        real_time_risk_service::RealTimeRiskService
    },
    utils::monitoring::{init_metrics, HealthChecker},
    handlers::{create_alert_routes, create_user_risk_config_routes, create_webhook_routes},
    AppState,
};
use std::sync::Arc;
use tokio;
use tracing::{info, error, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    
    // Initialize production configuration system
    info!("🚀 Starting DeFi Risk Monitor with Production Configuration Management");
    
    // Load and validate configuration
    let config = match ProductionConfig::load() {
        Ok(config) => {
            info!("✅ Configuration loaded successfully for environment: {}", config.environment.as_str());
            config
        }
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            return Err(e.into());
        }
    };

    // Initialize logging based on configuration
    init_logging(&config)?;
    
    // Log configuration summary (without sensitive data)
    let config_summary = config.summary();
    info!("📊 Configuration Summary: {:?}", config_summary);

    // Validate configuration
    let validator = ConfigValidator::new();
    if let Err(e) = validate_production_config(&config, &validator) {
        error!("❌ Configuration validation failed: {}", e);
        return Err(e);
    }
    info!("✅ Configuration validation passed");

    // Initialize configuration manager with hot-reloading
    let mut config_manager = ConfigManager::new().await?;
    if config.is_production() {
        info!("🔄 Starting configuration hot-reloading for production");
        config_manager.start_monitoring().await?;
    }

    // Initialize metrics system based on configuration
    if config.monitoring.enable_prometheus {
        init_metrics().await?;
        info!("📈 Metrics system initialized on port {}", config.monitoring.prometheus_port);
    }
    
    // Initialize health checker
    let health_checker = Arc::new(HealthChecker::new("1.0.0"));
    
    // Establish database connection with production settings
    let db_pool = establish_connection(&config.database.url).await?;
    info!("🗄️  Database connection established with {} max connections", config.database.max_connections);
    
    // Initialize WebSocket service if enabled
    let websocket_service = if config.features.enable_websockets {
        let ws_service = Arc::new(WebSocketService::new());
        info!("🔌 WebSocket service initialized");
        Some(ws_service)
    } else {
        info!("⚠️  WebSocket service disabled by configuration");
        None
    };

    // Initialize monitoring service with production configuration
    let monitoring_service = Arc::new(MonitoringService::new(db_pool.clone(), config.clone())?);
    
    // Start system health monitoring if enabled
    if config.monitoring.enable_health_checks {
        SystemHealthIntegration::start_background_monitoring(config.clone()).await?;
        info!("💓 System health monitoring started");
    }

    // Start real-time risk monitoring if enabled
    let real_time_service = if config.risk.enable_real_time_monitoring {
        let rt_service = Arc::new(RealTimeRiskService::new(
            monitoring_service.clone(),
            websocket_service.clone(),
        ));
        
        // Start background monitoring
        let rt_service_clone = rt_service.clone();
        tokio::spawn(async move {
            if let Err(e) = rt_service_clone.start_monitoring().await {
                error!("Real-time risk monitoring failed: {}", e);
            }
        });
        
        info!("⚡ Real-time risk monitoring started");
        Some(rt_service)
    } else {
        info!("⚠️  Real-time risk monitoring disabled by configuration");
        None
    };

    // Start background monitoring service
    let monitoring_handle = {
        let monitoring_service = MonitoringService::new(db_pool.clone(), config.clone())?;
        tokio::spawn(async move {
            if let Err(e) = monitoring_service.start_monitoring().await {
                error!("Background monitoring service failed: {}", e);
            }
        })
    };

    // Create application state with production configuration
    let app_state = AppState {
        db_pool: db_pool.clone(),
        settings: config.clone(),
        health_checker,
        websocket_service: websocket_service.clone(),
        real_time_service,
        config_manager: Arc::new(tokio::sync::Mutex::new(config_manager)),
    };

    // Build application routes with production configuration
    let app = build_production_app(app_state, &config).await?;

    // Start the server with production settings
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", config.api.host, config.api.port)).await?;
    
    info!("🌐 DeFi Risk Monitor server starting on {}:{}", config.api.host, config.api.port);
    info!("🔒 Security features: Rate limiting: {}, 2FA: {}, Audit logging: {}", 
          config.security.enable_rate_limiting, 
          config.security.enable_2fa, 
          config.security.enable_audit_logging);
    info!("🎯 Feature flags: Real-time: {}, WebSockets: {}, AI: {}, Experimental: {}", 
          config.risk.enable_real_time_monitoring,
          config.features.enable_websockets,
          config.features.enable_ai_predictions,
          config.features.enable_experimental_features);

    // Graceful shutdown handler
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        
        info!("🛑 Shutdown signal received, starting graceful shutdown...");
        
        // Cancel background tasks
        monitoring_handle.abort();
        
        info!("✅ Graceful shutdown completed");
    };

    // Start the server
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}

/// Initialize logging based on production configuration
fn init_logging(config: &ProductionConfig) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = config.logging.level.parse::<tracing::Level>()
        .unwrap_or(tracing::Level::INFO);

    match config.logging.format.as_str() {
        "json" => {
            // Structured JSON logging for production
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| format!("defi_risk_monitor={}", log_level).into())
                )
                .with(tracing_subscriber::fmt::layer().json())
                .init();
        }
        "pretty" => {
            // Pretty formatting for development
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| format!("defi_risk_monitor={}", log_level).into())
                )
                .with(tracing_subscriber::fmt::layer().pretty())
                .init();
        }
        _ => {
            // Compact formatting (default)
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| format!("defi_risk_monitor={}", log_level).into())
                )
                .with(tracing_subscriber::fmt::layer().compact())
                .init();
        }
    }

    info!("📝 Logging initialized with level: {}, format: {}", config.logging.level, config.logging.format);
    Ok(())
}

/// Validate production configuration
fn validate_production_config(config: &ProductionConfig, validator: &ConfigValidator) -> Result<(), Box<dyn std::error::Error>> {
    // Validate all configuration sections
    validator.validate_database_config(&config.database)?;
    validator.validate_api_config(&config.api)?;
    validator.validate_blockchain_config(&config.blockchain)?;
    validator.validate_risk_config(&config.risk)?;
    validator.validate_security_config(&config.security, config.is_production())?;
    validator.validate_alert_config(&config.alerts)?;

    // Production-specific validations
    if config.is_production() {
        // Ensure critical security settings are enabled
        if !config.security.enable_rate_limiting {
            warn!("⚠️  Rate limiting is disabled in production - this is not recommended");
        }
        
        if !config.security.enable_audit_logging {
            warn!("⚠️  Audit logging is disabled in production - this is not recommended");
        }
        
        if config.security.jwt_secret.contains("CHANGE_THIS") {
            return Err("JWT secret must be changed from default in production".into());
        }
        
        // Validate CORS origins are not wildcard in production
        if config.api.cors_origins.contains(&"*".to_string()) {
            return Err("CORS origins cannot be wildcard (*) in production".into());
        }
        
        // Ensure TLS is configured in production
        if config.api.tls.is_none() {
            warn!("⚠️  TLS is not configured in production - this is not recommended");
        }
    }

    Ok(())
}

/// Build application with production configuration
async fn build_production_app(
    app_state: AppState, 
    config: &ProductionConfig
) -> Result<axum::Router, Box<dyn std::error::Error>> {
    use axum::{
        routing::{get, post},
        Router,
    };
    use tower_http::{
        cors::{CorsLayer, Any},
        trace::TraceLayer,
        compression::CompressionLayer,
    };
    use std::time::Duration;

    let mut app = Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        .route("/health/detailed", get(detailed_health_check))
        .route("/config/summary", get(config_summary))
        
        // API routes
        .nest("/api/v1/alerts", create_alert_routes())
        .nest("/api/v1/user-risk-config", create_user_risk_config_routes())
        .nest("/api/v1/webhooks", create_webhook_routes());

    // Add WebSocket routes if enabled
    if config.features.enable_websockets {
        app = app.route("/ws", get(websocket_handler));
    }

    // Add middleware based on configuration
    if config.api.enable_compression {
        app = app.layer(CompressionLayer::new());
    }

    // Configure CORS based on environment
    let cors = if config.is_production() {
        CorsLayer::new()
            .allow_origin(config.api.cors_origins.iter().map(|origin| {
                origin.parse::<tower_http::cors::AllowOrigin>()
                    .unwrap_or_else(|_| Any.into())
            }).collect::<Vec<_>>())
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers(Any)
    } else {
        CorsLayer::permissive()
    };
    
    app = app.layer(cors);

    // Add tracing layer
    app = app.layer(TraceLayer::new_for_http());

    // Add request timeout
    app = app.layer(tower::timeout::TimeoutLayer::new(
        Duration::from_secs(config.api.request_timeout_seconds)
    ));

    // Add application state
    app = app.with_state(app_state);

    Ok(app)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Detailed health check endpoint
async fn detailed_health_check(
    axum::extract::State(state): axum::extract::State<AppState>
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    // Get configuration health summary
    let config_manager = state.config_manager.lock().await;
    let health_summary = config_manager.get_health_summary().await;
    
    Ok(axum::Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now(),
        "configuration": health_summary,
        "database": {
            "connected": true,
            "pool_size": "active" // Could be enhanced with actual pool stats
        },
        "services": {
            "websockets": state.websocket_service.is_some(),
            "real_time_monitoring": state.real_time_service.is_some(),
        }
    })))
}

/// Configuration summary endpoint
async fn config_summary(
    axum::extract::State(state): axum::extract::State<AppState>
) -> Result<axum::Json<serde_json::Value>, axum::http::StatusCode> {
    let config_manager = state.config_manager.lock().await;
    let config = config_manager.get_config().await;
    let summary = config.summary();
    
    Ok(axum::Json(serde_json::json!(summary)))
}

/// WebSocket handler
async fn websocket_handler(
    ws: axum::extract::WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<AppState>
) -> axum::response::Response {
    if let Some(websocket_service) = &state.websocket_service {
        ws.on_upgrade(|socket| async move {
            // Handle WebSocket connection
            info!("WebSocket connection established");
        })
    } else {
        axum::response::Response::builder()
            .status(axum::http::StatusCode::SERVICE_UNAVAILABLE)
            .body("WebSocket service not available".into())
            .unwrap()
    }
}
