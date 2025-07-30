// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "forge-std/console.sol";
import "../src/LiquidityPoolFactory.sol";
import "./mocks/MockERC20.sol";
import "@uniswap/v3-core/contracts/interfaces/IUniswapV3Factory.sol";
import "@uniswap/v3-core/contracts/interfaces/IUniswapV3Pool.sol";
import "@uniswap/v3-periphery/contracts/interfaces/INonfungiblePositionManager.sol";

contract LiquidityPoolFactoryTest is Test {
    LiquidityPoolFactory public factory;
    MockERC20 public tokenA;
    MockERC20 public tokenB;
    
    // Mainnet addresses for forking
    address constant UNISWAP_V3_FACTORY = 0x1F98431c8aD98523631AE4a59f267346ea31F984;
    address constant POSITION_MANAGER = 0xC36442b4a4522E871399CD717aBDD847Ab11FE88;
    
    address public owner;
    address public user1;
    address public user2;
    address public commissionRecipient;
    
    uint256 constant INITIAL_SUPPLY = 1000000 * 1e18;
    uint256 constant LIQUIDITY_AMOUNT = 1000 * 1e18;
    
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

    function setUp() public {
        // Fork mainnet for realistic testing
        vm.createFork(vm.envString("MAINNET_RPC_URL"));
        
        owner = address(this);
        user1 = makeAddr("user1");
        user2 = makeAddr("user2");
        commissionRecipient = makeAddr("commissionRecipient");
        
        // Deploy mock tokens
        tokenA = new MockERC20("Token A", "TKNA", 18, INITIAL_SUPPLY);
        tokenB = new MockERC20("Token B", "TKNB", 18, INITIAL_SUPPLY);
        
        // Ensure tokenA < tokenB for Uniswap ordering
        if (address(tokenA) > address(tokenB)) {
            (tokenA, tokenB) = (tokenB, tokenA);
        }
        
        // Deploy factory
        factory = new LiquidityPoolFactory(
            address(this),
            UNISWAP_V3_FACTORY,
            POSITION_MANAGER,
            commissionRecipient
        );
        
        // Setup user balances
        tokenA.transfer(user1, LIQUIDITY_AMOUNT * 10);
        tokenB.transfer(user1, LIQUIDITY_AMOUNT * 10);
        tokenA.transfer(user2, LIQUIDITY_AMOUNT * 10);
        tokenB.transfer(user2, LIQUIDITY_AMOUNT * 10);
    }
    
    function testConstructor() public {
        assertEq(address(factory.uniswapFactory()), UNISWAP_V3_FACTORY);
        assertEq(address(factory.positionManager()), POSITION_MANAGER);
        assertEq(factory.commissionRecipient(), commissionRecipient);
        assertEq(factory.COMMISSION_BASIS_POINTS(), 30);
    }
    
    function testCreatePoolAndAddLiquidity() public {
        vm.startPrank(user1);
        
        // Approve tokens
        uint256 amount0 = LIQUIDITY_AMOUNT;
        uint256 amount1 = LIQUIDITY_AMOUNT;
        uint256 commission0 = (amount0 * 30) / 10000;
        uint256 commission1 = (amount1 * 30) / 10000;
        
        tokenA.approve(address(factory), amount0 + commission0);
        tokenB.approve(address(factory), amount1 + commission1);
        
        // Calculate sqrt price for 1:1 ratio
        uint160 sqrtPriceX96 = 79228162514264337593543950336; // sqrt(1) * 2^96
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 3000,
            sqrtPriceX96: sqrtPriceX96,
            amount0Desired: amount0,
            amount1Desired: amount1,
            amount0Min: amount0 * 95 / 100,
            amount1Min: amount1 * 95 / 100,
            tickLower: -887220, // Full range
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        // Expect events
        vm.expectEmit(true, true, true, false);
        emit PoolCreated(
            address(tokenA),
            address(tokenB),
            3000,
            address(0), // Pool address will be different
            user1,
            0 // Commission amount will be calculated
        );
        
        (address pool, uint256 tokenId) = factory.createPoolAndAddLiquidity(params);
        
        // Verify pool was created
        assertTrue(pool != address(0));
        assertTrue(tokenId > 0);
        
        // Verify pool info
        (address poolAddr, address creator, uint256 createdAt, uint256 totalCommission) = factory.poolInfo(pool);
        assertEq(poolAddr, pool);
        assertEq(creator, user1);
        assertGt(createdAt, 0);
        assertGt(totalCommission, 0);
        
        // Verify commission was collected
        assertGt(tokenA.balanceOf(commissionRecipient), 0);
        assertGt(tokenB.balanceOf(commissionRecipient), 0);
        
        vm.stopPrank();
    }
    
    function testAddLiquidityToExistingPool() public {
        // First create a pool
        testCreatePoolAndAddLiquidity();
        
        vm.startPrank(user2);
        
        uint256 amount0 = LIQUIDITY_AMOUNT / 2;
        uint256 amount1 = LIQUIDITY_AMOUNT / 2;
        uint256 commission0 = (amount0 * 30) / 10000;
        uint256 commission1 = (amount1 * 30) / 10000;
        
        tokenA.approve(address(factory), amount0 + commission0);
        tokenB.approve(address(factory), amount1 + commission1);
        
        uint256 tokenId = factory.addLiquidity(
            address(tokenA),
            address(tokenB),
            3000,
            -887220,
            887220,
            amount0,
            amount1,
            amount0 * 95 / 100,
            amount1 * 95 / 100,
            block.timestamp + 300
        );
        
        assertTrue(tokenId > 0);
        
        vm.stopPrank();
    }
    
    function testCommissionCalculation() public {
        uint256 amount = 1000 * 1e18;
        uint256 expectedCommission = (amount * 30) / 10000; // 0.3%
        
        // We can't directly test the internal function, but we can verify through pool creation
        vm.startPrank(user1);
        
        tokenA.approve(address(factory), amount + expectedCommission);
        tokenB.approve(address(factory), amount + expectedCommission);
        
        uint256 initialBalance = tokenA.balanceOf(commissionRecipient);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 3000,
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: amount,
            amount1Desired: amount,
            amount0Min: amount * 95 / 100,
            amount1Min: amount * 95 / 100,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        factory.createPoolAndAddLiquidity(params);
        
        uint256 finalBalance = tokenA.balanceOf(commissionRecipient);
        assertEq(finalBalance - initialBalance, expectedCommission);
        
        vm.stopPrank();
    }
    
    function testFailInvalidFeeTier() public {
        vm.startPrank(user1);
        
        tokenA.approve(address(factory), LIQUIDITY_AMOUNT * 2);
        tokenB.approve(address(factory), LIQUIDITY_AMOUNT * 2);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 1500, // Invalid fee tier
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: LIQUIDITY_AMOUNT,
            amount1Desired: LIQUIDITY_AMOUNT,
            amount0Min: LIQUIDITY_AMOUNT * 95 / 100,
            amount1Min: LIQUIDITY_AMOUNT * 95 / 100,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        // This should revert
        factory.createPoolAndAddLiquidity(params);
        
        vm.stopPrank();
    }
    
    function testFailExpiredDeadline() public {
        vm.startPrank(user1);
        
        tokenA.approve(address(factory), LIQUIDITY_AMOUNT * 2);
        tokenB.approve(address(factory), LIQUIDITY_AMOUNT * 2);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 3000,
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: LIQUIDITY_AMOUNT,
            amount1Desired: LIQUIDITY_AMOUNT,
            amount0Min: LIQUIDITY_AMOUNT * 95 / 100,
            amount1Min: LIQUIDITY_AMOUNT * 95 / 100,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp - 1 // Expired deadline
        });
        
        // This should revert
        factory.createPoolAndAddLiquidity(params);
        
        vm.stopPrank();
    }
    
    function testFailIdenticalTokens() public {
        vm.startPrank(user1);
        
        tokenA.approve(address(factory), LIQUIDITY_AMOUNT * 2);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenA), // Same token
            fee: 3000,
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: LIQUIDITY_AMOUNT,
            amount1Desired: LIQUIDITY_AMOUNT,
            amount0Min: LIQUIDITY_AMOUNT * 95 / 100,
            amount1Min: LIQUIDITY_AMOUNT * 95 / 100,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        // This should revert
        factory.createPoolAndAddLiquidity(params);
        
        vm.stopPrank();
    }
    
    function testGetAllPools() public {
        // Initially no pools
        address[] memory pools = factory.getAllPools();
        assertEq(pools.length, 0);
        
        // Create a pool
        testCreatePoolAndAddLiquidity();
        
        // Should have one pool now
        pools = factory.getAllPools();
        assertEq(pools.length, 1);
        assertEq(factory.getPoolCount(), 1);
    }
    
    function testAdminFunctions() public {
        // Test setting commission recipient
        address newRecipient = makeAddr("newRecipient");
        factory.setCommissionRecipient(newRecipient);
        assertEq(factory.commissionRecipient(), newRecipient);
        
        // Test adding fee tier
        factory.addFeeTier(100);
        // We can't directly test the internal array, but we can test by creating a pool
        
        // Test removing fee tier
        factory.removeFeeTier(500);
        // The 500 fee tier should no longer be valid
    }
    
    function testFailNonOwnerAdminFunctions() public {
        vm.startPrank(user1);
        
        // This should revert
        factory.setCommissionRecipient(user1);
        
        vm.stopPrank();
    }
    
    function testEmergencyWithdraw() public {
        // Send some tokens to the contract
        tokenA.transfer(address(factory), 1000);
        
        uint256 initialBalance = tokenA.balanceOf(owner);
        factory.emergencyWithdraw(address(tokenA), 1000);
        
        assertEq(tokenA.balanceOf(owner) - initialBalance, 1000);
    }
    
    function testFuzzCreatePool(uint256 amount0, uint256 amount1) public {
        // Bound the amounts to reasonable values
        amount0 = bound(amount0, 1e15, 1e21); // 0.001 to 1000 tokens
        amount1 = bound(amount1, 1e15, 1e21);
        
        vm.startPrank(user1);
        
        // Mint enough tokens
        tokenA.mint(user1, amount0 * 2);
        tokenB.mint(user1, amount1 * 2);
        
        uint256 commission0 = (amount0 * 30) / 10000;
        uint256 commission1 = (amount1 * 30) / 10000;
        
        tokenA.approve(address(factory), amount0 + commission0);
        tokenB.approve(address(factory), amount1 + commission1);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 3000,
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: amount0,
            amount1Desired: amount1,
            amount0Min: 0, // Accept any amount for fuzz testing
            amount1Min: 0,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        (address pool, uint256 tokenId) = factory.createPoolAndAddLiquidity(params);
        
        assertTrue(pool != address(0));
        assertTrue(tokenId > 0);
        
        vm.stopPrank();
    }
    
    function testInvariantCommissionAlwaysCollected() public {
        vm.startPrank(user1);
        
        uint256 amount0 = LIQUIDITY_AMOUNT;
        uint256 amount1 = LIQUIDITY_AMOUNT;
        uint256 commission0 = (amount0 * 30) / 10000;
        uint256 commission1 = (amount1 * 30) / 10000;
        
        tokenA.approve(address(factory), amount0 + commission0);
        tokenB.approve(address(factory), amount1 + commission1);
        
        uint256 initialCommissionBalance0 = tokenA.balanceOf(commissionRecipient);
        uint256 initialCommissionBalance1 = tokenB.balanceOf(commissionRecipient);
        
        LiquidityPoolFactory.PoolCreationParams memory params = LiquidityPoolFactory.PoolCreationParams({
            token0: address(tokenA),
            token1: address(tokenB),
            fee: 3000,
            sqrtPriceX96: 79228162514264337593543950336,
            amount0Desired: amount0,
            amount1Desired: amount1,
            amount0Min: amount0 * 95 / 100,
            amount1Min: amount1 * 95 / 100,
            tickLower: -887220,
            tickUpper: 887220,
            deadline: block.timestamp + 300
        });
        
        factory.createPoolAndAddLiquidity(params);
        
        // Commission should always be collected
        assertEq(tokenA.balanceOf(commissionRecipient) - initialCommissionBalance0, commission0);
        assertEq(tokenB.balanceOf(commissionRecipient) - initialCommissionBalance1, commission1);
        
        vm.stopPrank();
    }
}
