use crate::error::AppError;
use crate::database::ConnectionPoolStats;
use crate::services::system_health_service::*;
use sqlx::{PgPool, Row};
use tracing::warn;

impl SystemHealthService {
    // Helper methods for database metrics

    pub(crate) async fn get_index_hit_ratio(&self) -> Result<f64, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                sum(idx_blks_hit)::float / NULLIF(sum(idx_blks_hit + idx_blks_read), 0) * 100 as ratio
            FROM pg_statio_user_indexes
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get index hit ratio: {}", e)))?;

        Ok(row.get::<Option<f64>, _>("ratio").unwrap_or(0.0))
    }

    pub(crate) async fn get_slow_queries_count(&self) -> Result<i64, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM pg_stat_activity WHERE state = 'active' AND query_start < now() - interval '1 second'"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get slow queries count: {}", e)))?;

        Ok(row.get("count"))
    }

    pub(crate) async fn get_active_locks_count(&self) -> Result<i32, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) as count FROM pg_locks WHERE granted = true"
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get locks count: {}", e)))?;

        Ok(row.get::<i64, _>("count") as i32)
    }

    pub(crate) async fn get_disk_usage(&self) -> Result<DatabaseDiskUsage, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                pg_database_size(current_database())::bigint as total_size,
                COALESCE(sum(pg_relation_size(oid)), 0)::bigint as data_size,
                COALESCE(sum(pg_indexes_size(oid)), 0)::bigint as index_size,
                COALESCE(sum(pg_total_relation_size(oid) - pg_relation_size(oid) - pg_indexes_size(oid)), 0)::bigint as toast_size
            FROM pg_class 
            WHERE relkind IN ('r', 'i', 't')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get disk usage: {}", e)))?;

        let total_size_mb: i64 = row.get::<i64, _>("total_size") / (1024 * 1024);
        let data_size_mb: i64 = row.get::<i64, _>("data_size") / (1024 * 1024);
        let index_size_mb: i64 = row.get::<i64, _>("index_size") / (1024 * 1024);
        let toast_size_mb: i64 = row.get::<i64, _>("toast_size") / (1024 * 1024);

        Ok(DatabaseDiskUsage {
            total_size_mb,
            data_size_mb,
            index_size_mb,
            toast_size_mb,
            free_space_mb: None,
        })
    }

    pub(crate) async fn get_replication_lag(&self) -> Result<i64, AppError> {
        let row = sqlx::query(
            "SELECT EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp())) * 1000 as lag_ms"
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get replication lag: {}", e)))?;

        if let Some(row) = row {
            Ok(row.get::<Option<f64>, _>("lag_ms").unwrap_or(0.0) as i64)
        } else {
            Err(AppError::DatabaseError("Not a replica database".to_string()))
        }
    }

    // Helper methods for query performance stats

    pub(crate) async fn get_pg_stat_statements_data(&self) -> Result<(i64, f64, i64, f64, Vec<SlowQueryInfo>), AppError> {
        let stats_row = sqlx::query(
            r#"
            SELECT 
                sum(calls) as total_queries,
                avg(mean_exec_time) as avg_time_ms,
                sum(CASE WHEN mean_exec_time > 1000 THEN calls ELSE 0 END) as slow_queries,
                sum(calls) / EXTRACT(EPOCH FROM (now() - pg_postmaster_start_time())) as qps
            FROM pg_stat_statements
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get pg_stat_statements data: {}", e)))?;

        let total_queries: i64 = stats_row.get::<Option<i64>, _>("total_queries").unwrap_or(0);
        let avg_query_time_ms: f64 = stats_row.get::<Option<f64>, _>("avg_time_ms").unwrap_or(0.0);
        let slow_queries_count: i64 = stats_row.get::<Option<i64>, _>("slow_queries").unwrap_or(0);
        let queries_per_second: f64 = stats_row.get::<Option<f64>, _>("qps").unwrap_or(0.0);

        let slow_query_rows = sqlx::query(
            r#"
            SELECT 
                queryid::text as query_hash,
                LEFT(query, 200) as query_text,
                mean_exec_time as avg_time_ms,
                total_exec_time as total_time_ms,
                calls,
                rows / calls as rows_avg,
                shared_blks_hit::float / NULLIF(shared_blks_hit + shared_blks_read, 0) * 100 as hit_percent
            FROM pg_stat_statements 
            WHERE calls > 10
            ORDER BY mean_exec_time DESC 
            LIMIT 10
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get slow queries: {}", e)))?;

        let mut top_slow_queries = Vec::new();
        for row in slow_query_rows {
            let slow_query = SlowQueryInfo {
                query_hash: row.get("query_hash"),
                query_text: row.get("query_text"),
                avg_time_ms: row.get("avg_time_ms"),
                total_time_ms: row.get("total_time_ms"),
                calls: row.get("calls"),
                rows_avg: row.get::<Option<f64>, _>("rows_avg").unwrap_or(0.0),
                hit_percent: row.get::<Option<f64>, _>("hit_percent").unwrap_or(0.0),
            };
            top_slow_queries.push(slow_query);
        }

        Ok((total_queries, avg_query_time_ms, slow_queries_count, queries_per_second, top_slow_queries))
    }

    pub(crate) async fn get_basic_query_stats(&self) -> Result<(i64, f64, i64, f64, Vec<SlowQueryInfo>), AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                sum(numbackends) as total_queries,
                0::float8 as avg_time_ms,
                0 as slow_queries,
                sum(numbackends)::float8 / EXTRACT(EPOCH FROM (now() - pg_postmaster_start_time()))::float8 as qps
            FROM pg_stat_database 
            WHERE datname = current_database()
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get basic query stats: {}", e)))?;

        let total_queries: i64 = row.get::<Option<i64>, _>("total_queries").unwrap_or(0);
        let avg_query_time_ms: f64 = row.get("avg_time_ms");
        let slow_queries_count: i64 = row.get::<i32, _>("slow_queries") as i64;
        let queries_per_second: f64 = row.get::<Option<f64>, _>("qps").unwrap_or(0.0);

        Ok((total_queries, avg_query_time_ms, slow_queries_count, queries_per_second, Vec::new()))
    }

    pub(crate) async fn get_query_cache_stats(&self) -> Result<QueryCacheStats, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                COALESCE(sum(heap_blks_hit)::float / NULLIF(sum(heap_blks_hit + heap_blks_read), 0) * 100, 0.0) as cache_hit_ratio,
                COALESCE(sum(idx_blks_hit)::float / NULLIF(sum(idx_blks_hit + idx_blks_read), 0) * 100, 0.0) as shared_buffers_hit_ratio,
                COALESCE(sum(toast_blks_hit)::float / NULLIF(sum(toast_blks_hit + toast_blks_read), 0) * 100, 0.0) as buffer_cache_hit_ratio
            FROM pg_statio_user_tables
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get cache stats: {}", e)))?;

        let cache_hit_ratio: f64 = row.get::<Option<f64>, _>("cache_hit_ratio").unwrap_or(0.0);
        let shared_buffers_hit_ratio: f64 = row.get::<Option<f64>, _>("shared_buffers_hit_ratio").unwrap_or(0.0);
        let buffer_cache_hit_ratio: f64 = row.get::<Option<f64>, _>("buffer_cache_hit_ratio").unwrap_or(0.0);

        Ok(QueryCacheStats {
            cache_hit_ratio,
            shared_buffers_hit_ratio,
            buffer_cache_hit_ratio,
            effective_cache_size_mb: 128, // Default value, could be queried from pg_settings
        })
    }

    pub(crate) async fn get_index_usage_stats(&self) -> Result<Vec<IndexUsageInfo>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                pg_stat_user_indexes.schemaname||'.'||pg_stat_user_indexes.relname as table_name,
                pg_stat_user_indexes.indexrelname as index_name,
                pg_stat_user_indexes.idx_scan as index_scans,
                pg_stat_user_indexes.idx_tup_read as tuples_read,
                pg_stat_user_indexes.idx_tup_fetch as tuples_fetched,
                pg_relation_size(indexrelid) / (1024 * 1024) as size_mb,
                pg_stat_user_indexes.idx_scan::float / NULLIF(pg_stat_user_indexes.idx_scan + pg_stat_user_tables.seq_scan, 0) as usage_ratio
            FROM pg_stat_user_indexes
            LEFT JOIN pg_stat_user_tables ON pg_stat_user_indexes.relid = pg_stat_user_tables.relid
            ORDER BY pg_stat_user_indexes.idx_scan DESC
            LIMIT 20
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get index usage stats: {}", e)))?;

        let mut index_stats = Vec::new();
        for row in rows {
            let index_info = IndexUsageInfo {
                table_name: row.get("table_name"),
                index_name: row.get("index_name"),
                index_scans: row.get("index_scans"),
                tuples_read: row.get("tuples_read"),
                tuples_fetched: row.get("tuples_fetched"),
                size_mb: row.get("size_mb"),
                usage_ratio: row.get::<Option<f64>, _>("usage_ratio").unwrap_or(0.0),
            };
            index_stats.push(index_info);
        }

        Ok(index_stats)
    }

    pub(crate) async fn get_table_scan_stats(&self) -> Result<Vec<TableScanInfo>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT 
                pg_stat_user_tables.schemaname||'.'||pg_stat_user_tables.relname as table_name,
                seq_scan as seq_scans,
                seq_tup_read,
                idx_scan as idx_scans,
                idx_tup_fetch,
                n_tup_ins,
                n_tup_upd,
                n_tup_del,
                seq_scan::float / NULLIF(seq_scan + idx_scan, 0) as scan_ratio
            FROM pg_stat_user_tables
            ORDER BY seq_scan DESC
            LIMIT 20
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get table scan stats: {}", e)))?;

        let mut table_stats = Vec::new();
        for row in rows {
            let table_info = TableScanInfo {
                table_name: row.get("table_name"),
                seq_scans: row.get("seq_scans"),
                seq_tup_read: row.get("seq_tup_read"),
                idx_scans: row.get("idx_scans"),
                idx_tup_fetch: row.get("idx_tup_fetch"),
                n_tup_ins: row.get("n_tup_ins"),
                n_tup_upd: row.get("n_tup_upd"),
                n_tup_del: row.get("n_tup_del"),
                scan_ratio: row.get::<Option<f64>, _>("scan_ratio").unwrap_or(0.0),
            };
            table_stats.push(table_info);
        }

        Ok(table_stats)
    }

    pub(crate) async fn get_lock_wait_stats(&self) -> Result<LockWaitStats, AppError> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as active_locks,
                0::bigint as total_lock_waits,
                0.0::float8 as avg_lock_wait_time_ms,
                0::bigint as deadlocks,
                0::bigint as lock_timeouts
            FROM pg_locks 
            WHERE granted = true
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get lock wait stats: {}", e)))?;

        Ok(LockWaitStats {
            total_lock_waits: row.get("total_lock_waits"),
            avg_lock_wait_time_ms: row.get("avg_lock_wait_time_ms"),
            deadlocks: row.get("deadlocks"),
            lock_timeouts: row.get("lock_timeouts"),
            active_locks: row.get::<i64, _>("active_locks") as i32,
        })
    }

    // Helper methods for connection pool health

    pub(crate) async fn get_connection_errors(&self) -> Result<i64, AppError> {
        // This would typically come from application metrics
        // For now, return 0 as a placeholder
        Ok(0)
    }

    pub(crate) async fn get_connection_timeouts(&self) -> Result<i64, AppError> {
        // This would typically come from application metrics
        Ok(0)
    }

    pub(crate) async fn get_avg_connection_time(&self) -> Result<f64, AppError> {
        // This would typically come from application metrics
        Ok(0.0)
    }

    pub(crate) fn calculate_pool_health_score(
        &self,
        utilization_percent: f64,
        idle_percent: f64,
        connection_errors: i64,
        connection_timeouts: i64,
    ) -> f64 {
        let mut score: f64 = 1.0;

        // Penalize high utilization
        if utilization_percent > 80.0 {
            score -= 0.3;
        } else if utilization_percent > 60.0 {
            score -= 0.1;
        }

        // Penalize very low idle connections
        if idle_percent < 10.0 {
            score -= 0.2;
        }

        // Penalize connection errors
        if connection_errors > 10 {
            score -= 0.3;
        } else if connection_errors > 0 {
            score -= 0.1;
        }

        // Penalize connection timeouts
        if connection_timeouts > 5 {
            score -= 0.2;
        } else if connection_timeouts > 0 {
            score -= 0.05;
        }

        score.max(0.0).min(1.0)
    }

    pub(crate) fn generate_pool_recommendations(
        &self,
        pool_stats: &ConnectionPoolStats,
        utilization_percent: f64,
        idle_percent: f64,
        connection_errors: i64,
        connection_timeouts: i64,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if utilization_percent > 80.0 {
            recommendations.push("Consider increasing max_connections pool size".to_string());
        }

        if idle_percent < 10.0 {
            recommendations.push("Consider increasing min_connections to maintain idle connections".to_string());
        }

        if connection_errors > 0 {
            recommendations.push("Investigate connection errors and database connectivity".to_string());
        }

        if connection_timeouts > 0 {
            recommendations.push("Consider increasing acquire_timeout or optimizing query performance".to_string());
        }

        if pool_stats.active > (pool_stats.max_connections * 9 / 10) {
            recommendations.push("Pool nearing capacity - monitor for connection exhaustion".to_string());
        }

        recommendations
    }

    // Helper methods for table sizes

    pub(crate) async fn estimate_table_bloat(&self, table_name: &str) -> Result<f64, AppError> {
        // Simplified bloat estimation - in production, use pgstattuple extension
        let row = sqlx::query(
            r#"
            SELECT 
                pg_stat_get_live_tuples(oid) as live_tuples,
                pg_stat_get_dead_tuples(oid) as dead_tuples
            FROM pg_class 
            WHERE relname = $1 AND relkind = 'r'
            "#
        )
        .bind(table_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to estimate table bloat: {}", e)))?;

        if let Some(row) = row {
            let live_tuples: i64 = row.get("live_tuples");
            let dead_tuples: i64 = row.get("dead_tuples");
            
            if live_tuples + dead_tuples > 0 {
                Ok(dead_tuples as f64 / (live_tuples + dead_tuples) as f64)
            } else {
                Ok(0.0)
            }
        } else {
            Ok(0.0)
        }
    }

    pub(crate) async fn estimate_growth_rate(&self) -> Result<f64, AppError> {
        // This would require historical data tracking
        // For now, return a placeholder
        Ok(0.0)
    }

    pub(crate) fn generate_table_recommendations(
        &self,
        tables: &[TableSizeInfo],
        total_db_size_mb: i64,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Check for large tables that might need attention
        for table in tables.iter().take(5) {
            if table.total_size_mb > 1000 {
                recommendations.push(format!("Large table '{}' ({} MB) - consider partitioning or archiving", 
                    table.table_name, table.total_size_mb));
            }

            if let Some(bloat_ratio) = table.bloat_ratio {
                if bloat_ratio > 0.3 {
                    recommendations.push(format!("Table '{}' has high bloat ratio ({:.1}%) - consider VACUUM FULL", 
                        table.table_name, bloat_ratio * 100.0));
                }
            }

            if table.last_vacuum.is_none() || table.last_analyze.is_none() {
                recommendations.push(format!("Table '{}' needs maintenance - run VACUUM and ANALYZE", 
                    table.table_name));
            }
        }

        if total_db_size_mb > 10000 {
            recommendations.push("Database size is large - consider implementing data retention policies".to_string());
        }

        recommendations
    }
}
