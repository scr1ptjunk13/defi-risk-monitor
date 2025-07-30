// @ts-nocheck
import { useState, useCallback } from "react";
import { useAccount, usePublicClient } from "wagmi";
import { ethers } from "ethers";
import { ERC20_ABI } from "../constants";
import toast from "react-hot-toast";

export interface TokenInfo {
  address: string;
  name: string;
  symbol: string;
  decimals: number;
  totalSupply?: string;
  balance?: string;
  logoURI?: string;
  isCustom?: boolean;
}

export const useTokens = () => {
  const [tokenA, setTokenA] = useState<TokenInfo | null>(null);
  const [tokenB, setTokenB] = useState<TokenInfo | null>(null);
  const [customTokenAddress, setCustomTokenAddress] = useState<string>("");
  const [isLoadingToken, setIsLoadingToken] = useState<boolean>(false);

  const { address } = useAccount();
  const publicClient = usePublicClient();

  const fetchTokenInfo = useCallback(async (tokenAddress: string): Promise<TokenInfo> => {
    if (!tokenAddress || !ethers.utils.isAddress(tokenAddress)) {
      throw new Error("Invalid token address");
    }
    try {
      setIsLoadingToken(true);

      const calls = [
        { address: tokenAddress, abi: ERC20_ABI, functionName: "name" },
        { address: tokenAddress, abi: ERC20_ABI, functionName: "symbol" },
        { address: tokenAddress, abi: ERC20_ABI, functionName: "decimals" },
        { address: tokenAddress, abi: ERC20_ABI, functionName: "totalSupply" },
      ];

      const [name, symbol, decimals, totalSupply] = await Promise.all(
        calls.map((c) => publicClient.readContract(c as any))
      );

      return {
        address: tokenAddress,
        name,
        symbol,
        decimals: Number(decimals),
        totalSupply: ethers.utils.formatUnits(totalSupply, decimals),
      } as TokenInfo;
    } finally {
      setIsLoadingToken(false);
    }
  }, [publicClient]);

  const fetchTokenBalance = useCallback(async (tokenAddress: string, userAddress: string): Promise<string> => {
    if (!tokenAddress || !userAddress) return "0";
    try {
      if (tokenAddress.toLowerCase() === "native" || tokenAddress === ethers.constants.AddressZero) {
        const balance = await publicClient.getBalance({ address: userAddress });
        return ethers.utils.formatEther(balance);
      }
      const balanceBN = await publicClient.readContract({ address: tokenAddress, abi: ERC20_ABI, functionName: "balanceOf", args: [userAddress] });
      const decimals = await publicClient.readContract({ address: tokenAddress, abi: ERC20_ABI, functionName: "decimals" });
      return ethers.utils.formatUnits(balanceBN, decimals);
    } catch {
      return "0";
    }
  }, [publicClient]);

  const addCustomToken = async () => {
    if (!customTokenAddress) {
      toast.error("Please enter a token address");
      return null;
    }
    try {
      const info = await fetchTokenInfo(customTokenAddress);
      const balance = await fetchTokenBalance(customTokenAddress, address);
      const token: TokenInfo = { ...info, balance, isCustom: true };
      toast.success(`Token ${info.symbol} added successfully!`);
      setCustomTokenAddress("");
      return token;
    } catch {
      toast.error("Failed to fetch token information");
      return null;
    }
  };

  const validateTokenPair = () => {
    if (!tokenA || !tokenB) {
      toast.error("Please select both tokens");
      return false;
    }
    if (tokenA.address.toLowerCase() === tokenB.address.toLowerCase()) {
      toast.error("Tokens must be different");
      return false;
    }
    return true;
  };

  const clearTokenSelection = () => {
    setTokenA(null);
    setTokenB(null);
  };

  return {
    tokenA,
    tokenB,
    setTokenA,
    setTokenB,
    customTokenAddress,
    setCustomTokenAddress,
    isLoadingToken,
    fetchTokenInfo,
    fetchTokenBalance,
    addCustomToken,
    validateTokenPair,
    clearTokenSelection,
  } as const;
};
