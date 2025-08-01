// @ts-nocheck
import { useState } from "react";
import { useAccount, useChainId, useWalletClient } from "wagmi";
import { toast } from "react-hot-toast";
import { ethers } from "ethers";
import {
  STANDALONE_LIQUIDITY_FACTORY_ABI,
  ERC20_ABI,
  CONTRACT_ADDRESSES,
  calculateCommission,
  parseTokenAmount,
  validatePoolParams,
  FEE_TIERS
} from "../lib/contracts";

export const useLiquidity = () => {
  const [amountA, setAmountA] = useState<string>("");
  const [amountB, setAmountB] = useState<string>("");
  const [selectedFee, setSelectedFee] = useState<number>(3000); // Default to 0.3%
  const [isCreatingPool, setIsCreatingPool] = useState(false);
  const [isAddingLiquidity, setIsAddingLiquidity] = useState(false);

  const { address } = useAccount();
  const chainId = useChainId();
  const { data: walletClient } = useWalletClient();
  
  const createPool = async (tokenA, tokenB, fee = selectedFee, initialPrice = 1) => {
    if (!walletClient || !address || !chainId) {
      toast.error("Please connect your wallet");
      return false;
    }

    const factoryAddress = CONTRACT_ADDRESSES[chainId]?.STANDALONE_LIQUIDITY_FACTORY;
    if (!factoryAddress || factoryAddress === "0x0000000000000000000000000000000000000000") {
      toast.error("Contract not deployed on this network");
      return false;
    }

    try {
      setIsCreatingPool(true);
      
      // Create contract instances
      // Note: For wagmi v2, we need to use a different approach for contract interactions
      // This is a simplified version - in production, you'd use viem or ethers v6 with proper wallet client integration
      const factory = new ethers.Contract(factoryAddress, STANDALONE_LIQUIDITY_FACTORY_ABI, walletClient);
      const tokenAContract = new ethers.Contract(tokenA.address, ERC20_ABI, walletClient);
      const tokenBContract = new ethers.Contract(tokenB.address, ERC20_ABI, walletClient);
      
      // Get token decimals
      const [decimalsA, decimalsB] = await Promise.all([
        tokenAContract.decimals(),
        tokenBContract.decimals()
      ]);

      // Parse amounts
      const amount0Desired = parseTokenAmount(amountA, decimalsA);
      const amount1Desired = parseTokenAmount(amountB, decimalsB);
      
      // Calculate minimum amounts (accounting for 0.3% commission + 5% slippage)
      const commission0 = ethers.BigNumber.from(amount0Desired).mul(30).div(10000);
      const commission1 = ethers.BigNumber.from(amount1Desired).mul(30).div(10000);
      const amount0Min = ethers.BigNumber.from(amount0Desired).sub(commission0).mul(95).div(100);
      const amount1Min = ethers.BigNumber.from(amount1Desired).sub(commission1).mul(95).div(100);

      // Validate parameters
      const poolParams = {
        token0: tokenA.address,
        token1: tokenB.address,
        amount0Desired: ethers.utils.formatUnits(amount0Desired, decimalsA),
        amount1Desired: ethers.utils.formatUnits(amount1Desired, decimalsB),
        amount0Min: ethers.utils.formatUnits(amount0Min, decimalsA),
        amount1Min: ethers.utils.formatUnits(amount1Min, decimalsB),
        deadline: Math.floor(Date.now() / 1000) + 300 // 5 minutes
      };

      const validationErrors = validatePoolParams(poolParams);
      if (validationErrors.length > 0) {
        toast.error(`Validation failed: ${validationErrors.join(", ")}`);
        return false;
      }

      // Check token allowances
      const [allowanceA, allowanceB] = await Promise.all([
        tokenAContract.allowance(address, factoryAddress),
        tokenBContract.allowance(address, factoryAddress)
      ]);

      // Approve tokens if needed
      if (allowanceA.lt(amount0Desired)) {
        toast.info("Approving token A...");
        const approveTxA = await tokenAContract.approve(factoryAddress, amount0Desired);
        await approveTxA.wait();
      }

      if (allowanceB.lt(amount1Desired)) {
        toast.info("Approving token B...");
        const approveTxB = await tokenBContract.approve(factoryAddress, amount1Desired);
        await approveTxB.wait();
      }

      // Create pool parameters struct
      const params = {
        token0: tokenA.address,
        token1: tokenB.address,
        fee: fee,
        amount0Desired: amount0Desired,
        amount1Desired: amount1Desired,
        amount0Min: amount0Min.toString(),
        amount1Min: amount1Min.toString(),
        deadline: Math.floor(Date.now() / 1000) + 300
      };

      toast.info("Creating pool and adding liquidity...");
      
      // Call createPoolAndAddLiquidity
      const tx = await factory.createPoolAndAddLiquidity(params);
      const receipt = await tx.wait();

      // Parse events
      const poolCreatedEvent = receipt.events?.find(e => e.event === 'PoolCreated');
      const liquidityAddedEvent = receipt.events?.find(e => e.event === 'LiquidityAdded');

      toast.success("Pool created successfully!");
      
      return {
        success: true,
        txHash: receipt.transactionHash,
        poolAddress: poolCreatedEvent?.args?.pool,
        amount0Used: liquidityAddedEvent?.args?.amount0,
        amount1Used: liquidityAddedEvent?.args?.amount1,
        commission0: liquidityAddedEvent?.args?.commission0,
        commission1: liquidityAddedEvent?.args?.commission1
      };

    } catch (error) {
      console.error("Pool creation failed:", error);
      toast.error(`Pool creation failed: ${error.message}`);
      return false;
    } finally {
      setIsCreatingPool(false);
    }
  };

  const addLiquidity = async (tokenA, tokenB, fee = selectedFee) => {
    if (!walletClient || !address || !chainId) {
      toast.error("Please connect your wallet");
      return false;
    }

    const factoryAddress = CONTRACT_ADDRESSES[chainId]?.STANDALONE_LIQUIDITY_FACTORY;
    if (!factoryAddress || factoryAddress === "0x0000000000000000000000000000000000000000") {
      toast.error("Contract not deployed on this network");
      return false;
    }

    try {
      setIsAddingLiquidity(true);
      
      // Create contract instances
      // Note: For wagmi v2, we need to use a different approach for contract interactions
      // This is a simplified version - in production, you'd use viem or ethers v6 with proper wallet client integration
      const factory = new ethers.Contract(factoryAddress, STANDALONE_LIQUIDITY_FACTORY_ABI, walletClient);
      const tokenAContract = new ethers.Contract(tokenA.address, ERC20_ABI, walletClient);
      const tokenBContract = new ethers.Contract(tokenB.address, ERC20_ABI, walletClient);
      
      // Get token decimals
      const [decimalsA, decimalsB] = await Promise.all([
        tokenAContract.decimals(),
        tokenBContract.decimals()
      ]);

      // Parse amounts
      const amount0Desired = parseTokenAmount(amountA, decimalsA);
      const amount1Desired = parseTokenAmount(amountB, decimalsB);
      
      // Calculate minimum amounts (accounting for 0.3% commission + 5% slippage)
      const commission0 = ethers.BigNumber.from(amount0Desired).mul(30).div(10000);
      const commission1 = ethers.BigNumber.from(amount1Desired).mul(30).div(10000);
      const amount0Min = ethers.BigNumber.from(amount0Desired).sub(commission0).mul(95).div(100);
      const amount1Min = ethers.BigNumber.from(amount1Desired).sub(commission1).mul(95).div(100);

      // Check token allowances
      const [allowanceA, allowanceB] = await Promise.all([
        tokenAContract.allowance(address, factoryAddress),
        tokenBContract.allowance(address, factoryAddress)
      ]);

      // Approve tokens if needed
      if (allowanceA.lt(amount0Desired)) {
        toast.info("Approving token A...");
        const approveTxA = await tokenAContract.approve(factoryAddress, amount0Desired);
        await approveTxA.wait();
      }

      if (allowanceB.lt(amount1Desired)) {
        toast.info("Approving token B...");
        const approveTxB = await tokenBContract.approve(factoryAddress, amount1Desired);
        await approveTxB.wait();
      }

      // Create liquidity parameters struct
      const params = {
        token0: tokenA.address,
        token1: tokenB.address,
        fee: fee,
        amount0Desired: amount0Desired,
        amount1Desired: amount1Desired,
        amount0Min: amount0Min.toString(),
        amount1Min: amount1Min.toString(),
        deadline: Math.floor(Date.now() / 1000) + 300
      };

      toast.info("Adding liquidity...");
      
      // Call addLiquidity
      const tx = await factory.addLiquidity(params);
      const receipt = await tx.wait();

      // Parse events
      const liquidityAddedEvent = receipt.events?.find(e => e.event === 'LiquidityAdded');

      toast.success("Liquidity added successfully!");
      
      return {
        success: true,
        txHash: receipt.transactionHash,
        amount0Used: liquidityAddedEvent?.args?.amount0,
        amount1Used: liquidityAddedEvent?.args?.amount1,
        commission0: liquidityAddedEvent?.args?.commission0,
        commission1: liquidityAddedEvent?.args?.commission1
      };

    } catch (error) {
      console.error("Add liquidity failed:", error);
      toast.error(`Add liquidity failed: ${error.message}`);
      return false;
    } finally {
      setIsAddingLiquidity(false);
    }
  };

  const getCommissionEstimate = async (amount0: string, amount1: string) => {
    if (!chainId) return null;

    const factoryAddress = CONTRACT_ADDRESSES[chainId]?.STANDALONE_LIQUIDITY_FACTORY;
    if (!factoryAddress || factoryAddress === "0x0000000000000000000000000000000000000000") {
      // Fallback calculation
      return {
        commission0: calculateCommission(amount0),
        commission1: calculateCommission(amount1)
      };
    }

    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      const factory = new ethers.Contract(factoryAddress, STANDALONE_LIQUIDITY_FACTORY_ABI, provider);
      
      const amount0Wei = ethers.utils.parseEther(amount0 || "0");
      const amount1Wei = ethers.utils.parseEther(amount1 || "0");
      
      const [commission0, commission1] = await factory.getCommission(amount0Wei, amount1Wei);
      
      return {
        commission0: ethers.utils.formatEther(commission0),
        commission1: ethers.utils.formatEther(commission1)
      };
    } catch (error) {
      console.error("Failed to get commission estimate:", error);
      return {
        commission0: calculateCommission(amount0 || "0"),
        commission1: calculateCommission(amount1 || "0")
      };
    }
  };

  const getContractInfo = async () => {
    if (!chainId) return null;

    const factoryAddress = CONTRACT_ADDRESSES[chainId]?.STANDALONE_LIQUIDITY_FACTORY;
    if (!factoryAddress || factoryAddress === "0x0000000000000000000000000000000000000000") {
      return null;
    }

    try {
      const provider = new ethers.providers.Web3Provider(window.ethereum);
      const factory = new ethers.Contract(factoryAddress, STANDALONE_LIQUIDITY_FACTORY_ABI, provider);
      
      const [owner, commissionRecipient, commissionRate] = await factory.getContractInfo();
      
      return {
        owner,
        commissionRecipient,
        commissionRate: commissionRate.toNumber()
      };
    } catch (error) {
      console.error("Failed to get contract info:", error);
      return null;
    }
  };

  return {
    // State
    amountA,
    amountB,
    selectedFee,
    isCreatingPool,
    isAddingLiquidity,
    
    // Setters
    setAmountA,
    setAmountB,
    setSelectedFee,
    
    // Actions
    createPool,
    addLiquidity,
    getCommissionEstimate,
    getContractInfo,
    
    // Constants
    FEE_TIERS,
    COMMISSION_RATE: 30 // 0.3%
  };
};
