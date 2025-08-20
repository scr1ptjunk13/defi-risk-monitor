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
  entry_price?: string;
  current_price?: string;
  amount_usd: string;
  value_usd: string; // Backend uses value_usd
  liquidity_amount?: string;
  liquidity?: string;
  fee_tier?: number;
  tick_lower?: number;
  tick_upper?: number;
  pnl_usd?: string;
  fees_earned_usd?: string;
  impermanent_loss_usd?: string;
  risk_score?: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  pair?: string; // Backend includes pair info
  metadata?: any; // Backend includes metadata
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

  // Fetch user positions from all protocols across all chains
  async getPortfolioPositions(address: string): Promise<ApiResponse<Position[]>> {
    try {
      // Handle ENS names by resolving them first if needed
      const resolvedAddress = await this.resolveENSIfNeeded(address);
      
      const response = await this.request<any>(`/api/v1/positions/wallet/${resolvedAddress}`);
      
      if (response.success && response.data) {
        // Backend returns { positions: Position[], summary: {...} }
        const positions = response.data.positions || [];
        
        // Transform backend positions to frontend format
        const transformedPositions: Position[] = positions.map((pos: any) => ({
          id: pos.id || `${pos.protocol}-${Date.now()}`,
          user_id: pos.user_id || resolvedAddress,
          protocol: pos.protocol,
          pool_address: pos.pool_address || '',
          chain_id: pos.chain_id || 1,
          token0_address: pos.token0_address || '',
          token1_address: pos.token1_address || '',
          position_type: pos.position_type,
          entry_price: pos.entry_price,
          current_price: pos.current_price,
          amount_usd: pos.value_usd || pos.amount_usd || '0',
          value_usd: pos.value_usd || '0',
          liquidity_amount: pos.liquidity_amount,
          liquidity: pos.liquidity,
          fee_tier: pos.fee_tier,
          tick_lower: pos.tick_lower,
          tick_upper: pos.tick_upper,
          pnl_usd: pos.pnl_usd || '0',
          fees_earned_usd: pos.fees_earned_usd || '0',
          impermanent_loss_usd: pos.impermanent_loss_usd || '0',
          risk_score: pos.risk_score || 50,
          is_active: pos.is_active !== false,
          created_at: pos.created_at,
          updated_at: pos.updated_at,
          pair: pos.pair,
          metadata: pos.metadata
        }));
        
        return { success: true, data: transformedPositions };
      }
      
      return response;
    } catch (error) {
      console.error('Failed to fetch portfolio positions:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to fetch positions'
      };
    }
  }

  // Get portfolio summary with risk metrics
  async getPortfolioSummary(walletAddress: string): Promise<ApiResponse<PortfolioSummary>> {
    try {
      const resolvedAddress = await this.resolveENSIfNeeded(walletAddress);
      
      // First get positions to calculate summary
      const positionsResponse = await this.getPortfolioPositions(resolvedAddress);
      
      if (positionsResponse.success && positionsResponse.data) {
        const positions = positionsResponse.data;
        
        // Calculate summary from positions
        const totalValue = positions.reduce((sum, pos) => sum + parseFloat(pos.value_usd || pos.amount_usd || '0'), 0);
        const totalPnL = positions.reduce((sum, pos) => sum + parseFloat(pos.pnl_usd || '0'), 0);
        const pnlPercentage = totalValue > 0 ? (totalPnL / totalValue) * 100 : 0;
        
        // Calculate average risk score
        const avgRiskScore = positions.length > 0 
          ? positions.reduce((sum, pos) => sum + (pos.risk_score || 50), 0) / positions.length
          : 50;
        
        const portfolioSummary: PortfolioSummary = {
          total_value_usd: totalValue,
          total_pnl_usd: totalPnL,
          pnl_percentage: pnlPercentage,
          risk_score: avgRiskScore / 100, // Convert to 0-1 scale
          positions: positions
        };
        
        return { success: true, data: portfolioSummary };
      }
      
      return {
        success: false,
        error: positionsResponse.error || 'Failed to fetch positions for summary'
      };
    } catch (error) {
      console.error('Failed to fetch portfolio summary:', error);
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Failed to fetch portfolio summary'
      };
    }
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
    return address.endsWith('.eth') || address.endsWith('.ens');
  }

  // Resolve ENS name to address if needed
  private async resolveENSIfNeeded(address: string): Promise<string> {
    if (this.isENSName(address)) {
      // For now, return the ENS name as-is since backend should handle ENS resolution
      // In the future, we could add client-side ENS resolution here
      return address;
    }
    return address;
  }

  // Test connection to backend with real position fetching
  async testConnection(address: string = 'vitalik.eth'): Promise<ApiResponse<any>> {
    try {
      const healthCheck = await this.healthCheck();
      if (!healthCheck.success) {
        return { success: false, error: 'Backend health check failed' };
      }
      
      const positionsTest = await this.getPortfolioPositions(address);
      return {
        success: true,
        data: {
          health: healthCheck.data,
          positions_count: positionsTest.data?.length || 0,
          test_address: address
        }
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : 'Connection test failed'
      };
    }
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
