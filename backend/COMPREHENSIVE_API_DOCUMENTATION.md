# DeFi Risk Monitor - Comprehensive REST API Documentation

## Overview

This document provides complete documentation for all REST API endpoints in the DeFi Risk Monitor system. The API is built with Axum and provides comprehensive coverage for all services including authentication, position management, risk assessment, portfolio analytics, system health monitoring, and more.

## Base URL
```
http://localhost:8080/api/v1
```

## Authentication

Most endpoints require authentication. Include the user's wallet address or authentication token in the request headers.

```http
Authorization: Bearer <token>
X-User-Address: <wallet_address>
```

---

## 1. Authentication & User Management

### Create User
```http
POST /api/v1/users
Content-Type: application/json

{
  "username": "string",
  "email": "string",
  "wallet_address": "string",
  "chain_id": 1
}
```

### Get User by ID
```http
GET /api/v1/users/{user_id}
```

### Get User by Wallet Address
```http
GET /api/v1/users/address/{wallet_address}
```

### Update User Settings
```http
PUT /api/v1/users/{user_id}/settings
Content-Type: application/json

{
  "notifications_enabled": true,
  "email_alerts": true,
  "risk_tolerance": "moderate",
  "preferred_currency": "USD",
  "dashboard_layout": {}
}
```

### Get User Portfolio Summary
```http
GET /api/v1/users/{user_id}/portfolio-summary
```

### Get User Risk Preferences
```http
GET /api/v1/users/{user_id}/risk-preferences
```

---

## 2. Position Management

### Create Position
```http
POST /api/v1/positions
Content-Type: application/json

{
  "user_id": "uuid",
  "protocol": "uniswap-v3",
  "pool_address": "0x...",
  "chain_id": 1,
  "token0_address": "0x...",
  "token1_address": "0x...",
  "position_type": "liquidity_provision",
  "entry_price": "1000.50",
  "amount_usd": "5000.00",
  "liquidity_amount": "1000000",
  "fee_tier": 3000,
  "tick_lower": -276320,
  "tick_upper": -276300
}
```

### Get Position
```http
GET /api/v1/positions/{position_id}
```

### Update Position
```http
PUT /api/v1/positions/{position_id}
Content-Type: application/json

{
  "amount_usd": "6000.00",
  "liquidity_amount": "1200000",
  "is_active": true
}
```

### Delete Position
```http
DELETE /api/v1/positions/{position_id}
```

### List Positions
```http
GET /api/v1/positions?user_id={uuid}&protocol=uniswap-v3&chain_id=1&is_active=true&page=1&limit=20
```

### Get Position Statistics
```http
GET /api/v1/positions/stats?user_id={uuid}&protocol=uniswap-v3
```

---

## 3. Risk Assessment & Analytics

### Create Risk Assessment
```http
POST /api/v1/risk-assessments
Content-Type: application/json

{
  "entity_id": "uuid",
  "entity_type": "position",
  "risk_type": "impermanent_loss",
  "risk_score": "0.75",
  "severity": "medium",
  "description": "Moderate impermanent loss risk detected",
  "metadata": {},
  "expires_at": "2024-12-31T23:59:59Z"
}
```

### Get Risk Assessment
```http
GET /api/v1/risk-assessments/{assessment_id}
```

### Update Risk Assessment
```http
PUT /api/v1/risk-assessments/{assessment_id}
Content-Type: application/json

{
  "risk_score": "0.85",
  "severity": "high",
  "description": "Updated risk assessment"
}
```

### List Risk Assessments
```http
GET /api/v1/risk-assessments?entity_id={uuid}&risk_type=impermanent_loss&severity=high&page=1&limit=50
```

### Get Risk Trends
```http
GET /api/v1/risk-trends?entity_id={uuid}&risk_type=impermanent_loss&granularity=daily&start_date=2024-01-01T00:00:00Z&end_date=2024-12-31T23:59:59Z
```

### Get Risk Correlation Matrix
```http
GET /api/v1/risk-correlation?entity_ids={uuid1},{uuid2},{uuid3}&risk_type=impermanent_loss
```

### Get Risk Distribution
```http
GET /api/v1/risk-distribution?entity_type=position&risk_type=impermanent_loss
```

### Get MEV Risk Analysis
```http
GET /api/v1/mev-risk/{pool_address}/{chain_id}
```

### Get Cross-Chain Risk Assessment
```http
GET /api/v1/cross-chain-risk/{user_id}
```

### Get Protocol Risk Assessment
```http
GET /api/v1/protocol-risk/{protocol_name}
```

---

## 4. Portfolio Analytics

### Get Portfolio Performance
```http
GET /api/v1/portfolio/performance?user_id={uuid}&period_days=30&start_date=2024-01-01T00:00:00Z&end_date=2024-12-31T23:59:59Z
```

### Get P&L History
```http
GET /api/v1/portfolio/pnl-history?user_id={uuid}&granularity=daily&start_date=2024-01-01T00:00:00Z&end_date=2024-12-31T23:59:59Z
```

### Get Asset Allocation
```http
GET /api/v1/portfolio/asset-allocation?user_id={uuid}&include_inactive=false
```

### Get Protocol Exposure
```http
GET /api/v1/portfolio/protocol-exposure?user_id={uuid}&include_risk_metrics=true
```

---

## 5. System Health & Monitoring

### Get System Health Overview
```http
GET /api/v1/system/health
```

### Get Database Metrics
```http
GET /api/v1/system/health/database?start_date=2024-01-01T00:00:00Z&include_historical=true
```

### Get Query Performance Stats
```http
GET /api/v1/system/health/query-performance
```

### Get Connection Pool Health
```http
GET /api/v1/system/health/connection-pool
```

### Get Table Sizes
```http
GET /api/v1/system/health/table-sizes?schema_name=public&min_size_mb=10&sort_by=size
```

### Get Health Alerts
```http
GET /api/v1/system/health/alerts
```

### Trigger Maintenance
```http
POST /api/v1/system/maintenance/{table_name}
```

---

## 6. Monitoring & Alerting

### Create Threshold
```http
POST /api/v1/monitoring/thresholds
Content-Type: application/json

{
  "user_id": "uuid",
  "position_id": "uuid",
  "protocol": "uniswap-v3",
  "threshold_type": "impermanent_loss",
  "operator": "gt",
  "value": "0.05",
  "severity": "medium",
  "is_enabled": true,
  "notification_channels": ["email", "webhook"]
}
```

### Get Threshold
```http
GET /api/v1/monitoring/thresholds/{threshold_id}
```

### Update Threshold
```http
PUT /api/v1/monitoring/thresholds/{threshold_id}
Content-Type: application/json

{
  "value": "0.10",
  "severity": "high",
  "is_enabled": true
}
```

### List Thresholds
```http
GET /api/v1/monitoring/thresholds?user_id={uuid}&position_id={uuid}&threshold_type=impermanent_loss&is_enabled=true&page=1&limit=50
```

### List Alerts
```http
GET /api/v1/monitoring/alerts?user_id={uuid}&severity=high&is_resolved=false&page=1&limit=50
```

### Resolve Alert
```http
PUT /api/v1/monitoring/alerts/{alert_id}/resolve
Content-Type: application/json

{
  "resolution_note": "Issue resolved by user action"
}
```

### Get Monitoring Statistics
```http
GET /api/v1/monitoring/stats
```

### Start/Stop Monitoring
```http
POST /api/v1/monitoring/start
POST /api/v1/monitoring/stop
```

---

## 7. Price Feed & Validation

### Get Token Price
```http
GET /api/v1/prices/{token_address}?force_refresh=false&include_metadata=true
```

### Get Multiple Token Prices
```http
POST /api/v1/prices/batch
Content-Type: application/json

{
  "token_addresses": ["0x...", "0x..."],
  "force_refresh": false
}
```

### Get Price History
```http
GET /api/v1/prices/{token_address}/history?start_date=2024-01-01T00:00:00Z&end_date=2024-12-31T23:59:59Z&granularity=daily&limit=100
```

### Validate Token Price
```http
GET /api/v1/prices/{token_address}/validate?threshold_percentage=10&min_sources=2
```

### Refresh Price Cache
```http
POST /api/v1/prices/{token_address}/refresh
```

### Get Supported Tokens
```http
GET /api/v1/prices/supported-tokens
```

### Get Price Sources
```http
GET /api/v1/prices/sources
```

---

## 8. WebSocket Endpoints

### Real-time Position Risk Stream
```
WS /ws/positions/{id}/risk-stream
```

### Live Alerts Feed
```
WS /ws/alerts/live-feed
```

### Position Value Stream
```
WS /ws/positions/{id}/value-stream
```

### Market Data Stream
```
WS /ws/market/{token_address}/stream
```

### System Status Stream
```
WS /ws/system/status
```

---

## Response Formats

### Success Response
```json
{
  "data": { ... },
  "timestamp": "2024-01-01T00:00:00Z",
  "request_id": "uuid"
}
```

### Error Response
```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input parameters",
    "details": { ... }
  },
  "timestamp": "2024-01-01T00:00:00Z",
  "request_id": "uuid"
}
```

### Paginated Response
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 100,
    "total_pages": 5
  },
  "timestamp": "2024-01-01T00:00:00Z"
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| `VALIDATION_ERROR` | Invalid input parameters |
| `AUTHENTICATION_ERROR` | Authentication failed |
| `AUTHORIZATION_ERROR` | Insufficient permissions |
| `NOT_FOUND` | Resource not found |
| `CONFLICT` | Resource conflict |
| `RATE_LIMIT_EXCEEDED` | Too many requests |
| `INTERNAL_ERROR` | Internal server error |
| `DATABASE_ERROR` | Database operation failed |
| `EXTERNAL_API_ERROR` | External service error |

---

## Rate Limits

- **General API**: 1000 requests per hour per user
- **Price Feed API**: 100 requests per minute per user
- **WebSocket Connections**: 10 concurrent connections per user
- **Batch Operations**: 50 items per request maximum

---

## Examples

### Complete Position Management Workflow

1. **Create a position:**
```bash
curl -X POST http://localhost:8080/api/v1/positions \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "protocol": "uniswap-v3",
    "pool_address": "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
    "chain_id": 1,
    "token0_address": "0xA0b86a33E6441b8e7a2e2B7b5b7c6e5a5c5d5e5f",
    "token1_address": "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    "position_type": "liquidity_provision",
    "entry_price": "2500.00",
    "amount_usd": "10000.00"
  }'
```

2. **Set up risk monitoring:**
```bash
curl -X POST http://localhost:8080/api/v1/monitoring/thresholds \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "position_id": "position-uuid-here",
    "threshold_type": "impermanent_loss",
    "operator": "gt",
    "value": "0.05",
    "severity": "medium",
    "is_enabled": true,
    "notification_channels": ["email"]
  }'
```

3. **Monitor portfolio performance:**
```bash
curl "http://localhost:8080/api/v1/portfolio/performance?user_id=550e8400-e29b-41d4-a716-446655440000&period_days=30"
```

This comprehensive API provides full coverage for all DeFi risk monitoring operations, from basic CRUD operations to advanced analytics and real-time monitoring.
