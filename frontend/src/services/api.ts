// API service for communicating with the Rust backend
const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

export interface Position {
  id: string;
  user_id: string;
  protocol: string;
  pool_address: string;
  chain_id: number;
  token0_address: string;
  token1_address: string;
  position_type: string;
  entry_price: string;
  current_price?: string;
  amount_usd: string;
  liquidity_amount?: string;
  fee_tier?: number;
  tick_lower?: number;
  tick_upper?: number;
  pnl_usd?: string;
  fees_earned_usd?: string;
  impermanent_loss_usd?: string;
  risk_score?: number; // Real calculated risk score from backend
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface PortfolioSummary {
  total_value_usd: number;
  total_pnl_usd: number;
  pnl_percentage: number;
  risk_score: number;
  positions: Position[];
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// Risk Monitor specific interfaces
export interface PortfolioRiskMetrics {
  overall_risk: number;
  liquidity_risk: number;
  volatility_risk: number;
  mev_risk: number;
  protocol_risk: number;
  timestamp: string;
}

export interface LiveRiskAlert {
  id: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  alert_type: string;
  message: string;
  position_id?: string;
  protocol?: string;
  timestamp: string;
  acknowledged: boolean;
}

export interface RiskFactors {
  liquidity: number;
  volatility: number;
  mev: number;
  protocol: number;
}

export interface PositionRiskHeatmap {
  id: string;
  protocol: string;
  pair: string;
  risk_score: number;
  risk_factors: RiskFactors;
  alerts: number;
  trend: 'up' | 'down' | 'stable';
}

// Advanced Analytics interfaces
export interface PortfolioAnalytics {
  total_return_usd: string;
  total_return_percentage: string;
  volatility: string;
  sharpe_ratio: string;
  max_drawdown: string;
  alpha?: string;
  beta?: string;
}

export interface CorrelationMatrix {
  [asset: string]: { [asset: string]: number };
}

export interface RiskDecomposition {
  systematic_risk: number;
  idiosyncratic_risk: number;
  concentration_risk: number;
  liquidity_risk: number;
}

export interface StressTestResult {
  scenario: string;
  impact: number;
  probability: number;
}

class ApiService {
  private baseUrl: string;

  constructor() {
    this.baseUrl = API_BASE_URL;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    try {
      const url = `${this.baseUrl}${endpoint}`;
      const response = await fetch(url, {
        headers: {
          'Content-Type': 'application/json',
          ...options.headers,
        },
        ...options,
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      return { success: true, data };
    } catch (error) {
      console.error('API request failed:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  // Fetch user positions from all protocols (currently Uniswap V3 only)
  async getPortfolioPositions(address: string): Promise<ApiResponse<Position[]>> {
    const response = await this.request<Position[]>(`/api/v1/positions/wallet/${address}`);
    return response;
  }

  // Get portfolio summary with risk metrics
  async getPortfolioSummary(walletAddress: string): Promise<ApiResponse<PortfolioSummary>> {
    return this.request<PortfolioSummary>(`/api/v1/portfolio/summary?user_address=${encodeURIComponent(walletAddress)}`);
  }

  // Health check endpoint
  async healthCheck(): Promise<ApiResponse<{ status: string }>> {
    return this.request<{ status: string }>('/health');
  }

  // Validate wallet address format
  validateWalletAddress(address: string): boolean {
    // Basic Ethereum address validation
    const ethAddressRegex = /^0x[a-fA-F0-9]{40}$/;
    return ethAddressRegex.test(address);
  }

  // Check if address is ENS name
  isENSName(address: string): boolean {
    return address.endsWith('.eth');
  }

  // Risk Monitor API endpoints
  async getPortfolioRiskMetrics(address: string): Promise<ApiResponse<PortfolioRiskMetrics>> {
    return this.request<PortfolioRiskMetrics>(`/api/v1/portfolio-risk-metrics?address=${encodeURIComponent(address)}`);
  }

  async getLiveRiskAlerts(address: string): Promise<ApiResponse<LiveRiskAlert[]>> {
    return this.request<LiveRiskAlert[]>(`/api/v1/live-alerts?address=${encodeURIComponent(address)}`);
  }

  async getPositionRiskHeatmap(address: string): Promise<ApiResponse<PositionRiskHeatmap[]>> {
    return this.request<PositionRiskHeatmap[]>(`/api/v1/position-risk-heatmap?address=${address}`);
  }

  // Advanced Analytics API methods
  async getPortfolioAnalytics(address: string, period: string = '30d'): Promise<ApiResponse<PortfolioAnalytics>> {
    return this.request<PortfolioAnalytics>(`/api/v1/analytics/portfolio-performance?user_address=${address}&period=${period}`);
  }

  async getCorrelationMatrix(address: string): Promise<ApiResponse<CorrelationMatrix>> {
    return this.request<CorrelationMatrix>(`/api/v1/analytics/correlation-matrix?user_address=${address}`);
  }

  async getRiskDecomposition(address: string): Promise<ApiResponse<RiskDecomposition>> {
    return this.request<RiskDecomposition>(`/api/v1/analytics/risk-decomposition?user_address=${address}`);
  }

  async getStressTestResults(address: string): Promise<ApiResponse<StressTestResult[]>> {
    return this.request<StressTestResult[]>(`/api/v1/analytics/stress-test?user_address=${address}`);
  }
}

export const apiService = new ApiService();
export default apiService;
