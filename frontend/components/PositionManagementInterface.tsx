/**
 * PositionManagementInterface Component
 * 
 * Advanced position management interface with filtering, sorting,
 * bulk operations, and detailed analytics
 */

import React, { useState, useMemo } from 'react';
import { Position, RiskMetrics } from '../lib/api-client';
import { LoadingSpinner, SearchIcon, SettingsIcon } from './Icons';
import PositionCard from './PositionCard';

interface PositionManagementInterfaceProps {
  positions: Position[];
  riskMetrics: Record<string, RiskMetrics> | null;
  isLoading?: boolean;
  onViewDetails?: (positionId: string) => void;
  onCalculateRisk?: (positionId: string) => void;
  onExplainRisk?: (positionId: string) => void;
  onBulkAction?: (action: string, positionIds: string[]) => void;
  className?: string;
}

type SortField = 'created_at' | 'current_value_usd' | 'risk_score' | 'pnl' | 'impermanent_loss';
type SortDirection = 'asc' | 'desc';
type ViewMode = 'grid' | 'list';

const PositionManagementInterface: React.FC<PositionManagementInterfaceProps> = ({
  positions,
  riskMetrics,
  isLoading = false,
  onViewDetails,
  onCalculateRisk,
  onExplainRisk,
  onBulkAction,
  className = ''
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedPositions, setSelectedPositions] = useState<Set<string>>(new Set());
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortDirection, setSortDirection] = useState<SortDirection>('desc');
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [filters, setFilters] = useState({
    minValue: '',
    maxValue: '',
    riskLevel: 'all', // all, low, medium, high, critical
    protocol: 'all',
    chain: 'all'
  });

  // Filter and sort positions
  const filteredAndSortedPositions = useMemo(() => {
    let filtered = positions.filter(position => {
      // Search filter
      const searchMatch = searchTerm === '' || 
        position.token0_symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
        position.token1_symbol.toLowerCase().includes(searchTerm.toLowerCase()) ||
        position.id.toLowerCase().includes(searchTerm.toLowerCase());

      if (!searchMatch) return false;

      // Value filters
      const currentValue = parseFloat(position.current_value_usd);
      if (filters.minValue && currentValue < parseFloat(filters.minValue)) return false;
      if (filters.maxValue && currentValue > parseFloat(filters.maxValue)) return false;

      // Risk level filter
      if (filters.riskLevel !== 'all' && riskMetrics) {
        const positionRisk = riskMetrics[position.id];
        if (positionRisk) {
          const riskScore = parseFloat(positionRisk.overall_risk_score);
          const riskLevel = riskScore >= 80 ? 'critical' : 
                           riskScore >= 60 ? 'high' : 
                           riskScore >= 30 ? 'medium' : 'low';
          if (riskLevel !== filters.riskLevel) return false;
        }
      }

      return true;
    });

    // Sort positions
    filtered.sort((a, b) => {
      let aValue: any, bValue: any;

      switch (sortField) {
        case 'created_at':
          aValue = new Date(a.created_at).getTime();
          bValue = new Date(b.created_at).getTime();
          break;
        case 'current_value_usd':
          aValue = parseFloat(a.current_value_usd);
          bValue = parseFloat(b.current_value_usd);
          break;
        case 'risk_score':
          aValue = riskMetrics?.[a.id] ? parseFloat(riskMetrics[a.id].overall_risk_score) : 0;
          bValue = riskMetrics?.[b.id] ? parseFloat(riskMetrics[b.id].overall_risk_score) : 0;
          break;
        case 'pnl':
          const aPnl = parseFloat(a.current_value_usd) - parseFloat(a.liquidity_amount);
          const bPnl = parseFloat(b.current_value_usd) - parseFloat(b.liquidity_amount);
          aValue = aPnl;
          bValue = bPnl;
          break;
        case 'impermanent_loss':
          aValue = parseFloat(a.impermanent_loss_pct);
          bValue = parseFloat(b.impermanent_loss_pct);
          break;
        default:
          return 0;
      }

      if (sortDirection === 'asc') {
        return aValue > bValue ? 1 : -1;
      } else {
        return aValue < bValue ? 1 : -1;
      }
    });

    return filtered;
  }, [positions, searchTerm, filters, sortField, sortDirection, riskMetrics]);

  const handleSelectPosition = (positionId: string, selected: boolean) => {
    const newSelected = new Set(selectedPositions);
    if (selected) {
      newSelected.add(positionId);
    } else {
      newSelected.delete(positionId);
    }
    setSelectedPositions(newSelected);
  };

  const handleSelectAll = () => {
    if (selectedPositions.size === filteredAndSortedPositions.length) {
      setSelectedPositions(new Set());
    } else {
      setSelectedPositions(new Set(filteredAndSortedPositions.map(p => p.id)));
    }
  };

  const handleBulkAction = (action: string) => {
    if (onBulkAction && selectedPositions.size > 0) {
      onBulkAction(action, Array.from(selectedPositions));
      setSelectedPositions(new Set());
    }
  };

  const getRiskLevelStats = () => {
    if (!riskMetrics) return { low: 0, medium: 0, high: 0, critical: 0 };
    
    return filteredAndSortedPositions.reduce((stats, position) => {
      const risk = riskMetrics[position.id];
      if (risk) {
        const score = parseFloat(risk.overall_risk_score);
        if (score >= 80) stats.critical++;
        else if (score >= 60) stats.high++;
        else if (score >= 30) stats.medium++;
        else stats.low++;
      }
      return stats;
    }, { low: 0, medium: 0, high: 0, critical: 0 });
  };

  const riskStats = getRiskLevelStats();

  return (
    <div className={`bg-gray-800/50 rounded-xl p-6 border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-lg font-semibold text-white">Position Management</h3>
          <p className="text-sm text-gray-400 mt-1">
            {filteredAndSortedPositions.length} of {positions.length} positions
            {selectedPositions.size > 0 && ` • ${selectedPositions.size} selected`}
          </p>
        </div>
        
        <div className="flex items-center gap-2">
          {/* View Mode Toggle */}
          <div className="flex bg-gray-700 rounded-lg p-1">
            <button
              onClick={() => setViewMode('grid')}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                viewMode === 'grid' ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
              }`}
            >
              Grid
            </button>
            <button
              onClick={() => setViewMode('list')}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                viewMode === 'list' ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
              }`}
            >
              List
            </button>
          </div>
        </div>
      </div>

      {/* Risk Level Statistics */}
      <div className="grid grid-cols-4 gap-4 mb-6">
        <div className="bg-green-900/20 border border-green-500/30 rounded-lg p-3 text-center">
          <div className="text-lg font-bold text-green-400">{riskStats.low}</div>
          <div className="text-xs text-green-400">Low Risk</div>
        </div>
        <div className="bg-yellow-900/20 border border-yellow-500/30 rounded-lg p-3 text-center">
          <div className="text-lg font-bold text-yellow-400">{riskStats.medium}</div>
          <div className="text-xs text-yellow-400">Medium Risk</div>
        </div>
        <div className="bg-orange-900/20 border border-orange-500/30 rounded-lg p-3 text-center">
          <div className="text-lg font-bold text-orange-400">{riskStats.high}</div>
          <div className="text-xs text-orange-400">High Risk</div>
        </div>
        <div className="bg-red-900/20 border border-red-500/30 rounded-lg p-3 text-center">
          <div className="text-lg font-bold text-red-400">{riskStats.critical}</div>
          <div className="text-xs text-red-400">Critical Risk</div>
        </div>
      </div>

      {/* Search and Filters */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        {/* Search */}
        <div className="relative">
          <SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-gray-400" />
          <input
            type="text"
            placeholder="Search positions..."
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            className="w-full pl-10 pr-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
          />
        </div>

        {/* Risk Level Filter */}
        <select
          value={filters.riskLevel}
          onChange={(e) => setFilters(prev => ({ ...prev, riskLevel: e.target.value }))}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
        >
          <option value="all">All Risk Levels</option>
          <option value="low">Low Risk</option>
          <option value="medium">Medium Risk</option>
          <option value="high">High Risk</option>
          <option value="critical">Critical Risk</option>
        </select>

        {/* Value Range */}
        <input
          type="number"
          placeholder="Min Value ($)"
          value={filters.minValue}
          onChange={(e) => setFilters(prev => ({ ...prev, minValue: e.target.value }))}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
        />

        <input
          type="number"
          placeholder="Max Value ($)"
          value={filters.maxValue}
          onChange={(e) => setFilters(prev => ({ ...prev, maxValue: e.target.value }))}
          className="px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
        />
      </div>

      {/* Sort and Bulk Actions */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-4">
          {/* Select All */}
          <label className="flex items-center gap-2">
            <input
              type="checkbox"
              checked={selectedPositions.size === filteredAndSortedPositions.length && filteredAndSortedPositions.length > 0}
              onChange={handleSelectAll}
              className="rounded border-gray-600 bg-gray-700 text-blue-600"
            />
            <span className="text-sm text-gray-300">Select All</span>
          </label>

          {/* Sort */}
          <select
            value={`${sortField}-${sortDirection}`}
            onChange={(e) => {
              const [field, direction] = e.target.value.split('-');
              setSortField(field as SortField);
              setSortDirection(direction as SortDirection);
            }}
            className="px-3 py-1 bg-gray-700 border border-gray-600 rounded text-sm text-white focus:outline-none focus:border-blue-500"
          >
            <option value="created_at-desc">Newest First</option>
            <option value="created_at-asc">Oldest First</option>
            <option value="current_value_usd-desc">Highest Value</option>
            <option value="current_value_usd-asc">Lowest Value</option>
            <option value="risk_score-desc">Highest Risk</option>
            <option value="risk_score-asc">Lowest Risk</option>
            <option value="pnl-desc">Best Performance</option>
            <option value="pnl-asc">Worst Performance</option>
          </select>
        </div>

        {/* Bulk Actions */}
        {selectedPositions.size > 0 && (
          <div className="flex items-center gap-2">
            <button
              onClick={() => handleBulkAction('calculate_risk')}
              className="px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white text-sm rounded transition-colors"
            >
              Update Risk ({selectedPositions.size})
            </button>
            <button
              onClick={() => handleBulkAction('export')}
              className="px-3 py-1 bg-gray-600 hover:bg-gray-700 text-white text-sm rounded transition-colors"
            >
              Export
            </button>
          </div>
        )}
      </div>

      {/* Positions Display */}
      {isLoading ? (
        <div className="flex items-center justify-center py-12">
          <LoadingSpinner className="w-8 h-8" />
          <span className="ml-2 text-gray-400">Loading positions...</span>
        </div>
      ) : filteredAndSortedPositions.length === 0 ? (
        <div className="text-center py-12 text-gray-400">
          <SettingsIcon className="w-12 h-12 mx-auto mb-4 opacity-50" />
          <p>No positions match your filters</p>
          <p className="text-sm mt-2">Try adjusting your search criteria</p>
        </div>
      ) : (
        <div className={viewMode === 'grid' ? 'grid gap-6' : 'space-y-4'}>
          {filteredAndSortedPositions.map((position) => (
            <div key={position.id} className="relative">
              {/* Selection Checkbox */}
              <div className="absolute top-4 left-4 z-10">
                <input
                  type="checkbox"
                  checked={selectedPositions.has(position.id)}
                  onChange={(e) => handleSelectPosition(position.id, e.target.checked)}
                  className="rounded border-gray-600 bg-gray-700 text-blue-600"
                />
              </div>
              
              <PositionCard
                position={position}
                riskMetrics={riskMetrics ? riskMetrics[position.id] : undefined}
                onViewDetails={onViewDetails}
                onCalculateRisk={onCalculateRisk}
                onExplainRisk={onExplainRisk}
                className="ml-8" // Account for checkbox space
              />
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

export default PositionManagementInterface;
