
use tracing::{info, error};

use crate::config::Settings;
use crate::services::SystemHealthAlertingService;
use crate::error::AppError;

/// Integration service to start system health monitoring
pub struct SystemHealthIntegration {
    #[allow(dead_code)]
    alerting_service: SystemHealthAlertingService,
}

impl SystemHealthIntegration {
    pub fn new(settings: &Settings) -> Self {
        Self {
            alerting_service: SystemHealthAlertingService::new(settings),
        }
    }

    /// Start system health monitoring in background
    pub async fn start_background_monitoring(settings: Settings) -> Result<(), AppError> {
        info!("Starting system health monitoring integration");
        
        let mut alerting_service = SystemHealthAlertingService::new(&settings);
        
        // Start monitoring in background task
        tokio::spawn(async move {
            if let Err(e) = alerting_service.start_monitoring().await {
                error!("System health monitoring failed: {}", e);
            }
        });
        
        info!("System health monitoring started successfully");
        Ok(())
    }
}
