/**
 * useRiskMonitoring Hook
 * 
 * Custom hook for managing position risk monitoring, real-time updates,
 * and risk analytics integration with the backend API
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { useAccount } from 'wagmi';
import { toast } from 'react-hot-toast';
import { 
  apiClient, 
  Position, 
  RiskMetrics, 
  RiskExplanation, 
  CreatePositionRequest 
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
  connectRealTime: () => void;
  disconnectRealTime: () => void;

  // Error Handling
  error: string | null;
  clearError: () => void;
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
  const [error, setError] = useState<string | null>(null);

  // WebSocket connection ref
  const wsRef = useRef<WebSocket | null>(null);

  // Account connection
  const { address, isConnected: walletConnected } = useAccount();

  // Error handling
  const handleError = useCallback((error: any, context: string) => {
    console.error(`Risk monitoring error in ${context}:`, error);
    const errorMessage = error?.message || `Failed to ${context}`;
    setError(errorMessage);
    toast.error(errorMessage);
  }, []);

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  // Position Management Functions
  const createPosition = useCallback(async (positionData: CreatePositionRequest): Promise<Position | null> => {
    if (!walletConnected || !address) {
      toast.error('Please connect your wallet');
      return null;
    }

    try {
      setIsLoadingPositions(true);
      clearError();

      const positionWithUser = {
        ...positionData,
        user_address: address,
      };

      const newPosition = await apiClient.createPosition(positionWithUser);
      
      // Add to positions list
      setPositions(prev => [newPosition, ...prev]);
      setCurrentPosition(newPosition);
      
      toast.success('Position created and risk monitoring enabled!');
      return newPosition;
    } catch (error) {
      handleError(error, 'create position');
      return null;
    } finally {
      setIsLoadingPositions(false);
    }
  }, [address, walletConnected, handleError, clearError]);

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
  }, [handleError, clearError]);

  const refreshPositions = useCallback(async () => {
    if (!walletConnected || !address) return;

    try {
      setIsLoadingPositions(true);
      clearError();

      const userPositions = await apiClient.getPositions(address);
      setPositions(userPositions);
    } catch (error) {
      handleError(error, 'refresh positions');
    } finally {
      setIsLoadingPositions(false);
    }
  }, [address, walletConnected, handleError, clearError]);

  // Risk Analytics Functions
  const calculateRisk = useCallback(async (positionId: string) => {
    try {
      setIsLoadingRisk(true);
      clearError();

      const risk = await apiClient.getPositionRisk(positionId);
      setRiskMetrics(risk);
      setLastUpdate(new Date());
    } catch (error) {
      handleError(error, 'calculate risk');
    } finally {
      setIsLoadingRisk(false);
    }
  }, [handleError, clearError]);

  const explainRisk = useCallback(async (positionId: string) => {
    try {
      setIsLoadingRisk(true);
      clearError();

      const explanation = await apiClient.explainRisk(positionId);
      setRiskExplanation(explanation);
    } catch (error) {
      handleError(error, 'explain risk');
    } finally {
      setIsLoadingRisk(false);
    }
  }, [handleError, clearError]);

  // Real-time WebSocket Functions
  const connectRealTime = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    try {
      wsRef.current = apiClient.connectWebSocket(
        (data) => {
          setLastUpdate(new Date());
          
          switch (data.type) {
            case 'RISK_UPDATE':
              if (data.position_id === positionId || data.position_id === currentPosition?.id) {
                setRiskMetrics(data.risk_metrics);
              }
              break;
              
            case 'POSITION_UPDATE':
              if (data.position_id === positionId || data.position_id === currentPosition?.id) {
                setCurrentPosition(prev => prev ? { ...prev, ...data.updates } : null);
              }
              // Update in positions list
              setPositions(prev => 
                prev.map(pos => 
                  pos.id === data.position_id 
                    ? { ...pos, ...data.updates }
                    : pos
                )
              );
              break;
              
            case 'ALERT_NOTIFICATION':
              if (data.alert.position_id === positionId || data.alert.position_id === currentPosition?.id) {
                toast.error(`Risk Alert: ${data.alert.message}`, {
                  duration: 10000,
                  icon: '⚠️',
                });
              }
              break;
              
            default:
              console.log('Received WebSocket message:', data);
          }
        },
        (error) => {
          console.error('WebSocket error:', error);
          setIsConnected(false);
          toast.error('Lost connection to risk monitoring service');
        }
      );

      wsRef.current.onopen = () => {
        setIsConnected(true);
        console.log('Connected to real-time risk monitoring');
      };

      wsRef.current.onclose = () => {
        setIsConnected(false);
        console.log('Disconnected from real-time risk monitoring');
      };

    } catch (error) {
      handleError(error, 'connect to real-time monitoring');
    }
  }, [positionId, currentPosition?.id, handleError]);

  const disconnectRealTime = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
      setIsConnected(false);
    }
  }, []);

  // Effects
  
  // Load positions when wallet connects
  useEffect(() => {
    if (walletConnected && address) {
      refreshPositions();
    } else {
      setPositions([]);
      setCurrentPosition(null);
      setRiskMetrics(null);
      setRiskExplanation(null);
    }
  }, [walletConnected, address, refreshPositions]);

  // Load specific position if positionId provided
  useEffect(() => {
    if (positionId && positionId !== currentPosition?.id) {
      getPosition(positionId);
    }
  }, [positionId, currentPosition?.id, getPosition]);

  // Calculate risk for current position
  useEffect(() => {
    if (currentPosition?.id) {
      calculateRisk(currentPosition.id);
    }
  }, [currentPosition?.id, calculateRisk]);

  // Auto-connect to real-time updates when position is available
  useEffect(() => {
    if (currentPosition?.id || (positions.length > 0 && walletConnected)) {
      connectRealTime();
    }

    return () => {
      disconnectRealTime();
    };
  }, [currentPosition?.id, positions.length, walletConnected, connectRealTime, disconnectRealTime]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
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

    // Error Handling
    error,
    clearError,
  };
};
