/**
 * PositionDetails Component
 * 
 * Comprehensive detailed view for individual DeFi positions
 * with risk analysis, performance metrics, and historical data
 */

import React, { useState, useEffect } from 'react';
import { Position, RiskMetrics, RiskExplanation } from '../lib/api-client';
import { toast } from 'react-hot-toast';
import { 
  ArrowLeftIcon,
  EditIcon,
  TrashIcon,
  ExternalLinkIcon,
  AlertTriangleIcon,
  TrendingUpIcon,
  TrendingDownIcon,
  InfoIcon,
  RefreshIcon,
  CopyIcon,
  ShareIcon
} from './Icons';

// Import other components for detailed views
import RiskFactorBreakdown from './RiskFactorBreakdown';
import HistoricalRiskCharts from './HistoricalRiskCharts';

interface PositionDetailsProps {
  position: Position;
  riskMetrics?: RiskMetrics;
  riskExplanation?: RiskExplanation;
  onBack: () => void;
  onEdit: (position: Position) => void;
  onDelete: (positionId: string) => void;
  onRefresh: (positionId: string) => void;
  className?: string;
}

interface PositionMetrics {
  currentValue: number;
  entryValue: number;
  pnl: number;
  pnlPercentage: number;
  feesEarned: number;
  impermanentLoss: number;
  totalReturn: number;
}

const CHAIN_NAMES = {
  1: 'Ethereum',
  56: 'BSC',
  137: 'Polygon',
  42161: 'Arbitrum',
  10: 'Optimism',
  43114: 'Avalanche'
};

const CHAIN_EXPLORERS = {
  1: 'https://etherscan.io',
  56: 'https://bscscan.com',
  137: 'https://polygonscan.com',
  42161: 'https://arbiscan.io',
  10: 'https://optimistic.etherscan.io',
  43114: 'https://snowtrace.io'
};

const PositionDetails: React.FC<PositionDetailsProps> = ({
  position,
  riskMetrics,
  riskExplanation,
  onBack,
  onEdit,
  onDelete,
  onRefresh,
  className = ''
}) => {
  const [activeTab, setActiveTab] = useState<'overview' | 'risk' | 'history' | 'transactions'>('overview');
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Calculate position metrics
  const metrics: PositionMetrics = React.useMemo(() => {
    const currentValue = parseFloat(position.current_value_usd) || 0;
    const entryValue = parseFloat(position.entry_price_usd || position.current_value_usd) || currentValue;
    const pnl = currentValue - entryValue;
    const pnlPercentage = entryValue > 0 ? (pnl / entryValue) * 100 : 0;
    
    // Mock data for fees and IL (would come from API in real implementation)
    const feesEarned = currentValue * 0.02; // 2% estimated fees
    const impermanentLoss = currentValue * 0.01; // 1% estimated IL
    const totalReturn = pnl + feesEarned - impermanentLoss;

    return {
      currentValue,
      entryValue,
      pnl,
      pnlPercentage,
      feesEarned,
      impermanentLoss,
      totalReturn
    };
  }, [position]);

  // Calculate overall risk score
  const riskScore = React.useMemo(() => {
    if (!riskMetrics) return 0;
    
    const scores = [
      parseFloat(riskMetrics.volatility_risk) || 0,
      parseFloat(riskMetrics.liquidity_risk) || 0,
      parseFloat(riskMetrics.protocol_risk) || 0,
      parseFloat(riskMetrics.mev_risk) || 0,
      parseFloat(riskMetrics.cross_chain_risk) || 0,
      parseFloat(riskMetrics.impermanent_loss_risk) || 0
    ];
    
    return scores.reduce((sum, score) => sum + score, 0) / scores.length;
  }, [riskMetrics]);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await onRefresh(position.id);
      toast.success('Position data refreshed');
    } catch (error) {
      toast.error('Failed to refresh position data');
    } finally {
      setIsRefreshing(false);
    }
  };

  const handleCopyAddress = (address: string, type: string) => {
    navigator.clipboard.writeText(address);
    toast.success(`${type} address copied to clipboard`);
  };

  const handleShare = () => {
    const url = `${window.location.origin}/positions/${position.id}`;
    navigator.clipboard.writeText(url);
    toast.success('Position link copied to clipboard');
  };

  const getRiskLevelInfo = (score: number) => {
    if (score <= 30) return { level: 'Low', color: 'text-green-400', bg: 'bg-green-900/20', border: 'border-green-500/30' };
    if (score <= 60) return { level: 'Medium', color: 'text-yellow-400', bg: 'bg-yellow-900/20', border: 'border-yellow-500/30' };
    if (score <= 80) return { level: 'High', color: 'text-orange-400', bg: 'bg-orange-900/20', border: 'border-orange-500/30' };
    return { level: 'Critical', color: 'text-red-400', bg: 'bg-red-900/20', border: 'border-red-500/30' };
  };

  const riskInfo = getRiskLevelInfo(riskScore);
  const explorerUrl = CHAIN_EXPLORERS[position.chain_id as keyof typeof CHAIN_EXPLORERS];

  const tabs = [
    { id: 'overview', name: 'Overview', description: 'Position summary and metrics' },
    { id: 'risk', name: 'Risk Analysis', description: 'Detailed risk breakdown' },
    { id: 'history', name: 'History', description: 'Historical performance' },
    { id: 'transactions', name: 'Transactions', description: 'Transaction history' }
  ];

  return (
    <div className={`bg-gray-800/50 rounded-xl border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="p-6 border-b border-gray-700">
        <div className="flex items-center justify-between mb-4">
          <button
            onClick={onBack}
            className="flex items-center gap-2 text-gray-400 hover:text-white transition-colors"
          >
            <ArrowLeftIcon className="w-4 h-4" />
            Back to Positions
          </button>

          <div className="flex items-center gap-2">
            <button
              onClick={handleRefresh}
              disabled={isRefreshing}
              className="p-2 text-gray-400 hover:text-white transition-colors disabled:opacity-50"
              title="Refresh Data"
            >
              <RefreshIcon className={`w-4 h-4 ${isRefreshing ? 'animate-spin' : ''}`} />
            </button>
            <button
              onClick={handleShare}
              className="p-2 text-gray-400 hover:text-white transition-colors"
              title="Share Position"
            >
              <ShareIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onEdit(position)}
              className="p-2 text-gray-400 hover:text-yellow-400 transition-colors"
              title="Edit Position"
            >
              <EditIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onDelete(position.id)}
              className="p-2 text-gray-400 hover:text-red-400 transition-colors"
              title="Delete Position"
            >
              <TrashIcon className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Position Header */}
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <h1 className="text-2xl font-bold text-white mb-2">
              {position.token0_symbol}/{position.token1_symbol}
            </h1>
            <div className="flex items-center gap-4 text-sm text-gray-400">
              <span>{position.protocol}</span>
              <span>•</span>
              <span>{CHAIN_NAMES[position.chain_id as keyof typeof CHAIN_NAMES]}</span>
              <span>•</span>
              <span>Fee Tier: {position.fee_tier}%</span>
              <span>•</span>
              <div className={`flex items-center gap-1 ${
                position.is_active ? 'text-green-400' : 'text-gray-400'
              }`}>
                <div className={`w-2 h-2 rounded-full ${
                  position.is_active ? 'bg-green-500' : 'bg-gray-500'
                }`} />
                {position.is_active ? 'Active' : 'Inactive'}
              </div>
            </div>
          </div>

          <div className={`px-4 py-2 rounded-lg border ${riskInfo.bg} ${riskInfo.color} ${riskInfo.border}`}>
            <div className="text-center">
              <div className="text-lg font-semibold">{riskInfo.level} Risk</div>
              <div className="text-sm opacity-80">{riskScore.toFixed(1)}% Score</div>
            </div>
          </div>
        </div>
      </div>

      {/* Key Metrics */}
      <div className="p-6 border-b border-gray-700">
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
          <div className="text-center">
            <p className="text-sm text-gray-400 mb-1">Current Value</p>
            <p className="text-2xl font-bold text-white">
              ${metrics.currentValue.toLocaleString()}
            </p>
          </div>

          <div className="text-center">
            <p className="text-sm text-gray-400 mb-1">P&L</p>
            <div className={`flex items-center justify-center gap-1 text-2xl font-bold ${
              metrics.pnl >= 0 ? 'text-green-400' : 'text-red-400'
            }`}>
              {metrics.pnl >= 0 ? <TrendingUpIcon className="w-6 h-6" /> : <TrendingDownIcon className="w-6 h-6" />}
              {metrics.pnl >= 0 ? '+' : ''}${metrics.pnl.toLocaleString()}
            </div>
            <p className={`text-sm ${metrics.pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
              {metrics.pnlPercentage >= 0 ? '+' : ''}{metrics.pnlPercentage.toFixed(2)}%
            </p>
          </div>

          <div className="text-center">
            <p className="text-sm text-gray-400 mb-1">Fees Earned</p>
            <p className="text-2xl font-bold text-green-400">
              +${metrics.feesEarned.toLocaleString()}
            </p>
          </div>

          <div className="text-center">
            <p className="text-sm text-gray-400 mb-1">Impermanent Loss</p>
            <p className="text-2xl font-bold text-red-400">
              -${metrics.impermanentLoss.toLocaleString()}
            </p>
          </div>
        </div>
      </div>

      {/* Navigation Tabs */}
      <div className="border-b border-gray-700">
        <div className="flex overflow-x-auto">
          {tabs.map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`flex-shrink-0 px-6 py-4 text-sm font-medium border-b-2 transition-colors ${
                activeTab === tab.id
                  ? 'border-blue-500 text-blue-400'
                  : 'border-transparent text-gray-400 hover:text-gray-300'
              }`}
            >
              <div>
                <div>{tab.name}</div>
                <div className="text-xs text-gray-500 mt-1">{tab.description}</div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Tab Content */}
      <div className="p-6">
        {activeTab === 'overview' && (
          <div className="space-y-6">
            {/* Position Information */}
            <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
              <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
                <h3 className="text-lg font-semibold text-white mb-4">Position Details</h3>
                <div className="space-y-3">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Pool Address</span>
                    <div className="flex items-center gap-2">
                      <span className="text-white font-mono text-sm">
                        {position.pool_address?.slice(0, 6)}...{position.pool_address?.slice(-4)}
                      </span>
                      <button
                        onClick={() => handleCopyAddress(position.pool_address || '', 'Pool')}
                        className="text-gray-400 hover:text-white transition-colors"
                      >
                        <CopyIcon className="w-4 h-4" />
                      </button>
                      {explorerUrl && (
                        <a
                          href={`${explorerUrl}/address/${position.pool_address}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-gray-400 hover:text-blue-400 transition-colors"
                        >
                          <ExternalLinkIcon className="w-4 h-4" />
                        </a>
                      )}
                    </div>
                  </div>

                  <div className="flex justify-between">
                    <span className="text-gray-400">Liquidity Amount</span>
                    <span className="text-white">{parseFloat(position.liquidity_amount || '0').toLocaleString()}</span>
                  </div>

                  <div className="flex justify-between">
                    <span className="text-gray-400">Entry Price</span>
                    <span className="text-white">${parseFloat(position.entry_price_usd || '0').toLocaleString()}</span>
                  </div>

                  {position.position_range_lower && position.position_range_upper && (
                    <div className="flex justify-between">
                      <span className="text-gray-400">Price Range</span>
                      <span className="text-white">
                        ${parseFloat(position.position_range_lower).toLocaleString()} - ${parseFloat(position.position_range_upper).toLocaleString()}
                      </span>
                    </div>
                  )}

                  <div className="flex justify-between">
                    <span className="text-gray-400">Created</span>
                    <span className="text-white">
                      {new Date(position.created_at || '').toLocaleDateString()}
                    </span>
                  </div>
                </div>
              </div>

              <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
                <h3 className="text-lg font-semibold text-white mb-4">Token Information</h3>
                <div className="space-y-4">
                  <div>
                    <h4 className="text-sm font-medium text-gray-300 mb-2">{position.token0_symbol}</h4>
                    <div className="flex items-center gap-2">
                      <span className="text-white font-mono text-sm">
                        {position.token0_address?.slice(0, 6)}...{position.token0_address?.slice(-4)}
                      </span>
                      <button
                        onClick={() => handleCopyAddress(position.token0_address || '', position.token0_symbol || 'Token 0')}
                        className="text-gray-400 hover:text-white transition-colors"
                      >
                        <CopyIcon className="w-4 h-4" />
                      </button>
                      {explorerUrl && (
                        <a
                          href={`${explorerUrl}/token/${position.token0_address}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-gray-400 hover:text-blue-400 transition-colors"
                        >
                          <ExternalLinkIcon className="w-4 h-4" />
                        </a>
                      )}
                    </div>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium text-gray-300 mb-2">{position.token1_symbol}</h4>
                    <div className="flex items-center gap-2">
                      <span className="text-white font-mono text-sm">
                        {position.token1_address?.slice(0, 6)}...{position.token1_address?.slice(-4)}
                      </span>
                      <button
                        onClick={() => handleCopyAddress(position.token1_address || '', position.token1_symbol || 'Token 1')}
                        className="text-gray-400 hover:text-white transition-colors"
                      >
                        <CopyIcon className="w-4 h-4" />
                      </button>
                      {explorerUrl && (
                        <a
                          href={`${explorerUrl}/token/${position.token1_address}`}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-gray-400 hover:text-blue-400 transition-colors"
                        >
                          <ExternalLinkIcon className="w-4 h-4" />
                        </a>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Performance Summary */}
            <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
              <h3 className="text-lg font-semibold text-white mb-4">Performance Summary</h3>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div className="text-center p-4 bg-gray-800/50 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">Total Return</p>
                  <p className={`text-xl font-bold ${
                    metrics.totalReturn >= 0 ? 'text-green-400' : 'text-red-400'
                  }`}>
                    {metrics.totalReturn >= 0 ? '+' : ''}${metrics.totalReturn.toLocaleString()}
                  </p>
                </div>
                <div className="text-center p-4 bg-gray-800/50 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">ROI</p>
                  <p className={`text-xl font-bold ${
                    metrics.totalReturn >= 0 ? 'text-green-400' : 'text-red-400'
                  }`}>
                    {((metrics.totalReturn / metrics.entryValue) * 100).toFixed(2)}%
                  </p>
                </div>
                <div className="text-center p-4 bg-gray-800/50 rounded-lg">
                  <p className="text-sm text-gray-400 mb-1">Risk Score</p>
                  <p className={`text-xl font-bold ${riskInfo.color}`}>
                    {riskScore.toFixed(1)}%
                  </p>
                </div>
              </div>
            </div>

            {/* Notes */}
            {position.notes && (
              <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
                <h3 className="text-lg font-semibold text-white mb-2">Notes</h3>
                <p className="text-gray-300">{position.notes}</p>
              </div>
            )}
          </div>
        )}

        {activeTab === 'risk' && (
          <div>
            <RiskFactorBreakdown
              riskMetrics={riskMetrics}
              positionId={position.id}
              className="mb-6"
            />
            
            {riskExplanation && (
              <div className="bg-blue-900/10 border border-blue-500/30 rounded-lg p-4">
                <div className="flex items-start gap-2">
                  <InfoIcon className="w-5 h-5 text-blue-400 mt-0.5 flex-shrink-0" />
                  <div>
                    <h4 className="text-blue-400 font-medium mb-2">Risk Explanation</h4>
                    <p className="text-blue-300/80 text-sm leading-relaxed">
                      {riskExplanation.overall_assessment}
                    </p>
                    {riskExplanation.recommendations && riskExplanation.recommendations.length > 0 && (
                      <div className="mt-3">
                        <p className="text-blue-400 text-sm font-medium mb-1">Recommendations:</p>
                        <ul className="text-blue-300/80 text-sm space-y-1">
                          {riskExplanation.recommendations.map((rec, index) => (
                            <li key={index} className="flex items-start gap-2">
                              <span className="text-blue-400 mt-1">•</span>
                              <span>{rec}</span>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}
          </div>
        )}

        {activeTab === 'history' && (
          <div>
            <HistoricalRiskCharts
              positionId={position.id}
              userAddress={position.user_address}
              className="mb-6"
            />
          </div>
        )}

        {activeTab === 'transactions' && (
          <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
            <h3 className="text-lg font-semibold text-white mb-4">Transaction History</h3>
            <div className="text-center py-8">
              <AlertTriangleIcon className="w-12 h-12 mx-auto text-gray-400 mb-4" />
              <h4 className="text-lg font-medium text-white mb-2">Coming Soon</h4>
              <p className="text-gray-400">
                Transaction history integration is in development
              </p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default PositionDetails;
