/**
 * AlertsPanel Component
 * 
 * Comprehensive alert configuration and management interface
 * for setting up risk thresholds and notifications
 */

import React, { useState, useEffect } from 'react';
import { AlertThreshold } from '../lib/api-client';
import { LoadingSpinner, InfoIcon } from './Icons';
import { toast } from 'react-hot-toast';

interface AlertsPanelProps {
  userAddress?: string;
  alerts: AlertThreshold[];
  isLoading?: boolean;
  onCreateAlert?: (alert: Omit<AlertThreshold, 'id' | 'created_at'>) => Promise<void>;
  onUpdateAlert?: (alertId: string, updates: Partial<AlertThreshold>) => Promise<void>;
  onDeleteAlert?: (alertId: string) => Promise<void>;
  className?: string;
}

const AlertsPanel: React.FC<AlertsPanelProps> = ({
  userAddress,
  alerts,
  isLoading = false,
  onCreateAlert,
  onUpdateAlert,
  onDeleteAlert,
  className = ''
}) => {
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [editingAlert, setEditingAlert] = useState<AlertThreshold | null>(null);
  const [formData, setFormData] = useState({
    threshold_type: 'overall_risk',
    operator: 'greater_than',
    value: '70',
    enabled: true
  });

  const thresholdTypes = [
    { value: 'overall_risk', label: 'Overall Risk Score' },
    { value: 'liquidity_risk', label: 'Liquidity Risk' },
    { value: 'volatility_risk', label: 'Volatility Risk' },
    { value: 'protocol_risk', label: 'Protocol Risk' },
    { value: 'mev_risk', label: 'MEV Risk' },
    { value: 'cross_chain_risk', label: 'Cross-Chain Risk' },
    { value: 'impermanent_loss', label: 'Impermanent Loss' },
    { value: 'position_value', label: 'Position Value' },
    { value: 'pnl_percentage', label: 'P&L Percentage' }
  ];

  const operators = [
    { value: 'greater_than', label: 'Greater than', symbol: '>' },
    { value: 'less_than', label: 'Less than', symbol: '<' },
    { value: 'equals', label: 'Equals', symbol: '=' },
    { value: 'greater_equal', label: 'Greater than or equal', symbol: '≥' },
    { value: 'less_equal', label: 'Less than or equal', symbol: '≤' }
  ];

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
      setFormData({
        threshold_type: 'overall_risk',
        operator: 'greater_than',
        value: '70',
        enabled: true
      });
      
      toast.success('Alert created successfully!');
    } catch (error) {
      toast.error('Failed to create alert');
    }
  };

  const handleToggleAlert = async (alert: AlertThreshold) => {
    if (!onUpdateAlert) return;
    
    try {
      await onUpdateAlert(alert.id, { enabled: !alert.enabled });
      toast.success(`Alert ${alert.enabled ? 'disabled' : 'enabled'}`);
    } catch (error) {
      toast.error('Failed to update alert');
    }
  };

  const handleDeleteAlert = async (alertId: string) => {
    if (!onDeleteAlert) return;
    
    if (window.confirm('Are you sure you want to delete this alert?')) {
      try {
        await onDeleteAlert(alertId);
        toast.success('Alert deleted successfully');
      } catch (error) {
        toast.error('Failed to delete alert');
      }
    }
  };

  const getThresholdLabel = (type: string) => {
    return thresholdTypes.find(t => t.value === type)?.label || type;
  };

  const getOperatorSymbol = (operator: string) => {
    return operators.find(o => o.value === operator)?.symbol || operator;
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

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Alert Configuration</h3>
          <p className="text-sm text-gray-400 mt-1">
            Set up notifications for risk thresholds and position changes
          </p>
        </div>
        
        {userAddress && onCreateAlert && (
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
          >
            {showCreateForm ? 'Cancel' : 'Create Alert'}
          </button>
        )}
      </div>

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
                  Value {formData.threshold_type.includes('risk') || formData.threshold_type.includes('pnl') ? '(%)' : ''}
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
                onClick={() => setShowCreateForm(false)}
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
      <div>
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <LoadingSpinner className="w-8 h-8" />
            <span className="ml-2 text-gray-400">Loading alerts...</span>
          </div>
        ) : alerts.length === 0 ? (
          <div className="text-center py-12 text-gray-400">
            <InfoIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
            <p>No alerts configured</p>
            <p className="text-sm mt-2">Create your first alert to get notified about risk changes</p>
          </div>
        ) : (
          <div className="space-y-3">
            {alerts.map((alert) => (
              <div key={alert.id} className={`p-4 rounded-lg border ${getAlertColor(alert)}`}>
                <div className="flex items-center justify-between">
                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-2">
                      <span className="font-medium text-white">
                        {getThresholdLabel(alert.threshold_type)}
                      </span>
                      <span className="text-sm text-gray-400">
                        {getOperatorSymbol(alert.operator)} {alert.value}
                        {alert.threshold_type.includes('risk') || alert.threshold_type.includes('pnl') ? '%' : ''}
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
                      onClick={() => handleToggleAlert(alert)}
                      className={`px-3 py-1 text-xs rounded-lg transition-colors ${
                        alert.enabled 
                          ? 'bg-yellow-600 hover:bg-yellow-700 text-white' 
                          : 'bg-green-600 hover:bg-green-700 text-white'
                      }`}
                    >
                      {alert.enabled ? 'Disable' : 'Enable'}
                    </button>
                    
                    <button
                      onClick={() => handleDeleteAlert(alert.id)}
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

      {/* Quick Setup Suggestions */}
      {alerts.length === 0 && !showCreateForm && (
        <div className="mt-6 p-4 bg-blue-900/10 rounded-lg border border-blue-500/20">
          <h4 className="text-sm font-medium text-blue-400 mb-3">Suggested Alert Setup</h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
            <div className="flex items-center gap-2">
              <span className="text-blue-400">•</span>
              <span className="text-gray-300">Overall Risk {'>'}80% (Critical)</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-blue-400">•</span>
              <span className="text-gray-300">MEV Risk {'>'} 60% (High)</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-blue-400">•</span>
              <span className="text-gray-300">Impermanent Loss {'>'} 10%</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-blue-400">•</span>
              <span className="text-gray-300">P&L {'<'} -20% (Stop Loss)</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default AlertsPanel;
