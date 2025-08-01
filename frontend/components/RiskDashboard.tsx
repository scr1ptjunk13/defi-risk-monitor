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
import { Position, ProtocolEvent, RiskMetrics, RiskExplanation, AlertThreshold } from '../lib/api-client';
import { toast } from 'react-hot-toast';
import RiskMetricsCard from './RiskMetricsCard';
import PositionCard from './PositionCard';
import AlertsPanel from './AlertsPanel';

interface RiskDashboardProps {
  className?: string;
  userAddress: string;
}

const RiskDashboard: React.FC<RiskDashboardProps> = ({ userAddress }) => {
  const [activeTab, setActiveTab] = useState<'positions' | 'events' | 'alerts'>('positions');
  const [selectedPosition, setSelectedPosition] = useState<string | null>(null);
  const [riskExplanations, setRiskExplanations] = useState<Record<string, RiskExplanation>>({});
  const [alerts, setAlerts] = useState<AlertThreshold[]>([]);
  const [alertsLoading, setAlertsLoading] = useState(false);

  const {
    positions,
    riskMetrics,
    riskExplanation,
    isLoadingPositions,
    isLoadingRisk,
    error,
    isConnected,
    retryCount,
    clearError,
    calculateRisk,
    refreshPositions,
    explainRisk
  } = useRiskMonitoring(userAddress);

  const {
    events,
    isLoadingEvents: eventsLoading,
    error: eventsError
  } = useProtocolEvents();

  useEffect(() => {
    if (userAddress) {
      refreshPositions();
      loadAlerts();
    }
  }, [userAddress, refreshPositions]);

  const loadAlerts = async () => {
    if (!userAddress) return;

    setAlertsLoading(true);
    try {
      const mockAlerts: AlertThreshold[] = [
        {
          id: '1',
          user_address: userAddress,
          threshold_type: 'overall_risk',
          operator: 'greater_than',
          value: '80',
          enabled: true,
          created_at: new Date().toISOString()
        }
      ];
      setAlerts(mockAlerts);
    } catch (error) {
      console.error('Failed to load alerts:', error);
    } finally {
      setAlertsLoading(false);
    }
  };

  const handleExplainRisk = async (positionId: string) => {
    if (!explainRisk) return;
    
    try {
      await explainRisk(positionId);
      // The explanation will be available in riskExplanation state after the call
      if (riskExplanation) {
        setRiskExplanations(prev => ({ ...prev, [positionId]: riskExplanation }));
      }
    } catch (error) {
      toast.error('Failed to get risk explanation');
    }
  };

  const handleCreateAlert = async (alert: Omit<AlertThreshold, 'id' | 'created_at'>) => {
    const newAlert: AlertThreshold = {
      ...alert,
      id: Date.now().toString(),
      created_at: new Date().toISOString()
    };
    setAlerts(prev => [...prev, newAlert]);
  };

  const handleUpdateAlert = async (alertId: string, updates: Partial<AlertThreshold>) => {
    setAlerts(prev => prev.map(alert => 
      alert.id === alertId ? { ...alert, ...updates } : alert
    ));
  };

  const handleDeleteAlert = async (alertId: string) => {
    setAlerts(prev => prev.filter(alert => alert.id !== alertId));
  };

  // Connection status indicator
  const ConnectionStatus = ({ isConnected, label }: { isConnected: boolean; label: string }) => (
    <div className="flex items-center gap-2">
      <div className={`w-2 h-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
      <span className="text-xs text-gray-400">{label}: {isConnected ? 'Connected' : 'Disconnected'}</span>
    </div>
  );

  return (
    <div className={`bg-gray-900 rounded-2xl p-6 border border-gray-700`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-white">DeFi Risk Dashboard</h2>
        <div className="flex items-center gap-4">
          <ConnectionStatus isConnected={isConnected} label="Risk Monitor" />
          <ConnectionStatus isConnected={true} label="Protocol Events" />
        </div>
      </div>

      {/* Error Messages */}
      {(error || eventsError) && (
        <div className="mb-4 p-3 bg-red-900/20 border border-red-500/30 rounded-lg">
          {error && (
            <div className="flex items-center justify-between text-red-400 text-sm mb-2">
              <span>⚠️ Risk Monitor: {error.message || error.toString()}</span>
              <button onClick={clearError} className="text-blue-400 hover:text-blue-300">
                Clear
              </button>
            </div>
          )}
          {eventsError && (
            <div className="flex items-center justify-between text-red-400 text-sm">
              <span>⚠️ Protocol Events: {eventsError}</span>
              <button onClick={() => {}} className="text-blue-400 hover:text-blue-300">
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
          { key: 'alerts', label: 'Alerts', count: alerts.length },
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
          <div className="space-y-6">
            {isLoadingPositions ? (
              <div className="flex items-center justify-center py-12">
                <LoadingSpinner className="w-8 h-8" />
                <span className="ml-2 text-gray-400">Loading positions...</span>
              </div>
            ) : positions.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No positions found</p>
                <p className="text-sm mt-2">Connect your wallet to view your DeFi positions</p>
              </div>
            ) : (
              <div className="space-y-6">
                {/* Overall Risk Summary */}
                {riskMetrics && Object.keys(riskMetrics).length > 0 && (
                  <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    {riskMetrics && positions[0] && (riskMetrics as any)[positions[0].id] && (
                      <RiskMetricsCard
                        metrics={(riskMetrics as any)[positions[0].id]}
                        explanation={riskExplanations[positions[0].id]}
                        onExplainRisk={() => handleExplainRisk(positions[0].id)}
                      />
                    )}
                    
                    <div className="bg-gray-800/50 rounded-xl p-6 border border-gray-700">
                      <h3 className="text-lg font-semibold text-white mb-4">Portfolio Overview</h3>
                      <div className="grid grid-cols-2 gap-4">
                        <div className="bg-gray-900/50 rounded-lg p-3">
                          <div className="text-xs text-gray-400 mb-1">Total Positions</div>
                          <div className="text-2xl font-bold text-white">{positions.length}</div>
                        </div>
                        <div className="bg-gray-900/50 rounded-lg p-3">
                          <div className="text-xs text-gray-400 mb-1">Total Value</div>
                          <div className="text-2xl font-bold text-white">
                            ${positions.reduce((sum, pos) => sum + parseFloat(pos.current_value_usd), 0).toLocaleString()}
                          </div>
                        </div>
                        <div className="bg-gray-900/50 rounded-lg p-3">
                          <div className="text-xs text-gray-400 mb-1">Avg Risk Score</div>
                          <div className="text-2xl font-bold text-white">
                            {riskMetrics && Object.values(riskMetrics).length > 0 ? 
                              (Object.values(riskMetrics).reduce((sum: number, risk: any) => sum + parseFloat(risk.overall_risk_score), 0) / Object.values(riskMetrics).length).toFixed(1)
                              : '0.0'
                            }%
                          </div>
                        </div>
                        <div className="bg-gray-900/50 rounded-lg p-3">
                          <div className="text-xs text-gray-400 mb-1">Active Alerts</div>
                          <div className="text-2xl font-bold text-white">
                            {alerts.filter(alert => alert.enabled).length}
                          </div>
                        </div>
                      </div>
                    </div>
                  </div>
                )}
                
                {/* Position Cards */}
                <div className="grid gap-6">
                  {positions.map((position) => (
                    <PositionCard
                      key={position.id}
                      position={position}
                      riskMetrics={riskMetrics ? (riskMetrics as any)[position.id] : undefined}
                      isLoadingRisk={isLoadingRisk}
                      onViewDetails={(positionId) => setSelectedPosition(positionId)}
                      onCalculateRisk={calculateRisk}
                      onExplainRisk={handleExplainRisk}
                    />
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {/* Protocol Events Tab */}
        {activeTab === 'events' && (
          <div>
            {eventsLoading ? (
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
          <AlertsPanel
            userAddress={userAddress}
            alerts={alerts}
            isLoading={alertsLoading}
            onCreateAlert={handleCreateAlert}
            onUpdateAlert={handleUpdateAlert}
            onDeleteAlert={handleDeleteAlert}
          />
        )}
      </div>

      {/* Footer */}
      <div className="mt-6 pt-4 border-t border-gray-700 text-xs text-gray-500">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            Risk Data: {new Date().toLocaleTimeString()}
          </div>
          <div>
            Events: {new Date().toLocaleTimeString()}
          </div>
        </div>
      </div>
    </div>
  );
};

export default RiskDashboard;
