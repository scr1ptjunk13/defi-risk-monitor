// @ts-nocheck
import { useState, type FC, useEffect } from "react";
import { useAccount } from "wagmi";
import { FEE_TIERS, COMMISSION_AMOUNT } from "../constants";
import { useTokens } from "../hooks/useTokens";
import { useLiquidity } from "../hooks/useLiquidity";
import { useRiskMonitoring } from "../hooks/useRiskMonitoring";
import TokenSelector from "./TokenSelector";
import { SwapIcon, InfoIcon, LoadingSpinner, SettingsIcon } from "./Icons";
import toast from "react-hot-toast";

const LiquidityForm: FC = () => {
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [poolExists, setPoolExists] = useState(false);

  const { address, isConnected } = useAccount();

  const {
    tokenA,
    tokenB,
    setTokenA,
    setTokenB,
    validateTokenPair,
    clearTokenSelection,
  } = useTokens();

  const {
    amountA,
    amountB,
    selectedFee,
    priceRange,
    isCreatingPool,
    isAddingLiquidity,
    setAmountA,
    setAmountB,
    setSelectedFee,
    setPriceRange,
    createPool,
    addLiquidity,
    resetForm,
  } = useLiquidity();

  // Backend risk monitoring integration
  const {
    positions,
    riskMetrics,
    isLoading: isLoadingRisk,
    error: riskError,
    createPosition,
    getRiskMetrics,
    isConnected: isRiskConnected,
    clearError: clearRiskError,
  } = useRiskMonitoring();

  const handleSwapTokens = () => {
    setTokenA(tokenB);
    setTokenB(tokenA);
    setAmountA(amountB);
    setAmountB(amountA);
  };

  const handleCreatePool = async () => {
    if (!isConnected) return toast.error("Please connect your wallet");
    if (!validateTokenPair()) return;
    if (!amountA || !amountB) return toast.error("Enter amounts for both tokens");
    
    try {
      // Create pool on blockchain
      const ok = await createPool(tokenA, tokenB, selectedFee, 1);
      if (ok) {
        setPoolExists(true);
        toast.success("Pool created! Add liquidity now.");
      }
    } catch (error) {
      console.error('Pool creation failed:', error);
      toast.error('Failed to create pool');
    }
  };

  const handleAddLiquidity = async () => {
    if (!isConnected) return toast.error("Please connect your wallet");
    if (!validateTokenPair()) return;
    
    try {
      // Add liquidity on blockchain
      const ok = await addLiquidity(tokenA, tokenB, selectedFee);
      if (ok) {
        // Create position in backend risk monitoring system
        try {
          const positionData = {
            user_address: address!,
            chain_id: 1, // Ethereum mainnet
            pool_address: `${tokenA.address}-${tokenB.address}-${selectedFee}`, // Mock pool address
            token0_address: tokenA.address,
            token1_address: tokenB.address,
            liquidity_amount: parseFloat(amountA) + parseFloat(amountB), // Simplified calculation
            price_range_lower: priceRange.min || 0,
            price_range_upper: priceRange.max || 0,
            fee_tier: selectedFee,
            protocol_name: "Uniswap V3",
          };

          const backendPosition = await createPosition(positionData);
          
          // Get initial risk metrics for the position
          if (backendPosition?.id) {
            await getRiskMetrics(backendPosition.id);
          }

          toast.success("✅ Liquidity added & risk monitoring enabled!");
        } catch (riskError) {
          console.error('Backend position creation failed:', riskError);
          toast.error('⚠️ Liquidity added but risk monitoring failed');
        }

        resetForm();
        clearTokenSelection();
      }
    } catch (error) {
      console.error('Liquidity addition failed:', error);
      toast.error('Failed to add liquidity');
    }
  };

  const canProceed = !!(tokenA && tokenB && amountA && amountB && parseFloat(amountA) > 0 && parseFloat(amountB) > 0);

  // UI trimmed down for brevity but functional
  return (
    <div className="w-full max-w-md mx-auto">
      <div className="bg-gray-900 rounded-2xl p-6 border border-gray-700">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold">Add Liquidity</h2>
          <button onClick={() => setShowAdvanced(!showAdvanced)}>
            <SettingsIcon className="w-5 h-5 text-gray-400" />
          </button>
        </div>

        <div className="mb-4 p-3 bg-blue-900/20 border border-blue-500/30 rounded-lg text-sm text-blue-200">
          A commission of {COMMISSION_AMOUNT} POL will be charged to create liquidity pool.
        </div>

        {/* Risk Monitoring Status */}
        <div className="mb-4 p-3 bg-gray-800/50 border border-gray-600 rounded-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-300">Risk Monitoring</span>
            <div className="flex items-center gap-2">
              <div className={`w-2 h-2 rounded-full ${
                isRiskConnected ? 'bg-green-500' : 'bg-red-500'
              }`} />
              <span className="text-xs text-gray-400">
                {isRiskConnected ? 'Connected' : 'Disconnected'}
              </span>
            </div>
          </div>
          
          {riskError && (
            <div className="text-xs text-red-400 mb-2">
              ⚠️ {riskError}
              <button 
                onClick={clearRiskError}
                className="ml-2 text-blue-400 hover:text-blue-300"
              >
                Clear
              </button>
            </div>
          )}
          
          {positions.length > 0 && (
            <div className="text-xs text-gray-400">
              📊 Monitoring {positions.length} position{positions.length !== 1 ? 's' : ''}
            </div>
          )}
          
          {riskMetrics && (
            <div className="mt-2 text-xs">
              <div className="flex justify-between items-center">
                <span className="text-gray-400">Overall Risk:</span>
                <span className={`font-medium ${
                  riskMetrics.overall_risk_score > 70 ? 'text-red-400' :
                  riskMetrics.overall_risk_score > 40 ? 'text-yellow-400' :
                  'text-green-400'
                }`}>
                  {riskMetrics.overall_risk_score.toFixed(1)}%
                </span>
              </div>
              <div className="flex justify-between items-center">
                <span className="text-gray-400">IL Risk:</span>
                <span className={`font-medium ${
                  riskMetrics.impermanent_loss_risk > 10 ? 'text-red-400' :
                  riskMetrics.impermanent_loss_risk > 5 ? 'text-yellow-400' :
                  'text-green-400'
                }`}>
                  {riskMetrics.impermanent_loss_risk.toFixed(1)}%
                </span>
              </div>
            </div>
          )}
        </div>

        <TokenSelector selectedToken={tokenA} onTokenSelect={setTokenA} otherToken={tokenB} label="First Token" className="mb-4" />
        {tokenA && (
          <input type="number" value={amountA} onChange={(e) => setAmountA(e.target.value)} className="w-full mb-4 bg-gray-800 p-3 rounded" placeholder={`${tokenA.symbol} amount`} />
        )}

        <div className="flex justify-center mb-4">
          <button onClick={handleSwapTokens} className="p-2 bg-gray-700 rounded">
            <SwapIcon className="w-5 h-5 text-gray-300" />
          </button>
        </div>

        <TokenSelector selectedToken={tokenB} onTokenSelect={setTokenB} otherToken={tokenA} label="Second Token" className="mb-4" />
        {tokenB && (
          <input type="number" value={amountB} onChange={(e) => setAmountB(e.target.value)} className="w-full mb-4 bg-gray-800 p-3 rounded" placeholder={`${tokenB.symbol} amount`} />
        )}

        <div className="grid grid-cols-3 gap-2 mb-4">
          {FEE_TIERS.map((tier) => (
            <button key={tier.value} onClick={() => setSelectedFee(tier.value)} className={`p-2 rounded ${selectedFee === tier.value ? "bg-blue-600" : "bg-gray-700"}`}>{tier.label}</button>
          ))}
        </div>

        <button 
          onClick={poolExists ? handleAddLiquidity : handleCreatePool} 
          disabled={!canProceed || isCreatingPool || isAddingLiquidity || isLoadingRisk} 
          className="w-full bg-blue-600 p-3 rounded text-white disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {(isCreatingPool || isAddingLiquidity || isLoadingRisk) && (
            <LoadingSpinner className="w-4 h-4" />
          )}
          {isCreatingPool ? "Creating Pool..." :
           isAddingLiquidity ? "Adding Liquidity..." :
           isLoadingRisk ? "Setting up Risk Monitoring..." :
           poolExists ? "Add Liquidity & Enable Risk Monitoring" : "Create Pool & Enable Risk Monitoring"}
        </button>
      </div>
    </div>
  );
};

export default LiquidityForm;
