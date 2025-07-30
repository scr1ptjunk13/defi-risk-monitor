/**
 * RiskDashboard Component
 * 
 * Comprehensive dashboard for displaying DeFi position risks,
 * protocol events, and real-time monitoring status
 */

import React, { useState, useEffect } from 'react';
import { useRiskMonitoring } from '../hooks/useRiskMonitoring';
import { useProtocolEvents } from '../hooks/useProtocolEvents';
import { LoadingSpinner, InfoIcon } from './Icons';
import toast from 'react-hot-toast';

interface RiskDashboardProps {
  className?: string;
}

const RiskDashboard: React.FC<RiskDashboardProps> = ({ className = '' }) => {
  const [activeTab, setActiveTab] = useState<'positions' | 'events' | 'alerts'>('positions');

  // Risk monitoring integration
  const {
    positions,
    riskMetrics,
    isLoadingPositions: isLoadingRisk,
    error: riskError,
    isConnected: isRiskConnected,
    lastUpdate: lastRiskUpdate,
    clearError: clearRiskError,
  } = useRiskMonitoring();

  // Protocol events integration
  const {
    events,
    eventStats,
    isLoadingEvents,
    isConnected: isEventsConnected,
    lastEventUpdate,
    error: eventsError,
    clearError: clearEventsError,
  } = useProtocolEvents({
    per_page: 10,
  });

  // Connection status indicator
  const ConnectionStatus = ({ isConnected, label }: { isConnected: boolean; label: string }) => (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
      <span className="text-xs text-gray-400">{label}: {isConnected ? 'Connected' : 'Disconnected'}</span>
    </div>
  );

  // Risk score color helper
  const getRiskColor = (score: string | number) => {
    const numScore = typeof score === 'string' ? parseFloat(score) : score;
    if (numScore > 70) return 'text-red-400';
    if (numScore > 40) return 'text-yellow-400';
    return 'text-green-400';
  };

  // Risk score background helper
  const getRiskBgColor = (score: string | number) => {
    const numScore = typeof score === 'string' ? parseFloat(score) : score;
    if (numScore > 70) return 'bg-red-900/20 border-red-500/30';
    if (numScore > 40) return 'bg-yellow-900/20 border-yellow-500/30';
    return 'bg-green-900/20 border-green-500/30';
  };

  // Helper to format risk score
  const formatRiskScore = (score: string) => {
    return parseFloat(score).toFixed(1);
  };

  return (
    <div className={`bg-gray-900 rounded-2xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-white">DeFi Risk Dashboard</h2>
        <div className="flex flex-col gap-1">
          <ConnectionStatus isConnected={isRiskConnected} label="Risk Monitor" />
          <ConnectionStatus isConnected={isEventsConnected} label="Protocol Events" />
        </div>
      </div>

      {/* Error Messages */}
      {(riskError || eventsError) && (
        <div className="mb-4 p-3 bg-red-900/20 border border-red-500/30 rounded-lg">
          {riskError && (
            <div className="flex items-center justify-between text-red-400 text-sm mb-2">
              <span>⚠️ Risk Monitor: {riskError}</span>
              <button onClick={clearRiskError} className="text-blue-400 hover:text-blue-300">
                Clear
              </button>
            </div>
          )}
          {eventsError && (
            <div className="flex items-center justify-between text-red-400 text-sm">
              <span>⚠️ Protocol Events: {eventsError}</span>
              <button onClick={clearEventsError} className="text-blue-400 hover:text-blue-300">
                Clear
              </button>
            </div>
          )}
        </div>
      )}

      {/* Tab Navigation */}
      <div className="flex gap-4 mb-6 border-b border-gray-700">
        {[
          { key: 'positions', label: 'Positions', count: positions.length },
          { key: 'events', label: 'Protocol Events', count: events.length },
          { key: 'alerts', label: 'Alerts', count: 0 }, // Placeholder for future alerts implementation
        ].map(({ key, label, count }) => (
          <button
            key={key}
            onClick={() => setActiveTab(key as any)}
            className={`pb-2 px-1 text-sm font-medium border-b-2 transition-colors ${
              activeTab === key
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-gray-300'
            }`}
          >
            {label} {count > 0 && <span className="ml-1 text-xs">({count})</span>}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="min-h-[400px]">
        {/* Positions Tab */}
        {activeTab === 'positions' && (
          <div>
            {isLoadingRisk ? (
              <div className="flex items-center justify-center py-12">
                <LoadingSpinner className="w-8 h-8" />
                <span className="ml-2 text-gray-400">Loading positions...</span>
              </div>
            ) : positions.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No positions found</p>
                <p className="text-sm mt-2">Add liquidity to start monitoring risks</p>
              </div>
            ) : (
              <div className="space-y-4">
                {positions.map((position) => (
                  <div key={position.id} className="bg-gray-800/50 rounded-lg p-4 border border-gray-600">
                    <div className="flex items-center justify-between mb-3">
                      <div>
                        <h3 className="font-medium text-white">
                          {position.token0_symbol}/{position.token1_symbol}
                        </h3>
                        <p className="text-sm text-gray-400">Uniswap V3</p>
                      </div>
                      {riskMetrics && (
                        <div className={`px-3 py-1 rounded-full text-sm font-medium ${getRiskBgColor(riskMetrics.overall_risk_score)}`}>
                          <span className={getRiskColor(riskMetrics.overall_risk_score)}>
                            {formatRiskScore(riskMetrics.overall_risk_score)}% Risk
                          </span>
                        </div>
                      )}
                    </div>
                    
                    <div className="grid grid-cols-2 gap-4 text-sm">
                      <div>
                        <span className="text-gray-400">Liquidity:</span>
                        <span className="ml-2 text-white">${position.liquidity_amount.toLocaleString()}</span>
                      </div>
                      <div>
                        <span className="text-gray-400">Fee Tier:</span>
                        <span className="ml-2 text-white">{position.fee_tier}%</span>
                      </div>
                      <div>
                        <span className="text-gray-400">Chain:</span>
                        <span className="ml-2 text-white">
                          Ethereum
                        </span>
                      </div>
                      <div>
                        <span className="text-gray-400">Created:</span>
                        <span className="ml-2 text-white">
                          {new Date(position.created_at).toLocaleDateString()}
                        </span>
                      </div>
                    </div>

                    {riskMetrics && (
                      <div className="mt-3 pt-3 border-t border-gray-700">
                        <div className="grid grid-cols-3 gap-4 text-xs">
                          <div className="text-center">
                            <div className="text-gray-400 mb-1">IL Risk</div>
                            <div className={`font-medium ${getRiskColor(riskMetrics.impermanent_loss_risk)}`}>
                              {formatRiskScore(riskMetrics.impermanent_loss_risk)}%
                            </div>
                          </div>
                          <div className="text-center">
                            <div className="text-gray-400 mb-1">Liquidity Risk</div>
                            <div className={`font-medium ${getRiskColor(riskMetrics.liquidity_risk)}`}>
                              {formatRiskScore(riskMetrics.liquidity_risk)}%
                            </div>
                          </div>
                          <div className="text-center">
                            <div className="text-gray-400 mb-1">Volatility Risk</div>
                            <div className={`font-medium ${getRiskColor(riskMetrics.volatility_risk)}`}>
                              {formatRiskScore(riskMetrics.volatility_risk)}%
                            </div>
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Protocol Events Tab */}
        {activeTab === 'events' && (
          <div>
            {isLoadingEvents ? (
              <div className="flex items-center justify-center py-12">
                <LoadingSpinner className="w-8 h-8" />
                <span className="ml-2 text-gray-400">Loading protocol events...</span>
              </div>
            ) : events.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No recent protocol events</p>
              </div>
            ) : (
              <div className="space-y-3">
                {events.map((event) => (
                  <div key={event.id} className="bg-gray-800/50 rounded-lg p-4 border border-gray-600">
                    <div className="flex items-start justify-between mb-2">
                      <div className="flex-1">
                        <h3 className="font-medium text-white mb-1">{event.title}</h3>
                        <p className="text-sm text-gray-400">{event.protocol_name}</p>
                      </div>
                      <div className={`px-2 py-1 rounded text-xs font-medium ${
                        event.severity === 'Critical' ? 'bg-red-900/50 text-red-400' :
                        event.severity === 'High' ? 'bg-orange-900/50 text-orange-400' :
                        event.severity === 'Medium' ? 'bg-yellow-900/50 text-yellow-400' :
                        'bg-blue-900/50 text-blue-400'
                      }`}>
                        {event.severity}
                      </div>
                    </div>
                    
                    <p className="text-sm text-gray-300 mb-3">{event.description}</p>
                    
                    <div className="flex items-center justify-between text-xs text-gray-400">
                      <span>{event.event_type}</span>
                      <span>{new Date(event.event_timestamp).toLocaleString()}</span>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Alerts Tab */}
        {activeTab === 'alerts' && (
          <div>
            <div className="text-center py-12 text-gray-400">
              <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
              <p>No active alerts</p>
              <p className="text-sm mt-2">Alerts will appear here when risk thresholds are exceeded</p>
              <p className="text-xs mt-4 text-gray-500">Alert system integration coming soon</p>
            </div>
          </div>
        )}
      </div>

      {/* Footer with last update info */}
      <div className="mt-6 pt-4 border-t border-gray-700 flex items-center justify-between text-xs text-gray-400">
        <div>
          {lastRiskUpdate && (
            <span>Risk data updated: {lastRiskUpdate.toLocaleTimeString()}</span>
          )}
        </div>
        <div>
          {lastEventUpdate && (
            <span>Events updated: {lastEventUpdate.toLocaleTimeString()}</span>
          )}
        </div>
      </div>
    </div>
  );
};

export default RiskDashboard;
