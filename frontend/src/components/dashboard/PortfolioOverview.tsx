'use client';

import React, { useState, useEffect } from 'react';

interface PortfolioOverviewProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

interface PortfolioMetrics {
  totalValue: number;
  totalPnL: number;
  pnlPercentage: number;
  activePositions: number;
  protocols: number;
  riskScore: number;
  riskTrend: 'up' | 'down' | 'stable';
}

interface Position {
  id: string;
  protocol: string;
  pair: string;
  value: number;
  pnl: number;
  pnlPercentage: number;
  riskScore: number;
  chain: string;
}

const PortfolioOverview: React.FC<PortfolioOverviewProps> = ({ userAddress, userTier }) => {
  const [metrics, setMetrics] = useState<PortfolioMetrics | null>(null);
  const [positions, setPositions] = useState<Position[]>([]);
  const [loading, setLoading] = useState(true);
  const [timeRange, setTimeRange] = useState<'24h' | '7d' | '30d' | '90d'>('24h');

  useEffect(() => {
    // Simulate API call - replace with actual API integration
    const fetchPortfolioData = async () => {
      setLoading(true);
      
      // Mock data - replace with actual API calls
      setTimeout(() => {
        setMetrics({
          totalValue: 2847392.50,
          totalPnL: 128492.30,
          pnlPercentage: 4.72,
          activePositions: 5,
          protocols: 5,
          riskScore: 78,
          riskTrend: 'up'
        });

        setPositions([
          {
            id: '1',
            protocol: 'Uniswap V3',
            pair: 'ETH/USDC',
            value: 485920.30,
            pnl: 23847.20,
            pnlPercentage: 5.16,
            riskScore: 65,
            chain: 'Ethereum'
          },
          {
            id: '2',
            protocol: 'Aave V3',
            pair: 'WETH Supply',
            value: 324850.75,
            pnl: 18293.45,
            pnlPercentage: 5.96,
            riskScore: 42,
            chain: 'Ethereum'
          },
          {
            id: '3',
            protocol: 'Curve',
            pair: 'stETH/ETH',
            value: 298475.20,
            pnl: 12847.85,
            pnlPercentage: 4.49,
            riskScore: 58,
            chain: 'Ethereum'
          },
          {
            id: '4',
            protocol: 'Compound V3',
            pair: 'USDC Supply',
            value: 187394.60,
            pnl: 3847.20,
            pnlPercentage: 2.09,
            riskScore: 35,
            chain: 'Ethereum'
          },
          {
            id: '5',
            protocol: 'Lido',
            pair: 'stETH',
            value: 142850.45,
            pnl: 8394.75,
            pnlPercentage: 6.24,
            riskScore: 28,
            chain: 'Ethereum'
          }
        ]);
        
        setLoading(false);
      }, 1000);
    };

    fetchPortfolioData();
  }, [userAddress, timeRange]);

  const formatCurrency = (value: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 2,
      maximumFractionDigits: 2
    }).format(value);
  };

  const getRiskColor = (score: number) => {
    if (score >= 80) return 'text-red-400 bg-red-900/20 border-red-500/30';
    if (score >= 60) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
    if (score >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
    return 'text-green-400 bg-green-900/20 border-green-500/30';
  };

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'up': return '↗️';
      case 'down': return '↘️';
      default: return '➡️';
    }
  };

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="grid grid-cols-1 md:grid-cols-4 gap-6 mb-8">
            {[...Array(4)].map((_, i) => (
              <div key={i} className="bg-gray-800 rounded-lg h-32"></div>
            ))}
          </div>
          <div className="bg-gray-800 rounded-lg h-64"></div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with Time Range Selector */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Portfolio Overview</h2>
        <div className="flex space-x-2">
          {(['24h', '7d', '30d', '90d'] as const).map((range) => (
            <button
              key={range}
              onClick={() => setTimeRange(range)}
              className={`px-3 py-1 rounded text-sm transition-colors ${
                timeRange === range
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              {range}
            </button>
          ))}
        </div>
      </div>

      {/* Key Metrics Cards */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-sm text-gray-400 mb-2">Total Portfolio Value</div>
          <div className="text-2xl font-bold text-white mb-1">
            {formatCurrency(metrics?.totalValue || 0)}
          </div>
          <div className={`text-sm ${(metrics?.pnlPercentage || 0) >= 0 ? 'text-green-400' : 'text-red-400'}`}>
            {(metrics?.pnlPercentage || 0) >= 0 ? '+' : ''}{metrics?.pnlPercentage.toFixed(2)}% ({timeRange})
          </div>
        </div>

        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-sm text-gray-400 mb-2">Total P&L</div>
          <div className={`text-2xl font-bold mb-1 ${(metrics?.totalPnL || 0) >= 0 ? 'text-green-400' : 'text-red-400'}`}>
            {(metrics?.totalPnL || 0) >= 0 ? '+' : ''}{formatCurrency(metrics?.totalPnL || 0)}
          </div>
          <div className="text-sm text-gray-400">
            Realized + Unrealized
          </div>
        </div>

        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-sm text-gray-400 mb-2">Risk Score</div>
          <div className={`text-2xl font-bold mb-1 ${getRiskColor(metrics?.riskScore || 0).split(' ')[0]}`}>
            {metrics?.riskScore}/100
          </div>
          <div className="flex items-center text-sm text-gray-400">
            <span className="mr-1">{getTrendIcon(metrics?.riskTrend || 'stable')}</span>
            Risk Level: {(metrics?.riskScore || 0) >= 80 ? 'High' : (metrics?.riskScore || 0) >= 60 ? 'Medium' : 'Low'}
          </div>
        </div>

        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-sm text-gray-400 mb-2">Active Positions</div>
          <div className="text-2xl font-bold text-white mb-1">
            {metrics?.activePositions}
          </div>
          <div className="text-sm text-gray-400">
            {metrics?.protocols} protocols on Ethereum
          </div>
        </div>
      </div>

      {/* Positions Table */}
      <div className="bg-gray-900/50 border border-gray-700 rounded-lg overflow-hidden">
        <div className="p-6 border-b border-gray-700">
          <h3 className="text-lg font-semibold text-white">Top Positions</h3>
        </div>
        
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead className="bg-gray-800/50">
              <tr>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Position
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Value
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  P&L
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Risk Score
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Chain
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-700">
              {positions.slice(0, userTier === 'basic' ? 5 : positions.length).map((position) => (
                <tr key={position.id} className="hover:bg-gray-800/30">
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div>
                      <div className="text-sm font-medium text-white">{position.protocol}</div>
                      <div className="text-sm text-gray-400">{position.pair}</div>
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className="text-sm text-white">{formatCurrency(position.value)}</div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <div className={`text-sm ${position.pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                      {position.pnl >= 0 ? '+' : ''}{formatCurrency(position.pnl)}
                    </div>
                    <div className={`text-xs ${position.pnl >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                      ({position.pnl >= 0 ? '+' : ''}{position.pnlPercentage.toFixed(2)}%)
                    </div>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full border ${getRiskColor(position.riskScore)}`}>
                      {position.riskScore}
                    </span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap">
                    <span className="text-sm text-gray-300">{position.chain}</span>
                  </td>
                  <td className="px-6 py-4 whitespace-nowrap text-sm">
                    <button className="text-blue-400 hover:text-blue-300 mr-3">
                      View
                    </button>
                    <button className="text-gray-400 hover:text-gray-300">
                      Analyze
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {userTier === 'basic' && positions.length > 5 && (
          <div className="p-4 bg-gray-800/30 border-t border-gray-700 text-center">
            <p className="text-sm text-gray-400 mb-2">
              Showing 5 of {positions.length} positions
            </p>
            <button className="text-blue-400 hover:text-blue-300 text-sm font-medium">
              Upgrade to Professional to view all positions →
            </button>
          </div>
        )}
      </div>
    </div>
  );
};

export default PortfolioOverview;
