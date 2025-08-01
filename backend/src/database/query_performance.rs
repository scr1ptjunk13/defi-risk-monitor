use sqlx::{PgPool, Row, Postgres, QueryBuilder};
use crate::error::AppError;
use tracing::{info, warn, error, debug, instrument};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;

/// Enhanced query performance monitoring service with EXPLAIN ANALYZE
#[derive(Clone)]
pub struct QueryPerformanceService {
    pool: PgPool,
    performance_metrics: Arc<RwLock<QueryPerformanceMetrics>>,
    query_plans: Arc<RwLock<HashMap<String, QueryPlan>>>,
    slow_query_threshold_ms: u64,
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct QueryPerformanceMetrics {
    pub total_queries: u64,
    pub avg_query_time_ms: f64,
    pub slow_queries: u64,
    pub failed_queries: u64,
    pub queries_by_type: HashMap<String, QueryTypeMetrics>,
    pub slowest_queries: Vec<SlowQueryRecord>,
    pub query_plan_cache_hits: u64,
    pub query_plan_cache_misses: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueryTypeMetrics {
    pub count: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: u64,
    pub min_duration_ms: u64,
    pub error_count: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct SlowQueryRecord {
    pub query_hash: String,
    pub query_type: String,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub execution_plan: Option<String>,
    pub table_scans: u32,
    pub index_scans: u32,
    pub rows_examined: u64,
    pub rows_returned: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct QueryPlan {
    pub plan_hash: String,
    pub execution_plan: String,
    pub estimated_cost: f64,
    pub estimated_rows: u64,
    pub cached_at: DateTime<Utc>,
    pub usage_count: u64,
    pub avg_actual_time: f64,
}

#[derive(Debug, Serialize)]
pub struct QueryAnalysis {
    pub query_hash: String,
    pub execution_time_ms: u64,
    pub plan_analysis: PlanAnalysis,
    pub performance_recommendations: Vec<String>,
    pub index_recommendations: Vec<IndexRecommendation>,
}

#[derive(Debug, Serialize)]
pub struct PlanAnalysis {
    pub total_cost: f64,
    pub startup_cost: f64,
    pub rows_estimate: u64,
    pub width_estimate: u32,
    pub actual_time_ms: f64,
    pub actual_rows: u64,
    pub seq_scans: u32,
    pub index_scans: u32,
    pub nested_loops: u32,
    pub hash_joins: u32,
    pub sort_operations: u32,
}

#[derive(Debug, Serialize)]
pub struct IndexRecommendation {
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub estimated_benefit: String,
    pub reason: String,
}

impl QueryPerformanceService {
    pub fn new(pool: PgPool, slow_query_threshold_ms: u64) -> Self {
        Self {
            pool,
            performance_metrics: Arc::new(RwLock::new(QueryPerformanceMetrics::default())),
            query_plans: Arc::new(RwLock::new(HashMap::new())),
            slow_query_threshold_ms,
        }
    }

    /// Execute query with comprehensive performance monitoring and analysis
    #[instrument(skip(self, query))]
    pub async fn execute_with_analysis(&self, 
        query: &str, 
        query_type: &str
    ) -> Result<(Vec<sqlx::postgres::PgRow>, QueryAnalysis), AppError> {
        let start_time = Instant::now();
        let query_hash = self.generate_query_hash(query);
        
        // Execute the actual query
        let rows = match sqlx::query(query).fetch_all(&self.pool).await {
            Ok(rows) => rows,
            Err(e) => {
                self.record_failed_query(&query_hash, query_type, start_time.elapsed()).await;
                return Err(AppError::DatabaseError(e.to_string()));
            }
        };
        
        let execution_time = start_time.elapsed();
        let execution_time_ms = execution_time.as_millis() as u64;
        
        // Get or generate execution plan analysis
        let plan_analysis = if execution_time_ms >= self.slow_query_threshold_ms {
            self.analyze_query_plan(query).await?
        } else {
            // Use cached plan if available
            self.get_cached_plan_analysis(&query_hash).await
        };
        
        // Generate performance recommendations
        let performance_recommendations = self.generate_performance_recommendations(&plan_analysis, execution_time_ms);
        let index_recommendations = self.generate_index_recommendations(query, &plan_analysis);
        
        let query_analysis = QueryAnalysis {
            query_hash: query_hash.clone(),
            execution_time_ms,
            plan_analysis,
            performance_recommendations,
            index_recommendations,
        };
        
        // Record performance metrics
        self.record_query_performance(&query_hash, query_type, execution_time, true, &query_analysis).await;
        
        Ok((rows, query_analysis))
    }

    /// Analyze query execution plan using EXPLAIN ANALYZE
    #[instrument(skip(self, query))]
    async fn analyze_query_plan(&self, 
        query: &str
    ) -> Result<PlanAnalysis, AppError> {
        let explain_query = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) {}", query);
        
        let explain_result = sqlx::query(&explain_query).fetch_one(&self.pool).await
            .map_err(|e| AppError::DatabaseError(format!("Failed to execute EXPLAIN ANALYZE: {}", e)))?;
        
        let plan_json: serde_json::Value = explain_result.try_get(0)
            .map_err(|e| AppError::DatabaseError(format!("Failed to parse EXPLAIN result: {}", e)))?;
        
        self.parse_execution_plan(&plan_json).await
    }

    /// Parse PostgreSQL execution plan JSON
    async fn parse_execution_plan(&self, plan_json: &serde_json::Value) -> Result<PlanAnalysis, AppError> {
        let plan = &plan_json[0]["Plan"];
        
        let total_cost = plan["Total Cost"].as_f64().unwrap_or(0.0);
        let startup_cost = plan["Startup Cost"].as_f64().unwrap_or(0.0);
        let rows_estimate = plan["Plan Rows"].as_u64().unwrap_or(0);
        let width_estimate = plan["Plan Width"].as_u64().unwrap_or(0) as u32;
        let actual_time_ms = plan["Actual Total Time"].as_f64().unwrap_or(0.0);
        let actual_rows = plan["Actual Rows"].as_u64().unwrap_or(0);
        
        // Analyze plan nodes for operation counts
        let mut seq_scans = 0u32;
        let mut index_scans = 0u32;
        let mut nested_loops = 0u32;
        let mut hash_joins = 0u32;
        let mut sort_operations = 0u32;
        
        self.count_plan_operations(plan, &mut seq_scans, &mut index_scans, 
                                  &mut nested_loops, &mut hash_joins, &mut sort_operations);
        
        Ok(PlanAnalysis {
            total_cost,
            startup_cost,
            rows_estimate,
            width_estimate,
            actual_time_ms,
            actual_rows,
            seq_scans,
            index_scans,
            nested_loops,
            hash_joins,
            sort_operations,
        })
    }

    /// Recursively count operations in execution plan
    fn count_plan_operations(&self, 
        plan: &serde_json::Value,
        seq_scans: &mut u32,
        index_scans: &mut u32,
        nested_loops: &mut u32,
        hash_joins: &mut u32,
        sort_operations: &mut u32
    ) {
        if let Some(node_type) = plan["Node Type"].as_str() {
            match node_type {
                "Seq Scan" => *seq_scans += 1,
                "Index Scan" | "Index Only Scan" | "Bitmap Index Scan" => *index_scans += 1,
                "Nested Loop" => *nested_loops += 1,
                "Hash Join" => *hash_joins += 1,
                "Sort" => *sort_operations += 1,
                _ => {}
            }
        }
        
        // Recursively process child plans
        if let Some(plans) = plan["Plans"].as_array() {
            for child_plan in plans {
                self.count_plan_operations(child_plan, seq_scans, index_scans, 
                                         nested_loops, hash_joins, sort_operations);
            }
        }
    }

    /// Get cached plan analysis or return default
    async fn get_cached_plan_analysis(&self, query_hash: &str) -> PlanAnalysis {
        let plans = self.query_plans.read().await;
        if let Some(cached_plan) = plans.get(query_hash) {
            let mut metrics = self.performance_metrics.write().await;
            metrics.query_plan_cache_hits += 1;
            
            PlanAnalysis {
                total_cost: cached_plan.estimated_cost,
                startup_cost: 0.0,
                rows_estimate: cached_plan.estimated_rows,
                width_estimate: 0,
                actual_time_ms: cached_plan.avg_actual_time,
                actual_rows: cached_plan.estimated_rows,
                seq_scans: 0,
                index_scans: 0,
                nested_loops: 0,
                hash_joins: 0,
                sort_operations: 0,
            }
        } else {
            let mut metrics = self.performance_metrics.write().await;
            metrics.query_plan_cache_misses += 1;
            
            PlanAnalysis {
                total_cost: 0.0,
                startup_cost: 0.0,
                rows_estimate: 0,
                width_estimate: 0,
                actual_time_ms: 0.0,
                actual_rows: 0,
                seq_scans: 0,
                index_scans: 0,
                nested_loops: 0,
                hash_joins: 0,
                sort_operations: 0,
            }
        }
    }

    /// Generate performance recommendations based on plan analysis
    fn generate_performance_recommendations(&self, plan: &PlanAnalysis, execution_time_ms: u64) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        if execution_time_ms > self.slow_query_threshold_ms {
            recommendations.push("Query execution time exceeds threshold - consider optimization".to_string());
        }
        
        if plan.seq_scans > 0 {
            recommendations.push(format!("Query performs {} sequential scan(s) - consider adding indexes", plan.seq_scans));
        }
        
        if plan.nested_loops > 3 {
            recommendations.push("High number of nested loops detected - consider query restructuring".to_string());
        }
        
        if plan.sort_operations > 2 {
            recommendations.push("Multiple sort operations detected - consider adding composite indexes".to_string());
        }
        
        if plan.actual_rows > plan.rows_estimate * 10 {
            recommendations.push("Actual rows significantly exceed estimate - consider updating table statistics".to_string());
        }
        
        if plan.total_cost > 10000.0 {
            recommendations.push("High query cost detected - consider query optimization or partitioning".to_string());
        }
        
        recommendations
    }

    /// Generate index recommendations based on query analysis
    fn generate_index_recommendations(&self, query: &str, plan: &PlanAnalysis) -> Vec<IndexRecommendation> {
        let mut recommendations = Vec::new();
        
        // Simple heuristic-based index recommendations
        if plan.seq_scans > 0 {
            // Extract table names and common WHERE clause patterns
            if let Some(table_name) = self.extract_table_name(query) {
                if query.contains("WHERE") {
                    let columns = self.extract_where_columns(query);
                    if !columns.is_empty() {
                        recommendations.push(IndexRecommendation {
                            table_name: table_name.clone(),
                            columns,
                            index_type: "B-tree".to_string(),
                            estimated_benefit: "High".to_string(),
                            reason: "Eliminate sequential scan".to_string(),
                        });
                    }
                }
                
                if query.contains("ORDER BY") {
                    let order_columns = self.extract_order_by_columns(query);
                    if !order_columns.is_empty() {
                        recommendations.push(IndexRecommendation {
                            table_name,
                            columns: order_columns,
                            index_type: "B-tree".to_string(),
                            estimated_benefit: "Medium".to_string(),
                            reason: "Optimize ORDER BY clause".to_string(),
                        });
                    }
                }
            }
        }
        
        recommendations
    }

    /// Extract table name from query (simple pattern matching)
    fn extract_table_name(&self, query: &str) -> Option<String> {
        let query_lower = query.to_lowercase();
        if let Some(from_pos) = query_lower.find("from ") {
            let after_from = &query_lower[from_pos + 5..];
            if let Some(space_pos) = after_from.find(' ') {
                Some(after_from[..space_pos].trim().to_string())
            } else {
                Some(after_from.trim().to_string())
            }
        } else {
            None
        }
    }

    /// Extract WHERE clause columns (simple pattern matching)
    fn extract_where_columns(&self, query: &str) -> Vec<String> {
        let mut columns = Vec::new();
        let query_lower = query.to_lowercase();
        
        // Simple pattern matching for common WHERE patterns
        let patterns = ["where ", "and ", "or "];
        for pattern in &patterns {
            if let Some(pos) = query_lower.find(pattern) {
                let after_pattern = &query_lower[pos + pattern.len()..];
                if let Some(eq_pos) = after_pattern.find('=') {
                    let column = after_pattern[..eq_pos].trim();
                    if !column.is_empty() && !columns.contains(&column.to_string()) {
                        columns.push(column.to_string());
                    }
                }
            }
        }
        
        columns
    }

    /// Extract ORDER BY columns
    fn extract_order_by_columns(&self, query: &str) -> Vec<String> {
        let mut columns = Vec::new();
        let query_lower = query.to_lowercase();
        
        if let Some(order_pos) = query_lower.find("order by ") {
            let after_order = &query_lower[order_pos + 9..];
            let order_clause = if let Some(limit_pos) = after_order.find(" limit") {
                &after_order[..limit_pos]
            } else {
                after_order
            };
            
            for column in order_clause.split(',') {
                let col = column.trim().replace(" desc", "").replace(" asc", "");
                if !col.is_empty() {
                    columns.push(col);
                }
            }
        }
        
        columns
    }

    /// Record query performance metrics
    async fn record_query_performance(&self, 
        query_hash: &str,
        query_type: &str, 
        duration: Duration,
        success: bool,
        analysis: &QueryAnalysis
    ) {
        let mut metrics = self.performance_metrics.write().await;
        let duration_ms = duration.as_millis() as u64;
        
        metrics.total_queries += 1;
        
        if !success {
            metrics.failed_queries += 1;
        }
        
        if duration_ms >= self.slow_query_threshold_ms {
            metrics.slow_queries += 1;
            
            // Add to slowest queries (keep top 100)
            metrics.slowest_queries.push(SlowQueryRecord {
                query_hash: query_hash.to_string(),
                query_type: query_type.to_string(),
                duration_ms,
                timestamp: Utc::now(),
                execution_plan: Some(serde_json::to_string(&analysis.plan_analysis).unwrap_or_default()),
                table_scans: analysis.plan_analysis.seq_scans,
                index_scans: analysis.plan_analysis.index_scans,
                rows_examined: analysis.plan_analysis.actual_rows,
                rows_returned: analysis.plan_analysis.actual_rows,
            });
            
            // Keep only top 100 slowest queries
            metrics.slowest_queries.sort_by(|a, b| b.duration_ms.cmp(&a.duration_ms));
            metrics.slowest_queries.truncate(100);
        }
        
        // Update query type metrics
        let type_metrics = metrics.queries_by_type.entry(query_type.to_string()).or_insert(QueryTypeMetrics {
            count: 0,
            avg_duration_ms: 0.0,
            max_duration_ms: 0,
            min_duration_ms: u64::MAX,
            error_count: 0,
        });
        
        type_metrics.count += 1;
        if !success {
            type_metrics.error_count += 1;
        }
        
        // Update duration statistics
        let total_time = type_metrics.avg_duration_ms * (type_metrics.count - 1) as f64 + duration_ms as f64;
        type_metrics.avg_duration_ms = total_time / type_metrics.count as f64;
        type_metrics.max_duration_ms = type_metrics.max_duration_ms.max(duration_ms);
        type_metrics.min_duration_ms = type_metrics.min_duration_ms.min(duration_ms);
        
        // Update overall average
        let total_time = metrics.avg_query_time_ms * (metrics.total_queries - 1) as f64 + duration_ms as f64;
        metrics.avg_query_time_ms = total_time / metrics.total_queries as f64;
    }

    /// Record failed query
    async fn record_failed_query(&self, _query_hash: &str, query_type: &str, _duration: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.total_queries += 1;
        metrics.failed_queries += 1;
        
        let type_metrics = metrics.queries_by_type.entry(query_type.to_string()).or_insert(QueryTypeMetrics {
            count: 0,
            avg_duration_ms: 0.0,
            max_duration_ms: 0,
            min_duration_ms: u64::MAX,
            error_count: 0,
        });
        
        type_metrics.count += 1;
        type_metrics.error_count += 1;
    }

    /// Generate query hash for caching and identification
    fn generate_query_hash(&self, query: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        query.hash(&mut hasher);
        format!("query_{:x}", hasher.finish())
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> QueryPerformanceMetrics {
        self.performance_metrics.read().await.clone()
    }

    /// Reset performance metrics
    pub async fn reset_metrics(&self) {
        let mut metrics = self.performance_metrics.write().await;
        *metrics = QueryPerformanceMetrics::default();
    }

    /// Get slow queries report
    pub async fn get_slow_queries_report(&self, limit: usize) -> Vec<SlowQueryRecord> {
        let metrics = self.performance_metrics.read().await;
        metrics.slowest_queries.iter().take(limit).cloned().collect()
    }

    /// Cache query plan for future reference
    pub async fn cache_query_plan(&self, query_hash: String, plan: QueryPlan) {
        let mut plans = self.query_plans.write().await;
        plans.insert(query_hash, plan);
        
        // Limit cache size to prevent memory bloat
        if plans.len() > 1000 {
            // Remove oldest entries (simple LRU approximation)
            let oldest_key = plans.keys().next().unwrap().clone();
            plans.remove(&oldest_key);
        }
    }
}
