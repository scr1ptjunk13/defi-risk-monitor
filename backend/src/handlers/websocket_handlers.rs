use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::Response,
};
use serde::Deserialize;
use uuid::Uuid;
use tracing::{info, warn};

use crate::AppState;

/// Query parameters for WebSocket connections
#[derive(Debug, Deserialize)]
pub struct WebSocketQuery {
    /// User address for authentication and filtering
    pub user_address: Option<String>,
    /// Auto-subscribe to specific subscription types
    pub auto_subscribe: Option<String>,
}

/// Handle WebSocket connection for position risk streaming
/// GET /ws/positions/{id}/risk-stream?user_address=0x123
pub async fn position_risk_stream(
    ws: WebSocketUpgrade,
    Path(position_id): Path<Uuid>,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection request for position risk: {}", position_id);

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        // Auto-subscribe to position risk updates
        // Note: In a real implementation, you'd want to send a subscription message
        // to the client or handle auto-subscription differently
        info!("Position risk stream established for position: {}", position_id);
    })
}

/// Handle WebSocket connection for live alert feed
/// GET /ws/alerts/live-feed?user_address=0x123
pub async fn alerts_live_feed(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection request for alerts live feed");

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        info!("Alerts live feed established for user: {:?}", params.user_address);
    })
}

/// Handle WebSocket connection for position value updates
/// GET /ws/positions/{id}/value-stream?user_address=0x123
pub async fn position_value_stream(
    ws: WebSocketUpgrade,
    Path(position_id): Path<Uuid>,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection request for position value: {}", position_id);

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        info!("Position value stream established for position: {}", position_id);
    })
}

/// Handle WebSocket connection for market data streaming
/// GET /ws/market/{token_address}/stream?user_address=0x123
pub async fn market_data_stream(
    ws: WebSocketUpgrade,
    Path(token_address): Path<String>,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection request for market data: {}", token_address);

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        info!("Market data stream established for token: {}", token_address);
    })
}

/// Handle WebSocket connection for system status updates
/// GET /ws/system/status?user_address=0x123
pub async fn system_status_stream(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("WebSocket connection request for system status");

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        info!("System status stream established");
    })
}

/// Handle general WebSocket connection with manual subscription management
/// GET /ws/stream?user_address=0x123
pub async fn general_stream(
    ws: WebSocketUpgrade,
    Query(params): Query<WebSocketQuery>,
    State(state): State<AppState>,
) -> Response {
    info!("General WebSocket connection request");

    ws.on_upgrade(move |socket| async move {
        if let Some(websocket_service) = &state.websocket_service {
            // Handle the connection
            if let Err(e) = websocket_service.handle_connection(socket, params.user_address.clone()).await {
                warn!("WebSocket connection error: {}", e);
                return;
            }
        } else {
            warn!("WebSocket service not available");
            return;
        }

        info!("General WebSocket stream established for user: {:?}", params.user_address);
    })
}
