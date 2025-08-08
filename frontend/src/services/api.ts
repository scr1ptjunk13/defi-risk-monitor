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
    return this.request<PortfolioSummary>(`/api/v1/portfolio/${walletAddress}/summary`);
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
}

export const apiService = new ApiService();
export default apiService;
