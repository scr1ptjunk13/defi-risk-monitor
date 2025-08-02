use crate::models::{Position, PoolState, RiskConfig, Alert, CreateAlert, AlertSeverity};
use crate::services::{BlockchainService, RiskCalculator, AlertService};
use crate::services::websocket_service::WebSocketService;
use crate::config::Settings;
use crate::error::AppError;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, error};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitoringStats {
    pub total_positions_monitored: i64,
    pub active_thresholds: i64,
    pub alerts_last_24h: i64,
    pub critical_alerts_active: i64,
    pub avg_response_time_ms: BigDecimal,
    pub uptime_percentage: BigDecimal,
    pub last_check: DateTime<Utc>,
}

pub struct MonitoringService {
    db_pool: PgPool,
    blockchain_service: Arc<BlockchainService>,
    risk_calculator: Arc<RiskCalculator>,
    alert_service: Arc<AlertService>,
    websocket_service: Option<Arc<WebSocketService>>,
    settings: Settings,
}

impl MonitoringService {
    pub fn new(db_pool: PgPool, settings: Settings) -> Result<Self, AppError> {
        let blockchain_service = Arc::new(BlockchainService::new(&settings, db_pool.clone())?);
        let risk_calculator = Arc::new(RiskCalculator::new());
        let alert_service = Arc::new(AlertService::new(&settings));

        Ok(Self {
            db_pool,
            blockchain_service,
            risk_calculator,
            alert_service,
            websocket_service: None,
            settings,
        })
    }

    /// Set the WebSocket service for real-time updates
    pub fn set_websocket_service(&mut self, websocket_service: Arc<WebSocketService>) {
        self.websocket_service = Some(websocket_service);
    }

    pub async fn start_monitoring(&self) -> Result<(), AppError> {
        info!("Starting risk monitoring service");
        
        let mut interval = time::interval(Duration::from_secs(
            self.settings.blockchain.risk_check_interval_seconds
        ));

        loop {
            interval.tick().await;
            
            if let Err(e) = self.monitor_all_positions().await {
                error!("Error during monitoring cycle: {}", e);
            }
        }
    }

    async fn monitor_all_positions(&self) -> Result<(), AppError> {
        info!("Starting monitoring cycle");

        // Fetch all active positions
        let positions = self.fetch_all_positions().await?;
        info!("Monitoring {} positions", positions.len());

        for position in positions {
            if let Err(e) = self.monitor_position(&position).await {
                error!("Error monitoring position {}: {}", position.id, e);
            }
        }

        info!("Completed monitoring cycle");
        Ok(())
    }

    async fn monitor_position(&self, position: &Position) -> Result<(), AppError> {
        use chrono::{Duration, Utc};
        use crate::services::price_storage::PriceStorageService;

        // Fetch current pool state
        let pool_state = self.blockchain_service
            .get_pool_state(&position.pool_address, position.chain_id)
            .await?;

        // Store pool state
        self.store_pool_state(&pool_state).await?;

        // Fetch historical pool state data
        let historical_data = self.fetch_historical_pool_data(
            &position.pool_address,
            position.chain_id,
            30, // Last 30 data points
        ).await?;

        // Fetch historical price data for both tokens
        let price_storage = PriceStorageService::new(self.db_pool.clone());
        let now = Utc::now();
        let lookback = now - Duration::days(7);
        let token0_price_history = price_storage.get_history(
            &position.token0_address,
            position.chain_id,
            lookback,
            now,
        ).await?;
        let token1_price_history = price_storage.get_history(
            &position.token1_address,
            position.chain_id,
            lookback,
            now,
        ).await?;

        // Fetch risk configuration for the user
        let risk_config = self.fetch_risk_config(&position.user_address).await?;

        // Calculate risk metrics with historical price data
        let risk_metrics = self.risk_calculator.calculate_position_risk(
            position,
            &pool_state,
            &risk_config,
            &historical_data,
            &token0_price_history,
            &token1_price_history,
            None, // No protocol name available in this context
            None, // No user risk params in monitoring context
        ).await?;

        // Store risk metrics in database
        self.store_risk_metrics(position.id, &risk_metrics).await?;

        // Send real-time risk update via WebSocket
        if let Some(ref websocket_service) = self.websocket_service {
            if let Err(e) = websocket_service.send_risk_update(position.id, risk_metrics.clone()).await {
                error!("Failed to send WebSocket risk update for position {}: {}", position.id, e);
            }
        }

        // Calculate and send position value update
        let current_value_usd = self.calculate_position_value(&position, &pool_state).await?;
        // Calculate P&L based on entry prices if available, otherwise use current value
        let initial_value = self.calculate_initial_position_value(&position).await.unwrap_or_else(|_| current_value_usd.clone());
        let pnl_usd = current_value_usd.clone() - &initial_value;
        let impermanent_loss_pct = risk_metrics.impermanent_loss.clone();
        
        if let Some(ref websocket_service) = self.websocket_service {
            if let Err(e) = websocket_service.send_position_update(
                position.id,
                current_value_usd,
                pnl_usd,
                impermanent_loss_pct,
            ).await {
                error!("Failed to send WebSocket position update for position {}: {}", position.id, e);
            }
        }

        // Check for risk threshold violations
        let risk_config = self.fetch_risk_config(&position.user_address).await?;
        if let Some(violation) = self.check_risk_thresholds(&risk_metrics, &risk_config) {
            // Create and store alert
            let alert = CreateAlert {
                user_address: position.user_address.clone(),
                position_id: Some(position.id),
                threshold_id: None,
                alert_type: "risk_threshold_exceeded".to_string(),
                severity: self.determine_alert_severity(&risk_metrics),
                title: "Risk Threshold Exceeded".to_string(),
                message: violation,
                risk_score: Some(risk_metrics.overall_risk_score.clone()),
                current_value: None,
                threshold_value: None,
                metadata: None,
            };

            let alert = Alert::new(alert);
            self.store_alert(&alert).await?;
            
            // Send alert notification via traditional service
            if let Err(e) = self.alert_service.send_alert(&alert).await {
                error!("Failed to send alert: {}", e);
            }
            
            // Send real-time alert via WebSocket
            if let Some(ref websocket_service) = self.websocket_service {
                if let Err(e) = websocket_service.send_alert(alert).await {
                    error!("Failed to send WebSocket alert for position {}: {}", position.id, e);
                }
            }
        }

        Ok(())
    }

    async fn fetch_all_positions(&self) -> Result<Vec<Position>, AppError> {
        let positions = sqlx::query_as::<_, Position>(
            "SELECT * FROM positions ORDER BY created_at DESC"
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(positions)
    }

    async fn store_pool_state(&self, pool_state: &PoolState) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO pool_states (
                id, pool_address, chain_id, current_tick, sqrt_price_x96, 
                liquidity, token0_price_usd, token1_price_usd, tvl_usd, 
                volume_24h_usd, fees_24h_usd, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (pool_address, chain_id, timestamp) DO NOTHING
            "#,
            pool_state.id,
            pool_state.pool_address,
            pool_state.chain_id,
            pool_state.current_tick,
            pool_state.sqrt_price_x96,
            pool_state.liquidity,
            pool_state.token0_price_usd,
            pool_state.token1_price_usd,
            pool_state.tvl_usd,
            pool_state.volume_24h_usd,
            pool_state.fees_24h_usd,
            pool_state.timestamp
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn fetch_historical_pool_data(
        &self,
        pool_address: &str,
        chain_id: i32,
        limit: i32,
    ) -> Result<Vec<PoolState>, AppError> {
        let pool_states = sqlx::query_as::<_, PoolState>(
            r#"
            SELECT * FROM pool_states 
            WHERE pool_address = $1 AND chain_id = $2 
            ORDER BY timestamp DESC 
            LIMIT $3
            "#
        )
        .bind(pool_address)
        .bind(chain_id)
        .bind(limit)
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(pool_states)
    }

    async fn fetch_risk_config(&self, user_address: &str) -> Result<RiskConfig, AppError> {
        // First, get the user_id from the user_addresses table
        let user_result = sqlx::query!(
            r#"
            SELECT u.id as user_id
            FROM users u 
            JOIN user_addresses ua ON u.id = ua.user_id 
            WHERE LOWER(ua.address) = LOWER($1)
            LIMIT 1
            "#,
            user_address
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let user_id = user_result
            .ok_or_else(|| AppError::NotFound(format!("User not found for address {}", user_address)))?
            .user_id;

        // Now get the risk preferences for this user_id
        let risk_prefs = sqlx::query!(
            r#"
            SELECT 
                max_position_size_usd,
                max_protocol_allocation_percent,
                max_single_pool_percent,
                min_liquidity_threshold_usd,
                max_risk_score,
                max_slippage_percent,
                stop_loss_threshold_percent
            FROM user_risk_preferences 
            WHERE user_id = $1
            "#,
            user_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Convert user_risk_preferences to RiskConfig format
        if let Some(prefs) = risk_prefs {
            let risk_config = RiskConfig {
                id: uuid::Uuid::new_v4(),
                user_address: user_address.to_string(),
                max_position_size_usd: prefs.max_position_size_usd.unwrap_or_else(|| BigDecimal::from(100000)),
                liquidation_threshold: BigDecimal::from_str("0.85").unwrap(),
                price_impact_threshold: prefs.max_slippage_percent.unwrap_or_else(|| BigDecimal::from(5)).clone() / BigDecimal::from(100),
                impermanent_loss_threshold: BigDecimal::from_str("0.10").unwrap(),
                volatility_threshold: BigDecimal::from_str("0.20").unwrap(),
                correlation_threshold: BigDecimal::from_str("0.80").unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            Ok(risk_config)
        } else {
            // Return default config if none exists
            let default_config = RiskConfig {
                id: uuid::Uuid::new_v4(),
                user_address: user_address.to_string(),
                max_position_size_usd: BigDecimal::from(100000),
                liquidation_threshold: BigDecimal::from_str("0.85").unwrap(),
                price_impact_threshold: BigDecimal::from_str("0.05").unwrap(),
                impermanent_loss_threshold: BigDecimal::from_str("0.10").unwrap(),
                volatility_threshold: BigDecimal::from_str("0.20").unwrap(),
                correlation_threshold: BigDecimal::from_str("0.80").unwrap(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            Ok(default_config)
        }
    }

    async fn store_risk_metrics(
        &self,
        position_id: uuid::Uuid,
        metrics: &crate::services::risk_calculator::RiskMetrics,
    ) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO risk_metrics (
                id, position_id, impermanent_loss, price_impact, volatility_score,
                correlation_score, liquidity_score, overall_risk_score,
                value_at_risk_1d, value_at_risk_7d, calculated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
            "#,
            uuid::Uuid::new_v4(),
            position_id,
            metrics.impermanent_loss,
            metrics.price_impact,
            metrics.volatility_score,
            metrics.correlation_score,
            metrics.liquidity_score,
            metrics.overall_risk_score,
            metrics.value_at_risk_1d,
            metrics.value_at_risk_7d
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn store_alert(&self, alert: &Alert) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO alerts (
                id, position_id, alert_type, severity, title, message,
                risk_score, current_value, threshold_value, is_resolved,
                resolved_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
            alert.id,
            alert.position_id,
            alert.alert_type,
            alert.severity,
            alert.title,
            alert.message,
            alert.risk_score,
            alert.current_value,
            alert.threshold_value,
            alert.is_resolved,
            alert.resolved_at,
            alert.created_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    fn determine_alert_severity(
        &self,
        metrics: &crate::services::risk_calculator::RiskMetrics,
    ) -> AlertSeverity {
        if metrics.overall_risk_score >= BigDecimal::from(8) {
            AlertSeverity::Critical
        } else if metrics.overall_risk_score >= BigDecimal::from(6) {
            AlertSeverity::High
        } else if metrics.overall_risk_score >= BigDecimal::from(4) {
            AlertSeverity::Medium
        } else {
            AlertSeverity::Low
        }
    }

    /// Calculate current position value based on pool state
    async fn calculate_position_value(
        &self,
        position: &Position,
        pool_state: &PoolState,
    ) -> Result<BigDecimal, AppError> {
        // Simple calculation based on current token prices
        // In a real implementation, this would be more sophisticated
        let default_price = BigDecimal::from(0);
        let token0_price = pool_state.token0_price_usd.as_ref().unwrap_or(&default_price);
        let token1_price = pool_state.token1_price_usd.as_ref().unwrap_or(&default_price);
        
        let token0_value = &position.token0_amount * token0_price;
        let token1_value = &position.token1_amount * token1_price;
        Ok(token0_value + token1_value)
    }

    /// Calculate initial position value based on entry prices
    async fn calculate_initial_position_value(
        &self,
        position: &Position,
    ) -> Result<BigDecimal, AppError> {
        // Use entry prices if available, otherwise return zero for P&L calculation
        let default_price = BigDecimal::from(0);
        let token0_entry_price = position.entry_token0_price_usd.as_ref().unwrap_or(&default_price);
        let token1_entry_price = position.entry_token1_price_usd.as_ref().unwrap_or(&default_price);
        
        let token0_value = &position.token0_amount * token0_entry_price;
        let token1_value = &position.token1_amount * token1_entry_price;
        Ok(token0_value + token1_value)
    }

    /// Check if risk metrics exceed configured thresholds
    fn check_risk_thresholds(
        &self,
        metrics: &crate::services::risk_calculator::RiskMetrics,
        config: &RiskConfig,
    ) -> Option<String> {
        let mut violations = Vec::new();

        if metrics.impermanent_loss > config.impermanent_loss_threshold {
            violations.push(format!(
                "Impermanent loss ({:.2}%) exceeds threshold ({:.2}%)",
                metrics.impermanent_loss.clone() * BigDecimal::from(100),
                config.impermanent_loss_threshold.clone() * BigDecimal::from(100)
            ));
        }

        if metrics.volatility_score > config.volatility_threshold {
            violations.push(format!(
                "Volatility risk ({:.2}) exceeds threshold ({:.2})",
                metrics.volatility_score,
                config.volatility_threshold
            ));
        }

        if metrics.overall_risk_score >= BigDecimal::from(8) {
            violations.push(format!(
                "Overall risk score ({:.2}) is critical",
                metrics.overall_risk_score
            ));
        }

        if violations.is_empty() {
            None
        } else {
            Some(violations.join("; "))
        }
    }
}
