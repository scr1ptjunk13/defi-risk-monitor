// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "forge-std/Script.sol";
import "../src/StandaloneLiquidityFactory.sol";

contract DeployStandalone is Script {
    function run() external {
        uint256 deployerPrivateKey = vm.envUint("PRIVATE_KEY");
        address commissionRecipient = vm.envAddress("COMMISSION_RECIPIENT");
        
        vm.startBroadcast(deployerPrivateKey);
        
        StandaloneLiquidityFactory factory = new StandaloneLiquidityFactory(commissionRecipient);
        
        console.log("StandaloneLiquidityFactory deployed to:", address(factory));
        console.log("Commission recipient:", commissionRecipient);
        console.log("Commission rate:", factory.COMMISSION_RATE(), "basis points (0.3%)");
        
        vm.stopBroadcast();
    }
}
