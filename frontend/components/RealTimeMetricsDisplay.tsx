/**
 * RealTimeMetricsDisplay Component
 * 
 * Real-time risk metrics display with live updates, trend indicators,
 * and animated transitions for risk score changes
 */

import React, { useState, useEffect, useRef } from 'react';
import { RiskMetrics } from '../lib/api-client';
import { LoadingSpinner } from './Icons';

interface RealTimeMetricsDisplayProps {
  metrics: RiskMetrics | null;
  isConnected: boolean;
  lastUpdate: Date | null;
  className?: string;
}

interface MetricTrend {
  current: number;
  previous: number;
  change: number;
  trend: 'up' | 'down' | 'stable';
}

const RealTimeMetricsDisplay: React.FC<RealTimeMetricsDisplayProps> = ({
  metrics,
  isConnected,
  lastUpdate,
  className = ''
}) => {
  const [trends, setTrends] = useState<Record<string, MetricTrend>>({});
  const [isAnimating, setIsAnimating] = useState(false);
  const previousMetrics = useRef<RiskMetrics | null>(null);

  // Track metric changes and calculate trends
  useEffect(() => {
    if (metrics && previousMetrics.current) {
      const newTrends: Record<string, MetricTrend> = {};
      
      const metricKeys = [
        'overall_risk_score',
        'liquidity_risk',
        'volatility_risk',
        'protocol_risk',
        'mev_risk',
        'cross_chain_risk',
        'impermanent_loss_risk'
      ];

      metricKeys.forEach(key => {
        const current = parseFloat(metrics[key as keyof RiskMetrics] as string);
        const previous = parseFloat(previousMetrics.current![key as keyof RiskMetrics] as string);
        const change = current - previous;
        
        newTrends[key] = {
          current,
          previous,
          change,
          trend: Math.abs(change) < 0.1 ? 'stable' : change > 0 ? 'up' : 'down'
        };
      });

      setTrends(newTrends);
      setIsAnimating(true);
      setTimeout(() => setIsAnimating(false), 1000);
    }
    
    previousMetrics.current = metrics;
  }, [metrics]);

  const getRiskColor = (score: number) => {
    if (score >= 80) return 'text-red-400 bg-red-900/20 border-red-500/30';
    if (score >= 60) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
    if (score >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
    return 'text-green-400 bg-green-900/20 border-green-500/30';
  };

  const getTrendIcon = (trend: 'up' | 'down' | 'stable') => {
    switch (trend) {
      case 'up': return '↗️';
      case 'down': return '↘️';
      case 'stable': return '➡️';
    }
  };

  const getTrendColor = (trend: 'up' | 'down' | 'stable') => {
    switch (trend) {
      case 'up': return 'text-red-400';
      case 'down': return 'text-green-400';
      case 'stable': return 'text-gray-400';
    }
  };

  const formatChange = (change: number) => {
    const sign = change > 0 ? '+' : '';
    return `${sign}${change.toFixed(1)}%`;
  };

  if (!metrics) {
    return (
      <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
        <div className="flex items-center justify-center py-12">
          <LoadingSpinner className="w-8 h-8" />
          <span className="ml-2 text-gray-400">Loading real-time metrics...</span>
        </div>
      </div>
    );
  }

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header with Connection Status */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Real-Time Risk Metrics</h3>
          <div className="flex items-center gap-2 mt-1">
            <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-xs text-gray-400">
              {isConnected ? 'Live' : 'Disconnected'}
            </span>
            {lastUpdate && (
              <span className="text-xs text-gray-500 ml-2">
                Updated: {lastUpdate.toLocaleTimeString()}
              </span>
            )}
          </div>
        </div>
        
        {isAnimating && (
          <div className="flex items-center gap-1 text-blue-400">
            <LoadingSpinner className="w-4 h-4" />
            <span className="text-xs">Updating...</span>
          </div>
        )}
      </div>

      {/* Overall Risk Score - Large Display */}
      <div className="mb-6">
        <div className="text-center">
          <div className="text-sm text-gray-400 mb-2">Overall Risk Score</div>
          <div className={`inline-flex items-center gap-3 px-6 py-4 rounded-xl border transition-all duration-500 ${
            isAnimating ? 'scale-105' : 'scale-100'
          } ${getRiskColor(parseFloat(metrics.overall_risk_score))}`}>
            <span className="text-4xl font-bold">
              {parseFloat(metrics.overall_risk_score).toFixed(1)}%
            </span>
            {trends.overall_risk_score && (
              <div className="flex flex-col items-center">
                <span className="text-lg">
                  {getTrendIcon(trends.overall_risk_score.trend)}
                </span>
                <span className={`text-xs ${getTrendColor(trends.overall_risk_score.trend)}`}>
                  {formatChange(trends.overall_risk_score.change)}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Risk Factors Grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
        {[
          { key: 'liquidity_risk', label: 'Liquidity Risk' },
          { key: 'volatility_risk', label: 'Volatility Risk' },
          { key: 'protocol_risk', label: 'Protocol Risk' },
          { key: 'mev_risk', label: 'MEV Risk' },
          { key: 'cross_chain_risk', label: 'Cross-Chain Risk' },
          { key: 'impermanent_loss_risk', label: 'IL Risk' }
        ].map(({ key, label }) => {
          const value = parseFloat(metrics[key as keyof RiskMetrics] as string);
          const trend = trends[key];
          
          return (
            <div
              key={key}
              className={`bg-gray-900/50 rounded-lg p-4 border border-gray-600 transition-all duration-300 ${
                isAnimating ? 'transform scale-105' : ''
              }`}
            >
              <div className="flex items-center justify-between mb-2">
                <div className="text-xs text-gray-400">{label}</div>
                {trend && (
                  <div className="flex items-center gap-1">
                    <span className="text-xs">{getTrendIcon(trend.trend)}</span>
                    <span className={`text-xs ${getTrendColor(trend.trend)}`}>
                      {formatChange(trend.change)}
                    </span>
                  </div>
                )}
              </div>
              
              <div className={`text-lg font-semibold ${getRiskColor(value).split(' ')[0]}`}>
                {value.toFixed(1)}%
              </div>
              
              {/* Mini Progress Bar */}
              <div className="w-full bg-gray-700 rounded-full h-1 mt-2">
                <div
                  className={`h-1 rounded-full transition-all duration-500 ${
                    value >= 80 ? 'bg-red-500' :
                    value >= 60 ? 'bg-orange-500' :
                    value >= 30 ? 'bg-yellow-500' :
                    'bg-green-500'
                  }`}
                  style={{ width: `${Math.min(value, 100)}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* Risk Level Indicator */}
      <div className="mt-6 p-4 bg-gray-900/30 rounded-lg border border-gray-600">
        <div className="flex items-center justify-between">
          <div>
            <div className="text-sm font-medium text-white">Risk Level</div>
            <div className="text-xs text-gray-400 mt-1">
              Based on overall risk score and market conditions
            </div>
          </div>
          <div className={`px-3 py-1 rounded-lg text-sm font-medium ${
            parseFloat(metrics.overall_risk_score) >= 80 ? 'bg-red-900/50 text-red-400' :
            parseFloat(metrics.overall_risk_score) >= 60 ? 'bg-orange-900/50 text-orange-400' :
            parseFloat(metrics.overall_risk_score) >= 30 ? 'bg-yellow-900/50 text-yellow-400' :
            'bg-green-900/50 text-green-400'
          }`}>
            {parseFloat(metrics.overall_risk_score) >= 80 ? 'CRITICAL' :
             parseFloat(metrics.overall_risk_score) >= 60 ? 'HIGH' :
             parseFloat(metrics.overall_risk_score) >= 30 ? 'MEDIUM' :
             'LOW'}
          </div>
        </div>
      </div>

      {/* Update Frequency Info */}
      <div className="mt-4 text-xs text-gray-500 text-center">
        {isConnected ? (
          'Metrics update every 30 seconds via WebSocket connection'
        ) : (
          'Reconnecting to real-time data feed...'
        )}
      </div>
    </div>
  );
};

export default RealTimeMetricsDisplay;
