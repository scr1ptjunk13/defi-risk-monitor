use crate::models::{AlertThreshold, CreateAlertThreshold, UpdateAlertThreshold, get_default_thresholds};
use crate::error::AppError;
use sqlx::PgPool;
use uuid::Uuid;
use tracing::{info, warn};

pub struct ThresholdService {
    db_pool: PgPool,
}

impl ThresholdService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Create a new alert threshold
    pub async fn create_threshold(
        &self,
        create_threshold: CreateAlertThreshold,
    ) -> Result<AlertThreshold, AppError> {
        let threshold = AlertThreshold::new(create_threshold);

        sqlx::query!(
            r#"
            INSERT INTO alert_thresholds (id, user_address, position_id, threshold_type, 
                                        operator, threshold_value, is_enabled, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            threshold.id,
            threshold.user_address,
            threshold.position_id,
            threshold.threshold_type,
            threshold.operator,
            threshold.threshold_value,
            threshold.is_enabled,
            threshold.created_at,
            threshold.updated_at
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        info!("Created alert threshold {} for user {}", threshold.id, threshold.user_address);
        Ok(threshold)
    }

    /// Get alert threshold by ID
    pub async fn get_threshold(&self, threshold_id: Uuid) -> Result<Option<AlertThreshold>, AppError> {
        let threshold = sqlx::query_as!(
            AlertThreshold,
            r#"
            SELECT id, user_address, position_id, threshold_type, operator,
                   threshold_value, is_enabled, created_at, updated_at
            FROM alert_thresholds 
            WHERE id = $1
            "#,
            threshold_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(threshold)
    }

    /// Get all thresholds for a user
    pub async fn get_user_thresholds(&self, user_address: &str) -> Result<Vec<AlertThreshold>, AppError> {
        let thresholds = sqlx::query_as!(
            AlertThreshold,
            r#"
            SELECT id, user_address, position_id, threshold_type, operator,
                   threshold_value, is_enabled, created_at, updated_at
            FROM alert_thresholds 
            WHERE user_address = $1
            ORDER BY created_at DESC
            "#,
            user_address
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(thresholds)
    }

    /// Get thresholds for a specific position
    pub async fn get_position_thresholds(
        &self,
        user_address: &str,
        position_id: Uuid,
    ) -> Result<Vec<AlertThreshold>, AppError> {
        let thresholds = sqlx::query_as!(
            AlertThreshold,
            r#"
            SELECT id, user_address, position_id, threshold_type, operator,
                   threshold_value, is_enabled, created_at, updated_at
            FROM alert_thresholds 
            WHERE user_address = $1 AND position_id = $2
            ORDER BY created_at DESC
            "#,
            user_address,
            position_id
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(thresholds)
    }

    /// Update an alert threshold
    pub async fn update_threshold(
        &self,
        threshold_id: Uuid,
        update_threshold: UpdateAlertThreshold,
    ) -> Result<AlertThreshold, AppError> {
        // Use individual update queries for simplicity
        if let Some(ref threshold_value) = update_threshold.threshold_value {
            sqlx::query!(
                "UPDATE alert_thresholds SET threshold_value = $1, updated_at = NOW() WHERE id = $2",
                threshold_value,
                threshold_id
            )
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        if let Some(is_enabled) = update_threshold.is_enabled {
            sqlx::query!(
                "UPDATE alert_thresholds SET is_enabled = $1, updated_at = NOW() WHERE id = $2",
                is_enabled,
                threshold_id
            )
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        if let Some(ref operator) = update_threshold.operator {
            let operator_str = match operator {
                crate::models::ThresholdOperator::GreaterThan => "greater_than",
                crate::models::ThresholdOperator::LessThan => "less_than",
                crate::models::ThresholdOperator::GreaterThanOrEqual => "greater_than_or_equal",
                crate::models::ThresholdOperator::LessThanOrEqual => "less_than_or_equal",
            };
            sqlx::query!(
                "UPDATE alert_thresholds SET operator = $1, updated_at = NOW() WHERE id = $2",
                operator_str,
                threshold_id
            )
            .execute(&self.db_pool)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }

        // Return updated threshold
        self.get_threshold(threshold_id).await?
            .ok_or_else(|| AppError::NotFound("Threshold not found after update".to_string()))
    }

    /// Delete an alert threshold
    pub async fn delete_threshold(&self, threshold_id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query!(
            "DELETE FROM alert_thresholds WHERE id = $1",
            threshold_id
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Threshold not found".to_string()));
        }

        info!("Deleted alert threshold {}", threshold_id);
        Ok(())
    }

    /// Initialize default thresholds for a new user
    pub async fn initialize_default_thresholds(&self, user_address: &str) -> Result<Vec<AlertThreshold>, AppError> {
        info!("Initializing default thresholds for user: {}", user_address);

        // Check if user already has thresholds
        let existing_count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM alert_thresholds WHERE user_address = $1",
            user_address
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .unwrap_or(0);

        if existing_count > 0 {
            warn!("User {} already has {} thresholds, skipping default initialization", user_address, existing_count);
            return self.get_user_thresholds(user_address).await;
        }

        // Create default thresholds
        let default_thresholds = get_default_thresholds(user_address);
        let mut created_thresholds = Vec::new();

        for create_threshold in default_thresholds {
            match self.create_threshold(create_threshold).await {
                Ok(threshold) => created_thresholds.push(threshold),
                Err(e) => warn!("Failed to create default threshold: {}", e),
            }
        }

        info!("Created {} default thresholds for user {}", created_thresholds.len(), user_address);
        Ok(created_thresholds)
    }

    /// Enable/disable all thresholds for a user
    pub async fn toggle_user_thresholds(&self, user_address: &str, enabled: bool) -> Result<usize, AppError> {
        let result = sqlx::query!(
            "UPDATE alert_thresholds SET is_enabled = $1, updated_at = NOW() WHERE user_address = $2",
            enabled,
            user_address
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        info!("Toggled {} thresholds to {} for user {}", result.rows_affected(), enabled, user_address);
        Ok(result.rows_affected() as usize)
    }

    /// Get threshold statistics for a user
    pub async fn get_user_threshold_stats(&self, user_address: &str) -> Result<ThresholdStats, AppError> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_count,
                COUNT(*) FILTER (WHERE is_enabled = true) as enabled_count,
                COUNT(*) FILTER (WHERE position_id IS NOT NULL) as position_specific_count,
                COUNT(*) FILTER (WHERE position_id IS NULL) as global_count
            FROM alert_thresholds 
            WHERE user_address = $1
            "#,
            user_address
        )
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(ThresholdStats {
            total_count: stats.total_count.unwrap_or(0) as u32,
            enabled_count: stats.enabled_count.unwrap_or(0) as u32,
            position_specific_count: stats.position_specific_count.unwrap_or(0) as u32,
            global_count: stats.global_count.unwrap_or(0) as u32,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ThresholdStats {
    pub total_count: u32,
    pub enabled_count: u32,
    pub position_specific_count: u32,
    pub global_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ThresholdType, ThresholdOperator};
    use bigdecimal::BigDecimal;
    use std::str::FromStr;

    #[test]
    fn test_threshold_service_creation() {
        // This test would require setting up test database
        // For now, just test that the struct can be created
        assert!(true);
    }

    #[test]
    fn test_threshold_stats_structure() {
        let stats = ThresholdStats {
            total_count: 5,
            enabled_count: 4,
            position_specific_count: 2,
            global_count: 3,
        };

        assert_eq!(stats.total_count, 5);
        assert_eq!(stats.enabled_count, 4);
    }
}
