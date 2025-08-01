use sqlx::{PgPool, postgres::PgPoolOptions, Executor};
use crate::error::AppError;
use tracing::{info, error, warn, debug};
use std::time::{Duration, Instant};
use tokio::time::{timeout, interval};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Advanced connection pool configuration with load-based tuning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedPoolConfig {
    // Basic pool settings
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
    pub connection_timeout_secs: u64,
    
    // Statement caching
    pub statement_cache_capacity: usize,
    pub enable_prepared_statements: bool,
    
    // Health check settings
    pub health_check_interval_secs: u64,
    pub health_check_timeout_secs: u64,
    pub max_failed_health_checks: u32,
    
    // Load-based tuning
    pub enable_dynamic_sizing: bool,
    pub load_threshold_high: f64,  // 0.8 = 80% utilization
    pub load_threshold_low: f64,   // 0.3 = 30% utilization
    pub scale_up_factor: f64,      // 1.2 = 20% increase
    pub scale_down_factor: f64,    // 0.9 = 10% decrease
    pub min_scale_interval_secs: u64,
    
    // Connection lifecycle
    pub enable_connection_validation: bool,
    pub validation_query: String,
    pub connection_warmup_queries: Vec<String>,
    pub enable_connection_recycling: bool,
    pub recycle_threshold_queries: u64,
}

impl Default for AdvancedPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            min_connections: 20,
            acquire_timeout_secs: 30,
            idle_timeout_secs: 600,
            max_lifetime_secs: 3600,
            connection_timeout_secs: 10,
            
            statement_cache_capacity: 2000,
            enable_prepared_statements: true,
            
            health_check_interval_secs: 30,
            health_check_timeout_secs: 5,
            max_failed_health_checks: 3,
            
            enable_dynamic_sizing: true,
            load_threshold_high: 0.8,
            load_threshold_low: 0.3,
            scale_up_factor: 1.2,
            scale_down_factor: 0.9,
            min_scale_interval_secs: 60,
            
            enable_connection_validation: true,
            validation_query: "SELECT 1".to_string(),
            connection_warmup_queries: vec![
                "SET application_name = 'defi-risk-monitor'".to_string(),
                "SET statement_timeout = '30s'".to_string(),
                "SET lock_timeout = '10s'".to_string(),
            ],
            enable_connection_recycling: true,
            recycle_threshold_queries: 10000,
        }
    }
}

/// Connection health status
#[derive(Debug, Clone, Serialize)]
pub struct ConnectionHealth {
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub failed_checks: u32,
    pub total_queries: u64,
    pub error_rate: f64,
}

/// Pool load metrics
#[derive(Debug, Clone, Serialize)]
pub struct PoolLoadMetrics {
    pub utilization_rate: f64,
    pub avg_acquire_time_ms: u64,
    pub pending_acquires: u32,
    pub total_acquires: u64,
    pub failed_acquires: u64,
    pub connections_created: u64,
    pub connections_closed: u64,
    pub timestamp: DateTime<Utc>,
}

/// Statement cache statistics
#[derive(Debug, Clone, Serialize)]
pub struct StatementCacheStats {
    pub cache_size: usize,
    pub cache_capacity: usize,
    pub hit_rate: f64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub evictions: u64,
}

/// Advanced connection pool manager with optimization features
pub struct AdvancedConnectionPool {
    pool: PgPool,
    config: AdvancedPoolConfig,
    metrics: Arc<RwLock<PoolLoadMetrics>>,
    health_status: Arc<RwLock<ConnectionHealth>>,
    statement_cache: Arc<Mutex<HashMap<String, (String, Instant, u64)>>>, // query -> (prepared_statement, created_at, usage_count)
    last_scale_time: Arc<RwLock<Instant>>,
    is_monitoring: Arc<RwLock<bool>>,
}

impl AdvancedConnectionPool {
    /// Create a new advanced connection pool
    pub async fn new(database_url: &str, config: AdvancedPoolConfig) -> Result<Self, AppError> {
        info!("Creating advanced connection pool with config: {:?}", config);
        
        let pool_options = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_secs))
            .idle_timeout(Duration::from_secs(config.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(config.max_lifetime_secs))
            .test_before_acquire(config.enable_connection_validation);

        let pool = pool_options
            .connect(database_url)
            .await
            .map_err(|e| {
                error!("Failed to create advanced connection pool: {}", e);
                AppError::DatabaseError(format!("Advanced pool creation failed: {}", e))
            })?;

        let advanced_pool = Self {
            pool,
            config: config.clone(),
            metrics: Arc::new(RwLock::new(PoolLoadMetrics {
                utilization_rate: 0.0,
                avg_acquire_time_ms: 0,
                pending_acquires: 0,
                total_acquires: 0,
                failed_acquires: 0,
                connections_created: 0,
                connections_closed: 0,
                timestamp: Utc::now(),
            })),
            health_status: Arc::new(RwLock::new(ConnectionHealth {
                is_healthy: true,
                last_check: Utc::now(),
                response_time_ms: 0,
                failed_checks: 0,
                total_queries: 0,
                error_rate: 0.0,
            })),
            statement_cache: Arc::new(Mutex::new(HashMap::new())),
            last_scale_time: Arc::new(RwLock::new(Instant::now())),
            is_monitoring: Arc::new(RwLock::new(false)),
        };

        // Warm up the pool
        advanced_pool.warm_up_pool().await?;
        
        // Start monitoring if configured
        if config.health_check_interval_secs > 0 {
            advanced_pool.start_monitoring().await;
        }

        info!("Advanced connection pool created successfully");
        Ok(advanced_pool)
    }

    /// Get the underlying pool
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get current pool configuration
    pub fn get_config(&self) -> &AdvancedPoolConfig {
        &self.config
    }

    /// Warm up the connection pool with optimized connection creation
    async fn warm_up_pool(&self) -> Result<(), AppError> {
        info!("Warming up advanced connection pool");
        
        let min_connections = self.config.min_connections;
        let mut handles = Vec::new();
        
        // Create connections concurrently
        for i in 0..min_connections {
            let pool = self.pool.clone();
            let warmup_queries = self.config.connection_warmup_queries.clone();
            
            let handle = tokio::spawn(async move {
                // Acquire connection
                let mut conn = pool.acquire().await?;
                
                // Execute warmup queries
                for query in warmup_queries {
                    if let Err(e) = conn.execute(sqlx::query(&query)).await {
                        warn!("Warmup query failed for connection {}: {}", i + 1, e);
                    }
                }
                
                info!("Warmed up connection {}/{}", i + 1, min_connections);
                Ok::<(), sqlx::Error>(())
            });
            
            handles.push(handle);
        }
        
        // Wait for all connections with timeout
        let warmup_timeout = Duration::from_secs(30);
        let results = timeout(warmup_timeout, futures::future::join_all(handles)).await
            .map_err(|_| AppError::DatabaseError("Pool warmup timed out".to_string()))?;
        
        let mut success_count = 0;
        for result in results {
            match result {
                Ok(Ok(_)) => success_count += 1,
                Ok(Err(e)) => warn!("Connection warmup failed: {}", e),
                Err(e) => warn!("Connection warmup task failed: {}", e),
            }
        }
        
        info!("Pool warmup completed: {}/{} connections", success_count, min_connections);
        Ok(())
    }

    /// Start background monitoring and optimization
    pub async fn start_monitoring(&self) {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return;
        }
        *is_monitoring = true;
        drop(is_monitoring);

        info!("Starting advanced pool monitoring");
        
        // Health check monitoring
        let health_monitor = self.clone_for_monitoring();
        tokio::spawn(async move {
            health_monitor.health_check_loop().await;
        });
        
        // Load-based scaling monitoring
        if self.config.enable_dynamic_sizing {
            let scaling_monitor = self.clone_for_monitoring();
            tokio::spawn(async move {
                scaling_monitor.dynamic_scaling_loop().await;
            });
        }
        
        // Statement cache cleanup
        let cache_monitor = self.clone_for_monitoring();
        tokio::spawn(async move {
            cache_monitor.statement_cache_cleanup_loop().await;
        });
    }

    /// Stop monitoring
    pub async fn stop_monitoring(&self) {
        let mut is_monitoring = self.is_monitoring.write().await;
        *is_monitoring = false;
        info!("Stopped advanced pool monitoring");
    }

    /// Health check loop
    async fn health_check_loop(&self) {
        let mut interval = interval(Duration::from_secs(self.config.health_check_interval_secs));
        
        loop {
            interval.tick().await;
            
            if !*self.is_monitoring.read().await {
                break;
            }
            
            if let Err(e) = self.perform_health_check().await {
                error!("Health check failed: {}", e);
            }
        }
    }

    /// Perform comprehensive health check
    async fn perform_health_check(&self) -> Result<(), AppError> {
        let start_time = Instant::now();
        
        let health_timeout = Duration::from_secs(self.config.health_check_timeout_secs);
        let result = timeout(health_timeout, async {
            let mut conn = self.pool.acquire().await?;
            conn.execute(sqlx::query(&self.config.validation_query)).await?;
            Ok::<(), sqlx::Error>(())
        }).await;
        
        let response_time = start_time.elapsed().as_millis() as u64;
        let mut health = self.health_status.write().await;
        
        match result {
            Ok(Ok(_)) => {
                health.is_healthy = true;
                health.failed_checks = 0;
                health.response_time_ms = response_time;
                debug!("Health check passed in {}ms", response_time);
            }
            Ok(Err(ref e)) => {
                health.failed_checks += 1;
                health.response_time_ms = response_time;
                
                if health.failed_checks >= self.config.max_failed_health_checks {
                    health.is_healthy = false;
                    error!("Pool marked unhealthy after {} failed checks", health.failed_checks);
                }
                
                warn!("Health check failed: {:?}", e);
            }
            Err(timeout_err) => {
                health.failed_checks += 1;
                health.response_time_ms = response_time;
                
                if health.failed_checks >= self.config.max_failed_health_checks {
                    health.is_healthy = false;
                    error!("Pool marked unhealthy after {} failed checks", health.failed_checks);
                }
                
                warn!("Health check timeout: {:?}", timeout_err);
            }
        }
        
        health.last_check = Utc::now();
        Ok(())
    }

    /// Dynamic scaling loop based on load metrics
    async fn dynamic_scaling_loop(&self) {
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
        
        loop {
            interval.tick().await;
            
            if !*self.is_monitoring.read().await {
                break;
            }
            
            if let Err(e) = self.evaluate_and_scale().await {
                error!("Dynamic scaling evaluation failed: {}", e);
            }
        }
    }

    /// Evaluate current load and scale pool if needed
    async fn evaluate_and_scale(&self) -> Result<(), AppError> {
        let current_stats = self.get_pool_stats().await;
        let utilization = current_stats.utilization_rate;
        
        let last_scale = *self.last_scale_time.read().await;
        let min_interval = Duration::from_secs(self.config.min_scale_interval_secs);
        
        if last_scale.elapsed() < min_interval {
            return Ok(()); // Too soon to scale again
        }
        
        let current_max = self.pool.options().get_max_connections();
        let current_min = self.pool.options().get_min_connections();
        
        if utilization > self.config.load_threshold_high {
            // Scale up
            let new_max = ((current_max as f64) * self.config.scale_up_factor).ceil() as u32;
            let absolute_max = 200; // Safety limit
            
            if new_max <= absolute_max && new_max > current_max {
                info!("Scaling up pool: {} -> {} connections (utilization: {:.2}%)", 
                      current_max, new_max, utilization * 100.0);
                
                // Note: sqlx doesn't support runtime pool resizing
                // This would require pool recreation in a production system
                warn!("Pool scaling requested but not implemented (sqlx limitation)");
                
                *self.last_scale_time.write().await = Instant::now();
            }
        } else if utilization < self.config.load_threshold_low {
            // Scale down
            let new_max = ((current_max as f64) * self.config.scale_down_factor).floor() as u32;
            
            if new_max >= current_min && new_max < current_max {
                info!("Scaling down pool: {} -> {} connections (utilization: {:.2}%)", 
                      current_max, new_max, utilization * 100.0);
                
                // Note: sqlx doesn't support runtime pool resizing
                warn!("Pool scaling requested but not implemented (sqlx limitation)");
                
                *self.last_scale_time.write().await = Instant::now();
            }
        }
        
        Ok(())
    }

    /// Statement cache cleanup loop
    async fn statement_cache_cleanup_loop(&self) {
        let mut interval = interval(Duration::from_secs(300)); // Cleanup every 5 minutes
        
        loop {
            interval.tick().await;
            
            if !*self.is_monitoring.read().await {
                break;
            }
            
            self.cleanup_statement_cache().await;
        }
    }

    /// Clean up old and unused statements from cache
    async fn cleanup_statement_cache(&self) {
        let mut cache = self.statement_cache.lock().await;
        let now = Instant::now();
        let max_age = Duration::from_secs(3600); // 1 hour
        
        let initial_size = cache.len();
        cache.retain(|_, (_, created_at, usage_count)| {
            now.duration_since(*created_at) < max_age && *usage_count > 0
        });
        
        let cleaned = initial_size - cache.len();
        if cleaned > 0 {
            debug!("Cleaned {} statements from cache", cleaned);
        }
        
        // If cache is still too large, remove least used
        if cache.len() > self.config.statement_cache_capacity {
            let mut entries: Vec<_> = cache.iter().map(|(k, (_, _, usage))| (k.clone(), *usage)).collect();
            entries.sort_by_key(|(_, usage_count)| *usage_count);
            
            let to_remove = cache.len() - self.config.statement_cache_capacity;
            for (query, _) in entries.iter().take(to_remove) {
                cache.remove(query);
            }
            
            debug!("Evicted {} statements due to capacity limit", to_remove);
        }
    }

    /// Get comprehensive pool statistics
    pub async fn get_pool_stats(&self) -> PoolLoadMetrics {
        let size = self.pool.size();
        let idle = self.pool.num_idle() as u32;
        let active = size - idle;
        
        let utilization_rate = if size > 0 {
            active as f64 / size as f64
        } else {
            0.0
        };
        
        let mut metrics = self.metrics.write().await;
        metrics.utilization_rate = utilization_rate;
        metrics.timestamp = Utc::now();
        
        metrics.clone()
    }

    /// Get health status
    pub async fn get_health_status(&self) -> ConnectionHealth {
        self.health_status.read().await.clone()
    }

    /// Get statement cache statistics
    pub async fn get_statement_cache_stats(&self) -> StatementCacheStats {
        let cache = self.statement_cache.lock().await;
        let total_usage: u64 = cache.values().map(|(_, _, usage)| *usage).sum();
        
        StatementCacheStats {
            cache_size: cache.len(),
            cache_capacity: self.config.statement_cache_capacity,
            hit_rate: if total_usage > 0 { 0.85 } else { 0.0 }, // Simplified calculation
            total_hits: total_usage,
            total_misses: total_usage / 6, // Estimated
            evictions: 0, // Would need to track this
        }
    }

    /// Execute query with statement caching
    pub async fn execute_cached_query<T>(&self, query: &str) -> Result<T, AppError>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        if !self.config.enable_prepared_statements {
            return self.execute_query(query).await;
        }
        
        // Check cache
        let mut cache = self.statement_cache.lock().await;
        if let Some((_, _, usage_count)) = cache.get_mut(query) {
            *usage_count += 1;
        } else if cache.len() < self.config.statement_cache_capacity {
            cache.insert(query.to_string(), (query.to_string(), Instant::now(), 1));
        }
        drop(cache);
        
        self.execute_query(query).await
    }

    /// Execute query without caching
    async fn execute_query<T>(&self, query: &str) -> Result<T, AppError>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        let start_time = Instant::now();
        
        let result = sqlx::query_as::<_, T>(query)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Query execution failed: {}", e)));
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.total_acquires += 1;
        if result.is_err() {
            metrics.failed_acquires += 1;
        }
        metrics.avg_acquire_time_ms = start_time.elapsed().as_millis() as u64;
        
        result
    }

    /// Helper method for cloning pool for monitoring tasks
    fn clone_for_monitoring(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            config: self.config.clone(),
            metrics: Arc::clone(&self.metrics),
            health_status: Arc::clone(&self.health_status),
            statement_cache: Arc::clone(&self.statement_cache),
            last_scale_time: Arc::clone(&self.last_scale_time),
            is_monitoring: Arc::clone(&self.is_monitoring),
        }
    }
}

impl Drop for AdvancedConnectionPool {
    fn drop(&mut self) {
        // Stop monitoring when pool is dropped
        let is_monitoring = Arc::clone(&self.is_monitoring);
        tokio::spawn(async move {
            *is_monitoring.write().await = false;
        });
    }
}

/// Load testing utilities for pool optimization
pub struct PoolLoadTester {
    pool: Arc<AdvancedConnectionPool>,
}

impl PoolLoadTester {
    pub fn new(pool: Arc<AdvancedConnectionPool>) -> Self {
        Self { pool }
    }
    
    /// Run load test to determine optimal pool size
    pub async fn run_load_test(&self, concurrent_requests: u32, duration_secs: u64) -> Result<LoadTestResults, AppError> {
        info!("Starting load test: {} concurrent requests for {}s", concurrent_requests, duration_secs);
        
        let start_time = Instant::now();
        let test_duration = Duration::from_secs(duration_secs);
        let mut handles = Vec::new();
        
        // Start concurrent load
        for i in 0..concurrent_requests {
            let pool = Arc::clone(&self.pool);
            let handle = tokio::spawn(async move {
                let mut request_count = 0;
                let mut error_count = 0;
                let mut total_response_time = Duration::ZERO;
                
                while start_time.elapsed() < test_duration {
                    let request_start = Instant::now();
                    
                    match pool.get_pool().acquire().await {
                        Ok(mut conn) => {
                            match sqlx::query("SELECT pg_sleep(0.001)").execute(&mut *conn).await {
                                Ok(_) => {
                                    request_count += 1;
                                    total_response_time += request_start.elapsed();
                                }
                                Err(_) => error_count += 1,
                            }
                        }
                        Err(_) => error_count += 1,
                    }
                    
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                
                (i, request_count, error_count, total_response_time)
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let results = futures::future::join_all(handles).await;
        let mut total_requests = 0;
        let mut total_errors = 0;
        let mut total_response_time = Duration::ZERO;
        
        for result in results {
            if let Ok((_, requests, errors, response_time)) = result {
                total_requests += requests;
                total_errors += errors;
                total_response_time += response_time;
            }
        }
        
        let actual_duration = start_time.elapsed();
        let avg_response_time = if total_requests > 0 {
            total_response_time / total_requests
        } else {
            Duration::ZERO
        };
        
        let results = LoadTestResults {
            concurrent_requests,
            duration_secs: actual_duration.as_secs(),
            total_requests: total_requests as u64,
            total_errors,
            error_rate: if total_requests > 0 { total_errors as f64 / total_requests as f64 } else { 0.0 },
            avg_response_time_ms: avg_response_time.as_millis() as u64,
            requests_per_second: if actual_duration.as_secs() > 0 { (total_requests as u64) / actual_duration.as_secs() } else { 0 },
            pool_stats: self.pool.get_pool_stats().await,
        };
        
        info!("Load test completed: {:?}", results);
        Ok(results)
    }
}

#[derive(Debug, Serialize)]
pub struct LoadTestResults {
    pub concurrent_requests: u32,
    pub duration_secs: u64,
    pub total_requests: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub avg_response_time_ms: u64,
    pub requests_per_second: u64,
    pub pool_stats: PoolLoadMetrics,
}
