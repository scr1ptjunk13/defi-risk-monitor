/**
 * PositionCard Component
 * 
 * Enhanced position display card with real-time risk metrics,
 * performance indicators, and management actions
 */

import React, { useState } from 'react';
import { Position, RiskMetrics } from '../lib/api-client';
import { LoadingSpinner, InfoIcon } from './Icons';
import RiskMetricsCard from './RiskMetricsCard';

interface PositionCardProps {
  position: Position;
  riskMetrics?: RiskMetrics;
  isLoadingRisk?: boolean;
  onViewDetails?: (positionId: string) => void;
  onCalculateRisk?: (positionId: string) => void;
  onExplainRisk?: (positionId: string) => void;
  className?: string;
}

const PositionCard: React.FC<PositionCardProps> = ({
  position,
  riskMetrics,
  isLoadingRisk = false,
  onViewDetails,
  onCalculateRisk,
  onExplainRisk,
  className = ''
}) => {
  const [showRiskDetails, setShowRiskDetails] = useState(false);

  // Helper functions
  const formatCurrency = (value: string | number) => {
    const num = typeof value === 'string' ? parseFloat(value) : value;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(num);
  };

  const formatPercentage = (value: string | number) => {
    const num = typeof value === 'string' ? parseFloat(value) : value;
    return `${num.toFixed(2)}%`;
  };

  const getRiskColor = (score: string | number) => {
    const numScore = typeof score === 'string' ? parseFloat(score) : score;
    if (numScore >= 80) return 'text-red-400 bg-red-900/20 border-red-500/30';
    if (numScore >= 60) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
    if (numScore >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
    return 'text-green-400 bg-green-900/20 border-green-500/30';
  };

  const getChainInfo = (address: string) => {
    // Simple chain detection based on address patterns
    // In a real implementation, this would be more sophisticated
    return {
      name: 'Ethereum',
      icon: '⟠',
      color: 'text-blue-400'
    };
  };

  const chainInfo = getChainInfo(position.pool_address);

  // Calculate P&L and performance metrics
  const currentValue = parseFloat(position.current_value_usd);
  const entryValue = parseFloat(position.liquidity_amount);
  const pnl = currentValue - entryValue;
  const pnlPercentage = (pnl / entryValue) * 100;
  const impermanentLoss = parseFloat(position.impermanent_loss_pct);

  return (
    <div className={`bg-gray-800/50 rounded-xl border border-gray-700 overflow-hidden ${className}`}>
      {/* Header */}
      <div className="p-6 border-b border-gray-700">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2">
              <div className="w-8 h-8 bg-gray-700 rounded-full flex items-center justify-center">
                <span className="text-xs font-bold text-white">
                  {position.token0_symbol.charAt(0)}
                </span>
              </div>
              <div className="w-8 h-8 bg-gray-700 rounded-full flex items-center justify-center -ml-2">
                <span className="text-xs font-bold text-white">
                  {position.token1_symbol.charAt(0)}
                </span>
              </div>
            </div>
            <div>
              <h3 className="text-lg font-semibold text-white">
                {position.token0_symbol}/{position.token1_symbol}
              </h3>
              <div className="flex items-center gap-2 text-sm text-gray-400">
                <span className={chainInfo.color}>{chainInfo.icon}</span>
                <span>{chainInfo.name}</span>
                <span>•</span>
                <span>{formatPercentage(position.fee_tier)} Fee</span>
              </div>
            </div>
          </div>

          {/* Risk Score Badge */}
          {riskMetrics && (
            <div className={`px-3 py-2 rounded-lg border ${getRiskColor(riskMetrics.overall_risk_score)}`}>
              <div className="text-center">
                <div className="text-lg font-bold">
                  {parseFloat(riskMetrics.overall_risk_score).toFixed(1)}%
                </div>
                <div className="text-xs opacity-75">Risk</div>
              </div>
            </div>
          )}
        </div>

        {/* Position Metrics */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="bg-gray-900/50 rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Liquidity</div>
            <div className="text-sm font-semibold text-white">
              {formatCurrency(position.liquidity_amount)}
            </div>
          </div>
          
          <div className="bg-gray-900/50 rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Current Value</div>
            <div className="text-sm font-semibold text-white">
              {formatCurrency(position.current_value_usd)}
            </div>
          </div>

          <div className="bg-gray-900/50 rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">P&L</div>
            <div className={`text-sm font-semibold ${pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              {pnl >= 0 ? '+' : ''}{formatCurrency(pnl)}
            </div>
            <div className={`text-xs ${pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              ({pnl >= 0 ? '+' : ''}{pnlPercentage.toFixed(2)}%)
            </div>
          </div>

          <div className="bg-gray-900/50 rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Impermanent Loss</div>
            <div className={`text-sm font-semibold ${impermanentLoss < 0 ? 'text-red-400' : 'text-gray-400'}`}>
              {formatPercentage(impermanentLoss)}
            </div>
          </div>
        </div>
      </div>

      {/* Risk Metrics Section */}
      {riskMetrics && (
        <div className="p-6 border-b border-gray-700">
          <div className="flex items-center justify-between mb-4">
            <h4 className="text-md font-medium text-white">Risk Breakdown</h4>
            <button
              onClick={() => setShowRiskDetails(!showRiskDetails)}
              className="text-xs text-blue-400 hover:text-blue-300"
            >
              {showRiskDetails ? 'Hide Details' : 'Show Details'}
            </button>
          </div>

          {/* Quick Risk Overview */}
          <div className="grid grid-cols-3 gap-3 mb-4">
            <div className="text-center">
              <div className="text-xs text-gray-400 mb-1">MEV Risk</div>
              <div className={`text-sm font-semibold ${getRiskColor(riskMetrics.mev_risk).split(' ')[0]}`}>
                {parseFloat(riskMetrics.mev_risk).toFixed(1)}%
              </div>
            </div>
            <div className="text-center">
              <div className="text-xs text-gray-400 mb-1">Liquidity Risk</div>
              <div className={`text-sm font-semibold ${getRiskColor(riskMetrics.liquidity_risk).split(' ')[0]}`}>
                {parseFloat(riskMetrics.liquidity_risk).toFixed(1)}%
              </div>
            </div>
            <div className="text-center">
              <div className="text-xs text-gray-400 mb-1">Protocol Risk</div>
              <div className={`text-sm font-semibold ${getRiskColor(riskMetrics.protocol_risk).split(' ')[0]}`}>
                {parseFloat(riskMetrics.protocol_risk).toFixed(1)}%
              </div>
            </div>
          </div>

          {/* Detailed Risk Metrics */}
          {showRiskDetails && (
            <RiskMetricsCard
              metrics={riskMetrics}
              onExplainRisk={() => onExplainRisk?.(position.id)}
              className="mt-4"
            />
          )}
        </div>
      )}

      {/* Actions */}
      <div className="p-6">
        <div className="flex items-center justify-between">
          <div className="text-xs text-gray-400">
            Created: {new Date(position.created_at).toLocaleDateString()}
            {position.updated_at && (
              <span className="ml-4">
                Updated: {new Date(position.updated_at).toLocaleDateString()}
              </span>
            )}
          </div>
          
          <div className="flex items-center gap-2">
            {onCalculateRisk && (
              <button
                onClick={() => onCalculateRisk(position.id)}
                disabled={isLoadingRisk}
                className="flex items-center gap-1 px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded-lg transition-colors"
              >
                {isLoadingRisk ? <LoadingSpinner className="w-3 h-3" /> : <InfoIcon className="w-3 h-3" />}
                {isLoadingRisk ? 'Calculating...' : 'Update Risk'}
              </button>
            )}
            
            {onViewDetails && (
              <button
                onClick={() => onViewDetails(position.id)}
                className="px-3 py-1 text-xs bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors"
              >
                View Details
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

export default PositionCard;
