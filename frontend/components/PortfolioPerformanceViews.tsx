/**
 * PortfolioPerformanceViews Component
 * 
 * Comprehensive portfolio performance analytics with multiple views:
 * - Performance overview with key metrics
 * - Asset allocation breakdown
 * - P&L analysis and trends
 * - Protocol exposure analysis
 * - Risk-adjusted returns
 */

import React, { useState, useEffect, useMemo } from 'react';
import { Line, Pie, Column, Area } from '@ant-design/charts';
import { TrendingUpIcon, TrendingDownIcon, DollarSignIcon, PieChartIcon, BarChartIcon, InfoIcon } from './Icons';

interface PortfolioPerformanceViewsProps {
  userAddress?: string;
  className?: string;
}

interface PerformanceMetrics {
  totalValue: number;
  totalPnL: number;
  pnlPercentage: number;
  realizedPnL: number;
  unrealizedPnL: number;
  totalFees: number;
  impermanentLoss: number;
  sharpeRatio: number;
  maxDrawdown: number;
  winRate: number;
}

interface AssetAllocation {
  symbol: string;
  value: number;
  percentage: number;
  pnl: number;
  pnlPercentage: number;
  riskScore: number;
}

interface ProtocolExposure {
  protocol: string;
  value: number;
  percentage: number;
  positions: number;
  avgRisk: number;
  yield: number;
}

interface PnLDataPoint {
  timestamp: string;
  cumulativePnL: number;
  realizedPnL: number;
  unrealizedPnL: number;
  portfolioValue: number;
}

const PortfolioPerformanceViews: React.FC<PortfolioPerformanceViewsProps> = ({
  userAddress,
  className = ''
}) => {
  const [activeView, setActiveView] = useState<'overview' | 'allocation' | 'pnl' | 'protocols' | 'risk'>('overview');
  const [timeRange, setTimeRange] = useState<'24h' | '7d' | '30d' | '90d' | '1y'>('30d');
  const [isLoading, setIsLoading] = useState(false);

  // Mock data - in real app, this would come from API
  const [performanceMetrics, setPerformanceMetrics] = useState<PerformanceMetrics>({
    totalValue: 125420.50,
    totalPnL: 15420.50,
    pnlPercentage: 14.05,
    realizedPnL: 8920.30,
    unrealizedPnL: 6500.20,
    totalFees: 2340.80,
    impermanentLoss: -1240.60,
    sharpeRatio: 1.85,
    maxDrawdown: -8.2,
    winRate: 68.5
  });

  const [assetAllocation, setAssetAllocation] = useState<AssetAllocation[]>([
    { symbol: 'ETH', value: 45680.20, percentage: 36.4, pnl: 5420.30, pnlPercentage: 13.5, riskScore: 65 },
    { symbol: 'USDC', value: 32150.80, percentage: 25.6, pnl: 1240.50, pnlPercentage: 4.0, riskScore: 15 },
    { symbol: 'WBTC', value: 28940.10, percentage: 23.1, pnl: 6890.20, pnlPercentage: 31.2, riskScore: 72 },
    { symbol: 'USDT', value: 12450.30, percentage: 9.9, pnl: 890.40, pnlPercentage: 7.7, riskScore: 18 },
    { symbol: 'DAI', value: 6199.10, percentage: 4.9, pnl: 979.10, pnlPercentage: 18.8, riskScore: 22 }
  ]);

  const [protocolExposure, setProtocolExposure] = useState<ProtocolExposure[]>([
    { protocol: 'Uniswap V3', value: 52340.20, percentage: 41.7, positions: 8, avgRisk: 58, yield: 12.4 },
    { protocol: 'Aave', value: 28950.80, percentage: 23.1, positions: 3, avgRisk: 35, yield: 8.2 },
    { protocol: 'Compound', value: 22140.30, percentage: 17.6, positions: 4, avgRisk: 42, yield: 6.8 },
    { protocol: 'Curve', value: 15680.90, percentage: 12.5, positions: 5, avgRisk: 28, yield: 15.6 },
    { protocol: 'Balancer', value: 6308.30, percentage: 5.0, positions: 2, avgRisk: 48, yield: 9.3 }
  ]);

  const [pnlHistory, setPnlHistory] = useState<PnLDataPoint[]>([]);

  // Generate mock P&L history data
  useEffect(() => {
    const generatePnLHistory = () => {
      const now = new Date();
      const dataPoints: PnLDataPoint[] = [];
      let intervals: number;
      let intervalMs: number;

      switch (timeRange) {
        case '24h':
          intervals = 24;
          intervalMs = 60 * 60 * 1000; // 1 hour
          break;
        case '7d':
          intervals = 28;
          intervalMs = 6 * 60 * 60 * 1000; // 6 hours
          break;
        case '30d':
          intervals = 30;
          intervalMs = 24 * 60 * 60 * 1000; // 1 day
          break;
        case '90d':
          intervals = 30;
          intervalMs = 3 * 24 * 60 * 60 * 1000; // 3 days
          break;
        case '1y':
          intervals = 52;
          intervalMs = 7 * 24 * 60 * 60 * 1000; // 1 week
          break;
        default:
          intervals = 30;
          intervalMs = 24 * 60 * 60 * 1000;
      }

      let cumulativePnL = 0;
      let portfolioValue = 110000;

      for (let i = intervals; i >= 0; i--) {
        const timestamp = new Date(now.getTime() - (i * intervalMs));
        
        // Generate realistic P&L progression
        const dailyReturn = (Math.random() - 0.45) * 0.05; // Slightly positive bias
        const dailyPnL = portfolioValue * dailyReturn;
        cumulativePnL += dailyPnL;
        portfolioValue += dailyPnL;

        const realizedRatio = Math.random() * 0.6 + 0.2; // 20-80% realized
        
        dataPoints.push({
          timestamp: timestamp.toISOString(),
          cumulativePnL,
          realizedPnL: cumulativePnL * realizedRatio,
          unrealizedPnL: cumulativePnL * (1 - realizedRatio),
          portfolioValue
        });
      }

      setPnlHistory(dataPoints);
    };

    generatePnLHistory();
  }, [timeRange]);

  // Chart configurations
  const pnlChartConfig = {
    data: pnlHistory.map(point => [
      {
        timestamp: point.timestamp,
        type: 'Realized P&L',
        value: point.realizedPnL
      },
      {
        timestamp: point.timestamp,
        type: 'Unrealized P&L',
        value: point.unrealizedPnL
      },
      {
        timestamp: point.timestamp,
        type: 'Total P&L',
        value: point.cumulativePnL
      }
    ]).flat(),
    xField: 'timestamp',
    yField: 'value',
    seriesField: 'type',
    smooth: true,
    color: ['#22c55e', '#f59e0b', '#3b82f6'],
    xAxis: {
      type: 'time',
      label: {
        formatter: (text: string) => {
          const date = new Date(text);
          return timeRange === '24h' 
            ? date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
            : date.toLocaleDateString([], { month: 'short', day: 'numeric' });
        }
      }
    },
    yAxis: {
      label: {
        formatter: (text: string) => `$${(parseFloat(text) / 1000).toFixed(1)}K`
      }
    },
    tooltip: {
      formatter: (datum: any) => ({
        name: datum.type,
        value: `$${datum.value.toLocaleString()}`
      })
    }
  };

  const allocationChartConfig = {
    data: assetAllocation,
    angleField: 'value',
    colorField: 'symbol',
    radius: 0.8,
    innerRadius: 0.4,
    label: {
      type: 'outer',
      content: '{name} {percentage}%'
    },
    interactions: [{ type: 'element-selected' }, { type: 'element-active' }],
    statistic: {
      title: false,
      content: {
        style: {
          whiteSpace: 'pre-wrap',
          overflow: 'hidden',
          textOverflow: 'ellipsis',
        },
        content: 'Total\n$125.4K',
      },
    },
  };

  const protocolChartConfig = {
    data: protocolExposure,
    xField: 'protocol',
    yField: 'value',
    color: '#3b82f6',
    columnWidthRatio: 0.6,
    label: {
      position: 'middle' as const,
      style: {
        fill: '#FFFFFF',
        opacity: 0.8,
      },
      formatter: (datum: any) => `$${(datum.value / 1000).toFixed(0)}K`
    },
    xAxis: {
      label: {
        autoRotate: false,
      },
    },
    yAxis: {
      label: {
        formatter: (text: string) => `$${(parseFloat(text) / 1000).toFixed(0)}K`
      }
    }
  };

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(value);
  };

  const formatPercentage = (value: number) => {
    return `${value >= 0 ? '+' : ''}${value.toFixed(2)}%`;
  };

  const getPerformanceColor = (value: number) => {
    return value >= 0 ? 'text-green-400' : 'text-red-400';
  };

  const renderOverview = () => (
    <div className="space-y-6">
      {/* Key Metrics Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Total Value</span>
            <DollarSignIcon className="w-4 h-4 text-blue-400" />
          </div>
          <div className="text-2xl font-bold text-white">
            {formatCurrency(performanceMetrics.totalValue)}
          </div>
        </div>

        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Total P&L</span>
            {performanceMetrics.totalPnL >= 0 ? 
              <TrendingUpIcon className="w-4 h-4 text-green-400" /> :
              <TrendingDownIcon className="w-4 h-4 text-red-400" />
            }
          </div>
          <div className={`text-2xl font-bold ${getPerformanceColor(performanceMetrics.totalPnL)}`}>
            {formatCurrency(performanceMetrics.totalPnL)}
          </div>
          <div className={`text-sm ${getPerformanceColor(performanceMetrics.pnlPercentage)}`}>
            {formatPercentage(performanceMetrics.pnlPercentage)}
          </div>
        </div>

        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Sharpe Ratio</span>
            <InfoIcon className="w-4 h-4 text-gray-400" />
          </div>
          <div className="text-2xl font-bold text-white">
            {performanceMetrics.sharpeRatio.toFixed(2)}
          </div>
          <div className="text-sm text-gray-400">Risk-adjusted return</div>
        </div>

        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm text-gray-400">Win Rate</span>
            <BarChartIcon className="w-4 h-4 text-green-400" />
          </div>
          <div className="text-2xl font-bold text-white">
            {performanceMetrics.winRate.toFixed(1)}%
          </div>
          <div className="text-sm text-gray-400">Profitable positions</div>
        </div>
      </div>

      {/* Additional Metrics */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-3">P&L Breakdown</h4>
          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Realized</span>
              <span className={`text-sm ${getPerformanceColor(performanceMetrics.realizedPnL)}`}>
                {formatCurrency(performanceMetrics.realizedPnL)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Unrealized</span>
              <span className={`text-sm ${getPerformanceColor(performanceMetrics.unrealizedPnL)}`}>
                {formatCurrency(performanceMetrics.unrealizedPnL)}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Fees Earned</span>
              <span className="text-sm text-green-400">
                {formatCurrency(performanceMetrics.totalFees)}
              </span>
            </div>
            <div className="flex justify-between border-t border-gray-600 pt-2">
              <span className="text-sm text-gray-400">IL Impact</span>
              <span className="text-sm text-red-400">
                {formatCurrency(performanceMetrics.impermanentLoss)}
              </span>
            </div>
          </div>
        </div>

        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Risk Metrics</h4>
          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Max Drawdown</span>
              <span className="text-sm text-red-400">
                {performanceMetrics.maxDrawdown.toFixed(1)}%
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Volatility</span>
              <span className="text-sm text-yellow-400">24.5%</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Beta</span>
              <span className="text-sm text-blue-400">1.12</span>
            </div>
          </div>
        </div>

        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Portfolio Stats</h4>
          <div className="space-y-2">
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Total Positions</span>
              <span className="text-sm text-white">22</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Active Protocols</span>
              <span className="text-sm text-white">5</span>
            </div>
            <div className="flex justify-between">
              <span className="text-sm text-gray-400">Avg Position Size</span>
              <span className="text-sm text-white">$5.7K</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  const renderAllocation = () => (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Pie Chart */}
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-4">Asset Distribution</h4>
          <div className="h-64">
            <Pie {...allocationChartConfig} />
          </div>
        </div>

        {/* Asset Details */}
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-4">Asset Performance</h4>
          <div className="space-y-3">
            {assetAllocation.map((asset) => (
              <div key={asset.symbol} className="flex items-center justify-between p-3 bg-gray-800/50 rounded-lg">
                <div className="flex items-center gap-3">
                  <div className="w-8 h-8 bg-blue-600 rounded-full flex items-center justify-center text-xs font-bold text-white">
                    {asset.symbol.slice(0, 2)}
                  </div>
                  <div>
                    <div className="font-medium text-white">{asset.symbol}</div>
                    <div className="text-xs text-gray-400">{asset.percentage.toFixed(1)}%</div>
                  </div>
                </div>
                
                <div className="text-right">
                  <div className="font-medium text-white">
                    {formatCurrency(asset.value)}
                  </div>
                  <div className={`text-xs ${getPerformanceColor(asset.pnl)}`}>
                    {formatCurrency(asset.pnl)} ({formatPercentage(asset.pnlPercentage)})
                  </div>
                </div>
                
                <div className="text-right">
                  <div className="text-xs text-gray-400">Risk</div>
                  <div className={`text-xs ${
                    asset.riskScore > 70 ? 'text-red-400' :
                    asset.riskScore > 40 ? 'text-yellow-400' : 'text-green-400'
                  }`}>
                    {asset.riskScore}%
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );

  const renderPnL = () => (
    <div className="space-y-6">
      <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
        <h4 className="text-sm font-medium text-gray-300 mb-4">P&L Trends</h4>
        <div className="h-64">
          <Line {...pnlChartConfig} />
        </div>
      </div>
    </div>
  );

  const renderProtocols = () => (
    <div className="space-y-6">
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Protocol Chart */}
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-4">Protocol Exposure</h4>
          <div className="h-64">
            <Column {...protocolChartConfig} />
          </div>
        </div>

        {/* Protocol Details */}
        <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
          <h4 className="text-sm font-medium text-gray-300 mb-4">Protocol Analysis</h4>
          <div className="space-y-3">
            {protocolExposure.map((protocol) => (
              <div key={protocol.protocol} className="p-3 bg-gray-800/50 rounded-lg">
                <div className="flex items-center justify-between mb-2">
                  <span className="font-medium text-white">{protocol.protocol}</span>
                  <span className="text-sm text-gray-400">{protocol.percentage.toFixed(1)}%</span>
                </div>
                
                <div className="grid grid-cols-3 gap-4 text-xs">
                  <div>
                    <div className="text-gray-400">Value</div>
                    <div className="text-white font-medium">
                      {formatCurrency(protocol.value)}
                    </div>
                  </div>
                  <div>
                    <div className="text-gray-400">Positions</div>
                    <div className="text-white font-medium">{protocol.positions}</div>
                  </div>
                  <div>
                    <div className="text-gray-400">Avg Yield</div>
                    <div className="text-green-400 font-medium">{protocol.yield.toFixed(1)}%</div>
                  </div>
                </div>
                
                <div className="mt-2">
                  <div className="flex justify-between text-xs mb-1">
                    <span className="text-gray-400">Risk Score</span>
                    <span className={`${
                      protocol.avgRisk > 70 ? 'text-red-400' :
                      protocol.avgRisk > 40 ? 'text-yellow-400' : 'text-green-400'
                    }`}>
                      {protocol.avgRisk}%
                    </span>
                  </div>
                  <div className="w-full bg-gray-700 rounded-full h-1">
                    <div 
                      className={`h-1 rounded-full ${
                        protocol.avgRisk > 70 ? 'bg-red-500' :
                        protocol.avgRisk > 40 ? 'bg-yellow-500' : 'bg-green-500'
                      }`}
                      style={{ width: `${protocol.avgRisk}%` }}
                    />
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between mb-6 gap-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Portfolio Performance</h3>
          <p className="text-sm text-gray-400 mt-1">
            Comprehensive portfolio analytics and performance tracking
          </p>
        </div>
        
        <div className="flex items-center gap-3">
          <select
            value={timeRange}
            onChange={(e) => setTimeRange(e.target.value as any)}
            className="bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white"
          >
            <option value="24h">24 Hours</option>
            <option value="7d">7 Days</option>
            <option value="30d">30 Days</option>
            <option value="90d">90 Days</option>
            <option value="1y">1 Year</option>
          </select>
        </div>
      </div>

      {/* Tab Navigation */}
      <div className="flex border-b border-gray-700 mb-6 overflow-x-auto">
        {[
          { key: 'overview', label: 'Overview', icon: BarChartIcon },
          { key: 'allocation', label: 'Asset Allocation', icon: PieChartIcon },
          { key: 'pnl', label: 'P&L Analysis', icon: TrendingUpIcon },
          { key: 'protocols', label: 'Protocol Exposure', icon: DollarSignIcon }
        ].map(({ key, label, icon: Icon }) => (
          <button
            key={key}
            onClick={() => setActiveView(key as any)}
            className={`flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors whitespace-nowrap ${
              activeView === key
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-gray-300'
            }`}
          >
            <Icon className="w-4 h-4" />
            {label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="min-h-[500px]">
        {activeView === 'overview' && renderOverview()}
        {activeView === 'allocation' && renderAllocation()}
        {activeView === 'pnl' && renderPnL()}
        {activeView === 'protocols' && renderProtocols()}
      </div>
    </div>
  );
};

export default PortfolioPerformanceViews;
