use crate::error::AppError;
use crate::database::{get_pool_stats, ConnectionPoolStats};
use sqlx::{PgPool, Row};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};
use chrono::{DateTime, Utc};

/// System Health Service for comprehensive database and application monitoring
pub struct SystemHealthService {
    pub(crate) pool: PgPool,
}

/// Comprehensive database metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseMetrics {
    pub connection_stats: ConnectionPoolStats,
    pub database_size_mb: i64,
    pub active_connections: i32,
    pub max_connections: i32,
    pub total_queries: i64,
    pub cache_hit_ratio: f64,
    pub index_hit_ratio: f64,
    pub deadlocks: i64,
    pub slow_queries: i64,
    pub locks_count: i32,
    pub temp_files: i64,
    pub temp_bytes: i64,
    pub uptime_seconds: i64,
    pub transactions_per_second: f64,
    pub disk_usage: DatabaseDiskUsage,
    pub replication_lag_ms: Option<i64>,
    pub timestamp: DateTime<Utc>,
}

/// Database disk usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseDiskUsage {
    pub total_size_mb: i64,
    pub data_size_mb: i64,
    pub index_size_mb: i64,
    pub toast_size_mb: i64,
    pub free_space_mb: Option<i64>,
}

/// Query performance statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryPerformanceStats {
    pub total_queries: i64,
    pub avg_query_time_ms: f64,
    pub slow_queries_count: i64,
    pub slow_query_threshold_ms: i64,
    pub queries_per_second: f64,
    pub top_slow_queries: Vec<SlowQueryInfo>,
    pub query_cache_stats: QueryCacheStats,
    pub index_usage_stats: Vec<IndexUsageInfo>,
    pub table_scan_stats: Vec<TableScanInfo>,
    pub lock_wait_stats: LockWaitStats,
    pub timestamp: DateTime<Utc>,
}

/// Information about slow queries
#[derive(Debug, Serialize, Deserialize)]
pub struct SlowQueryInfo {
    pub query_hash: String,
    pub query_text: String,
    pub avg_time_ms: f64,
    pub total_time_ms: f64,
    pub calls: i64,
    pub rows_avg: f64,
    pub hit_percent: f64,
}

/// Query cache statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct QueryCacheStats {
    pub cache_hit_ratio: f64,
    pub shared_buffers_hit_ratio: f64,
    pub buffer_cache_hit_ratio: f64,
    pub effective_cache_size_mb: i64,
}

/// Index usage information
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexUsageInfo {
    pub table_name: String,
    pub index_name: String,
    pub index_scans: i64,
    pub tuples_read: i64,
    pub tuples_fetched: i64,
    pub size_mb: i64,
    pub usage_ratio: f64,
}

/// Table scan statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct TableScanInfo {
    pub table_name: String,
    pub seq_scans: i64,
    pub seq_tup_read: i64,
    pub idx_scans: i64,
    pub idx_tup_fetch: i64,
    pub n_tup_ins: i64,
    pub n_tup_upd: i64,
    pub n_tup_del: i64,
    pub scan_ratio: f64,
}

/// Lock wait statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct LockWaitStats {
    pub total_lock_waits: i64,
    pub avg_lock_wait_time_ms: f64,
    pub deadlocks: i64,
    pub lock_timeouts: i64,
    pub active_locks: i32,
}

/// Connection pool health information
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionPoolHealth {
    pub pool_stats: ConnectionPoolStats,
    pub health_score: f64,
    pub status: PoolHealthStatus,
    pub connection_errors: i64,
    pub connection_timeouts: i64,
    pub avg_connection_time_ms: f64,
    pub pool_utilization_percent: f64,
    pub idle_connection_percent: f64,
    pub recommendations: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

/// Pool health status enum
#[derive(Debug, Serialize, Deserialize)]
pub enum PoolHealthStatus {
    Healthy,
    Warning,
    Critical,
    Degraded,
}

/// Table size information
#[derive(Debug, Serialize, Deserialize)]
pub struct TableSizeInfo {
    pub table_name: String,
    pub schema_name: String,
    pub total_size_mb: i64,
    pub table_size_mb: i64,
    pub index_size_mb: i64,
    pub toast_size_mb: i64,
    pub row_count: i64,
    pub avg_row_size_bytes: i64,
    pub bloat_ratio: Option<f64>,
    pub last_vacuum: Option<DateTime<Utc>>,
    pub last_analyze: Option<DateTime<Utc>>,
}

/// Complete table sizes summary
#[derive(Debug, Serialize, Deserialize)]
pub struct TableSizes {
    pub total_database_size_mb: i64,
    pub total_tables_size_mb: i64,
    pub total_indexes_size_mb: i64,
    pub total_toast_size_mb: i64,
    pub table_count: i32,
    pub largest_tables: Vec<TableSizeInfo>,
    pub growth_rate_mb_per_day: Option<f64>,
    pub recommendations: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

impl SystemHealthService {
    /// Create a new system health service
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get comprehensive database metrics
    pub async fn get_database_metrics(&self) -> Result<DatabaseMetrics, AppError> {
        info!("Collecting comprehensive database metrics");
        let start_time = Instant::now();

        // Get connection pool stats
        let connection_stats = get_pool_stats(&self.pool);

        // Get database size
        let db_size_row = sqlx::query(
            "SELECT pg_database_size(current_database()) as size"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get database size: {}", e)))?;
        let database_size_mb: i64 = db_size_row.get::<i64, _>("size") / (1024 * 1024);

        // Get connection statistics
        let conn_stats = sqlx::query(
            r#"
            SELECT 
                count(*) as active_connections,
                (SELECT setting::int FROM pg_settings WHERE name = 'max_connections') as max_connections
            FROM pg_stat_activity
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get connection stats: {}", e)))?;

        let active_connections: i32 = conn_stats.get::<i64, _>("active_connections") as i32;
        let max_connections: i32 = conn_stats.get("max_connections");

        // Get database statistics
        let stats_row = sqlx::query(
            r#"
            SELECT 
                COALESCE(sum(numbackends), 0)::bigint as total_queries,
                COALESCE(sum(xact_commit + xact_rollback), 0)::bigint as transactions,
                COALESCE(sum(blks_hit)::float8 / NULLIF(sum(blks_hit + blks_read), 0) * 100, 0.0) as cache_hit_ratio,
                COALESCE(sum(deadlocks), 0)::bigint as deadlocks,
                COALESCE(sum(temp_files), 0)::bigint as temp_files,
                COALESCE(sum(temp_bytes), 0)::bigint as temp_bytes,
                EXTRACT(EPOCH FROM (now() - pg_postmaster_start_time()))::float8 as uptime_seconds
            FROM pg_stat_database 
            WHERE datname = current_database()
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get database stats: {}", e)))?;

        let total_queries: i64 = stats_row.get("total_queries");
        let cache_hit_ratio: f64 = stats_row.get("cache_hit_ratio");
        let deadlocks: i64 = stats_row.get("deadlocks");
        let temp_files: i64 = stats_row.get("temp_files");
        let temp_bytes: i64 = stats_row.get("temp_bytes");
        let uptime_seconds: i64 = stats_row.get::<f64, _>("uptime_seconds") as i64;

        // Get index hit ratio
        let index_hit_ratio = self.get_index_hit_ratio().await?;

        // Get slow queries count
        let slow_queries = self.get_slow_queries_count().await?;

        // Get current locks
        let locks_count = self.get_active_locks_count().await?;

        // Calculate transactions per second
        let transactions_per_second = if uptime_seconds > 0 {
            total_queries as f64 / uptime_seconds as f64
        } else {
            0.0
        };

        // Get disk usage
        let disk_usage = self.get_disk_usage().await?;

        // Get replication lag (if applicable)
        let replication_lag_ms = self.get_replication_lag().await.ok();

        let metrics = DatabaseMetrics {
            connection_stats,
            database_size_mb,
            active_connections,
            max_connections,
            total_queries,
            cache_hit_ratio,
            index_hit_ratio,
            deadlocks,
            slow_queries,
            locks_count,
            temp_files,
            temp_bytes,
            uptime_seconds,
            transactions_per_second,
            disk_usage,
            replication_lag_ms,
            timestamp: Utc::now(),
        };

        let duration = start_time.elapsed();
        info!("Database metrics collected in {:?}", duration);
        Ok(metrics)
    }

    /// Get query performance statistics
    pub async fn get_query_performance_stats(&self) -> Result<QueryPerformanceStats, AppError> {
        info!("Collecting query performance statistics");
        let start_time = Instant::now();

        // Check if pg_stat_statements extension is available
        let extension_check = sqlx::query(
            "SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements'"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to check pg_stat_statements: {}", e)))?;

        let (total_queries, avg_query_time_ms, slow_queries_count, queries_per_second, top_slow_queries) = 
            if extension_check.is_some() {
                self.get_pg_stat_statements_data().await?
            } else {
                warn!("pg_stat_statements extension not available, using basic statistics");
                self.get_basic_query_stats().await?
            };

        let query_cache_stats = self.get_query_cache_stats().await?;
        let index_usage_stats = self.get_index_usage_stats().await?;
        let table_scan_stats = self.get_table_scan_stats().await?;
        let lock_wait_stats = self.get_lock_wait_stats().await?;

        let stats = QueryPerformanceStats {
            total_queries,
            avg_query_time_ms,
            slow_queries_count,
            slow_query_threshold_ms: 1000,
            queries_per_second,
            top_slow_queries,
            query_cache_stats,
            index_usage_stats,
            table_scan_stats,
            lock_wait_stats,
            timestamp: Utc::now(),
        };

        let duration = start_time.elapsed();
        info!("Query performance stats collected in {:?}", duration);
        Ok(stats)
    }

    /// Get connection pool health
    pub async fn get_connection_pool_health(&self) -> Result<ConnectionPoolHealth, AppError> {
        info!("Analyzing connection pool health");
        let start_time = Instant::now();

        let pool_stats = get_pool_stats(&self.pool);
        
        let pool_utilization_percent = if pool_stats.max_connections > 0 {
            (pool_stats.active as f64 / pool_stats.max_connections as f64) * 100.0
        } else {
            0.0
        };

        let idle_connection_percent = if pool_stats.size > 0 {
            (pool_stats.idle as f64 / pool_stats.size as f64) * 100.0
        } else {
            0.0
        };

        let connection_errors = self.get_connection_errors().await.unwrap_or(0);
        let connection_timeouts = self.get_connection_timeouts().await.unwrap_or(0);
        let avg_connection_time_ms = self.get_avg_connection_time().await.unwrap_or(0.0);

        let health_score = self.calculate_pool_health_score(
            pool_utilization_percent,
            idle_connection_percent,
            connection_errors,
            connection_timeouts,
        );

        let status = match health_score {
            s if s >= 0.9 => PoolHealthStatus::Healthy,
            s if s >= 0.7 => PoolHealthStatus::Warning,
            s if s >= 0.5 => PoolHealthStatus::Degraded,
            _ => PoolHealthStatus::Critical,
        };

        let recommendations = self.generate_pool_recommendations(
            &pool_stats,
            pool_utilization_percent,
            idle_connection_percent,
            connection_errors,
            connection_timeouts,
        );

        let health = ConnectionPoolHealth {
            pool_stats,
            health_score,
            status,
            connection_errors,
            connection_timeouts,
            avg_connection_time_ms,
            pool_utilization_percent,
            idle_connection_percent,
            recommendations,
            timestamp: Utc::now(),
        };

        let duration = start_time.elapsed();
        info!("Connection pool health analyzed in {:?}", duration);
        Ok(health)
    }

    /// Get table sizes
    pub async fn get_table_sizes(&self) -> Result<TableSizes, AppError> {
        info!("Collecting table size information");
        let start_time = Instant::now();

        let db_size_row = sqlx::query(
            "SELECT pg_database_size(current_database()) as total_size"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get database size: {}", e)))?;
        let total_database_size_mb: i64 = db_size_row.get::<i64, _>("total_size") / (1024 * 1024);

        let table_rows = sqlx::query(
            r#"
            SELECT 
                pg_tables.schemaname as schema_name,
                pg_tables.tablename as table_name,
                pg_total_relation_size(pg_tables.schemaname||'.'||pg_tables.tablename) as total_size,
                pg_relation_size(pg_tables.schemaname||'.'||pg_tables.tablename) as table_size,
                pg_indexes_size(pg_tables.schemaname||'.'||pg_tables.tablename) as index_size,
                COALESCE(pg_total_relation_size(pg_tables.schemaname||'.'||pg_tables.tablename) - 
                         pg_relation_size(pg_tables.schemaname||'.'||pg_tables.tablename) - 
                         pg_indexes_size(pg_tables.schemaname||'.'||pg_tables.tablename), 0) as toast_size,
                COALESCE(n_tup_ins + n_tup_upd + n_tup_del, 0) as row_estimate,
                last_vacuum,
                last_analyze
            FROM pg_tables 
            LEFT JOIN pg_stat_user_tables ON pg_tables.tablename = pg_stat_user_tables.relname
            WHERE pg_tables.schemaname = 'public'
            ORDER BY pg_total_relation_size(pg_tables.schemaname||'.'||pg_tables.tablename) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get table sizes: {}", e)))?;

        let mut largest_tables = Vec::new();
        let mut total_tables_size_mb = 0i64;
        let mut total_indexes_size_mb = 0i64;
        let mut total_toast_size_mb = 0i64;

        for row in table_rows {
            let schema_name: String = row.get("schema_name");
            let table_name: String = row.get("table_name");
            let total_size: i64 = row.get("total_size");
            let table_size: i64 = row.get("table_size");
            let index_size: i64 = row.get("index_size");
            let toast_size: i64 = row.get("toast_size");
            let row_estimate: i64 = row.get("row_estimate");
            let last_vacuum: Option<DateTime<Utc>> = row.get("last_vacuum");
            let last_analyze: Option<DateTime<Utc>> = row.get("last_analyze");

            let total_size_mb = total_size / (1024 * 1024);
            let table_size_mb = table_size / (1024 * 1024);
            let index_size_mb = index_size / (1024 * 1024);
            let toast_size_mb = toast_size / (1024 * 1024);

            total_tables_size_mb += table_size_mb;
            total_indexes_size_mb += index_size_mb;
            total_toast_size_mb += toast_size_mb;

            let avg_row_size_bytes = if row_estimate > 0 {
                table_size / row_estimate
            } else {
                0
            };

            let bloat_ratio = self.estimate_table_bloat(&table_name).await.ok();

            let table_info = TableSizeInfo {
                table_name,
                schema_name,
                total_size_mb,
                table_size_mb,
                index_size_mb,
                toast_size_mb,
                row_count: row_estimate,
                avg_row_size_bytes,
                bloat_ratio,
                last_vacuum,
                last_analyze,
            };

            largest_tables.push(table_info);
        }

        let table_count = largest_tables.len() as i32;
        let growth_rate_mb_per_day = self.estimate_growth_rate().await.ok();
        let recommendations = self.generate_table_recommendations(&largest_tables, total_database_size_mb);

        let table_sizes = TableSizes {
            total_database_size_mb,
            total_tables_size_mb,
            total_indexes_size_mb,
            total_toast_size_mb,
            table_count,
            largest_tables,
            growth_rate_mb_per_day,
            recommendations,
            timestamp: Utc::now(),
        };

        let duration = start_time.elapsed();
        info!("Table sizes collected in {:?}", duration);
        Ok(table_sizes)
    }

    // Helper methods implementation continues in next part...
}
