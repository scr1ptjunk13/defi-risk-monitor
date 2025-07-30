# DeFi Risk Monitor API Documentation

## Overview

The DeFi Risk Monitor provides a comprehensive REST API for managing positions, risk configurations, and real-time notifications. This API is designed for production-grade DeFi applications handling millions of dollars in real funds.

## Base URL

```
https://api.defi-risk-monitor.com/api/v1
```

## Authentication

All API endpoints require authentication. Include your API key in the request headers:

```
Authorization: Bearer YOUR_API_KEY
```

## Response Format

All API responses follow a consistent format:

```json
{
  "success": boolean,
  "data": object | array | null,
  "message": string | null
}
```

## Error Handling

Error responses include appropriate HTTP status codes and descriptive messages:

```json
{
  "success": false,
  "data": null,
  "message": "Error description"
}
```

Common HTTP status codes:
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `404` - Not Found
- `500` - Internal Server Error

---

## Position Management API

### Create Position

Create a new liquidity position with automatic entry price tracking.

**Endpoint:** `POST /positions`

**Request Body:**
```json
{
  "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
  "protocol": "uniswap_v3",
  "pool_address": "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
  "token0_address": "0xa0b86a33e6c3e0c5d4e5c5e5c5e5c5e5c5e5c5e5",
  "token1_address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
  "token0_amount": "1000.50",
  "token1_amount": "0.5",
  "liquidity": "1500000000000000000",
  "tick_lower": -887220,
  "tick_upper": 887220,
  "fee_tier": 3000,
  "chain_id": 1
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "position": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
      "protocol": "uniswap_v3",
      "pool_address": "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
      "token0_address": "0xa0b86a33e6c3e0c5d4e5c5e5c5e5c5e5c5e5c5e5",
      "token1_address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
      "token0_amount": "1000.50",
      "token1_amount": "0.5",
      "liquidity": "1500000000000000000",
      "tick_lower": -887220,
      "tick_upper": 887220,
      "fee_tier": 3000,
      "chain_id": 1,
      "entry_token0_price_usd": "1.00",
      "entry_token1_price_usd": "3200.00",
      "entry_timestamp": "2024-01-15T10:30:00Z",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  },
  "message": "Position created successfully"
}
```

### Get Positions

Retrieve positions with filtering and pagination.

**Endpoint:** `GET /positions`

**Query Parameters:**
- `user_address` (optional) - Filter by user address
- `protocol` (optional) - Filter by protocol (e.g., "uniswap_v3")
- `chain_id` (optional) - Filter by chain ID
- `page` (optional) - Page number (default: 1)
- `per_page` (optional) - Items per page (default: 50, max: 100)

**Example Request:**
```
GET /positions?user_address=0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8&page=1&per_page=10
```

**Response:**
```json
{
  "success": true,
  "data": {
    "positions": [
      {
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
        "protocol": "uniswap_v3",
        "token0_amount": "1000.50",
        "token1_amount": "0.5",
        "created_at": "2024-01-15T10:30:00Z"
      }
    ],
    "total": 1,
    "page": 1,
    "per_page": 10
  },
  "message": null
}
```

### Get Position by ID

**Endpoint:** `GET /positions/{id}`

**Response:**
```json
{
  "success": true,
  "data": {
    "position": {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
      "protocol": "uniswap_v3",
      "pool_address": "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
      "token0_amount": "1000.50",
      "token1_amount": "0.5",
      "liquidity": "1500000000000000000",
      "entry_token0_price_usd": "1.00",
      "entry_token1_price_usd": "3200.00",
      "created_at": "2024-01-15T10:30:00Z",
      "updated_at": "2024-01-15T10:30:00Z"
    }
  },
  "message": null
}
```

### Update Position

**Endpoint:** `PUT /positions/{id}`

**Request Body:**
```json
{
  "token0_amount": "1200.75",
  "token1_amount": "0.6",
  "liquidity": "1800000000000000000"
}
```

### Delete Position

**Endpoint:** `DELETE /positions/{id}`

**Response:**
```json
{
  "success": true,
  "data": null,
  "message": "Position deleted successfully"
}
```

### Get Position Statistics

**Endpoint:** `GET /positions/stats`

**Query Parameters:**
- `user_address` (optional) - Filter by user address

**Response:**
```json
{
  "success": true,
  "data": {
    "total_positions": 25,
    "total_value_usd": "125000.50",
    "protocols": {
      "uniswap_v3": 15,
      "curve": 10
    },
    "chains": {
      "1": 20,
      "137": 5
    }
  },
  "message": null
}
```

---

## Risk Explainability API

The Risk Explainability API provides advanced analytics and transparent explanations for position risk assessments, enabling users to understand the factors driving their risk scores.

### Explain Position Risk

Get comprehensive explainable risk analysis for a specific position.

**Endpoint:** `GET /positions/{id}/explain-risk`

**Query Parameters:**
- `include_recommendations` (optional, boolean) - Include risk mitigation recommendations (default: true)
- `include_market_context` (optional, boolean) - Include market context analysis (default: true)
- `confidence_threshold` (optional, number) - Minimum confidence score for risk factors (default: 0.7)

**Example Request:**
```
GET /positions/550e8400-e29b-41d4-a716-446655440000/explain-risk?include_recommendations=true
```

**Response:**
```json
{
  "success": true,
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "overall_risk_score": "0.72",
    "risk_level": "High",
    "confidence_score": "0.85",
    "primary_risk_factors": [
      {
        "factor_type": "Impermanent Loss",
        "severity": "High",
        "score": "0.78",
        "explanation": "Current impermanent loss of 12.5% due to significant price divergence between ETH and USDC",
        "confidence": "0.92"
      },
      {
        "factor_type": "Liquidity Risk",
        "severity": "Medium",
        "score": "0.65",
        "explanation": "Pool TVL has decreased by 35% in the last 24 hours, indicating potential liquidity concerns",
        "confidence": "0.88"
      }
    ],
    "secondary_risk_factors": [
      {
        "factor_type": "Protocol Risk",
        "severity": "Low",
        "score": "0.25",
        "explanation": "Uniswap V3 has strong security track record with recent audit completion",
        "confidence": "0.95"
      }
    ],
    "recommendations": [
      {
        "priority": "High",
        "action": "Consider reducing position size",
        "rationale": "High impermanent loss risk due to volatile price movements",
        "impact": "Could reduce overall risk by 30-40%"
      },
      {
        "priority": "Medium",
        "action": "Monitor liquidity levels closely",
        "rationale": "TVL decline may affect position exit capabilities",
        "impact": "Improved exit strategy preparation"
      }
    ],
    "market_context": {
      "market_volatility": "High",
      "trend_direction": "Bearish",
      "correlation_risk": "0.45",
      "external_factors": [
        "Federal Reserve meeting scheduled this week",
        "Major protocol upgrade pending"
      ]
    },
    "position_context": {
      "time_in_position": "7 days",
      "entry_vs_current_price": "-8.5%",
      "performance_vs_hodl": "-3.2%",
      "fee_earnings": "0.15%"
    },
    "summary": "Position shows high risk primarily due to impermanent loss from ETH/USDC price divergence. Consider position size reduction and close monitoring of liquidity conditions.",
    "last_updated": "2024-01-15T15:30:00Z"
  },
  "message": null
}
```

### Get Risk Summary

Get a concise risk summary for a position.

**Endpoint:** `GET /positions/{id}/risk-summary`

**Response:**
```json
{
  "success": true,
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "overall_risk_score": "0.72",
    "risk_level": "High",
    "primary_concern": "Impermanent Loss",
    "key_metrics": {
      "impermanent_loss_pct": "12.5",
      "liquidity_risk_score": "0.65",
      "volatility_score": "0.78"
    },
    "trend": "Increasing",
    "last_updated": "2024-01-15T15:30:00Z"
  },
  "message": null
}
```

### Get Risk Recommendations

Get specific risk mitigation recommendations for a position.

**Endpoint:** `GET /positions/{id}/recommendations`

**Query Parameters:**
- `priority_filter` (optional) - Filter by priority: "High", "Medium", "Low"
- `max_recommendations` (optional, number) - Maximum number of recommendations (default: 10)

**Response:**
```json
{
  "success": true,
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "recommendations": [
      {
        "id": "rec_001",
        "priority": "High",
        "category": "Position Management",
        "action": "Reduce position size by 50%",
        "rationale": "High impermanent loss risk exceeds risk tolerance",
        "expected_impact": "Risk reduction of 35-45%",
        "implementation_difficulty": "Easy",
        "estimated_cost": "Gas fees: ~$15-25"
      },
      {
        "id": "rec_002",
        "priority": "Medium",
        "category": "Monitoring",
        "action": "Set up automated alerts for TVL drops >20%",
        "rationale": "Early warning for liquidity risk escalation",
        "expected_impact": "Improved risk response time",
        "implementation_difficulty": "Easy",
        "estimated_cost": "Free"
      }
    ],
    "total_recommendations": 2,
    "risk_reduction_potential": "40-50%",
    "last_updated": "2024-01-15T15:30:00Z"
  },
  "message": null
}
```

### Get Market Context

Get market context and external factors affecting position risk.

**Endpoint:** `GET /positions/{id}/market-context`

**Response:**
```json
{
  "success": true,
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "market_conditions": {
      "overall_sentiment": "Bearish",
      "volatility_regime": "High",
      "trend_strength": "Strong",
      "market_phase": "Risk-off"
    },
    "token_analysis": {
      "token0": {
        "symbol": "ETH",
        "price_trend": "Declining",
        "volatility_24h": "8.5%",
        "volume_change": "+15%"
      },
      "token1": {
        "symbol": "USDC",
        "price_trend": "Stable",
        "volatility_24h": "0.1%",
        "volume_change": "+5%"
      }
    },
    "correlation_analysis": {
      "eth_btc_correlation": "0.85",
      "risk_on_correlation": "0.72",
      "defi_sector_correlation": "0.68"
    },
    "external_factors": [
      {
        "type": "Macroeconomic",
        "event": "Federal Reserve Meeting",
        "impact_level": "High",
        "timeline": "This week"
      },
      {
        "type": "Protocol",
        "event": "Uniswap V4 Launch",
        "impact_level": "Medium",
        "timeline": "Next month"
      }
    ],
    "risk_environment": "Elevated risk due to macro uncertainty and high volatility regime",
    "last_updated": "2024-01-15T15:30:00Z"
  },
  "message": null
}
```

---

## User Risk Configuration API

### Create Risk Configuration

**Endpoint:** `POST /risk-configs`

**Request Body:**
```json
{
  "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
  "profile_name": "Conservative Trading",
  "risk_tolerance_level": "Conservative",
  "custom_weights": {
    "liquidity_risk_weight": "0.25",
    "volatility_risk_weight": "0.20",
    "protocol_risk_weight": "0.15",
    "mev_risk_weight": "0.10",
    "cross_chain_risk_weight": "0.30"
  },
  "custom_thresholds": {
    "min_tvl_threshold": "1000000",
    "max_slippage_tolerance": "0.05",
    "high_volatility_threshold": "0.15",
    "overall_risk_threshold": "0.70"
  }
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
    "profile_name": "Conservative Trading",
    "is_active": true,
    "risk_tolerance_level": "Conservative",
    "weights": {
      "liquidity_risk_weight": "0.25",
      "volatility_risk_weight": "0.20",
      "protocol_risk_weight": "0.15",
      "mev_risk_weight": "0.10",
      "cross_chain_risk_weight": "0.30"
    },
    "thresholds": {
      "min_tvl_threshold": "1000000",
      "max_slippage_tolerance": "0.05",
      "high_volatility_threshold": "0.15",
      "overall_risk_threshold": "0.70"
    },
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-15T10:30:00Z"
  },
  "message": "Risk configuration created successfully"
}
```

### Get Risk Configurations

**Endpoint:** `GET /risk-configs`

**Query Parameters:**
- `user_address` (required) - User's Ethereum address
- `include_inactive` (optional) - Include inactive configurations

### Activate Risk Configuration

**Endpoint:** `PUT /risk-configs/{id}/activate`

### Get Risk Parameters for Calculation

**Endpoint:** `GET /risk-configs/{user_address}/params`

**Response:**
```json
{
  "success": true,
  "data": {
    "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
    "active_profile": "Conservative Trading",
    "parameters": {
      "liquidity_risk_weight": "0.25",
      "volatility_risk_weight": "0.20",
      "protocol_risk_weight": "0.15",
      "mev_risk_weight": "0.10",
      "cross_chain_risk_weight": "0.30",
      "min_tvl_threshold": "1000000",
      "max_slippage_tolerance": "0.05",
      "overall_risk_threshold": "0.70"
    }
  },
  "message": null
}
```

---

## Webhook API

### Create Webhook

Set up real-time push notifications for risk events.

**Endpoint:** `POST /webhooks`

**Request Body:**
```json
{
  "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
  "endpoint_url": "https://your-app.com/webhooks/defi-risk",
  "secret": "your-webhook-secret-key",
  "event_types": [
    "PositionCreated",
    "RiskThresholdExceeded",
    "LiquidityRiskAlert",
    "ImpermanentLossAlert"
  ],
  "timeout_seconds": 30,
  "max_retries": 3
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "webhook": {
      "id": "550e8400-e29b-41d4-a716-446655440002",
      "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
      "endpoint_url": "https://your-app.com/webhooks/defi-risk",
      "event_types": [
        "PositionCreated",
        "RiskThresholdExceeded",
        "LiquidityRiskAlert",
        "ImpermanentLossAlert"
      ],
      "is_active": true,
      "max_retries": 3,
      "timeout_seconds": 30,
      "created_at": "2024-01-15T10:30:00Z"
    }
  },
  "message": "Webhook created successfully"
}
```

### Webhook Event Types

**Endpoint:** `GET /webhooks/event-types`

**Response:**
```json
{
  "success": true,
  "data": [
    "PositionCreated",
    "PositionUpdated", 
    "PositionDeleted",
    "RiskThresholdExceeded",
    "LiquidityRiskAlert",
    "VolatilityAlert",
    "ProtocolRiskAlert",
    "MevRiskAlert",
    "CrossChainRiskAlert",
    "SystemHealthAlert",
    "PriceAlert",
    "ImpermanentLossAlert"
  ],
  "message": null
}
```

### Test Webhook

**Endpoint:** `POST /webhooks/{id}/test`

**Request Body:**
```json
{
  "event_type": "RiskThresholdExceeded",
  "test_data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "risk_score": 0.85,
    "threshold": 0.70,
    "risk_type": "liquidity_risk"
  }
}
```

### Webhook Payload Format

When events occur, webhooks receive payloads in this format:

```json
{
  "event_type": "RiskThresholdExceeded",
  "event_id": "550e8400-e29b-41d4-a716-446655440003",
  "timestamp": "2024-01-15T10:30:00Z",
  "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
  "data": {
    "position_id": "550e8400-e29b-41d4-a716-446655440000",
    "risk_metrics": {
      "overall_risk_score": 0.85,
      "liquidity_risk": 0.75,
      "volatility_risk": 0.60,
      "protocol_risk": 0.30,
      "impermanent_loss_risk": 0.45
    },
    "threshold_type": "overall_risk",
    "threshold_value": 0.70
  },
  "signature": "sha256=a8b7c9d2e3f4g5h6i7j8k9l0m1n2o3p4q5r6s7t8u9v0w1x2y3z4"
}
```

### Webhook Security

Verify webhook authenticity using the signature:

```python
import hmac
import hashlib
import json

def verify_webhook(payload, signature, secret):
    payload_str = json.dumps(payload, separators=(',', ':'))
    expected_signature = hmac.new(
        secret.encode(),
        payload_str.encode(),
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(f"sha256={expected_signature}", signature)
```

---

## Risk Calculation API

### Calculate Position Risk

**Endpoint:** `POST /risk/calculate`

**Request Body:**
```json
{
  "position_id": "550e8400-e29b-41d4-a716-446655440000",
  "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "risk_metrics": {
      "overall_risk_score": 0.65,
      "liquidity_risk": 0.45,
      "volatility_risk": 0.70,
      "protocol_risk": 0.25,
      "mev_risk": 0.30,
      "cross_chain_risk": 0.20,
      "impermanent_loss_risk": 0.55,
      "tvl_risk": 0.35,
      "slippage_risk": 0.40,
      "thin_pool_risk": 0.25,
      "max_estimated_slippage": "0.025"
    },
    "risk_factors": {
      "high_risk_factors": ["volatility", "impermanent_loss"],
      "medium_risk_factors": ["liquidity", "slippage"],
      "low_risk_factors": ["protocol", "mev"]
    },
    "recommendations": [
      "Consider reducing position size due to high volatility",
      "Monitor impermanent loss closely",
      "Set stop-loss at 15% IL threshold"
    ],
    "calculated_at": "2024-01-15T10:30:00Z"
  },
  "message": null
}
```

---

## Code Examples

### JavaScript/Node.js

```javascript
const axios = require('axios');

class DeFiRiskMonitorAPI {
  constructor(apiKey, baseURL = 'https://api.defi-risk-monitor.com/api/v1') {
    this.apiKey = apiKey;
    this.baseURL = baseURL;
    this.client = axios.create({
      baseURL: this.baseURL,
      headers: {
        'Authorization': `Bearer ${this.apiKey}`,
        'Content-Type': 'application/json'
      }
    });
  }

  async createPosition(positionData) {
    try {
      const response = await this.client.post('/positions', positionData);
      return response.data;
    } catch (error) {
      throw new Error(`Failed to create position: ${error.response?.data?.message || error.message}`);
    }
  }

  async getPositions(userAddress, options = {}) {
    try {
      const params = { user_address: userAddress, ...options };
      const response = await this.client.get('/positions', { params });
      return response.data;
    } catch (error) {
      throw new Error(`Failed to get positions: ${error.response?.data?.message || error.message}`);
    }
  }

  async createWebhook(webhookData) {
    try {
      const response = await this.client.post('/webhooks', webhookData);
      return response.data;
    } catch (error) {
      throw new Error(`Failed to create webhook: ${error.response?.data?.message || error.message}`);
    }
  }

  async calculateRisk(positionId, userAddress) {
    try {
      const response = await this.client.post('/risk/calculate', {
        position_id: positionId,
        user_address: userAddress
      });
      return response.data;
    } catch (error) {
      throw new Error(`Failed to calculate risk: ${error.response?.data?.message || error.message}`);
    }
  }
}

// Usage example
const api = new DeFiRiskMonitorAPI('your-api-key');

async function example() {
  // Create a position
  const position = await api.createPosition({
    user_address: '0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8',
    protocol: 'uniswap_v3',
    pool_address: '0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640',
    token0_address: '0xa0b86a33e6c3e0c5d4e5c5e5c5e5c5e5c5e5c5e5',
    token1_address: '0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2',
    token0_amount: '1000.50',
    token1_amount: '0.5',
    liquidity: '1500000000000000000',
    tick_lower: -887220,
    tick_upper: 887220,
    fee_tier: 3000,
    chain_id: 1
  });

  console.log('Position created:', position);

  // Set up webhook for risk alerts
  const webhook = await api.createWebhook({
    user_address: '0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8',
    endpoint_url: 'https://your-app.com/webhooks/defi-risk',
    secret: 'your-webhook-secret',
    event_types: ['RiskThresholdExceeded', 'LiquidityRiskAlert']
  });

  console.log('Webhook created:', webhook);
}
```

### Python

```python
import requests
import json
from typing import Dict, List, Optional

class DeFiRiskMonitorAPI:
    def __init__(self, api_key: str, base_url: str = "https://api.defi-risk-monitor.com/api/v1"):
        self.api_key = api_key
        self.base_url = base_url
        self.headers = {
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json"
        }

    def create_position(self, position_data: Dict) -> Dict:
        response = requests.post(
            f"{self.base_url}/positions",
            headers=self.headers,
            json=position_data
        )
        response.raise_for_status()
        return response.json()

    def get_positions(self, user_address: str, **kwargs) -> Dict:
        params = {"user_address": user_address, **kwargs}
        response = requests.get(
            f"{self.base_url}/positions",
            headers=self.headers,
            params=params
        )
        response.raise_for_status()
        return response.json()

    def create_webhook(self, webhook_data: Dict) -> Dict:
        response = requests.post(
            f"{self.base_url}/webhooks",
            headers=self.headers,
            json=webhook_data
        )
        response.raise_for_status()
        return response.json()

    def calculate_risk(self, position_id: str, user_address: str) -> Dict:
        response = requests.post(
            f"{self.base_url}/risk/calculate",
            headers=self.headers,
            json={
                "position_id": position_id,
                "user_address": user_address
            }
        )
        response.raise_for_status()
        return response.json()

# Usage example
api = DeFiRiskMonitorAPI("your-api-key")

# Create position
position = api.create_position({
    "user_address": "0x742d35cc6bf8c7d7b8f8e3e8e8e8e8e8e8e8e8e8",
    "protocol": "uniswap_v3",
    "pool_address": "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640",
    "token0_address": "0xa0b86a33e6c3e0c5d4e5c5e5c5e5c5e5c5e5c5e5",
    "token1_address": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
    "token0_amount": "1000.50",
    "token1_amount": "0.5",
    "liquidity": "1500000000000000000",
    "tick_lower": -887220,
    "tick_upper": 887220,
    "fee_tier": 3000,
    "chain_id": 1
})

print("Position created:", position)
```

---

## Rate Limits

- **Standard tier:** 1,000 requests per hour
- **Premium tier:** 10,000 requests per hour
- **Enterprise tier:** Unlimited

Rate limit headers are included in responses:
- `X-RateLimit-Limit` - Request limit per hour
- `X-RateLimit-Remaining` - Remaining requests
- `X-RateLimit-Reset` - Reset time (Unix timestamp)

---

## Support

For API support and questions:
- **Documentation:** [https://docs.defi-risk-monitor.com](https://docs.defi-risk-monitor.com)
- **Support Email:** api-support@defi-risk-monitor.com
- **Discord:** [DeFi Risk Monitor Community](https://discord.gg/defi-risk-monitor)

---

## Changelog

### v1.0.0 (2024-01-15)
- Initial API release
- Position management endpoints
- Risk configuration API
- Webhook system for real-time notifications
- Comprehensive risk calculation engine
- Production-grade security and monitoring
