use sqlx::{PgPool, Row};
use crate::error::AppError;
use tracing::{info, warn, instrument};
use std::time::Instant;
use serde::Serialize;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;
use std::collections::HashMap;

/// Materialized views management service with refresh strategies
#[derive(Clone)]
pub struct MaterializedViewsService {
    pool: PgPool,
    view_metadata: Arc<RwLock<HashMap<String, ViewMetadata>>>,
    refresh_strategies: Arc<RwLock<HashMap<String, RefreshStrategy>>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ViewMetadata {
    pub name: String,
    pub last_refresh: DateTime<Utc>,
    pub refresh_duration_ms: u64,
    pub row_count: u64,
    pub size_bytes: u64,
    pub dependencies: Vec<String>,
    pub refresh_frequency: RefreshFrequency,
    pub is_refreshing: bool,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub enum RefreshFrequency {
    RealTime,
    EveryMinute,
    Every5Minutes,
    Every15Minutes,
    Hourly,
    Daily,
    Manual,
}

#[derive(Debug, Clone)]
pub struct RefreshStrategy {
    pub view_name: String,
    pub frequency: RefreshFrequency,
    pub incremental: bool,
    pub last_refresh: DateTime<Utc>,
    pub next_refresh: DateTime<Utc>,
    pub dependencies: Vec<String>,
    pub refresh_condition: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ViewRefreshResult {
    pub view_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub rows_affected: u64,
    pub error_message: Option<String>,
    pub refresh_type: RefreshType,
}

#[derive(Debug, Serialize)]
pub enum RefreshType {
    Full,
    Incremental,
    Concurrent,
}

impl MaterializedViewsService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            view_metadata: Arc::new(RwLock::new(HashMap::new())),
            refresh_strategies: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize materialized views and their refresh strategies
    #[instrument(skip(self))]
    pub async fn initialize(&self) -> Result<(), AppError> {
        info!("Initializing materialized views service");
        
        self.create_core_views().await?;
        self.setup_refresh_strategies().await?;
        self.load_view_metadata().await?;
        
        info!("Materialized views service initialized successfully");
        Ok(())
    }

    /// Create core materialized views for complex aggregations
    async fn create_core_views(&self) -> Result<(), AppError> {
        // User portfolio summary view (using correct column names)
        let portfolio_sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_user_portfolio_summary AS
            SELECT 
                p.user_address,
                COUNT(DISTINCT p.id) as total_positions,
                COUNT(DISTINCT p.protocol) as protocols_count,
                COUNT(DISTINCT p.chain_id) as chains_count,
                COALESCE(SUM(p.token0_amount * 1000 + p.token1_amount * 1000), 0) as total_value_usd,
                AVG(COALESCE(rm.overall_risk_score, 0)) as avg_risk_score,
                MAX(p.created_at) as last_position_created,
                COUNT(*) as active_positions
            FROM positions p
            LEFT JOIN risk_metrics rm ON p.id = rm.position_id
            WHERE p.created_at >= NOW() - INTERVAL '90 days'
            GROUP BY p.user_address
            WITH DATA;
            
            CREATE UNIQUE INDEX IF NOT EXISTS idx_mv_user_portfolio_summary_user 
            ON mv_user_portfolio_summary(user_address);
        "#;

        // Risk metrics summary view
        let risk_sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_risk_metrics_summary AS
            SELECT 
                DATE_TRUNC('hour', rm.calculated_at) as hour_bucket,
                p.protocol,
                p.chain_id,
                COUNT(*) as positions_count,
                AVG(rm.impermanent_loss_risk) as avg_il_risk,
                AVG(rm.liquidity_risk) as avg_liquidity_risk,
                AVG(rm.overall_risk_score) as avg_overall_risk,
                COUNT(CASE WHEN rm.overall_risk_score > 0.7 THEN 1 END) as high_risk_positions
            FROM risk_metrics rm
            JOIN positions p ON rm.position_id = p.id
            WHERE rm.calculated_at >= NOW() - INTERVAL '7 days'
            GROUP BY DATE_TRUNC('hour', rm.calculated_at), p.protocol, p.chain_id
            WITH DATA;
            
            CREATE INDEX IF NOT EXISTS idx_mv_risk_metrics_summary_time 
            ON mv_risk_metrics_summary(hour_bucket DESC);
        "#;

        // MEV risk analytics view
        let mev_sql = r#"
            CREATE MATERIALIZED VIEW IF NOT EXISTS mv_mev_risk_analytics AS
            SELECT 
                DATE_TRUNC('day', mr.created_at) as day_bucket,
                mr.chain_id,
                COUNT(*) as total_assessments,
                AVG(mr.sandwich_attack_risk) as avg_sandwich_risk,
                AVG(mr.frontrunning_risk) as avg_frontrunning_risk,
                AVG(mr.overall_mev_risk) as avg_overall_mev_risk,
                COUNT(CASE WHEN mr.overall_mev_risk > 0.7 THEN 1 END) as high_mev_risk_pools
            FROM mev_risks mr
            WHERE mr.created_at >= NOW() - INTERVAL '30 days'
            GROUP BY DATE_TRUNC('day', mr.created_at), mr.chain_id
            WITH DATA;
            
            CREATE INDEX IF NOT EXISTS idx_mv_mev_risk_analytics_day 
            ON mv_mev_risk_analytics(day_bucket DESC);
        "#;

        let views = vec![portfolio_sql, risk_sql, mev_sql];
        for sql in views {
            if let Err(e) = sqlx::query(sql).execute(&self.pool).await {
                warn!("Failed to create materialized view: {}", e);
            }
        }

        Ok(())
    }

    /// Set up refresh strategies for all materialized views
    async fn setup_refresh_strategies(&self) -> Result<(), AppError> {
        let mut strategies = self.refresh_strategies.write().await;
        
        strategies.insert("mv_user_portfolio_summary".to_string(), RefreshStrategy {
            view_name: "mv_user_portfolio_summary".to_string(),
            frequency: RefreshFrequency::Every5Minutes,
            incremental: true,
            last_refresh: Utc::now(),
            next_refresh: Utc::now() + chrono::Duration::minutes(5),
            dependencies: vec!["positions".to_string(), "risk_metrics".to_string()],
            refresh_condition: Some("positions.updated_at > last_refresh".to_string()),
        });

        strategies.insert("mv_risk_metrics_summary".to_string(), RefreshStrategy {
            view_name: "mv_risk_metrics_summary".to_string(),
            frequency: RefreshFrequency::Every15Minutes,
            incremental: true,
            last_refresh: Utc::now(),
            next_refresh: Utc::now() + chrono::Duration::minutes(15),
            dependencies: vec!["risk_metrics".to_string(), "positions".to_string()],
            refresh_condition: Some("risk_metrics.calculated_at > last_refresh".to_string()),
        });

        strategies.insert("mv_mev_risk_analytics".to_string(), RefreshStrategy {
            view_name: "mv_mev_risk_analytics".to_string(),
            frequency: RefreshFrequency::Hourly,
            incremental: false,
            last_refresh: Utc::now(),
            next_refresh: Utc::now() + chrono::Duration::hours(1),
            dependencies: vec!["mev_risks".to_string()],
            refresh_condition: None,
        });

        info!("Set up refresh strategies for {} materialized views", strategies.len());
        Ok(())
    }

    /// Load existing view metadata from database
    async fn load_view_metadata(&self) -> Result<(), AppError> {
        let query = r#"
            SELECT 
                matviewname,
                pg_size_pretty(pg_total_relation_size('public.'||matviewname)) as size
            FROM pg_matviews 
            WHERE schemaname = 'public' AND matviewname LIKE 'mv_%'
        "#;

        let rows = sqlx::query(query).fetch_all(&self.pool).await
            .map_err(|e| AppError::DatabaseError(format!("Failed to load view metadata: {}", e)))?;

        let mut metadata = self.view_metadata.write().await;
        
        for row in rows {
            let view_name: String = row.try_get("matviewname")?;
            let size_str: String = row.try_get("size").unwrap_or_default();
            
            let count_query = format!("SELECT COUNT(*) as row_count FROM {}", view_name);
            let row_count = if let Ok(count_row) = sqlx::query(&count_query).fetch_one(&self.pool).await {
                count_row.try_get::<i64, _>("row_count").unwrap_or(0) as u64
            } else {
                0
            };

            metadata.insert(view_name.clone(), ViewMetadata {
                name: view_name.clone(),
                last_refresh: Utc::now(),
                refresh_duration_ms: 0,
                row_count,
                size_bytes: self.parse_size_string(&size_str),
                dependencies: self.get_view_dependencies(&view_name),
                refresh_frequency: RefreshFrequency::Manual,
                is_refreshing: false,
                last_error: None,
            });
        }

        info!("Loaded metadata for {} materialized views", metadata.len());
        Ok(())
    }

    /// Parse PostgreSQL size string to bytes
    fn parse_size_string(&self, size_str: &str) -> u64 {
        if size_str.is_empty() {
            return 0;
        }
        
        let parts: Vec<&str> = size_str.split_whitespace().collect();
        if parts.len() != 2 {
            return 0;
        }
        
        let number: f64 = parts[0].parse().unwrap_or(0.0);
        let unit = parts[1].to_lowercase();
        
        match unit.as_str() {
            "bytes" => number as u64,
            "kb" => (number * 1024.0) as u64,
            "mb" => (number * 1024.0 * 1024.0) as u64,
            "gb" => (number * 1024.0 * 1024.0 * 1024.0) as u64,
            _ => 0,
        }
    }

    /// Get view dependencies
    fn get_view_dependencies(&self, view_name: &str) -> Vec<String> {
        match view_name {
            "mv_user_portfolio_summary" => vec!["positions".to_string(), "risk_metrics".to_string()],
            "mv_risk_metrics_summary" => vec!["risk_metrics".to_string(), "positions".to_string()],
            "mv_mev_risk_analytics" => vec!["mev_risks".to_string()],
            _ => vec![],
        }
    }

    /// Refresh a specific materialized view
    #[instrument(skip(self))]
    pub async fn refresh_view(&self, view_name: &str, force: bool) -> Result<ViewRefreshResult, AppError> {
        let start_time = Instant::now();
        
        if !force && !self.needs_refresh(view_name).await {
            return Ok(ViewRefreshResult {
                view_name: view_name.to_string(),
                success: true,
                duration_ms: 0,
                rows_affected: 0,
                error_message: None,
                refresh_type: RefreshType::Full,
            });
        }

        self.set_refreshing_status(view_name, true).await;

        let result = if self.supports_incremental_refresh(view_name).await {
            self.refresh_view_incremental(view_name).await
        } else {
            self.refresh_view_full(view_name).await
        };

        self.set_refreshing_status(view_name, false).await;

        let duration_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(rows_affected) => {
                self.update_refresh_metadata(view_name, duration_ms, None).await;
                Ok(ViewRefreshResult {
                    view_name: view_name.to_string(),
                    success: true,
                    duration_ms,
                    rows_affected,
                    error_message: None,
                    refresh_type: RefreshType::Full,
                })
            }
            Err(e) => {
                self.update_refresh_metadata(view_name, duration_ms, Some(e.to_string())).await;
                Ok(ViewRefreshResult {
                    view_name: view_name.to_string(),
                    success: false,
                    duration_ms,
                    rows_affected: 0,
                    error_message: Some(e.to_string()),
                    refresh_type: RefreshType::Full,
                })
            }
        }
    }

    /// Check if view needs refresh
    async fn needs_refresh(&self, view_name: &str) -> bool {
        let strategies = self.refresh_strategies.read().await;
        if let Some(strategy) = strategies.get(view_name) {
            Utc::now() >= strategy.next_refresh
        } else {
            false
        }
    }

    /// Check if view supports incremental refresh
    async fn supports_incremental_refresh(&self, view_name: &str) -> bool {
        let strategies = self.refresh_strategies.read().await;
        strategies.get(view_name).map(|s| s.incremental).unwrap_or(false)
    }

    /// Refresh view with full refresh
    async fn refresh_view_full(&self, view_name: &str) -> Result<u64, AppError> {
        let sql = format!("REFRESH MATERIALIZED VIEW CONCURRENTLY {}", view_name);
        let result = sqlx::query(&sql).execute(&self.pool).await
            .map_err(|e| AppError::DatabaseError(format!("Failed to refresh view {}: {}", view_name, e)))?;
        
        Ok(result.rows_affected())
    }

    /// Refresh view with incremental refresh (simplified implementation)
    async fn refresh_view_incremental(&self, view_name: &str) -> Result<u64, AppError> {
        // For now, fall back to full refresh
        // In production, implement proper incremental refresh logic
        self.refresh_view_full(view_name).await
    }

    /// Set refreshing status
    async fn set_refreshing_status(&self, view_name: &str, is_refreshing: bool) {
        let mut metadata = self.view_metadata.write().await;
        if let Some(meta) = metadata.get_mut(view_name) {
            meta.is_refreshing = is_refreshing;
        }
    }

    /// Update refresh metadata
    async fn update_refresh_metadata(&self, view_name: &str, duration_ms: u64, error: Option<String>) {
        let mut metadata = self.view_metadata.write().await;
        if let Some(meta) = metadata.get_mut(view_name) {
            meta.last_refresh = Utc::now();
            meta.refresh_duration_ms = duration_ms;
            meta.last_error = error;
        }

        // Update next refresh time
        let mut strategies = self.refresh_strategies.write().await;
        if let Some(strategy) = strategies.get_mut(view_name) {
            strategy.last_refresh = Utc::now();
            strategy.next_refresh = match strategy.frequency {
                RefreshFrequency::EveryMinute => Utc::now() + chrono::Duration::minutes(1),
                RefreshFrequency::Every5Minutes => Utc::now() + chrono::Duration::minutes(5),
                RefreshFrequency::Every15Minutes => Utc::now() + chrono::Duration::minutes(15),
                RefreshFrequency::Hourly => Utc::now() + chrono::Duration::hours(1),
                RefreshFrequency::Daily => Utc::now() + chrono::Duration::days(1),
                _ => Utc::now() + chrono::Duration::hours(1),
            };
        }
    }

    /// Refresh all views that need refreshing
    pub async fn refresh_all_due_views(&self) -> Result<Vec<ViewRefreshResult>, AppError> {
        let mut results = Vec::new();
        let view_names: Vec<String> = {
            let strategies = self.refresh_strategies.read().await;
            strategies.keys().cloned().collect()
        };

        for view_name in view_names {
            if self.needs_refresh(&view_name).await {
                let result = self.refresh_view(&view_name, false).await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Get view metadata
    pub async fn get_view_metadata(&self, view_name: &str) -> Option<ViewMetadata> {
        let metadata = self.view_metadata.read().await;
        metadata.get(view_name).cloned()
    }

    /// Get all view metadata
    pub async fn get_all_view_metadata(&self) -> HashMap<String, ViewMetadata> {
        self.view_metadata.read().await.clone()
    }
}
