use crate::database::advanced_pool::{
    AdvancedConnectionPool, AdvancedPoolConfig, PoolLoadMetrics, 
    ConnectionHealth, StatementCacheStats, PoolLoadTester, LoadTestResults
};
use crate::error::AppError;
use sqlx::PgPool;
use tracing::{info, error, warn, debug};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use num_traits::{ToPrimitive, FromPrimitive};
use rust_decimal::Decimal;
use bigdecimal::BigDecimal;

/// Connection pool optimization service
pub struct ConnectionPoolService {
    pools: Arc<RwLock<HashMap<String, Arc<AdvancedConnectionPool>>>>,
    db_pool: PgPool,
}

impl ConnectionPoolService {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
            db_pool,
        }
    }

    /// Create and register a new advanced connection pool
    pub async fn create_pool(
        &self,
        pool_name: String,
        database_url: &str,
        config: AdvancedPoolConfig,
    ) -> Result<Arc<AdvancedConnectionPool>, AppError> {
        info!("Creating advanced connection pool: {}", pool_name);
        
        let pool = Arc::new(AdvancedConnectionPool::new(database_url, config).await?);
        
        // Register the pool
        let mut pools = self.pools.write().await;
        pools.insert(pool_name.clone(), Arc::clone(&pool));
        
        // Store initial metrics
        self.store_pool_metrics(&pool_name, &pool).await?;
        
        info!("Advanced connection pool '{}' created and registered", pool_name);
        Ok(pool)
    }

    /// Get a registered pool by name
    pub async fn get_pool(&self, pool_name: &str) -> Option<Arc<AdvancedConnectionPool>> {
        let pools = self.pools.read().await;
        pools.get(pool_name).cloned()
    }

    /// List all registered pools
    pub async fn list_pools(&self) -> Vec<String> {
        let pools = self.pools.read().await;
        pools.keys().cloned().collect()
    }

    /// Remove a pool from registry
    pub async fn remove_pool(&self, pool_name: &str) -> Result<(), AppError> {
        let mut pools = self.pools.write().await;
        if let Some(pool) = pools.remove(pool_name) {
            pool.stop_monitoring().await;
            info!("Pool '{}' removed from registry", pool_name);
        }
        Ok(())
    }

    /// Store pool metrics to database
    pub async fn store_pool_metrics(
        &self,
        pool_name: &str,
        pool: &AdvancedConnectionPool,
    ) -> Result<(), AppError> {
        let metrics = pool.get_pool_stats().await;
        let health = pool.get_health_status().await;
        let cache_stats = pool.get_statement_cache_stats().await;
        let config = pool.get_config();

        // Store pool metrics
        sqlx::query!(
            r#"
            INSERT INTO connection_pool_metrics (
                pool_name, max_connections, min_connections, current_size,
                idle_connections, active_connections, utilization_rate,
                avg_acquire_time_ms, pending_acquires, total_acquires,
                failed_acquires, connections_created, connections_closed,
                health_check_failures, last_health_check, avg_health_response_ms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
            pool_name,
            config.max_connections as i32,
            config.min_connections as i32,
            pool.get_pool().size() as i32,
            pool.get_pool().num_idle() as i32,
            (pool.get_pool().size() - pool.get_pool().num_idle() as u32) as i32,
            BigDecimal::from_f64(metrics.utilization_rate).unwrap_or_default(),
            metrics.avg_acquire_time_ms as i64,
            metrics.pending_acquires as i32,
            metrics.total_acquires as i64,
            metrics.failed_acquires as i64,
            metrics.connections_created as i64,
            metrics.connections_closed as i64,
            health.failed_checks as i32,
            health.last_check,
            health.response_time_ms as i64
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store pool metrics: {}", e)))?;

        // Store health status
        sqlx::query!(
            r#"
            INSERT INTO connection_health_status (
                pool_name, is_healthy, health_score, last_check,
                response_time_ms, failed_checks, consecutive_failures,
                total_queries, error_rate, validation_success
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
            pool_name,
            health.is_healthy,
            BigDecimal::from_f64(1.0 - (health.failed_checks as f64 / 10.0).min(1.0)).unwrap_or_default(),
            health.last_check,
            health.response_time_ms as i64,
            health.failed_checks as i32,
            health.failed_checks as i32, // Simplified
            health.total_queries as i64,
            BigDecimal::from_f64(health.error_rate).unwrap_or_default(),
            true
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store health status: {}", e)))?;

        // Store statement cache metrics
        sqlx::query!(
            r#"
            INSERT INTO statement_cache_metrics (
                pool_name, cache_size, cache_capacity, hit_rate,
                total_hits, total_misses, total_evictions,
                avg_cache_lookup_ms
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            pool_name,
            cache_stats.cache_size as i32,
            cache_stats.cache_capacity as i32,
            BigDecimal::from_f64(cache_stats.hit_rate).unwrap_or_default(),
            cache_stats.total_hits as i64,
            cache_stats.total_misses as i64,
            cache_stats.evictions as i64,
            BigDecimal::from_f64(0.1).unwrap_or_default() // Estimated
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store cache metrics: {}", e)))?;

        debug!("Stored metrics for pool '{}'", pool_name);
        Ok(())
    }

    /// Get pool performance summary
    pub async fn get_pool_performance_summary(&self, pool_name: &str) -> Result<PoolPerformanceSummary, AppError> {
        let summary = sqlx::query_as!(
            PoolPerformanceSummary,
            r#"
            SELECT 
                pool_name,
                utilization_rate as "utilization_rate!: BigDecimal",
                avg_acquire_time_ms,
                failed_acquires,
                total_acquires,
                CASE 
                    WHEN utilization_rate > 0.9 THEN 'HIGH'
                    WHEN utilization_rate > 0.7 THEN 'MEDIUM'
                    ELSE 'LOW'
                END as "load_level!",
                timestamp
            FROM connection_pool_metrics 
            WHERE pool_name = $1 
            ORDER BY timestamp DESC 
            LIMIT 1
            "#,
            pool_name
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get performance summary: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("No metrics found for pool '{}'", pool_name)))?;

        Ok(summary)
    }

    /// Get pool health summary
    pub async fn get_pool_health_summary(&self, pool_name: &str) -> Result<PoolHealthSummary, AppError> {
        let summary = sqlx::query_as!(
            PoolHealthSummary,
            r#"
            SELECT 
                pool_name,
                is_healthy,
                health_score as "health_score!: BigDecimal",
                response_time_ms,
                error_rate as "error_rate!: BigDecimal",
                last_check
            FROM connection_health_status 
            WHERE pool_name = $1 
            ORDER BY timestamp DESC 
            LIMIT 1
            "#,
            pool_name
        )
        .fetch_optional(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get health summary: {}", e)))?
        .ok_or_else(|| AppError::NotFound(format!("No health data found for pool '{}'", pool_name)))?;

        Ok(summary)
    }

    /// Run load test on a pool
    pub async fn run_load_test(
        &self,
        pool_name: &str,
        concurrent_requests: u32,
        duration_secs: u64,
    ) -> Result<LoadTestResults, AppError> {
        let pool = self.get_pool(pool_name).await
            .ok_or_else(|| AppError::NotFound(format!("Pool '{}' not found", pool_name)))?;

        let load_tester = PoolLoadTester::new(pool);
        let results = load_tester.run_load_test(concurrent_requests, duration_secs).await?;

        // Store test results
        self.store_load_test_results(pool_name, &results).await?;

        Ok(results)
    }

    /// Store load test results
    async fn store_load_test_results(
        &self,
        pool_name: &str,
        results: &LoadTestResults,
    ) -> Result<(), AppError> {
        let performance_grade = self.calculate_performance_grade(results);
        let recommendations = self.generate_pool_recommendations(results);

        sqlx::query!(
            r#"
            INSERT INTO pool_load_test_results (
                pool_name, concurrent_requests, test_duration_secs,
                total_requests, total_errors, error_rate,
                avg_response_time_ms, requests_per_second,
                pool_max_connections, pool_min_connections,
                peak_utilization, avg_utilization,
                recommended_max_connections, recommended_min_connections,
                performance_grade, notes
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
            pool_name,
            results.concurrent_requests as i32,
            results.duration_secs as i32,
            results.total_requests as i64,
            results.total_errors as i64,
            BigDecimal::from_f64(results.error_rate).unwrap_or_default(),
            results.avg_response_time_ms as i64,
            BigDecimal::from_f64(results.requests_per_second as f64).unwrap_or_default(),
            100, // Default max connections
            20,  // Default min connections
            BigDecimal::from_f64(results.pool_stats.utilization_rate).unwrap_or_default(),
            BigDecimal::from_f64(results.pool_stats.utilization_rate).unwrap_or_default(),
            recommendations.recommended_max_connections,
            recommendations.recommended_min_connections,
            performance_grade.to_string(),
            recommendations.notes
        )
        .execute(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to store load test results: {}", e)))?;

        info!("Stored load test results for pool '{}'", pool_name);
        Ok(())
    }

    /// Calculate performance grade based on test results
    fn calculate_performance_grade(&self, results: &LoadTestResults) -> char {
        let error_rate = results.error_rate;
        let avg_response_time = results.avg_response_time_ms;
        let utilization = results.pool_stats.utilization_rate;

        match (error_rate, avg_response_time, utilization) {
            (e, r, u) if e < 0.01 && r < 50 && u < 0.8 => 'A',
            (e, r, u) if e < 0.05 && r < 100 && u < 0.9 => 'B',
            (e, r, u) if e < 0.10 && r < 200 && u < 0.95 => 'C',
            (e, r, _u) if e < 0.20 && r < 500 => 'D',
            _ => 'F',
        }
    }

    /// Generate pool size recommendations
    fn generate_pool_recommendations(&self, results: &LoadTestResults) -> PoolRecommendations {
        let utilization = results.pool_stats.utilization_rate;
        let error_rate = results.error_rate;
        let current_max = 100; // Would get from actual pool config
        let current_min = 20;

        let (recommended_max, recommended_min, notes) = if error_rate > 0.05 {
            // High error rate, increase pool size
            let new_max = (current_max as f64 * 1.5).ceil() as i32;
            let new_min = (current_min as f64 * 1.2).ceil() as i32;
            (new_max, new_min, "High error rate detected. Increase pool size to handle load.".to_string())
        } else if utilization > 0.9 {
            // High utilization, scale up
            let new_max = (current_max as f64 * 1.3).ceil() as i32;
            let new_min = current_min;
            (new_max, new_min, "High utilization detected. Increase max connections.".to_string())
        } else if utilization < 0.3 {
            // Low utilization, can scale down
            let new_max = (current_max as f64 * 0.8).ceil() as i32;
            let new_min = (current_min as f64 * 0.8).ceil() as i32;
            (new_max.max(10), new_min.max(5), "Low utilization. Pool can be scaled down to save resources.".to_string())
        } else {
            // Optimal range
            (current_max, current_min, "Pool size is optimal for current load.".to_string())
        };

        PoolRecommendations {
            recommended_max_connections: recommended_max,
            recommended_min_connections: recommended_min,
            notes,
        }
    }

    /// Get historical performance trends
    pub async fn get_performance_trends(
        &self,
        pool_name: &str,
        hours: i32,
    ) -> Result<Vec<PerformanceTrend>, AppError> {
        let trends = sqlx::query_as!(
            PerformanceTrend,
            r#"
            SELECT 
                timestamp,
                utilization_rate as "utilization_rate!: BigDecimal",
                avg_acquire_time_ms,
                failed_acquires,
                total_acquires
            FROM connection_pool_metrics 
            WHERE pool_name = $1 
                AND timestamp >= NOW() - INTERVAL '1 hour' * $2
            ORDER BY timestamp ASC
            "#,
            pool_name,
            hours as f64
        )
        .fetch_all(&self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get performance trends: {}", e)))?;

        Ok(trends)
    }

    /// Optimize all registered pools based on current metrics
    pub async fn optimize_all_pools(&self) -> Result<OptimizationReport, AppError> {
        let pool_names = self.list_pools().await;
        let mut optimizations = Vec::new();
        let mut total_pools = 0;
        let mut optimized_pools = 0;

        for pool_name in pool_names {
            total_pools += 1;
            
            if let Ok(optimization) = self.optimize_pool(&pool_name).await {
                if optimization.action_taken {
                    optimized_pools += 1;
                }
                optimizations.push(optimization);
            }
        }

        Ok(OptimizationReport {
            total_pools,
            optimized_pools,
            optimizations,
            timestamp: Utc::now(),
        })
    }

    /// Optimize a specific pool
    async fn optimize_pool(&self, pool_name: &str) -> Result<PoolOptimization, AppError> {
        let performance = self.get_pool_performance_summary(pool_name).await?;
        let health = self.get_pool_health_summary(pool_name).await?;
        
        let utilization = performance.utilization_rate.to_f64().unwrap_or(0.0);
        let mut recommendations = Vec::new();
        let mut action_taken = false;

        // Analyze utilization
        if utilization > 0.9 {
            recommendations.push("Consider increasing max_connections".to_string());
        } else if utilization < 0.2 {
            recommendations.push("Consider decreasing max_connections to save resources".to_string());
        }

        // Analyze health
        if !health.is_healthy {
            recommendations.push("Pool health issues detected - check connection validation".to_string());
            action_taken = true;
        }

        // Analyze response time
        if performance.avg_acquire_time_ms > 1000 {
            recommendations.push("High connection acquire time - consider pool tuning".to_string());
        }

        Ok(PoolOptimization {
            pool_name: pool_name.to_string(),
            current_utilization: utilization,
            health_score: health.health_score.to_f64().unwrap_or(0.0),
            recommendations,
            action_taken,
            timestamp: Utc::now(),
        })
    }
}

// Data structures for responses
#[derive(Debug, Serialize, Deserialize)]
pub struct PoolPerformanceSummary {
    pub pool_name: String,
    pub utilization_rate: BigDecimal,
    pub avg_acquire_time_ms: i64,
    pub failed_acquires: i64,
    pub total_acquires: i64,
    pub load_level: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PoolHealthSummary {
    pub pool_name: String,
    pub is_healthy: bool,
    pub health_score: BigDecimal,
    pub response_time_ms: i64,
    pub error_rate: BigDecimal,
    pub last_check: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceTrend {
    pub timestamp: DateTime<Utc>,
    pub utilization_rate: BigDecimal,
    pub avg_acquire_time_ms: i64,
    pub failed_acquires: i64,
    pub total_acquires: i64,
}

#[derive(Debug, Serialize)]
pub struct PoolRecommendations {
    pub recommended_max_connections: i32,
    pub recommended_min_connections: i32,
    pub notes: String,
}

#[derive(Debug, Serialize)]
pub struct PoolOptimization {
    pub pool_name: String,
    pub current_utilization: f64,
    pub health_score: f64,
    pub recommendations: Vec<String>,
    pub action_taken: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OptimizationReport {
    pub total_pools: usize,
    pub optimized_pools: usize,
    pub optimizations: Vec<PoolOptimization>,
    pub timestamp: DateTime<Utc>,
}
