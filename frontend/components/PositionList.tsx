/**
 * PositionList Component
 * 
 * Comprehensive list/grid view for DeFi positions with filtering,
 * sorting, search, and bulk operations
 */

import React, { useState, useMemo } from 'react';
import { Position, RiskMetrics } from '../lib/api-client';
import { toast } from 'react-hot-toast';
import { 
  SearchIcon,
  FilterIcon,
  SortIcon,
  GridIcon,
  ListIcon,
  EditIcon,
  TrashIcon,
  EyeIcon,
  AlertTriangleIcon,
  CheckIcon,
  XIcon,
  TrendingUpIcon,
  TrendingDownIcon,
  ExternalLinkIcon
} from './Icons';

interface PositionListProps {
  positions: Position[];
  riskMetrics: Record<string, RiskMetrics>;
  isLoading: boolean;
  onEdit: (position: Position) => void;
  onDelete: (positionId: string) => void;
  onView: (position: Position) => void;
  onBulkAction: (action: string, positionIds: string[]) => void;
  className?: string;
}

type ViewMode = 'list' | 'grid';
type SortField = 'created_at' | 'current_value_usd' | 'protocol' | 'risk_score' | 'pnl';
type SortOrder = 'asc' | 'desc';

interface FilterState {
  search: string;
  protocol: string;
  chain: string;
  status: string;
  riskLevel: string;
}

const CHAIN_NAMES = {
  1: 'Ethereum',
  56: 'BSC',
  137: 'Polygon',
  42161: 'Arbitrum',
  10: 'Optimism',
  43114: 'Avalanche'
};

const RISK_LEVELS = [
  { value: 'low', label: 'Low Risk (0-30%)', color: 'text-green-400' },
  { value: 'medium', label: 'Medium Risk (30-60%)', color: 'text-yellow-400' },
  { value: 'high', label: 'High Risk (60-80%)', color: 'text-orange-400' },
  { value: 'critical', label: 'Critical Risk (80%+)', color: 'text-red-400' }
];

const PositionList: React.FC<PositionListProps> = ({
  positions,
  riskMetrics,
  isLoading,
  onEdit,
  onDelete,
  onView,
  onBulkAction,
  className = ''
}) => {
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [sortField, setSortField] = useState<SortField>('created_at');
  const [sortOrder, setSortOrder] = useState<SortOrder>('desc');
  const [selectedPositions, setSelectedPositions] = useState<Set<string>>(new Set());
  const [showFilters, setShowFilters] = useState(false);
  const [filters, setFilters] = useState<FilterState>({
    search: '',
    protocol: '',
    chain: '',
    status: '',
    riskLevel: ''
  });

  // Get unique values for filter options
  const filterOptions = useMemo(() => {
    const protocols = Array.from(new Set(positions.map(p => p.protocol).filter(Boolean)));
    const chains = Array.from(new Set(positions.map(p => p.chain_id).filter(Boolean)));
    
    return { protocols, chains };
  }, [positions]);

  // Calculate risk score from risk metrics
  const getRiskScore = (positionId: string): number => {
    const metrics = riskMetrics[positionId];
    if (!metrics) return 0;
    
    const scores = [
      parseFloat(metrics.volatility_risk) || 0,
      parseFloat(metrics.liquidity_risk) || 0,
      parseFloat(metrics.protocol_risk) || 0,
      parseFloat(metrics.mev_risk) || 0,
      parseFloat(metrics.cross_chain_risk) || 0,
      parseFloat(metrics.impermanent_loss_risk) || 0
    ];
    
    return scores.reduce((sum, score) => sum + score, 0) / scores.length;
  };

  // Calculate P&L
  const getPnL = (position: Position): number => {
    const current = parseFloat(position.current_value_usd) || 0;
    const entry = parseFloat(position.entry_price_usd || position.current_value_usd) || current;
    return entry > 0 ? ((current - entry) / entry) * 100 : 0;
  };

  // Filter and sort positions
  const filteredAndSortedPositions = useMemo(() => {
    let filtered = positions.filter(position => {
      // Search filter
      if (filters.search) {
        const searchTerm = filters.search.toLowerCase();
        const matchesSearch = 
          position.token0_symbol?.toLowerCase().includes(searchTerm) ||
          position.token1_symbol?.toLowerCase().includes(searchTerm) ||
          position.protocol?.toLowerCase().includes(searchTerm) ||
          position.pool_address?.toLowerCase().includes(searchTerm);
        
        if (!matchesSearch) return false;
      }

      // Protocol filter
      if (filters.protocol && position.protocol !== filters.protocol) return false;

      // Chain filter
      if (filters.chain && position.chain_id?.toString() !== filters.chain) return false;

      // Status filter
      if (filters.status) {
        if (filters.status === 'active' && !position.is_active) return false;
        if (filters.status === 'inactive' && position.is_active) return false;
      }

      // Risk level filter
      if (filters.riskLevel) {
        const riskScore = getRiskScore(position.id);
        const level = filters.riskLevel;
        
        if (level === 'low' && riskScore > 30) return false;
        if (level === 'medium' && (riskScore <= 30 || riskScore > 60)) return false;
        if (level === 'high' && (riskScore <= 60 || riskScore > 80)) return false;
        if (level === 'critical' && riskScore <= 80) return false;
      }

      return true;
    });

    // Sort positions
    filtered.sort((a, b) => {
      let aValue: any, bValue: any;

      switch (sortField) {
        case 'current_value_usd':
          aValue = parseFloat(a.current_value_usd) || 0;
          bValue = parseFloat(b.current_value_usd) || 0;
          break;
        case 'protocol':
          aValue = a.protocol || '';
          bValue = b.protocol || '';
          break;
        case 'risk_score':
          aValue = getRiskScore(a.id);
          bValue = getRiskScore(b.id);
          break;
        case 'pnl':
          aValue = getPnL(a);
          bValue = getPnL(b);
          break;
        default:
          aValue = new Date(a.created_at || 0).getTime();
          bValue = new Date(b.created_at || 0).getTime();
      }

      if (sortOrder === 'asc') {
        return aValue > bValue ? 1 : -1;
      } else {
        return aValue < bValue ? 1 : -1;
      }
    });

    return filtered;
  }, [positions, filters, sortField, sortOrder, riskMetrics]);

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortField(field);
      setSortOrder('desc');
    }
  };

  const handleSelectPosition = (positionId: string) => {
    const newSelected = new Set(selectedPositions);
    if (newSelected.has(positionId)) {
      newSelected.delete(positionId);
    } else {
      newSelected.add(positionId);
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
    if (selectedPositions.size === 0) {
      toast.error('No positions selected');
      return;
    }

    onBulkAction(action, Array.from(selectedPositions));
    setSelectedPositions(new Set());
  };

  const getRiskLevelInfo = (riskScore: number) => {
    if (riskScore <= 30) return { level: 'Low', color: 'text-green-400', bg: 'bg-green-900/20' };
    if (riskScore <= 60) return { level: 'Medium', color: 'text-yellow-400', bg: 'bg-yellow-900/20' };
    if (riskScore <= 80) return { level: 'High', color: 'text-orange-400', bg: 'bg-orange-900/20' };
    return { level: 'Critical', color: 'text-red-400', bg: 'bg-red-900/20' };
  };

  const renderPositionCard = (position: Position) => {
    const riskScore = getRiskScore(position.id);
    const riskInfo = getRiskLevelInfo(riskScore);
    const pnl = getPnL(position);
    const isSelected = selectedPositions.has(position.id);

    return (
      <div
        key={position.id}
        className={`bg-gray-800/50 rounded-lg border transition-all ${
          isSelected ? 'border-blue-500 bg-blue-900/10' : 'border-gray-700 hover:border-gray-600'
        }`}
      >
        <div className="p-4">
          {/* Header */}
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-3">
              <input
                type="checkbox"
                checked={isSelected}
                onChange={() => handleSelectPosition(position.id)}
                className="rounded border-gray-600 bg-gray-700"
              />
              <div>
                <h3 className="font-medium text-white">
                  {position.token0_symbol}/{position.token1_symbol}
                </h3>
                <p className="text-sm text-gray-400">
                  {position.protocol} • {CHAIN_NAMES[position.chain_id as keyof typeof CHAIN_NAMES]}
                </p>
              </div>
            </div>
            
            <div className="flex items-center gap-2">
              <div className={`px-2 py-1 rounded text-xs ${riskInfo.bg} ${riskInfo.color}`}>
                {riskInfo.level} Risk
              </div>
              <div className={`flex items-center gap-1 text-sm ${
                position.is_active ? 'text-green-400' : 'text-gray-400'
              }`}>
                <div className={`w-2 h-2 rounded-full ${
                  position.is_active ? 'bg-green-500' : 'bg-gray-500'
                }`} />
                {position.is_active ? 'Active' : 'Inactive'}
              </div>
            </div>
          </div>

          {/* Metrics */}
          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <p className="text-xs text-gray-400">Current Value</p>
              <p className="text-lg font-semibold text-white">
                ${parseFloat(position.current_value_usd).toLocaleString()}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-400">P&L</p>
              <div className={`flex items-center gap-1 text-lg font-semibold ${
                pnl >= 0 ? 'text-green-400' : 'text-red-400'
              }`}>
                {pnl >= 0 ? <TrendingUpIcon className="w-4 h-4" /> : <TrendingDownIcon className="w-4 h-4" />}
                {pnl >= 0 ? '+' : ''}{pnl.toFixed(2)}%
              </div>
            </div>
          </div>

          {/* Risk Score */}
          <div className="mb-4">
            <div className="flex items-center justify-between text-xs text-gray-400 mb-1">
              <span>Risk Score</span>
              <span>{riskScore.toFixed(1)}%</span>
            </div>
            <div className="w-full bg-gray-700 rounded-full h-2">
              <div
                className={`h-2 rounded-full ${
                  riskScore <= 30 ? 'bg-green-500' :
                  riskScore <= 60 ? 'bg-yellow-500' :
                  riskScore <= 80 ? 'bg-orange-500' : 'bg-red-500'
                }`}
                style={{ width: `${Math.min(riskScore, 100)}%` }}
              />
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center justify-end gap-2">
            <button
              onClick={() => onView(position)}
              className="p-2 text-gray-400 hover:text-blue-400 transition-colors"
              title="View Details"
            >
              <EyeIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onEdit(position)}
              className="p-2 text-gray-400 hover:text-yellow-400 transition-colors"
              title="Edit Position"
            >
              <EditIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onDelete(position.id)}
              className="p-2 text-gray-400 hover:text-red-400 transition-colors"
              title="Delete Position"
            >
              <TrashIcon className="w-4 h-4" />
            </button>
            <a
              href={`https://etherscan.io/address/${position.pool_address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="p-2 text-gray-400 hover:text-blue-400 transition-colors"
              title="View on Explorer"
            >
              <ExternalLinkIcon className="w-4 h-4" />
            </a>
          </div>
        </div>
      </div>
    );
  };

  const renderPositionRow = (position: Position) => {
    const riskScore = getRiskScore(position.id);
    const riskInfo = getRiskLevelInfo(riskScore);
    const pnl = getPnL(position);
    const isSelected = selectedPositions.has(position.id);

    return (
      <tr
        key={position.id}
        className={`border-b border-gray-700 transition-colors ${
          isSelected ? 'bg-blue-900/10' : 'hover:bg-gray-800/30'
        }`}
      >
        <td className="p-4">
          <input
            type="checkbox"
            checked={isSelected}
            onChange={() => handleSelectPosition(position.id)}
            className="rounded border-gray-600 bg-gray-700"
          />
        </td>
        <td className="p-4">
          <div>
            <div className="font-medium text-white">
              {position.token0_symbol}/{position.token1_symbol}
            </div>
            <div className="text-sm text-gray-400">
              {position.protocol}
            </div>
          </div>
        </td>
        <td className="p-4 text-gray-300">
          {CHAIN_NAMES[position.chain_id as keyof typeof CHAIN_NAMES]}
        </td>
        <td className="p-4 text-white font-medium">
          ${parseFloat(position.current_value_usd).toLocaleString()}
        </td>
        <td className="p-4">
          <div className={`flex items-center gap-1 font-medium ${
            pnl >= 0 ? 'text-green-400' : 'text-red-400'
          }`}>
            {pnl >= 0 ? <TrendingUpIcon className="w-4 h-4" /> : <TrendingDownIcon className="w-4 h-4" />}
            {pnl >= 0 ? '+' : ''}{pnl.toFixed(2)}%
          </div>
        </td>
        <td className="p-4">
          <div className={`px-2 py-1 rounded text-xs ${riskInfo.bg} ${riskInfo.color}`}>
            {riskInfo.level} ({riskScore.toFixed(1)}%)
          </div>
        </td>
        <td className="p-4">
          <div className={`flex items-center gap-1 text-sm ${
            position.is_active ? 'text-green-400' : 'text-gray-400'
          }`}>
            <div className={`w-2 h-2 rounded-full ${
              position.is_active ? 'bg-green-500' : 'bg-gray-500'
            }`} />
            {position.is_active ? 'Active' : 'Inactive'}
          </div>
        </td>
        <td className="p-4">
          <div className="flex items-center gap-2">
            <button
              onClick={() => onView(position)}
              className="p-1 text-gray-400 hover:text-blue-400 transition-colors"
              title="View Details"
            >
              <EyeIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onEdit(position)}
              className="p-1 text-gray-400 hover:text-yellow-400 transition-colors"
              title="Edit Position"
            >
              <EditIcon className="w-4 h-4" />
            </button>
            <button
              onClick={() => onDelete(position.id)}
              className="p-1 text-gray-400 hover:text-red-400 transition-colors"
              title="Delete Position"
            >
              <TrashIcon className="w-4 h-4" />
            </button>
            <a
              href={`https://etherscan.io/address/${position.pool_address}`}
              target="_blank"
              rel="noopener noreferrer"
              className="p-1 text-gray-400 hover:text-blue-400 transition-colors"
              title="View on Explorer"
            >
              <ExternalLinkIcon className="w-4 h-4" />
            </a>
          </div>
        </td>
      </tr>
    );
  };

  return (
    <div className={`bg-gray-800/50 rounded-xl border border-gray-700 ${className}`}>
      {/* Header */}
      <div className="p-6 border-b border-gray-700">
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <h2 className="text-xl font-semibold text-white">Positions</h2>
            <p className="text-sm text-gray-400 mt-1">
              {filteredAndSortedPositions.length} of {positions.length} positions
              {selectedPositions.size > 0 && ` • ${selectedPositions.size} selected`}
            </p>
          </div>

          <div className="flex items-center gap-3">
            {/* Search */}
            <div className="relative">
              <SearchIcon className="absolute left-3 top-1/2 transform -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input
                type="text"
                placeholder="Search positions..."
                value={filters.search}
                onChange={(e) => setFilters(prev => ({ ...prev, search: e.target.value }))}
                className="pl-10 pr-4 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white text-sm w-64"
              />
            </div>

            {/* Filters Toggle */}
            <button
              onClick={() => setShowFilters(!showFilters)}
              className={`p-2 rounded-lg border transition-colors ${
                showFilters ? 'border-blue-500 bg-blue-900/20 text-blue-400' : 'border-gray-600 text-gray-400 hover:text-white'
              }`}
            >
              <FilterIcon className="w-4 h-4" />
            </button>

            {/* View Mode Toggle */}
            <div className="flex border border-gray-600 rounded-lg overflow-hidden">
              <button
                onClick={() => setViewMode('list')}
                className={`p-2 transition-colors ${
                  viewMode === 'list' ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
                }`}
              >
                <ListIcon className="w-4 h-4" />
              </button>
              <button
                onClick={() => setViewMode('grid')}
                className={`p-2 transition-colors ${
                  viewMode === 'grid' ? 'bg-blue-600 text-white' : 'text-gray-400 hover:text-white'
                }`}
              >
                <GridIcon className="w-4 h-4" />
              </button>
            </div>
          </div>
        </div>

        {/* Filters */}
        {showFilters && (
          <div className="mt-4 p-4 bg-gray-900/30 rounded-lg border border-gray-600">
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
              <div>
                <label className="block text-sm text-gray-400 mb-1">Protocol</label>
                <select
                  value={filters.protocol}
                  onChange={(e) => setFilters(prev => ({ ...prev, protocol: e.target.value }))}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                >
                  <option value="">All Protocols</option>
                  {filterOptions.protocols.map(protocol => (
                    <option key={protocol} value={protocol}>{protocol}</option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">Chain</label>
                <select
                  value={filters.chain}
                  onChange={(e) => setFilters(prev => ({ ...prev, chain: e.target.value }))}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                >
                  <option value="">All Chains</option>
                  {filterOptions.chains.map(chainId => (
                    <option key={chainId} value={chainId?.toString() || ''}>
                      {chainId ? CHAIN_NAMES[chainId as keyof typeof CHAIN_NAMES] || `Chain ${chainId}` : 'Unknown Chain'}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">Status</label>
                <select
                  value={filters.status}
                  onChange={(e) => setFilters(prev => ({ ...prev, status: e.target.value }))}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                >
                  <option value="">All Status</option>
                  <option value="active">Active</option>
                  <option value="inactive">Inactive</option>
                </select>
              </div>

              <div>
                <label className="block text-sm text-gray-400 mb-1">Risk Level</label>
                <select
                  value={filters.riskLevel}
                  onChange={(e) => setFilters(prev => ({ ...prev, riskLevel: e.target.value }))}
                  className="w-full bg-gray-700 border border-gray-600 rounded px-3 py-2 text-white text-sm"
                >
                  <option value="">All Risk Levels</option>
                  {RISK_LEVELS.map(level => (
                    <option key={level.value} value={level.value}>{level.label}</option>
                  ))}
                </select>
              </div>
            </div>

            <div className="flex items-center justify-between mt-4">
              <button
                onClick={() => setFilters({
                  search: '',
                  protocol: '',
                  chain: '',
                  status: '',
                  riskLevel: ''
                })}
                className="text-sm text-gray-400 hover:text-white transition-colors"
              >
                Clear Filters
              </button>
            </div>
          </div>
        )}

        {/* Bulk Actions */}
        {selectedPositions.size > 0 && (
          <div className="mt-4 p-3 bg-blue-900/20 border border-blue-500/30 rounded-lg">
            <div className="flex items-center justify-between">
              <span className="text-sm text-blue-400">
                {selectedPositions.size} position{selectedPositions.size > 1 ? 's' : ''} selected
              </span>
              <div className="flex items-center gap-2">
                <button
                  onClick={() => handleBulkAction('activate')}
                  className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-sm rounded transition-colors"
                >
                  Activate
                </button>
                <button
                  onClick={() => handleBulkAction('deactivate')}
                  className="px-3 py-1 bg-yellow-600 hover:bg-yellow-700 text-white text-sm rounded transition-colors"
                >
                  Deactivate
                </button>
                <button
                  onClick={() => handleBulkAction('delete')}
                  className="px-3 py-1 bg-red-600 hover:bg-red-700 text-white text-sm rounded transition-colors"
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Content */}
      <div className="p-6">
        {isLoading ? (
          <div className="flex items-center justify-center py-12">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-500"></div>
            <span className="ml-3 text-gray-400">Loading positions...</span>
          </div>
        ) : filteredAndSortedPositions.length === 0 ? (
          <div className="text-center py-12">
            <AlertTriangleIcon className="w-12 h-12 mx-auto text-gray-400 mb-4" />
            <h3 className="text-lg font-medium text-white mb-2">No positions found</h3>
            <p className="text-gray-400">
              {positions.length === 0 
                ? 'Create your first position to start monitoring'
                : 'Try adjusting your filters to see more results'
              }
            </p>
          </div>
        ) : viewMode === 'grid' ? (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {filteredAndSortedPositions.map(renderPositionCard)}
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-gray-700">
                  <th className="text-left p-4">
                    <input
                      type="checkbox"
                      checked={selectedPositions.size === filteredAndSortedPositions.length && filteredAndSortedPositions.length > 0}
                      onChange={handleSelectAll}
                      className="rounded border-gray-600 bg-gray-700"
                    />
                  </th>
                  <th className="text-left p-4">
                    <button
                      onClick={() => handleSort('protocol')}
                      className="flex items-center gap-1 text-gray-300 hover:text-white transition-colors"
                    >
                      Position
                      <SortIcon className="w-4 h-4" />
                    </button>
                  </th>
                  <th className="text-left p-4 text-gray-300">Chain</th>
                  <th className="text-left p-4">
                    <button
                      onClick={() => handleSort('current_value_usd')}
                      className="flex items-center gap-1 text-gray-300 hover:text-white transition-colors"
                    >
                      Value
                      <SortIcon className="w-4 h-4" />
                    </button>
                  </th>
                  <th className="text-left p-4">
                    <button
                      onClick={() => handleSort('pnl')}
                      className="flex items-center gap-1 text-gray-300 hover:text-white transition-colors"
                    >
                      P&L
                      <SortIcon className="w-4 h-4" />
                    </button>
                  </th>
                  <th className="text-left p-4">
                    <button
                      onClick={() => handleSort('risk_score')}
                      className="flex items-center gap-1 text-gray-300 hover:text-white transition-colors"
                    >
                      Risk
                      <SortIcon className="w-4 h-4" />
                    </button>
                  </th>
                  <th className="text-left p-4 text-gray-300">Status</th>
                  <th className="text-left p-4 text-gray-300">Actions</th>
                </tr>
              </thead>
              <tbody>
                {filteredAndSortedPositions.map(renderPositionRow)}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};

export default PositionList;
