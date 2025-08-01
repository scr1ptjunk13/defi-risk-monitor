/**
 * HistoricalRiskCharts Component
 * 
 * Interactive charts showing historical risk trends, correlations,
 * and comparative analysis across different time periods
 */

import React, { useState, useEffect, useMemo } from 'react';
import { Line, Area, Bar } from '@ant-design/charts';
import { RiskMetrics } from '../lib/api-client';
import { CalendarIcon, TrendingUpIcon, BarChartIcon, InfoIcon } from './Icons';

interface HistoricalRiskChartsProps {
  positionId?: string;
  userAddress?: string;
  className?: string;
}

interface RiskDataPoint {
  timestamp: string;
  overall_risk: number;
  liquidity_risk: number;
  volatility_risk: number;
  protocol_risk: number;
  mev_risk: number;
  cross_chain_risk: number;
  impermanent_loss_risk: number;
}

interface ChartConfig {
  id: string;
  name: string;
  type: 'line' | 'area' | 'bar';
  description: string;
  metrics: string[];
}

const HistoricalRiskCharts: React.FC<HistoricalRiskChartsProps> = ({
  positionId,
  userAddress,
  className = ''
}) => {
  const [timeRange, setTimeRange] = useState<'1h' | '6h' | '24h' | '7d' | '30d' | '90d'>('24h');
  const [selectedChart, setSelectedChart] = useState<string>('overview');
  const [selectedMetrics, setSelectedMetrics] = useState<string[]>(['overall_risk', 'liquidity_risk', 'mev_risk']);
  const [isLoading, setIsLoading] = useState(false);
  const [riskData, setRiskData] = useState<RiskDataPoint[]>([]);

  // Chart configurations
  const chartConfigs: ChartConfig[] = [
    {
      id: 'overview',
      name: 'Risk Overview',
      type: 'area',
      description: 'Overall risk trends and key metrics',
      metrics: ['overall_risk', 'liquidity_risk', 'mev_risk']
    },
    {
      id: 'detailed',
      name: 'Detailed Analysis',
      type: 'line',
      description: 'All risk factors with detailed breakdown',
      metrics: ['overall_risk', 'liquidity_risk', 'volatility_risk', 'protocol_risk', 'mev_risk', 'cross_chain_risk', 'impermanent_loss_risk']
    },
    {
      id: 'comparison',
      name: 'Risk Comparison',
      type: 'bar',
      description: 'Compare risk levels across different periods',
      metrics: ['overall_risk', 'liquidity_risk', 'volatility_risk', 'protocol_risk']
    }
  ];

  const riskMetricLabels = {
    overall_risk: 'Overall Risk',
    liquidity_risk: 'Liquidity Risk',
    volatility_risk: 'Volatility Risk',
    protocol_risk: 'Protocol Risk',
    mev_risk: 'MEV Risk',
    cross_chain_risk: 'Cross-Chain Risk',
    impermanent_loss_risk: 'Impermanent Loss Risk'
  };

  const riskMetricColors = {
    overall_risk: '#ef4444',
    liquidity_risk: '#f97316',
    volatility_risk: '#eab308',
    protocol_risk: '#22c55e',
    mev_risk: '#3b82f6',
    cross_chain_risk: '#8b5cf6',
    impermanent_loss_risk: '#ec4899'
  };

  // Generate mock historical data
  useEffect(() => {
    setIsLoading(true);
    
    // Simulate API call delay
    setTimeout(() => {
      const now = new Date();
      const dataPoints: RiskDataPoint[] = [];
      
      let intervals: number;
      let intervalMs: number;
      
      switch (timeRange) {
        case '1h':
          intervals = 12; // 5-minute intervals
          intervalMs = 5 * 60 * 1000;
          break;
        case '6h':
          intervals = 24; // 15-minute intervals
          intervalMs = 15 * 60 * 1000;
          break;
        case '24h':
          intervals = 24; // 1-hour intervals
          intervalMs = 60 * 60 * 1000;
          break;
        case '7d':
          intervals = 28; // 6-hour intervals
          intervalMs = 6 * 60 * 60 * 1000;
          break;
        case '30d':
          intervals = 30; // 1-day intervals
          intervalMs = 24 * 60 * 60 * 1000;
          break;
        case '90d':
          intervals = 30; // 3-day intervals
          intervalMs = 3 * 24 * 60 * 60 * 1000;
          break;
        default:
          intervals = 24;
          intervalMs = 60 * 60 * 1000;
      }

      for (let i = intervals; i >= 0; i--) {
        const timestamp = new Date(now.getTime() - (i * intervalMs));
        
        // Generate realistic risk data with some correlation
        const baseRisk = 50 + Math.sin(i * 0.3) * 20 + Math.random() * 10;
        const volatility = Math.random() * 0.3 + 0.8; // 0.8 to 1.1 multiplier
        
        dataPoints.push({
          timestamp: timestamp.toISOString(),
          overall_risk: Math.max(0, Math.min(100, baseRisk * volatility)),
          liquidity_risk: Math.max(0, Math.min(100, (baseRisk - 10) * (volatility * 0.9))),
          volatility_risk: Math.max(0, Math.min(100, (baseRisk + 5) * (volatility * 1.1))),
          protocol_risk: Math.max(0, Math.min(100, 30 + Math.random() * 20)),
          mev_risk: Math.max(0, Math.min(100, (baseRisk + 10) * (volatility * 1.2))),
          cross_chain_risk: Math.max(0, Math.min(100, 40 + Math.random() * 30)),
          impermanent_loss_risk: Math.max(0, Math.min(100, (baseRisk - 20) * (volatility * 0.7)))
        });
      }
      
      setRiskData(dataPoints);
      setIsLoading(false);
    }, 500);
  }, [timeRange, positionId]);

  // Prepare chart data based on selected metrics
  const chartData = useMemo(() => {
    if (!riskData.length) return [];
    
    return riskData.flatMap(point => 
      selectedMetrics.map(metric => ({
        timestamp: point.timestamp,
        date: new Date(point.timestamp).toLocaleDateString(),
        time: new Date(point.timestamp).toLocaleTimeString(),
        metric: riskMetricLabels[metric as keyof typeof riskMetricLabels],
        value: point[metric as keyof RiskDataPoint] as number,
        color: riskMetricColors[metric as keyof typeof riskMetricColors]
      }))
    );
  }, [riskData, selectedMetrics]);

  // Chart configurations for different chart types
  const getChartConfig = () => {
    const selectedConfig = chartConfigs.find(c => c.id === selectedChart);
    const baseConfig = {
      data: chartData,
      xField: 'timestamp',
      yField: 'value',
      seriesField: 'metric',
      smooth: true,
      animation: {
        appear: {
          animation: 'path-in',
          duration: 1000,
        },
      },
      color: selectedMetrics.map(metric => riskMetricColors[metric as keyof typeof riskMetricColors]),
      xAxis: {
        type: 'time',
        tickCount: 6,
        label: {
          formatter: (text: string) => {
            const date = new Date(text);
            if (timeRange === '1h' || timeRange === '6h') {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
            } else if (timeRange === '24h') {
              return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
            } else {
              return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
            }
          }
        }
      },
      yAxis: {
        label: {
          formatter: (text: string) => `${text}%`
        },
        max: 100,
        min: 0
      },
      tooltip: {
        formatter: (datum: any) => ({
          name: datum.metric,
          value: `${datum.value.toFixed(1)}%`
        })
      },
      legend: {
        position: 'top' as const
      }
    };

    switch (selectedConfig?.type) {
      case 'area':
        return {
          ...baseConfig,
          areaStyle: { fillOpacity: 0.3 }
        };
      case 'bar':
        return {
          ...baseConfig,
          isGroup: true,
          columnWidthRatio: 0.8
        };
      default:
        return baseConfig;
    }
  };

  // Risk statistics
  const riskStats = useMemo(() => {
    if (!riskData.length) return {};
    
    const stats: Record<string, any> = {};
    
    selectedMetrics.forEach(metric => {
      const values = riskData.map(d => d[metric as keyof RiskDataPoint] as number);
      const current = values[values.length - 1];
      const previous = values[values.length - 2] || current;
      const min = Math.min(...values);
      const max = Math.max(...values);
      const avg = values.reduce((sum, val) => sum + val, 0) / values.length;
      const change = current - previous;
      const changePercent = previous !== 0 ? (change / previous) * 100 : 0;
      
      stats[metric] = {
        current,
        change,
        changePercent,
        min,
        max,
        avg,
        trend: change > 0 ? 'up' : change < 0 ? 'down' : 'stable'
      };
    });
    
    return stats;
  }, [riskData, selectedMetrics]);

  const renderChart = () => {
    const selectedConfig = chartConfigs.find(c => c.id === selectedChart);
    
    try {
      const config = getChartConfig();
      
      // Ensure data is valid before rendering
      if (!config.data || config.data.length === 0) {
        return (
          <div className="flex items-center justify-center h-64 text-gray-400">
            <div className="text-center">
              <BarChartIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>No chart data available</p>
            </div>
          </div>
        );
      }
      
      switch (selectedConfig?.type) {
        case 'area':
          return <Area {...config} />;
        case 'bar':
          return <Bar {...config} />;
        default:
          return <Line {...config} />;
      }
    } catch (error) {
      console.error('Chart rendering error:', error);
      return (
        <div className="flex items-center justify-center h-64 text-red-400">
          <div className="text-center">
            <BarChartIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>Error loading chart</p>
            <p className="text-sm text-gray-500 mt-1">Please try refreshing the page</p>
          </div>
        </div>
      );
    }
  };

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex flex-col md:flex-row md:items-center justify-between mb-6 gap-4">
        <div>
          <h3 className="text-lg font-semibold text-white">Historical Risk Analysis</h3>
          <p className="text-sm text-gray-400 mt-1">
            Track risk trends and identify patterns over time
          </p>
        </div>
        
        <div className="flex items-center gap-3">
          {/* Time Range Selector */}
          <div className="flex items-center gap-2">
            <CalendarIcon className="w-4 h-4 text-gray-400" />
            <select
              value={timeRange}
              onChange={(e) => setTimeRange(e.target.value as any)}
              className="bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white"
            >
              <option value="1h">1 Hour</option>
              <option value="6h">6 Hours</option>
              <option value="24h">24 Hours</option>
              <option value="7d">7 Days</option>
              <option value="30d">30 Days</option>
              <option value="90d">90 Days</option>
            </select>
          </div>
          
          {/* Chart Type Selector */}
          <select
            value={selectedChart}
            onChange={(e) => setSelectedChart(e.target.value)}
            className="bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-sm text-white"
          >
            {chartConfigs.map(config => (
              <option key={config.id} value={config.id}>{config.name}</option>
            ))}
          </select>
        </div>
      </div>

      {/* Risk Statistics */}
      <div className="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-4 gap-4 mb-6">
        {selectedMetrics.slice(0, 4).map(metric => {
          const stats = riskStats[metric];
          if (!stats) return null;
          
          return (
            <div key={metric} className="bg-gray-900/30 rounded-lg p-3 border border-gray-600">
              <div className="flex items-center justify-between mb-2">
                <span className="text-sm text-gray-400">
                  {riskMetricLabels[metric as keyof typeof riskMetricLabels]}
                </span>
                <div className={`flex items-center gap-1 text-xs ${
                  stats.trend === 'up' ? 'text-red-400' :
                  stats.trend === 'down' ? 'text-green-400' : 'text-gray-400'
                }`}>
                  {stats.trend === 'up' && <TrendingUpIcon className="w-3 h-3" />}
                  {stats.changePercent !== 0 && `${stats.changePercent > 0 ? '+' : ''}${stats.changePercent.toFixed(1)}%`}
                </div>
              </div>
              
              <div className="text-xl font-semibold text-white mb-1">
                {stats.current.toFixed(1)}%
              </div>
              
              <div className="flex justify-between text-xs text-gray-500">
                <span>Min: {stats.min.toFixed(1)}%</span>
                <span>Max: {stats.max.toFixed(1)}%</span>
              </div>
            </div>
          );
        })}
      </div>

      {/* Metric Selector */}
      <div className="mb-6">
        <h4 className="text-sm font-medium text-gray-300 mb-3">Risk Metrics</h4>
        <div className="flex flex-wrap gap-2">
          {Object.entries(riskMetricLabels).map(([key, label]) => (
            <button
              key={key}
              onClick={() => {
                if (selectedMetrics.includes(key)) {
                  setSelectedMetrics(prev => prev.filter(m => m !== key));
                } else {
                  setSelectedMetrics(prev => [...prev, key]);
                }
              }}
              className={`px-3 py-1 text-sm rounded-lg border transition-all ${
                selectedMetrics.includes(key)
                  ? 'border-blue-500 bg-blue-900/20 text-blue-400'
                  : 'border-gray-600 bg-gray-900/30 text-gray-400 hover:bg-gray-900/50'
              }`}
              style={{
                borderColor: selectedMetrics.includes(key) 
                  ? riskMetricColors[key as keyof typeof riskMetricColors]
                  : undefined
              }}
            >
              <div className="flex items-center gap-2">
                <div 
                  className="w-2 h-2 rounded-full"
                  style={{ backgroundColor: riskMetricColors[key as keyof typeof riskMetricColors] }}
                />
                {label}
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Chart */}
      <div className="bg-gray-900/30 rounded-lg p-4 border border-gray-600">
        {isLoading ? (
          <div className="flex items-center justify-center h-64">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            <span className="ml-2 text-gray-400">Loading chart data...</span>
          </div>
        ) : chartData.length === 0 ? (
          <div className="flex items-center justify-center h-64 text-gray-400">
            <div className="text-center">
              <BarChartIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>No data available for selected time range</p>
            </div>
          </div>
        ) : (
          <div className="h-64">
            {renderChart()}
          </div>
        )}
      </div>

      {/* Chart Description */}
      <div className="mt-4 p-3 bg-blue-900/10 border border-blue-500/30 rounded-lg">
        <div className="flex items-start gap-2">
          <InfoIcon className="w-4 h-4 text-blue-400 mt-0.5 flex-shrink-0" />
          <div>
            <p className="text-sm text-blue-400 font-medium mb-1">
              {chartConfigs.find(c => c.id === selectedChart)?.name}
            </p>
            <p className="text-xs text-blue-300/80">
              {chartConfigs.find(c => c.id === selectedChart)?.description}
            </p>
          </div>
        </div>
      </div>

      {/* Key Insights */}
      {riskData.length > 0 && (
        <div className="mt-6 pt-6 border-t border-gray-700">
          <h4 className="text-sm font-medium text-gray-300 mb-3">Key Insights</h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="bg-gray-900/30 rounded-lg p-3 border border-gray-600">
              <h5 className="text-sm font-medium text-white mb-2">Risk Trend</h5>
              <p className="text-xs text-gray-400">
                {(() => {
                  const overallStats = riskStats['overall_risk'];
                  if (!overallStats) return 'No data available';
                  
                  if (overallStats.trend === 'up') {
                    return `Risk has increased by ${Math.abs(overallStats.changePercent).toFixed(1)}% in the selected period. Monitor closely for potential issues.`;
                  } else if (overallStats.trend === 'down') {
                    return `Risk has decreased by ${Math.abs(overallStats.changePercent).toFixed(1)}% in the selected period. Conditions are improving.`;
                  } else {
                    return 'Risk levels have remained relatively stable in the selected period.';
                  }
                })()}
              </p>
            </div>
            
            <div className="bg-gray-900/30 rounded-lg p-3 border border-gray-600">
              <h5 className="text-sm font-medium text-white mb-2">Volatility Analysis</h5>
              <p className="text-xs text-gray-400">
                {(() => {
                  const overallStats = riskStats['overall_risk'];
                  if (!overallStats) return 'No data available';
                  
                  const volatility = overallStats.max - overallStats.min;
                  if (volatility > 30) {
                    return `High volatility detected (${volatility.toFixed(1)}% range). Consider adjusting position size or implementing additional risk controls.`;
                  } else if (volatility > 15) {
                    return `Moderate volatility observed (${volatility.toFixed(1)}% range). Normal market conditions with some fluctuation.`;
                  } else {
                    return `Low volatility period (${volatility.toFixed(1)}% range). Stable risk conditions with minimal fluctuation.`;
                  }
                })()}
              </p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default HistoricalRiskCharts;
