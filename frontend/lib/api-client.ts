/**
 * DeFi Risk Monitor API Client
 * 
 * Comprehensive API client for integrating with the Rust backend
 * Provides methods for position management, risk analytics, and real-time monitoring
 */

// API Types
export interface CreatePositionRequest {
  user_address: string;
  pool_address: string;
  token0_address: string;
  token1_address: string;
  token0_symbol: string;
  token1_symbol: string;
  liquidity_amount: string;
  fee_tier: number;
  price_range_lower?: string;
  price_range_upper?: string;
  entry_price_token0?: string;
  entry_price_token1?: string;
}

export interface Position {
  id: string;
  user_address: string;
  pool_address: string;
  token0_address: string;
  token1_address: string;
  token0_symbol: string;
  token1_symbol: string;
  liquidity_amount: string;
  fee_tier: number;
  current_value_usd: string;
  entry_price_token0: string;
  entry_price_token1: string;
  impermanent_loss_pct: string;
  created_at: string;
  updated_at: string;
}

export interface RiskMetrics {
  position_id: string;
  overall_risk_score: string;
  liquidity_risk: string;
  volatility_risk: string;
  protocol_risk: string;
  mev_risk: string;
  cross_chain_risk: string;
  impermanent_loss_risk: string;
  calculated_at: string;
}

export interface RiskExplanation {
  position_id: string;
  overall_assessment: string;
  risk_factors: Array<{
    factor: string;
    score: string;
    explanation: string;
    severity: string;
  }>;
  recommendations: string[];
  market_context: {
    market_conditions: string;
    volatility_outlook: string;
    liquidity_analysis: string;
  };
}

export interface ProtocolEvent {
  id: string;
  protocol_name: string;
  event_type: string;
  severity: string;
  title: string;
  description: string;
  impact_score: string;
  event_timestamp: string;
  created_at: string;
}

export interface EventStats {
  event_types: Array<{ type: string; count: number }>;
  severity_distribution: Array<{ severity: string; count: number }>;
  top_protocols: Array<{ protocol: string; count: number }>;
  total_funds_lost_usd: string;
}

export interface AlertThreshold {
  id: string;
  user_address: string;
  threshold_type: string;
  operator: string;
  value: string;
  enabled: boolean;
  created_at: string;
}

// API Client Class
export class DeFiRiskAPI {
  private baseURL: string;
  private wsUrl: string;

  constructor(baseURL = 'http://localhost:8080/api/v1', wsUrl = 'ws://localhost:8080/ws') {
    this.baseURL = baseURL;
    this.wsUrl = wsUrl;
  }

  // Helper method for API requests
  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseURL}${endpoint}`;
    
    const config: RequestInit = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    };

    try {
      const response = await fetch(url, config);
      
      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(`API Error ${response.status}: ${errorText}`);
      }

      return await response.json();
    } catch (error) {
      console.error(`API request failed for ${endpoint}:`, error);
      throw error;
    }
  }

  // Portfolio Management
  async getPortfolio(userAddress: string): Promise<any> {
    return this.request(`/portfolio?user_address=${userAddress}`);
  }

  // Position Management
  async getPositions(userAddress: string): Promise<Position[]> {
    return this.request<Position[]>(`/positions?user_address=${userAddress}`);
  }

  async createPosition(position: CreatePositionRequest): Promise<Position> {
    return this.request<Position>('/positions', {
      method: 'POST',
      body: JSON.stringify(position),
    });
  }

  async getPosition(positionId: string): Promise<Position> {
    return this.request<Position>(`/positions/${positionId}`);
  }

  async updatePosition(positionId: string, updates: Partial<Position>): Promise<Position> {
    return this.request<Position>(`/positions/${positionId}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deletePosition(positionId: string): Promise<void> {
    return this.request<void>(`/positions/${positionId}`, {
      method: 'DELETE',
    });
  }

  // Risk Analytics
  async getPositionRisk(positionId: string): Promise<RiskMetrics> {
    return this.request<RiskMetrics>(`/positions/${positionId}/risk`);
  }

  async explainRisk(positionId: string): Promise<RiskExplanation> {
    return this.request<RiskExplanation>(`/positions/${positionId}/explain-risk`);
  }

  async getRiskSummary(positionId: string): Promise<any> {
    return this.request<any>(`/positions/${positionId}/risk-summary`);
  }

  async getRiskRecommendations(positionId: string): Promise<any> {
    return this.request<any>(`/positions/${positionId}/recommendations`);
  }

  async getMarketContext(positionId: string): Promise<any> {
    return this.request<any>(`/positions/${positionId}/market-context`);
  }

  // Protocol Events
  async getProtocolEvents(params?: {
    protocol?: string;
    event_type?: string;
    severity?: string;
    from_date?: string;
    to_date?: string;
    page?: number;
    per_page?: number;
  }): Promise<{ events: ProtocolEvent[]; total: number; page: number; per_page: number }> {
    const searchParams = new URLSearchParams();
    
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          searchParams.append(key, value.toString());
        }
      });
    }

    const queryString = searchParams.toString();
    const endpoint = `/protocol-events${queryString ? `?${queryString}` : ''}`;
    
    return this.request<any>(endpoint);
  }

  async getProtocolEvent(eventId: string): Promise<ProtocolEvent> {
    return this.request<ProtocolEvent>(`/protocol-events/${eventId}`);
  }

  async getEventStats(params?: {
    protocol_name?: string;
    from_date?: string;
    to_date?: string;
  }): Promise<EventStats> {
    const searchParams = new URLSearchParams();
    
    if (params) {
      Object.entries(params).forEach(([key, value]) => {
        if (value !== undefined) {
          searchParams.append(key, value.toString());
        }
      });
    }

    const queryString = searchParams.toString();
    const endpoint = `/protocol-events/stats${queryString ? `?${queryString}` : ''}`;
    
    return this.request<EventStats>(endpoint);
  }

  // Alert Management
  async getAlertThresholds(userAddress: string): Promise<AlertThreshold[]> {
    return this.request<AlertThreshold[]>(`/thresholds?user_address=${userAddress}`);
  }

  async createAlertThreshold(threshold: Omit<AlertThreshold, 'id' | 'created_at'>): Promise<AlertThreshold> {
    return this.request<AlertThreshold>('/thresholds', {
      method: 'POST',
      body: JSON.stringify(threshold),
    });
  }

  async updateAlertThreshold(thresholdId: string, updates: Partial<AlertThreshold>): Promise<AlertThreshold> {
    return this.request<AlertThreshold>(`/thresholds/${thresholdId}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    });
  }

  async deleteAlertThreshold(thresholdId: string): Promise<void> {
    return this.request<void>(`/thresholds/${thresholdId}`, {
      method: 'DELETE',
    });
  }

  async initializeDefaultThresholds(userAddress: string): Promise<AlertThreshold[]> {
    return this.request<AlertThreshold[]>('/thresholds/defaults', {
      method: 'POST',
      body: JSON.stringify({ user_address: userAddress }),
    });
  }

  // WebSocket Connection for Real-time Updates
  connectWebSocket(onMessage: (data: any) => void, onError?: (error: Event) => void): WebSocket {
    const ws = new WebSocket(this.wsUrl);

    ws.onopen = () => {
      console.log('WebSocket connected to DeFi Risk Monitor');
    };

    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        onMessage(data);
      } catch (error) {
        console.error('Failed to parse WebSocket message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      if (onError) {
        onError(error);
      }
    };

    ws.onclose = () => {
      console.log('WebSocket connection closed');
    };

    return ws;
  }

  // Health Check
  async healthCheck(): Promise<{ status: string; timestamp: string }> {
    return this.request<{ status: string; timestamp: string }>('/health');
  }
}

// Default API client instance
export const apiClient = new DeFiRiskAPI();

// Export types for use in components
export type {
  CreatePositionRequest as CreatePositionRequestType,
  Position as PositionType,
  RiskMetrics as RiskMetricsType,
  RiskExplanation as RiskExplanationType,
  ProtocolEvent as ProtocolEventType,
  EventStats as EventStatsType,
  AlertThreshold as AlertThresholdType,
};
