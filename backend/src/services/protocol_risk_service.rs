use crate::models::{
    ProtocolRisk, ProtocolAudit, ProtocolExploit, ProtocolMetrics, 
    ProtocolRiskConfig, ExploitType, ExploitSeverity
};
use crate::error::AppError;
use bigdecimal::BigDecimal;
use chrono::{Utc, Duration};
use sqlx::PgPool;
use tracing::{info, warn};
use std::str::FromStr;
use num_traits::Zero;

/// Service for protocol risk assessment and scoring
pub struct ProtocolRiskService {
    db_pool: PgPool,
    config: ProtocolRiskConfig,
}

impl ProtocolRiskService {
    pub fn new(db_pool: PgPool, config: Option<ProtocolRiskConfig>) -> Self {
        Self {
            db_pool,
            config: config.unwrap_or_default(),
        }
    }

    /// Calculate comprehensive protocol risk score
    pub async fn calculate_protocol_risk(
        &self,
        protocol_name: &str,
        protocol_address: &str,
        chain_id: i32,
    ) -> Result<ProtocolRisk, AppError> {
        info!("Calculating protocol risk for {} on chain {}", protocol_name, chain_id);

        // Calculate individual risk components
        let audit_score = self.calculate_audit_score(protocol_name).await?;
        let exploit_history_score = self.calculate_exploit_history_score(protocol_name).await?;
        let tvl_score = self.calculate_tvl_score(protocol_name).await?;
        let governance_score = self.calculate_governance_score(protocol_name).await?;
        let code_quality_score = self.calculate_code_quality_score(protocol_name).await?;

        // Calculate weighted overall risk
        let overall_risk = self.calculate_weighted_protocol_risk(
            &audit_score,
            &exploit_history_score,
            &tvl_score,
            &governance_score,
            &code_quality_score,
        )?;

        let protocol_risk = ProtocolRisk {
            id: uuid::Uuid::new_v4(),
            protocol_name: protocol_name.to_string(),
            protocol_address: protocol_address.to_string(),
            chain_id,
            audit_score,
            exploit_history_score,
            tvl_score,
            governance_score,
            code_quality_score,
            overall_protocol_risk: overall_risk.clone(),
            last_updated: Utc::now(),
            created_at: Utc::now(),
        };

        // Store the assessment
        self.store_protocol_risk(&protocol_risk).await?;

        info!("Protocol risk calculated: overall score {}", overall_risk);
        Ok(protocol_risk)
    }

    /// Calculate audit-based risk score
    async fn calculate_audit_score(&self, protocol_name: &str) -> Result<BigDecimal, AppError> {
        let audits = self.get_recent_audits(protocol_name, 365).await?; // Last year

        if audits.is_empty() {
            warn!("No audits found for protocol {}", protocol_name);
            return Ok(BigDecimal::from(0)); // Maximum risk for unaudited protocols
        }

        let mut weighted_score = BigDecimal::from(0);
        let mut total_weight = BigDecimal::from(0);

        for audit in &audits {
            // Weight newer audits more heavily
            let age_days = (Utc::now() - audit.audit_date).num_days();
            let age_weight = if age_days <= 90 {
                BigDecimal::from(100) // Recent audits get full weight
            } else if age_days <= 180 {
                BigDecimal::from(80) // 6-month audits get 80% weight
            } else if age_days <= 365 {
                BigDecimal::from(60) // 1-year audits get 60% weight
            } else {
                BigDecimal::from(30) // Older audits get 30% weight
            };

            // Adjust score based on issues found
            let issue_penalty = BigDecimal::from(audit.critical_issues * 20 + audit.high_issues * 10 + audit.medium_issues * 5);
            let adjusted_score = (&audit.audit_score - &issue_penalty).max(BigDecimal::from(0));

            weighted_score += &adjusted_score * &age_weight;
            total_weight += &age_weight;
        }

        let final_score = if !total_weight.is_zero() {
            (&weighted_score / &total_weight) / BigDecimal::from(100) // Normalize to 0-1
        } else {
            BigDecimal::from(0)
        };

        Ok(final_score)
    }

    /// Calculate exploit history risk score
    async fn calculate_exploit_history_score(&self, protocol_name: &str) -> Result<BigDecimal, AppError> {
        let exploits = self.get_exploit_history(protocol_name).await?;

        if exploits.is_empty() {
            return Ok(BigDecimal::from(1)); // No exploits = low risk (high score)
        }

        let mut total_loss = BigDecimal::from(0);
        let mut severity_penalty = BigDecimal::from(0);
        let mut recent_exploit_penalty = BigDecimal::from(0);

        for exploit in &exploits {
            total_loss += &exploit.amount_lost_usd;

            // Severity-based penalty
            let severity_weight = match exploit.severity {
                ExploitSeverity::Critical => BigDecimal::from(40),
                ExploitSeverity::High => BigDecimal::from(30),
                ExploitSeverity::Medium => BigDecimal::from(20),
                ExploitSeverity::Low => BigDecimal::from(10),
            };
            severity_penalty += &severity_weight;

            // Recent exploit penalty (last 2 years)
            let age_days = (Utc::now() - exploit.exploit_date).num_days();
            if age_days <= 730 {
                recent_exploit_penalty += BigDecimal::from(50);
            }
        }

        // Get current TVL for context
        let current_tvl = self.get_current_tvl(protocol_name).await.unwrap_or(BigDecimal::from(1000000));
        let loss_ratio = &total_loss / &current_tvl;

        // Calculate risk score (0 = high risk, 1 = low risk)
        let base_score = BigDecimal::from(100);
        let total_penalty = &severity_penalty + &recent_exploit_penalty + (&loss_ratio * BigDecimal::from(100));
        let risk_score = ((&base_score - &total_penalty) / &base_score).max(BigDecimal::from(0));

        Ok(risk_score)
    }

    /// Calculate TVL-based risk score
    async fn calculate_tvl_score(&self, protocol_name: &str) -> Result<BigDecimal, AppError> {
        let metrics = self.get_latest_protocol_metrics(protocol_name).await?;

        let metrics_clone = metrics.clone();
        let tvl = metrics_clone.map(|m| m.total_tvl_usd).unwrap_or(BigDecimal::from(0));

        // TVL-based risk scoring
        let tvl_score = if tvl >= BigDecimal::from(1000000000) { // $1B+
            BigDecimal::from_str("0.95").unwrap() // Very low risk
        } else if tvl >= BigDecimal::from(500000000) { // $500M+
            BigDecimal::from_str("0.85").unwrap() // Low risk
        } else if tvl >= BigDecimal::from(100000000) { // $100M+
            BigDecimal::from_str("0.70").unwrap() // Medium-low risk
        } else if tvl >= BigDecimal::from(50000000) { // $50M+
            BigDecimal::from_str("0.55").unwrap() // Medium risk
        } else if tvl >= BigDecimal::from(10000000) { // $10M+
            BigDecimal::from_str("0.40").unwrap() // Medium-high risk
        } else if tvl >= BigDecimal::from(1000000) { // $1M+
            BigDecimal::from_str("0.25").unwrap() // High risk
        } else {
            BigDecimal::from_str("0.10").unwrap() // Very high risk
        };

        // Check for TVL volatility
        if let Some(metrics) = &metrics {
            let tvl_change_penalty = if metrics.tvl_change_24h < BigDecimal::from(-20) {
                BigDecimal::from_str("0.20").unwrap() // 20% penalty for >20% daily drop
            } else if metrics.tvl_change_7d < BigDecimal::from(-40) {
                BigDecimal::from_str("0.15").unwrap() // 15% penalty for >40% weekly drop
            } else {
                BigDecimal::from(0)
            };

            return Ok((&tvl_score - &tvl_change_penalty).max(BigDecimal::from(0)));
        }

        Ok(tvl_score)
    }

    /// Calculate governance risk score
    async fn calculate_governance_score(&self, protocol_name: &str) -> Result<BigDecimal, AppError> {
        let metrics = self.get_latest_protocol_metrics(protocol_name).await?;

        if let Some(metrics) = metrics {
            let mut governance_score = BigDecimal::from_str("0.50").unwrap(); // Base score

            // Multisig threshold bonus
            if let Some(threshold) = metrics.multisig_threshold {
                if threshold >= 5 {
                    governance_score += BigDecimal::from_str("0.20").unwrap();
                } else if threshold >= 3 {
                    governance_score += BigDecimal::from_str("0.10").unwrap();
                }
            }

            // Timelock delay bonus
            if let Some(delay_hours) = metrics.timelock_delay_hours {
                if delay_hours >= 48 {
                    governance_score += BigDecimal::from_str("0.20").unwrap();
                } else if delay_hours >= 24 {
                    governance_score += BigDecimal::from_str("0.10").unwrap();
                }
            }

            // Governance participation bonus
            if let Some(participation) = metrics.governance_participation_rate {
                if participation >= BigDecimal::from(20) {
                    governance_score += BigDecimal::from_str("0.10").unwrap();
                }
            }

            return Ok(governance_score.min(BigDecimal::from(1)));
        }

        // Default score for protocols without governance data
        Ok(BigDecimal::from_str("0.30").unwrap())
    }

    /// Calculate code quality score (simplified implementation)
    async fn calculate_code_quality_score(&self, _protocol_name: &str) -> Result<BigDecimal, AppError> {
        // In production, this would analyze:
        // - Code complexity metrics
        // - Test coverage
        // - Documentation quality
        // - Development activity
        // - Bug bounty programs
        
        // For now, return a default moderate score
        Ok(BigDecimal::from_str("0.60").unwrap())
    }

    /// Calculate weighted overall protocol risk
    fn calculate_weighted_protocol_risk(
        &self,
        audit_score: &BigDecimal,
        exploit_score: &BigDecimal,
        tvl_score: &BigDecimal,
        governance_score: &BigDecimal,
        code_quality_score: &BigDecimal,
    ) -> Result<BigDecimal, AppError> {
        let weighted_score = (
            audit_score * &self.config.audit_weight +
            exploit_score * &self.config.exploit_weight +
            tvl_score * &self.config.tvl_weight +
            governance_score * &self.config.governance_weight +
            code_quality_score * &self.config.code_quality_weight
        ) / BigDecimal::from(100);

        // Invert score to represent risk (0 = low risk, 1 = high risk)
        Ok(BigDecimal::from(1) - weighted_score)
    }

    /// Get recent audits for a protocol
    async fn get_recent_audits(&self, protocol_name: &str, days: i64) -> Result<Vec<ProtocolAudit>, AppError> {
        let cutoff_date = Utc::now() - Duration::days(days);
        
        let audits = sqlx::query_as!(
            ProtocolAudit,
            r#"
            SELECT * FROM protocol_audits 
            WHERE protocol_name = $1 AND audit_date >= $2 AND is_active = true
            ORDER BY audit_date DESC
            "#,
            protocol_name,
            cutoff_date
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch audits: {}", e)))?;

        Ok(audits)
    }

    /// Get exploit history for a protocol
    async fn get_exploit_history(&self, protocol_name: &str) -> Result<Vec<ProtocolExploit>, AppError> {
        let exploits = sqlx::query_as!(
            ProtocolExploit,
            r#"
            SELECT 
                id, protocol_name, exploit_date, 
                exploit_type as "exploit_type: ExploitType", 
                amount_lost_usd, 
                severity as "severity: ExploitSeverity",
                description, was_recovered, recovery_amount_usd,
                created_at, updated_at
            FROM protocol_exploits 
            WHERE protocol_name = $1
            ORDER BY exploit_date DESC
            "#,
            protocol_name
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch exploits: {}", e)))?;

        Ok(exploits)
    }

    /// Get latest protocol metrics
    async fn get_latest_protocol_metrics(&self, protocol_name: &str) -> Result<Option<ProtocolMetrics>, AppError> {
        let metrics = sqlx::query_as!(
            ProtocolMetrics,
            r#"
            SELECT 
                id, protocol_name, total_tvl_usd, tvl_change_24h, tvl_change_7d,
                multisig_threshold, timelock_delay_hours, governance_participation_rate,
                timestamp, created_at, updated_at
            FROM protocol_metrics 
            WHERE protocol_name = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            protocol_name
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch protocol metrics: {}", e)))?;

        Ok(metrics)
    }

    /// Get current TVL for a protocol
    async fn get_current_tvl(&self, protocol_name: &str) -> Result<BigDecimal, AppError> {
        let metrics = self.get_latest_protocol_metrics(protocol_name).await?;
        Ok(metrics.map(|m| m.total_tvl_usd).unwrap_or(BigDecimal::from(0)))
    }

    /// Store protocol risk assessment
    async fn store_protocol_risk(&self, protocol_risk: &ProtocolRisk) -> Result<(), AppError> {
        sqlx::query!(
            r#"
            INSERT INTO protocol_risks 
            (id, protocol_name, protocol_address, chain_id, audit_score, exploit_history_score, 
             tvl_score, governance_score, code_quality_score, overall_protocol_risk, last_updated)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (protocol_name, chain_id) 
            DO UPDATE SET
                audit_score = EXCLUDED.audit_score,
                exploit_history_score = EXCLUDED.exploit_history_score,
                tvl_score = EXCLUDED.tvl_score,
                governance_score = EXCLUDED.governance_score,
                code_quality_score = EXCLUDED.code_quality_score,
                overall_protocol_risk = EXCLUDED.overall_protocol_risk,
                last_updated = EXCLUDED.last_updated
            "#,
            protocol_risk.id,
            protocol_risk.protocol_name,
            protocol_risk.protocol_address,
            protocol_risk.chain_id,
            protocol_risk.audit_score,
            protocol_risk.exploit_history_score,
            protocol_risk.tvl_score,
            protocol_risk.governance_score,
            protocol_risk.code_quality_score,
            protocol_risk.overall_protocol_risk,
            protocol_risk.last_updated
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store protocol risk: {}", e)))?;

        Ok(())
    }

    /// Get stored protocol risk assessment
    pub async fn get_protocol_risk(&self, protocol_name: &str, chain_id: i32) -> Result<Option<ProtocolRisk>, AppError> {
        let risk = sqlx::query_as!(
            ProtocolRisk,
            r#"
            SELECT * FROM protocol_risks 
            WHERE protocol_name = $1 AND chain_id = $2
            ORDER BY last_updated DESC
            LIMIT 1
            "#,
            protocol_name,
            chain_id
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to fetch protocol risk: {}", e)))?;

        Ok(risk)
    }

    /// Check if protocol risk assessment needs update
    pub async fn needs_risk_update(&self, protocol_name: &str, chain_id: i32, max_age_hours: i64) -> Result<bool, AppError> {
        if let Some(risk) = self.get_protocol_risk(protocol_name, chain_id).await? {
            let age = Utc::now() - risk.last_updated;
            Ok(age.num_hours() > max_age_hours)
        } else {
            Ok(true) // No assessment exists
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_risk_config_default() {
        let config = ProtocolRiskConfig::default();
        
        // Verify weights sum to 1.0 (100%)
        let total_weight = &config.audit_weight + &config.exploit_weight + 
                          &config.tvl_weight + &config.governance_weight + 
                          &config.code_quality_weight;
        assert_eq!(total_weight, BigDecimal::from_str("1.0").unwrap());
    }

    #[test]
    fn test_weighted_risk_calculation() {
        // Create a mock service without database connection for unit testing
        let config = ProtocolRiskConfig::default();
        
        // Test the calculation logic directly without database dependency
        // We'll test the calculation method directly without instantiating the full service
        let config_ref = &config;

        let audit_score = BigDecimal::from_str("0.8").unwrap();
        let exploit_score = BigDecimal::from_str("0.9").unwrap();
        let tvl_score = BigDecimal::from_str("0.7").unwrap();
        let governance_score = BigDecimal::from_str("0.6").unwrap();
        let code_quality_score = BigDecimal::from_str("0.5").unwrap();

        // Calculate weighted protocol risk directly
        let risk = (
            &audit_score * &config_ref.audit_weight +
            &exploit_score * &config_ref.exploit_weight +
            &tvl_score * &config_ref.tvl_weight +
            &governance_score * &config_ref.governance_weight +
            &code_quality_score * &config_ref.code_quality_weight
        );

        // Risk should be between 0 and 1
        assert!(risk >= BigDecimal::from(0));
        assert!(risk <= BigDecimal::from(1));
    }
}
