'use client';

import React, { useState, useEffect } from 'react';

interface AlertsCenterProps {
  userAddress: string;
  userTier: 'basic' | 'professional' | 'institutional' | 'enterprise';
}

interface AlertRule {
  id: string;
  name: string;
  type: 'risk_threshold' | 'price_change' | 'liquidity_change' | 'protocol_event' | 'mev_detection';
  condition: string;
  threshold: number;
  enabled: boolean;
  notifications: ('email' | 'webhook' | 'sms')[];
  positions?: string[];
  protocols?: string[];
  createdAt: number;
  triggeredCount: number;
}

interface Alert {
  id: string;
  ruleId: string;
  ruleName: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  message: string;
  details: string;
  positionId?: string;
  protocol?: string;
  timestamp: number;
  acknowledged: boolean;
  resolvedAt?: number;
}

const AlertsCenter: React.FC<AlertsCenterProps> = ({ userAddress, userTier }) => {
  const [activeTab, setActiveTab] = useState<'alerts' | 'rules' | 'history'>('alerts');
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [alertRules, setAlertRules] = useState<AlertRule[]>([]);
  const [showCreateRule, setShowCreateRule] = useState(false);
  const [newRule, setNewRule] = useState({
    name: '',
    type: 'risk_threshold' as const,
    condition: 'greater_than',
    threshold: 70,
    notifications: ['email'] as ('email' | 'webhook' | 'sms')[]
  });

  useEffect(() => {
    // Initialize with mock data
    setAlerts([
      {
        id: '1',
        ruleId: 'rule_1',
        ruleName: 'High Risk Alert',
        severity: 'high',
        message: 'Risk score exceeded threshold',
        details: 'ETH/USDC position risk score reached 85/100, above your threshold of 80',
        positionId: 'pos_1',
        protocol: 'Uniswap V3',
        timestamp: Date.now() - 300000, // 5 minutes ago
        acknowledged: false
      },
      {
        id: '2',
        ruleId: 'rule_2',
        ruleName: 'MEV Detection',
        severity: 'critical',
        message: 'MEV attack detected',
        details: 'Sandwich attack detected on your large ETH/USDC swap',
        positionId: 'pos_1',
        protocol: 'Uniswap V3',
        timestamp: Date.now() - 600000, // 10 minutes ago
        acknowledged: false
      },
      {
        id: '3',
        ruleId: 'rule_3',
        ruleName: 'Liquidity Drop',
        severity: 'medium',
        message: 'Pool liquidity decreased',
        details: 'stETH/ETH pool liquidity dropped by 25% in the last hour',
        protocol: 'Curve',
        timestamp: Date.now() - 1800000, // 30 minutes ago
        acknowledged: true
      }
    ]);

    setAlertRules([
      {
        id: 'rule_1',
        name: 'High Risk Alert',
        type: 'risk_threshold',
        condition: 'greater_than',
        threshold: 80,
        enabled: true,
        notifications: ['email', 'webhook'],
        positions: ['pos_1', 'pos_2'],
        createdAt: Date.now() - 86400000,
        triggeredCount: 3
      },
      {
        id: 'rule_2',
        name: 'MEV Detection',
        type: 'mev_detection',
        condition: 'detected',
        threshold: 0,
        enabled: true,
        notifications: ['email', 'sms'],
        protocols: ['Uniswap V3', 'SushiSwap'],
        createdAt: Date.now() - 172800000,
        triggeredCount: 1
      },
      {
        id: 'rule_3',
        name: 'Liquidity Drop',
        type: 'liquidity_change',
        condition: 'decreases_by',
        threshold: 20,
        enabled: true,
        notifications: ['email'],
        protocols: ['Curve', 'Balancer'],
        createdAt: Date.now() - 259200000,
        triggeredCount: 5
      }
    ]);
  }, [userAddress]);

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical': return 'text-red-400 bg-red-900/20 border-red-500/30';
      case 'high': return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
      case 'medium': return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
      default: return 'text-blue-400 bg-blue-900/20 border-blue-500/30';
    }
  };

  const acknowledgeAlert = (alertId: string) => {
    setAlerts(prev => prev.map(alert => 
      alert.id === alertId ? { ...alert, acknowledged: true } : alert
    ));
  };

  const toggleRule = (ruleId: string) => {
    setAlertRules(prev => prev.map(rule => 
      rule.id === ruleId ? { ...rule, enabled: !rule.enabled } : rule
    ));
  };

  const createRule = () => {
    const rule: AlertRule = {
      id: `rule_${Date.now()}`,
      ...newRule,
      enabled: true,
      positions: [],
      protocols: [],
      createdAt: Date.now(),
      triggeredCount: 0
    };

    setAlertRules(prev => [...prev, rule]);
    setShowCreateRule(false);
    setNewRule({
      name: '',
      type: 'risk_threshold',
      condition: 'greater_than',
      threshold: 70,
      notifications: ['email']
    });
  };

  const formatTimeAgo = (timestamp: number) => {
    const diff = Date.now() - timestamp;
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    return `${minutes}m ago`;
  };

  return (
    <div className="space-y-6">
      {/* Header with Tab Navigation */}
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold text-white">Alerts Center</h2>
        
        <div className="flex space-x-2">
          {[
            { id: 'alerts', label: 'Active Alerts', icon: '🚨' },
            { id: 'rules', label: 'Alert Rules', icon: '⚙️' },
            { id: 'history', label: 'History', icon: '📜' }
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as any)}
              className={`flex items-center space-x-1 px-3 py-1 rounded text-sm transition-colors ${
                activeTab === tab.id
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
              }`}
            >
              <span>{tab.icon}</span>
              <span>{tab.label}</span>
            </button>
          ))}
        </div>
      </div>

      {/* Active Alerts */}
      {activeTab === 'alerts' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <span className="text-sm text-gray-400">
                {alerts.filter(a => !a.acknowledged).length} active alerts
              </span>
              <span className="text-sm text-gray-400">
                {alerts.filter(a => a.acknowledged).length} acknowledged
              </span>
            </div>
            <button 
              onClick={() => setAlerts(prev => prev.map(a => ({ ...a, acknowledged: true })))}
              className="text-sm text-blue-400 hover:text-blue-300"
            >
              Acknowledge All
            </button>
          </div>

          <div className="space-y-3">
            {alerts.length === 0 ? (
              <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-8 text-center">
                <div className="text-4xl mb-4">✅</div>
                <h3 className="text-lg font-semibold text-white mb-2">No Active Alerts</h3>
                <p className="text-gray-400">Your portfolio is operating within normal parameters.</p>
              </div>
            ) : (
              alerts.map((alert) => (
                <div
                  key={alert.id}
                  className={`p-4 rounded-lg border transition-all ${
                    alert.acknowledged 
                      ? 'bg-gray-800/30 border-gray-600 opacity-60' 
                      : `bg-gray-900/50 border-gray-700 ${getSeverityColor(alert.severity).includes('animate-pulse') ? 'animate-pulse' : ''}`
                  }`}
                >
                  <div className="flex items-start justify-between">
                    <div className="flex-1">
                      <div className="flex items-center space-x-2 mb-2">
                        <span className={`text-xs font-medium px-2 py-1 rounded border ${getSeverityColor(alert.severity)}`}>
                          {alert.severity.toUpperCase()}
                        </span>
                        <span className="text-sm font-medium text-white">{alert.ruleName}</span>
                        <span className="text-xs text-gray-400">{formatTimeAgo(alert.timestamp)}</span>
                      </div>
                      
                      <p className="text-sm text-white mb-1">{alert.message}</p>
                      <p className="text-xs text-gray-400 mb-2">{alert.details}</p>
                      
                      <div className="flex items-center space-x-4 text-xs text-gray-400">
                        {alert.protocol && <span>Protocol: {alert.protocol}</span>}
                        {alert.positionId && <span>Position: {alert.positionId}</span>}
                      </div>
                    </div>
                    
                    <div className="flex items-center space-x-2 ml-4">
                      {!alert.acknowledged && (
                        <button
                          onClick={() => acknowledgeAlert(alert.id)}
                          className="text-xs bg-blue-600 hover:bg-blue-700 text-white px-2 py-1 rounded"
                        >
                          Acknowledge
                        </button>
                      )}
                      <button className="text-xs text-gray-400 hover:text-gray-300">
                        Details
                      </button>
                    </div>
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      )}

      {/* Alert Rules */}
      {activeTab === 'rules' && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">
              {alertRules.filter(r => r.enabled).length} of {alertRules.length} rules active
            </span>
            <button
              onClick={() => setShowCreateRule(true)}
              className="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded text-sm"
            >
              Create Rule
            </button>
          </div>

          {/* Create Rule Form */}
          {showCreateRule && (
            <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
              <h3 className="text-lg font-semibold text-white mb-4">Create Alert Rule</h3>
              
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-4">
                <div>
                  <label className="block text-sm text-gray-400 mb-2">Rule Name</label>
                  <input
                    type="text"
                    value={newRule.name}
                    onChange={(e) => setNewRule(prev => ({ ...prev, name: e.target.value }))}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                    placeholder="Enter rule name"
                  />
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-2">Alert Type</label>
                  <select
                    value={newRule.type}
                    onChange={(e) => setNewRule(prev => ({ ...prev, type: e.target.value as any }))}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  >
                    <option value="risk_threshold">Risk Threshold</option>
                    <option value="price_change">Price Change</option>
                    <option value="liquidity_change">Liquidity Change</option>
                    <option value="protocol_event">Protocol Event</option>
                    <option value="mev_detection">MEV Detection</option>
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-2">Condition</label>
                  <select
                    value={newRule.condition}
                    onChange={(e) => setNewRule(prev => ({ ...prev, condition: e.target.value }))}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  >
                    <option value="greater_than">Greater Than</option>
                    <option value="less_than">Less Than</option>
                    <option value="changes_by">Changes By</option>
                    <option value="detected">Detected</option>
                  </select>
                </div>
                
                <div>
                  <label className="block text-sm text-gray-400 mb-2">Threshold</label>
                  <input
                    type="number"
                    value={newRule.threshold}
                    onChange={(e) => setNewRule(prev => ({ ...prev, threshold: parseFloat(e.target.value) }))}
                    className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                  />
                </div>
              </div>
              
              <div className="mb-4">
                <label className="block text-sm text-gray-400 mb-2">Notifications</label>
                <div className="flex space-x-4">
                  {(['email', 'webhook', 'sms'] as const).map((type) => (
                    <label key={type} className="flex items-center space-x-2">
                      <input
                        type="checkbox"
                        checked={newRule.notifications.includes(type)}
                        onChange={(e) => {
                          if (e.target.checked) {
                            setNewRule(prev => ({ ...prev, notifications: [...prev.notifications, type] }));
                          } else {
                            setNewRule(prev => ({ ...prev, notifications: prev.notifications.filter(n => n !== type) }));
                          }
                        }}
                        className="rounded"
                      />
                      <span className="text-sm text-gray-300 capitalize">{type}</span>
                    </label>
                  ))}
                </div>
              </div>
              
              <div className="flex space-x-2">
                <button
                  onClick={createRule}
                  disabled={!newRule.name.trim()}
                  className="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white px-4 py-2 rounded text-sm"
                >
                  Create Rule
                </button>
                <button
                  onClick={() => setShowCreateRule(false)}
                  className="bg-gray-700 hover:bg-gray-600 text-gray-300 px-4 py-2 rounded text-sm"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}

          {/* Rules List */}
          <div className="space-y-3">
            {alertRules.map((rule) => (
              <div key={rule.id} className="bg-gray-900/50 border border-gray-700 rounded-lg p-4">
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center space-x-3">
                    <button
                      onClick={() => toggleRule(rule.id)}
                      className={`w-4 h-4 rounded border-2 flex items-center justify-center ${
                        rule.enabled 
                          ? 'bg-green-600 border-green-600' 
                          : 'border-gray-600'
                      }`}
                    >
                      {rule.enabled && <span className="text-white text-xs">✓</span>}
                    </button>
                    <div>
                      <h4 className="text-sm font-medium text-white">{rule.name}</h4>
                      <div className="text-xs text-gray-400">
                        {rule.type.replace('_', ' ')} • {rule.condition.replace('_', ' ')} {rule.threshold}
                      </div>
                    </div>
                  </div>
                  
                  <div className="flex items-center space-x-4">
                    <div className="text-xs text-gray-400">
                      Triggered: {rule.triggeredCount}x
                    </div>
                    <div className="flex space-x-1">
                      {rule.notifications.map((notif) => (
                        <span key={notif} className="text-xs bg-gray-700 text-gray-300 px-1 rounded">
                          {notif}
                        </span>
                      ))}
                    </div>
                    <button className="text-xs text-gray-400 hover:text-gray-300">
                      Edit
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Alert History */}
      {activeTab === 'history' && (
        <div className="bg-gray-900/50 border border-gray-700 rounded-lg p-6">
          <h3 className="text-lg font-semibold text-white mb-4">Alert History</h3>
          <div className="space-y-3">
            {alerts.filter(a => a.acknowledged).map((alert) => (
              <div key={alert.id} className="flex items-center justify-between py-2 border-b border-gray-700 last:border-b-0">
                <div>
                  <div className="text-sm text-white">{alert.message}</div>
                  <div className="text-xs text-gray-400">
                    {alert.protocol} • {formatTimeAgo(alert.timestamp)}
                  </div>
                </div>
                <span className={`text-xs px-2 py-1 rounded border ${getSeverityColor(alert.severity)}`}>
                  {alert.severity}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default AlertsCenter;
