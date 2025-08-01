use std::time::Duration;
use serde::{Serialize, Deserialize};
// use crate::database::replication::{DatabaseNode, DatabaseRole, FailoverConfig};
// Temporarily commented out until replication module is implemented

// Placeholder types until replication module is implemented
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseRole {
    Primary,
    Replica,
    Standby,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub timeout_seconds: u64,
    pub max_retries: u32,
    pub health_check_interval: Duration,
    pub failure_threshold: u32,
    pub recovery_threshold: u32,
    pub max_replication_lag_ms: u64,
    pub failover_timeout: Duration,
    pub auto_failback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseNode {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub url: String,
    pub role: DatabaseRole,
    pub priority: u32,
    pub max_connections: u32,
    pub health_check_interval: Duration,
    pub is_active: bool,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            health_check_interval: Duration::from_secs(5),
            failure_threshold: 3,
            recovery_threshold: 5,
            max_replication_lag_ms: 1000,
            failover_timeout: Duration::from_secs(30),
            auto_failback: false,
        }
    }
}

/// Comprehensive disaster recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisasterRecoveryConfig {
    pub database_cluster: DatabaseClusterConfig,
    pub backup_strategy: BackupStrategy,
    pub recovery_procedures: RecoveryProcedures,
    pub monitoring_thresholds: MonitoringThresholds,
    pub notification_settings: NotificationSettings,
}

/// Database cluster configuration for high availability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseClusterConfig {
    pub nodes: Vec<DatabaseNode>,
    pub failover_config: FailoverConfig,
    pub load_balancing: LoadBalancingConfig,
    pub connection_pooling: ConnectionPoolingConfig,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    pub read_strategy: ReadStrategy,
    pub write_strategy: WriteStrategy,
    pub health_check_interval: Duration,
    pub circuit_breaker_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReadStrategy {
    RoundRobin,
    LeastConnections,
    PriorityBased,
    GeographicProximity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WriteStrategy {
    PrimaryOnly,
    PrimaryWithSyncReplica,
    MultiMaster,
}

/// Connection pooling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolingConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_lifetime: Duration,
    pub health_check_interval: Duration,
}

/// Backup strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupStrategy {
    pub full_backup_schedule: BackupSchedule,
    pub incremental_backup_schedule: BackupSchedule,
    pub point_in_time_recovery: PointInTimeRecovery,
    pub backup_retention: BackupRetention,
    pub backup_locations: Vec<BackupLocation>,
    pub encryption: BackupEncryption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSchedule {
    pub enabled: bool,
    pub cron_expression: String,
    pub max_duration: Duration,
    pub compression: bool,
    pub verification: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointInTimeRecovery {
    pub enabled: bool,
    pub wal_archiving: bool,
    pub archive_location: String,
    pub retention_period: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupRetention {
    pub daily_backups: u32,
    pub weekly_backups: u32,
    pub monthly_backups: u32,
    pub yearly_backups: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupLocation {
    pub location_type: BackupLocationType,
    pub path: String,
    pub credentials: Option<String>,
    pub is_primary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackupLocationType {
    Local,
    S3,
    GoogleCloud,
    Azure,
    SFTP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEncryption {
    pub enabled: bool,
    pub algorithm: String,
    pub key_management: KeyManagement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyManagement {
    Local,
    AwsKms,
    HashiCorpVault,
    AzureKeyVault,
}

/// Recovery procedures and automation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryProcedures {
    pub automatic_failover: AutomaticFailover,
    pub manual_procedures: Vec<ManualProcedure>,
    pub recovery_testing: RecoveryTesting,
    pub rollback_procedures: RollbackProcedures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomaticFailover {
    pub enabled: bool,
    pub failure_detection_timeout: Duration,
    pub failover_timeout: Duration,
    pub auto_failback: bool,
    pub failback_delay: Duration,
    pub notification_channels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualProcedure {
    pub name: String,
    pub description: String,
    pub steps: Vec<String>,
    pub estimated_duration: Duration,
    pub required_permissions: Vec<String>,
    pub verification_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryTesting {
    pub enabled: bool,
    pub test_schedule: String,
    pub test_scenarios: Vec<TestScenario>,
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub name: String,
    pub description: String,
    pub failure_type: FailureType,
    pub expected_recovery_time: Duration,
    pub data_loss_tolerance: DataLossTolerance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FailureType {
    PrimaryDatabaseFailure,
    ReplicaFailure,
    NetworkPartition,
    DataCorruption,
    HardwareFailure,
    SoftwareFailure,
    HumanError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataLossTolerance {
    Zero,
    Minimal,      // < 1 minute
    Acceptable,   // < 5 minutes
    High,         // < 15 minutes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackProcedures {
    pub enabled: bool,
    pub rollback_window: Duration,
    pub verification_required: bool,
    pub approval_required: bool,
    pub automated_rollback_triggers: Vec<RollbackTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackTrigger {
    pub name: String,
    pub condition: String,
    pub threshold: f64,
    pub action: RollbackAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackAction {
    Alert,
    AutomaticRollback,
    RequestApproval,
}

/// Monitoring thresholds for disaster recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringThresholds {
    pub database_health: DatabaseHealthThresholds,
    pub replication_lag: ReplicationLagThresholds,
    pub performance_metrics: PerformanceThresholds,
    pub resource_utilization: ResourceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealthThresholds {
    pub connection_failure_rate: f64,
    pub query_timeout_rate: f64,
    pub error_rate: f64,
    pub response_time_p99: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationLagThresholds {
    pub warning_threshold: Duration,
    pub critical_threshold: Duration,
    pub failover_threshold: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    pub cpu_utilization: f64,
    pub memory_utilization: f64,
    pub disk_utilization: f64,
    pub network_utilization: f64,
    pub query_performance: QueryPerformanceThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryPerformanceThresholds {
    pub slow_query_threshold: Duration,
    pub blocked_query_threshold: Duration,
    pub deadlock_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceThresholds {
    pub disk_space_warning: f64,
    pub disk_space_critical: f64,
    pub connection_pool_utilization: f64,
    pub wal_size_threshold: u64,
}

/// Notification settings for disaster recovery events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub channels: Vec<NotificationChannel>,
    pub escalation_policy: EscalationPolicy,
    pub notification_templates: Vec<NotificationTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub name: String,
    pub channel_type: ChannelType,
    pub endpoint: String,
    pub severity_filter: Vec<Severity>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    Email,
    Slack,
    PagerDuty,
    Webhook,
    SMS,
    Discord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
    Emergency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    pub enabled: bool,
    pub escalation_levels: Vec<EscalationLevel>,
    pub acknowledgment_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationLevel {
    pub level: u32,
    pub delay: Duration,
    pub channels: Vec<String>,
    pub required_acknowledgment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub name: String,
    pub event_type: String,
    pub subject_template: String,
    pub body_template: String,
    pub variables: Vec<String>,
}

impl Default for DisasterRecoveryConfig {
    fn default() -> Self {
        Self {
            database_cluster: DatabaseClusterConfig::default(),
            backup_strategy: BackupStrategy::default(),
            recovery_procedures: RecoveryProcedures::default(),
            monitoring_thresholds: MonitoringThresholds::default(),
            notification_settings: NotificationSettings::default(),
        }
    }
}

impl Default for DatabaseClusterConfig {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            failover_config: FailoverConfig::default(),
            load_balancing: LoadBalancingConfig::default(),
            connection_pooling: ConnectionPoolingConfig::default(),
        }
    }
}

impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            read_strategy: ReadStrategy::PriorityBased,
            write_strategy: WriteStrategy::PrimaryOnly,
            health_check_interval: Duration::from_secs(5),
            circuit_breaker_threshold: 5,
        }
    }
}

impl Default for ConnectionPoolingConfig {
    fn default() -> Self {
        Self {
            min_connections: 5,
            max_connections: 50,
            connection_timeout: Duration::from_secs(10),
            idle_timeout: Duration::from_secs(300),
            max_lifetime: Duration::from_secs(1800),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

impl Default for BackupStrategy {
    fn default() -> Self {
        Self {
            full_backup_schedule: BackupSchedule {
                enabled: true,
                cron_expression: "0 2 * * *".to_string(), // Daily at 2 AM
                max_duration: Duration::from_secs(3600),
                compression: true,
                verification: true,
            },
            incremental_backup_schedule: BackupSchedule {
                enabled: true,
                cron_expression: "0 */6 * * *".to_string(), // Every 6 hours
                max_duration: Duration::from_secs(1800),
                compression: true,
                verification: true,
            },
            point_in_time_recovery: PointInTimeRecovery {
                enabled: true,
                wal_archiving: true,
                archive_location: "/var/lib/postgresql/wal_archive".to_string(),
                retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            },
            backup_retention: BackupRetention {
                daily_backups: 7,
                weekly_backups: 4,
                monthly_backups: 12,
                yearly_backups: 7,
            },
            backup_locations: vec![
                BackupLocation {
                    location_type: BackupLocationType::Local,
                    path: "/var/backups/postgresql".to_string(),
                    credentials: None,
                    is_primary: true,
                },
            ],
            encryption: BackupEncryption {
                enabled: true,
                algorithm: "AES-256-GCM".to_string(),
                key_management: KeyManagement::Local,
            },
        }
    }
}

impl Default for RecoveryProcedures {
    fn default() -> Self {
        Self {
            automatic_failover: AutomaticFailover {
                enabled: true,
                failure_detection_timeout: Duration::from_secs(30),
                failover_timeout: Duration::from_secs(60),
                auto_failback: false, // Manual failback for safety
                failback_delay: Duration::from_secs(300),
                notification_channels: vec!["critical_alerts".to_string()],
            },
            manual_procedures: vec![
                ManualProcedure {
                    name: "Primary Database Recovery".to_string(),
                    description: "Steps to recover primary database from backup".to_string(),
                    steps: vec![
                        "1. Stop application traffic".to_string(),
                        "2. Assess data corruption extent".to_string(),
                        "3. Restore from latest backup".to_string(),
                        "4. Apply WAL files for point-in-time recovery".to_string(),
                        "5. Verify data integrity".to_string(),
                        "6. Resume application traffic".to_string(),
                    ],
                    estimated_duration: Duration::from_secs(1800), // 30 minutes
                    required_permissions: vec!["database_admin".to_string()],
                    verification_steps: vec![
                        "Check database connectivity".to_string(),
                        "Verify recent transactions".to_string(),
                        "Run data integrity checks".to_string(),
                    ],
                },
            ],
            recovery_testing: RecoveryTesting {
                enabled: true,
                test_schedule: "0 3 * * 0".to_string(), // Weekly on Sunday at 3 AM
                test_scenarios: vec![
                    TestScenario {
                        name: "Primary Database Failure".to_string(),
                        description: "Simulate primary database failure and test failover".to_string(),
                        failure_type: FailureType::PrimaryDatabaseFailure,
                        expected_recovery_time: Duration::from_secs(60),
                        data_loss_tolerance: DataLossTolerance::Zero,
                    },
                ],
                success_criteria: vec![
                    "Failover completes within 60 seconds".to_string(),
                    "No data loss detected".to_string(),
                    "Application remains available".to_string(),
                ],
            },
            rollback_procedures: RollbackProcedures {
                enabled: true,
                rollback_window: Duration::from_secs(3600), // 1 hour
                verification_required: true,
                approval_required: true,
                automated_rollback_triggers: vec![
                    RollbackTrigger {
                        name: "High Error Rate".to_string(),
                        condition: "error_rate > 0.05".to_string(),
                        threshold: 0.05,
                        action: RollbackAction::RequestApproval,
                    },
                ],
            },
        }
    }
}

impl Default for MonitoringThresholds {
    fn default() -> Self {
        Self {
            database_health: DatabaseHealthThresholds {
                connection_failure_rate: 0.01, // 1%
                query_timeout_rate: 0.005,     // 0.5%
                error_rate: 0.001,             // 0.1%
                response_time_p99: Duration::from_millis(1000),
            },
            replication_lag: ReplicationLagThresholds {
                warning_threshold: Duration::from_secs(5),
                critical_threshold: Duration::from_secs(30),
                failover_threshold: Duration::from_secs(60),
            },
            performance_metrics: PerformanceThresholds {
                cpu_utilization: 0.8,    // 80%
                memory_utilization: 0.85, // 85%
                disk_utilization: 0.9,   // 90%
                network_utilization: 0.8, // 80%
                query_performance: QueryPerformanceThresholds {
                    slow_query_threshold: Duration::from_secs(5),
                    blocked_query_threshold: Duration::from_secs(10),
                    deadlock_rate: 0.001, // 0.1%
                },
            },
            resource_utilization: ResourceThresholds {
                disk_space_warning: 0.8,  // 80%
                disk_space_critical: 0.95, // 95%
                connection_pool_utilization: 0.9, // 90%
                wal_size_threshold: 10 * 1024 * 1024 * 1024, // 10GB
            },
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            channels: vec![
                NotificationChannel {
                    name: "critical_alerts".to_string(),
                    channel_type: ChannelType::Email,
                    endpoint: "alerts@company.com".to_string(),
                    severity_filter: vec![Severity::Critical, Severity::Emergency],
                    enabled: true,
                },
            ],
            escalation_policy: EscalationPolicy {
                enabled: true,
                escalation_levels: vec![
                    EscalationLevel {
                        level: 1,
                        delay: Duration::from_secs(300), // 5 minutes
                        channels: vec!["critical_alerts".to_string()],
                        required_acknowledgment: true,
                    },
                ],
                acknowledgment_timeout: Duration::from_secs(900), // 15 minutes
            },
            notification_templates: vec![
                NotificationTemplate {
                    name: "database_failover".to_string(),
                    event_type: "failover".to_string(),
                    subject_template: "CRITICAL: Database Failover - {{cluster_name}}".to_string(),
                    body_template: "Database failover occurred at {{timestamp}} for cluster {{cluster_name}}. New primary: {{new_primary}}".to_string(),
                    variables: vec!["cluster_name".to_string(), "timestamp".to_string(), "new_primary".to_string()],
                },
            ],
        }
    }
}

/// Production-ready disaster recovery configuration for financial applications
pub fn create_production_dr_config() -> DisasterRecoveryConfig {
    DisasterRecoveryConfig {
        database_cluster: DatabaseClusterConfig {
            nodes: vec![
                DatabaseNode {
                    id: "primary".to_string(),
                    host: "localhost".to_string(),
                    port: 5432,
                    url: std::env::var("DATABASE_PRIMARY_URL").unwrap_or_else(|_| "postgresql://localhost:5432/defi_risk_monitor".to_string()),
                    role: DatabaseRole::Primary,
                    priority: 100,
                    max_connections: 100,
                    health_check_interval: Duration::from_secs(5),
                    is_active: true,
                },
                DatabaseNode {
                    id: "replica_1".to_string(),
                    host: "localhost".to_string(),
                    port: 5433,
                    url: std::env::var("DATABASE_REPLICA1_URL").unwrap_or_else(|_| "postgresql://localhost:5433/defi_risk_monitor".to_string()),
                    role: DatabaseRole::Replica,
                    priority: 90,
                    max_connections: 50,
                    health_check_interval: Duration::from_secs(5),
                    is_active: true,
                },
                DatabaseNode {
                    id: "standby".to_string(),
                    host: "localhost".to_string(),
                    port: 5434,
                    url: std::env::var("DATABASE_STANDBY_URL").unwrap_or_else(|_| "postgresql://localhost:5434/defi_risk_monitor".to_string()),
                    role: DatabaseRole::Standby,
                    priority: 80,
                    max_connections: 100,
                    health_check_interval: Duration::from_secs(5),
                    is_active: true,
                },
            ],
            failover_config: FailoverConfig {
                timeout_seconds: 30,
                max_retries: 3,
                health_check_interval: Duration::from_secs(2), // More frequent for production
                failure_threshold: 3,
                recovery_threshold: 5,
                max_replication_lag_ms: 500, // Stricter for financial data
                failover_timeout: Duration::from_secs(30),
                auto_failback: false, // Manual failback for safety
            },
            load_balancing: LoadBalancingConfig {
                read_strategy: ReadStrategy::PriorityBased,
                write_strategy: WriteStrategy::PrimaryOnly,
                health_check_interval: Duration::from_secs(2),
                circuit_breaker_threshold: 3,
            },
            connection_pooling: ConnectionPoolingConfig {
                min_connections: 10,
                max_connections: 200,
                connection_timeout: Duration::from_secs(5),
                idle_timeout: Duration::from_secs(300),
                max_lifetime: Duration::from_secs(1800),
                health_check_interval: Duration::from_secs(30),
            },
        },
        backup_strategy: BackupStrategy {
            full_backup_schedule: BackupSchedule {
                enabled: true,
                cron_expression: "0 1 * * *".to_string(), // Daily at 1 AM
                max_duration: Duration::from_secs(7200), // 2 hours max
                compression: true,
                verification: true,
            },
            incremental_backup_schedule: BackupSchedule {
                enabled: true,
                cron_expression: "0 */4 * * *".to_string(), // Every 4 hours
                max_duration: Duration::from_secs(1800),
                compression: true,
                verification: true,
            },
            point_in_time_recovery: PointInTimeRecovery {
                enabled: true,
                wal_archiving: true,
                archive_location: std::env::var("WAL_ARCHIVE_LOCATION").unwrap_or_else(|_| "/var/lib/postgresql/wal_archive".to_string()),
                retention_period: Duration::from_secs(30 * 24 * 3600), // 30 days for financial compliance
            },
            backup_retention: BackupRetention {
                daily_backups: 30,   // 30 days
                weekly_backups: 12,  // 3 months
                monthly_backups: 24, // 2 years
                yearly_backups: 7,   // 7 years for financial compliance
            },
            backup_locations: vec![
                BackupLocation {
                    location_type: BackupLocationType::S3,
                    path: std::env::var("BACKUP_S3_BUCKET").unwrap_or_else(|_| "s3://defi-risk-monitor-backups".to_string()),
                    credentials: Some("aws_credentials".to_string()),
                    is_primary: true,
                },
                BackupLocation {
                    location_type: BackupLocationType::Local,
                    path: "/var/backups/postgresql".to_string(),
                    credentials: None,
                    is_primary: false,
                },
            ],
            encryption: BackupEncryption {
                enabled: true,
                algorithm: "AES-256-GCM".to_string(),
                key_management: KeyManagement::AwsKms,
            },
        },
        recovery_procedures: RecoveryProcedures::default(),
        monitoring_thresholds: MonitoringThresholds {
            database_health: DatabaseHealthThresholds {
                connection_failure_rate: 0.005, // 0.5% - stricter for financial systems
                query_timeout_rate: 0.001,      // 0.1%
                error_rate: 0.0005,             // 0.05%
                response_time_p99: Duration::from_millis(500), // 500ms
            },
            replication_lag: ReplicationLagThresholds {
                warning_threshold: Duration::from_secs(1),  // 1 second
                critical_threshold: Duration::from_secs(5), // 5 seconds
                failover_threshold: Duration::from_secs(10), // 10 seconds
            },
            performance_metrics: PerformanceThresholds {
                cpu_utilization: 0.7,    // 70% - more conservative
                memory_utilization: 0.8,  // 80%
                disk_utilization: 0.85,   // 85%
                network_utilization: 0.7, // 70%
                query_performance: QueryPerformanceThresholds {
                    slow_query_threshold: Duration::from_secs(2),
                    blocked_query_threshold: Duration::from_secs(5),
                    deadlock_rate: 0.0001, // 0.01%
                },
            },
            resource_utilization: ResourceThresholds {
                disk_space_warning: 0.75,  // 75%
                disk_space_critical: 0.9,  // 90%
                connection_pool_utilization: 0.85, // 85%
                wal_size_threshold: 5 * 1024 * 1024 * 1024, // 5GB
            },
        },
        notification_settings: NotificationSettings::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disaster_recovery_config_creation() {
        let config = DisasterRecoveryConfig::default();
        assert!(config.backup_strategy.full_backup_schedule.enabled);
        assert!(config.recovery_procedures.automatic_failover.enabled);
    }

    #[test]
    fn test_production_dr_config() {
        let config = create_production_dr_config();
        assert_eq!(config.database_cluster.nodes.len(), 3);
        assert!(config.backup_strategy.encryption.enabled);
        assert_eq!(config.backup_strategy.backup_retention.yearly_backups, 7);
    }

    #[test]
    fn test_monitoring_thresholds() {
        let thresholds = MonitoringThresholds::default();
        assert!(thresholds.database_health.connection_failure_rate < 0.02);
        assert!(thresholds.replication_lag.critical_threshold > Duration::from_secs(10));
    }
}
