use sqlx::{PgPool, Row, Postgres, QueryBuilder};
use crate::error::AppError;
use tracing::{info, warn, error, debug};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Database query service with optimization and caching
#[derive(Clone)]
pub struct DatabaseQueryService {
    pool: PgPool,
    query_cache: std::sync::Arc<tokio::sync::RwLock<HashMap<String, CachedQuery>>>,
    performance_metrics: std::sync::Arc<tokio::sync::RwLock<QueryPerformanceMetrics>>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CachedQuery {
    result: String, // JSON serialized result
    timestamp: chrono::DateTime<chrono::Utc>,
    ttl_seconds: u64,
}

#[derive(Debug, Default, Serialize)]
pub struct QueryPerformanceMetrics {
    pub total_queries: u64,
    pub avg_query_time_ms: f64,
    pub slow_queries: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub failed_queries: u64,
    pub slowest_queries: Vec<SlowQuery>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SlowQuery {
    pub query_hash: String,
    pub duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub query_type: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResult<T> {
    pub data: T,
    pub execution_time_ms: u64,
    pub from_cache: bool,
    pub query_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
}

impl DatabaseQueryService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            query_cache: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            performance_metrics: std::sync::Arc::new(tokio::sync::RwLock::new(QueryPerformanceMetrics::default())),
        }
    }

    /// Execute a query with performance monitoring (simplified without dynamic params)
    pub async fn execute_query(
        &self,
        query: &str,
    ) -> Result<QueryResult<Vec<sqlx::postgres::PgRow>>, AppError> {
        let start_time = std::time::Instant::now();
        
        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Query execution failed: {}", e)))?;
        
        let execution_time = start_time.elapsed();
        
        // Update performance metrics
        self.performance_metrics.write().await.total_queries += 1;
        self.performance_metrics.write().await.avg_query_time_ms = (self.performance_metrics.read().await.avg_query_time_ms * (self.performance_metrics.read().await.total_queries - 1) as f64 + execution_time.as_millis() as f64) / self.performance_metrics.read().await.total_queries as f64;
        
        Ok(QueryResult {
            data: rows,
            execution_time_ms: execution_time.as_millis() as u64,
            from_cache: false,
            query_hash: String::new(),
        })
    }

    /// Execute a paginated query with sorting
    pub async fn execute_paginated_query(
        &self,
        base_query: &str,
        count_query: &str,
        _params: &[&(dyn sqlx::Encode<'_, Postgres> + Send + Sync)],
        pagination: PaginationParams,
    ) -> Result<PaginatedResult<sqlx::postgres::PgRow>, AppError> {
        let page = pagination.page.unwrap_or(1).max(1);
        let per_page = pagination.per_page.unwrap_or(50).min(1000).max(1); // Max 1000 per page
        let offset = (page - 1) * per_page;

        // Build the complete query with sorting and pagination
        let mut query_builder = QueryBuilder::new(base_query);
        
        // Add sorting if specified
        if let Some(sort_by) = &pagination.sort_by {
            let sort_order = pagination.sort_order.as_deref().unwrap_or("ASC");
            query_builder.push(" ORDER BY ");
            query_builder.push(sort_by);
            query_builder.push(" ");
            query_builder.push(sort_order);
        }
        
        // Add pagination
        query_builder.push(" LIMIT ");
        query_builder.push_bind(per_page as i64);
        query_builder.push(" OFFSET ");
        query_builder.push_bind(offset as i64);

        let query = query_builder.build();
        
        // Execute both queries concurrently
        let (data_result, count_result) = tokio::try_join!(
            query.fetch_all(&self.pool),
            sqlx::query_scalar::<_, i64>(count_query).fetch_one(&self.pool)
        ).map_err(|e| AppError::DatabaseError(format!("Paginated query failed: {}", e)))?;

        let total_count = count_result as u32;
        let total_pages = (total_count + per_page - 1) / per_page;

        Ok(PaginatedResult {
            data: data_result,
            page,
            per_page,
            total_count,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        })
    }

    /// Execute bulk insert with batch optimization
    pub async fn bulk_insert<T>(
        &self,
        table_name: &str,
        columns: &[&str],
        data: Vec<T>,
        batch_size: usize,
    ) -> Result<u64, AppError>
    where
        T: Send + Sync,
        for<'a> &'a T: sqlx::Encode<'a, Postgres> + sqlx::Type<Postgres>,
    {
        if data.is_empty() {
            return Ok(0);
        }

        let mut total_inserted = 0u64;
        let chunks: Vec<_> = data.chunks(batch_size).collect();
        
        info!("Executing bulk insert: {} records in {} batches", data.len(), chunks.len());

        for (batch_idx, chunk) in chunks.iter().enumerate() {
            let start_time = Instant::now();
            
            // Build dynamic insert query
            let mut query_builder = QueryBuilder::new("INSERT INTO ");
            query_builder.push(table_name);
            query_builder.push(" (");
            
            for (i, column) in columns.iter().enumerate() {
                if i > 0 {
                    query_builder.push(", ");
                }
                query_builder.push(*column);
            }
            
            query_builder.push(") VALUES ");
            
            for (row_idx, row_data) in chunk.iter().enumerate() {
                if row_idx > 0 {
                    query_builder.push(", ");
                }
                query_builder.push("(");
                
                for (col_idx, _) in columns.iter().enumerate() {
                    if col_idx > 0 {
                        query_builder.push(", ");
                    }
                    query_builder.push_bind(row_data);
                }
                
                query_builder.push(")");
            }

            let query = query_builder.build();
            let result = query.execute(&self.pool).await
                .map_err(|e| AppError::DatabaseError(format!("Bulk insert batch {} failed: {}", batch_idx + 1, e)))?;
            
            total_inserted += result.rows_affected();
            
            let batch_duration = start_time.elapsed();
            debug!("Bulk insert batch {}/{} completed: {} rows in {:?}", 
                   batch_idx + 1, chunks.len(), chunk.len(), batch_duration);
        }

        info!("Bulk insert completed: {} total records inserted", total_inserted);
        Ok(total_inserted)
    }

    /// Execute transaction with automatic rollback on error
    pub async fn execute_transaction<F, T>(&self, transaction_fn: F) -> Result<T, AppError>
    where
        F: for<'a> FnOnce(&'a mut sqlx::Transaction<'_, Postgres>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T, AppError>> + Send + 'a>>,
    {
        let mut tx = self.pool.begin().await
            .map_err(|e| AppError::DatabaseError(format!("Failed to begin transaction: {}", e)))?;

        match transaction_fn(&mut tx).await {
            Ok(result) => {
                tx.commit().await
                    .map_err(|e| AppError::DatabaseError(format!("Failed to commit transaction: {}", e)))?;
                Ok(result)
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    error!("Failed to rollback transaction: {}", rollback_err);
                }
                Err(e)
            }
        }
    }

    /// Get query performance metrics
    pub async fn get_performance_metrics(&self) -> QueryPerformanceMetrics {
        let metrics = self.performance_metrics.read().await;
        QueryPerformanceMetrics {
            total_queries: metrics.total_queries,
            avg_query_time_ms: metrics.avg_query_time_ms,
            slow_queries: metrics.slow_queries,
            cache_hits: metrics.cache_hits,
            cache_misses: metrics.cache_misses,
            failed_queries: metrics.failed_queries,
            slowest_queries: metrics.slowest_queries.clone(),
        }
    }

    /// Clear query cache
    pub async fn clear_cache(&self) {
        let mut cache = self.query_cache.write().await;
        cache.clear();
        info!("Query cache cleared");
    }

    /// Refresh materialized views
    pub async fn refresh_materialized_views(&self) -> Result<(), AppError> {
        info!("Refreshing materialized views");
        
        sqlx::query("SELECT refresh_materialized_views()")
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to refresh materialized views: {}", e)))?;
        
        info!("Materialized views refreshed successfully");
        Ok(())
    }

    // Private helper methods
    
    #[allow(dead_code)]
    async fn execute_with_monitoring(
        &self,
        query: sqlx::query::Query<'_, Postgres, sqlx::postgres::PgArguments>,
        query_hash: &str,
    ) -> Result<Vec<sqlx::postgres::PgRow>, AppError> {
        let start_time = Instant::now();
        
        let result = query.fetch_all(&self.pool).await;
        
        let duration = start_time.elapsed();
        self.record_query_performance(query_hash, duration, result.is_ok()).await;
        
        result.map_err(|e| AppError::DatabaseError(format!("Query execution failed: {}", e)))
    }

    #[allow(dead_code)]
    fn generate_query_hash(&self, query: &str, params: &[&(dyn sqlx::Encode<'_, Postgres> + Send + Sync)]) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        params.len().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    #[allow(dead_code)]
    async fn get_from_cache(&self, cache_key: &str) -> Option<String> {
        let cache = self.query_cache.read().await;
        if let Some(cached) = cache.get(cache_key) {
            let age = chrono::Utc::now().signed_duration_since(cached.timestamp);
            if age.num_seconds() < cached.ttl_seconds as i64 {
                return Some(cached.result.clone());
            }
        }
        None
    }

    #[allow(dead_code)]
    async fn store_in_cache(&self, cache_key: &str, result: &str, ttl_seconds: u64) {
        let mut cache = self.query_cache.write().await;
        cache.insert(cache_key.to_string(), CachedQuery {
            result: result.to_string(),
            timestamp: chrono::Utc::now(),
            ttl_seconds,
        });
    }

    #[allow(dead_code)]
    async fn record_cache_hit(&self) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.cache_hits += 1;
    }

    #[allow(dead_code)]
    async fn record_cache_miss(&self) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.cache_misses += 1;
    }

    #[allow(dead_code)]
    async fn record_query_performance(&self, query_hash: &str, duration: Duration, success: bool) {
        let mut metrics = self.performance_metrics.write().await;
        
        metrics.total_queries += 1;
        
        if !success {
            metrics.failed_queries += 1;
            return;
        }

        let duration_ms = duration.as_millis() as u64;
        
        // Update average (simple moving average)
        metrics.avg_query_time_ms = 
            (metrics.avg_query_time_ms * (metrics.total_queries - 1) as f64 + duration_ms as f64) 
            / metrics.total_queries as f64;

        // Track slow queries (>1000ms)
        if duration_ms > 1000 {
            metrics.slow_queries += 1;
            
            let slow_query = SlowQuery {
                query_hash: query_hash.to_string(),
                duration_ms,
                timestamp: chrono::Utc::now(),
                query_type: "unknown".to_string(), // Could be enhanced to detect query type
            };
            
            metrics.slowest_queries.push(slow_query);
            
            // Keep only the 10 slowest queries
            metrics.slowest_queries.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
            metrics.slowest_queries.truncate(10);
            
            warn!("Slow query detected: {} ms (hash: {})", duration_ms, query_hash);
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResult<T> {
    pub data: Vec<T>,
    pub page: u32,
    pub per_page: u32,
    pub total_count: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Database health monitoring service
#[derive(Clone)]
pub struct DatabaseHealthMonitor {
    query_service: DatabaseQueryService,
}

impl DatabaseHealthMonitor {
    pub fn new(pool: PgPool) -> Self {
        Self {
            query_service: DatabaseQueryService::new(pool),
        }
    }

    /// Perform comprehensive database health check
    pub async fn comprehensive_health_check(&self) -> Result<DatabaseHealthReport, AppError> {
        let start_time = Instant::now();
        
        // Basic connectivity test
        let connectivity_result = self.test_connectivity().await;
        
        // Performance metrics
        let performance_metrics = self.query_service.get_performance_metrics().await;
        
        // Table statistics
        let table_stats = self.get_table_statistics().await?;
        
        // Index usage statistics
        let index_stats = self.get_index_statistics().await?;
        
        // Connection pool status
        let pool_stats = crate::database::connection::get_pool_stats(&self.query_service.pool);
        
        let total_duration = start_time.elapsed();
        
        Ok(DatabaseHealthReport {
            is_healthy: connectivity_result.is_ok(),
            connectivity_status: connectivity_result.map(|_| "OK".to_string()).unwrap_or_else(|e| e.to_string()),
            performance_metrics,
            table_statistics: table_stats,
            index_statistics: index_stats,
            pool_statistics: pool_stats,
            check_duration_ms: total_duration.as_millis() as u64,
            timestamp: chrono::Utc::now(),
        })
    }

    async fn test_connectivity(&self) -> Result<(), AppError> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.query_service.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Connectivity test failed: {}", e)))?;
        Ok(())
    }

    async fn get_table_statistics(&self) -> Result<Vec<TableStatistics>, AppError> {
        let rows = sqlx::query(r#"
            SELECT 
                schemaname,
                tablename,
                n_tup_ins as inserts,
                n_tup_upd as updates,
                n_tup_del as deletes,
                n_live_tup as live_tuples,
                n_dead_tup as dead_tuples,
                last_vacuum,
                last_autovacuum,
                last_analyze,
                last_autoanalyze
            FROM pg_stat_user_tables
            ORDER BY n_live_tup DESC
        "#)
        .fetch_all(&self.query_service.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get table statistics: {}", e)))?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(TableStatistics {
                schema_name: row.get("schemaname"),
                table_name: row.get("tablename"),
                inserts: row.get::<i64, _>("inserts") as u64,
                updates: row.get::<i64, _>("updates") as u64,
                deletes: row.get::<i64, _>("deletes") as u64,
                live_tuples: row.get::<i64, _>("live_tuples") as u64,
                dead_tuples: row.get::<i64, _>("dead_tuples") as u64,
                last_vacuum: row.get("last_vacuum"),
                last_analyze: row.get("last_analyze"),
            });
        }

        Ok(stats)
    }

    async fn get_index_statistics(&self) -> Result<Vec<IndexStatistics>, AppError> {
        let rows = sqlx::query(r#"
            SELECT 
                schemaname,
                tablename,
                indexname,
                idx_tup_read,
                idx_tup_fetch,
                idx_scan
            FROM pg_stat_user_indexes
            ORDER BY idx_scan DESC
        "#)
        .fetch_all(&self.query_service.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get index statistics: {}", e)))?;

        let mut stats = Vec::new();
        for row in rows {
            stats.push(IndexStatistics {
                schema_name: row.get("schemaname"),
                table_name: row.get("tablename"),
                index_name: row.get("indexname"),
                scans: row.get::<i64, _>("idx_scan") as u64,
                tuples_read: row.get::<i64, _>("idx_tup_read") as u64,
                tuples_fetched: row.get::<i64, _>("idx_tup_fetch") as u64,
            });
        }

        Ok(stats)
    }
}

#[derive(Debug, Serialize)]
pub struct DatabaseHealthReport {
    pub is_healthy: bool,
    pub connectivity_status: String,
    pub performance_metrics: QueryPerformanceMetrics,
    pub table_statistics: Vec<TableStatistics>,
    pub index_statistics: Vec<IndexStatistics>,
    pub pool_statistics: crate::database::connection::ConnectionPoolStats,
    pub check_duration_ms: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct TableStatistics {
    pub schema_name: String,
    pub table_name: String,
    pub inserts: u64,
    pub updates: u64,
    pub deletes: u64,
    pub live_tuples: u64,
    pub dead_tuples: u64,
    pub last_vacuum: Option<chrono::DateTime<chrono::Utc>>,
    pub last_analyze: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
pub struct IndexStatistics {
    pub schema_name: String,
    pub table_name: String,
    pub index_name: String,
    pub scans: u64,
    pub tuples_read: u64,
    pub tuples_fetched: u64,
}
