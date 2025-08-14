'use client';

import React, { useState, useEffect, useRef } from 'react';
import { apiService, PortfolioRiskMetrics, LiveRiskAlert, PositionRiskHeatmap } from '../../services/api';
import { usePortfolio } from '../../hooks/usePortfolio';

interface RealTimeRiskMonitorProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

// Using imported interfaces from API service
// PortfolioRiskMetrics, LiveRiskAlert, PositionRiskHeatmap

const RealTimeRiskMonitor: React.FC<RealTimeRiskMonitorProps> = ({ userAddress, userTier }) => {
  const [riskMetrics, setRiskMetrics] = useState<PortfolioRiskMetrics | null>(null);
  const [alerts, setAlerts] = useState<LiveRiskAlert[]>([]);
  const [positionRisks, setPositionRisks] = useState<PositionRiskHeatmap[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<number>(Date.now());
  const wsRef = useRef<WebSocket | null>(null);

  // Use the same portfolio hook as Portfolio Overview for data consistency
  const {
    portfolio,
    positions,
    loading,
    error,
    lastUpdated,
    fetchPortfolio,
    refreshPortfolio
  } = usePortfolio();

  // Calculate risk metrics from user's actual portfolio positions
  const calculateRiskMetricsFromPortfolio = () => {
    if (!portfolio || !positions.length) {
      setRiskMetrics(null);
      setPositionRisks([]);
      return;
    }

    // Calculate portfolio-level risk metrics based on actual positions
    const totalValue = portfolio.total_value_usd;
    let liquidityRisk = 0;
    let volatilityRisk = 0;
    let mevRisk = 0;
    let protocolRisk = 0;

    // Calculate risk factors based on actual positions
    positions.forEach(position => {
      const positionWeight = parseFloat(position.amount_usd) / totalValue;
      
      // Calculate position-specific risks (simplified heuristics)
      const posLiquidityRisk = parseFloat(position.amount_usd) > 10000 ? 70 : 40;
      const posVolatilityRisk = position.protocol.toLowerCase().includes('uniswap') ? 65 : 45;
      const posMevRisk = position.protocol.toLowerCase().includes('uniswap') ? 75 : 30;
      const posProtocolRisk = position.protocol.toLowerCase().includes('uniswap') ? 50 : 40;
      
      liquidityRisk += posLiquidityRisk * positionWeight;
      volatilityRisk += posVolatilityRisk * positionWeight;
      mevRisk += posMevRisk * positionWeight;
      protocolRisk += posProtocolRisk * positionWeight;
    });

    const overallRisk = (liquidityRisk * 0.3 + volatilityRisk * 0.25 + mevRisk * 0.25 + protocolRisk * 0.2);

    setRiskMetrics({
      overall_risk: Math.round(overallRisk),
      liquidity_risk: Math.round(liquidityRisk),
      volatility_risk: Math.round(volatilityRisk),
      mev_risk: Math.round(mevRisk),
      protocol_risk: Math.round(protocolRisk),
      timestamp: new Date().toISOString()
    });

    // Create position risk heatmap from actual positions
    const heatmap = positions.map(position => {
      const positionValue = parseFloat(position.amount_usd);
      const pnl = parseFloat(position.pnl_usd || '0');
      
      return {
        id: position.id,
        protocol: position.protocol,
        pair: `${position.token0_address.slice(0,4)}.../${position.token1_address.slice(0,4)}...`, // Simplified pair display
        risk_score: position.risk_score || Math.round(overallRisk),
        risk_factors: {
          liquidity: positionValue > 10000 ? 70 : 40,
          volatility: position.protocol.toLowerCase().includes('uniswap') ? 65 : 45,
          mev: position.protocol.toLowerCase().includes('uniswap') ? 75 : 30,
          protocol: position.protocol.toLowerCase().includes('uniswap') ? 50 : 40
        },
        alerts: 0, // TODO: Calculate based on risk thresholds
        trend: pnl > 0 ? 'up' as const : pnl < 0 ? 'down' as const : 'stable' as const
      };
    });

    setPositionRisks(heatmap);
    setIsConnected(true);
    setLastUpdate(Date.now());
  };

  // Initial data load - fetch portfolio data
  useEffect(() => {
    if (userAddress) {
      fetchPortfolio(userAddress);
    }
  }, [userAddress, fetchPortfolio]);

  // Calculate risk metrics when portfolio data changes
  useEffect(() => {
    calculateRiskMetricsFromPortfolio();
  }, [portfolio, positions]);

  // Set up periodic refresh for real-time updates
  useEffect(() => {
    if (!userAddress) return;
    
    const refreshInterval = setInterval(() => {
      refreshPortfolio();
    }, 30000); // Refresh every 30 seconds

    return () => clearInterval(refreshInterval);
  }, [userAddress, refreshPortfolio]);

  // WebSocket connection for real-time updates (future enhancement)
  useEffect(() => {
    // TODO: Implement WebSocket connection for real-time updates
    // const connectWebSocket = () => {
    //   const ws = new WebSocket(`ws://localhost:8080/ws/risk/${userAddress}`);
    //   wsRef.current = ws;
    //   // Handle WebSocket messages
    // };
    // connectWebSocket();
    
    // For now, we'll keep the real-time simulation until WebSocket is implemented
    // This will be replaced with actual WebSocket updates later
    return () => {
      if (wsRef.current) {
        wsRef.current.close();
      }
    };
  }, [userAddress]);

  const getRiskColor = (score: number) => {
    if (score >= 80) return 'text-red-400 bg-red-900/20 border-red-500/30';
    if (score >= 60) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
    if (score >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
    return 'text-green-400 bg-green-900/20 border-green-500/30';
  };

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return 'text-red-400 bg-red-900/20 border-red-500/30';
      case 'high': return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
      case 'medium': return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
      default: return 'text-blue-400 bg-blue-900/20 border-blue-500/30';
    }
  };

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'up': return '↗️';
      case 'down': return '↘️';
      default: return '➡️';
    }
  };

  const acknowledgeAlert = (alertId: string) => {
    setAlerts(prev => prev.map(alert => 
      alert.id === alertId ? { ...alert, acknowledged: true } : alert
    ));
  };

  return (
    <div className="space-y-6">
      {/* Header with Connection Status */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Real-Time Risk Monitor</h2>
        <div className="flex items-center space-x-4">
          <div className="flex items-center space-x-2">
            <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500 animate-pulse' : 'bg-red-500'}`}></div>
            <span className="text-sm text-gray-400">
              {isConnected ? 'Live' : 'Disconnected'}
            </span>
          </div>
          <div className="text-sm text-gray-400">
            Last update: {new Date(lastUpdate).toLocaleTimeString()}
          </div>
        </div>
      </div>

      {/* Overall Risk Score */}
      <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-white">Portfolio Risk Score</h3>
          <div className={`px-3 py-1 rounded-full text-sm font-medium border ${getRiskColor(riskMetrics?.overall_risk || 0)}`}>
            {Math.round(riskMetrics?.overall_risk || 0)}/100
          </div>
        </div>
        
        {/* Risk Factors Breakdown */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {[
            { key: 'liquidity_risk', label: 'Liquidity', value: riskMetrics?.liquidity_risk || 0 },
            { key: 'volatility_risk', label: 'Volatility', value: riskMetrics?.volatility_risk || 0 },
            { key: 'mev_risk', label: 'MEV', value: riskMetrics?.mev_risk || 0 },
            { key: 'protocol_risk', label: 'Protocol', value: riskMetrics?.protocol_risk || 0 },

          ].map((factor) => (
            <div key={factor.key} className="bg-gray-800/50 rounded-lg p-3">
              <div className="text-xs text-gray-400 mb-1">{factor.label}</div>
              <div className={`text-lg font-semibold ${getRiskColor(factor.value).split(' ')[0]}`}>
                {Math.round(factor.value)}
              </div>
              <div className="w-full bg-gray-700 rounded-full h-1 mt-2">
                <div
                  className={`h-1 rounded-full transition-all duration-500 ${
                    factor.value >= 80 ? 'bg-red-500' :
                    factor.value >= 60 ? 'bg-orange-500' :
                    factor.value >= 30 ? 'bg-yellow-500' : 'bg-green-500'
                  }`}
                  style={{ width: `${factor.value}%` }}
                ></div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Live Alerts */}
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg">
          <div className="p-6 border-b border-gray-700">
            <h3 className="text-lg font-semibold text-white">Live Risk Alerts</h3>
          </div>
          
          <div className="max-h-96 overflow-y-auto">
            {alerts.length === 0 ? (
              <div className="p-6 text-center text-gray-400">
                <div className="text-2xl mb-2">✅</div>
                <p>No active alerts</p>
              </div>
            ) : (
              <div className="space-y-2 p-4">
                {alerts.slice(0, userTier === 'basic' ? 5 : alerts.length).map((alert) => (
                  <div
                    key={alert.id}
                    className={`p-3 rounded-lg border transition-all ${
                      alert.acknowledged 
                        ? 'bg-gray-800/30 border-gray-600 opacity-60' 
                        : `${getSeverityColor(alert.severity)} animate-pulse`
                    }`}
                  >
                    <div className="flex items-start justify-between">
                      <div className="flex-1">
                        <div className="flex items-center space-x-2 mb-1">
                          <span className={`text-xs font-medium px-2 py-1 rounded ${getSeverityColor(alert.severity)}`}>
                            {alert.severity.toUpperCase()}
                          </span>
                          <span className="text-sm text-gray-400">{alert.alert_type}</span>
                        </div>
                        <p className="text-sm text-white mb-1">{alert.message}</p>
                        <div className="text-xs text-gray-400">
                          {alert.protocol} • {new Date(alert.timestamp).toLocaleTimeString()}
                        </div>
                      </div>
                      {!alert.acknowledged && (
                        <button
                          onClick={() => acknowledgeAlert(alert.id)}
                          className="text-xs text-blue-400 hover:text-blue-300 ml-2"
                        >
                          Ack
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {userTier === 'basic' && alerts.length > 5 && (
            <div className="p-4 bg-gray-800/30 border-t border-gray-700 text-center">
              <p className="text-sm text-gray-400 mb-2">
                Showing 5 of {alerts.length} alerts
              </p>
              <button className="text-blue-400 hover:text-blue-300 text-sm font-medium">
                Upgrade to see all alerts →
              </button>
            </div>
          )}
        </div>

        {/* Position Risk Heatmap */}
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg">
          <div className="p-6 border-b border-gray-700">
            <h3 className="text-lg font-semibold text-white">Position Risk Heatmap</h3>
          </div>
          
          <div className="p-4 space-y-3">
            {positionRisks.map((position) => (
              <div key={position.id} className="bg-gray-800/50 rounded-lg p-4">
                <div className="flex items-center justify-between mb-3">
                  <div>
                    <div className="text-sm font-medium text-white">{position.protocol}</div>
                    <div className="text-xs text-gray-400">{position.pair}</div>
                  </div>
                  <div className="flex items-center space-x-2">
                    <span className="text-xs">{getTrendIcon(position.trend)}</span>
                    <span className={`px-2 py-1 rounded text-xs font-medium border ${getRiskColor(position.risk_score)}`}>
                      {position.risk_score}
                    </span>
                    {position.alerts > 0 && (
                      <span className="bg-red-600 text-white text-xs px-1 rounded-full">
                        {position.alerts}
                      </span>
                    )}
                  </div>
                </div>
                
                {/* Risk Factor Bars */}
                <div className="grid grid-cols-2 gap-2 text-xs">
                  {Object.entries(position.risk_factors).map(([factor, value]) => {
                    const numValue = value as number;
                    return (
                      <div key={factor} className="flex items-center space-x-2">
                        <span className="text-gray-400 w-16 capitalize">{factor}:</span>
                        <div className="flex-1 bg-gray-700 rounded-full h-1">
                          <div
                            className={`h-1 rounded-full transition-all ${
                              numValue >= 80 ? 'bg-red-500' :
                              numValue >= 60 ? 'bg-orange-500' :
                              numValue >= 30 ? 'bg-yellow-500' : 'bg-green-500'
                            }`}
                            style={{ width: `${numValue}%` }}
                          ></div>
                        </div>
                        <span className="text-gray-300 w-6">{numValue}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};

export default RealTimeRiskMonitor;
