use crate::models::{
    RiskAssessment, RiskAssessmentHistory, BulkRiskAssessment, RiskAssessmentFilter,
    RiskEntityType, RiskType, RiskSeverity
};
use crate::models::risk_assessment::*;
use crate::error::AppError;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::str::FromStr;
use tracing::{info, warn, error};
use uuid::Uuid;

pub struct RiskAssessmentService {
    db_pool: PgPool,
}

impl RiskAssessmentService {
    pub fn new(db_pool: PgPool) -> Self {
        Self { db_pool }
    }

    /// Get risk history for an entity
    pub async fn get_risk_history(
        &self,
        entity_type: RiskEntityType,
        entity_id: &str,
        risk_type: Option<RiskType>,
        days_back: Option<i32>,
        limit: Option<i64>,
    ) -> Result<Vec<RiskAssessment>, AppError> {
        info!("Getting risk history for entity: {} {}", entity_type.clone() as i32, entity_id);

        let days_back = days_back.unwrap_or(30);
        let limit = limit.unwrap_or(100);
        let cutoff_date = Utc::now() - chrono::Duration::days(days_back as i64);

        // Use dynamic query building to avoid type compatibility issues
        let mut query_str = r#"
            SELECT 
                id,
                entity_type::text,
                entity_id,
                user_id,
                risk_type::text,
                risk_score,
                severity::text,
                confidence,
                description,
                metadata,
                expires_at,
                is_active,
                created_at,
                updated_at
            FROM risk_assessments 
            WHERE entity_type = $1 AND entity_id = $2
        "#.to_string();

        let mut param_count = 2;
        
        if let Some(_) = risk_type {
            param_count += 1;
            query_str.push_str(&format!(" AND risk_type = ${}", param_count));
        }
        
        param_count += 1;
        query_str.push_str(&format!(" AND created_at >= ${}", param_count));
        query_str.push_str(" ORDER BY created_at DESC");
        param_count += 1;
        query_str.push_str(&format!(" LIMIT ${}", param_count));

        // Use a simpler query approach to avoid enum mapping issues
        let rows = if let Some(risk_type) = risk_type {
            sqlx::query(&query_str)
                .bind(entity_type)
                .bind(entity_id)
                .bind(risk_type)
                .bind(cutoff_date)
                .bind(limit)
                .fetch_all(&self.db_pool)
                .await?
        } else {
            sqlx::query(&query_str)
                .bind(entity_type)
                .bind(entity_id)
                .bind(cutoff_date)
                .bind(limit)
                .fetch_all(&self.db_pool)
                .await?
        };

        let mut assessments = Vec::new();
        for row in rows {
            // Parse snake_case enum values from database
            let entity_type_str: String = row.get("entity_type");
            let entity_type = match entity_type_str.as_str() {
                "position" => RiskEntityType::Position,
                "protocol" => RiskEntityType::Protocol,
                "user" => RiskEntityType::User,
                "portfolio" => RiskEntityType::Portfolio,
                "pool" => RiskEntityType::Pool,
                "token" => RiskEntityType::Token,
                _ => RiskEntityType::Position,
            };
            
            let risk_type_str: String = row.get("risk_type");
            let risk_type = match risk_type_str.as_str() {
                "impermanent_loss" => RiskType::ImpermanentLoss,
                "liquidity" => RiskType::Liquidity,
                "protocol" => RiskType::Protocol,
                "mev" => RiskType::Mev,
                "cross_chain" => RiskType::CrossChain,
                "market" => RiskType::Market,
                "slippage" => RiskType::Slippage,
                "correlation" => RiskType::Correlation,
                "volatility" => RiskType::Volatility,
                "overall" => RiskType::Overall,
                _ => RiskType::Liquidity,
            };
            
            let severity_str: String = row.get("severity");
            let severity = match severity_str.as_str() {
                "critical" => RiskSeverity::Critical,
                "high" => RiskSeverity::High,
                "medium" => RiskSeverity::Medium,
                "low" => RiskSeverity::Low,
                "minimal" => RiskSeverity::Minimal,
                _ => RiskSeverity::Low,
            };

            let assessment = RiskAssessment {
                id: row.get("id"),
                entity_type,
                entity_id: row.get("entity_id"),
                user_id: row.get("user_id"),
                risk_type,
                risk_score: row.get("risk_score"),
                severity,
                confidence: row.get("confidence"),
                description: row.get("description"),
                metadata: row.get("metadata"),
                expires_at: row.get("expires_at"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };
            assessments.push(assessment);
        }
        
        info!("Retrieved {} risk history records", assessments.len());
        Ok(assessments)
    }

    /// Update or create a risk assessment
    pub async fn update_risk_assessment(
        &self,
        entity_type: RiskEntityType,
        entity_id: &str,
        user_id: Option<Uuid>,
        risk_type: RiskType,
        risk_score: BigDecimal,
        severity: RiskSeverity,
        confidence: Option<BigDecimal>,
        description: Option<String>,
        metadata: Option<serde_json::Value>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<RiskAssessment, AppError> {
        info!("Updating risk assessment for {} {} with score {}", entity_type.clone() as i32, entity_id, risk_score);

        let confidence = confidence.unwrap_or_else(|| BigDecimal::from(1));

        // Check if assessment already exists
        let existing = sqlx::query!(
            "SELECT id, risk_score, severity as \"severity: RiskSeverity\" FROM risk_assessments 
             WHERE entity_type = $1 AND entity_id = $2 AND risk_type = $3 AND is_active = true",
            entity_type.clone() as RiskEntityType,
            entity_id,
            risk_type.clone() as RiskType
        )
        .fetch_optional(&self.db_pool)
        .await?;

        let assessment_id = if let Some(existing) = existing {
            // Create history record before updating
            let previous_severity = existing.severity;

            sqlx::query!(
                "INSERT INTO risk_assessment_history 
                 (risk_assessment_id, previous_risk_score, new_risk_score, previous_severity, new_severity, change_reason)
                 VALUES ($1, $2, $3, $4, $5, $6)",
                existing.id,
                existing.risk_score,
                risk_score,
                previous_severity as RiskSeverity,
                severity.clone() as RiskSeverity,
                "Risk assessment update"
            )
            .execute(&self.db_pool)
            .await?;

            // Update existing assessment
            sqlx::query!(
                "UPDATE risk_assessments 
                 SET risk_score = $1, severity = $2, confidence = $3, description = $4, 
                     metadata = $5, expires_at = $6, updated_at = NOW()
                 WHERE id = $7",
                risk_score,
                severity.clone() as RiskSeverity,
                confidence,
                description,
                metadata,
                expires_at,
                existing.id
            )
            .execute(&self.db_pool)
            .await?;

            existing.id
        } else {
            // Create new assessment
            let new_id = Uuid::new_v4();
            sqlx::query!(
                "INSERT INTO risk_assessments 
                 (id, entity_type, entity_id, user_id, risk_type, risk_score, severity, 
                  confidence, description, metadata, expires_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                new_id,
                entity_type as RiskEntityType,
                entity_id,
                user_id,
                risk_type as RiskType,
                risk_score,
                severity as RiskSeverity,
                confidence,
                description,
                metadata,
                expires_at
            )
            .execute(&self.db_pool)
            .await?;

            new_id
        };

        // Fetch and return the updated assessment
        let assessment = sqlx::query_as!(
            RiskAssessment,
            r#"
            SELECT 
                id,
                entity_type as "entity_type: RiskEntityType",
                entity_id,
                user_id,
                risk_type as "risk_type: RiskType",
                risk_score,
                severity as "severity: RiskSeverity",
                confidence,
                description,
                metadata,
                expires_at,
                is_active,
                created_at,
                updated_at
            FROM risk_assessments WHERE id = $1
            "#,
            assessment_id
        )
        .fetch_one(&self.db_pool)
        .await?;

        info!("Successfully updated risk assessment with ID: {}", assessment_id);
        Ok(assessment)
    }

    /// Get risks by severity level
    pub async fn get_risks_by_severity(
        &self,
        severity: RiskSeverity,
        entity_type: Option<RiskEntityType>,
        user_id: Option<Uuid>,
        active_only: bool,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<RiskAssessment>, AppError> {
        info!("Getting risks by severity: {:?}", severity);

        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let mut query_str = r#"
            SELECT 
                id,
                entity_type as "entity_type: RiskEntityType",
                entity_id,
                user_id,
                risk_type as "risk_type: RiskType",
                risk_score,
                severity as "severity: RiskSeverity",
                confidence,
                description,
                metadata,
                expires_at,
                is_active,
                created_at,
                updated_at
            FROM risk_assessments 
            WHERE severity = $1
        "#.to_string();

        let mut param_count = 1;
        let mut conditions = Vec::new();

        if let Some(_) = entity_type {
            param_count += 1;
            conditions.push(format!(" AND entity_type = ${}", param_count));
        }

        if let Some(_) = user_id {
            param_count += 1;
            conditions.push(format!(" AND user_id = ${}", param_count));
        }

        if active_only {
            conditions.push(" AND is_active = true".to_string());
        }

        for condition in conditions {
            query_str.push_str(&condition);
        }

        query_str.push_str(" ORDER BY risk_score DESC, created_at DESC");
        param_count += 1;
        query_str.push_str(&format!(" LIMIT ${}", param_count));
        param_count += 1;
        query_str.push_str(&format!(" OFFSET ${}", param_count));

        let mut query = sqlx::query_as::<_, RiskAssessment>(&query_str)
            .bind(&severity);

        if let Some(entity_type) = entity_type {
            query = query.bind(entity_type);
        }

        if let Some(user_id) = user_id {
            query = query.bind(user_id);
        }

        query = query.bind(limit).bind(offset);

        let assessments = query.fetch_all(&self.db_pool).await?;
        info!("Retrieved {} risks with severity {:?}", assessments.len(), &severity);
        Ok(assessments)
    }

    /// Get expired risks
    pub async fn get_expired_risks(
        &self,
        entity_type: Option<RiskEntityType>,
        user_id: Option<Uuid>,
        limit: Option<i64>,
    ) -> Result<Vec<RiskAssessment>, AppError> {
        info!("Getting expired risks");

        let limit = limit.unwrap_or(100);
        let now = Utc::now();

        let assessments = if let Some(entity_type) = entity_type {
            if let Some(user_id) = user_id {
                sqlx::query_as!(
                    RiskAssessment,
                    r#"
                    SELECT 
                        id,
                        entity_type as "entity_type: RiskEntityType",
                        entity_id,
                        user_id,
                        risk_type as "risk_type: RiskType",
                        risk_score,
                        severity as "severity: RiskSeverity",
                        confidence,
                        description,
                        metadata,
                        expires_at,
                        is_active,
                        created_at,
                        updated_at
                    FROM risk_assessments 
                    WHERE expires_at IS NOT NULL 
                        AND expires_at < $1 
                        AND entity_type = $2
                        AND user_id = $3
                        AND is_active = true
                    ORDER BY expires_at ASC 
                    LIMIT $4
                    "#,
                    now,
                    entity_type as RiskEntityType,
                    user_id,
                    limit
                )
                .fetch_all(&self.db_pool)
                .await?
            } else {
                sqlx::query_as!(
                    RiskAssessment,
                    r#"
                    SELECT 
                        id,
                        entity_type as "entity_type: RiskEntityType",
                        entity_id,
                        user_id,
                        risk_type as "risk_type: RiskType",
                        risk_score,
                        severity as "severity: RiskSeverity",
                        confidence,
                        description,
                        metadata,
                        expires_at,
                        is_active,
                        created_at,
                        updated_at
                    FROM risk_assessments 
                    WHERE expires_at IS NOT NULL 
                        AND expires_at < $1 
                        AND entity_type = $2
                        AND is_active = true
                    ORDER BY expires_at ASC 
                    LIMIT $3
                    "#,
                    now,
                    entity_type as RiskEntityType,
                    limit
                )
                .fetch_all(&self.db_pool)
                .await?
            }
        } else if let Some(user_id) = user_id {
            sqlx::query_as!(
                RiskAssessment,
                r#"
                SELECT 
                    id,
                    entity_type as "entity_type: RiskEntityType",
                    entity_id,
                    user_id,
                    risk_type as "risk_type: RiskType",
                    risk_score,
                    severity as "severity: RiskSeverity",
                    confidence,
                    description,
                    metadata,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
                FROM risk_assessments 
                WHERE expires_at IS NOT NULL 
                    AND expires_at < $1 
                    AND user_id = $2
                    AND is_active = true
                ORDER BY expires_at ASC 
                LIMIT $3
                "#,
                now,
                user_id,
                limit
            )
            .fetch_all(&self.db_pool)
            .await?
        } else {
            sqlx::query_as!(
                RiskAssessment,
                r#"
                SELECT 
                    id,
                    entity_type as "entity_type: RiskEntityType",
                    entity_id,
                    user_id,
                    risk_type as "risk_type: RiskType",
                    risk_score,
                    severity as "severity: RiskSeverity",
                    confidence,
                    description,
                    metadata,
                    expires_at,
                    is_active,
                    created_at,
                    updated_at
                FROM risk_assessments 
                WHERE expires_at IS NOT NULL 
                    AND expires_at < $1 
                    AND is_active = true
                ORDER BY expires_at ASC 
                LIMIT $2
                "#,
                now,
                limit
            )
            .fetch_all(&self.db_pool)
            .await?
        };

        info!("Retrieved {} expired risks", assessments.len());
        Ok(assessments)
    }

    /// Bulk insert risk assessments
    pub async fn bulk_insert_risks(
        &self,
        assessments: Vec<BulkRiskAssessment>,
    ) -> Result<Vec<Uuid>, AppError> {
        info!("Bulk inserting {} risk assessments", assessments.len());

        if assessments.is_empty() {
            return Ok(Vec::new());
        }

        let mut tx = self.db_pool.begin().await?;
        let mut inserted_ids = Vec::new();

        for assessment in assessments {
            let id = Uuid::new_v4();
            
            sqlx::query!(
                "INSERT INTO risk_assessments 
                 (id, entity_type, entity_id, user_id, risk_type, risk_score, severity, 
                  confidence, description, metadata, expires_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
                id,
                assessment.entity_type as RiskEntityType,
                assessment.entity_id,
                assessment.user_id,
                assessment.risk_type as RiskType,
                assessment.risk_score,
                assessment.severity as RiskSeverity,
                assessment.confidence,
                assessment.description,
                assessment.metadata,
                assessment.expires_at
            )
            .execute(&mut *tx)
            .await?;

            inserted_ids.push(id);
        }

        tx.commit().await?;
        info!("Successfully bulk inserted {} risk assessments", inserted_ids.len());
        Ok(inserted_ids)
    }

    /// Clean up old risk assessments
    pub async fn cleanup_old_risks(
        &self,
        days_old: i32,
        batch_size: Option<i64>,
        keep_critical: bool,
    ) -> Result<u64, AppError> {
        info!("Cleaning up risks older than {} days", days_old);

        let batch_size = batch_size.unwrap_or(1000);
        let cutoff_date = Utc::now() - chrono::Duration::days(days_old as i64);
        let mut total_deleted = 0u64;

        loop {
            let deleted_count = if keep_critical {
                sqlx::query!(
                    "DELETE FROM risk_assessments 
                     WHERE id IN (
                         SELECT id FROM risk_assessments 
                         WHERE created_at < $1 
                             AND severity != 'critical'
                             AND is_active = false
                         ORDER BY created_at ASC 
                         LIMIT $2
                     )",
                    cutoff_date,
                    batch_size
                )
                .execute(&self.db_pool)
                .await?
                .rows_affected()
            } else {
                sqlx::query!(
                    "DELETE FROM risk_assessments 
                     WHERE id IN (
                         SELECT id FROM risk_assessments 
                         WHERE created_at < $1 
                             AND is_active = false
                         ORDER BY created_at ASC 
                         LIMIT $2
                     )",
                    cutoff_date,
                    batch_size
                )
                .execute(&self.db_pool)
                .await?
                .rows_affected()
            };

            total_deleted += deleted_count;

            if deleted_count == 0 {
                break;
            }

            // Small delay between batches to avoid overwhelming the database
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        info!("Cleaned up {} old risk assessments", total_deleted);
        Ok(total_deleted)
    }

    /// Get risk assessment by ID
    pub async fn get_risk_assessment_by_id(&self, id: Uuid) -> Result<Option<RiskAssessment>, AppError> {
        let assessment = sqlx::query_as!(
            RiskAssessment,
            r#"
            SELECT 
                id,
                entity_type as "entity_type: RiskEntityType",
                entity_id,
                user_id,
                risk_type as "risk_type: RiskType",
                risk_score,
                severity as "severity: RiskSeverity",
                confidence,
                description,
                metadata,
                expires_at,
                is_active,
                created_at,
                updated_at
            FROM risk_assessments WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.db_pool)
        .await?;

        Ok(assessment)
    }

    /// Deactivate risk assessment
    pub async fn deactivate_risk_assessment(&self, id: Uuid) -> Result<bool, AppError> {
        let result = sqlx::query!(
            "UPDATE risk_assessments SET is_active = false, updated_at = NOW() WHERE id = $1",
            id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get risk assessment statistics
    pub async fn get_risk_statistics(
        &self,
        entity_type: Option<RiskEntityType>,
        user_id: Option<Uuid>,
    ) -> Result<serde_json::Value, AppError> {
        // Use dynamic query building to avoid type compatibility issues
        let mut query_str = r#"
            SELECT 
                COUNT(*) as total_assessments,
                COUNT(*) FILTER (WHERE severity = 'critical') as critical_count,
                COUNT(*) FILTER (WHERE severity = 'high') as high_count,
                COUNT(*) FILTER (WHERE severity = 'medium') as medium_count,
                COUNT(*) FILTER (WHERE severity = 'low') as low_count,
                COUNT(*) FILTER (WHERE severity = 'minimal') as minimal_count,
                AVG(risk_score) as avg_risk_score,
                MAX(risk_score) as max_risk_score,
                COUNT(*) FILTER (WHERE expires_at IS NOT NULL AND expires_at < NOW()) as expired_count,
                COUNT(*) FILTER (WHERE is_active = true) as active_count
            FROM risk_assessments
        "#.to_string();

        let mut conditions = Vec::new();
        let mut param_count = 0;

        if let Some(_) = entity_type {
            param_count += 1;
            conditions.push(format!(" entity_type = ${}", param_count));
        }

        if let Some(_) = user_id {
            param_count += 1;
            conditions.push(format!(" user_id = ${}", param_count));
        }

        if !conditions.is_empty() {
            query_str.push_str(" WHERE");
            query_str.push_str(&conditions.join(" AND"));
        }

        let mut query = sqlx::query(&query_str);

        if let Some(entity_type) = entity_type {
            query = query.bind(entity_type);
        }

        if let Some(user_id) = user_id {
            query = query.bind(user_id);
        }

        let row = query.fetch_one(&self.db_pool).await?;
        
        let total_assessments: i64 = row.get("total_assessments");
        let critical_count: i64 = row.get("critical_count");
        let high_count: i64 = row.get("high_count");
        let medium_count: i64 = row.get("medium_count");
        let low_count: i64 = row.get("low_count");
        let minimal_count: i64 = row.get("minimal_count");
        let avg_risk_score: Option<BigDecimal> = row.get("avg_risk_score");
        let max_risk_score: Option<BigDecimal> = row.get("max_risk_score");
        let expired_count: i64 = row.get("expired_count");
        let active_count: i64 = row.get("active_count");

        let statistics = serde_json::json!({
            "total_assessments": total_assessments,
            "severity_breakdown": {
                "critical": critical_count,
                "high": high_count,
                "medium": medium_count,
                "low": low_count,
                "minimal": minimal_count
            },
            "risk_metrics": {
                "average_risk_score": avg_risk_score,
                "maximum_risk_score": max_risk_score
            },
            "status_counts": {
                "active": active_count,
                "expired": expired_count
            }
        });

        Ok(statistics)
    }
}
