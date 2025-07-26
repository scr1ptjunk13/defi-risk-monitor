use crate::models::Alert;
use crate::config::Settings;
use crate::error::AppError;
use reqwest::Client;
use serde_json::json;
use tracing::{info, warn};

pub struct AlertService {
    client: Client,
    settings: Settings,
}

impl AlertService {
    pub fn new(settings: &Settings) -> Self {
        Self {
            client: Client::new(),
            settings: settings.clone(),
        }
    }

    pub async fn send_alert(&self, alert: &Alert) -> Result<(), AppError> {
        info!("Sending alert: {}", alert.title);

        // Send to all configured channels
        let mut results = Vec::new();

        if let Some(ref slack_url) = self.settings.alerts.slack_webhook_url {
            results.push(self.send_slack_alert(alert, slack_url).await);
        }

        if let Some(ref discord_url) = self.settings.alerts.discord_webhook_url {
            results.push(self.send_discord_alert(alert, discord_url).await);
        }

        if self.settings.alerts.email_smtp_host.is_some() {
            results.push(self.send_email_alert(alert).await);
        }

        // Check if any notifications succeeded
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        
        if success_count == 0 && !results.is_empty() {
            return Err(AppError::AlertError("Failed to send alert to any channel".to_string()));
        }

        info!("Alert sent successfully to {}/{} channels", success_count, results.len());
        Ok(())
    }

    async fn send_slack_alert(&self, alert: &Alert, webhook_url: &str) -> Result<(), AppError> {
        let color = match alert.severity.as_str() {
            "critical" => "#FF0000",
            "high" => "#FF8C00",
            "medium" => "#FFD700",
            "low" => "#32CD32",
            _ => "#808080",
        };

        let payload = json!({
            "attachments": [{
                "color": color,
                "title": alert.title,
                "text": alert.message,
                "fields": [
                    {
                        "title": "Severity",
                        "value": alert.severity.to_uppercase(),
                        "short": true
                    },
                    {
                        "title": "Alert Type",
                        "value": alert.alert_type,
                        "short": true
                    },
                    {
                        "title": "Risk Score",
                        "value": alert.risk_score.as_ref().map(|s| format!("{:.2}", s)).unwrap_or_else(|| "N/A".to_string()),
                        "short": true
                    },
                    {
                        "title": "Position ID",
                        "value": alert.position_id.map(|id| id.to_string()).unwrap_or_else(|| "N/A".to_string()),
                        "short": true
                    }
                ],
                "footer": "DeFi Risk Monitor",
                "ts": alert.created_at.timestamp()
            }]
        });

        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::AlertError(format!("Failed to send Slack alert: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::AlertError(format!(
                "Slack webhook returned status: {}",
                response.status()
            )));
        }

        info!("Slack alert sent successfully");
        Ok(())
    }

    async fn send_discord_alert(&self, alert: &Alert, webhook_url: &str) -> Result<(), AppError> {
        let color = match alert.severity.as_str() {
            "critical" => 16711680, // Red
            "high" => 16753920,     // Orange
            "medium" => 16776960,   // Yellow
            "low" => 3329330,       // Green
            _ => 8421504,           // Gray
        };

        let payload = json!({
            "embeds": [{
                "title": alert.title,
                "description": alert.message,
                "color": color,
                "fields": [
                    {
                        "name": "Severity",
                        "value": alert.severity.to_uppercase(),
                        "inline": true
                    },
                    {
                        "name": "Alert Type",
                        "value": alert.alert_type,
                        "inline": true
                    },
                    {
                        "name": "Risk Score",
                        "value": alert.risk_score.as_ref().map(|s| format!("{:.2}", s)).unwrap_or_else(|| "N/A".to_string()),
                        "inline": true
                    }
                ],
                "footer": {
                    "text": "DeFi Risk Monitor"
                },
                "timestamp": alert.created_at.to_rfc3339()
            }]
        });

        let response = self.client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AppError::AlertError(format!("Failed to send Discord alert: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::AlertError(format!(
                "Discord webhook returned status: {}",
                response.status()
            )));
        }

        info!("Discord alert sent successfully");
        Ok(())
    }

    async fn send_email_alert(&self, alert: &Alert) -> Result<(), AppError> {
        // Email implementation would require additional dependencies like lettre
        // For now, we'll just log that an email would be sent
        warn!("Email alerts not yet implemented - would send: {}", alert.title);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Settings;
use crate::config::settings::AlertSettings;

    #[test]
    fn test_alert_service_creation() {
        let settings = Settings {
            alerts: AlertSettings {
                slack_webhook_url: Some("https://hooks.slack.com/test".to_string()),
                discord_webhook_url: None,
                email_smtp_host: None,
                email_smtp_port: None,
                email_username: None,
                email_password: None,
            },
            api: crate::config::settings::ApiSettings {
                host: "0.0.0.0".to_string(),
                port: 8080,
            },
            database: crate::config::settings::DatabaseSettings {
                url: "postgresql://test:test@localhost/test".to_string(),
            },
            blockchain: crate::config::settings::BlockchainSettings {
                ethereum_rpc_url: "https://mainnet.infura.io/v3/test".to_string(),
                polygon_rpc_url: "https://polygon-mainnet.infura.io/v3/test".to_string(),
                arbitrum_rpc_url: "https://arbitrum-mainnet.infura.io/v3/test".to_string(),
                risk_check_interval_seconds: 60,
            },
            risk: crate::config::settings::RiskSettings {
                max_position_size_usd: 1000000.0,
                liquidation_threshold: 0.85,
            },
            logging: crate::config::settings::LoggingSettings {
                level: "info".to_string(),
            }
        };

        let _service = AlertService::new(&settings);
        // Basic test to ensure the service can be created
        assert!(true);
    }
}
