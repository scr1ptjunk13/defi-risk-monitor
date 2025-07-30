// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "../lib/openzeppelin-contracts/contracts/access/Ownable.sol";
import "../lib/openzeppelin-contracts/contracts/utils/ReentrancyGuard.sol";
import "../lib/openzeppelin-contracts/contracts/token/ERC20/IERC20.sol";
import "../lib/openzeppelin-contracts/contracts/token/ERC20/utils/SafeERC20.sol";
import "@uniswap/v3-core/contracts/interfaces/IUniswapV3Factory.sol";
import "@uniswap/v3-core/contracts/interfaces/IUniswapV3Pool.sol";
import "@uniswap/v3-periphery/contracts/interfaces/INonfungiblePositionManager.sol";
// Removed problematic imports - will use direct interfaces

// TickMath constants for full range liquidity
int24 constant MIN_TICK = -887272;
int24 constant MAX_TICK = 887272;

/**
 * @title LiquidityPoolFactory
 * @dev Automated pool creation and liquidity management with built-in commission system
 * @notice This contract integrates with the DeFi Risk Monitor platform
 */
contract LiquidityPoolFactory is Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // Uniswap V3 contracts
    IUniswapV3Factory public immutable uniswapFactory;
    INonfungiblePositionManager public immutable positionManager;

    // Commission settings
    uint256 public constant COMMISSION_BASIS_POINTS = 30; // 0.3%
    uint256 public constant BASIS_POINTS_DENOMINATOR = 10000;
    address public commissionRecipient;

    // Supported fee tiers
    uint24[] public supportedFeeTiers = [500, 3000, 10000]; // 0.05%, 0.3%, 1%

    // Events
    event PoolCreated(
        address indexed token0,
        address indexed token1,
        uint24 indexed fee,
        address pool,
        address creator,
        uint256 commission
    );

    event LiquidityAdded(
        address indexed pool,
        address indexed user,
        uint256 tokenId,
        uint128 liquidity,
        uint256 amount0,
        uint256 amount1,
        uint256 commission
    );

    event CommissionCollected(
        address indexed token,
        uint256 amount,
        address recipient
    );

    // Structs
    struct PoolCreationParams {
        address token0;
        address token1;
        uint24 fee;
        uint160 sqrtPriceX96;
        uint256 amount0Desired;
        uint256 amount1Desired;
        uint256 amount0Min;
        uint256 amount1Min;
        int24 tickLower;
        int24 tickUpper;
        uint256 deadline;
    }

    struct PoolInfo {
        address pool;
        address creator;
        uint256 createdAt;
        uint256 totalCommissionCollected;
    }

    // Storage
    mapping(address => PoolInfo) public poolInfo;
    mapping(address => uint256) public userPoolCount;
    address[] public allPools;

    constructor(
        address initialOwner,
        address _uniswapFactory,
        address _positionManager,
        address _commissionRecipient
    ) Ownable(initialOwner) {
        require(_uniswapFactory != address(0), "Invalid factory address");
        require(_positionManager != address(0), "Invalid position manager");
        require(_commissionRecipient != address(0), "Invalid commission recipient");

        uniswapFactory = IUniswapV3Factory(_uniswapFactory);
        positionManager = INonfungiblePositionManager(_positionManager);
        commissionRecipient = _commissionRecipient;
    }

    /**
     * @dev Create a new Uniswap V3 pool and add initial liquidity
     * @param params Pool creation parameters
     * @return pool Address of the created pool
     * @return tokenId NFT token ID for the liquidity position
     */
    function createPoolAndAddLiquidity(PoolCreationParams calldata params)
        external
        payable
        nonReentrant
        returns (address pool, uint256 tokenId)
    {
        require(params.deadline >= block.timestamp, "Deadline expired");
        require(_isValidFeeTier(params.fee), "Unsupported fee tier");
        require(params.token0 != params.token1, "Identical tokens");
        require(params.token0 != address(0) && params.token1 != address(0), "Zero address");

        // Ensure token0 < token1
        (address token0, address token1) = params.token0 < params.token1 
            ? (params.token0, params.token1) 
            : (params.token1, params.token0);

        // Check if pool already exists
        pool = uniswapFactory.getPool(token0, token1, params.fee);
        
        if (pool == address(0)) {
            // Create new pool
            pool = uniswapFactory.createPool(token0, token1, params.fee);
            IUniswapV3Pool(pool).initialize(params.sqrtPriceX96);

            // Store pool info
            poolInfo[pool] = PoolInfo({
                pool: pool,
                creator: msg.sender,
                createdAt: block.timestamp,
                totalCommissionCollected: 0
            });

            allPools.push(pool);
            userPoolCount[msg.sender]++;
        }

        // Calculate and collect commission
        uint256 commission0 = _calculateCommission(params.amount0Desired);
        uint256 commission1 = _calculateCommission(params.amount1Desired);

        // Transfer tokens from user (including commission)
        IERC20(token0).safeTransferFrom(
            msg.sender, 
            address(this), 
            params.amount0Desired + commission0
        );
        IERC20(token1).safeTransferFrom(
            msg.sender, 
            address(this), 
            params.amount1Desired + commission1
        );

        // Collect commission
        if (commission0 > 0) {
            IERC20(token0).safeTransfer(commissionRecipient, commission0);
            poolInfo[pool].totalCommissionCollected += commission0;
            emit CommissionCollected(token0, commission0, commissionRecipient);
        }
        if (commission1 > 0) {
            IERC20(token1).safeTransfer(commissionRecipient, commission1);
            poolInfo[pool].totalCommissionCollected += commission1;
            emit CommissionCollected(token1, commission1, commissionRecipient);
        }

        // Approve position manager
        IERC20(token0).approve(address(positionManager), 0);
        IERC20(token0).approve(address(positionManager), params.amount0Desired);
        IERC20(token1).approve(address(positionManager), 0);
        IERC20(token1).approve(address(positionManager), params.amount1Desired);

        // Add liquidity
        INonfungiblePositionManager.MintParams memory mintParams = INonfungiblePositionManager.MintParams({
            token0: token0,
            token1: token1,
            fee: params.fee,
            tickLower: params.tickLower,
            tickUpper: params.tickUpper,
            amount0Desired: params.amount0Desired,
            amount1Desired: params.amount1Desired,
            amount0Min: params.amount0Min,
            amount1Min: params.amount1Min,
            recipient: msg.sender,
            deadline: params.deadline
        });

        uint256 amount0;
        uint256 amount1;
        (tokenId, , amount0, amount1) = positionManager.mint(mintParams);

        // Refund unused tokens
        if (params.amount0Desired > amount0) {
            IERC20(token0).safeTransfer(msg.sender, params.amount0Desired - amount0);
        }
        if (params.amount1Desired > amount1) {
            IERC20(token1).safeTransfer(msg.sender, params.amount1Desired - amount1);
        }

        emit PoolCreated(
            token0,
            token1,
            params.fee,
            pool,
            msg.sender,
            commission0 + commission1
        );

        emit LiquidityAdded(
            pool,
            msg.sender,
            tokenId,
            0, // liquidity amount would need to be calculated
            amount0,
            amount1,
            commission0 + commission1
        );
    }

    /**
     * @dev Add liquidity to an existing pool
     */
    function addLiquidity(
        address token0,
        address token1,
        uint24 fee,
        int24 tickLower,
        int24 tickUpper,
        uint256 amount0Desired,
        uint256 amount1Desired,
        uint256 amount0Min,
        uint256 amount1Min,
        uint256 deadline
    ) external nonReentrant returns (uint256 tokenId) {
        require(deadline >= block.timestamp, "Deadline expired");
        
        address pool = uniswapFactory.getPool(token0, token1, fee);
        require(pool != address(0), "Pool does not exist");

        // Calculate and collect commission
        uint256 commission0 = _calculateCommission(amount0Desired);
        uint256 commission1 = _calculateCommission(amount1Desired);

        // Transfer tokens including commission
        IERC20(token0).safeTransferFrom(msg.sender, address(this), amount0Desired + commission0);
        IERC20(token1).safeTransferFrom(msg.sender, address(this), amount1Desired + commission1);

        // Collect commission
        IERC20(token0).safeTransfer(commissionRecipient, commission0);
        IERC20(token1).safeTransfer(commissionRecipient, commission1);

        // Approve and mint position
        IERC20(token0).approve(address(positionManager), 0);
        IERC20(token0).approve(address(positionManager), amount0Desired);
        IERC20(token1).approve(address(positionManager), 0);
        IERC20(token1).approve(address(positionManager), amount1Desired);

        INonfungiblePositionManager.MintParams memory mintParams = INonfungiblePositionManager.MintParams({
            token0: token0,
            token1: token1,
            fee: fee,
            tickLower: tickLower,
            tickUpper: tickUpper,
            amount0Desired: amount0Desired,
            amount1Desired: amount1Desired,
            amount0Min: amount0Min,
            amount1Min: amount1Min,
            recipient: msg.sender,
            deadline: deadline
        });

        uint256 amount0;
        uint256 amount1;
        (tokenId, , amount0, amount1) = positionManager.mint(mintParams);

        // Refund unused tokens
        if (amount0Desired > amount0) {
            IERC20(token0).safeTransfer(msg.sender, amount0Desired - amount0);
        }
        if (amount1Desired > amount1) {
            IERC20(token1).safeTransfer(msg.sender, amount1Desired - amount1);
        }

        emit LiquidityAdded(pool, msg.sender, tokenId, 0, amount0, amount1, commission0 + commission1);
    }

    // View functions
    function getAllPools() external view returns (address[] memory) {
        return allPools;
    }

    function getPoolCount() external view returns (uint256) {
        return allPools.length;
    }

    function isPoolCreatedByFactory(address pool) external view returns (bool) {
        return poolInfo[pool].pool != address(0);
    }

    // Internal functions
    function _calculateCommission(uint256 amount) internal pure returns (uint256) {
        return (amount * COMMISSION_BASIS_POINTS) / BASIS_POINTS_DENOMINATOR;
    }

    function _isValidFeeTier(uint24 fee) internal view returns (bool) {
        for (uint256 i = 0; i < supportedFeeTiers.length; i++) {
            if (supportedFeeTiers[i] == fee) {
                return true;
            }
        }
        return false;
    }

    // Admin functions
    function setCommissionRecipient(address _newRecipient) external onlyOwner {
        require(_newRecipient != address(0), "Invalid recipient");
        commissionRecipient = _newRecipient;
    }

    function addFeeTier(uint24 _fee) external onlyOwner {
        require(!_isValidFeeTier(_fee), "Fee tier already supported");
        supportedFeeTiers.push(_fee);
    }

    function removeFeeTier(uint24 _fee) external onlyOwner {
        for (uint256 i = 0; i < supportedFeeTiers.length; i++) {
            if (supportedFeeTiers[i] == _fee) {
                supportedFeeTiers[i] = supportedFeeTiers[supportedFeeTiers.length - 1];
                supportedFeeTiers.pop();
                break;
            }
        }
    }

    // Emergency functions
    function emergencyWithdraw(address token, uint256 amount) external onlyOwner {
        IERC20(token).safeTransfer(owner(), amount);
    }
}
