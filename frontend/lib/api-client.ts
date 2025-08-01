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
  // Extended properties for frontend components
  protocol?: string;
  chain_id?: number;
  entry_price_usd?: string;
  position_range_lower?: string;
  position_range_upper?: string;
  is_active?: boolean;
  notes?: string;
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

import { toast } from 'react-hot-toast';
import { errorHandler, ErrorSeverity, ErrorCategory } from './error-handling';

// Enhanced Error Classes
export class APIError extends Error {
  constructor(
    message: string,
    public statusCode: number,
    public code: string,
    public details?: any
  ) {
    super(message);
    this.name = 'APIError';
  }
}

export class NetworkError extends APIError {
  constructor(message: string, public originalError?: Error) {
    super(message, 0, 'NETWORK_ERROR');
    this.name = 'NetworkError';
  }
}

export class AuthenticationError extends APIError {
  constructor(message: string = 'Authentication required') {
    super(message, 401, 'AUTH_ERROR');
    this.name = 'AuthenticationError';
  }
}

// Enhanced WebSocket Manager with Error Handling
export class WebSocketManager {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;
  private messageHandlers: Set<(data: any) => void> = new Set();
  private errorHandlers: Set<(error: Event) => void> = new Set();
  private isConnecting = false;

  constructor(url: string) {
    this.url = url;
  }

  async connect(): Promise<void> {
    if (this.isConnecting || (this.ws && this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    this.isConnecting = true;

    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          this.isConnecting = false;
          this.reconnectAttempts = 0;
          console.log('WebSocket connected');
          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            this.messageHandlers.forEach(handler => handler(data));
          } catch (error) {
            errorHandler.handleError(error as Error, {
              severity: ErrorSeverity.LOW,
              category: ErrorCategory.API,
              component: 'WebSocketManager',
              action: 'Parse Message',
              showToast: false
            });
          }
        };

        this.ws.onerror = (error) => {
          errorHandler.handleError(new Error('WebSocket connection error'), {
            severity: ErrorSeverity.HIGH,
            category: ErrorCategory.NETWORK,
            component: 'WebSocketManager',
            action: 'Connection',
            additionalData: { url: this.url, reconnectAttempts: this.reconnectAttempts }
          });
          this.errorHandlers.forEach(handler => handler(error));
        };

        this.ws.onclose = () => {
          this.isConnecting = false;
          this.ws = null;
          this.handleReconnect();
        };

      } catch (error) {
        this.isConnecting = false;
        errorHandler.handleError(error as Error, {
          severity: ErrorSeverity.CRITICAL,
          category: ErrorCategory.NETWORK,
          component: 'WebSocketManager',
          action: 'Initialize Connection'
        });
        reject(error);
      }
    });
  }

  private async handleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      errorHandler.handleError(new Error('Max WebSocket reconnection attempts reached'), {
        severity: ErrorSeverity.CRITICAL,
        category: ErrorCategory.NETWORK,
        component: 'WebSocketManager',
        action: 'Reconnect',
        additionalData: { maxAttempts: this.maxReconnectAttempts }
      });
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

    setTimeout(() => {
      this.connect().catch(error => {
        errorHandler.handleError(error, {
          severity: ErrorSeverity.HIGH,
          category: ErrorCategory.NETWORK,
          component: 'WebSocketManager',
          action: 'Reconnect Attempt',
          additionalData: { attempt: this.reconnectAttempts, delay }
        });
      });
    }, delay);
  }

  addMessageHandler(handler: (data: any) => void) {
    this.messageHandlers.add(handler);
  }

  removeMessageHandler(handler: (data: any) => void) {
    this.messageHandlers.delete(handler);
  }

  addErrorHandler(handler: (error: Event) => void) {
    this.errorHandlers.add(handler);
  }

  removeErrorHandler(handler: (error: Event) => void) {
    this.errorHandlers.delete(handler);
  }

  send(data: any) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      try {
        this.ws.send(JSON.stringify(data));
      } catch (error) {
        errorHandler.handleError(error as Error, {
          severity: ErrorSeverity.MEDIUM,
          category: ErrorCategory.API,
          component: 'WebSocketManager',
          action: 'Send Message'
        });
      }
    } else {
      errorHandler.handleError(new Error('WebSocket not connected'), {
        severity: ErrorSeverity.MEDIUM,
        category: ErrorCategory.NETWORK,
        component: 'WebSocketManager',
        action: 'Send Message',
        showToast: false
      });
    }
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.reconnectAttempts = 0;
  }

  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }
}

// Enhanced API Client Class
export class DeFiRiskAPI {
  private baseURL: string;
  private wsManager: WebSocketManager;
  private authToken: string | null = null;
  private requestQueue: Array<() => Promise<any>> = [];
  private isProcessingQueue = false;

  constructor(
    baseURL = process.env.NEXT_PUBLIC_API_BASE_URL || 'http://localhost:8080/api/v1',
    wsUrl = process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws'
  ) {
    this.baseURL = baseURL;
    this.wsManager = new WebSocketManager(wsUrl);
  }

  // Authentication methods
  setAuthToken(token: string) {
    this.authToken = token;
  }

  clearAuthToken() {
    this.authToken = null;
  }

  // Enhanced request method with retry logic and better error handling
  private async request<T>(
    endpoint: string,
    options: RequestInit = {},
    retryCount = 0
  ): Promise<T> {
    const maxRetries = 3;
    const retryDelay = 1000 * Math.pow(2, retryCount); // Exponential backoff
    
    try {
      const url = `${this.baseURL}${endpoint}`;
      
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        ...options.headers as Record<string, string>,
      };

      // Add authentication header if token is available
      if (this.authToken) {
        headers['Authorization'] = `Bearer ${this.authToken}`;
      }

      const response = await fetch(url, {
        ...options,
        headers,
      });

      // Handle different HTTP status codes
      if (!response.ok) {
        const errorText = await response.text();
        let errorData;
        
        try {
          errorData = JSON.parse(errorText);
        } catch {
          errorData = { message: errorText };
        }

        switch (response.status) {
          case 401:
            throw new AuthenticationError(errorData.message || 'Authentication required');
          case 403:
            throw new APIError('Access forbidden', 403, 'FORBIDDEN', errorData);
          case 404:
            throw new APIError('Resource not found', 404, 'NOT_FOUND', errorData);
          case 429:
            throw new APIError('Rate limit exceeded', 429, 'RATE_LIMIT', errorData);
          case 500:
            throw new APIError('Internal server error', 500, 'SERVER_ERROR', errorData);
          default:
            throw new APIError(
              errorData.message || `HTTP error! status: ${response.status}`,
              response.status,
              'HTTP_ERROR',
              errorData
            );
        }
      }

      const contentType = response.headers.get('content-type');
      if (contentType && contentType.includes('application/json')) {
        return response.json();
      } else {
        return response.text() as any;
      }
    } catch (error) {
      // Handle network errors and retry logic
      if (error instanceof TypeError && error.message.includes('fetch')) {
        const networkError = new NetworkError('Network request failed', error);
        
        if (retryCount < maxRetries) {
          console.warn(`Request failed, retrying in ${retryDelay}ms (attempt ${retryCount + 1}/${maxRetries})`);
          await new Promise(resolve => setTimeout(resolve, retryDelay));
          return this.request<T>(endpoint, options, retryCount + 1);
        }
        
        throw networkError;
      }
      
      // Re-throw API errors without retry
      if (error instanceof APIError) {
        throw error;
      }
      
      // Handle unexpected errors
      throw new APIError(
        error instanceof Error ? error.message : 'Unknown error occurred',
        0,
        'UNKNOWN_ERROR',
        error
      );
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

  // Enhanced WebSocket Connection Management
  async connectWebSocket(onMessage: (data: any) => void, onError?: (error: Event) => void): Promise<void> {
    try {
      await this.wsManager.connect();
      this.wsManager.addMessageHandler(onMessage);
      if (onError) {
        this.wsManager.addErrorHandler(onError);
      }
      console.log('WebSocket connected to DeFi Risk Monitor');
    } catch (error) {
      console.error('Failed to connect WebSocket:', error);
      if (onError) {
        onError(error as Event);
      }
      throw error;
    }
  }

  disconnectWebSocket(): void {
    this.wsManager.disconnect();
    console.log('WebSocket disconnected from DeFi Risk Monitor');
  }

  isWebSocketConnected(): boolean {
    return this.wsManager.isConnected();
  }

  sendWebSocketMessage(data: any): void {
    try {
      this.wsManager.send(data);
    } catch (error) {
      console.error('Failed to send WebSocket message:', error);
      throw error;
    }
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
