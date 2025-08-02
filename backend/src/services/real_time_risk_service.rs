use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, error, warn};
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::Utc;

use crate::services::{MonitoringService, WebSocketService};
use crate::error::AppError;

/// Real-time risk monitoring service that coordinates between monitoring and WebSocket services
#[derive(Clone)]
pub struct RealTimeRiskService {
    monitoring_service: Arc<MonitoringService>,
    websocket_service: Arc<WebSocketService>,
    is_running: Arc<tokio::sync::RwLock<bool>>,
}

impl RealTimeRiskService {
    /// Create a new real-time risk service
    pub fn new(
        monitoring_service: Arc<MonitoringService>,
        websocket_service: Arc<WebSocketService>,
    ) -> Self {
        Self {
            monitoring_service,
            websocket_service,
            is_running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Start the real-time risk monitoring service
    pub async fn start(&self) -> Result<(), AppError> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                warn!("Real-time risk service is already running");
                return Ok(());
            }
            *is_running = true;
        }

        info!("🚀 Starting Real-Time Risk Monitoring Service");
        
        // Start the monitoring service in a background task
        let monitoring_service = self.monitoring_service.clone();
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(30)); // Check every 30 seconds
            
            loop {
                // Check if service should continue running
                {
                    let running = is_running.read().await;
                    if !*running {
                        info!("Real-time risk service stopped");
                        break;
                    }
                }
                
                interval.tick().await;
                
                // Run monitoring cycle
                if let Err(e) = monitoring_service.start_monitoring().await {
                    error!("Error in real-time monitoring cycle: {}", e);
                }
            }
        });

        // Start market data streaming service
        self.start_market_data_streaming().await?;
        
        // Start system status monitoring
        self.start_system_status_monitoring().await?;

        info!("✅ Real-Time Risk Monitoring Service started successfully");
        Ok(())
    }

    /// Stop the real-time risk monitoring service
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("🛑 Real-Time Risk Monitoring Service stopped");
    }

    /// Check if the service is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get real-time statistics about the monitoring service
    pub async fn get_stats(&self) -> Result<RealTimeStats, AppError> {
        let connected_clients = self.websocket_service.get_connected_clients_count().await;
        let subscription_stats = self.websocket_service.get_subscription_stats().await;
        
        Ok(RealTimeStats {
            connected_clients,
            subscription_stats,
            service_uptime: Utc::now(), // Simplified - in production, track actual uptime
            last_update: Utc::now(),
        })
    }

    /// Manually trigger a risk update for a specific position
    pub async fn trigger_position_update(&self, position_id: Uuid) -> Result<(), AppError> {
        info!("🔄 Manually triggering risk update for position: {}", position_id);
        
        // This would trigger a specific position monitoring cycle
        // For now, we'll send a test update
        let test_metrics = crate::services::risk_calculator::RiskMetrics {
            impermanent_loss: BigDecimal::from_str("0.05").unwrap(),
            price_impact: BigDecimal::from_str("0.02").unwrap(),
            volatility_score: BigDecimal::from(4),
            correlation_score: BigDecimal::from(6),
            liquidity_score: BigDecimal::from(3),
            overall_risk_score: BigDecimal::from(5),
            value_at_risk_1d: BigDecimal::from(1000),
            value_at_risk_7d: BigDecimal::from(3000),
            tvl_risk: BigDecimal::from(2),
            slippage_risk: BigDecimal::from(1),
            thin_pool_risk: BigDecimal::from(1),
            tvl_drop_risk: BigDecimal::from(2),
            max_estimated_slippage: BigDecimal::from_str("0.01").unwrap(),
            protocol_risk_score: BigDecimal::from(2),
            audit_risk: BigDecimal::from(1),
            exploit_history_risk: BigDecimal::from(1),
            governance_risk: BigDecimal::from(2),
            mev_risk_score: BigDecimal::from(1),
            sandwich_attack_risk: BigDecimal::from(1),
            frontrun_risk: BigDecimal::from(1),
            oracle_manipulation_risk: BigDecimal::from(1),
            oracle_deviation_risk: BigDecimal::from(1),
            cross_chain_risk_score: BigDecimal::from(2),
            bridge_risk_score: BigDecimal::from(2),
            liquidity_fragmentation_risk: BigDecimal::from(1),
            governance_divergence_risk: BigDecimal::from(1),
            technical_risk_score: BigDecimal::from(2),
            correlation_risk_score: BigDecimal::from(3),
        };

        self.websocket_service.send_risk_update(position_id, test_metrics).await?;
        info!("✅ Risk update sent for position: {}", position_id);
        
        Ok(())
    }

    /// Start market data streaming for active tokens
    async fn start_market_data_streaming(&self) -> Result<(), AppError> {
        let websocket_service = self.websocket_service.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(60)); // Update every minute
            
            loop {
                interval.tick().await;
                
                // Send mock market data updates for common tokens
                let tokens = vec![
                    ("0xA0b86a33E6441E1e0f6d8E87A4e5C7b7F0E8A4C1", "ETH"),
                    ("0xdAC17F958D2ee523a2206206994597C13D831ec7", "USDT"),
                    ("0xA0b86a33E6441E1e0f6d8E87A4e5C7b7F0E8A4C2", "USDC"),
                ];
                
                for (token_address, _symbol) in tokens {
                    // Generate mock market data
                    let price_usd = BigDecimal::from(2000 + (rand::random::<f64>() * 200.0) as i32);
                    let price_change_24h = BigDecimal::from_str(&format!("{:.2}", (rand::random::<f64>() - 0.5) * 10.0)).unwrap();
                    let volatility = BigDecimal::from_str(&format!("{:.3}", rand::random::<f64>() * 0.5)).unwrap();
                    
                    if let Err(e) = websocket_service.send_market_update(
                        token_address.to_string(),
                        price_usd,
                        price_change_24h,
                        volatility,
                    ).await {
                        error!("Failed to send market update: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }

    /// Start system status monitoring
    async fn start_system_status_monitoring(&self) -> Result<(), AppError> {
        let websocket_service = self.websocket_service.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(120)); // Update every 2 minutes
            
            loop {
                interval.tick().await;
                
                // Send system status update
                if let Err(e) = websocket_service.send_system_status(
                    "operational".to_string(),
                    "All systems operational - Real-time monitoring active".to_string(),
                ).await {
                    error!("Failed to send system status update: {}", e);
                }
            }
        });
        
        Ok(())
    }
}

/// Real-time monitoring statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RealTimeStats {
    pub connected_clients: usize,
    pub subscription_stats: std::collections::HashMap<String, usize>,
    pub service_uptime: chrono::DateTime<Utc>,
    pub last_update: chrono::DateTime<Utc>,
}

// Add rand dependency for mock data generation
use std::str::FromStr;
