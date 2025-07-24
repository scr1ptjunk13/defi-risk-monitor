use crate::models::{Position, PoolState, RiskConfig, Alert, CreateAlert, AlertSeverity};
use crate::services::{BlockchainService, RiskCalculator, AlertService};
use crate::config::Settings;
use crate::error::AppError;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{info, error, warn};
use bigdecimal::BigDecimal;

pub struct MonitoringService {
    db_pool: PgPool,
    blockchain_service: Arc<BlockchainService>,
    risk_calculator: Arc<RiskCalculator>,
    alert_service: Arc<AlertService>,
    settings: Settings,
}

impl MonitoringService {
    pub fn new(db_pool: PgPool, settings: Settings) -> Result<Self, AppError> {
        let blockchain_service = Arc::new(BlockchainService::new(&settings)?);
        let risk_calculator = Arc::new(RiskCalculator::new());
        let alert_service = Arc::new(AlertService::new(&settings));

        Ok(Self {
            db_pool,
            blockchain_service,
            risk_calculator,
            alert_service,
            settings,
        })
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
        // Fetch current pool state
        let pool_state = self.blockchain_service
            .get_pool_state(&position.pool_address, position.chain_id)
            .await?;

        // Store pool state
        self.store_pool_state(&pool_state).await?;

        // Fetch historical data for risk calculation
        let historical_data = self.fetch_historical_pool_data(
            &position.pool_address,
            position.chain_id,
            30, // Last 30 data points
        ).await?;

        // Fetch risk configuration for the user
        let risk_config = self.fetch_risk_config(&position.user_address).await?;

        // Calculate risk metrics
        let risk_metrics = self.risk_calculator.calculate_position_risk(
            position,
            &pool_state,
            &risk_config,
            &historical_data,
        )?;

        // Store risk metrics
        self.store_risk_metrics(position.id, &risk_metrics).await?;

        // Check for risk threshold violations
        let violations = self.risk_calculator.check_risk_thresholds(&risk_metrics, &risk_config);

        // Generate alerts for violations
        for violation in violations {
            let alert = CreateAlert {
                position_id: Some(position.id),
                alert_type: "risk_threshold_violation".to_string(),
                severity: self.determine_alert_severity(&risk_metrics),
                title: "Risk Threshold Exceeded".to_string(),
                message: violation,
                risk_score: Some(risk_metrics.overall_risk_score.clone()),
                current_value: None,
                threshold_value: None,
            };

            let alert = Alert::new(alert);
            self.store_alert(&alert).await?;
            
            // Send alert notification
            if let Err(e) = self.alert_service.send_alert(&alert).await {
                error!("Failed to send alert: {}", e);
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
        let risk_config = sqlx::query_as::<_, RiskConfig>(
            "SELECT * FROM risk_configs WHERE user_address = $1"
        )
        .bind(user_address)
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Return default config if none exists
        risk_config.ok_or_else(|| {
            AppError::NotFound(format!("Risk config not found for user {}", user_address))
        })
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
}
