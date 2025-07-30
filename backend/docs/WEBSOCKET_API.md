# WebSocket Streaming API Documentation

## Overview

The DeFi Risk Monitor WebSocket API provides real-time streaming of risk updates, alerts, position values, and market data. This enables instant notifications and live dashboard updates without polling.

## Base URL
```
ws://localhost:8080/ws/
```

## Authentication
WebSocket connections accept an optional `user_address` query parameter for user-specific filtering:
```
ws://localhost:8080/ws/alerts/live-feed?user_address=0x1234567890abcdef
```

## WebSocket Endpoints

### 1. Position Risk Stream
**Endpoint:** `/ws/positions/{id}/risk-stream`

Real-time risk updates for a specific position.

**Parameters:**
- `id` (UUID): Position ID to monitor

**Query Parameters:**
- `user_address` (optional): User wallet address for authentication

**Example:**
```
ws://localhost:8080/ws/positions/550e8400-e29b-41d4-a716-446655440000/risk-stream?user_address=0x123
```

**Message Format:**
```json
{
  "type": "RiskUpdate",
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "risk_metrics": {
      "overall_risk_score": 0.75,
      "liquidity_risk": 0.60,
      "volatility_risk": 0.80,
      "protocol_risk": 0.25,
      "mev_risk_score": 0.15,
      "cross_chain_risk": 0.30,
      "impermanent_loss_pct": 12.5,
      "current_il_usd": 1250.00,
      "max_drawdown_pct": 18.2,
      "sharpe_ratio": 1.45,
      "var_95_pct": 22.1
    },
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### 2. Alerts Live Feed
**Endpoint:** `/ws/alerts/live-feed`

Real-time alert notifications for all user alerts.

**Query Parameters:**
- `user_address` (optional): Filter alerts for specific user

**Example:**
```
ws://localhost:8080/ws/alerts/live-feed?user_address=0x123
```

**Message Format:**
```json
{
  "type": "AlertNotification",
  "data": {
    "alert": {
      "id": "alert-uuid",
      "position_id": "position-uuid",
      "alert_type": "impermanent_loss",
      "severity": "Critical",
      "title": "High Impermanent Loss Detected",
      "message": "Position IL exceeded 15% threshold (current: 18.2%)",
      "risk_score": 0.85,
      "current_value": 82000.00,
      "threshold_value": 85000.00,
      "is_resolved": false,
      "created_at": "2025-07-26T05:41:20.123Z"
    },
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### 3. Position Value Stream
**Endpoint:** `/ws/positions/{id}/value-stream`

Real-time position value, P&L, and impermanent loss updates.

**Parameters:**
- `id` (UUID): Position ID to monitor

**Message Format:**
```json
{
  "type": "PositionUpdate",
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "current_value_usd": 95000.00,
    "pnl_usd": -5000.00,
    "impermanent_loss_pct": 8.2,
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### 4. Market Data Stream
**Endpoint:** `/ws/market/{token_address}/stream`

Real-time market data for specific tokens.

**Parameters:**
- `token_address` (string): Token contract address

**Example:**
```
ws://localhost:8080/ws/market/0xA0b86a33E6441b8C7013A0eC5C5b0e2F9C7a0b86/stream
```

**Message Format:**
```json
{
  "type": "MarketUpdate",
  "data": {
    "token_address": "0xA0b86a33E6441b8C7013A0eC5C5b0e2F9C7a0b86",
    "price_usd": 1850.25,
    "price_change_24h": -2.5,
    "volatility": 15.8,
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### 5. System Status Stream
**Endpoint:** `/ws/system/status`

Real-time system health and status updates.

**Message Format:**
```json
{
  "type": "SystemStatus",
  "data": {
    "status": "healthy",
    "message": "All systems operational",
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### 6. General Stream
**Endpoint:** `/ws/stream`

General WebSocket connection with manual subscription management.

**Client-to-Server Messages:**

**Subscribe to Updates:**
```json
{
  "action": "subscribe",
  "subscription_type": {
    "PositionRisk": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

**Unsubscribe:**
```json
{
  "action": "unsubscribe",
  "subscription_type": {
    "UserAlerts": "0x1234567890abcdef"
  }
}
```

**Heartbeat:**
```json
{
  "action": "heartbeat"
}
```

## Subscription Types

### Available Subscription Types:
- `PositionRisk(UUID)` - Risk updates for specific position
- `UserAlerts(String)` - All alerts for user address
- `PositionValue(UUID)` - Value updates for specific position
- `MarketData(String)` - Market data for token address
- `SystemStatus` - System health updates

## Connection Management

### Connection Confirmation
Upon successful connection, clients receive:
```json
{
  "type": "Connected",
  "data": {
    "session_id": "session-uuid",
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### Heartbeat Messages
Server sends periodic heartbeat messages:
```json
{
  "type": "Heartbeat",
  "data": {
    "timestamp": "2025-07-26T05:41:20.123Z"
  }
}
```

### Connection Timeout
- Connections timeout after 5 minutes of inactivity
- Clients should respond to heartbeat messages
- Automatic cleanup of stale connections

## JavaScript Client Example

```javascript
// Connect to position risk stream
const ws = new WebSocket('ws://localhost:8080/ws/positions/550e8400-e29b-41d4-a716-446655440000/risk-stream?user_address=0x123');

ws.onopen = function(event) {
    console.log('WebSocket connected');
};

ws.onmessage = function(event) {
    const message = JSON.parse(event.data);
    
    switch(message.type) {
        case 'Connected':
            console.log('Session ID:', message.data.session_id);
            break;
            
        case 'RiskUpdate':
            console.log('Risk Update:', message.data.risk_metrics);
            updateDashboard(message.data);
            break;
            
        case 'AlertNotification':
            console.log('New Alert:', message.data.alert);
            showNotification(message.data.alert);
            break;
            
        case 'Heartbeat':
            // Respond to heartbeat to maintain connection
            ws.send(JSON.stringify({action: 'heartbeat'}));
            break;
    }
};

ws.onerror = function(error) {
    console.error('WebSocket error:', error);
};

ws.onclose = function(event) {
    console.log('WebSocket closed:', event.code, event.reason);
    // Implement reconnection logic
};

// Subscribe to additional updates
function subscribeToAlerts(userAddress) {
    ws.send(JSON.stringify({
        action: 'subscribe',
        subscription_type: {
            UserAlerts: userAddress
        }
    }));
}
```

## Python Client Example

```python
import asyncio
import websockets
import json

async def risk_monitor_client():
    uri = "ws://localhost:8080/ws/alerts/live-feed?user_address=0x123"
    
    async with websockets.connect(uri) as websocket:
        print("Connected to DeFi Risk Monitor")
        
        async for message in websocket:
            data = json.loads(message)
            
            if data['type'] == 'AlertNotification':
                alert = data['data']['alert']
                print(f"🚨 {alert['severity']}: {alert['title']}")
                print(f"   {alert['message']}")
                
                # Send to notification service
                if alert['severity'] == 'Critical':
                    await send_push_notification(alert)
            
            elif data['type'] == 'Heartbeat':
                # Respond to heartbeat
                await websocket.send(json.dumps({"action": "heartbeat"}))

# Run the client
asyncio.run(risk_monitor_client())
```

## React Hook Example

```javascript
import { useState, useEffect, useRef } from 'react';

export function useWebSocketRisk(positionId, userAddress) {
    const [riskData, setRiskData] = useState(null);
    const [alerts, setAlerts] = useState([]);
    const [connectionStatus, setConnectionStatus] = useState('disconnected');
    const ws = useRef(null);

    useEffect(() => {
        const connectWebSocket = () => {
            const url = `ws://localhost:8080/ws/positions/${positionId}/risk-stream?user_address=${userAddress}`;
            ws.current = new WebSocket(url);

            ws.current.onopen = () => {
                setConnectionStatus('connected');
                console.log('Risk monitor connected');
            };

            ws.current.onmessage = (event) => {
                const message = JSON.parse(event.data);
                
                switch(message.type) {
                    case 'RiskUpdate':
                        setRiskData(message.data.risk_metrics);
                        break;
                    case 'AlertNotification':
                        setAlerts(prev => [message.data.alert, ...prev]);
                        break;
                    case 'Heartbeat':
                        ws.current.send(JSON.stringify({action: 'heartbeat'}));
                        break;
                }
            };

            ws.current.onclose = () => {
                setConnectionStatus('disconnected');
                // Reconnect after 3 seconds
                setTimeout(connectWebSocket, 3000);
            };

            ws.current.onerror = (error) => {
                console.error('WebSocket error:', error);
                setConnectionStatus('error');
            };
        };

        connectWebSocket();

        return () => {
            if (ws.current) {
                ws.current.close();
            }
        };
    }, [positionId, userAddress]);

    return { riskData, alerts, connectionStatus };
}
```

## Error Handling

### Common Error Scenarios:
1. **Connection Refused** - Server not running or wrong URL
2. **Authentication Failed** - Invalid user_address
3. **Position Not Found** - Invalid position ID
4. **Rate Limiting** - Too many connections from same IP
5. **Server Overload** - Temporary service unavailability

### Best Practices:
- Implement exponential backoff for reconnections
- Handle connection drops gracefully
- Validate message formats before processing
- Implement client-side rate limiting
- Use connection pooling for multiple subscriptions

## Performance Considerations

### Server Limits:
- Maximum 1000 concurrent connections per server instance
- Maximum 100 subscriptions per client
- Heartbeat interval: 30 seconds
- Connection timeout: 5 minutes

### Optimization Tips:
- Use specific endpoints instead of general stream when possible
- Unsubscribe from unused streams
- Implement client-side message deduplication
- Use compression for large message payloads
- Batch multiple subscriptions when possible

## Security

### Authentication:
- User address verification (optional but recommended)
- Rate limiting per IP address
- Connection origin validation
- Message size limits

### Data Privacy:
- User-specific data filtering
- No sensitive data in WebSocket URLs
- Secure WebSocket (WSS) in production
- Regular security audits

## Production Deployment

### WebSocket Configuration:
```toml
# In production, use WSS (WebSocket Secure)
WEBSOCKET_URL="wss://api.defi-risk-monitor.com/ws/"
WEBSOCKET_MAX_CONNECTIONS=10000
WEBSOCKET_HEARTBEAT_INTERVAL=30
WEBSOCKET_CONNECTION_TIMEOUT=300
```

### Load Balancing:
- Use sticky sessions for WebSocket connections
- Implement horizontal scaling with Redis pub/sub
- Monitor connection counts and memory usage
- Set up health checks for WebSocket endpoints

### Monitoring:
- Track active connection counts
- Monitor message throughput
- Alert on connection failures
- Log WebSocket errors and reconnections

---

## Summary

The WebSocket Streaming API provides real-time capabilities that transform your DeFi risk monitoring platform from a traditional polling-based system to a modern, reactive application. This enables:

- **Instant risk alerts** during market volatility
- **Live dashboard updates** without page refreshes  
- **Real-time position monitoring** for active traders
- **Immediate notifications** for critical events
- **Enhanced user experience** with responsive interfaces

This real-time streaming capability is a significant competitive advantage that enables premium pricing and attracts institutional clients who require immediate risk notifications.
