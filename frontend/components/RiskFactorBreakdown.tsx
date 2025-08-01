/**
 * RiskFactorBreakdown Component
 * 
 * Detailed breakdown and visualization of individual risk factors
 * with explanations, trends, and actionable insights
 */

import React, { useState, useEffect } from 'react';
import { RiskMetrics } from '../lib/api-client';
import { InfoIcon, TrendingUpIcon, TrendingDownIcon, AlertTriangleIcon } from './Icons';

interface RiskFactorBreakdownProps {
  riskMetrics?: RiskMetrics;
  positionId?: string;
  className?: string;
}

interface RiskFactor {
  id: string;
  name: string;
  value: number;
  trend: 'up' | 'down' | 'stable';
  trendValue: number;
  severity: 'low' | 'medium' | 'high' | 'critical';
  description: string;
  factors: string[];
  recommendations: string[];
  historicalData: { timestamp: string; value: number }[];
}

interface RiskCategory {
  id: string;
  name: string;
  description: string;
  weight: number;
  factors: RiskFactor[];
}

const RiskFactorBreakdown: React.FC<RiskFactorBreakdownProps> = ({
  riskMetrics,
  positionId,
  className = ''
}) => {
  const [selectedCategory, setSelectedCategory] = useState<string>('market');
  const [selectedFactor, setSelectedFactor] = useState<RiskFactor | null>(null);
  const [timeRange, setTimeRange] = useState<'1h' | '24h' | '7d' | '30d'>('24h');

  // Mock risk categories and factors (in real app, this would come from riskMetrics)
  const riskCategories: RiskCategory[] = [
    {
      id: 'market',
      name: 'Market Risk',
      description: 'Risks related to market conditions and price movements',
      weight: 0.35,
      factors: [
        {
          id: 'volatility',
          name: 'Price Volatility',
          value: riskMetrics ? parseFloat(riskMetrics.volatility_risk) || 65 : 65,
          trend: 'up',
          trendValue: 5.2,
          severity: 'medium',
          description: 'Measures the price volatility of underlying assets',
          factors: [
            'ETH volatility: 45% (24h)',
            'USDC stability: 99.8%',
            'Correlation coefficient: 0.23'
          ],
          recommendations: [
            'Consider reducing position size during high volatility',
            'Set tighter stop-loss orders',
            'Monitor correlation with major market movements'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 60 },
            { timestamp: '2024-01-01T06:00:00Z', value: 62 },
            { timestamp: '2024-01-01T12:00:00Z', value: 65 },
            { timestamp: '2024-01-01T18:00:00Z', value: 67 }
          ]
        },
        {
          id: 'liquidity',
          name: 'Liquidity Risk',
          value: riskMetrics ? parseFloat(riskMetrics.liquidity_risk) || 42 : 42,
          trend: 'down',
          trendValue: -2.1,
          severity: 'low',
          description: 'Risk of not being able to exit positions at fair prices',
          factors: [
            'Pool TVL: $2.4M',
            'Daily volume: $450K',
            'Bid-ask spread: 0.05%'
          ],
          recommendations: [
            'Liquidity is adequate for current position size',
            'Monitor during market stress events',
            'Consider partial exits during low volume periods'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 45 },
            { timestamp: '2024-01-01T06:00:00Z', value: 44 },
            { timestamp: '2024-01-01T12:00:00Z', value: 43 },
            { timestamp: '2024-01-01T18:00:00Z', value: 42 }
          ]
        }
      ]
    },
    {
      id: 'protocol',
      name: 'Protocol Risk',
      description: 'Risks specific to the DeFi protocol being used',
      weight: 0.25,
      factors: [
        {
          id: 'smart_contract',
          name: 'Smart Contract Risk',
          value: riskMetrics ? parseFloat(riskMetrics.protocol_risk) || 38 : 38,
          trend: 'stable',
          trendValue: 0.1,
          severity: 'low',
          description: 'Risk of smart contract bugs or exploits',
          factors: [
            'Protocol age: 2.5 years',
            'Total audits: 4',
            'Bug bounty: $2M max',
            'No recent exploits'
          ],
          recommendations: [
            'Protocol has strong security track record',
            'Monitor for any protocol updates',
            'Consider diversifying across protocols'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 38 },
            { timestamp: '2024-01-01T06:00:00Z', value: 38 },
            { timestamp: '2024-01-01T12:00:00Z', value: 38 },
            { timestamp: '2024-01-01T18:00:00Z', value: 38 }
          ]
        },
        {
          id: 'governance',
          name: 'Governance Risk',
          value: 25,
          trend: 'down',
          trendValue: -1.5,
          severity: 'low',
          description: 'Risk from governance decisions affecting the protocol',
          factors: [
            'Governance token distribution: Moderate',
            'Voting participation: 45%',
            'Recent proposals: 3 (all passed)',
            'Time lock: 48 hours'
          ],
          recommendations: [
            'Monitor governance proposals',
            'Participate in voting if holding governance tokens',
            'Stay informed about protocol changes'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 27 },
            { timestamp: '2024-01-01T06:00:00Z', value: 26 },
            { timestamp: '2024-01-01T12:00:00Z', value: 25 },
            { timestamp: '2024-01-01T18:00:00Z', value: 25 }
          ]
        }
      ]
    },
    {
      id: 'operational',
      name: 'Operational Risk',
      description: 'Risks from operational factors and external dependencies',
      weight: 0.20,
      factors: [
        {
          id: 'mev',
          name: 'MEV Risk',
          value: riskMetrics ? parseFloat(riskMetrics.mev_risk) || 72 : 72,
          trend: 'up',
          trendValue: 8.3,
          severity: 'high',
          description: 'Risk of value extraction by miners/validators',
          factors: [
            'Position visibility: High',
            'Arbitrage opportunities: Moderate',
            'Sandwich attack risk: 15%',
            'MEV protection: None'
          ],
          recommendations: [
            'Use MEV protection services',
            'Consider private mempools',
            'Split large transactions',
            'Monitor for unusual slippage'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 64 },
            { timestamp: '2024-01-01T06:00:00Z', value: 68 },
            { timestamp: '2024-01-01T12:00:00Z', value: 70 },
            { timestamp: '2024-01-01T18:00:00Z', value: 72 }
          ]
        },
        {
          id: 'cross_chain',
          name: 'Cross-Chain Risk',
          value: riskMetrics ? parseFloat(riskMetrics.cross_chain_risk) || 55 : 55,
          trend: 'stable',
          trendValue: 0.5,
          severity: 'medium',
          description: 'Risks from cross-chain operations and bridges',
          factors: [
            'Bridge security score: 7.5/10',
            'Cross-chain exposure: 30%',
            'Bridge TVL: $150M',
            'Recent incidents: 0'
          ],
          recommendations: [
            'Monitor bridge health regularly',
            'Consider native chain alternatives',
            'Limit cross-chain exposure',
            'Use multiple bridges for diversification'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 55 },
            { timestamp: '2024-01-01T06:00:00Z', value: 55 },
            { timestamp: '2024-01-01T12:00:00Z', value: 55 },
            { timestamp: '2024-01-01T18:00:00Z', value: 55 }
          ]
        }
      ]
    },
    {
      id: 'financial',
      name: 'Financial Risk',
      description: 'Financial performance and loss-related risks',
      weight: 0.20,
      factors: [
        {
          id: 'impermanent_loss',
          name: 'Impermanent Loss',
          value: riskMetrics ? parseFloat(riskMetrics.impermanent_loss_risk) || 28 : 28,
          trend: 'down',
          trendValue: -3.2,
          severity: 'low',
          description: 'Potential loss from providing liquidity vs holding assets',
          factors: [
            'Current IL: 2.8%',
            'Price divergence: 12%',
            'Time in position: 15 days',
            'Fee earnings offset: 65%'
          ],
          recommendations: [
            'IL risk is currently manageable',
            'Monitor price divergence closely',
            'Consider rebalancing if divergence increases',
            'Fee earnings are helping offset IL'
          ],
          historicalData: [
            { timestamp: '2024-01-01T00:00:00Z', value: 32 },
            { timestamp: '2024-01-01T06:00:00Z', value: 30 },
            { timestamp: '2024-01-01T12:00:00Z', value: 29 },
            { timestamp: '2024-01-01T18:00:00Z', value: 28 }
          ]
        }
      ]
    }
  ];

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'low': return 'text-green-400 bg-green-900/20 border-green-500/30';
      case 'medium': return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
      case 'high': return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
      case 'critical': return 'text-red-400 bg-red-900/20 border-red-500/30';
      default: return 'text-gray-400 bg-gray-900/20 border-gray-500/30';
    }
  };

  const getTrendIcon = (trend: string, value: number) => {
    if (trend === 'up') {
      return <TrendingUpIcon className="w-4 h-4 text-red-400" />;
    } else if (trend === 'down') {
      return <TrendingDownIcon className="w-4 h-4 text-green-400" />;
    }
    return <div className="w-4 h-4 bg-gray-400 rounded-full" />;
  };

  const getRiskBar = (value: number) => {
    const percentage = Math.min(value, 100);
    let colorClass = 'bg-green-500';
    if (percentage > 70) colorClass = 'bg-red-500';
    else if (percentage > 40) colorClass = 'bg-yellow-500';

    return (
      <div className="w-full bg-gray-700 rounded-full h-2">
        <div 
          className={`h-2 rounded-full transition-all duration-300 ${colorClass}`}
          style={{ width: `${percentage}%` }}
        />
      </div>
    );
  };

  const selectedCategoryData = riskCategories.find(cat => cat.id === selectedCategory);

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Risk Factor Breakdown</h3>
          <p className="text-sm text-gray-400 mt-1">
            Detailed analysis of individual risk components
          </p>
        </div>
        
        <div className="flex items-center gap-2">
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value as any)}
            className="bg-gray-700 border border-gray-600 rounded-lg px-3 py-1 text-sm text-white"
          >
            <option value="1h">1 Hour</option>
            <option value="24h">24 Hours</option>
            <option value="7d">7 Days</option>
            <option value="30d">30 Days</option>
          </select>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Category Selector */}
        <div className="lg:col-span-1">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Risk Categories</h4>
          <div className="space-y-2">
            {riskCategories.map((category) => {
              const avgRisk = category.factors.reduce((sum, factor) => sum + factor.value, 0) / category.factors.length;
              
              return (
                <button
                  key={category.id}
                  onClick={() => setSelectedCategory(category.id)}
                  className={`w-full text-left p-3 rounded-lg border transition-all ${
                    selectedCategory === category.id
                      ? 'border-blue-500 bg-blue-900/20'
                      : 'border-gray-600 bg-gray-900/30 hover:bg-gray-900/50'
                  }`}
                >
                  <div className="flex items-center justify-between mb-2">
                    <span className="font-medium text-white">{category.name}</span>
                    <span className="text-sm text-gray-400">{Math.round(avgRisk)}%</span>
                  </div>
                  
                  <div className="mb-2">
                    {getRiskBar(avgRisk)}
                  </div>
                  
                  <p className="text-xs text-gray-400">{category.description}</p>
                  
                  <div className="flex items-center justify-between mt-2">
                    <span className="text-xs text-gray-500">
                      Weight: {Math.round(category.weight * 100)}%
                    </span>
                    <span className="text-xs text-gray-500">
                      {category.factors.length} factors
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        {/* Risk Factors */}
        <div className="lg:col-span-2">
          {selectedCategoryData && (
            <>
              <div className="flex items-center justify-between mb-4">
                <h4 className="text-sm font-medium text-gray-300">
                  {selectedCategoryData.name} Factors
                </h4>
                <span className="text-xs text-gray-400">
                  {selectedCategoryData.factors.length} factors analyzed
                </span>
              </div>

              <div className="space-y-4">
                {selectedCategoryData.factors.map((factor) => (
                  <div
                    key={factor.id}
                    className={`p-4 rounded-lg border cursor-pointer transition-all ${
                      selectedFactor?.id === factor.id
                        ? 'border-blue-500 bg-blue-900/10'
                        : 'border-gray-600 bg-gray-900/30 hover:bg-gray-900/50'
                    }`}
                    onClick={() => setSelectedFactor(selectedFactor?.id === factor.id ? null : factor)}
                  >
                    {/* Factor Header */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-3">
                        <span className="font-medium text-white">{factor.name}</span>
                        <div className={`px-2 py-1 rounded text-xs border ${getSeverityColor(factor.severity)}`}>
                          {factor.severity.toUpperCase()}
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-2">
                        {getTrendIcon(factor.trend, factor.trendValue)}
                        <span className={`text-sm ${
                          factor.trend === 'up' ? 'text-red-400' : 
                          factor.trend === 'down' ? 'text-green-400' : 'text-gray-400'
                        }`}>
                          {factor.trend === 'up' ? '+' : factor.trend === 'down' ? '' : '±'}
                          {Math.abs(factor.trendValue).toFixed(1)}%
                        </span>
                        <span className="text-lg font-semibold text-white">
                          {factor.value}%
                        </span>
                      </div>
                    </div>

                    {/* Risk Bar */}
                    <div className="mb-3">
                      {getRiskBar(factor.value)}
                    </div>

                    {/* Description */}
                    <p className="text-sm text-gray-400 mb-3">{factor.description}</p>

                    {/* Expanded Details */}
                    {selectedFactor?.id === factor.id && (
                      <div className="mt-4 pt-4 border-t border-gray-600">
                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                          {/* Contributing Factors */}
                          <div>
                            <h5 className="text-sm font-medium text-gray-300 mb-2">
                              Contributing Factors
                            </h5>
                            <ul className="space-y-1">
                              {factor.factors.map((item, index) => (
                                <li key={index} className="text-xs text-gray-400 flex items-start gap-2">
                                  <span className="w-1 h-1 bg-gray-500 rounded-full mt-2 flex-shrink-0" />
                                  {item}
                                </li>
                              ))}
                            </ul>
                          </div>

                          {/* Recommendations */}
                          <div>
                            <h5 className="text-sm font-medium text-gray-300 mb-2">
                              Recommendations
                            </h5>
                            <ul className="space-y-1">
                              {factor.recommendations.map((rec, index) => (
                                <li key={index} className="text-xs text-gray-400 flex items-start gap-2">
                                  <InfoIcon className="w-3 h-3 text-blue-400 mt-0.5 flex-shrink-0" />
                                  {rec}
                                </li>
                              ))}
                            </ul>
                          </div>
                        </div>

                        {/* Mini Chart */}
                        <div className="mt-4">
                          <h5 className="text-sm font-medium text-gray-300 mb-2">
                            Trend ({timeRange})
                          </h5>
                          <div className="h-16 bg-gray-900/50 rounded-lg p-2 flex items-end justify-between">
                            {factor.historicalData.map((point, index) => (
                              <div
                                key={index}
                                className="bg-blue-500 rounded-sm w-2 transition-all hover:bg-blue-400"
                                style={{ 
                                  height: `${(point.value / 100) * 100}%`,
                                  minHeight: '2px'
                                }}
                                title={`${point.value}% at ${new Date(point.timestamp).toLocaleTimeString()}`}
                              />
                            ))}
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </div>

      {/* Overall Risk Summary */}
      <div className="mt-6 pt-6 border-t border-gray-700">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
          {riskCategories.map((category) => {
            const avgRisk = category.factors.reduce((sum, factor) => sum + factor.value, 0) / category.factors.length;
            const weightedRisk = avgRisk * category.weight;
            
            return (
              <div key={category.id} className="text-center">
                <div className="text-2xl font-bold text-white mb-1">
                  {Math.round(avgRisk)}%
                </div>
                <div className="text-sm text-gray-400 mb-2">{category.name}</div>
                <div className="text-xs text-gray-500">
                  Weighted: {Math.round(weightedRisk)}%
                </div>
              </div>
            );
          })}
        </div>
        
        <div className="mt-4 text-center">
          <div className="text-xs text-gray-400 mb-1">Overall Weighted Risk Score</div>
          <div className="text-3xl font-bold text-white">
            {Math.round(riskCategories.reduce((sum, cat) => {
              const avgRisk = cat.factors.reduce((s, f) => s + f.value, 0) / cat.factors.length;
              return sum + (avgRisk * cat.weight);
            }, 0))}%
          </div>
        </div>
      </div>
    </div>
  );
};

export default RiskFactorBreakdown;
