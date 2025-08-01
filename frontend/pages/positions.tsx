/**
 * Positions Management Page
 * 
 * Comprehensive page for managing DeFi positions with CRUD operations,
 * bulk actions, filtering, and detailed views
 */

import React, { useState, useEffect } from 'react';
import { useAccount } from 'wagmi';
import { useRiskMonitoring } from '../hooks/useRiskMonitoring';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import { toast } from 'react-hot-toast';
import { Position, RiskMetrics } from '../lib/api-client';

// Import position management components
import PositionForm from '../components/PositionForm';
import PositionList from '../components/PositionList';
import PositionDetails from '../components/PositionDetails';

// Import icons
import { 
  PlusIcon, 
  LoadingSpinner, 
  AlertTriangleIcon,
  PieChartIcon,
  BrainIcon,
  RefreshIcon
} from '../components/Icons';

type ViewMode = 'list' | 'details' | 'form';

interface BulkActionConfirmation {
  action: string;
  positionIds: string[];
  isOpen: boolean;
}

const PositionsPage: React.FC = () => {
  const { address, isConnected } = useAccount();
  const [viewMode, setViewMode] = useState<ViewMode>('list');
  const [selectedPosition, setSelectedPosition] = useState<Position | null>(null);
  const [isFormOpen, setIsFormOpen] = useState(false);
  const [editingPosition, setEditingPosition] = useState<Position | null>(null);
  const [bulkConfirmation, setBulkConfirmation] = useState<BulkActionConfirmation>({
    action: '',
    positionIds: [],
    isOpen: false
  });

  // Get positions and risk data
  const {
    positions,
    riskMetrics,
    riskExplanation,
    isLoadingPositions,
    isLoadingRisk,
    error,
    isConnected: riskServiceConnected,
    refreshPositions,
    calculateRisk,
    explainRisk
  } = useRiskMonitoring(address);

  // Auto-refresh positions periodically
  useEffect(() => {
    if (address && isConnected) {
      const interval = setInterval(() => {
        refreshPositions();
      }, 60000); // Refresh every minute

      return () => clearInterval(interval);
    }
  }, [address, isConnected, refreshPositions]);

  // Mock API functions (would be replaced with real API calls)
  const handleCreatePosition = async (positionData: Partial<Position>) => {
    // Mock create position
    console.log('Creating position:', positionData);
    
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // In real implementation, this would call the API
    toast.success('Position created successfully');
    refreshPositions();
  };

  const handleUpdatePosition = async (positionData: Partial<Position>) => {
    // Mock update position
    console.log('Updating position:', positionData);
    
    // Simulate API call
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    toast.success('Position updated successfully');
    refreshPositions();
  };

  const handleDeletePosition = async (positionId: string) => {
    if (!confirm('Are you sure you want to delete this position?')) {
      return;
    }

    try {
      // Mock delete position
      console.log('Deleting position:', positionId);
      
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 500));
      
      toast.success('Position deleted successfully');
      refreshPositions();
      
      // If we're viewing the deleted position, go back to list
      if (selectedPosition?.id === positionId) {
        setViewMode('list');
        setSelectedPosition(null);
      }
    } catch (error) {
      toast.error('Failed to delete position');
    }
  };

  const handleBulkAction = (action: string, positionIds: string[]) => {
    setBulkConfirmation({
      action,
      positionIds,
      isOpen: true
    });
  };

  const executeBulkAction = async () => {
    const { action, positionIds } = bulkConfirmation;
    
    try {
      console.log(`Executing bulk ${action} on positions:`, positionIds);
      
      // Simulate API call
      await new Promise(resolve => setTimeout(resolve, 1000));
      
      let message = '';
      switch (action) {
        case 'activate':
          message = `${positionIds.length} position${positionIds.length > 1 ? 's' : ''} activated`;
          break;
        case 'deactivate':
          message = `${positionIds.length} position${positionIds.length > 1 ? 's' : ''} deactivated`;
          break;
        case 'delete':
          message = `${positionIds.length} position${positionIds.length > 1 ? 's' : ''} deleted`;
          break;
        default:
          message = 'Bulk action completed';
      }
      
      toast.success(message);
      refreshPositions();
    } catch (error) {
      toast.error('Bulk action failed');
    } finally {
      setBulkConfirmation({ action: '', positionIds: [], isOpen: false });
    }
  };

  const handleEditPosition = (position: Position) => {
    setEditingPosition(position);
    setIsFormOpen(true);
  };

  const handleViewPosition = (position: Position) => {
    setSelectedPosition(position);
    setViewMode('details');
  };

  const handleRefreshPosition = async (positionId: string) => {
    try {
      await calculateRisk(positionId);
      await explainRisk(positionId);
    } catch (error) {
      console.error('Failed to refresh position:', error);
    }
  };

  const handleFormClose = () => {
    setIsFormOpen(false);
    setEditingPosition(null);
  };

  const handleFormSave = async (positionData: Partial<Position>) => {
    if (editingPosition) {
      await handleUpdatePosition({ ...positionData, id: editingPosition.id });
    } else {
      await handleCreatePosition(positionData);
    }
    handleFormClose();
  };

  // Show connection prompt if not connected
  if (!isConnected) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
        <div className="flex items-center justify-center min-h-screen">
          <div className="text-center">
            <div className="mb-8">
              <PieChartIcon className="w-24 h-24 mx-auto text-blue-400 mb-4" />
              <h1 className="text-4xl font-bold text-white mb-2">Position Management</h1>
              <p className="text-xl text-gray-400">Manage your DeFi positions and monitor risks</p>
            </div>
            
            <div className="bg-gray-800/50 rounded-xl p-8 border border-gray-700 max-w-md mx-auto">
              <h2 className="text-xl font-semibold text-white mb-4">Connect Your Wallet</h2>
              <p className="text-gray-400 mb-6">
                Connect your wallet to manage your DeFi positions, track performance, and monitor risks in real-time.
              </p>
              <ConnectButton />
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-gray-900 via-gray-800 to-gray-900">
      {/* Header */}
      <div className="bg-gray-800/50 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-3">
              <PieChartIcon className="w-8 h-8 text-blue-400" />
              <div>
                <h1 className="text-xl font-bold text-white">Position Management</h1>
                <p className="text-sm text-gray-400">
                  {viewMode === 'details' && selectedPosition 
                    ? `${selectedPosition.token0_symbol}/${selectedPosition.token1_symbol} Details`
                    : `${positions.length} position${positions.length !== 1 ? 's' : ''} tracked`
                  }
                </p>
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
              
              {/* Actions */}
              {viewMode === 'list' && (
                <>
                  <button
                    onClick={refreshPositions}
                    disabled={isLoadingPositions}
                    className="p-2 text-gray-400 hover:text-white transition-colors disabled:opacity-50"
                    title="Refresh Positions"
                  >
                    <RefreshIcon className={`w-5 h-5 ${isLoadingPositions ? 'animate-spin' : ''}`} />
                  </button>
                  
                  <button
                    onClick={() => setIsFormOpen(true)}
                    className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
                  >
                    <PlusIcon className="w-4 h-4" />
                    Add Position
                  </button>
                </>
              )}
              
              {/* Wallet Connection */}
              <ConnectButton />
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {viewMode === 'list' && (
          <PositionList
            positions={positions}
            riskMetrics={(riskMetrics as unknown as Record<string, RiskMetrics>) || {}}
            isLoading={isLoadingPositions}
            onEdit={handleEditPosition}
            onDelete={handleDeletePosition}
            onView={handleViewPosition}
            onBulkAction={handleBulkAction}
          />
        )}

        {viewMode === 'details' && selectedPosition && (
          <PositionDetails
            position={selectedPosition}
            riskMetrics={riskMetrics ? (riskMetrics as any)[selectedPosition.id] : undefined}
            riskExplanation={riskExplanation || undefined}
            onBack={() => {
              setViewMode('list');
              setSelectedPosition(null);
            }}
            onEdit={handleEditPosition}
            onDelete={handleDeletePosition}
            onRefresh={handleRefreshPosition}
          />
        )}
      </div>

      {/* Position Form Modal */}
      <PositionForm
        position={editingPosition}
        isOpen={isFormOpen}
        onClose={handleFormClose}
        onSave={handleFormSave}
      />

      {/* Bulk Action Confirmation Modal */}
      {bulkConfirmation.isOpen && (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
          <div className="bg-gray-800 rounded-xl border border-gray-700 p-6 max-w-md w-full">
            <div className="flex items-center gap-3 mb-4">
              <AlertTriangleIcon className="w-6 h-6 text-yellow-400" />
              <h3 className="text-lg font-semibold text-white">Confirm Bulk Action</h3>
            </div>
            
            <p className="text-gray-300 mb-6">
              Are you sure you want to {bulkConfirmation.action} {bulkConfirmation.positionIds.length} position{bulkConfirmation.positionIds.length > 1 ? 's' : ''}?
              {bulkConfirmation.action === 'delete' && (
                <span className="block mt-2 text-red-400 text-sm">
                  This action cannot be undone.
                </span>
              )}
            </p>
            
            <div className="flex items-center justify-end gap-3">
              <button
                onClick={() => setBulkConfirmation({ action: '', positionIds: [], isOpen: false })}
                className="px-4 py-2 text-gray-400 hover:text-white transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={executeBulkAction}
                className={`px-4 py-2 rounded-lg transition-colors ${
                  bulkConfirmation.action === 'delete'
                    ? 'bg-red-600 hover:bg-red-700 text-white'
                    : 'bg-blue-600 hover:bg-blue-700 text-white'
                }`}
              >
                {bulkConfirmation.action === 'delete' ? 'Delete' : 
                 bulkConfirmation.action === 'activate' ? 'Activate' : 'Deactivate'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Loading Overlay */}
      {(isLoadingPositions || isLoadingRisk) && (
        <div className="fixed bottom-4 right-4 bg-gray-800 border border-gray-700 rounded-lg p-3 flex items-center gap-2">
          <LoadingSpinner className="w-4 h-4 text-blue-400" />
          <span className="text-sm text-gray-300">
            {isLoadingPositions ? 'Loading positions...' : 'Calculating risks...'}
          </span>
        </div>
      )}
    </div>
  );
};

export default PositionsPage;
