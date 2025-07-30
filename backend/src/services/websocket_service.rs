use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use axum::extract::ws::{Message, WebSocket};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use tracing::{info, warn, error};

use crate::models::Alert;
use crate::services::risk_calculator::RiskMetrics;
use crate::error::AppError;

/// WebSocket message types for real-time streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum StreamMessage {
    /// Real-time risk updates for a specific position
    RiskUpdate {
        position_id: Uuid,
        risk_metrics: RiskMetrics,
        timestamp: DateTime<Utc>,
    },
    /// Live alert notifications
    AlertNotification {
        alert: Alert,
        timestamp: DateTime<Utc>,
    },
    /// Position value updates (current value, PnL, etc.)
    PositionUpdate {
        position_id: Uuid,
        current_value_usd: BigDecimal,
        pnl_usd: BigDecimal,
        impermanent_loss_pct: BigDecimal,
        timestamp: DateTime<Utc>,
    },
    /// Market data updates (prices, volatility)
    MarketUpdate {
        token_address: String,
        price_usd: BigDecimal,
        price_change_24h: BigDecimal,
        volatility: BigDecimal,
        timestamp: DateTime<Utc>,
    },
    /// System health and status updates
    SystemStatus {
        status: String,
        message: String,
        timestamp: DateTime<Utc>,
    },
    /// Connection confirmation
    Connected {
        session_id: Uuid,
        timestamp: DateTime<Utc>,
    },
    /// Heartbeat/ping message
    Heartbeat {
        timestamp: DateTime<Utc>,
    },
}

/// WebSocket subscription types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// Subscribe to risk updates for specific position
    PositionRisk(Uuid),
    /// Subscribe to all alerts for a user
    UserAlerts(String), // user_address
    /// Subscribe to position value updates
    PositionValue(Uuid),
    /// Subscribe to market data for specific token
    MarketData(String), // token_address
    /// Subscribe to system status updates
    SystemStatus,
}

/// Client connection information
#[derive(Debug, Clone)]
pub struct WebSocketClient {
    pub session_id: Uuid,
    pub user_address: Option<String>,
    pub subscriptions: Vec<SubscriptionType>,
    pub connected_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

/// WebSocket service for managing real-time connections and streaming
#[derive(Clone)]
pub struct WebSocketService {
    /// Broadcast channel for sending messages to all clients
    broadcast_tx: broadcast::Sender<StreamMessage>,
    /// Connected clients registry
    clients: Arc<RwLock<HashMap<Uuid, WebSocketClient>>>,
    /// Subscription mapping: subscription_type -> set of client session_ids
    subscriptions: Arc<RwLock<HashMap<SubscriptionType, Vec<Uuid>>>>,
}

impl WebSocketService {
    /// Create a new WebSocket service
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        
        Self {
            broadcast_tx,
            clients: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Handle a new WebSocket connection
    pub async fn handle_connection(
        &self,
        socket: WebSocket,
        user_address: Option<String>,
    ) -> Result<(), AppError> {
        let session_id = Uuid::new_v4();
        let mut rx = self.broadcast_tx.subscribe();
        
        // Register client
        let client = WebSocketClient {
            session_id,
            user_address: user_address.clone(),
            subscriptions: Vec::new(),
            connected_at: Utc::now(),
            last_heartbeat: Utc::now(),
        };
        
        {
            let mut clients = self.clients.write().await;
            clients.insert(session_id, client);
        }

        info!("WebSocket client connected: session_id={}, user={:?}", session_id, user_address);

        // Send connection confirmation
        let connected_msg = StreamMessage::Connected {
            session_id,
            timestamp: Utc::now(),
        };
        
        let (mut sender, mut receiver) = socket.split();
        
        // Send initial connection message
        if let Ok(msg_json) = serde_json::to_string(&connected_msg) {
            if let Err(e) = sender.send(Message::Text(msg_json)).await {
                warn!("Failed to send connection message: {}", e);
            }
        }

        // Spawn task to handle incoming messages from client
        let clients_clone = self.clients.clone();
        let subscriptions_clone = self.subscriptions.clone();
        let session_id_clone = session_id;
        
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = Self::handle_client_message(
                            &text,
                            session_id_clone,
                            &clients_clone,
                            &subscriptions_clone,
                        ).await {
                            warn!("Error handling client message: {}", e);
                        }
                    }
                    Ok(Message::Ping(_data)) => {
                        // Update heartbeat
                        let mut clients = clients_clone.write().await;
                        if let Some(client) = clients.get_mut(&session_id_clone) {
                            client.last_heartbeat = Utc::now();
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket client disconnected: {}", session_id_clone);
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        // Handle outgoing messages to client
        let clients_clone2 = self.clients.clone();
        tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                // Check if client should receive this message based on subscriptions
                let should_send = {
                    let clients = clients_clone2.read().await;
                    if let Some(client) = clients.get(&session_id) {
                        Self::should_send_message(&message, client)
                    } else {
                        false
                    }
                };

                if should_send {
                    if let Ok(msg_json) = serde_json::to_string(&message) {
                        if let Err(e) = sender.send(Message::Text(msg_json)).await {
                            warn!("Failed to send message to client {}: {}", session_id, e);
                            break;
                        }
                    }
                }
            }

            // Clean up client on disconnect
            let mut clients = clients_clone2.write().await;
            clients.remove(&session_id);
            info!("Cleaned up disconnected client: {}", session_id);
        });

        Ok(())
    }

    /// Handle incoming message from client (subscriptions, etc.)
    async fn handle_client_message(
        message: &str,
        session_id: Uuid,
        clients: &Arc<RwLock<HashMap<Uuid, WebSocketClient>>>,
        subscriptions: &Arc<RwLock<HashMap<SubscriptionType, Vec<Uuid>>>>,
    ) -> Result<(), AppError> {
        #[derive(Deserialize)]
        struct ClientMessage {
            action: String,
            subscription_type: Option<SubscriptionType>,
        }

        let client_msg: ClientMessage = serde_json::from_str(message)
            .map_err(|e| AppError::ValidationError(format!("Invalid message format: {}", e)))?;

        match client_msg.action.as_str() {
            "subscribe" => {
                if let Some(sub_type) = client_msg.subscription_type {
                    // Add subscription to client
                    {
                        let mut clients_guard = clients.write().await;
                        if let Some(client) = clients_guard.get_mut(&session_id) {
                            if !client.subscriptions.contains(&sub_type) {
                                client.subscriptions.push(sub_type.clone());
                            }
                        }
                    }

                    // Add client to subscription mapping
                    {
                        let mut subs_guard = subscriptions.write().await;
                        let client_list = subs_guard.entry(sub_type.clone()).or_insert_with(Vec::new);
                        if !client_list.contains(&session_id) {
                            client_list.push(session_id);
                        }
                    }

                    info!("Client {} subscribed to {:?}", session_id, sub_type);
                }
            }
            "unsubscribe" => {
                if let Some(sub_type) = client_msg.subscription_type {
                    // Remove subscription from client
                    {
                        let mut clients_guard = clients.write().await;
                        if let Some(client) = clients_guard.get_mut(&session_id) {
                            client.subscriptions.retain(|s| s != &sub_type);
                        }
                    }

                    // Remove client from subscription mapping
                    {
                        let mut subs_guard = subscriptions.write().await;
                        if let Some(client_list) = subs_guard.get_mut(&sub_type) {
                            client_list.retain(|&id| id != session_id);
                        }
                    }

                    info!("Client {} unsubscribed from {:?}", session_id, sub_type);
                }
            }
            "heartbeat" => {
                // Update client heartbeat
                let mut clients_guard = clients.write().await;
                if let Some(client) = clients_guard.get_mut(&session_id) {
                    client.last_heartbeat = Utc::now();
                }
            }
            _ => {
                warn!("Unknown client action: {}", client_msg.action);
            }
        }

        Ok(())
    }

    /// Check if a message should be sent to a specific client based on subscriptions
    fn should_send_message(message: &StreamMessage, client: &WebSocketClient) -> bool {
        match message {
            StreamMessage::RiskUpdate { position_id, .. } => {
                client.subscriptions.contains(&SubscriptionType::PositionRisk(*position_id))
            }
            StreamMessage::AlertNotification { alert: _alert, .. } => {
                if let Some(user_addr) = &client.user_address {
                    client.subscriptions.contains(&SubscriptionType::UserAlerts(user_addr.clone()))
                } else {
                    false
                }
            }
            StreamMessage::PositionUpdate { position_id, .. } => {
                client.subscriptions.contains(&SubscriptionType::PositionValue(*position_id))
            }
            StreamMessage::MarketUpdate { token_address, .. } => {
                client.subscriptions.contains(&SubscriptionType::MarketData(token_address.clone()))
            }
            StreamMessage::SystemStatus { .. } => {
                client.subscriptions.contains(&SubscriptionType::SystemStatus)
            }
            StreamMessage::Connected { .. } | StreamMessage::Heartbeat { .. } => true,
        }
    }

    /// Broadcast a message to all subscribed clients
    pub async fn broadcast(&self, message: StreamMessage) -> Result<(), AppError> {
        match self.broadcast_tx.send(message.clone()) {
            Ok(_) => Ok(()),
            Err(tokio::sync::broadcast::error::SendError(_)) => {
                // No receivers - this is normal during startup or when no clients are connected
                // Only log as debug to avoid noise
                tracing::debug!("No WebSocket clients connected to receive broadcast");
                Ok(()) // Don't treat this as an error
            }
        }
    }

    /// Send a risk update for a specific position
    pub async fn send_risk_update(
        &self,
        position_id: Uuid,
        risk_metrics: RiskMetrics,
    ) -> Result<(), AppError> {
        let message = StreamMessage::RiskUpdate {
            position_id,
            risk_metrics,
            timestamp: Utc::now(),
        };
        self.broadcast(message).await
    }

    /// Send an alert notification
    pub async fn send_alert(&self, alert: Alert) -> Result<(), AppError> {
        let message = StreamMessage::AlertNotification {
            alert,
            timestamp: Utc::now(),
        };
        self.broadcast(message).await
    }

    /// Send a position value update
    pub async fn send_position_update(
        &self,
        position_id: Uuid,
        current_value_usd: BigDecimal,
        pnl_usd: BigDecimal,
        impermanent_loss_pct: BigDecimal,
    ) -> Result<(), AppError> {
        let message = StreamMessage::PositionUpdate {
            position_id,
            current_value_usd,
            pnl_usd,
            impermanent_loss_pct,
            timestamp: Utc::now(),
        };
        self.broadcast(message).await
    }

    /// Send market data update
    pub async fn send_market_update(
        &self,
        token_address: String,
        price_usd: BigDecimal,
        price_change_24h: BigDecimal,
        volatility: BigDecimal,
    ) -> Result<(), AppError> {
        let message = StreamMessage::MarketUpdate {
            token_address,
            price_usd,
            price_change_24h,
            volatility,
            timestamp: Utc::now(),
        };
        self.broadcast(message).await
    }

    /// Send system status update
    pub async fn send_system_status(&self, status: String, message: String) -> Result<(), AppError> {
        let msg = StreamMessage::SystemStatus {
            status,
            message,
            timestamp: Utc::now(),
        };
        self.broadcast(msg).await
    }

    /// Get connected clients count
    pub async fn get_connected_clients_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Get subscription statistics
    pub async fn get_subscription_stats(&self) -> HashMap<String, usize> {
        let subscriptions = self.subscriptions.read().await;
        let mut stats = HashMap::new();
        
        for (sub_type, clients) in subscriptions.iter() {
            let key = format!("{:?}", sub_type);
            stats.insert(key, clients.len());
        }
        
        stats
    }

    /// Clean up stale connections (heartbeat timeout)
    pub async fn cleanup_stale_connections(&self, timeout_seconds: i64) {
        let cutoff = Utc::now() - chrono::Duration::seconds(timeout_seconds);
        let mut clients_to_remove = Vec::new();
        
        {
            let clients = self.clients.read().await;
            for (session_id, client) in clients.iter() {
                if client.last_heartbeat < cutoff {
                    clients_to_remove.push(*session_id);
                }
            }
        }
        
        if !clients_to_remove.is_empty() {
            let mut clients = self.clients.write().await;
            for session_id in clients_to_remove {
                clients.remove(&session_id);
                info!("Removed stale WebSocket connection: {}", session_id);
            }
        }
    }

    /// Start heartbeat task to send periodic pings and clean up stale connections
    pub fn start_heartbeat_task(&self) {
        let service = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Send heartbeat
                let heartbeat = StreamMessage::Heartbeat {
                    timestamp: Utc::now(),
                };
                
                if let Err(e) = service.broadcast(heartbeat).await {
                    warn!("Failed to send heartbeat: {}", e);
                }
                
                // Clean up stale connections (5 minute timeout)
                service.cleanup_stale_connections(300).await;
            }
        });
    }
}
