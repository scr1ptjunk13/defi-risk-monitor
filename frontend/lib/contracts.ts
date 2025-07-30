import { ethers } from 'ethers';

// Contract ABI for StandaloneLiquidityFactory
export const STANDALONE_LIQUIDITY_FACTORY_ABI = [
  {
    "inputs": [
      {"internalType": "address", "name": "_commissionRecipient", "type": "address"}
    ],
    "stateMutability": "nonpayable",
    "type": "constructor"
  },
  {
    "inputs": [
      {
        "components": [
          {"internalType": "address", "name": "token0", "type": "address"},
          {"internalType": "address", "name": "token1", "type": "address"},
          {"internalType": "uint24", "name": "fee", "type": "uint24"},
          {"internalType": "uint256", "name": "amount0Desired", "type": "uint256"},
          {"internalType": "uint256", "name": "amount1Desired", "type": "uint256"},
          {"internalType": "uint256", "name": "amount0Min", "type": "uint256"},
          {"internalType": "uint256", "name": "amount1Min", "type": "uint256"},
          {"internalType": "uint256", "name": "deadline", "type": "uint256"}
        ],
        "internalType": "struct StandaloneLiquidityFactory.PoolParams",
        "name": "params",
        "type": "tuple"
      }
    ],
    "name": "createPoolAndAddLiquidity",
    "outputs": [
      {"internalType": "uint256", "name": "amount0Used", "type": "uint256"},
      {"internalType": "uint256", "name": "amount1Used", "type": "uint256"},
      {"internalType": "uint256", "name": "commission0", "type": "uint256"},
      {"internalType": "uint256", "name": "commission1", "type": "uint256"}
    ],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {
        "components": [
          {"internalType": "address", "name": "token0", "type": "address"},
          {"internalType": "address", "name": "token1", "type": "address"},
          {"internalType": "uint24", "name": "fee", "type": "uint24"},
          {"internalType": "uint256", "name": "amount0Desired", "type": "uint256"},
          {"internalType": "uint256", "name": "amount1Desired", "type": "uint256"},
          {"internalType": "uint256", "name": "amount0Min", "type": "uint256"},
          {"internalType": "uint256", "name": "amount1Min", "type": "uint256"},
          {"internalType": "uint256", "name": "deadline", "type": "uint256"}
        ],
        "internalType": "struct StandaloneLiquidityFactory.PoolParams",
        "name": "params",
        "type": "tuple"
      }
    ],
    "name": "addLiquidity",
    "outputs": [
      {"internalType": "uint256", "name": "amount0Used", "type": "uint256"},
      {"internalType": "uint256", "name": "amount1Used", "type": "uint256"},
      {"internalType": "uint256", "name": "commission0", "type": "uint256"},
      {"internalType": "uint256", "name": "commission1", "type": "uint256"}
    ],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {"internalType": "uint256", "name": "amount0", "type": "uint256"},
      {"internalType": "uint256", "name": "amount1", "type": "uint256"}
    ],
    "name": "getCommission",
    "outputs": [
      {"internalType": "uint256", "name": "commission0", "type": "uint256"},
      {"internalType": "uint256", "name": "commission1", "type": "uint256"}
    ],
    "stateMutability": "pure",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "getContractInfo",
    "outputs": [
      {"internalType": "address", "name": "_owner", "type": "address"},
      {"internalType": "address", "name": "_commissionRecipient", "type": "address"},
      {"internalType": "uint256", "name": "_commissionRate", "type": "uint256"}
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [
      {"internalType": "address", "name": "newRecipient", "type": "address"}
    ],
    "name": "updateCommissionRecipient",
    "outputs": [],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "COMMISSION_RATE",
    "outputs": [
      {"internalType": "uint256", "name": "", "type": "uint256"}
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "commissionRecipient",
    "outputs": [
      {"internalType": "address", "name": "", "type": "address"}
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "owner",
    "outputs": [
      {"internalType": "address", "name": "", "type": "address"}
    ],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "anonymous": false,
    "inputs": [
      {"indexed": true, "internalType": "address", "name": "token0", "type": "address"},
      {"indexed": true, "internalType": "address", "name": "token1", "type": "address"},
      {"indexed": false, "internalType": "uint24", "name": "fee", "type": "uint24"},
      {"indexed": false, "internalType": "address", "name": "pool", "type": "address"},
      {"indexed": false, "internalType": "uint256", "name": "commission0", "type": "uint256"},
      {"indexed": false, "internalType": "uint256", "name": "commission1", "type": "uint256"}
    ],
    "name": "PoolCreated",
    "type": "event"
  },
  {
    "anonymous": false,
    "inputs": [
      {"indexed": true, "internalType": "address", "name": "user", "type": "address"},
      {"indexed": true, "internalType": "address", "name": "token0", "type": "address"},
      {"indexed": true, "internalType": "address", "name": "token1", "type": "address"},
      {"indexed": false, "internalType": "uint256", "name": "amount0", "type": "uint256"},
      {"indexed": false, "internalType": "uint256", "name": "amount1", "type": "uint256"},
      {"indexed": false, "internalType": "uint256", "name": "commission0", "type": "uint256"},
      {"indexed": false, "internalType": "uint256", "name": "commission1", "type": "uint256"}
    ],
    "name": "LiquidityAdded",
    "type": "event"
  }
];

// Standard ERC20 ABI (minimal)
export const ERC20_ABI = [
  {
    "inputs": [
      {"internalType": "address", "name": "spender", "type": "address"},
      {"internalType": "uint256", "name": "amount", "type": "uint256"}
    ],
    "name": "approve",
    "outputs": [{"internalType": "bool", "name": "", "type": "bool"}],
    "stateMutability": "nonpayable",
    "type": "function"
  },
  {
    "inputs": [
      {"internalType": "address", "name": "account", "type": "address"}
    ],
    "name": "balanceOf",
    "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "decimals",
    "outputs": [{"internalType": "uint8", "name": "", "type": "uint8"}],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "symbol",
    "outputs": [{"internalType": "string", "name": "", "type": "string"}],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [],
    "name": "name",
    "outputs": [{"internalType": "string", "name": "", "type": "string"}],
    "stateMutability": "view",
    "type": "function"
  },
  {
    "inputs": [
      {"internalType": "address", "name": "owner", "type": "address"},
      {"internalType": "address", "name": "spender", "type": "address"}
    ],
    "name": "allowance",
    "outputs": [{"internalType": "uint256", "name": "", "type": "uint256"}],
    "stateMutability": "view",
    "type": "function"
  }
];

// Contract addresses for different networks
export const CONTRACT_ADDRESSES: { [chainId: number]: { STANDALONE_LIQUIDITY_FACTORY: string } } = {
  1: { // Mainnet
    STANDALONE_LIQUIDITY_FACTORY: "0x0000000000000000000000000000000000000000" // To be deployed
  },
  11155111: { // Sepolia
    STANDALONE_LIQUIDITY_FACTORY: "0x0000000000000000000000000000000000000000" // To be deployed
  },
  31337: { // Local/Hardhat
    STANDALONE_LIQUIDITY_FACTORY: "0x0000000000000000000000000000000000000000" // To be deployed
  }
};

// Utility functions
export const calculateCommission = (amount: string): string => {
  const amountBN = ethers.utils.parseEther(amount);
  const commission = amountBN.mul(30).div(10000); // 0.3% commission
  return ethers.utils.formatEther(commission);
};

export const parseTokenAmount = (amount: string, decimals: number): string => {
  return ethers.utils.parseUnits(amount, decimals).toString();
};

export const formatTokenAmount = (amount: string, decimals: number): string => {
  return ethers.utils.formatUnits(amount, decimals);
};

// Pool parameter validation
export const validatePoolParams = (params: {
  token0: string;
  token1: string;
  amount0Desired: string;
  amount1Desired: string;
  amount0Min: string;
  amount1Min: string;
  deadline: number;
}) => {
  const errors: string[] = [];

  if (!ethers.utils.isAddress(params.token0)) {
    errors.push("Invalid token0 address");
  }
  if (!ethers.utils.isAddress(params.token1)) {
    errors.push("Invalid token1 address");
  }
  if (params.token0.toLowerCase() === params.token1.toLowerCase()) {
    errors.push("Token addresses must be different");
  }
  if (parseFloat(params.amount0Desired) <= 0) {
    errors.push("Amount0 must be greater than 0");
  }
  if (parseFloat(params.amount1Desired) <= 0) {
    errors.push("Amount1 must be greater than 0");
  }
  if (params.deadline <= Math.floor(Date.now() / 1000)) {
    errors.push("Deadline must be in the future");
  }

  return errors;
};

// Fee tier options
export const FEE_TIERS = [
  { value: 500, label: "0.05%" },
  { value: 3000, label: "0.3%" },
  { value: 10000, label: "1%" }
];

// Commission rate constant
export const COMMISSION_RATE = 30; // 0.3% in basis points
