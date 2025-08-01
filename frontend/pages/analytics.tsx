/**
 * Analytics Page - Comprehensive DeFi Risk Analytics Dashboard
 * 
 * This page showcases all the advanced components created for the DeFi Risk Monitor:
 * - Risk Factor Breakdown with detailed analysis
 * - Historical Risk Charts with interactive visualizations
 * - Portfolio Performance Views with comprehensive metrics
 * - Explainable AI Interface with intelligent recommendations
 * - Alert Configuration UI with advanced templates
 */

import React, { useState, useEffect } from 'react';
import { useAccount } from 'wagmi';
import { useRiskMonitoring } from '../hooks/useRiskMonitoring';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import { toast } from 'react-hot-toast';

// Import all the advanced components
import RiskFactorBreakdown from '../components/RiskFactorBreakdown';
import HistoricalRiskCharts from '../components/HistoricalRiskCharts';
import PortfolioPerformanceViews from '../components/PortfolioPerformanceViews';
import ExplainableAIInterface from '../components/ExplainableAIInterface';
import AlertConfigurationUI from '../components/AlertConfigurationUI';

// Import basic components for header
import { LoadingSpinner, BarChartIcon, BrainIcon, AlertTriangleIcon, TrendingUpIcon, PieChartIcon } from '../components/Icons';

const AnalyticsPage: React.FC = () => {
  const { address, isConnected } = useAccount();
  const [activeSection, setActiveSection] = useState<'overview' | 'risk-analysis' | 'performance' | 'ai-insights' | 'alerts'>('overview');
  const [selectedPosition, setSelectedPosition] = useState<string | null>(null);

  // Get risk data for the connected user
  const {
    positions,
    riskMetrics,
    riskExplanation,
    isLoadingPositions,
    isLoadingRisk,
    error,
    isConnected: riskServiceConnected,
    refreshPositions
  } = useRiskMonitoring(address);

  // Auto-refresh data periodically
  useEffect(() => {
    if (address && isConnected) {
      const interval = setInterval(() => {
        refreshPositions();
      }, 30000); // Refresh every 30 seconds

      return () => clearInterval(interval);
    }
  }, [address, isConnected, refreshPositions]);

  // Show connection prompt if not connected
  if (!isConnected) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <div className="mb-8">
              <BrainIcon className="w-24 h-24 mx-auto text-blue-400 mb-4" />
              <h1 className="text-4xl font-bold text-white mb-2">DeFi Risk Analytics</h1>
              <p className="text-xl text-gray-400">Advanced risk analysis and portfolio insights</p>
            </div>
            
            <div className="bg-gray-800/50 rounded-xl p-8 border border-gray-700 max-w-md mx-auto">
              <h2 className="text-xl font-semibold text-white mb-4">Connect Your Wallet</h2>
              <p className="text-gray-400 mb-6">
                Connect your wallet to access advanced DeFi risk analytics, AI-powered insights, and comprehensive portfolio performance tracking.
              </p>
              <ConnectButton />
            </div>
          </div>
        </div>
      </div>
    );
  }

  // Navigation sections
  const sections = [
    {
      id: 'overview',
      name: 'Overview',
      icon: BarChartIcon,
      description: 'Risk breakdown and factor analysis'
    },
    {
      id: 'risk-analysis',
      name: 'Risk Analysis',
      icon: AlertTriangleIcon,
      description: 'Historical trends and patterns'
    },
    {
      id: 'performance',
      name: 'Performance',
      icon: PieChartIcon,
      description: 'Portfolio analytics and metrics'
    },
    {
      id: 'ai-insights',
      name: 'AI Insights',
      icon: BrainIcon,
      description: 'Intelligent recommendations'
    },
    {
      id: 'alerts',
      name: 'Alerts',
      icon: TrendingUpIcon,
      description: 'Alert configuration and management'
    }
  ];

  const renderContent = () => {
    switch (activeSection) {
      case 'overview':
        return (
          <div className="space-y-8">
            <div className="text-center mb-8">
              <h2 className="text-3xl font-bold text-white mb-2">Risk Factor Analysis</h2>
              <p className="text-gray-400">Detailed breakdown of your DeFi position risks</p>
            </div>
            <RiskFactorBreakdown 
              riskMetrics={riskMetrics || undefined}
              positionId={selectedPosition || positions[0]?.id}
              className="mb-8"
            />
          </div>
        );
        
      case 'risk-analysis':
        return (
          <div className="space-y-8">
            <div className="text-center mb-8">
              <h2 className="text-3xl font-bold text-white mb-2">Historical Risk Analysis</h2>
              <p className="text-gray-400">Track risk trends and identify patterns over time</p>
            </div>
            <HistoricalRiskCharts 
              positionId={selectedPosition || positions[0]?.id}
              userAddress={address}
              className="mb-8"
            />
          </div>
        );
        
      case 'performance':
        return (
          <div className="space-y-8">
            <div className="text-center mb-8">
              <h2 className="text-3xl font-bold text-white mb-2">Portfolio Performance</h2>
              <p className="text-gray-400">Comprehensive analytics and performance tracking</p>
            </div>
            <PortfolioPerformanceViews 
              userAddress={address}
              className="mb-8"
            />
          </div>
        );
        
      case 'ai-insights':
        return (
          <div className="space-y-8">
            <div className="text-center mb-8">
              <h2 className="text-3xl font-bold text-white mb-2">AI Risk Analyst</h2>
              <p className="text-gray-400">Intelligent risk analysis and personalized recommendations</p>
            </div>
            <ExplainableAIInterface 
              riskMetrics={riskMetrics || undefined}
              positionId={selectedPosition || positions[0]?.id}
              userAddress={address}
              className="mb-8"
            />
          </div>
        );
        
      case 'alerts':
        return (
          <div className="space-y-8">
            <div className="text-center mb-8">
              <h2 className="text-3xl font-bold text-white mb-2">Alert Configuration</h2>
              <p className="text-gray-400">Manage risk alerts and notification preferences</p>
            </div>
            <AlertConfigurationUI 
              userAddress={address}
              alerts={[]} // This would come from the useRiskMonitoring hook
              className="mb-8"
            />
          </div>
        );
        
      default:
        return null;
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
      {/* Header */}
      <div className="bg-gray-800/50 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-3">
              <BrainIcon className="w-8 h-8 text-blue-400" />
              <div>
                <h1 className="text-xl font-bold text-white">DeFi Risk Analytics</h1>
                <p className="text-sm text-gray-400">Advanced portfolio insights</p>
              </div>
            </div>
            
            <div className="flex items-center gap-4">
              {/* Connection Status */}
              <div className="flex items-center gap-2 text-sm">
                <div className={`w-2 h-2 rounded-full ${riskServiceConnected ? 'bg-green-500' : 'bg-red-500'}`} />
                <span className="text-gray-400">
                  {riskServiceConnected ? 'Connected' : 'Disconnected'}
                </span>
              </div>
              
              {/* Wallet Connection */}
              <ConnectButton />
            </div>
          </div>
        </div>
      </div>

      {/* Navigation */}
      <div className="bg-gray-800/30 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex space-x-8 overflow-x-auto">
            {sections.map((section) => {
              const Icon = section.icon;
              return (
                <button
                  key={section.id}
                  onClick={() => setActiveSection(section.id as any)}
                  className={`flex items-center gap-2 px-4 py-4 text-sm font-medium border-b-2 transition-colors whitespace-nowrap ${
                    activeSection === section.id
                      ? 'border-blue-500 text-blue-400'
                      : 'border-transparent text-gray-400 hover:text-gray-300 hover:border-gray-600'
                  }`}
                >
                  <Icon className="w-4 h-4" />
                  <div className="text-left">
                    <div>{section.name}</div>
                    <div className="text-xs text-gray-500">{section.description}</div>
                  </div>
                </button>
              );
            })}
          </div>
        </div>
      </div>

      {/* Position Selector */}
      {positions.length > 0 && (
        <div className="bg-gray-800/20 border-b border-gray-700">
          <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-3">
            <div className="flex items-center gap-4">
              <span className="text-sm text-gray-400">Analyze Position:</span>
              <select
                value={selectedPosition || positions[0]?.id || ''}
                onChange={(e) => setSelectedPosition(e.target.value)}
                className="bg-gray-700 border border-gray-600 rounded-lg px-3 py-1 text-sm text-white"
              >
                {positions.map((position) => (
                  <option key={position.id} value={position.id}>
                    {position.token0_symbol}/{position.token1_symbol} - ${parseFloat(position.current_value_usd).toLocaleString()}
                  </option>
                ))}
              </select>
              
              {isLoadingPositions && (
                <LoadingSpinner className="w-4 h-4 text-blue-400" />
              )}
            </div>
          </div>
        </div>
      )}

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {isLoadingPositions ? (
          <div className="flex items-center justify-center py-12">
            <LoadingSpinner className="w-8 h-8 text-blue-400 mr-3" />
            <span className="text-gray-400">Loading portfolio data...</span>
          </div>
        ) : error ? (
          <div className="text-center py-12">
            <AlertTriangleIcon className="w-12 h-12 mx-auto text-red-400 mb-4" />
            <h3 className="text-lg font-medium text-white mb-2">Error Loading Data</h3>
            <p className="text-gray-400 mb-4">{error instanceof Error ? error.message : String(error)}</p>
            <button
              onClick={refreshPositions}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
            >
              Retry
            </button>
          </div>
        ) : positions.length === 0 ? (
          <div className="text-center py-12">
            <PieChartIcon className="w-12 h-12 mx-auto text-gray-400 mb-4" />
            <h3 className="text-lg font-medium text-white mb-2">No Positions Found</h3>
            <p className="text-gray-400">
              Connect a wallet with DeFi positions to see advanced analytics
            </p>
          </div>
        ) : (
          renderContent()
        )}
      </div>

      {/* Footer */}
      <div className="bg-gray-800/30 border-t border-gray-700 mt-16">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
          <div className="text-center text-sm text-gray-400">
            <p>DeFi Risk Monitor - Advanced Portfolio Analytics & Risk Management</p>
            <p className="mt-1">Real-time risk monitoring • AI-powered insights • Comprehensive analytics</p>
          </div>
        </div>
      </div>
    </div>
  );
};

export default AnalyticsPage;
