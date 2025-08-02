use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::{info, warn, error};
use sqlx::PgPool;
use reqwest::Client;
use serde_json::Value;
use chrono::{DateTime, Utc};
use uuid::Uuid;
use bigdecimal::BigDecimal;

use crate::models::protocol_events::*;
use crate::error::AppError;
use crate::services::alert_service::AlertService;

/// Protocol Event Monitoring Service
/// Provides comprehensive external event monitoring for proactive risk detection
pub struct ProtocolEventMonitorService {
    db_pool: Arc<PgPool>,
    http_client: Client,
    alert_service: Arc<AlertService>,
    config: ProtocolEventMonitor,
    running: Arc<tokio::sync::RwLock<bool>>,
}

impl ProtocolEventMonitorService {
    /// Create new protocol event monitor service
    pub fn new(
        db_pool: Arc<PgPool>,
        alert_service: Arc<AlertService>,
        config: Option<ProtocolEventMonitor>,
    ) -> Self {
        Self {
            db_pool,
            http_client: Client::new(),
            alert_service,
            config: config.unwrap_or_default(),
            running: Arc::new(tokio::sync::RwLock::new(false)),
        }
    }

    /// Start the monitoring service
    pub async fn start(&self) -> Result<(), AppError> {
        let mut running = self.running.write().await;
        if *running {
            return Ok(());
        }
        *running = true;
        drop(running);

        info!("Starting Protocol Event Monitor Service");

        // Start exploit monitoring
        let exploit_monitor = self.clone();
        tokio::spawn(async move {
            exploit_monitor.monitor_exploits().await;
        });

        // Start governance monitoring
        let governance_monitor = self.clone();
        tokio::spawn(async move {
            governance_monitor.monitor_governance().await;
        });

        // Start audit monitoring
        let audit_monitor = self.clone();
        tokio::spawn(async move {
            audit_monitor.monitor_audits().await;
        });

        info!("Protocol Event Monitor Service started successfully");
        Ok(())
    }

    /// Stop the monitoring service
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
        info!("Protocol Event Monitor Service stopped");
    }

    /// Monitor exploit feeds for new security incidents
    async fn monitor_exploits(&self) {
        let mut interval = interval(Duration::from_secs(
            self.config.exploit_feeds.first()
                .map(|f| f.polling_interval_seconds)
                .unwrap_or(300)
        ));

        loop {
            if !*self.running.read().await {
                break;
            }

            interval.tick().await;

            for feed in &self.config.exploit_feeds {
                if !feed.enabled {
                    continue;
                }

                match self.fetch_exploit_events(feed).await {
                    Ok(events) => {
                        for event in events {
                            if let Err(e) = self.process_exploit_event(event).await {
                                error!("Failed to process exploit event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch from exploit feed {}: {}", feed.name, e);
                    }
                }
            }
        }
    }

    /// Monitor governance changes across protocols
    async fn monitor_governance(&self) {
        let mut interval = interval(Duration::from_secs(
            self.config.governance_changes.polling_interval_seconds
        ));

        loop {
            if !*self.running.read().await {
                break;
            }

            interval.tick().await;

            // Monitor Snapshot spaces
            for space in &self.config.governance_changes.snapshot_spaces {
                match self.fetch_snapshot_proposals(space).await {
                    Ok(proposals) => {
                        for proposal in proposals {
                            if let Err(e) = self.process_governance_event(proposal).await {
                                error!("Failed to process governance event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch Snapshot proposals for {}: {}", space, e);
                    }
                }
            }

            // Monitor on-chain governance
            for governor in &self.config.governance_changes.on_chain_governance {
                match self.fetch_onchain_proposals(governor).await {
                    Ok(proposals) => {
                        for proposal in proposals {
                            if let Err(e) = self.process_governance_event(proposal).await {
                                error!("Failed to process on-chain governance event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch on-chain proposals for {}: {}", governor, e);
                    }
                }
            }
        }
    }

    /// Monitor audit updates and security reports
    async fn monitor_audits(&self) {
        let mut interval = interval(Duration::from_secs(
            self.config.audit_updates.polling_interval_seconds
        ));

        loop {
            if !*self.running.read().await {
                break;
            }

            interval.tick().await;

            // Monitor GitHub repositories for audit reports
            for repo in &self.config.audit_updates.github_repos {
                match self.fetch_github_audits(repo).await {
                    Ok(audits) => {
                        for audit in audits {
                            if let Err(e) = self.process_audit_event(audit).await {
                                error!("Failed to process audit event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch GitHub audits for {}: {}", repo, e);
                    }
                }
            }

            // Monitor audit platforms
            for platform in &self.config.audit_updates.audit_platforms {
                match self.fetch_platform_audits(platform).await {
                    Ok(audits) => {
                        for audit in audits {
                            if let Err(e) = self.process_audit_event(audit).await {
                                error!("Failed to process platform audit event: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch audits from platform {}: {}", platform, e);
                    }
                }
            }
        }
    }

    /// Fetch exploit events from RSS/API feeds
    async fn fetch_exploit_events(&self, feed: &ExploitFeed) -> Result<Vec<ProtocolEvent>, AppError> {
        let response = match feed.feed_type.as_str() {
            "rss" => self.fetch_rss_feed(&feed.url).await?,
            "api" => self.fetch_api_feed(&feed.url, feed.api_key.as_deref()).await?,
            _ => return Err(AppError::ValidationError("Unsupported feed type".to_string())),
        };

        self.parse_exploit_events(response, feed).await
    }

    /// Fetch Snapshot governance proposals
    async fn fetch_snapshot_proposals(&self, space: &str) -> Result<Vec<ProtocolEvent>, AppError> {
        let query = format!(r#"
            {{
                proposals(
                    first: 20,
                    skip: 0,
                    where: {{
                        space_in: ["{}"],
                        state: "active"
                    }},
                    orderBy: "created",
                    orderDirection: desc
                ) {{
                    id
                    title
                    body
                    choices
                    start
                    end
                    snapshot
                    state
                    author
                    space {{
                        id
                        name
                    }}
                }}
            }}
        "#, space);

        let response = self.http_client
            .post("https://hub.snapshot.org/graphql")
            .json(&serde_json::json!({ "query": query }))
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("Snapshot API error: {}", e)))?;

        let data: Value = response.json().await
            .map_err(|e| AppError::ExternalServiceError(format!("Snapshot response parse error: {}", e)))?;

        self.parse_snapshot_proposals(data).await
    }

    /// Fetch on-chain governance proposals
    async fn fetch_onchain_proposals(&self, governor_address: &str) -> Result<Vec<ProtocolEvent>, AppError> {
        // This would integrate with your blockchain service to fetch on-chain governance events
        // For now, returning empty vector as placeholder
        info!("Fetching on-chain proposals for governor: {}", governor_address);
        Ok(vec![])
    }

    /// Fetch audit reports from GitHub
    async fn fetch_github_audits(&self, repo: &str) -> Result<Vec<ProtocolEvent>, AppError> {
        let url = format!("https://api.github.com/repos/{}/contents/audits", repo);
        
        let response = self.http_client
            .get(&url)
            .header("User-Agent", "DeFi-Risk-Monitor")
            .send()
            .await
            .map_err(|e| AppError::ExternalServiceError(format!("GitHub API error: {}", e)))?;

        if response.status().is_success() {
            let data: Value = response.json().await
                .map_err(|e| AppError::ExternalServiceError(format!("GitHub response parse error: {}", e)))?;
            
            self.parse_github_audits(data, repo).await
        } else {
            Ok(vec![])
        }
    }

    /// Fetch audits from audit platforms
    async fn fetch_platform_audits(&self, platform: &str) -> Result<Vec<ProtocolEvent>, AppError> {
        match platform {
            "code4rena" => self.fetch_code4rena_audits().await,
            "immunefi" => self.fetch_immunefi_reports().await,
            _ => Ok(vec![]),
        }
    }

    /// Process exploit event and create alerts
    async fn process_exploit_event(&self, event: ProtocolEvent) -> Result<(), AppError> {
        // Check if event already exists
        if self.event_exists(&event.id).await? {
            return Ok(());
        }

        // Store event in database
        self.store_protocol_event(&event).await?;

        // Create exploit-specific record
        let exploit_event = ExploitEvent {
            id: Uuid::new_v4(),
            protocol_event_id: event.id,
            exploit_type: self.extract_exploit_type(&event),
            funds_lost_usd: self.extract_funds_lost(&event),
            attack_vector: self.extract_attack_vector(&event),
            root_cause: self.extract_root_cause(&event),
            affected_contracts: self.extract_affected_contracts(&event),
            exploit_tx_hash: self.extract_tx_hash(&event),
            attacker_address: self.extract_attacker_address(&event),
            recovery_status: "Unknown".to_string(),
            post_mortem_url: event.source_url.clone(),
            created_at: Utc::now(),
        };

        self.store_exploit_event(&exploit_event).await?;

        // Calculate impact and send alerts
        let impact = self.calculate_event_impact(&event).await?;
        self.send_event_alerts(&event, &impact).await?;

        info!("Processed exploit event: {} for protocol: {}", event.title, event.protocol_name);
        Ok(())
    }

    /// Process governance event
    async fn process_governance_event(&self, event: ProtocolEvent) -> Result<(), AppError> {
        if self.event_exists(&event.id).await? {
            return Ok(());
        }

        self.store_protocol_event(&event).await?;

        let governance_event = GovernanceEvent {
            id: Uuid::new_v4(),
            protocol_event_id: event.id,
            proposal_id: self.extract_proposal_id(&event),
            proposal_type: self.extract_proposal_type(&event),
            voting_status: "Active".to_string(),
            voting_deadline: self.extract_voting_deadline(&event),
            quorum_required: self.extract_quorum(&event),
            current_votes: self.extract_current_votes(&event),
            proposal_url: event.source_url.clone(),
            risk_impact: self.assess_governance_risk(&event),
            created_at: Utc::now(),
        };

        self.store_governance_event(&governance_event).await?;

        let impact = self.calculate_event_impact(&event).await?;
        self.send_event_alerts(&event, &impact).await?;

        info!("Processed governance event: {} for protocol: {}", event.title, event.protocol_name);
        Ok(())
    }

    /// Process audit event
    async fn process_audit_event(&self, event: ProtocolEvent) -> Result<(), AppError> {
        if self.event_exists(&event.id).await? {
            return Ok(());
        }

        self.store_protocol_event(&event).await?;

        let audit_event = AuditEvent {
            id: Uuid::new_v4(),
            protocol_event_id: event.id,
            auditor_name: self.extract_auditor_name(&event),
            audit_type: self.extract_audit_type(&event),
            audit_status: "Completed".to_string(),
            findings_count: self.extract_findings_count(&event),
            critical_findings: self.extract_critical_findings(&event),
            high_findings: self.extract_high_findings(&event),
            medium_findings: self.extract_medium_findings(&event),
            low_findings: self.extract_low_findings(&event),
            audit_report_url: event.source_url.clone(),
            completion_date: Some(event.event_timestamp),
            created_at: Utc::now(),
        };

        self.store_audit_event(&audit_event).await?;

        let impact = self.calculate_event_impact(&event).await?;
        self.send_event_alerts(&event, &impact).await?;

        info!("Processed audit event: {} for protocol: {}", event.title, event.protocol_name);
        Ok(())
    }

    /// Calculate the impact of an event on user positions
    async fn calculate_event_impact(&self, event: &ProtocolEvent) -> Result<EventImpact, AppError> {
        let risk_score_change = event.calculate_impact_score();
        
        // Find affected positions
        let affected_positions = self.find_affected_positions(event).await?;
        
        // Generate recommendations based on event type and severity
        let recommended_actions = self.generate_event_recommendations(event);
        
        // Estimate financial impact
        let estimated_impact_usd = self.estimate_financial_impact(event, &affected_positions).await?;

        Ok(EventImpact {
            event_id: event.id,
            risk_score_change,
            affected_positions,
            recommended_actions,
            urgency_level: event.severity.clone(),
            estimated_impact_usd,
        })
    }

    /// Send alerts for protocol events
    async fn send_event_alerts(&self, event: &ProtocolEvent, impact: &EventImpact) -> Result<(), AppError> {
        if !self.config.auto_alert_enabled {
            return Ok(());
        }

        // Get users who want alerts for this protocol/event type
        let alert_configs = self.get_event_alert_configs(&event.protocol_name, &event.event_type).await?;

        for config in alert_configs {
            if self.should_send_alert(&config, event) {
                let alert_message = self.format_event_alert(event, impact);
                
                for _channel in &config.notification_channels {
                    // Create alert object for the service
                let alert = crate::models::Alert {
                    id: uuid::Uuid::new_v4(),
                    user_address: config.user_address.clone(),
                    position_id: None,
                    threshold_id: None,
                    alert_type: "protocol_event".to_string(),
                    severity: match event.severity {
                        EventSeverity::Critical => "critical".to_string(),
                        EventSeverity::High => "high".to_string(),
                        EventSeverity::Medium => "medium".to_string(),
                        EventSeverity::Low => "low".to_string(),
                        EventSeverity::Info => "info".to_string(),
                    },
                    title: format!("Protocol Event: {}", event.protocol_name),
                    message: alert_message.clone(),
                    risk_score: Some(event.impact_score.clone()),
                    current_value: None,
                    threshold_value: None,
                    metadata: None,
                    is_resolved: false,
                    resolved_at: None,
                    created_at: chrono::Utc::now(),
                };
                
                if let Err(e) = self.alert_service.send_alert(&alert).await {
                        error!("Failed to send event alert to {}: {}", config.user_address, e);
                    }
                }
            }
        }

        Ok(())
    }

    // Helper methods for data extraction and parsing
    fn extract_exploit_type(&self, _event: &ProtocolEvent) -> String {
        // Extract exploit type from event metadata or description
        "Unknown".to_string() // Placeholder
    }

    fn extract_funds_lost(&self, _event: &ProtocolEvent) -> Option<BigDecimal> {
        // Parse funds lost from event description
        None // Placeholder
    }

    // Additional helper methods would be implemented here...
    // (extract_attack_vector, extract_root_cause, etc.)

    /// Check if event already exists in database
    async fn event_exists(&self, event_id: &Uuid) -> Result<bool, AppError> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM protocol_events WHERE id = $1"
        )
        .bind(event_id)
        .fetch_one(&*self.db_pool)
        .await?;

        Ok(count > 0)
    }

    /// Store protocol event in database
    async fn store_protocol_event(&self, event: &ProtocolEvent) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO protocol_events (
                id, protocol_name, event_type, severity, title, description,
                source, source_url, impact_score, affected_chains, affected_tokens,
                event_timestamp, detected_at, processed, alert_sent, metadata,
                created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            "#
        )
        .bind(&event.id)
        .bind(&event.protocol_name)
        .bind(&event.event_type)
        .bind(&event.severity)
        .bind(&event.title)
        .bind(&event.description)
        .bind(&event.source)
        .bind(&event.source_url)
        .bind(&event.impact_score)
        .bind(&event.affected_chains)
        .bind(&event.affected_tokens)
        .bind(&event.event_timestamp)
        .bind(&event.detected_at)
        .bind(&event.processed)
        .bind(&event.alert_sent)
        .bind(&event.metadata)
        .bind(&event.created_at)
        .bind(&event.updated_at)
        .execute(&*self.db_pool)
        .await?;

        Ok(())
    }

    // Additional database storage methods would be implemented here...
    // (store_exploit_event, store_governance_event, store_audit_event)

    // Placeholder implementations for remaining methods
    async fn fetch_rss_feed(&self, _url: &str) -> Result<Value, AppError> {
        Ok(serde_json::json!({}))
    }

    async fn fetch_api_feed(&self, _url: &str, _api_key: Option<&str>) -> Result<Value, AppError> {
        Ok(serde_json::json!({}))
    }

    async fn parse_exploit_events(&self, _data: Value, _feed: &ExploitFeed) -> Result<Vec<ProtocolEvent>, AppError> {
        Ok(vec![])
    }

    async fn parse_snapshot_proposals(&self, _data: Value) -> Result<Vec<ProtocolEvent>, AppError> {
        Ok(vec![])
    }

    async fn parse_github_audits(&self, _data: Value, _repo: &str) -> Result<Vec<ProtocolEvent>, AppError> {
        Ok(vec![])
    }

    async fn fetch_code4rena_audits(&self) -> Result<Vec<ProtocolEvent>, AppError> {
        Ok(vec![])
    }

    async fn fetch_immunefi_reports(&self) -> Result<Vec<ProtocolEvent>, AppError> {
        Ok(vec![])
    }

    async fn find_affected_positions(&self, _event: &ProtocolEvent) -> Result<Vec<Uuid>, AppError> {
        Ok(vec![])
    }

    fn generate_event_recommendations(&self, _event: &ProtocolEvent) -> Vec<String> {
        vec![]
    }

    async fn estimate_financial_impact(&self, _event: &ProtocolEvent, _positions: &[Uuid]) -> Result<Option<BigDecimal>, AppError> {
        Ok(None)
    }

    async fn get_event_alert_configs(&self, _protocol: &str, _event_type: &EventType) -> Result<Vec<EventAlert>, AppError> {
        Ok(vec![])
    }

    fn should_send_alert(&self, _config: &EventAlert, _event: &ProtocolEvent) -> bool {
        true
    }

    fn format_event_alert(&self, _event: &ProtocolEvent, _impact: &EventImpact) -> String {
        "Protocol event alert".to_string()
    }

    // Additional placeholder methods for data extraction
    fn extract_proposal_id(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_proposal_type(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_voting_deadline(&self, _event: &ProtocolEvent) -> Option<DateTime<Utc>> { None }
    fn extract_quorum(&self, _event: &ProtocolEvent) -> Option<BigDecimal> { None }
    fn extract_current_votes(&self, _event: &ProtocolEvent) -> Option<BigDecimal> { None }
    fn assess_governance_risk(&self, _event: &ProtocolEvent) -> String { "Low".to_string() }
    fn extract_auditor_name(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_audit_type(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_findings_count(&self, _event: &ProtocolEvent) -> i32 { 0 }
    fn extract_critical_findings(&self, _event: &ProtocolEvent) -> i32 { 0 }
    fn extract_high_findings(&self, _event: &ProtocolEvent) -> i32 { 0 }
    fn extract_medium_findings(&self, _event: &ProtocolEvent) -> i32 { 0 }
    fn extract_low_findings(&self, _event: &ProtocolEvent) -> i32 { 0 }
    fn extract_attack_vector(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_root_cause(&self, _event: &ProtocolEvent) -> String { "".to_string() }
    fn extract_affected_contracts(&self, _event: &ProtocolEvent) -> Vec<String> { vec![] }
    fn extract_tx_hash(&self, _event: &ProtocolEvent) -> Option<String> { None }
    fn extract_attacker_address(&self, _event: &ProtocolEvent) -> Option<String> { None }

    async fn store_exploit_event(&self, _event: &ExploitEvent) -> Result<(), AppError> { Ok(()) }
    async fn store_governance_event(&self, _event: &GovernanceEvent) -> Result<(), AppError> { Ok(()) }
    async fn store_audit_event(&self, _event: &AuditEvent) -> Result<(), AppError> { Ok(()) }
}

impl Clone for ProtocolEventMonitorService {
    fn clone(&self) -> Self {
        Self {
            db_pool: Arc::clone(&self.db_pool),
            http_client: self.http_client.clone(),
            alert_service: Arc::clone(&self.alert_service),
            config: self.config.clone(),
            running: Arc::clone(&self.running),
        }
    }
}
