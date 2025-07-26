use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;
use tracing::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use crate::error::AppError;
use crate::utils::fault_tolerance::{FaultTolerantService, RetryConfig};

/// Database node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseNode {
    pub id: String,
    pub url: String,
    pub role: DatabaseRole,
    pub priority: u8,        // Higher priority = preferred for reads
    pub max_connections: u32,
    pub health_check_interval: Duration,
    pub is_active: bool,
}

/// Database role types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DatabaseRole {
    Primary,    // Read/Write master
    Replica,    // Read-only replica
    Standby,    // Hot standby for failover
}

/// Database health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub node_id: String,
    pub is_healthy: bool,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: u64,
    pub connection_count: u32,
    pub replication_lag_ms: Option<u64>,
    pub error_message: Option<String>,
}

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub health_check_interval: Duration,
    pub failure_threshold: u32,        // Number of consecutive failures before failover
    pub recovery_threshold: u32,       // Number of consecutive successes before recovery
    pub max_replication_lag_ms: u64,   // Maximum acceptable replication lag
    pub failover_timeout: Duration,     // Maximum time for failover process
    pub auto_failback: bool,           // Automatically failback to primary when recovered
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            health_check_interval: Duration::from_secs(5),
            failure_threshold: 3,
            recovery_threshold: 5,
            max_replication_lag_ms: 1000, // 1 second max lag
            failover_timeout: Duration::from_secs(30),
            auto_failback: true,
        }
    }
}

/// Database cluster state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterState {
    pub primary_node_id: String,
    pub active_replicas: Vec<String>,
    pub failed_nodes: Vec<String>,
    pub last_failover: Option<DateTime<Utc>>,
    pub failover_count: u64,
    pub cluster_health: ClusterHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterHealth {
    Healthy,        // All nodes operational
    Degraded,       // Some replicas down but primary healthy
    Critical,       // Primary down, running on standby
    Failed,         // All nodes down
}

/// Database replication and failover manager
pub struct DatabaseReplicationManager {
    nodes: HashMap<String, DatabaseNode>,
    pools: Arc<RwLock<HashMap<String, PgPool>>>,
    health_status: Arc<RwLock<HashMap<String, DatabaseHealth>>>,
    cluster_state: Arc<RwLock<ClusterState>>,
    config: FailoverConfig,
    #[allow(dead_code)]
    fault_tolerant_service: FaultTolerantService,
    failure_counts: Arc<RwLock<HashMap<String, u32>>>,
    recovery_counts: Arc<RwLock<HashMap<String, u32>>>,
}

impl DatabaseReplicationManager {
    pub async fn new(
        nodes: Vec<DatabaseNode>,
        config: FailoverConfig,
    ) -> Result<Self, AppError> {
        let mut node_map = HashMap::new();
        let mut pools = HashMap::new();
        let mut health_status = HashMap::new();
        let mut failure_counts = HashMap::new();
        let mut recovery_counts = HashMap::new();

        // Find primary node
        let primary_node = nodes.iter()
            .find(|n| n.role == DatabaseRole::Primary)
            .ok_or_else(|| AppError::ConfigError("No primary database node configured".to_string()))?;

        info!("Initializing database replication manager with {} nodes", nodes.len());

        // Initialize connections to all nodes
        for node in nodes.clone() {
            info!("Connecting to database node: {} ({})", node.id, node.url);
            
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(node.max_connections)
                .min_connections(1)
                .acquire_timeout(Duration::from_secs(10))
                .idle_timeout(Duration::from_secs(300))
                .max_lifetime(Duration::from_secs(1800))
                .connect(&node.url)
                .await
                .map_err(|e| AppError::DatabaseError(format!("Failed to connect to {}: {}", node.id, e)))?;

            pools.insert(node.id.clone(), pool);
            health_status.insert(node.id.clone(), DatabaseHealth {
                node_id: node.id.clone(),
                is_healthy: true,
                last_check: Utc::now(),
                response_time_ms: 0,
                connection_count: 0,
                replication_lag_ms: None,
                error_message: None,
            });
            failure_counts.insert(node.id.clone(), 0);
            recovery_counts.insert(node.id.clone(), 0);
            node_map.insert(node.id.clone(), node);
        }

        let cluster_state = ClusterState {
            primary_node_id: primary_node.id.clone(),
            active_replicas: node_map.values()
                .filter(|n| n.role == DatabaseRole::Replica && n.is_active)
                .map(|n| n.id.clone())
                .collect(),
            failed_nodes: Vec::new(),
            last_failover: None,
            failover_count: 0,
            cluster_health: ClusterHealth::Healthy,
        };

        let fault_tolerant_service = FaultTolerantService::new(
            "database_replication",
            RetryConfig::database(),
        );

        let manager = Self {
            nodes: node_map,
            pools: Arc::new(RwLock::new(pools)),
            health_status: Arc::new(RwLock::new(health_status)),
            cluster_state: Arc::new(RwLock::new(cluster_state)),
            config,
            fault_tolerant_service,
            failure_counts: Arc::new(RwLock::new(failure_counts)),
            recovery_counts: Arc::new(RwLock::new(recovery_counts)),
        };

        info!("Database replication manager initialized successfully");
        Ok(manager)
    }

    /// Get connection pool for read operations (load balanced across replicas)
    pub async fn get_read_pool(&self) -> Result<PgPool, AppError> {
        let cluster_state = self.cluster_state.read().await;
        let health_status = self.health_status.read().await;
        let pools = self.pools.read().await;

        // Find healthy replicas, sorted by priority
        let mut healthy_replicas: Vec<_> = cluster_state.active_replicas.iter()
            .filter(|node_id| {
                health_status.get(*node_id)
                    .map(|h| h.is_healthy)
                    .unwrap_or(false)
            })
            .filter_map(|node_id| self.nodes.get(node_id).map(|node| (node_id, node)))
            .collect();

        healthy_replicas.sort_by(|a, b| b.1.priority.cmp(&a.1.priority));

        // Return highest priority healthy replica
        if let Some((node_id, _)) = healthy_replicas.first() {
            if let Some(pool) = pools.get(*node_id) {
                debug!("Using replica {} for read operation", node_id);
                return Ok(pool.clone());
            }
        }

        // Fallback to primary if no healthy replicas
        let primary_pool = pools.get(&cluster_state.primary_node_id)
            .ok_or_else(|| AppError::DatabaseError("Primary database pool not available".to_string()))?;

        warn!("No healthy replicas available, using primary for read operation");
        Ok(primary_pool.clone())
    }

    /// Get connection pool for write operations (always primary)
    pub async fn get_write_pool(&self) -> Result<PgPool, AppError> {
        let cluster_state = self.cluster_state.read().await;
        let health_status = self.health_status.read().await;
        let pools = self.pools.read().await;

        // Check if primary is healthy
        if let Some(primary_health) = health_status.get(&cluster_state.primary_node_id) {
            if !primary_health.is_healthy {
                error!("Primary database is unhealthy, attempting failover");
                drop(cluster_state);
                drop(health_status);
                drop(pools);
                self.attempt_failover().await?;
                
                // Re-acquire locks after failover
                let cluster_state = self.cluster_state.read().await;
                let pools = self.pools.read().await;
                
                let primary_pool = pools.get(&cluster_state.primary_node_id)
                    .ok_or_else(|| AppError::DatabaseError("No healthy primary database available after failover".to_string()))?;
                
                return Ok(primary_pool.clone());
            }
        }

        let primary_pool = pools.get(&cluster_state.primary_node_id)
            .ok_or_else(|| AppError::DatabaseError("Primary database pool not available".to_string()))?;

        Ok(primary_pool.clone())
    }

    /// Start health monitoring background task
    pub async fn start_health_monitoring(&self) {
        let health_status = Arc::clone(&self.health_status);
        let cluster_state = Arc::clone(&self.cluster_state);
        let pools: Arc<RwLock<HashMap<String, PgPool>>> = Arc::clone(&self.pools);
        let failure_counts = Arc::clone(&self.failure_counts);
        let recovery_counts = Arc::clone(&self.recovery_counts);
        let nodes = self.nodes.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(config.health_check_interval);
            
            loop {
                interval.tick().await;
                
                let pools_guard = pools.read().await;
                let mut health_guard = health_status.write().await;
                let mut failure_guard = failure_counts.write().await;
                let mut recovery_guard = recovery_counts.write().await;
                
                for (node_id, node) in &nodes {
                    if let Some(pool) = pools_guard.get(node_id) {
                        let start_time = std::time::Instant::now();
                        
                        // Perform health check
                        let health_result = Self::check_node_health(pool, node).await;
                        let response_time = start_time.elapsed().as_millis() as u64;
                        
                        let is_healthy = health_result.is_ok();
                        let error_message = health_result.as_ref().err().map(|e| e.to_string());
                        
                        // Update health status
                        if let Some(health) = health_guard.get_mut(node_id) {
                            health.is_healthy = is_healthy;
                            health.last_check = Utc::now();
                            health.response_time_ms = response_time;
                            health.error_message = error_message;
                            
                            // Get replication lag for replicas
                            if node.role == DatabaseRole::Replica {
                                health.replication_lag_ms = Self::get_replication_lag(pool).await.ok();
                            }
                        }
                        
                        // Update failure/recovery counts
                        if is_healthy {
                            failure_guard.insert(node_id.clone(), 0);
                            *recovery_guard.entry(node_id.clone()).or_insert(0) += 1;
                        } else {
                            *failure_guard.entry(node_id.clone()).or_insert(0) += 1;
                            recovery_guard.insert(node_id.clone(), 0);
                        }
                        
                        // Log health status changes
                        if !is_healthy {
                            warn!("Database node {} is unhealthy: {:?}", node_id, health_result);
                        } else {
                            debug!("Database node {} is healthy ({}ms)", node_id, response_time);
                        }
                    }
                }
                
                drop(pools_guard);
                drop(health_guard);
                drop(failure_guard);
                drop(recovery_guard);
                
                // Check if failover is needed
                Self::evaluate_failover_conditions(&cluster_state, &failure_counts, &config).await;
            }
        });
        
        info!("Database health monitoring started");
    }

    /// Perform health check on a database node
    async fn check_node_health(pool: &PgPool, node: &DatabaseNode) -> Result<(), AppError> {
        // Simple connectivity check
        let row = sqlx::query("SELECT 1 as health_check")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Health check failed for {}: {}", node.id, e)))?;

        let result: i32 = row.get("health_check");
        if result != 1 {
            return Err(AppError::DatabaseError("Health check returned unexpected result".to_string()));
        }

        // Check connection count
        let conn_row = sqlx::query("SELECT count(*) as active_connections FROM pg_stat_activity WHERE state = 'active'")
            .fetch_one(pool)
            .await
            .map_err(|e| AppError::DatabaseError(format!("Failed to get connection count: {}", e)))?;

        let active_connections: i64 = conn_row.get("active_connections");
        if active_connections as u32 > node.max_connections * 90 / 100 {
            warn!("Database node {} has high connection usage: {}/{}", 
                  node.id, active_connections, node.max_connections);
        }

        Ok(())
    }

    /// Get replication lag for replica nodes
    async fn get_replication_lag(pool: &PgPool) -> Result<u64, AppError> {
        let row = sqlx::query(
            "SELECT EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp())) * 1000 as lag_ms"
        )
        .fetch_one(pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get replication lag: {}", e)))?;

        let lag_ms: Option<f64> = row.get("lag_ms");
        Ok(lag_ms.unwrap_or(0.0) as u64)
    }

    /// Evaluate if failover conditions are met
    async fn evaluate_failover_conditions(
        cluster_state: &Arc<RwLock<ClusterState>>,
        failure_counts: &Arc<RwLock<HashMap<String, u32>>>,
        config: &FailoverConfig,
    ) {
        let cluster_guard = cluster_state.read().await;
        let failure_guard = failure_counts.read().await;

        let primary_failures = failure_guard.get(&cluster_guard.primary_node_id).unwrap_or(&0);

        if *primary_failures >= config.failure_threshold {
            warn!("Primary database has {} consecutive failures, failover may be needed", primary_failures);
            // Failover logic would be triggered here
        }
    }

    /// Attempt database failover to a healthy standby
    async fn attempt_failover(&self) -> Result<(), AppError> {
        info!("Attempting database failover");
        let start_time = std::time::Instant::now();

        let mut cluster_state = self.cluster_state.write().await;
        let health_status = self.health_status.read().await;

        // Find healthy standby nodes
        let healthy_standbys: Vec<_> = self.nodes.values()
            .filter(|node| node.role == DatabaseRole::Standby)
            .filter(|node| {
                health_status.get(&node.id)
                    .map(|h| h.is_healthy)
                    .unwrap_or(false)
            })
            .collect();

        if healthy_standbys.is_empty() {
            error!("No healthy standby nodes available for failover");
            cluster_state.cluster_health = ClusterHealth::Failed;
            return Err(AppError::DatabaseError("No healthy standby nodes for failover".to_string()));
        }

        // Select best standby (highest priority, lowest replication lag)
        let best_standby = healthy_standbys.iter()
            .max_by_key(|node| node.priority)
            .unwrap();

        info!("Failing over to standby node: {}", best_standby.id);

        // Update cluster state
        let old_primary = cluster_state.primary_node_id.clone();
        cluster_state.primary_node_id = best_standby.id.clone();
        cluster_state.failed_nodes.push(old_primary);
        cluster_state.last_failover = Some(Utc::now());
        cluster_state.failover_count += 1;
        cluster_state.cluster_health = ClusterHealth::Critical;

        let failover_duration = start_time.elapsed();
        info!("Database failover completed in {:?} to node {}", failover_duration, best_standby.id);

        if failover_duration > self.config.failover_timeout {
            warn!("Failover took longer than expected: {:?}", failover_duration);
        }

        Ok(())
    }

    /// Get cluster status and statistics
    pub async fn get_cluster_status(&self) -> ClusterStatus {
        let cluster_state = self.cluster_state.read().await;
        let health_status = self.health_status.read().await;

        let node_statuses: HashMap<String, NodeStatus> = self.nodes.iter()
            .map(|(node_id, node)| {
                let health = health_status.get(node_id).cloned().unwrap_or_else(|| DatabaseHealth {
                    node_id: node_id.clone(),
                    is_healthy: false,
                    last_check: Utc::now(),
                    response_time_ms: 0,
                    connection_count: 0,
                    replication_lag_ms: None,
                    error_message: Some("No health data available".to_string()),
                });

                let status = NodeStatus {
                    node: node.clone(),
                    health,
                    is_primary: node_id == &cluster_state.primary_node_id,
                };

                (node_id.clone(), status)
            })
            .collect();

        ClusterStatus {
            cluster_state: cluster_state.clone(),
            node_statuses,
            total_nodes: self.nodes.len(),
            healthy_nodes: health_status.values().filter(|h| h.is_healthy).count(),
            failed_nodes: cluster_state.failed_nodes.len(),
        }
    }
}

/// Cluster status for monitoring and reporting
#[derive(Debug, Clone, Serialize)]
pub struct ClusterStatus {
    pub cluster_state: ClusterState,
    pub node_statuses: HashMap<String, NodeStatus>,
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub failed_nodes: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeStatus {
    pub node: DatabaseNode,
    pub health: DatabaseHealth,
    pub is_primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_config_defaults() {
        let config = FailoverConfig::default();
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.recovery_threshold, 5);
        assert!(config.auto_failback);
    }

    #[test]
    fn test_database_node_creation() {
        let node = DatabaseNode {
            id: "primary".to_string(),
            url: "postgresql://localhost/test".to_string(),
            role: DatabaseRole::Primary,
            priority: 100,
            max_connections: 20,
            health_check_interval: Duration::from_secs(5),
            is_active: true,
        };

        assert_eq!(node.role, DatabaseRole::Primary);
        assert_eq!(node.priority, 100);
        assert!(node.is_active);
    }

    #[test]
    fn test_cluster_health_states() {
        let healthy = ClusterHealth::Healthy;
        let degraded = ClusterHealth::Degraded;
        let critical = ClusterHealth::Critical;
        let failed = ClusterHealth::Failed;

        // Test serialization
        assert!(serde_json::to_string(&healthy).is_ok());
        assert!(serde_json::to_string(&degraded).is_ok());
        assert!(serde_json::to_string(&critical).is_ok());
        assert!(serde_json::to_string(&failed).is_ok());
    }
}
