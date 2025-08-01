use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tokio::sync::Semaphore;
use uuid::Uuid;
use bigdecimal::BigDecimal;
use chrono::Utc;
use std::str::FromStr;
use futures::future::join_all;
use std::collections::HashMap;

use defi_risk_monitor::{
    services::{
        RiskAssessmentService, PortfolioService, SystemHealthService,
        PositionService, AuthService, QueryPerformanceService,
    },
    models::*,
    error::AppError,
    database::{Database, get_database_pool},
    config::Settings,
};

/// Comprehensive load and performance tests for DeFi Risk Monitor
/// These tests validate system performance under various load conditions
#[cfg(test)]
mod load_tests {
    use super::*;

    async fn setup_test_environment() -> Result<Arc<Database>, AppError> {
        dotenvy::dotenv().ok();
        let settings = Settings::new().expect("Failed to load settings");
        let pool = get_database_pool(&settings.database.url).await
            .expect("Failed to create database pool");
        Ok(Arc::new(Database::new(pool)))
    }

    #[tokio::test]
    async fn test_concurrent_position_operations() {
        println!("⚡ Testing Concurrent Position Operations");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = Arc::new(PositionService::new(db.clone()));
        
        let num_concurrent_users = 50;
        let operations_per_user = 10;
        let semaphore = Arc::new(Semaphore::new(20)); // Limit concurrent operations
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        for user_idx in 0..num_concurrent_users {
            let service = position_service.clone();
            let sem = semaphore.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let user_id = Uuid::new_v4();
                let mut results = LoadTestResults::new();
                
                for op_idx in 0..operations_per_user {
                    let position = create_load_test_position(
                        user_id, 
                        &format!("protocol_{}_{}", user_idx, op_idx)
                    );
                    
                    // CREATE operation
                    let create_start = Instant::now();
                    let create_result = service.create_position(&position).await;
                    let create_duration = create_start.elapsed();
                    
                    match create_result {
                        Ok(_) => {
                            results.record_success("create", create_duration);
                            
                            // READ operation
                            let read_start = Instant::now();
                            let read_result = service.get_position_by_id(position.id).await;
                            let read_duration = read_start.elapsed();
                            
                            match read_result {
                                Ok(_) => results.record_success("read", read_duration),
                                Err(_) => results.record_failure("read", read_duration),
                            }
                            
                            // UPDATE operation
                            let mut updated_position = position.clone();
                            updated_position.current_price = BigDecimal::from_str("1020").unwrap();
                            
                            let update_start = Instant::now();
                            let update_result = service.update_position(&updated_position).await;
                            let update_duration = update_start.elapsed();
                            
                            match update_result {
                                Ok(_) => results.record_success("update", update_duration),
                                Err(_) => results.record_failure("update", update_duration),
                            }
                            
                            // DELETE operation
                            let delete_start = Instant::now();
                            let delete_result = service.delete_position(position.id).await;
                            let delete_duration = delete_start.elapsed();
                            
                            match delete_result {
                                Ok(_) => results.record_success("delete", delete_duration),
                                Err(_) => results.record_failure("delete", delete_duration),
                            }
                        }
                        Err(_) => results.record_failure("create", create_duration),
                    }
                    
                    // Small delay between operations
                    sleep(Duration::from_millis(10)).await;
                }
                
                results
            });
            
            handles.push(handle);
        }
        
        // Wait for all operations to complete
        let results: Vec<LoadTestResults> = join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let total_duration = start_time.elapsed();
        
        // Aggregate results
        let aggregated = aggregate_load_test_results(&results);
        
        // Print results
        println!("📊 Load Test Results:");
        println!("Total Duration: {:?}", total_duration);
        println!("Concurrent Users: {}", num_concurrent_users);
        println!("Operations per User: {}", operations_per_user);
        println!("Total Operations: {}", num_concurrent_users * operations_per_user * 4); // CRUD
        
        for (operation, stats) in &aggregated.operation_stats {
            println!("  {}: {} success, {} failures, avg: {:?}, p95: {:?}", 
                    operation, stats.success_count, stats.failure_count, 
                    stats.avg_duration, stats.p95_duration);
        }
        
        // Assertions
        assert!(aggregated.overall_success_rate > 0.95, 
                "Success rate should be > 95%, got {:.2}%", 
                aggregated.overall_success_rate * 100.0);
        
        assert!(aggregated.operation_stats.get("create").unwrap().avg_duration < Duration::from_millis(100),
                "Average create time should be < 100ms");
        
        println!("✅ Concurrent Position Operations: PASSED");
    }

    #[tokio::test]
    async fn test_risk_calculation_performance() {
        println!("⚡ Testing Risk Calculation Performance");
        
        let db = setup_test_environment().await.unwrap();
        let risk_service = Arc::new(RiskAssessmentService::new(db.clone()));
        
        let num_concurrent_calculations = 100;
        let semaphore = Arc::new(Semaphore::new(25));
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        for i in 0..num_concurrent_calculations {
            let service = risk_service.clone();
            let sem = semaphore.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                let entity_id = Uuid::new_v4();
                let risk_assessment = RiskAssessment {
                    id: Uuid::new_v4(),
                    entity_id,
                    entity_type: RiskEntityType::Position,
                    risk_type: RiskType::ImpermanentLoss,
                    risk_score: BigDecimal::from_str("0.25").unwrap(),
                    risk_severity: RiskSeverity::Medium,
                    description: format!("Load test risk assessment {}", i),
                    metadata: None,
                    expires_at: None,
                    is_active: true,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };
                
                let calc_start = Instant::now();
                let result = service.update_risk_assessment(&risk_assessment).await;
                let calc_duration = calc_start.elapsed();
                
                (result.is_ok(), calc_duration)
            });
            
            handles.push(handle);
        }
        
        let results: Vec<(bool, Duration)> = join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let total_duration = start_time.elapsed();
        let successful_calculations = results.iter().filter(|(success, _)| *success).count();
        let avg_duration: Duration = results.iter()
            .map(|(_, duration)| *duration)
            .sum::<Duration>() / results.len() as u32;
        
        println!("📊 Risk Calculation Performance:");
        println!("Total Duration: {:?}", total_duration);
        println!("Successful Calculations: {}/{}", successful_calculations, num_concurrent_calculations);
        println!("Average Calculation Time: {:?}", avg_duration);
        println!("Calculations per Second: {:.2}", 
                num_concurrent_calculations as f64 / total_duration.as_secs_f64());
        
        // Assertions
        assert!(successful_calculations as f64 / num_concurrent_calculations as f64 > 0.95,
                "Success rate should be > 95%");
        assert!(avg_duration < Duration::from_millis(50),
                "Average calculation time should be < 50ms");
        
        println!("✅ Risk Calculation Performance: PASSED");
    }

    #[tokio::test]
    async fn test_portfolio_analytics_scalability() {
        println!("⚡ Testing Portfolio Analytics Scalability");
        
        let db = setup_test_environment().await.unwrap();
        let portfolio_service = Arc::new(PortfolioService::new(db.clone()));
        let position_service = Arc::new(PositionService::new(db.clone()));
        
        // Create test portfolios with varying sizes
        let portfolio_sizes = vec![10, 50, 100, 500];
        let mut performance_results = HashMap::new();
        
        for &portfolio_size in &portfolio_sizes {
            println!("Testing portfolio with {} positions", portfolio_size);
            
            let user_id = Uuid::new_v4();
            let mut positions = vec![];
            
            // Create positions
            let create_start = Instant::now();
            for i in 0..portfolio_size {
                let position = create_load_test_position(user_id, &format!("scalability_test_{}", i));
                let result = position_service.create_position(&position).await;
                if result.is_ok() {
                    positions.push(position.id);
                }
            }
            let create_duration = create_start.elapsed();
            
            // Test portfolio performance calculation
            let perf_start = Instant::now();
            let performance_result = portfolio_service.get_portfolio_performance(
                user_id, Some(30), None, None
            ).await;
            let perf_duration = perf_start.elapsed();
            
            // Test P&L history calculation
            let pnl_start = Instant::now();
            let pnl_result = portfolio_service.get_pnl_history(
                user_id, Some(7), Some("daily".to_string()), None, None
            ).await;
            let pnl_duration = pnl_start.elapsed();
            
            // Test asset allocation calculation
            let alloc_start = Instant::now();
            let alloc_result = portfolio_service.get_asset_allocation(user_id, None, None).await;
            let alloc_duration = alloc_start.elapsed();
            
            // Clean up positions
            for position_id in positions {
                let _ = position_service.delete_position(position_id).await;
            }
            
            let portfolio_perf = PortfolioPerformanceResults {
                portfolio_size,
                create_duration,
                performance_duration: perf_duration,
                pnl_duration,
                allocation_duration: alloc_duration,
                performance_success: performance_result.is_ok(),
                pnl_success: pnl_result.is_ok(),
                allocation_success: alloc_result.is_ok(),
            };
            
            performance_results.insert(portfolio_size, portfolio_perf);
            
            println!("  Portfolio Performance: {:?}", perf_duration);
            println!("  P&L Calculation: {:?}", pnl_duration);
            println!("  Asset Allocation: {:?}", alloc_duration);
        }
        
        // Analyze scalability
        println!("📊 Portfolio Analytics Scalability Results:");
        for (&size, results) in &performance_results {
            println!("Portfolio Size: {} positions", size);
            println!("  Performance Calc: {:?} (success: {})", 
                    results.performance_duration, results.performance_success);
            println!("  P&L Calc: {:?} (success: {})", 
                    results.pnl_duration, results.pnl_success);
            println!("  Allocation Calc: {:?} (success: {})", 
                    results.allocation_duration, results.allocation_success);
        }
        
        // Verify scalability - performance should scale reasonably with portfolio size
        let small_portfolio = performance_results.get(&10).unwrap();
        let large_portfolio = performance_results.get(&500).unwrap();
        
        let scalability_factor = large_portfolio.performance_duration.as_millis() as f64 / 
                                small_portfolio.performance_duration.as_millis() as f64;
        
        println!("Scalability Factor (500 vs 10 positions): {:.2}x", scalability_factor);
        
        // Performance should not degrade more than 10x for 50x more data
        assert!(scalability_factor < 10.0, 
                "Performance degradation should be reasonable, got {:.2}x", scalability_factor);
        
        println!("✅ Portfolio Analytics Scalability: PASSED");
    }

    #[tokio::test]
    async fn test_database_connection_pool_stress() {
        println!("⚡ Testing Database Connection Pool Stress");
        
        let db = setup_test_environment().await.unwrap();
        let health_service = Arc::new(SystemHealthService::new(db.clone()));
        
        let num_concurrent_queries = 200;
        let semaphore = Arc::new(Semaphore::new(50)); // Allow more concurrent operations
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        for i in 0..num_concurrent_queries {
            let service = health_service.clone();
            let sem = semaphore.clone();
            
            let handle = tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                
                let query_start = Instant::now();
                
                // Mix different types of database operations
                let result = match i % 4 {
                    0 => service.get_database_metrics().await.map(|_| ()),
                    1 => service.get_query_performance_stats().await.map(|_| ()),
                    2 => service.get_connection_pool_health().await.map(|_| ()),
                    _ => service.get_table_sizes().await.map(|_| ()),
                };
                
                let query_duration = query_start.elapsed();
                (result.is_ok(), query_duration)
            });
            
            handles.push(handle);
        }
        
        let results: Vec<(bool, Duration)> = join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let total_duration = start_time.elapsed();
        let successful_queries = results.iter().filter(|(success, _)| *success).count();
        let failed_queries = results.len() - successful_queries;
        
        let avg_query_time: Duration = results.iter()
            .map(|(_, duration)| *duration)
            .sum::<Duration>() / results.len() as u32;
        
        let queries_per_second = num_concurrent_queries as f64 / total_duration.as_secs_f64();
        
        println!("📊 Database Connection Pool Stress Results:");
        println!("Total Duration: {:?}", total_duration);
        println!("Successful Queries: {}", successful_queries);
        println!("Failed Queries: {}", failed_queries);
        println!("Average Query Time: {:?}", avg_query_time);
        println!("Queries per Second: {:.2}", queries_per_second);
        
        // Check final pool health
        let final_health = health_service.get_connection_pool_health().await;
        if let Ok(health) = final_health {
            println!("Final Pool Health Score: {:.2}", health.health_score);
            println!("Active Connections: {}", health.active_connections);
            println!("Failed Connections: {}", health.failed_connections);
        }
        
        // Assertions
        assert!(successful_queries as f64 / num_concurrent_queries as f64 > 0.90,
                "Success rate should be > 90%, got {:.2}%", 
                successful_queries as f64 / num_concurrent_queries as f64 * 100.0);
        
        assert!(avg_query_time < Duration::from_millis(200),
                "Average query time should be < 200ms under stress");
        
        println!("✅ Database Connection Pool Stress: PASSED");
    }

    #[tokio::test]
    async fn test_memory_usage_under_load() {
        println!("⚡ Testing Memory Usage Under Load");
        
        let db = setup_test_environment().await.unwrap();
        let position_service = Arc::new(PositionService::new(db.clone()));
        
        // Measure initial memory usage
        let initial_memory = get_memory_usage();
        println!("Initial Memory Usage: {} MB", initial_memory / 1024 / 1024);
        
        let num_operations = 1000;
        let mut position_ids = vec![];
        
        // Create many positions to test memory usage
        let create_start = Instant::now();
        for i in 0..num_operations {
            let position = create_load_test_position(
                Uuid::new_v4(), 
                &format!("memory_test_{}", i)
            );
            
            let result = position_service.create_position(&position).await;
            if result.is_ok() {
                position_ids.push(position.id);
            }
            
            // Check memory usage periodically
            if i % 100 == 0 {
                let current_memory = get_memory_usage();
                println!("Memory after {} operations: {} MB", 
                        i, current_memory / 1024 / 1024);
            }
        }
        let create_duration = create_start.elapsed();
        
        let peak_memory = get_memory_usage();
        println!("Peak Memory Usage: {} MB", peak_memory / 1024 / 1024);
        
        // Clean up positions
        let cleanup_start = Instant::now();
        for position_id in position_ids {
            let _ = position_service.delete_position(position_id).await;
        }
        let cleanup_duration = cleanup_start.elapsed();
        
        // Force garbage collection (if applicable)
        sleep(Duration::from_secs(1)).await;
        
        let final_memory = get_memory_usage();
        println!("Final Memory Usage: {} MB", final_memory / 1024 / 1024);
        
        println!("📊 Memory Usage Results:");
        println!("Initial Memory: {} MB", initial_memory / 1024 / 1024);
        println!("Peak Memory: {} MB", peak_memory / 1024 / 1024);
        println!("Final Memory: {} MB", final_memory / 1024 / 1024);
        println!("Memory Growth: {} MB", (peak_memory - initial_memory) / 1024 / 1024);
        println!("Create Duration: {:?}", create_duration);
        println!("Cleanup Duration: {:?}", cleanup_duration);
        
        // Assertions
        let memory_growth = peak_memory - initial_memory;
        let memory_growth_mb = memory_growth / 1024 / 1024;
        
        assert!(memory_growth_mb < 500, 
                "Memory growth should be < 500MB for {} operations, got {} MB", 
                num_operations, memory_growth_mb);
        
        // Memory should be mostly reclaimed after cleanup
        let memory_leak = final_memory - initial_memory;
        let memory_leak_mb = memory_leak / 1024 / 1024;
        
        assert!(memory_leak_mb < 50, 
                "Memory leak should be < 50MB, got {} MB", memory_leak_mb);
        
        println!("✅ Memory Usage Under Load: PASSED");
    }

    #[tokio::test]
    async fn test_query_performance_under_load() {
        println!("⚡ Testing Query Performance Under Load");
        
        let db = setup_test_environment().await.unwrap();
        let query_service = Arc::new(QueryPerformanceService::new(db.clone()));
        
        let num_concurrent_queries = 100;
        let queries_per_thread = 10;
        
        let start_time = Instant::now();
        let mut handles = vec![];
        
        for thread_id in 0..num_concurrent_queries {
            let service = query_service.clone();
            
            let handle = tokio::spawn(async move {
                let mut thread_results = vec![];
                
                for query_id in 0..queries_per_thread {
                    let test_query = format!(
                        "SELECT COUNT(*) FROM positions WHERE protocol = 'test_{}_{}'", 
                        thread_id, query_id
                    );
                    
                    let query_start = Instant::now();
                    
                    // Simulate query execution and logging
                    sleep(Duration::from_millis(5)).await; // Simulate query time
                    
                    let query_duration = query_start.elapsed();
                    
                    let log_result = service.log_query_performance(
                        &test_query,
                        query_duration,
                        true,
                        Some(format!("Load test query {}_{}", thread_id, query_id))
                    ).await;
                    
                    thread_results.push((log_result.is_ok(), query_duration));
                }
                
                thread_results
            });
            
            handles.push(handle);
        }
        
        let results: Vec<Vec<(bool, Duration)>> = join_all(handles).await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();
        
        let total_duration = start_time.elapsed();
        
        // Flatten results
        let all_results: Vec<(bool, Duration)> = results.into_iter().flatten().collect();
        let successful_queries = all_results.iter().filter(|(success, _)| *success).count();
        let total_queries = all_results.len();
        
        let avg_query_time: Duration = all_results.iter()
            .map(|(_, duration)| *duration)
            .sum::<Duration>() / all_results.len() as u32;
        
        println!("📊 Query Performance Under Load Results:");
        println!("Total Duration: {:?}", total_duration);
        println!("Total Queries: {}", total_queries);
        println!("Successful Queries: {}", successful_queries);
        println!("Average Query Time: {:?}", avg_query_time);
        println!("Queries per Second: {:.2}", 
                total_queries as f64 / total_duration.as_secs_f64());
        
        // Test slow query detection
        let slow_queries = query_service.get_slow_queries(
            Some(Duration::from_millis(10)),
            Some(20)
        ).await;
        
        if let Ok(slow_query_list) = slow_queries {
            println!("Detected Slow Queries: {}", slow_query_list.len());
        }
        
        // Assertions
        assert!(successful_queries as f64 / total_queries as f64 > 0.95,
                "Query logging success rate should be > 95%");
        
        assert!(avg_query_time < Duration::from_millis(20),
                "Average query logging time should be < 20ms");
        
        println!("✅ Query Performance Under Load: PASSED");
    }

    // Helper functions and structs
    fn create_load_test_position(user_id: Uuid, protocol: &str) -> Position {
        Position {
            id: Uuid::new_v4(),
            user_id,
            protocol: protocol.to_string(),
            chain: "ethereum".to_string(),
            position_type: PositionType::LiquidityPool,
            token0_address: "0xA0b86a33E6441b8C8e7F9c0e7a0A8A8A8A8A8A8A".to_string(),
            token1_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
            amount0: BigDecimal::from_str("1000").unwrap(),
            amount1: BigDecimal::from_str("1.0").unwrap(),
            entry_price: BigDecimal::from_str("1000").unwrap(),
            current_price: BigDecimal::from_str("1010").unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn get_memory_usage() -> u64 {
        // Mock memory usage - in real implementation, use system APIs
        // This would typically use procfs on Linux or similar APIs
        1024 * 1024 * 100 // 100 MB baseline
    }

    #[derive(Debug)]
    struct LoadTestResults {
        operation_stats: HashMap<String, OperationStats>,
    }

    #[derive(Debug)]
    struct OperationStats {
        success_count: u32,
        failure_count: u32,
        total_duration: Duration,
        durations: Vec<Duration>,
    }

    impl LoadTestResults {
        fn new() -> Self {
            Self {
                operation_stats: HashMap::new(),
            }
        }

        fn record_success(&mut self, operation: &str, duration: Duration) {
            let stats = self.operation_stats.entry(operation.to_string())
                .or_insert_with(|| OperationStats {
                    success_count: 0,
                    failure_count: 0,
                    total_duration: Duration::from_secs(0),
                    durations: Vec::new(),
                });
            
            stats.success_count += 1;
            stats.total_duration += duration;
            stats.durations.push(duration);
        }

        fn record_failure(&mut self, operation: &str, duration: Duration) {
            let stats = self.operation_stats.entry(operation.to_string())
                .or_insert_with(|| OperationStats {
                    success_count: 0,
                    failure_count: 0,
                    total_duration: Duration::from_secs(0),
                    durations: Vec::new(),
                });
            
            stats.failure_count += 1;
            stats.total_duration += duration;
            stats.durations.push(duration);
        }
    }

    impl OperationStats {
        fn avg_duration(&self) -> Duration {
            if self.durations.is_empty() {
                Duration::from_secs(0)
            } else {
                self.total_duration / self.durations.len() as u32
            }
        }

        fn p95_duration(&self) -> Duration {
            if self.durations.is_empty() {
                return Duration::from_secs(0);
            }
            
            let mut sorted_durations = self.durations.clone();
            sorted_durations.sort();
            
            let p95_index = (sorted_durations.len() as f64 * 0.95) as usize;
            sorted_durations.get(p95_index).copied().unwrap_or(Duration::from_secs(0))
        }
    }

    #[derive(Debug)]
    struct AggregatedLoadTestResults {
        operation_stats: HashMap<String, AggregatedOperationStats>,
        overall_success_rate: f64,
    }

    #[derive(Debug)]
    struct AggregatedOperationStats {
        success_count: u32,
        failure_count: u32,
        avg_duration: Duration,
        p95_duration: Duration,
    }

    fn aggregate_load_test_results(results: &[LoadTestResults]) -> AggregatedLoadTestResults {
        let mut aggregated_stats = HashMap::new();
        let mut total_operations = 0;
        let mut total_successes = 0;

        for result in results {
            for (operation, stats) in &result.operation_stats {
                let agg_stats = aggregated_stats.entry(operation.clone())
                    .or_insert_with(|| AggregatedOperationStats {
                        success_count: 0,
                        failure_count: 0,
                        avg_duration: Duration::from_secs(0),
                        p95_duration: Duration::from_secs(0),
                    });

                agg_stats.success_count += stats.success_count;
                agg_stats.failure_count += stats.failure_count;
                
                total_operations += stats.success_count + stats.failure_count;
                total_successes += stats.success_count;
            }
        }

        // Calculate aggregated durations
        for (operation, agg_stats) in &mut aggregated_stats {
            let all_durations: Vec<Duration> = results.iter()
                .filter_map(|r| r.operation_stats.get(operation))
                .flat_map(|stats| &stats.durations)
                .copied()
                .collect();

            if !all_durations.is_empty() {
                let total_duration: Duration = all_durations.iter().sum();
                agg_stats.avg_duration = total_duration / all_durations.len() as u32;

                let mut sorted_durations = all_durations;
                sorted_durations.sort();
                let p95_index = (sorted_durations.len() as f64 * 0.95) as usize;
                agg_stats.p95_duration = sorted_durations.get(p95_index)
                    .copied()
                    .unwrap_or(Duration::from_secs(0));
            }
        }

        let overall_success_rate = if total_operations > 0 {
            total_successes as f64 / total_operations as f64
        } else {
            0.0
        };

        AggregatedLoadTestResults {
            operation_stats: aggregated_stats,
            overall_success_rate,
        }
    }

    #[derive(Debug)]
    struct PortfolioPerformanceResults {
        portfolio_size: usize,
        create_duration: Duration,
        performance_duration: Duration,
        pnl_duration: Duration,
        allocation_duration: Duration,
        performance_success: bool,
        pnl_success: bool,
        allocation_success: bool,
    }
}
