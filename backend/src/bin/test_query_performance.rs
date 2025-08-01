use defi_risk_monitor::database::{
    establish_connection, QueryPerformanceService, MaterializedViewsService
};
use defi_risk_monitor::error::AppError;
use sqlx::{PgPool, Row};
use std::time::Instant;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Query Performance and Materialized Views Test");
    
    // Establish database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost:5434/defi_risk_monitor".to_string());
    let pool = establish_connection(&database_url).await?;
    info!("✅ Database connection established");
    
    // Test query performance monitoring
    test_query_performance_monitoring(&pool).await?;
    
    // Test materialized views service
    test_materialized_views_service(&pool).await?;
    
    // Test integration between services
    test_services_integration(&pool).await?;
    
    info!("🎉 All query performance and materialized views tests completed successfully!");
    Ok(())
}

async fn test_query_performance_monitoring(pool: &PgPool) -> Result<(), AppError> {
    info!("📊 Testing Query Performance Monitoring Service");
    
    // Initialize query performance service with 500ms slow query threshold
    let perf_service = QueryPerformanceService::new(pool.clone(), 500);
    
    // Test 1: Execute a fast query with analysis
    info!("Test 1: Fast query analysis");
    let fast_query = "SELECT COUNT(*) FROM positions WHERE created_at >= NOW() - INTERVAL '30 days'";
    
    let start_time = Instant::now();
    let (rows, analysis) = perf_service.execute_with_analysis(
        fast_query, 
        "count_query"
    ).await?;
    
    info!("Fast query executed in {}ms", start_time.elapsed().as_millis());
    info!("Query analysis: execution_time={}ms, recommendations={}", 
          analysis.execution_time_ms, analysis.performance_recommendations.len());
    info!("Rows returned: {}", rows.len());
    
    // Test 2: Execute a potentially slower query
    info!("Test 2: Complex aggregation query analysis");
    let complex_query = r#"
        SELECT 
            p.protocol,
            COUNT(*) as position_count,
            AVG(COALESCE(rm.overall_risk_score, 0)) as avg_risk,
            SUM(p.token0_amount + p.token1_amount) as total_amounts
        FROM positions p
        LEFT JOIN risk_metrics rm ON p.id = rm.position_id
        WHERE p.created_at >= NOW() - INTERVAL '30 days'
        GROUP BY p.protocol
        ORDER BY position_count DESC
    "#;
    
    let start_time = Instant::now();
    let (rows, analysis) = perf_service.execute_with_analysis(
        complex_query, 
        "aggregation_query"
    ).await?;
    
    info!("Complex query executed in {}ms", start_time.elapsed().as_millis());
    info!("Query analysis: execution_time={}ms, seq_scans={}, index_scans={}", 
          analysis.execution_time_ms, 
          analysis.plan_analysis.seq_scans,
          analysis.plan_analysis.index_scans);
    info!("Performance recommendations: {:?}", analysis.performance_recommendations);
    info!("Index recommendations: {}", analysis.index_recommendations.len());
    info!("Rows returned: {}", rows.len());
    
    // Test 3: Execute multiple queries to build performance metrics
    info!("Test 3: Building performance metrics with multiple queries");
    for i in 0..5 {
        let query = format!("SELECT * FROM positions LIMIT {} OFFSET {}", 10, i * 10);
        let _ = perf_service.execute_with_analysis(&query, "pagination_query").await?;
    }
    
    // Test 4: Get performance metrics
    info!("Test 4: Retrieving performance metrics");
    let metrics = perf_service.get_performance_metrics().await;
    info!("Performance Metrics:");
    info!("  Total queries: {}", metrics.total_queries);
    info!("  Average query time: {:.2}ms", metrics.avg_query_time_ms);
    info!("  Slow queries: {}", metrics.slow_queries);
    info!("  Failed queries: {}", metrics.failed_queries);
    info!("  Query plan cache hits: {}", metrics.query_plan_cache_hits);
    info!("  Query plan cache misses: {}", metrics.query_plan_cache_misses);
    info!("  Query types tracked: {}", metrics.queries_by_type.len());
    
    for (query_type, type_metrics) in &metrics.queries_by_type {
        info!("  {}: {} queries, avg {:.2}ms, {} errors", 
              query_type, type_metrics.count, type_metrics.avg_duration_ms, type_metrics.error_count);
    }
    
    // Test 5: Get slow queries report
    info!("Test 5: Slow queries report");
    let slow_queries = perf_service.get_slow_queries_report(10).await;
    info!("Found {} slow queries", slow_queries.len());
    for slow_query in &slow_queries {
        info!("  Slow query: type={}, duration={}ms, seq_scans={}", 
              slow_query.query_type, slow_query.duration_ms, slow_query.table_scans);
    }
    
    info!("✅ Query Performance Monitoring tests completed");
    Ok(())
}

async fn test_materialized_views_service(pool: &PgPool) -> Result<(), AppError> {
    info!("🔄 Testing Materialized Views Service");
    
    // Initialize materialized views service
    let mv_service = MaterializedViewsService::new(pool.clone());
    
    // Test 1: Initialize materialized views
    info!("Test 1: Initializing materialized views");
    mv_service.initialize().await?;
    info!("✅ Materialized views initialized");
    
    // Test 2: Get all view metadata
    info!("Test 2: Retrieving view metadata");
    let all_metadata = mv_service.get_all_view_metadata().await;
    info!("Found {} materialized views", all_metadata.len());
    
    for (view_name, metadata) in &all_metadata {
        info!("  View: {}", view_name);
        info!("    Row count: {}", metadata.row_count);
        info!("    Size: {} bytes", metadata.size_bytes);
        info!("    Dependencies: {:?}", metadata.dependencies);
        info!("    Last refresh: {}", metadata.last_refresh);
        info!("    Is refreshing: {}", metadata.is_refreshing);
    }
    
    // Test 3: Refresh specific views
    info!("Test 3: Refreshing specific materialized views");
    let views_to_refresh = vec![
        "mv_user_portfolio_summary",
        "mv_risk_metrics_summary", 
        "mv_mev_risk_analytics"
    ];
    
    for view_name in views_to_refresh {
        info!("Refreshing view: {}", view_name);
        let start_time = Instant::now();
        let result = mv_service.refresh_view(view_name, true).await?;
        let duration = start_time.elapsed();
        
        info!("  Refresh result: success={}, duration={}ms, rows_affected={}", 
              result.success, duration.as_millis(), result.rows_affected);
        
        if let Some(error) = &result.error_message {
            warn!("  Refresh error: {}", error);
        }
    }
    
    // Test 4: Check which views need refresh
    info!("Test 4: Checking views that need refresh");
    let refresh_results = mv_service.refresh_all_due_views().await?;
    info!("Refreshed {} views that were due for refresh", refresh_results.len());
    
    for result in &refresh_results {
        info!("  Refreshed {}: success={}, duration={}ms", 
              result.view_name, result.success, result.duration_ms);
    }
    
    // Test 5: Get updated metadata after refresh
    info!("Test 5: Updated metadata after refresh");
    let updated_metadata = mv_service.get_view_metadata("mv_user_portfolio_summary").await;
    if let Some(metadata) = updated_metadata {
        info!("Updated mv_user_portfolio_summary metadata:");
        info!("  Row count: {}", metadata.row_count);
        info!("  Last refresh: {}", metadata.last_refresh);
        info!("  Refresh duration: {}ms", metadata.refresh_duration_ms);
        info!("  Last error: {:?}", metadata.last_error);
    }
    
    info!("✅ Materialized Views Service tests completed");
    Ok(())
}

async fn test_services_integration(pool: &PgPool) -> Result<(), AppError> {
    info!("🔗 Testing Services Integration");
    
    let perf_service = QueryPerformanceService::new(pool.clone(), 1000);
    let _mv_service = MaterializedViewsService::new(pool.clone());
    
    // Test 1: Query materialized views with performance monitoring
    info!("Test 1: Querying materialized views with performance monitoring");
    
    let mv_queries = vec![
        ("SELECT * FROM mv_user_position_summary LIMIT 10", "mv_query"),
        ("SELECT * FROM mv_mev_risk_summary LIMIT 20", "mv_time_query"),
        ("SELECT * FROM mv_query_performance_analytics LIMIT 15", "mv_analytics_query"),
    ];
    
    for (query, query_type) in mv_queries {
        let start_time = Instant::now();
        
        let (rows, analysis) = perf_service.execute_with_analysis(query, query_type).await?;
        
        info!("Materialized view query '{}' executed:", query_type);
        info!("  Duration: {}ms", start_time.elapsed().as_millis());
        info!("  Rows returned: {}", rows.len());
        info!("  Performance recommendations: {}", analysis.performance_recommendations.len());
        info!("  Index recommendations: {}", analysis.index_recommendations.len());
    }
    
    // Test 2: Performance comparison - regular tables vs materialized views
    info!("Test 2: Performance comparison");
    
    // Query regular tables (complex aggregation)
    let regular_query = r#"
        SELECT 
            p.user_address,
            COUNT(DISTINCT p.id) as total_positions,
            COUNT(DISTINCT p.protocol) as protocols_count,
            AVG(COALESCE(rm.overall_risk_score, 0)) as avg_risk_score
        FROM positions p
        LEFT JOIN risk_metrics rm ON p.id = rm.position_id
        WHERE p.created_at >= NOW() - INTERVAL '30 days'
        GROUP BY p.user_address
        LIMIT 10
    "#;
    
    let start_time = Instant::now();
    let (regular_rows, regular_analysis) = perf_service.execute_with_analysis(
        regular_query, "regular_aggregation"
    ).await?;
    let regular_duration = start_time.elapsed();
    
    // Query materialized view (equivalent data)
    let mv_query = "SELECT * FROM mv_user_position_summary LIMIT 10";
    let start_time = Instant::now();
    let (mv_rows, mv_analysis) = perf_service.execute_with_analysis(
        mv_query, "mv_aggregation"
    ).await?;
    let mv_duration = start_time.elapsed();
    
    info!("Performance Comparison Results:");
    info!("  Regular tables query:");
    info!("    Duration: {}ms", regular_duration.as_millis());
    info!("    Rows: {}", regular_rows.len());
    info!("    Seq scans: {}", regular_analysis.plan_analysis.seq_scans);
    info!("    Index scans: {}", regular_analysis.plan_analysis.index_scans);
    info!("  Materialized view query:");
    info!("    Duration: {}ms", mv_duration.as_millis());
    info!("    Rows: {}", mv_rows.len());
    info!("    Seq scans: {}", mv_analysis.plan_analysis.seq_scans);
    info!("    Index scans: {}", mv_analysis.plan_analysis.index_scans);
    
    let speedup = if mv_duration.as_millis() > 0 {
        regular_duration.as_millis() as f64 / mv_duration.as_millis() as f64
    } else {
        0.0
    };
    info!("  Performance improvement: {:.2}x faster with materialized view", speedup);
    
    // Test 3: Get comprehensive performance metrics
    info!("Test 3: Comprehensive performance metrics");
    let final_metrics = perf_service.get_performance_metrics().await;
    
    info!("Final Performance Summary:");
    info!("  Total queries executed: {}", final_metrics.total_queries);
    info!("  Average execution time: {:.2}ms", final_metrics.avg_query_time_ms);
    info!("  Slow queries detected: {}", final_metrics.slow_queries);
    info!("  Query types analyzed: {}", final_metrics.queries_by_type.len());
    info!("  Cache efficiency: {:.1}% hit rate", 
          if final_metrics.query_plan_cache_hits + final_metrics.query_plan_cache_misses > 0 {
              (final_metrics.query_plan_cache_hits as f64 / 
               (final_metrics.query_plan_cache_hits + final_metrics.query_plan_cache_misses) as f64) * 100.0
          } else {
              0.0
          });
    
    info!("✅ Services Integration tests completed");
    Ok(())
}

/// Helper function to demonstrate query plan analysis
async fn demonstrate_explain_analyze(pool: &PgPool) -> Result<(), AppError> {
    info!("🔍 Demonstrating EXPLAIN ANALYZE functionality");
    
    // This would normally be called internally by QueryPerformanceService
    let explain_query = r#"
        EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
        SELECT p.user_address, COUNT(*) as position_count
        FROM positions p
        WHERE p.is_active = true
        GROUP BY p.user_address
        ORDER BY position_count DESC
        LIMIT 5
    "#;
    
    let result = sqlx::query(explain_query).fetch_one(pool).await?;
    let plan_json: serde_json::Value = result.try_get(0)?;
    
    info!("EXPLAIN ANALYZE Result:");
    info!("{}", serde_json::to_string_pretty(&plan_json)?);
    
    Ok(())
}
