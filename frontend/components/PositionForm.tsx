/**
 * PositionForm Component
 * 
 * Comprehensive form for creating and editing DeFi positions
 * with validation, protocol selection, and risk assessment
 */

import React, { useState, useEffect } from 'react';
import { Position } from '../lib/api-client';
import { toast } from 'react-hot-toast';
import { 
  SaveIcon, 
  XIcon, 
  AlertTriangleIcon, 
  InfoIcon,
  PlusIcon,
  MinusIcon,
  ExternalLinkIcon
} from './Icons';

interface PositionFormProps {
  position?: Position | null;
  isOpen: boolean;
  onClose: () => void;
  onSave: (position: Partial<Position>) => Promise<void>;
  className?: string;
}

interface FormData {
  protocol: string;
  pool_address: string;
  token0_symbol: string;
  token1_symbol: string;
  token0_address: string;
  token1_address: string;
  liquidity_amount: string;
  current_value_usd: string;
  entry_price_usd: string;
  fee_tier: string;
  position_range_lower: string;
  position_range_upper: string;
  chain_id: number;
  is_active: boolean;
  notes: string;
}

const SUPPORTED_PROTOCOLS = [
  { id: 'uniswap-v3', name: 'Uniswap V3', chains: [1, 137, 42161, 10] },
  { id: 'pancakeswap-v3', name: 'PancakeSwap V3', chains: [56, 1] },
  { id: 'sushiswap', name: 'SushiSwap', chains: [1, 137, 42161] },
  { id: 'curve', name: 'Curve Finance', chains: [1, 137, 42161] },
  { id: 'balancer', name: 'Balancer', chains: [1, 137, 42161] },
  { id: 'aave', name: 'Aave', chains: [1, 137, 42161, 43114] }
];

const CHAIN_NAMES = {
  1: 'Ethereum',
  56: 'BSC',
  137: 'Polygon',
  42161: 'Arbitrum',
  10: 'Optimism',
  43114: 'Avalanche'
};

const FEE_TIERS = [
  { value: '0.01', label: '0.01% (Stablecoin pairs)' },
  { value: '0.05', label: '0.05% (Low volatility)' },
  { value: '0.30', label: '0.30% (Standard)' },
  { value: '1.00', label: '1.00% (High volatility)' }
];

const PositionForm: React.FC<PositionFormProps> = ({
  position,
  isOpen,
  onClose,
  onSave,
  className = ''
}) => {
  const [formData, setFormData] = useState<FormData>({
    protocol: '',
    pool_address: '',
    token0_symbol: '',
    token1_symbol: '',
    token0_address: '',
    token1_address: '',
    liquidity_amount: '',
    current_value_usd: '',
    entry_price_usd: '',
    fee_tier: '0.30',
    position_range_lower: '',
    position_range_upper: '',
    chain_id: 1,
    is_active: true,
    notes: ''
  });

  const [isLoading, setIsLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [selectedProtocol, setSelectedProtocol] = useState<typeof SUPPORTED_PROTOCOLS[0] | null>(null);

  const isEditing = !!position;

  // Initialize form data when position changes
  useEffect(() => {
    if (position) {
      setFormData({
        protocol: position.protocol || '',
        pool_address: position.pool_address || '',
        token0_symbol: position.token0_symbol || '',
        token1_symbol: position.token1_symbol || '',
        token0_address: position.token0_address || '',
        token1_address: position.token1_address || '',
        liquidity_amount: position.liquidity_amount || '',
        current_value_usd: position.current_value_usd || '',
        entry_price_usd: position.entry_price_usd || '',
        fee_tier: position.fee_tier?.toString() || '0.30',
        position_range_lower: position.position_range_lower || '',
        position_range_upper: position.position_range_upper || '',
        chain_id: position.chain_id || 1,
        is_active: position.is_active !== false,
        notes: position.notes || ''
      });

      const protocol = SUPPORTED_PROTOCOLS.find(p => p.id === position.protocol);
      setSelectedProtocol(protocol || null);
    } else {
      // Reset form for new position
      setFormData({
        protocol: '',
        pool_address: '',
        token0_symbol: '',
        token1_symbol: '',
        token0_address: '',
        token1_address: '',
        liquidity_amount: '',
        current_value_usd: '',
        entry_price_usd: '',
        fee_tier: '0.30',
        position_range_lower: '',
        position_range_upper: '',
        chain_id: 1,
        is_active: true,
        notes: ''
      });
      setSelectedProtocol(null);
    }
    setErrors({});
  }, [position]);

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.protocol) newErrors.protocol = 'Protocol is required';
    if (!formData.pool_address) newErrors.pool_address = 'Pool address is required';
    if (!formData.token0_symbol) newErrors.token0_symbol = 'Token 0 symbol is required';
    if (!formData.token1_symbol) newErrors.token1_symbol = 'Token 1 symbol is required';
    if (!formData.token0_address) newErrors.token0_address = 'Token 0 address is required';
    if (!formData.token1_address) newErrors.token1_address = 'Token 1 address is required';
    if (!formData.liquidity_amount) newErrors.liquidity_amount = 'Liquidity amount is required';
    if (!formData.current_value_usd) newErrors.current_value_usd = 'Current value is required';

    // Validate addresses (basic format check)
    const addressRegex = /^0x[a-fA-F0-9]{40}$/;
    if (formData.pool_address && !addressRegex.test(formData.pool_address)) {
      newErrors.pool_address = 'Invalid pool address format';
    }
    if (formData.token0_address && !addressRegex.test(formData.token0_address)) {
      newErrors.token0_address = 'Invalid token address format';
    }
    if (formData.token1_address && !addressRegex.test(formData.token1_address)) {
      newErrors.token1_address = 'Invalid token address format';
    }

    // Validate numeric fields
    if (formData.liquidity_amount && isNaN(parseFloat(formData.liquidity_amount))) {
      newErrors.liquidity_amount = 'Must be a valid number';
    }
    if (formData.current_value_usd && isNaN(parseFloat(formData.current_value_usd))) {
      newErrors.current_value_usd = 'Must be a valid number';
    }
    if (formData.entry_price_usd && formData.entry_price_usd && isNaN(parseFloat(formData.entry_price_usd))) {
      newErrors.entry_price_usd = 'Must be a valid number';
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!validateForm()) {
      toast.error('Please fix the form errors');
      return;
    }

    setIsLoading(true);
    try {
      // Convert FormData to Position format
      const positionData: Partial<Position> = {
        ...formData,
        fee_tier: parseFloat(formData.fee_tier),
        chain_id: formData.chain_id,
        is_active: formData.is_active
      };
      
      await onSave(positionData);
      toast.success(isEditing ? 'Position updated successfully' : 'Position created successfully');
      onClose();
    } catch (error) {
      console.error('Error saving position:', error);
      toast.error('Failed to save position');
    } finally {
      setIsLoading(false);
    }
  };

  const handleProtocolChange = (protocolId: string) => {
    const protocol = SUPPORTED_PROTOCOLS.find(p => p.id === protocolId);
    setSelectedProtocol(protocol || null);
    setFormData(prev => ({
      ...prev,
      protocol: protocolId,
      chain_id: protocol?.chains[0] || 1
    }));
  };

  const handleInputChange = (field: keyof FormData, value: string | number | boolean) => {
    setFormData(prev => ({ ...prev, [field]: value }));
    if (errors[field]) {
      setErrors(prev => ({ ...prev, [field]: '' }));
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className={`bg-gray-800 rounded-xl border border-gray-700 w-full max-w-4xl max-h-[90vh] overflow-y-auto ${className}`}>
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-gray-700">
          <div>
            <h2 className="text-xl font-semibold text-white">
              {isEditing ? 'Edit Position' : 'Create New Position'}
            </h2>
            <p className="text-sm text-gray-400 mt-1">
              {isEditing ? 'Update position details and risk parameters' : 'Add a new DeFi position for monitoring'}
            </p>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-700 rounded-lg transition-colors"
          >
            <XIcon className="w-5 h-5 text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-6">
          {/* Protocol & Chain Selection */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Protocol *
              </label>
              <select
                value={formData.protocol}
                onChange={(e) => handleProtocolChange(e.target.value)}
                className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                  errors.protocol ? 'border-red-500' : 'border-gray-600'
                }`}
              >
                <option value="">Select Protocol</option>
                {SUPPORTED_PROTOCOLS.map(protocol => (
                  <option key={protocol.id} value={protocol.id}>
                    {protocol.name}
                  </option>
                ))}
              </select>
              {errors.protocol && (
                <p className="text-red-400 text-xs mt-1">{errors.protocol}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Chain *
              </label>
              <select
                value={formData.chain_id}
                onChange={(e) => handleInputChange('chain_id', parseInt(e.target.value))}
                className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                disabled={!selectedProtocol}
              >
                {selectedProtocol ? (
                  selectedProtocol.chains.map(chainId => (
                    <option key={chainId} value={chainId}>
                      {CHAIN_NAMES[chainId as keyof typeof CHAIN_NAMES] || `Chain ${chainId}`}
                    </option>
                  ))
                ) : (
                  <option value="">Select protocol first</option>
                )}
              </select>
            </div>
          </div>

          {/* Pool Information */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-white border-b border-gray-700 pb-2">
              Pool Information
            </h3>
            
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Pool Address *
              </label>
              <input
                type="text"
                value={formData.pool_address}
                onChange={(e) => handleInputChange('pool_address', e.target.value)}
                placeholder="0x..."
                className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                  errors.pool_address ? 'border-red-500' : 'border-gray-600'
                }`}
              />
              {errors.pool_address && (
                <p className="text-red-400 text-xs mt-1">{errors.pool_address}</p>
              )}
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Token 0 Symbol *
                </label>
                <input
                  type="text"
                  value={formData.token0_symbol}
                  onChange={(e) => handleInputChange('token0_symbol', e.target.value)}
                  placeholder="USDC"
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.token0_symbol ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.token0_symbol && (
                  <p className="text-red-400 text-xs mt-1">{errors.token0_symbol}</p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Token 1 Symbol *
                </label>
                <input
                  type="text"
                  value={formData.token1_symbol}
                  onChange={(e) => handleInputChange('token1_symbol', e.target.value)}
                  placeholder="ETH"
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.token1_symbol ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.token1_symbol && (
                  <p className="text-red-400 text-xs mt-1">{errors.token1_symbol}</p>
                )}
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Token 0 Address *
                </label>
                <input
                  type="text"
                  value={formData.token0_address}
                  onChange={(e) => handleInputChange('token0_address', e.target.value)}
                  placeholder="0x..."
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.token0_address ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.token0_address && (
                  <p className="text-red-400 text-xs mt-1">{errors.token0_address}</p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Token 1 Address *
                </label>
                <input
                  type="text"
                  value={formData.token1_address}
                  onChange={(e) => handleInputChange('token1_address', e.target.value)}
                  placeholder="0x..."
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.token1_address ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.token1_address && (
                  <p className="text-red-400 text-xs mt-1">{errors.token1_address}</p>
                )}
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Fee Tier
              </label>
              <select
                value={formData.fee_tier}
                onChange={(e) => handleInputChange('fee_tier', e.target.value)}
                className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
              >
                {FEE_TIERS.map(tier => (
                  <option key={tier.value} value={tier.value}>
                    {tier.label}
                  </option>
                ))}
              </select>
            </div>
          </div>

          {/* Position Details */}
          <div className="space-y-4">
            <h3 className="text-lg font-medium text-white border-b border-gray-700 pb-2">
              Position Details
            </h3>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Liquidity Amount *
                </label>
                <input
                  type="text"
                  value={formData.liquidity_amount}
                  onChange={(e) => handleInputChange('liquidity_amount', e.target.value)}
                  placeholder="1000000"
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.liquidity_amount ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.liquidity_amount && (
                  <p className="text-red-400 text-xs mt-1">{errors.liquidity_amount}</p>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Current Value (USD) *
                </label>
                <input
                  type="text"
                  value={formData.current_value_usd}
                  onChange={(e) => handleInputChange('current_value_usd', e.target.value)}
                  placeholder="5000.00"
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.current_value_usd ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.current_value_usd && (
                  <p className="text-red-400 text-xs mt-1">{errors.current_value_usd}</p>
                )}
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Entry Price (USD)
                </label>
                <input
                  type="text"
                  value={formData.entry_price_usd}
                  onChange={(e) => handleInputChange('entry_price_usd', e.target.value)}
                  placeholder="4800.00"
                  className={`w-full bg-gray-700 border rounded-lg px-3 py-2 text-white ${
                    errors.entry_price_usd ? 'border-red-500' : 'border-gray-600'
                  }`}
                />
                {errors.entry_price_usd && (
                  <p className="text-red-400 text-xs mt-1">{errors.entry_price_usd}</p>
                )}
              </div>

              <div className="flex items-center">
                <label className="flex items-center gap-2 text-sm text-gray-300">
                  <input
                    type="checkbox"
                    checked={formData.is_active}
                    onChange={(e) => handleInputChange('is_active', e.target.checked)}
                    className="rounded border-gray-600 bg-gray-700"
                  />
                  Position is active
                </label>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Range Lower
                </label>
                <input
                  type="text"
                  value={formData.position_range_lower}
                  onChange={(e) => handleInputChange('position_range_lower', e.target.value)}
                  placeholder="1800"
                  className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Range Upper
                </label>
                <input
                  type="text"
                  value={formData.position_range_upper}
                  onChange={(e) => handleInputChange('position_range_upper', e.target.value)}
                  placeholder="2200"
                  className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white"
                />
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Notes
              </label>
              <textarea
                value={formData.notes}
                onChange={(e) => handleInputChange('notes', e.target.value)}
                placeholder="Additional notes about this position..."
                rows={3}
                className="w-full bg-gray-700 border border-gray-600 rounded-lg px-3 py-2 text-white resize-none"
              />
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-end gap-3 pt-6 border-t border-gray-700">
            <button
              type="button"
              onClick={onClose}
              className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={isLoading}
              className="flex items-center gap-2 px-6 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800 text-white rounded-lg transition-colors"
            >
              {isLoading ? (
                <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white"></div>
              ) : (
                <SaveIcon className="w-4 h-4" />
              )}
              {isLoading ? 'Saving...' : (isEditing ? 'Update Position' : 'Create Position')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export default PositionForm;
