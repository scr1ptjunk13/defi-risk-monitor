use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use bigdecimal::BigDecimal;

/// Protocol event types for monitoring
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_type", rename_all = "snake_case")]
pub enum EventType {
    Exploit,
    Governance,
    Audit,
    Upgrade,
    Emergency,
    Vulnerability,
    Regulatory,
    Partnership,
    TokenListing,
    LiquidityChange,
}

/// Severity levels for protocol events
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "event_severity", rename_all = "snake_case")]
pub enum EventSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Protocol event from external sources
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolEvent {
    pub id: Uuid,
    pub protocol_name: String,
    pub event_type: EventType,
    pub severity: EventSeverity,
    pub title: String,
    pub description: String,
    pub source: String,
    pub source_url: Option<String>,
    pub impact_score: BigDecimal,
    pub affected_chains: Vec<i32>,
    pub affected_tokens: Vec<String>,
    pub event_timestamp: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
    pub processed: bool,
    pub alert_sent: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Exploit-specific event data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExploitEvent {
    pub id: Uuid,
    pub protocol_event_id: Uuid,
    pub exploit_type: String,
    pub funds_lost_usd: Option<BigDecimal>,
    pub attack_vector: String,
    pub root_cause: String,
    pub affected_contracts: Vec<String>,
    pub exploit_tx_hash: Option<String>,
    pub attacker_address: Option<String>,
    pub recovery_status: String,
    pub post_mortem_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Governance event data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GovernanceEvent {
    pub id: Uuid,
    pub protocol_event_id: Uuid,
    pub proposal_id: String,
    pub proposal_type: String,
    pub voting_status: String,
    pub voting_deadline: Option<DateTime<Utc>>,
    pub quorum_required: Option<BigDecimal>,
    pub current_votes: Option<BigDecimal>,
    pub proposal_url: Option<String>,
    pub risk_impact: String,
    pub created_at: DateTime<Utc>,
}

/// Audit event data
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditEvent {
    pub id: Uuid,
    pub protocol_event_id: Uuid,
    pub auditor_name: String,
    pub audit_type: String,
    pub audit_status: String,
    pub findings_count: i32,
    pub critical_findings: i32,
    pub high_findings: i32,
    pub medium_findings: i32,
    pub low_findings: i32,
    pub audit_report_url: Option<String>,
    pub completion_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// External feed configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploitFeed {
    pub name: String,
    pub url: String,
    pub feed_type: String,
    pub api_key: Option<String>,
    pub polling_interval_seconds: u64,
    pub last_checked: Option<DateTime<Utc>>,
    pub enabled: bool,
}

/// Governance monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceMonitor {
    pub protocols: Vec<String>,
    pub snapshot_spaces: Vec<String>,
    pub on_chain_governance: Vec<String>,
    pub polling_interval_seconds: u64,
    pub vote_threshold_alerts: bool,
}

/// Audit tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditTracker {
    pub audit_firms: Vec<String>,
    pub github_repos: Vec<String>,
    pub audit_platforms: Vec<String>,
    pub polling_interval_seconds: u64,
    pub alert_on_new_audits: bool,
}

/// Protocol event monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolEventMonitor {
    pub exploit_feeds: Vec<ExploitFeed>,
    pub governance_changes: GovernanceMonitor,
    pub audit_updates: AuditTracker,
    pub enabled_protocols: Vec<String>,
    pub risk_threshold: BigDecimal,
    pub auto_alert_enabled: bool,
}

/// Event impact assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventImpact {
    pub event_id: Uuid,
    pub risk_score_change: BigDecimal,
    pub affected_positions: Vec<Uuid>,
    pub recommended_actions: Vec<String>,
    pub urgency_level: EventSeverity,
    pub estimated_impact_usd: Option<BigDecimal>,
}

/// Event alert configuration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct EventAlert {
    pub id: Uuid,
    pub user_address: String,
    pub protocol_name: String,
    pub event_types: Vec<EventType>,
    pub min_severity: EventSeverity,
    pub notification_channels: Vec<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProtocolEvent {
    /// Calculate risk impact score based on event details
    pub fn calculate_impact_score(&self) -> BigDecimal {
        let base_score = match self.severity {
            EventSeverity::Critical => BigDecimal::from(100),
            EventSeverity::High => BigDecimal::from(75),
            EventSeverity::Medium => BigDecimal::from(50),
            EventSeverity::Low => BigDecimal::from(25),
            EventSeverity::Info => BigDecimal::from(10),
        };

        let type_multiplier = match self.event_type {
            EventType::Exploit => BigDecimal::from(2),
            EventType::Emergency => BigDecimal::from(2),
            EventType::Vulnerability => BigDecimal::from(2),
            EventType::Governance => BigDecimal::from(1),
            EventType::Audit => BigDecimal::from(1),
            EventType::Upgrade => BigDecimal::from(1),
            _ => BigDecimal::from(1),
        };

        base_score * type_multiplier
    }

    /// Check if event affects specific protocol
    pub fn affects_protocol(&self, protocol: &str) -> bool {
        self.protocol_name.to_lowercase() == protocol.to_lowercase()
    }

    /// Check if event affects specific chain
    pub fn affects_chain(&self, chain_id: i32) -> bool {
        self.affected_chains.contains(&chain_id)
    }
}

impl Default for ProtocolEventMonitor {
    fn default() -> Self {
        Self {
            exploit_feeds: vec![
                ExploitFeed {
                    name: "Rekt News".to_string(),
                    url: "https://rekt.news/feed/".to_string(),
                    feed_type: "rss".to_string(),
                    api_key: None,
                    polling_interval_seconds: 300, // 5 minutes
                    last_checked: None,
                    enabled: true,
                },
                ExploitFeed {
                    name: "DeFiYield Rekt Database".to_string(),
                    url: "https://defiyield.app/api/rekt".to_string(),
                    feed_type: "api".to_string(),
                    api_key: None,
                    polling_interval_seconds: 600, // 10 minutes
                    last_checked: None,
                    enabled: true,
                },
            ],
            governance_changes: GovernanceMonitor {
                protocols: vec![
                    "uniswap".to_string(),
                    "curve".to_string(),
                    "aave".to_string(),
                    "compound".to_string(),
                ],
                snapshot_spaces: vec![
                    "uniswap".to_string(),
                    "curve.eth".to_string(),
                    "aave.eth".to_string(),
                ],
                on_chain_governance: vec![
                    "0x5e4be8Bc9637f0EAA1A755019e06A68ce081D58F".to_string(), // Uniswap Governor
                ],
                polling_interval_seconds: 1800, // 30 minutes
                vote_threshold_alerts: true,
            },
            audit_updates: AuditTracker {
                audit_firms: vec![
                    "trail-of-bits".to_string(),
                    "consensys-diligence".to_string(),
                    "openzeppelin".to_string(),
                    "certik".to_string(),
                ],
                github_repos: vec![
                    "Uniswap/v3-core".to_string(),
                    "curvefi/curve-contract".to_string(),
                ],
                audit_platforms: vec![
                    "code4rena".to_string(),
                    "immunefi".to_string(),
                ],
                polling_interval_seconds: 3600, // 1 hour
                alert_on_new_audits: true,
            },
            enabled_protocols: vec![
                "uniswap_v3".to_string(),
                "curve".to_string(),
                "aave".to_string(),
            ],
            risk_threshold: BigDecimal::from(50),
            auto_alert_enabled: true,
        }
    }
}
