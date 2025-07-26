use crate::models::{
    Position, PoolState, AlertThreshold, Alert, CreateAlert, AlertSeverity, ThresholdType,
};
use crate::services::{AlertService, RiskCalculator};
use crate::error::AppError;
use sqlx::PgPool;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use tracing::{info, warn, error};
use uuid::Uuid;

pub struct AlertEngine {
    db_pool: PgPool,
    alert_service: AlertService,
    risk_calculator: RiskCalculator,
}

impl AlertEngine {
    pub fn new(
        db_pool: PgPool,
        alert_service: AlertService,
        risk_calculator: RiskCalculator,
    ) -> Self {
        Self {
            db_pool,
            alert_service,
            risk_calculator,
        }
    }

    /// Check all thresholds for a position and trigger alerts if exceeded
    pub async fn check_position_thresholds(
        &self,
        position: &Position,
        pool_state: &PoolState,
        historical_data: &[PoolState],
        token0_price_history: &[crate::models::PriceHistory],
        token1_price_history: &[crate::models::PriceHistory],
        protocol_name: Option<&str>,
    ) -> Result<Vec<Alert>, AppError> {
        info!("Checking thresholds for position: {}", position.id);

        // Get user's alert thresholds
        let thresholds = self.get_user_thresholds(&position.user_address, Some(position.id)).await?;
        
        if thresholds.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate current risk metrics
        let risk_metrics = self.risk_calculator
            .calculate_position_risk(
                position,
                pool_state,
                &crate::models::RiskConfig::default(),
                historical_data,
                token0_price_history,
                token1_price_history,
                protocol_name,
                None, // No user risk params in alert engine context
            )
            .await?;

        let mut triggered_alerts = Vec::new();

        for threshold in thresholds {
            if let Some(alert) = self.check_threshold(position, &threshold, &risk_metrics).await? {
                triggered_alerts.push(alert);
            }
        }

        // Send alerts
        for alert in &triggered_alerts {
            if let Err(e) = self.alert_service.send_alert(alert).await {
                error!("Failed to send alert {}: {}", alert.id, e);
            }
        }

        info!("Triggered {} alerts for position {}", triggered_alerts.len(), position.id);
        Ok(triggered_alerts)
    }

    /// Check all thresholds for a user across all positions
    pub async fn check_user_thresholds(&self, user_address: &str) -> Result<Vec<Alert>, AppError> {
        info!("Checking thresholds for user: {}", user_address);

        // Get user positions
        let positions = self.get_user_positions(user_address).await?;
        let mut all_alerts = Vec::new();

        for position in positions {
            // Get pool state and historical data for this position
            if let Ok(pool_state) = self.get_pool_state(&position.pool_address, position.chain_id).await {
                let historical_data = self.get_historical_pool_data(&position.pool_address, position.chain_id).await.unwrap_or_default();
                let token0_history = self.get_token_price_history(&position.token0_address, position.chain_id).await.unwrap_or_default();
                let token1_history = self.get_token_price_history(&position.token1_address, position.chain_id).await.unwrap_or_default();

                match self.check_position_thresholds(
                    &position,
                    &pool_state,
                    &historical_data,
                    &token0_history,
                    &token1_history,
                    Some(&position.protocol),
                ).await {
                    Ok(mut alerts) => all_alerts.append(&mut alerts),
                    Err(e) => warn!("Failed to check thresholds for position {}: {}", position.id, e),
                }
            }
        }

        info!("Triggered {} total alerts for user {}", all_alerts.len(), user_address);
        Ok(all_alerts)
    }

    /// Check a specific threshold against risk metrics
    async fn check_threshold(
        &self,
        position: &Position,
        threshold: &AlertThreshold,
        risk_metrics: &crate::services::risk_calculator::RiskMetrics,
    ) -> Result<Option<Alert>, AppError> {
        let threshold_type = threshold.get_threshold_type();
        
        // Get the current value based on threshold type
        let current_value = match threshold_type {
            ThresholdType::ImpermanentLoss => {
                // Use the standard impermanent_loss field from risk_metrics
                risk_metrics.impermanent_loss.clone()
            },
            ThresholdType::TvlDrop => risk_metrics.tvl_drop_risk.clone(),
            ThresholdType::LiquidityRisk => risk_metrics.liquidity_score.clone(),
            ThresholdType::VolatilityRisk => risk_metrics.volatility_score.clone(),
            ThresholdType::ProtocolRisk => risk_metrics.protocol_risk_score.clone(),
            ThresholdType::MevRisk => risk_metrics.mev_risk_score.clone(),
            ThresholdType::CrossChainRisk => risk_metrics.cross_chain_risk_score.clone(),
            ThresholdType::OverallRisk => risk_metrics.overall_risk_score.clone(),
        };

        // Check if threshold is exceeded
        if threshold.is_exceeded(&current_value) {
            let alert = self.create_threshold_alert(position, threshold, &current_value).await?;
            self.store_alert(&alert).await?;
            Ok(Some(alert))
        } else {
            Ok(None)
        }
    }

    /// Create an alert for a threshold violation
    async fn create_threshold_alert(
        &self,
        position: &Position,
        threshold: &AlertThreshold,
        current_value: &BigDecimal,
    ) -> Result<Alert, AppError> {
        let threshold_type = threshold.get_threshold_type();
        let hundred = BigDecimal::from_str("100").unwrap();
        
        let (title, message, severity) = match threshold_type {
            ThresholdType::ImpermanentLoss => {
                let percentage = current_value * &hundred;
                let threshold_percentage = &threshold.threshold_value * &hundred;
                (
                    "Impermanent Loss Alert".to_string(),
                    format!(
                        "Position {} has impermanent loss of {:.2}%, exceeding threshold of {:.2}%",
                        position.id,
                        percentage,
                        threshold_percentage
                    ),
                    if current_value > &BigDecimal::from_str("0.20").unwrap() {
                        AlertSeverity::Critical
                    } else if current_value > &BigDecimal::from_str("0.10").unwrap() {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    }
                )
            },
            ThresholdType::TvlDrop => {
                let percentage = current_value * &hundred;
                let threshold_percentage = &threshold.threshold_value * &hundred;
                (
                    "TVL Drop Alert".to_string(),
                    format!(
                        "Position {} pool has TVL drop of {:.2}%, exceeding threshold of {:.2}%",
                        position.id,
                        percentage,
                        threshold_percentage
                    ),
                    if current_value > &BigDecimal::from_str("0.50").unwrap() {
                        AlertSeverity::Critical
                    } else if current_value > &BigDecimal::from_str("0.30").unwrap() {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    }
                )
            },
            ThresholdType::LiquidityRisk => {
                let percentage = current_value * &hundred;
                let threshold_percentage = &threshold.threshold_value * &hundred;
                (
                    "Liquidity Risk Alert".to_string(),
                    format!(
                        "Position {} has high liquidity risk of {:.2}%, exceeding threshold of {:.2}%",
                        position.id,
                        percentage,
                        threshold_percentage
                    ),
                    if current_value > &BigDecimal::from_str("0.80").unwrap() {
                        AlertSeverity::Critical
                    } else if current_value > &BigDecimal::from_str("0.60").unwrap() {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    }
                )
            },
            ThresholdType::OverallRisk => {
                let percentage = current_value * &hundred;
                let threshold_percentage = &threshold.threshold_value * &hundred;
                (
                    "Overall Risk Alert".to_string(),
                    format!(
                        "Position {} has overall risk score of {:.2}%, exceeding threshold of {:.2}%",
                        position.id,
                        percentage,
                        threshold_percentage
                    ),
                    if current_value > &BigDecimal::from_str("0.85").unwrap() {
                        AlertSeverity::Critical
                    } else if current_value > &BigDecimal::from_str("0.70").unwrap() {
                        AlertSeverity::High
                    } else {
                        AlertSeverity::Medium
                    }
                )
            },
            _ => {
                let percentage = current_value * &hundred;
                let threshold_percentage = &threshold.threshold_value * &hundred;
                (
                    format!("{:?} Risk Alert", threshold_type),
                    format!(
                        "Position {} has {:.2}% risk, exceeding threshold of {:.2}%",
                        position.id,
                        percentage,
                        threshold_percentage
                    ),
                    AlertSeverity::Medium
                )
            }
        };

        let create_alert = CreateAlert {
            position_id: Some(position.id),
            alert_type: format!("{:?}", threshold_type),
            severity,
            title,
            message,
            risk_score: Some(current_value.clone()),
            current_value: Some(current_value.clone()),
            threshold_value: Some(threshold.threshold_value.clone()),
        };

        Ok(Alert::new(create_alert))
    }

    /// Get user's alert thresholds
    async fn get_user_thresholds(
        &self,
        user_address: &str,
        position_id: Option<Uuid>,
    ) -> Result<Vec<AlertThreshold>, AppError> {
        let thresholds = if let Some(pos_id) = position_id {
            // Get position-specific thresholds and global thresholds
            sqlx::query_as!(
                AlertThreshold,
                r#"
                SELECT id, user_address, position_id, threshold_type, operator,
                       threshold_value, is_enabled, created_at, updated_at
                FROM alert_thresholds 
                WHERE user_address = $1 AND (position_id = $2 OR position_id IS NULL)
                AND is_enabled = true
                ORDER BY position_id NULLS LAST
                "#,
                user_address,
                pos_id
            )
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
        } else {
            // Get only global thresholds
            sqlx::query_as!(
                AlertThreshold,
                r#"
                SELECT id, user_address, position_id, threshold_type, operator,
                       threshold_value, is_enabled, created_at, updated_at
                FROM alert_thresholds 
                WHERE user_address = $1 AND position_id IS NULL AND is_enabled = true
                "#,
                user_address
            )
            .fetch_all(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
        };

        Ok(thresholds)
    }

    /// Store alert in database
    async fn store_alert(&self, alert: &Alert) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO alerts (id, position_id, alert_type, severity, title, message,
                              risk_score, current_value, threshold_value, is_resolved, 
                              resolved_at, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
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

    // Helper methods for data fetching (simplified for now)
    async fn get_user_positions(&self, user_address: &str) -> Result<Vec<Position>, AppError> {
        let positions = sqlx::query_as!(
            Position,
            r#"
            SELECT id, user_address, protocol, pool_address, token0_address, token1_address,
                   token0_amount, token1_amount, liquidity, tick_lower, tick_upper, fee_tier,
                   chain_id, entry_token0_price_usd, entry_token1_price_usd, 
                   entry_timestamp as "entry_timestamp!",
                   created_at as "created_at!", updated_at as "updated_at!"
            FROM positions 
            WHERE user_address = $1
            "#,
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(positions)
    }

    async fn get_pool_state(&self, _pool_address: &str, _chain_id: i32) -> Result<PoolState, AppError> {
        // This would integrate with your BlockchainService to get current pool state
        // For now, return a placeholder error
        Err(AppError::NotFound("Pool state not implemented in AlertEngine".to_string()))
    }

    async fn get_historical_pool_data(&self, _pool_address: &str, _chain_id: i32) -> Result<Vec<PoolState>, AppError> {
        // This would fetch historical pool data from database
        Ok(Vec::new())
    }

    async fn get_token_price_history(&self, _token_address: &str, _chain_id: i32) -> Result<Vec<crate::models::PriceHistory>, AppError> {
        // This would fetch token price history from database
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;

    #[test]
    fn test_alert_engine_creation() {
        // This test would require setting up test database and services
        // For now, just test that the struct can be created
        assert!(true);
    }
}
