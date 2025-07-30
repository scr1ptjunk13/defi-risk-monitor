// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/LiquidityPoolFactory.sol";

contract DeployScript is Script {
    // Mainnet addresses
    address constant UNISWAP_V3_FACTORY = 0x1F98431c8aD98523631AE4a59f267346ea31F984;
    address constant POSITION_MANAGER = 0xC36442b4a4522E871399CD717aBDD847Ab11FE88;
    
    // Sepolia testnet addresses
    address constant SEPOLIA_UNISWAP_V3_FACTORY = 0x0227628f3F023bb0B980b67D528571c95c6DaC1c;
    address constant SEPOLIA_POSITION_MANAGER = 0x1238536071E1c677A632429e3655c799b22cDA52;

    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address commissionRecipient = vm.envAddress("COMMISSION_RECIPIENT");
        
        vm.startBroadcast(deployerPrivateKey);

        // Determine which network we're on
        uint256 chainId = block.chainid;
        address factory;
        address positionManager;
        
        if (chainId == 1) {
            // Mainnet
            factory = UNISWAP_V3_FACTORY;
            positionManager = POSITION_MANAGER;
        } else if (chainId == 11155111) {
            // Sepolia
            factory = SEPOLIA_UNISWAP_V3_FACTORY;
            positionManager = SEPOLIA_POSITION_MANAGER;
        } else {
            revert("Unsupported network");
        }

        LiquidityPoolFactory liquidityFactory = new LiquidityPoolFactory(
            address(this),
            factory,
            positionManager,
            commissionRecipient
        );

        console.log("LiquidityPoolFactory deployed to:", address(liquidityFactory));
        console.log("Chain ID:", chainId);
        console.log("Uniswap Factory:", factory);
        console.log("Position Manager:", positionManager);
        console.log("Commission Recipient:", commissionRecipient);

        vm.stopBroadcast();
    }
}
