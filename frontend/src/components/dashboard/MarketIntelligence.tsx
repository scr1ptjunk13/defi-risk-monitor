'use client';

import React, { useState, useEffect } from 'react';

interface MarketIntelligenceProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

interface MarketOpportunity {
  id: string;
  type: 'yield' | 'arbitrage' | 'liquidation' | 'new_protocol';
  title: string;
  description: string;
  apy: number;
  tvl: number;
  risk: number;
  timeframe: string;
  protocol: string;
  chain: string;
}

interface ProtocolHealth {
  name: string;
  tvl: number;
  tvlChange: number;
  volume24h: number;
  volumeChange: number;
  riskScore: number;
  governance: string;
  lastAudit: string;
}

interface MarketTrend {
  category: string;
  trend: 'bullish' | 'bearish' | 'neutral';
  strength: number;
  description: string;
  timeframe: string;
}

const MarketIntelligence: React.FC<MarketIntelligenceProps> = ({ userAddress, userTier }) => {
  const [activeView, setActiveView] = useState<'opportunities' | 'protocols' | 'trends' | 'sentiment'>('opportunities');
  const [opportunities, setOpportunities] = useState<MarketOpportunity[]>([]);
  const [protocolHealth, setProtocolHealth] = useState<ProtocolHealth[]>([]);
  const [marketTrends, setMarketTrends] = useState<MarketTrend[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Simulate API call for market intelligence data
    const fetchMarketData = async () => {
      setLoading(true);
      
      setTimeout(() => {
        setOpportunities([
          {
            id: '1',
            type: 'yield',
            title: 'High Yield stETH/ETH Pool',
            description: 'Curve stETH/ETH pool offering 8.2% APY with low impermanent loss risk',
            apy: 8.2,
            tvl: 1250000000,
            risk: 35,
            timeframe: 'Long-term',
            protocol: 'Curve',
            chain: 'Ethereum'
          },
          {
            id: '2',
            type: 'arbitrage',
            title: 'Cross-Chain USDC Arbitrage',
            description: 'Price difference between Ethereum and Polygon USDC pools',
            apy: 15.7,
            tvl: 45000000,
            risk: 65,
            timeframe: 'Short-term',
            protocol: 'Multiple',
            chain: 'Multi-chain'
          },
          {
            id: '3',
            type: 'new_protocol',
            title: 'New Lending Protocol Launch',
            description: 'Recently audited lending protocol with attractive bootstrap rewards',
            apy: 22.4,
            tvl: 12000000,
            risk: 78,
            timeframe: 'Medium-term',
            protocol: 'Radiant Capital',
            chain: 'Arbitrum'
          }
        ]);

        setProtocolHealth([
          {
            name: 'Uniswap V3',
            tvl: 3200000000,
            tvlChange: 5.2,
            volume24h: 1200000000,
            volumeChange: 12.4,
            riskScore: 25,
            governance: 'Decentralized',
            lastAudit: '2024-01-15'
          },
          {
            name: 'Aave',
            tvl: 8900000000,
            tvlChange: -2.1,
            volume24h: 450000000,
            volumeChange: -5.8,
            riskScore: 30,
            governance: 'Decentralized',
            lastAudit: '2024-02-01'
          },
          {
            name: 'Curve',
            tvl: 2100000000,
            tvlChange: 3.7,
            volume24h: 180000000,
            volumeChange: 8.9,
            riskScore: 35,
            governance: 'Decentralized',
            lastAudit: '2024-01-20'
          }
        ]);

        setMarketTrends([
          {
            category: 'DeFi TVL',
            trend: 'bullish',
            strength: 75,
            description: 'Total Value Locked in DeFi protocols increasing steadily',
            timeframe: '30 days'
          },
          {
            category: 'Yield Farming',
            trend: 'neutral',
            strength: 45,
            description: 'Yield farming returns stabilizing after recent volatility',
            timeframe: '7 days'
          },
          {
            category: 'Cross-Chain Activity',
            trend: 'bullish',
            strength: 82,
            description: 'Bridge volumes and multi-chain protocols seeing growth',
            timeframe: '14 days'
          }
        ]);

        setLoading(false);
      }, 1000);
    };

    fetchMarketData();
  }, [userAddress]);

  const formatCurrency = (value: number) => {
    if (value >= 1e9) return `$${(value / 1e9).toFixed(1)}B`;
    if (value >= 1e6) return `$${(value / 1e6).toFixed(1)}M`;
    if (value >= 1e3) return `$${(value / 1e3).toFixed(1)}K`;
    return `$${value.toFixed(2)}`;
  };

  const getRiskColor = (score: number) => {
    if (score >= 70) return 'text-red-400 bg-red-900/20 border-red-500/30';
    if (score >= 50) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
    if (score >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
    return 'text-green-400 bg-green-900/20 border-green-500/30';
  };

  const getTrendColor = (trend: string) => {
    switch (trend) {
      case 'bullish': return 'text-green-400';
      case 'bearish': return 'text-red-400';
      default: return 'text-gray-400';
    }
  };

  const getTrendIcon = (trend: string) => {
    switch (trend) {
      case 'bullish': return '📈';
      case 'bearish': return '📉';
      default: return '➡️';
    }
  };

  if (userTier === 'basic') {
    return (
      <div className="space-y-6">
        <div className="text-center py-12">
          <div className="text-4xl mb-4">🌍</div>
          <h3 className="text-xl font-semibold text-white mb-2">Market Intelligence</h3>
          <p className="text-gray-400 mb-6">
            Access comprehensive market analysis, protocol health monitoring, and yield opportunities across DeFi.
          </p>
          <button className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-3 rounded-lg font-medium">
            Upgrade to Professional →
          </button>
        </div>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="animate-pulse">
          <div className="bg-gray-800 rounded-lg h-64 mb-6"></div>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="bg-gray-800 rounded-lg h-48"></div>
            <div className="bg-gray-800 rounded-lg h-48"></div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header with View Selector */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Market Intelligence</h2>
        
        <div className="flex space-x-2">
          {[
            { id: 'opportunities', label: 'Opportunities', icon: '💎' },
            { id: 'protocols', label: 'Protocol Health', icon: '🏥' },
            { id: 'trends', label: 'Market Trends', icon: '📊' },
            { id: 'sentiment', label: 'Sentiment', icon: '🎭', premium: userTier !== 'institutional' && userTier !== 'enterprise' }
          ].map((view) => (
            <button
              key={view.id}
              onClick={() => setActiveView(view.id as any)}
              disabled={view.premium}
              className={`flex items-center space-x-1 px-3 py-1 rounded text-sm transition-colors ${
                activeView === view.id
                  ? 'bg-blue-600 text-white'
                  : view.premium
                  ? 'bg-gray-800 text-gray-500 cursor-not-allowed'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              <span>{view.icon}</span>
              <span>{view.label}</span>
              {view.premium && <span className="text-xs">INST</span>}
            </button>
          ))}
        </div>
      </div>

      {/* Market Opportunities */}
      {activeView === 'opportunities' && (
        <div className="space-y-4">
          <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
            <h3 className="text-lg font-semibold text-white mb-4">Top Yield Opportunities</h3>
            <div className="space-y-4">
              {opportunities.map((opp) => (
                <div key={opp.id} className="bg-gray-800/50 rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <div className="flex items-center space-x-2 mb-1">
                        <h4 className="text-md font-medium text-white">{opp.title}</h4>
                        <span className={`text-xs px-2 py-1 rounded ${
                          opp.type === 'yield' ? 'bg-green-600/20 text-green-400' :
                          opp.type === 'arbitrage' ? 'bg-blue-600/20 text-blue-400' :
                          opp.type === 'liquidation' ? 'bg-red-600/20 text-red-400' :
                          'bg-purple-600/20 text-purple-400'
                        }`}>
                          {opp.type.toUpperCase()}
                        </span>
                      </div>
                      <p className="text-sm text-gray-400 mb-2">{opp.description}</p>
                      <div className="flex items-center space-x-4 text-xs text-gray-400">
                        <span>Protocol: {opp.protocol}</span>
                        <span>Chain: {opp.chain}</span>
                        <span>Timeframe: {opp.timeframe}</span>
                      </div>
                    </div>
                    
                    <div className="text-right">
                      <div className="text-lg font-bold text-green-400 mb-1">
                        {opp.apy.toFixed(1)}% APY
                      </div>
                      <div className="text-xs text-gray-400">
                        TVL: {formatCurrency(opp.tvl)}
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex items-center justify-between">
                    <div className="flex items-center space-x-2">
                      <span className="text-xs text-gray-400">Risk:</span>
                      <span className={`text-xs px-2 py-1 rounded border ${getRiskColor(opp.risk)}`}>
                        {opp.risk}/100
                      </span>
                    </div>
                    
                    <div className="flex space-x-2">
                      <button className="bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded text-xs">
                        Analyze
                      </button>
                      <button className="bg-gray-700 hover:bg-gray-600 text-gray-300 px-3 py-1 rounded text-xs">
                        Track
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Protocol Health */}
      {activeView === 'protocols' && (
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg overflow-hidden">
          <div className="p-6 border-b border-gray-700">
            <h3 className="text-lg font-semibold text-white">Protocol Health Monitor</h3>
          </div>
          
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-gray-800/50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">Protocol</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">TVL</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">24h Volume</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">Risk Score</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">Last Audit</th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase">Status</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-700">
                {protocolHealth.map((protocol) => (
                  <tr key={protocol.name} className="hover:bg-gray-800/30">
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm font-medium text-white">{protocol.name}</div>
                      <div className="text-xs text-gray-400">{protocol.governance}</div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm text-white">{formatCurrency(protocol.tvl)}</div>
                      <div className={`text-xs ${protocol.tvlChange >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                        {protocol.tvlChange >= 0 ? '+' : ''}{protocol.tvlChange.toFixed(1)}%
                      </div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <div className="text-sm text-white">{formatCurrency(protocol.volume24h)}</div>
                      <div className={`text-xs ${protocol.volumeChange >= 0 ? 'text-green-400' : 'text-red-400'}`}>
                        {protocol.volumeChange >= 0 ? '+' : ''}{protocol.volumeChange.toFixed(1)}%
                      </div>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className={`inline-flex px-2 py-1 text-xs font-semibold rounded-full border ${getRiskColor(protocol.riskScore)}`}>
                        {protocol.riskScore}
                      </span>
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-300">
                      {protocol.lastAudit}
                    </td>
                    <td className="px-6 py-4 whitespace-nowrap">
                      <span className="inline-flex px-2 py-1 text-xs font-semibold rounded-full bg-green-900/20 text-green-400 border border-green-500/30">
                        Healthy
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Market Trends */}
      {activeView === 'trends' && (
        <div className="space-y-4">
          {marketTrends.map((trend, index) => (
            <div key={index} className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center space-x-3">
                  <span className="text-2xl">{getTrendIcon(trend.trend)}</span>
                  <div>
                    <h3 className="text-lg font-semibold text-white">{trend.category}</h3>
                    <div className="flex items-center space-x-2">
                      <span className={`text-sm font-medium ${getTrendColor(trend.trend)}`}>
                        {trend.trend.toUpperCase()}
                      </span>
                      <span className="text-sm text-gray-400">• {trend.timeframe}</span>
                    </div>
                  </div>
                </div>
                
                <div className="text-right">
                  <div className={`text-lg font-bold ${getTrendColor(trend.trend)}`}>
                    {trend.strength}%
                  </div>
                  <div className="text-xs text-gray-400">Strength</div>
                </div>
              </div>
              
              <p className="text-sm text-gray-400 mb-4">{trend.description}</p>
              
              <div className="w-full bg-gray-700 rounded-full h-2">
                <div
                  className={`h-2 rounded-full ${
                    trend.trend === 'bullish' ? 'bg-green-500' :
                    trend.trend === 'bearish' ? 'bg-red-500' : 'bg-gray-500'
                  }`}
                  style={{ width: `${trend.strength}%` }}
                ></div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Market Sentiment (Premium Feature) */}
      {activeView === 'sentiment' && (
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <div className="text-center py-8">
            <div className="text-4xl mb-4">🎭</div>
            <h3 className="text-lg font-semibold text-white mb-2">Market Sentiment Analysis</h3>
            <p className="text-gray-400 mb-4">
              Advanced sentiment tracking from social media, news, and on-chain activity.
            </p>
            <button className="bg-blue-600 hover:bg-blue-700 text-white px-6 py-2 rounded-lg">
              Upgrade to Institutional →
            </button>
          </div>
        </div>
      )}
    </div>
  );
};

export default MarketIntelligence;
