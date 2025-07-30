import type { NextApiRequest, NextApiResponse } from 'next';
import { ethers } from 'ethers';

interface CreatePoolRequest {
  tokenA: {
    address: string;
    symbol: string;
    decimals: number;
  };
  tokenB: {
    address: string;
    symbol: string;
    decimals: number;
  };
  amountA: string;
  amountB: string;
  fee: number;
  userAddress: string;
}

interface CreatePoolResponse {
  success: boolean;
  contractParams?: {
    token0: string;
    token1: string;
    fee: number;
    amount0Desired: string;
    amount1Desired: string;
    amount0Min: string;
    amount1Min: string;
    deadline: number;
    commission0: string;
    commission1: string;
  };
  error?: string;
}

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse<CreatePoolResponse>
) {
  if (req.method !== 'POST') {
    return res.status(405).json({ success: false, error: 'Method not allowed' });
  }

  try {
    const { tokenA, tokenB, amountA, amountB, fee, userAddress }: CreatePoolRequest = req.body;

    // Validate input
    if (!tokenA?.address || !tokenB?.address || !amountA || !amountB || !fee || !userAddress) {
      return res.status(400).json({ 
        success: false, 
        error: 'Missing required parameters' 
      });
    }

    // Validate addresses
    if (!ethers.utils.isAddress(tokenA.address) || !ethers.utils.isAddress(tokenB.address)) {
      return res.status(400).json({ 
        success: false, 
        error: 'Invalid token addresses' 
      });
    }

    if (!ethers.utils.isAddress(userAddress)) {
      return res.status(400).json({ 
        success: false, 
        error: 'Invalid user address' 
      });
    }

    // Validate amounts
    if (parseFloat(amountA) <= 0 || parseFloat(amountB) <= 0) {
      return res.status(400).json({ 
        success: false, 
        error: 'Amounts must be greater than 0' 
      });
    }

    // Parse amounts to wei
    const amount0Desired = ethers.utils.parseUnits(amountA, tokenA.decimals);
    const amount1Desired = ethers.utils.parseUnits(amountB, tokenB.decimals);

    // Calculate commission (0.3%)
    const commission0 = amount0Desired.mul(30).div(10000);
    const commission1 = amount1Desired.mul(30).div(10000);

    // Calculate minimum amounts (net amount after commission - 5% slippage tolerance)
    const netAmount0 = amount0Desired.sub(commission0);
    const netAmount1 = amount1Desired.sub(commission1);
    const amount0Min = netAmount0.mul(95).div(100);
    const amount1Min = netAmount1.mul(95).div(100);

    // Set deadline to 5 minutes from now
    const deadline = Math.floor(Date.now() / 1000) + 300;

    // Prepare contract parameters for StandaloneLiquidityFactory
    const contractParams = {
      token0: tokenA.address,
      token1: tokenB.address,
      fee: fee,
      amount0Desired: amount0Desired.toString(),
      amount1Desired: amount1Desired.toString(),
      amount0Min: amount0Min.toString(),
      amount1Min: amount1Min.toString(),
      deadline: deadline,
      commission0: ethers.utils.formatUnits(commission0, tokenA.decimals),
      commission1: ethers.utils.formatUnits(commission1, tokenB.decimals)
    };

    // Log the pool creation request
    console.log('Pool creation request:', {
      tokenA: `${tokenA.symbol} (${tokenA.address})`,
      tokenB: `${tokenB.symbol} (${tokenB.address})`,
      amountA,
      amountB,
      fee: `${fee / 10000}%`,
      userAddress,
      estimatedCommission0: contractParams.commission0,
      estimatedCommission1: contractParams.commission1
    });

    return res.status(200).json({
      success: true,
      contractParams
    });

  } catch (error) {
    console.error('Create pool API error:', error);
    return res.status(500).json({
      success: false,
      error: error instanceof Error ? error.message : 'Internal server error'
    });
  }
}
