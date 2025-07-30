// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../lib/openzeppelin-contracts/contracts/access/Ownable.sol";
import "../lib/openzeppelin-contracts/contracts/utils/ReentrancyGuard.sol";
import "../lib/openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "../lib/openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title SimpleLiquidityFactory
 * @dev Simplified pool creation and liquidity management with commission system
 */
contract SimpleLiquidityFactory is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // Commission rate in basis points (30 = 0.3%)
    uint256 public constant COMMISSION_RATE = 30;
    
    // Commission recipient
    address public commissionRecipient;
    
    // Events
    event PoolCreated(
        address indexed token0,
        address indexed token1,
        uint24 fee,
        address pool,
        uint256 commission0,
        uint256 commission1
    );
    
    event LiquidityAdded(
        address indexed user,
        address indexed token0,
        address indexed token1,
        uint256 amount0,
        uint256 amount1,
        uint256 commission0,
        uint256 commission1
    );
    
    event CommissionRecipientUpdated(address oldRecipient, address newRecipient);

    struct PoolParams {
        address token0;
        address token1;
        uint24 fee;
        uint256 amount0Desired;
        uint256 amount1Desired;
        uint256 amount0Min;
        uint256 amount1Min;
        uint256 deadline;
    }

    constructor(address initialOwner, address _commissionRecipient) Ownable(initialOwner) {
        require(_commissionRecipient != address(0), "Invalid commission recipient");
        commissionRecipient = _commissionRecipient;
    }

    /**
     * @dev Create pool and add initial liquidity with commission
     */
    function createPoolAndAddLiquidity(PoolParams calldata params) 
        external 
        nonReentrant 
        returns (uint256 amount0Used, uint256 amount1Used, uint256 commission0, uint256 commission1) 
    {
        require(block.timestamp <= params.deadline, "Transaction expired");
        require(params.token0 != params.token1, "Identical tokens");
        require(params.token0 != address(0) && params.token1 != address(0), "Zero address");
        require(params.amount0Desired > 0 && params.amount1Desired > 0, "Zero amounts");

        // Calculate commission
        commission0 = (params.amount0Desired * COMMISSION_RATE) / 10000;
        commission1 = (params.amount1Desired * COMMISSION_RATE) / 10000;
        
        // Calculate net amounts after commission
        amount0Used = params.amount0Desired - commission0;
        amount1Used = params.amount1Desired - commission1;
        
        require(amount0Used >= params.amount0Min, "Insufficient amount0");
        require(amount1Used >= params.amount1Min, "Insufficient amount1");

        // Transfer tokens from user
        IERC20(params.token0).safeTransferFrom(msg.sender, address(this), params.amount0Desired);
        IERC20(params.token1).safeTransferFrom(msg.sender, address(this), params.amount1Desired);

        // Transfer commission to recipient
        if (commission0 > 0) {
            IERC20(params.token0).safeTransfer(commissionRecipient, commission0);
        }
        if (commission1 > 0) {
            IERC20(params.token1).safeTransfer(commissionRecipient, commission1);
        }

        // For this simplified version, we'll simulate pool creation
        // In production, this would interact with Uniswap V3 Factory
        address mockPool = address(uint160(uint256(keccak256(abi.encodePacked(
            params.token0, 
            params.token1, 
            params.fee,
            block.timestamp
        )))));

        emit PoolCreated(
            params.token0,
            params.token1,
            params.fee,
            mockPool,
            commission0,
            commission1
        );

        emit LiquidityAdded(
            msg.sender,
            params.token0,
            params.token1,
            amount0Used,
            amount1Used,
            commission0,
            commission1
        );

        return (amount0Used, amount1Used, commission0, commission1);
    }

    /**
     * @dev Add liquidity to existing pool with commission
     */
    function addLiquidity(PoolParams calldata params)
        external
        nonReentrant
        returns (uint256 amount0Used, uint256 amount1Used, uint256 commission0, uint256 commission1)
    {
        require(block.timestamp <= params.deadline, "Transaction expired");
        require(params.amount0Desired > 0 && params.amount1Desired > 0, "Zero amounts");

        // Calculate commission
        commission0 = (params.amount0Desired * COMMISSION_RATE) / 10000;
        commission1 = (params.amount1Desired * COMMISSION_RATE) / 10000;
        
        // Calculate net amounts after commission
        amount0Used = params.amount0Desired - commission0;
        amount1Used = params.amount1Desired - commission1;
        
        require(amount0Used >= params.amount0Min, "Insufficient amount0");
        require(amount1Used >= params.amount1Min, "Insufficient amount1");

        // Transfer tokens from user
        IERC20(params.token0).safeTransferFrom(msg.sender, address(this), params.amount0Desired);
        IERC20(params.token1).safeTransferFrom(msg.sender, address(this), params.amount1Desired);

        // Transfer commission to recipient
        if (commission0 > 0) {
            IERC20(params.token0).safeTransfer(commissionRecipient, commission0);
        }
        if (commission1 > 0) {
            IERC20(params.token1).safeTransfer(commissionRecipient, commission1);
        }

        emit LiquidityAdded(
            msg.sender,
            params.token0,
            params.token1,
            amount0Used,
            amount1Used,
            commission0,
            commission1
        );

        return (amount0Used, amount1Used, commission0, commission1);
    }

    /**
     * @dev Update commission recipient (owner only)
     */
    function updateCommissionRecipient(address newRecipient) external onlyOwner {
        require(newRecipient != address(0), "Invalid recipient");
        address oldRecipient = commissionRecipient;
        commissionRecipient = newRecipient;
        emit CommissionRecipientUpdated(oldRecipient, newRecipient);
    }

    /**
     * @dev Emergency withdrawal function (owner only)
     */
    function emergencyWithdraw(address token, uint256 amount) external onlyOwner {
        IERC20(token).safeTransfer(owner(), amount);
    }

    /**
     * @dev Get commission for given amounts
     */
    function getCommission(uint256 amount0, uint256 amount1) 
        external 
        pure 
        returns (uint256 commission0, uint256 commission1) 
    {
        commission0 = (amount0 * COMMISSION_RATE) / 10000;
        commission1 = (amount1 * COMMISSION_RATE) / 10000;
    }
}
