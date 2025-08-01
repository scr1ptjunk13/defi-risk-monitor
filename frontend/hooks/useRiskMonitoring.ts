/**
 * useRiskMonitoring Hook
 * 
 * Enhanced custom hook for managing position risk monitoring, real-time updates,
 * risk analytics integration, authentication, and error handling with the backend API
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useAccount } from 'wagmi';
import { toast } from 'react-hot-toast';
import { 
  apiClient, 
  Position, 
  RiskMetrics, 
  RiskExplanation, 
  CreatePositionRequest,
  APIError,
  NetworkError,
  AuthenticationError
} from '../lib/api-client';

export interface UseRiskMonitoringReturn {
  // Position Management
  positions: Position[];
  currentPosition: Position | null;
  isLoadingPositions: boolean;
  createPosition: (positionData: CreatePositionRequest) => Promise<Position | null>;
  getPosition: (positionId: string) => Promise<void>;
  refreshPositions: () => Promise<void>;

  // Risk Analytics
  riskMetrics: RiskMetrics | null;
  riskExplanation: RiskExplanation | null;
  isLoadingRisk: boolean;
  calculateRisk: (positionId: string) => Promise<void>;
  explainRisk: (positionId: string) => Promise<void>;

  // Real-time Updates
  isConnected: boolean;
  lastUpdate: Date | null;
  connectRealTime: () => Promise<void>;
  disconnectRealTime: () => void;
  reconnectRealTime: () => Promise<void>;

  // Error Handling & Status
  error: APIError | null;
  isRetrying: boolean;
  retryCount: number;
  clearError: () => void;
  
  // Authentication
  isAuthenticated: boolean;
  authenticate: (token: string) => void;
  logout: () => void;

  // Configuration
  config: {
    autoRefresh: boolean;
    refreshInterval: number;
    enableRealTime: boolean;
  };
  updateConfig: (newConfig: Partial<{
    autoRefresh: boolean;
    refreshInterval: number;
    enableRealTime: boolean;
  }>) => void;
}

export const useRiskMonitoring = (positionId?: string): UseRiskMonitoringReturn => {
  // State Management
  const [positions, setPositions] = useState<Position[]>([]);
  const [currentPosition, setCurrentPosition] = useState<Position | null>(null);
  const [riskMetrics, setRiskMetrics] = useState<RiskMetrics | null>(null);
  const [riskExplanation, setRiskExplanation] = useState<RiskExplanation | null>(null);
  const [isLoadingPositions, setIsLoadingPositions] = useState(false);
  const [isLoadingRisk, setIsLoadingRisk] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);
  const [error, setError] = useState<APIError | null>(null);
  const [isRetrying, setIsRetrying] = useState(false);
  const [retryCount, setRetryCount] = useState(0);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [config, setConfig] = useState({
    autoRefresh: true,
    refreshInterval: parseInt(process.env.NEXT_PUBLIC_POSITION_REFRESH_INTERVAL || '60000'),
    enableRealTime: process.env.NEXT_PUBLIC_ENABLE_REAL_TIME_ALERTS === 'true'
  });

  // WebSocket connection ref
  const wsRef = useRef<WebSocket | null>(null);

  // Refs for cleanup and configuration
  const refreshIntervalRef = useRef<NodeJS.Timeout | null>(null);
  const retryTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const maxRetries = 3;
  const retryDelay = 5000;

  // Get wallet connection
  const { address, isConnected: walletConnected } = useAccount();

  // Enhanced error handling
  const handleError = useCallback((err: unknown, context: string) => {
    console.error(`Error in ${context}:`, err);
    
    if (err instanceof APIError) {
      setError(err);
      
      // Handle specific error types
      if (err instanceof AuthenticationError) {
        setIsAuthenticated(false);
        toast.error('Authentication required. Please reconnect your wallet.');
      } else if (err instanceof NetworkError) {
        toast.error('Network error. Retrying...');
        // Trigger retry for network errors
        if (retryCount < maxRetries) {
          setIsRetrying(true);
          setRetryCount(prev => prev + 1);
        }
      } else {
        toast.error(err.message || 'An error occurred');
      }
    } else {
      const genericError = new APIError(
        err instanceof Error ? err.message : 'Unknown error occurred',
        0,
        'UNKNOWN_ERROR'
      );
      setError(genericError);
      toast.error('An unexpected error occurred');
    }
  }, [retryCount]);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Position Management Functions
  const createPosition = useCallback(async (positionData: CreatePositionRequest): Promise<Position | null> => {
    clearError();
    
    try {
      const newPosition = await apiClient.createPosition(positionData);
      
      // Add to positions list
      setPositions(prev => [...prev, newPosition]);
      setCurrentPosition(newPosition);
      setLastUpdate(new Date());
      
      toast.success('Position created successfully!');
      return newPosition;
    } catch (err) {
      handleError(err, 'createPosition');
      return null;
    }
  }, [clearError, handleError]);

  const getPosition = useCallback(async (positionId: string) => {
    try {
      setIsLoadingPositions(true);
      clearError();

      const position = await apiClient.getPosition(positionId);
      setCurrentPosition(position);
    } catch (error) {
      handleError(error, 'fetch position');
    } finally {
      setIsLoadingPositions(false);
    }
  }, [clearError, handleError]);

  const refreshPositions = useCallback(async () => {
    if (!address) return;
    
    setIsLoadingPositions(true);
    clearError();
    
    try {
      const userPositions = await apiClient.getPositions(address);
      setPositions(userPositions);
      setLastUpdate(new Date());
      
      // If we have a specific position ID, find and set it
      if (positionId) {
        const foundPosition = userPositions.find(p => p.id === positionId);
        if (foundPosition) {
          setCurrentPosition(foundPosition);
        }
      }
      
      // Reset retry count on success
      setRetryCount(0);
      setIsRetrying(false);
    } catch (err) {
      handleError(err, 'refreshPositions');
    } finally {
      setIsLoadingPositions(false);
    }
  }, [address, positionId, clearError, handleError]);

  const calculateRisk = useCallback(async (positionId: string) => {
    setIsLoadingRisk(true);
    clearError();
    
    try {
      const risk = await apiClient.getPositionRisk(positionId);
      setRiskMetrics(risk);
      setLastUpdate(new Date());
    } catch (err) {
      handleError(err, 'calculateRisk');
    } finally {
      setIsLoadingRisk(false);
    }
  }, [clearError, handleError]);

  const explainRisk = useCallback(async (positionId: string) => {
    setIsLoadingRisk(true);
    clearError();
    
    try {
      const explanation = await apiClient.explainRisk(positionId);
      setRiskExplanation(explanation);
      setLastUpdate(new Date());
    } catch (err) {
      handleError(err, 'explainRisk');
    } finally {
      setIsLoadingRisk(false);
    }
  }, [clearError, handleError]);

  // Real-time WebSocket Functions
  const connectRealTime = useCallback(async () => {
    if (!address || isConnected || !config.enableRealTime) return;
    
    try {
      await apiClient.connectWebSocket(
        (data) => {
          // Handle real-time updates
          console.log('Real-time update received:', data);
          
          if (data.type === 'position_update') {
            // Update specific position
            setPositions(prev => 
              prev.map(pos => 
                pos.id === data.position_id ? { ...pos, ...data.updates } : pos
              )
            );
          } else if (data.type === 'risk_update') {
            // Update risk metrics
            if (data.position_id === currentPosition?.id) {
              setRiskMetrics(data.risk_metrics);
            }
          } else if (data.type === 'alert') {
            // Show alert notification
            const severity = data.severity || 'medium';
            if (severity === 'critical') {
              toast.error(`🚨 Critical Alert: ${data.message}`, { duration: 10000 });
            } else if (severity === 'high') {
              toast.error(`⚠️ High Risk Alert: ${data.message}`, { duration: 6000 });
            } else {
              toast(`📊 ${data.message}`, { duration: 4000 });
            }
          }
          
          setLastUpdate(new Date());
        },
        (error) => {
          console.error('WebSocket error:', error);
          setIsConnected(false);
          handleError(new NetworkError('WebSocket connection error'), 'connectRealTime');
        }
      );
      
      setIsConnected(true);
      toast.success('Real-time monitoring connected');
    } catch (err) {
      handleError(err, 'connectRealTime');
    }
  }, [address, isConnected, config.enableRealTime, currentPosition?.id, handleError]);

  const disconnectRealTime = useCallback(() => {
    apiClient.disconnectWebSocket();
    setIsConnected(false);
    console.log('Real-time monitoring disconnected');
  }, []);

  const authenticate = useCallback((token: string) => {
    apiClient.setAuthToken(token);
    setIsAuthenticated(true);
  }, []);

  const logout = useCallback(() => {
    apiClient.clearAuthToken();
    setIsAuthenticated(false);
  }, []);

  const updateConfig = useCallback((newConfig: Partial<typeof config>) => {
    setConfig(prev => ({ ...prev, ...newConfig }));
  }, [config]);

  const reconnectRealTime = useCallback(async () => {
    disconnectRealTime();
    await new Promise(resolve => setTimeout(resolve, 1000)); // Wait 1 second
    await connectRealTime();
  }, [disconnectRealTime, connectRealTime]);

  // Effects
  
  // Load positions when wallet connects
  useEffect(() => {
    if (address && walletConnected && config.autoRefresh) {
      refreshPositions();
      
      // Set up auto-refresh interval
      refreshIntervalRef.current = setInterval(() => {
        if (!isRetrying) {
          refreshPositions();
        }
      }, config.refreshInterval);
      
      return () => {
        if (refreshIntervalRef.current) {
          clearInterval(refreshIntervalRef.current);
        }
      };
    }
  }, [address, walletConnected, config.autoRefresh, config.refreshInterval, refreshPositions, isRetrying]);

  // Retry logic for failed requests
  useEffect(() => {
    if (isRetrying && retryCount <= maxRetries) {
      retryTimeoutRef.current = setTimeout(() => {
        console.log(`Retrying request (attempt ${retryCount}/${maxRetries})`);
        refreshPositions();
      }, retryDelay * retryCount);
      
      return () => {
        if (retryTimeoutRef.current) {
          clearTimeout(retryTimeoutRef.current);
        }
      };
    } else if (retryCount > maxRetries) {
      setIsRetrying(false);
      toast.error('Max retry attempts reached. Please check your connection.');
    }
  }, [isRetrying, retryCount, refreshPositions]);

  // Load specific position if positionId provided
  useEffect(() => {
    if (positionId && positionId !== currentPosition?.id) {
      getPosition(positionId);
    }
  }, [positionId, currentPosition?.id, getPosition]);

  // Auto-connect to real-time updates when position is available
  useEffect(() => {
    if (address && walletConnected && config.enableRealTime) {
      connectRealTime();
    }
    
    return () => {
      disconnectRealTime();
    };
  }, [address, walletConnected, config.enableRealTime, connectRealTime, disconnectRealTime]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (refreshIntervalRef.current) {
        clearInterval(refreshIntervalRef.current);
      }
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
      }
      disconnectRealTime();
    };
  }, [disconnectRealTime]);

  return {
    // Position Management
    positions,
    currentPosition,
    isLoadingPositions,
    createPosition,
    getPosition,
    refreshPositions,
    
    // Risk Analytics
    riskMetrics,
    riskExplanation,
    isLoadingRisk,
    calculateRisk,
    explainRisk,
    
    // Real-time Updates
    isConnected,
    lastUpdate,
    connectRealTime,
    disconnectRealTime,
    reconnectRealTime,
    
    // Error Handling & Status
    error,
    isRetrying,
    retryCount,
    clearError,
    
    // Authentication
    isAuthenticated,
    authenticate,
    logout,
    
    // Configuration
    config,
    updateConfig,
  };
};
