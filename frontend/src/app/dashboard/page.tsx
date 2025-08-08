'use client';

import React, { useState, useEffect } from 'react';
import { useRouter } from 'next/navigation';
import PortfolioOverview from '../../components/dashboard/PortfolioOverview';
import RealTimeRiskMonitor from '../../components/dashboard/RealTimeRiskMonitor';
import AdvancedAnalytics from '../../components/dashboard/AdvancedAnalytics';
import AIInsightsHub from '../../components/dashboard/AIInsightsHub';
import MarketIntelligence from '../../components/dashboard/MarketIntelligence';
import AlertsCenter from '../../components/dashboard/AlertsCenter';

interface DashboardProps {}

const Dashboard: React.FC<DashboardProps> = () => {
  const router = useRouter();
  const [activeTab, setActiveTab] = useState<'overview' | 'risk' | 'analytics' | 'ai' | 'market' | 'alerts'>('overview');
  const [userAddress, setUserAddress] = useState<string>('');
  const [isConnected, setIsConnected] = useState(false);
  const [userTier, setUserTier] = useState<'basic' | 'professional' | 'institutional' | 'enterprise'>('professional');

  useEffect(() => {
    // Check if user is connected (from localStorage or wallet)
    const savedAddress = localStorage.getItem('connectedWallet');
    if (savedAddress) {
      setUserAddress(savedAddress);
      setIsConnected(true);
    } else {
      // Redirect to main page if not connected
      router.push('/');
    }
  }, [router]);

  const tabs = [
    { id: 'overview', label: 'Portfolio Overview', icon: '📊' },
    { id: 'risk', label: 'Risk Monitor', icon: '🚨' },
    { id: 'analytics', label: 'Advanced Analytics', icon: '📈' },
    { id: 'ai', label: 'AI Insights', icon: '🤖', premium: true },
    { id: 'market', label: 'Market Intelligence', icon: '🌍', premium: true },
    { id: 'alerts', label: 'Alerts Center', icon: '🔔' }
  ];

  if (!isConnected) {
    return (
      <div className="min-h-screen bg-black flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto mb-4"></div>
          <p className="text-gray-400">Connecting to your wallet...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black text-white">
      {/* Header */}
      <header className="border-b border-gray-800 bg-gray-900/50 backdrop-blur-sm sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center space-x-4">
              <h1 className="text-xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent">
                DeFi Risk Dashboard
              </h1>
              <div className="px-2 py-1 bg-blue-600/20 border border-blue-500/30 rounded text-xs text-blue-400">
                {userTier.toUpperCase()}
              </div>
            </div>
            
            <div className="flex items-center space-x-4">
              <div className="text-sm text-gray-400">
                {userAddress.slice(0, 6)}...{userAddress.slice(-4)}
              </div>
              <div className="w-2 h-2 bg-green-500 rounded-full animate-pulse" title="Connected"></div>
            </div>
          </div>
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="border-b border-gray-800 bg-gray-900/30">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex space-x-8 overflow-x-auto">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id as any)}
                className={`flex items-center space-x-2 py-4 px-2 border-b-2 font-medium text-sm whitespace-nowrap transition-colors ${
                  activeTab === tab.id
                    ? 'border-blue-500 text-blue-400'
                    : 'border-transparent text-gray-400 hover:text-gray-300 hover:border-gray-600'
                }`}
              >
                <span>{tab.icon}</span>
                <span>{tab.label}</span>
                {tab.premium && userTier === 'basic' && (
                  <span className="text-xs bg-yellow-600/20 text-yellow-400 px-1 rounded">PRO</span>
                )}
              </button>
            ))}
          </div>
        </div>
      </nav>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {activeTab === 'overview' && (
          <PortfolioOverview userAddress={userAddress} userTier={userTier} />
        )}
        
        {activeTab === 'risk' && (
          <RealTimeRiskMonitor userAddress={userAddress} userTier={userTier} />
        )}
        
        {activeTab === 'analytics' && (
          <AdvancedAnalytics userAddress={userAddress} userTier={userTier} />
        )}
        
        {activeTab === 'ai' && (
          <AIInsightsHub userAddress={userAddress} userTier={userTier} />
        )}
        
        {activeTab === 'market' && (
          <MarketIntelligence userAddress={userAddress} userTier={userTier} />
        )}
        
        {activeTab === 'alerts' && (
          <AlertsCenter userAddress={userAddress} userTier={userTier} />
        )}
      </main>
    </div>
  );
};

export default Dashboard;
