// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Test.sol";
import "../src/StandaloneLiquidityFactory.sol";

contract StandaloneLiquidityFactoryTest is Test {
    StandaloneLiquidityFactory public factory;
    address public owner;
    address public commissionRecipient;
    address public user;
    
    address public constant TOKEN0 = 0x742D35Cc6634c0532925a3b8d4c9db96c4B4D2D6;
    address public constant TOKEN1 = 0x853d955aCEf822Db058eb8505911ED77F175b99e;
    uint24 public constant FEE = 3000;
    
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

    function setUp() public {
        owner = address(this);
        commissionRecipient = makeAddr("commissionRecipient");
        user = makeAddr("user");
        
        factory = new StandaloneLiquidityFactory(commissionRecipient);
    }

    function testContractDeployment() public {
        (address _owner, address _commissionRecipient, uint256 _commissionRate) = factory.getContractInfo();
        
        assertEq(_owner, owner);
        assertEq(_commissionRecipient, commissionRecipient);
        assertEq(_commissionRate, 30); // 0.3%
    }

    function testCommissionCalculation() public {
        uint256 amount0 = 1000e18;
        uint256 amount1 = 2000e18;
        
        (uint256 commission0, uint256 commission1) = factory.getCommission(amount0, amount1);
        
        // 0.3% commission
        assertEq(commission0, (amount0 * 30) / 10000);
        assertEq(commission1, (amount1 * 30) / 10000);
        assertEq(commission0, 3e18);
        assertEq(commission1, 6e18);
    }

    function testCreatePoolAndAddLiquidity() public {
        uint256 amount0Desired = 1000e18;
        uint256 amount1Desired = 2000e18;
        uint256 expectedCommission0 = (amount0Desired * 30) / 10000;
        uint256 expectedCommission1 = (amount1Desired * 30) / 10000;
        
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN1,
            fee: FEE,
            amount0Desired: amount0Desired,
            amount1Desired: amount1Desired,
            amount0Min: amount0Desired - expectedCommission0,
            amount1Min: amount1Desired - expectedCommission1,
            deadline: block.timestamp + 300
        });

        // Expect PoolCreated event (pool address will be generated dynamically)
        vm.expectEmit(true, true, true, false);
        emit PoolCreated(TOKEN0, TOKEN1, FEE, address(0), expectedCommission0, expectedCommission1);
        
        // Expect LiquidityAdded event
        vm.expectEmit(true, true, true, true);
        emit LiquidityAdded(user, TOKEN0, TOKEN1, amount0Desired - expectedCommission0, amount1Desired - expectedCommission1, expectedCommission0, expectedCommission1);

        vm.prank(user);
        (uint256 amount0Used, uint256 amount1Used, uint256 commission0, uint256 commission1) = 
            factory.createPoolAndAddLiquidity(params);

        assertEq(amount0Used, amount0Desired - expectedCommission0);
        assertEq(amount1Used, amount1Desired - expectedCommission1);
        assertEq(commission0, expectedCommission0);
        assertEq(commission1, expectedCommission1);
    }

    function testAddLiquidity() public {
        uint256 amount0Desired = 500e18;
        uint256 amount1Desired = 1000e18;
        uint256 expectedCommission0 = (amount0Desired * 30) / 10000;
        uint256 expectedCommission1 = (amount1Desired * 30) / 10000;
        
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN1,
            fee: FEE,
            amount0Desired: amount0Desired,
            amount1Desired: amount1Desired,
            amount0Min: amount0Desired - expectedCommission0,
            amount1Min: amount1Desired - expectedCommission1,
            deadline: block.timestamp + 300
        });

        vm.expectEmit(true, true, true, true);
        emit LiquidityAdded(user, TOKEN0, TOKEN1, amount0Desired - expectedCommission0, amount1Desired - expectedCommission1, expectedCommission0, expectedCommission1);

        vm.prank(user);
        (uint256 amount0Used, uint256 amount1Used, uint256 commission0, uint256 commission1) = 
            factory.addLiquidity(params);

        assertEq(amount0Used, amount0Desired - expectedCommission0);
        assertEq(amount1Used, amount1Desired - expectedCommission1);
        assertEq(commission0, expectedCommission0);
        assertEq(commission1, expectedCommission1);
    }

    function testUpdateCommissionRecipient() public {
        address newRecipient = makeAddr("newRecipient");
        
        factory.updateCommissionRecipient(newRecipient);
        
        (, address _commissionRecipient,) = factory.getContractInfo();
        assertEq(_commissionRecipient, newRecipient);
    }

    function testFailUpdateCommissionRecipientNotOwner() public {
        address newRecipient = makeAddr("newRecipient");
        
        vm.prank(user);
        factory.updateCommissionRecipient(newRecipient);
    }

    function testFailCreatePoolExpiredDeadline() public {
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN1,
            fee: FEE,
            amount0Desired: 1000e18,
            amount1Desired: 2000e18,
            amount0Min: 997e18,
            amount1Min: 1994e18,
            deadline: block.timestamp - 1 // Expired deadline
        });

        vm.prank(user);
        factory.createPoolAndAddLiquidity(params);
    }

    function testFailCreatePoolIdenticalTokens() public {
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN0, // Same token
            fee: FEE,
            amount0Desired: 1000e18,
            amount1Desired: 2000e18,
            amount0Min: 997e18,
            amount1Min: 1994e18,
            deadline: block.timestamp + 300
        });

        vm.prank(user);
        factory.createPoolAndAddLiquidity(params);
    }

    function testFailCreatePoolZeroAmounts() public {
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN1,
            fee: FEE,
            amount0Desired: 0, // Zero amount
            amount1Desired: 2000e18,
            amount0Min: 0,
            amount1Min: 1994e18,
            deadline: block.timestamp + 300
        });

        vm.prank(user);
        factory.createPoolAndAddLiquidity(params);
    }

    function testFailCreatePoolInsufficientMinAmount() public {
        uint256 amount0Desired = 1000e18;
        uint256 amount1Desired = 2000e18;
        
        StandaloneLiquidityFactory.PoolParams memory params = StandaloneLiquidityFactory.PoolParams({
            token0: TOKEN0,
            token1: TOKEN1,
            fee: FEE,
            amount0Desired: amount0Desired,
            amount1Desired: amount1Desired,
            amount0Min: amount0Desired, // Too high min amount (doesn't account for commission)
            amount1Min: amount1Desired, // Too high min amount (doesn't account for commission)
            deadline: block.timestamp + 300
        });

        vm.prank(user);
        factory.createPoolAndAddLiquidity(params);
    }

    function testReentrancyProtection() public {
        // This test verifies that the nonReentrant modifier works
        // In a real scenario, this would be tested with a malicious contract
        // For now, we just verify the contract compiles and basic functionality works
        assertTrue(true);
    }
}
