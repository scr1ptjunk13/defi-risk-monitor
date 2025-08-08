'use client';

import React, { useState, useEffect, useRef } from 'react';

interface RealTimeRiskMonitorProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

interface RiskMetrics {
  overallRisk: number;
  liquidityRisk: number;
  volatilityRisk: number;
  mevRisk: number;
  protocolRisk: number;
  crossChainRisk: number;
  timestamp: number;
}

interface RiskAlert {
  id: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  type: string;
  message: string;
  positionId?: string;
  protocol?: string;
  timestamp: number;
  acknowledged: boolean;
}

interface PositionRisk {
  id: string;
  protocol: string;
  pair: string;
  riskScore: number;
  riskFactors: {
    liquidity: number;
    volatility: number;
    mev: number;
    protocol: number;
  };
  alerts: number;
  trend: 'up' | 'down' | 'stable';
}

const RealTimeRiskMonitor: React.FC<RealTimeRiskMonitorProps> = ({ userAddress, userTier }) => {
  const [riskMetrics, setRiskMetrics] = useState<RiskMetrics | null>(null);
  const [alerts, setAlerts] = useState<RiskAlert[]>([]);
  const [positionRisks, setPositionRisks] = useState<PositionRisk[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<number>(Date.now());
  const wsRef = useRef<WebSocket | null>(null);

  // WebSocket connection for real-time updates
  useEffect(() => {
    // Simulate WebSocket connection - replace with actual WebSocket
    const connectWebSocket = () => {
      // Mock WebSocket behavior
      setIsConnected(true);
      
      // Simulate real-time risk updates
      const interval = setInterval(() => {
        const now = Date.now();
        
        // Update risk metrics with slight variations
        setRiskMetrics(prev => ({
          overallRisk: Math.max(0, Math.min(100, (prev?.overallRisk || 75) + (Math.random() - 0.5) * 5)),
          liquidityRisk: Math.max(0, Math.min(100, (prev?.liquidityRisk || 65) + (Math.random() - 0.5) * 8)),
          volatilityRisk: Math.max(0, Math.min(100, (prev?.volatilityRisk || 72) + (Math.random() - 0.5) * 6)),
          mevRisk: Math.max(0, Math.min(100, (prev?.mevRisk || 82) + (Math.random() - 0.5) * 4)),
          protocolRisk: Math.max(0, Math.min(100, (prev?.protocolRisk || 45) + (Math.random() - 0.5) * 3)),
          crossChainRisk: Math.max(0, Math.min(100, (prev?.crossChainRisk || 58) + (Math.random() - 0.5) * 7)),
          timestamp: now
        }));

        // Occasionally add new alerts
        if (Math.random() < 0.1) {
          const alertTypes = [
            { type: 'MEV Risk', severity: 'high' as const, message: 'High MEV vulnerability detected in ETH/USDC pool' },
            { type: 'Liquidity Risk', severity: 'medium' as const, message: 'Pool liquidity decreased by 15%' },
            { type: 'Protocol Risk', severity: 'low' as const, message: 'Protocol upgrade scheduled' },
            { type: 'Cross-Chain Risk', severity: 'critical' as const, message: 'Bridge congestion detected' }
          ];
          
          const randomAlert = alertTypes[Math.floor(Math.random() * alertTypes.length)];
          const newAlert: RiskAlert = {
            id: `alert_${now}`,
            ...randomAlert,
            timestamp: now,
            acknowledged: false,
            positionId: `pos_${Math.floor(Math.random() * 5) + 1}`,
            protocol: ['Uniswap V3', 'Aave', 'Curve', 'PancakeSwap'][Math.floor(Math.random() * 4)]
          };
          
          setAlerts(prev => [newAlert, ...prev.slice(0, 19)]);
        }

        setLastUpdate(now);
      }, 2000);

      return () => clearInterval(interval);
    };

    const cleanup = connectWebSocket();

    // Initial data load
    setRiskMetrics({
      overallRisk: 78,
      liquidityRisk: 65,
      volatilityRisk: 72,
      mevRisk: 82,
      protocolRisk: 45,
      crossChainRisk: 58,
      timestamp: Date.now()
    });

    setPositionRisks([
      {
        id: '1',
        protocol: 'Uniswap V3',
        pair: 'ETH/USDC',
        riskScore: 65,
        riskFactors: { liquidity: 60, volatility: 70, mev: 80, protocol: 40 },
        alerts: 2,
        trend: 'up'
      },
      {
        id: '2',
        protocol: 'Aave',
        pair: 'WETH Supply',
        riskScore: 42,
        riskFactors: { liquidity: 30, volatility: 45, mev: 20, protocol: 50 },
        alerts: 0,
        trend: 'stable'
      },
      {
        id: '3',
        protocol: 'Curve',
        pair: 'stETH/ETH',
        riskScore: 58,
        riskFactors: { liquidity: 55, volatility: 60, mev: 45, protocol: 35 },
        alerts: 1,
        trend: 'down'
      },
      {
        id: '4',
        protocol: 'PancakeSwap',
        pair: 'BNB/BUSD',
        riskScore: 73,
        riskFactors: { liquidity: 75, volatility: 70, mev: 85, protocol: 60 },
        alerts: 3,
        trend: 'up'
      }
    ]);

    return cleanup;
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
          <div className={`px-3 py-1 rounded-full text-sm font-medium border ${getRiskColor(riskMetrics?.overallRisk || 0)}`}>
            {Math.round(riskMetrics?.overallRisk || 0)}/100
          </div>
        </div>
        
        {/* Risk Factors Breakdown */}
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {[
            { key: 'liquidityRisk', label: 'Liquidity', value: riskMetrics?.liquidityRisk || 0 },
            { key: 'volatilityRisk', label: 'Volatility', value: riskMetrics?.volatilityRisk || 0 },
            { key: 'mevRisk', label: 'MEV', value: riskMetrics?.mevRisk || 0 },
            { key: 'protocolRisk', label: 'Protocol', value: riskMetrics?.protocolRisk || 0 },
            { key: 'crossChainRisk', label: 'Cross-Chain', value: riskMetrics?.crossChainRisk || 0 }
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
                          <span className="text-sm text-gray-400">{alert.type}</span>
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
                    <span className={`px-2 py-1 rounded text-xs font-medium border ${getRiskColor(position.riskScore)}`}>
                      {position.riskScore}
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
                  {Object.entries(position.riskFactors).map(([factor, value]) => (
                    <div key={factor} className="flex items-center space-x-2">
                      <span className="text-gray-400 w-16 capitalize">{factor}:</span>
                      <div className="flex-1 bg-gray-700 rounded-full h-1">
                        <div
                          className={`h-1 rounded-full transition-all ${
                            value >= 80 ? 'bg-red-500' :
                            value >= 60 ? 'bg-orange-500' :
                            value >= 30 ? 'bg-yellow-500' : 'bg-green-500'
                          }`}
                          style={{ width: `${value}%` }}
                        ></div>
                      </div>
                      <span className="text-gray-300 w-6">{value}</span>
                    </div>
                  ))}
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
