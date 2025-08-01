/**
 * AlertConfigurationUI Component
 * 
 * Advanced alert configuration interface with templates, notification preferences,
 * alert history, and smart recommendations
 */

import React, { useState, useEffect } from 'react';
import { AlertThreshold } from '../lib/api-client';
import { LoadingSpinner, InfoIcon, SettingsIcon } from './Icons';
import { toast } from 'react-hot-toast';

interface AlertConfigurationUIProps {
  userAddress?: string;
  alerts: AlertThreshold[];
  isLoading?: boolean;
  onCreateAlert?: (alert: Omit<AlertThreshold, 'id' | 'created_at'>) => Promise<void>;
  onUpdateAlert?: (alertId: string, updates: Partial<AlertThreshold>) => Promise<void>;
  onDeleteAlert?: (alertId: string) => Promise<void>;
  className?: string;
}

interface AlertTemplate {
  id: string;
  name: string;
  description: string;
  alerts: Omit<AlertThreshold, 'id' | 'created_at' | 'user_address'>[];
  category: 'conservative' | 'moderate' | 'aggressive';
}

interface NotificationSettings {
  email: boolean;
  push: boolean;
  sms: boolean;
  webhook: boolean;
  webhookUrl?: string;
}

const AlertConfigurationUI: React.FC<AlertConfigurationUIProps> = ({
  userAddress,
  alerts,
  isLoading = false,
  onCreateAlert,
  onUpdateAlert,
  onDeleteAlert,
  className = ''
}) => {
  const [activeTab, setActiveTab] = useState<'alerts' | 'templates' | 'settings' | 'history'>('alerts');
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingAlert, setEditingAlert] = useState<AlertThreshold | null>(null);
  const [notificationSettings, setNotificationSettings] = useState<NotificationSettings>({
    email: true,
    push: true,
    sms: false,
    webhook: false
  });
  const [alertHistory, setAlertHistory] = useState<any[]>([]);

  const [formData, setFormData] = useState({
    threshold_type: 'overall_risk',
    operator: 'greater_than',
    value: '70',
    enabled: true
  });

  // Alert Templates
  const alertTemplates: AlertTemplate[] = [
    {
      id: 'conservative',
      name: 'Conservative Risk Management',
      description: 'Low-risk thresholds for cautious investors',
      category: 'conservative',
      alerts: [
        { threshold_type: 'overall_risk', operator: 'greater_than', value: '60', enabled: true },
        { threshold_type: 'mev_risk', operator: 'greater_than', value: '40', enabled: true },
        { threshold_type: 'impermanent_loss', operator: 'greater_than', value: '5', enabled: true },
        { threshold_type: 'pnl_percentage', operator: 'less_than', value: '-10', enabled: true }
      ]
    },
    {
      id: 'moderate',
      name: 'Balanced Risk Management',
      description: 'Moderate thresholds for balanced portfolios',
      category: 'moderate',
      alerts: [
        { threshold_type: 'overall_risk', operator: 'greater_than', value: '75', enabled: true },
        { threshold_type: 'mev_risk', operator: 'greater_than', value: '60', enabled: true },
        { threshold_type: 'impermanent_loss', operator: 'greater_than', value: '10', enabled: true },
        { threshold_type: 'pnl_percentage', operator: 'less_than', value: '-20', enabled: true }
      ]
    },
    {
      id: 'aggressive',
      name: 'High-Risk Tolerance',
      description: 'Higher thresholds for risk-tolerant investors',
      category: 'aggressive',
      alerts: [
        { threshold_type: 'overall_risk', operator: 'greater_than', value: '85', enabled: true },
        { threshold_type: 'mev_risk', operator: 'greater_than', value: '80', enabled: true },
        { threshold_type: 'impermanent_loss', operator: 'greater_than', value: '20', enabled: true },
        { threshold_type: 'pnl_percentage', operator: 'less_than', value: '-30', enabled: true }
      ]
    }
  ];

  const thresholdTypes = [
    { value: 'overall_risk', label: 'Overall Risk Score', unit: '%' },
    { value: 'liquidity_risk', label: 'Liquidity Risk', unit: '%' },
    { value: 'volatility_risk', label: 'Volatility Risk', unit: '%' },
    { value: 'protocol_risk', label: 'Protocol Risk', unit: '%' },
    { value: 'mev_risk', label: 'MEV Risk', unit: '%' },
    { value: 'cross_chain_risk', label: 'Cross-Chain Risk', unit: '%' },
    { value: 'impermanent_loss', label: 'Impermanent Loss', unit: '%' },
    { value: 'position_value', label: 'Position Value', unit: '$' },
    { value: 'pnl_percentage', label: 'P&L Percentage', unit: '%' }
  ];

  const operators = [
    { value: 'greater_than', label: 'Greater than', symbol: '>' },
    { value: 'less_than', label: 'Less than', symbol: '<' },
    { value: 'equals', label: 'Equals', symbol: '=' },
    { value: 'greater_equal', label: 'Greater than or equal', symbol: '≥' },
    { value: 'less_equal', label: 'Less than or equal', symbol: '≤' }
  ];

  // Load alert history (mock data for now)
  useEffect(() => {
    const mockHistory = [
      {
        id: '1',
        timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000).toISOString(),
        type: 'overall_risk',
        message: 'Overall risk exceeded 80% threshold',
        severity: 'high',
        position: 'ETH/USDC',
        acknowledged: true
      },
      {
        id: '2',
        timestamp: new Date(Date.now() - 6 * 60 * 60 * 1000).toISOString(),
        type: 'mev_risk',
        message: 'MEV risk increased to 65%',
        severity: 'medium',
        position: 'WBTC/ETH',
        acknowledged: false
      }
    ];
    setAlertHistory(mockHistory);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!userAddress || !onCreateAlert) return;

    try {
      await onCreateAlert({
        user_address: userAddress,
        threshold_type: formData.threshold_type,
        operator: formData.operator,
        value: formData.value,
        enabled: formData.enabled
      });
      
      setShowCreateForm(false);
      resetForm();
      toast.success('Alert created successfully!');
    } catch (error) {
      toast.error('Failed to create alert');
    }
  };

  const handleApplyTemplate = async (template: AlertTemplate) => {
    if (!userAddress || !onCreateAlert) return;

    try {
      for (const alertConfig of template.alerts) {
        await onCreateAlert({
          user_address: userAddress,
          ...alertConfig
        });
      }
      toast.success(`Applied ${template.name} template successfully!`);
    } catch (error) {
      toast.error('Failed to apply template');
    }
  };

  const resetForm = () => {
    setFormData({
      threshold_type: 'overall_risk',
      operator: 'greater_than',
      value: '70',
      enabled: true
    });
    setEditingAlert(null);
  };

  const getAlertColor = (alert: AlertThreshold) => {
    if (!alert.enabled) return 'text-gray-400 bg-gray-900/20 border-gray-600';
    
    const value = parseFloat(alert.value);
    if (alert.threshold_type.includes('risk')) {
      if (value >= 80) return 'text-red-400 bg-red-900/20 border-red-500/30';
      if (value >= 60) return 'text-orange-400 bg-orange-900/20 border-orange-500/30';
      if (value >= 30) return 'text-yellow-400 bg-yellow-900/20 border-yellow-500/30';
      return 'text-green-400 bg-green-900/20 border-green-500/30';
    }
    
    return 'text-blue-400 bg-blue-900/20 border-blue-500/30';
  };

  const getTemplateColor = (category: string) => {
    switch (category) {
      case 'conservative': return 'border-green-500/30 bg-green-900/10';
      case 'moderate': return 'border-yellow-500/30 bg-yellow-900/10';
      case 'aggressive': return 'border-red-500/30 bg-red-900/10';
      default: return 'border-gray-500/30 bg-gray-900/10';
    }
  };

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Alert Configuration</h3>
          <p className="text-sm text-gray-400 mt-1">
            Manage risk alerts and notification preferences
          </p>
        </div>
        
        {userAddress && onCreateAlert && activeTab === 'alerts' && (
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
          >
            {showCreateForm ? 'Cancel' : 'Create Alert'}
          </button>
        )}
      </div>

      {/* Tab Navigation */}
      <div className="flex border-b border-gray-700 mb-6">
        {[
          { key: 'alerts', label: 'Active Alerts', count: alerts.length },
          { key: 'templates', label: 'Templates', count: alertTemplates.length },
          { key: 'settings', label: 'Notifications', count: null },
          { key: 'history', label: 'History', count: alertHistory.length }
        ].map(({ key, label, count }) => (
          <button
            key={key}
            onClick={() => setActiveTab(key as any)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === key
                ? 'border-blue-500 text-blue-400'
                : 'border-transparent text-gray-400 hover:text-gray-300'
            }`}
          >
            {label}
            {count !== null && (
              <span className="ml-2 px-2 py-1 text-xs bg-gray-700 rounded-full">
                {count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      <div className="min-h-[400px]">
        {/* Active Alerts Tab */}
        {activeTab === 'alerts' && (
          <div>
            {/* Create Alert Form */}
            {showCreateForm && (
              <div className="mb-6 p-4 bg-gray-900/50 rounded-lg border border-gray-600">
                <h4 className="text-md font-medium text-white mb-4">Create New Alert</h4>
                
                <form onSubmit={handleSubmit} className="space-y-4">
                  <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">Alert Type</label>
                      <select
                        value={formData.threshold_type}
                        onChange={(e) => setFormData(prev => ({ ...prev, threshold_type: e.target.value }))}
                        className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                      >
                        {thresholdTypes.map(type => (
                          <option key={type.value} value={type.value}>{type.label}</option>
                        ))}
                      </select>
                    </div>
                    
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">Condition</label>
                      <select
                        value={formData.operator}
                        onChange={(e) => setFormData(prev => ({ ...prev, operator: e.target.value }))}
                        className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                      >
                        {operators.map(op => (
                          <option key={op.value} value={op.value}>{op.label}</option>
                        ))}
                      </select>
                    </div>
                    
                    <div>
                      <label className="block text-sm text-gray-400 mb-2">
                        Value ({thresholdTypes.find(t => t.value === formData.threshold_type)?.unit})
                      </label>
                      <input
                        type="number"
                        step="0.1"
                        value={formData.value}
                        onChange={(e) => setFormData(prev => ({ ...prev, value: e.target.value }))}
                        className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                        required
                      />
                    </div>
                  </div>
                  
                  <div className="flex items-center gap-2">
                    <input
                      type="checkbox"
                      id="enabled"
                      checked={formData.enabled}
                      onChange={(e) => setFormData(prev => ({ ...prev, enabled: e.target.checked }))}
                      className="rounded border-gray-600 bg-gray-700 text-blue-600"
                    />
                    <label htmlFor="enabled" className="text-sm text-gray-300">
                      Enable alert immediately
                    </label>
                  </div>
                  
                  <div className="flex justify-end gap-2">
                    <button
                      type="button"
                      onClick={() => { setShowCreateForm(false); resetForm(); }}
                      className="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors"
                    >
                      Cancel
                    </button>
                    <button
                      type="submit"
                      className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
                    >
                      Create Alert
                    </button>
                  </div>
                </form>
              </div>
            )}

            {/* Alerts List */}
            {isLoading ? (
              <div className="flex items-center justify-center py-12">
                <LoadingSpinner className="w-8 h-8" />
                <span className="ml-2 text-gray-400">Loading alerts...</span>
              </div>
            ) : alerts.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No alerts configured</p>
                <p className="text-sm mt-2">Create your first alert or use a template</p>
              </div>
            ) : (
              <div className="space-y-3">
                {alerts.map((alert) => (
                  <div key={alert.id} className={`p-4 rounded-lg border ${getAlertColor(alert)}`}>
                    <div className="flex items-center justify-between">
                      <div className="flex-1">
                        <div className="flex items-center gap-3 mb-2">
                          <span className="font-medium text-white">
                            {thresholdTypes.find(t => t.value === alert.threshold_type)?.label}
                          </span>
                          <span className="text-sm text-gray-400">
                            {operators.find(o => o.value === alert.operator)?.symbol} {alert.value}
                            {thresholdTypes.find(t => t.value === alert.threshold_type)?.unit}
                          </span>
                          <div className={`px-2 py-1 rounded text-xs ${
                            alert.enabled ? 'bg-green-900/50 text-green-400' : 'bg-gray-900/50 text-gray-400'
                          }`}>
                            {alert.enabled ? 'Active' : 'Disabled'}
                          </div>
                        </div>
                        
                        <div className="text-xs text-gray-400">
                          Created: {new Date(alert.created_at).toLocaleDateString()}
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() => onUpdateAlert?.(alert.id, { enabled: !alert.enabled })}
                          className={`px-3 py-1 text-xs rounded-lg transition-colors ${
                            alert.enabled 
                              ? 'bg-yellow-600 hover:bg-yellow-700 text-white' 
                              : 'bg-green-600 hover:bg-green-700 text-white'
                          }`}
                        >
                          {alert.enabled ? 'Disable' : 'Enable'}
                        </button>
                        
                        <button
                          onClick={() => onDeleteAlert?.(alert.id)}
                          className="px-3 py-1 text-xs bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors"
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {/* Templates Tab */}
        {activeTab === 'templates' && (
          <div className="space-y-4">
            <p className="text-sm text-gray-400 mb-4">
              Quick setup with pre-configured alert templates based on risk tolerance
            </p>
            
            {alertTemplates.map((template) => (
              <div key={template.id} className={`p-4 rounded-lg border ${getTemplateColor(template.category)}`}>
                <div className="flex items-center justify-between mb-3">
                  <div>
                    <h4 className="font-medium text-white">{template.name}</h4>
                    <p className="text-sm text-gray-400 mt-1">{template.description}</p>
                  </div>
                  
                  <button
                    onClick={() => handleApplyTemplate(template)}
                    className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded-lg transition-colors"
                  >
                    Apply Template
                  </button>
                </div>
                
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                  {template.alerts.map((alert, index) => (
                    <div key={index} className="text-xs text-gray-300 bg-gray-900/30 rounded px-2 py-1">
                      {thresholdTypes.find(t => t.value === alert.threshold_type)?.label}{' '}
                      {operators.find(o => o.value === alert.operator)?.symbol} {alert.value}
                      {thresholdTypes.find(t => t.value === alert.threshold_type)?.unit}
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Notification Settings Tab */}
        {activeTab === 'settings' && (
          <div className="space-y-6">
            <div>
              <h4 className="font-medium text-white mb-4">Notification Channels</h4>
              
              <div className="space-y-4">
                {Object.entries(notificationSettings).map(([key, value]) => {
                  if (key === 'webhookUrl') return null;
                  
                  return (
                    <div key={key} className="flex items-center justify-between p-3 bg-gray-900/30 rounded-lg">
                      <div>
                        <div className="font-medium text-white capitalize">
                          {key === 'push' ? 'Push Notifications' : key.charAt(0).toUpperCase() + key.slice(1)}
                        </div>
                        <div className="text-sm text-gray-400">
                          {key === 'email' && 'Receive alerts via email'}
                          {key === 'push' && 'Browser push notifications'}
                          {key === 'sms' && 'SMS text messages'}
                          {key === 'webhook' && 'Custom webhook integration'}
                        </div>
                      </div>
                      
                      <label className="relative inline-flex items-center cursor-pointer">
                        <input
                          type="checkbox"
                          checked={value as boolean}
                          onChange={(e) => setNotificationSettings(prev => ({ 
                            ...prev, 
                            [key]: e.target.checked 
                          }))}
                          className="sr-only peer"
                        />
                        <div className="w-11 h-6 bg-gray-600 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"></div>
                      </label>
                    </div>
                  );
                })}
                
                {notificationSettings.webhook && (
                  <div className="p-3 bg-gray-900/30 rounded-lg">
                    <label className="block text-sm text-gray-400 mb-2">Webhook URL</label>
                    <input
                      type="url"
                      value={notificationSettings.webhookUrl || ''}
                      onChange={(e) => setNotificationSettings(prev => ({ 
                        ...prev, 
                        webhookUrl: e.target.value 
                      }))}
                      placeholder="https://your-webhook-url.com/alerts"
                      className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                    />
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Alert History Tab */}
        {activeTab === 'history' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h4 className="font-medium text-white">Recent Alert Activity</h4>
              <button className="text-sm text-blue-400 hover:text-blue-300">
                Clear History
              </button>
            </div>
            
            {alertHistory.length === 0 ? (
              <div className="text-center py-12 text-gray-400">
                <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
                <p>No alert history</p>
                <p className="text-sm mt-2">Alert notifications will appear here</p>
              </div>
            ) : (
              <div className="space-y-3">
                {alertHistory.map((historyItem) => (
                  <div key={historyItem.id} className="p-4 bg-gray-900/30 rounded-lg border border-gray-600">
                    <div className="flex items-center justify-between mb-2">
                      <div className="flex items-center gap-2">
                        <div className={`w-2 h-2 rounded-full ${
                          historyItem.severity === 'high' ? 'bg-red-500' :
                          historyItem.severity === 'medium' ? 'bg-yellow-500' :
                          'bg-blue-500'
                        }`} />
                        <span className="font-medium text-white">{historyItem.position}</span>
                        <span className={`px-2 py-1 text-xs rounded ${
                          historyItem.acknowledged ? 'bg-green-900/50 text-green-400' : 'bg-orange-900/50 text-orange-400'
                        }`}>
                          {historyItem.acknowledged ? 'Acknowledged' : 'Pending'}
                        </span>
                      </div>
                      
                      <span className="text-xs text-gray-400">
                        {new Date(historyItem.timestamp).toLocaleString()}
                      </span>
                    </div>
                    
                    <p className="text-sm text-gray-300">{historyItem.message}</p>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};

export default AlertConfigurationUI;
