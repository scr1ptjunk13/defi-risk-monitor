/**
 * RiskMetricsCard Component
 * 
 * Displays detailed risk metrics for a position with visual indicators
 * and explanations for each risk factor
 */

import React, { useState } from 'react';
import { RiskMetrics, RiskExplanation } from '../lib/api-client';
import { InfoIcon, LoadingSpinner } from './Icons';

interface RiskMetricsCardProps {
  metrics: RiskMetrics;
  explanation?: RiskExplanation;
  isLoading?: boolean;
  className?: string;
  onExplainRisk?: () => void;
}

const RiskMetricsCard: React.FC<RiskMetricsCardProps> = ({
  metrics,
  explanation,
  isLoading = false,
  className = '',
  onExplainRisk
}) => {
  const [showDetails, setShowDetails] = useState(false);

  // Risk level helpers
  const getRiskLevel = (score: string | number): 'low' | 'medium' | 'high' | 'critical' => {
    const numScore = typeof score === 'string' ? parseFloat(score) : score;
    if (numScore >= 80) return 'critical';
    if (numScore >= 60) return 'high';
    if (numScore >= 30) return 'medium';
    return 'low';
  };

  const getRiskColor = (level: string) => {
    switch (level) {
      case 'critical': return 'text-red-400 bg-red-900/20 border-red-500/30';
      case 'high': return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
      case 'medium': return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
      case 'low': return 'text-green-400 bg-green-900/20 border-green-500/30';
      default: return 'text-gray-400 bg-gray-900/20 border-gray-500/30';
    }
  };

  const formatScore = (score: string) => parseFloat(score).toFixed(1);

  const riskFactors = [
    { key: 'overall_risk_score', label: 'Overall Risk', value: metrics.overall_risk_score },
    { key: 'liquidity_risk', label: 'Liquidity Risk', value: metrics.liquidity_risk },
    { key: 'volatility_risk', label: 'Volatility Risk', value: metrics.volatility_risk },
    { key: 'protocol_risk', label: 'Protocol Risk', value: metrics.protocol_risk },
    { key: 'mev_risk', label: 'MEV Risk', value: metrics.mev_risk },
    { key: 'cross_chain_risk', label: 'Cross-Chain Risk', value: metrics.cross_chain_risk },
    { key: 'impermanent_loss_risk', label: 'Impermanent Loss Risk', value: metrics.impermanent_loss_risk },
  ];

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-semibold text-white">Risk Analysis</h3>
        <div className="flex items-center gap-2">
          {onExplainRisk && (
            <button
              onClick={onExplainRisk}
              disabled={isLoading}
              className="flex items-center gap-1 px-3 py-1 text-xs bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded-lg transition-colors"
            >
              {isLoading ? <LoadingSpinner className="w-3 h-3" /> : <InfoIcon className="w-3 h-3" />}
              Explain
            </button>
          )}
          <button
            onClick={() => setShowDetails(!showDetails)}
            className="text-xs text-gray-400 hover:text-gray-300"
          >
            {showDetails ? 'Hide Details' : 'Show Details'}
          </button>
        </div>
      </div>

      {/* Overall Risk Score */}
      <div className="mb-6">
        <div className="flex items-center justify-between mb-2">
          <span className="text-sm text-gray-400">Overall Risk Score</span>
          <span className="text-xs text-gray-500">Last updated: {new Date().toLocaleTimeString()}</span>
        </div>
        <div className="flex items-center gap-4">
          <div className={`px-4 py-2 rounded-lg border ${getRiskColor(getRiskLevel(metrics.overall_risk_score))}`}>
            <span className="text-2xl font-bold">{formatScore(metrics.overall_risk_score)}%</span>
          </div>
          <div className="flex-1">
            <div className="w-full bg-gray-700 rounded-full h-2">
              <div
                className={`h-2 rounded-full transition-all duration-300 ${
                  getRiskLevel(metrics.overall_risk_score) === 'critical' ? 'bg-red-500' :
                  getRiskLevel(metrics.overall_risk_score) === 'high' ? 'bg-orange-500' :
                  getRiskLevel(metrics.overall_risk_score) === 'medium' ? 'bg-yellow-500' :
                  'bg-green-500'
                }`}
                style={{ width: `${Math.min(parseFloat(metrics.overall_risk_score), 100)}%` }}
              />
            </div>
            <div className="flex justify-between text-xs text-gray-500 mt-1">
              <span>Low</span>
              <span>Medium</span>
              <span>High</span>
              <span>Critical</span>
            </div>
          </div>
        </div>
      </div>

      {/* Risk Factors Grid */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4 mb-6">
        {riskFactors.slice(1).map(({ key, label, value }) => {
          const level = getRiskLevel(value);
          return (
            <div key={key} className="bg-gray-900/50 rounded-lg p-3 border border-gray-600">
              <div className="text-xs text-gray-400 mb-1">{label}</div>
              <div className={`text-lg font-semibold ${getRiskColor(level).split(' ')[0]}`}>
                {formatScore(value)}%
              </div>
              <div className="w-full bg-gray-700 rounded-full h-1 mt-2">
                <div
                  className={`h-1 rounded-full transition-all duration-300 ${
                    level === 'critical' ? 'bg-red-500' :
                    level === 'high' ? 'bg-orange-500' :
                    level === 'medium' ? 'bg-yellow-500' :
                    'bg-green-500'
                  }`}
                  style={{ width: `${Math.min(parseFloat(value), 100)}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* Detailed Explanation */}
      {showDetails && explanation && (
        <div className="border-t border-gray-700 pt-6">
          <h4 className="text-md font-semibold text-white mb-4">Risk Assessment Details</h4>
          
          {/* Overall Assessment */}
          <div className="mb-4 p-4 bg-gray-900/50 rounded-lg border border-gray-600">
            <h5 className="text-sm font-medium text-white mb-2">Overall Assessment</h5>
            <p className="text-sm text-gray-300">{explanation.overall_assessment}</p>
          </div>

          {/* Risk Factors */}
          <div className="space-y-3 mb-4">
            <h5 className="text-sm font-medium text-white">Risk Factor Analysis</h5>
            {explanation.risk_factors.map((factor, index) => (
              <div key={index} className="p-3 bg-gray-900/30 rounded-lg border border-gray-600">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm font-medium text-white">{factor.factor}</span>
                  <span className={`text-xs px-2 py-1 rounded ${getRiskColor(factor.severity)}`}>
                    {factor.severity} ({formatScore(factor.score)}%)
                  </span>
                </div>
                <p className="text-xs text-gray-400">{factor.explanation}</p>
              </div>
            ))}
          </div>

          {/* Recommendations */}
          {explanation.recommendations && explanation.recommendations.length > 0 && (
            <div className="mb-4">
              <h5 className="text-sm font-medium text-white mb-2">Recommendations</h5>
              <ul className="space-y-2">
                {explanation.recommendations.map((rec, index) => (
                  <li key={index} className="flex items-start gap-2 text-sm text-gray-300">
                    <span className="text-blue-400 mt-1">•</span>
                    <span>{rec}</span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* Market Context */}
          {explanation.market_context && (
            <div className="p-4 bg-blue-900/10 rounded-lg border border-blue-500/20">
              <h5 className="text-sm font-medium text-blue-400 mb-2">Market Context</h5>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs">
                <div>
                  <span className="text-gray-400">Conditions:</span>
                  <p className="text-gray-300 mt-1">{explanation.market_context.market_conditions}</p>
                </div>
                <div>
                  <span className="text-gray-400">Volatility:</span>
                  <p className="text-gray-300 mt-1">{explanation.market_context.volatility_outlook}</p>
                </div>
                <div>
                  <span className="text-gray-400">Liquidity:</span>
                  <p className="text-gray-300 mt-1">{explanation.market_context.liquidity_analysis}</p>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default RiskMetricsCard;
