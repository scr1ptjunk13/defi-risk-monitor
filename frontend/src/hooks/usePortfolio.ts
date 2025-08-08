import { useState, useEffect, useCallback } from 'react';
import { apiService, PortfolioSummary, Position } from '../services/api';

export interface UsePortfolioState {
  portfolio: PortfolioSummary | null;
  positions: Position[];
  loading: boolean;
  error: string | null;
  lastUpdated: Date | null;
}

export interface UsePortfolioActions {
  fetchPortfolio: (walletAddress: string) => Promise<void>;
  refreshPortfolio: () => Promise<void>;
  clearPortfolio: () => void;
}

export interface UsePortfolioReturn extends UsePortfolioState, UsePortfolioActions {}

export function usePortfolio(initialWalletAddress?: string): UsePortfolioReturn {
  const [state, setState] = useState<UsePortfolioState>({
    portfolio: null,
    positions: [],
    loading: false,
    error: null,
    lastUpdated: null,
  });

  const [currentWalletAddress, setCurrentWalletAddress] = useState<string | null>(
    initialWalletAddress || null
  );

  const fetchPortfolio = useCallback(async (walletAddress: string) => {
    if (!walletAddress) {
      setState(prev => ({ ...prev, error: 'Wallet address is required' }));
      return;
    }

    // Validate wallet address format
    if (!apiService.validateWalletAddress(walletAddress) && !apiService.isENSName(walletAddress)) {
      setState(prev => ({ ...prev, error: 'Invalid wallet address format' }));
      return;
    }

    setState(prev => ({ ...prev, loading: true, error: null }));
    setCurrentWalletAddress(walletAddress);

    try {
      // First check if backend is healthy
      const healthCheck = await apiService.healthCheck();
      if (!healthCheck.success) {
        throw new Error('Backend service is not available');
      }

      // Fetch portfolio data
      const response = await apiService.getPortfolioPositions(walletAddress);
      
      if (response.success && response.data) {
        // Create portfolio summary from positions
        const positions = response.data;
        const totalValue = positions.reduce((sum, pos) => sum + parseFloat(pos.amount_usd), 0);
        const totalPnL = positions.reduce((sum, pos) => sum + (pos.pnl_usd ? parseFloat(pos.pnl_usd) : 0), 0);
        const pnlPercentage = totalValue > 0 ? (totalPnL / totalValue) * 100 : 0;
        
        const portfolioSummary: PortfolioSummary = {
          total_value_usd: totalValue,
          total_pnl_usd: totalPnL,
          pnl_percentage: pnlPercentage,
          risk_score: 5.0, // TODO: Calculate real risk score
          positions: positions,
        };
        
        setState(prev => ({
          ...prev,
          portfolio: portfolioSummary,
          positions: positions,
          loading: false,
          error: null,
          lastUpdated: new Date(),
        }));
      } else {
        throw new Error(response.error || 'Failed to fetch portfolio data');
      }
    } catch (error) {
      console.error('Failed to fetch portfolio:', error);
      setState(prev => ({
        ...prev,
        loading: false,
        error: error instanceof Error ? error.message : 'Unknown error occurred',
        portfolio: null,
        positions: [],
      }));
    }
  }, []);

  const refreshPortfolio = useCallback(async () => {
    if (currentWalletAddress) {
      await fetchPortfolio(currentWalletAddress);
    }
  }, [currentWalletAddress, fetchPortfolio]);

  const clearPortfolio = useCallback(() => {
    setState({
      portfolio: null,
      positions: [],
      loading: false,
      error: null,
      lastUpdated: null,
    });
    setCurrentWalletAddress(null);
  }, []);

  // Auto-fetch on mount if initial address provided
  useEffect(() => {
    if (initialWalletAddress) {
      fetchPortfolio(initialWalletAddress);
    }
  }, [initialWalletAddress, fetchPortfolio]);

  return {
    ...state,
    fetchPortfolio,
    refreshPortfolio,
    clearPortfolio,
  };
}

export default usePortfolio;
