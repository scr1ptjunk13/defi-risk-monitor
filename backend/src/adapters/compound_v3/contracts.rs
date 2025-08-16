// Compound V3 (Comet) contract interfaces and data structures
use alloy::{
    primitives::{Address, U256},
    sol,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Complete Compound V3 (Comet) contract interfaces using Alloy's sol! macro
sol! {
    #[sol(rpc)]
    interface IComet {
        struct AssetInfo {
            uint8 offset;
            address asset;
            address priceFeed;
            uint64 scale;
            uint64 borrowCollateralFactor;
            uint64 liquidateCollateralFactor;
            uint64 liquidationFactor;
            uint128 supplyCap;
        }

        struct Configuration {
            address governor;
            address pauseGuardian;
            address baseToken;
            address baseTokenPriceFeed;
            address extensionDelegate;
            uint64 supplyKink;
            uint64 supplyPerSecondInterestRateSlopeLow;
            uint64 supplyPerSecondInterestRateSlope;
            uint64 supplyPerSecondInterestRateBase;
            uint64 borrowKink;
            uint64 borrowPerSecondInterestRateSlopeLow;
            uint64 borrowPerSecondInterestRateSlope;
            uint64 borrowPerSecondInterestRateBase;
            uint64 storeFrontPriceFactor;
            uint64 trackingIndexScale;
            uint64 baseTrackingSupplySpeed;
            uint64 baseTrackingBorrowSpeed;
            uint104 baseMinForRewards;
            uint104 baseBorrowMin;
            uint104 targetReserves;
            AssetInfo[] assetConfigs;
        }

        struct UserBasic {
            int104 principal;
            uint64 baseTrackingIndex;
            uint64 baseTrackingAccrued;
            uint16 assetsIn;
            uint8 _reserved;
        }

        struct UserCollateral {
            uint128 balance;
            uint128 _reserved;
        }

        function baseToken() external view returns (address);
        function baseTokenPriceFeed() external view returns (address);
        function getConfiguration() external view returns (Configuration memory);
        
        function userBasic(address account) external view returns (UserBasic memory);
        function userCollateral(address account, address asset) external view returns (UserCollateral memory);
        
        function getAssetInfo(uint8 i) external view returns (AssetInfo memory);
        function getAssetInfoByAddress(address asset) external view returns (AssetInfo memory);
        
        function getSupplyRate(uint256 utilization) external view returns (uint64);
        function getBorrowRate(uint256 utilization) external view returns (uint64);
        function getUtilization() external view returns (uint256);
        
        function getPrice(address priceFeed) external view returns (uint256);
        function getReserves() external view returns (int256);
        function totalSupply() external view returns (uint256);
        function totalBorrow() external view returns (uint256);
        
        function balanceOf(address account) external view returns (uint256);
        function borrowBalanceOf(address account) external view returns (uint256);
        
        function getCollateralReserves(address asset) external view returns (uint256);
        function isLiquidatable(address account) external view returns (bool);
        
        function accrueAccount(address account) external;
        function getAccountLiquidity(address account) external view returns (int256);
        function getAccountBorrowCapacity(address account) external view returns (uint256);
    }

    #[sol(rpc)]
    interface ICometRewards {
        struct RewardConfig {
            address token;
            uint64 rescaleFactor;
            bool shouldUpscale;
        }
        
        struct RewardOwed {
            address token;
            uint256 owed;
        }
        
        function getRewardOwed(address comet, address account) external returns (RewardOwed memory);
        function claim(address comet, address src, bool shouldAccrue) external;
        function claimTo(address comet, address src, address to, bool shouldAccrue) external;
        function rewardConfig(address comet) external view returns (RewardConfig memory);
    }

    #[sol(rpc)]
    interface IERC20Metadata {
        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }
}

// Data structures for Compound operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundMarketInfo {
    pub market_address: Address,
    pub base_token: Address,
    pub base_token_symbol: String,
    pub base_token_decimals: u8,
    pub base_token_price_feed: Address,
    pub base_token_price: f64,
    pub total_supply: U256,
    pub total_borrow: U256,
    pub utilization: f64,
    pub supply_apy: f64,
    pub borrow_apy: f64,
    pub reserves: i128,
    pub supply_cap: Option<U256>,
    pub borrow_min: U256,
    pub collateral_assets: Vec<CompoundCollateralAsset>,
    pub target_reserves: U256,
    pub rewards_info: Option<CompoundRewardsInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundCollateralAsset {
    pub asset: Address,
    pub asset_symbol: String,
    pub asset_decimals: u8,
    pub price_feed: Address,
    pub borrow_collateral_factor: f64,
    pub liquidate_collateral_factor: f64,
    pub liquidation_factor: f64,
    pub supply_cap: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundRewardsInfo {
    pub token: Address,
    pub token_symbol: String,
    pub rescale_factor: u64,
    pub should_upscale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundUserPosition {
    pub market: CompoundMarketInfo,
    pub base_balance: i128,
    pub base_balance_usd: f64,
    pub collateral_positions: HashMap<Address, CompoundCollateralPosition>,
    pub total_collateral_value_usd: f64,
    pub borrow_capacity_usd: f64,
    pub liquidation_threshold_usd: f64,
    pub account_liquidity: i128,
    pub is_liquidatable: bool,
    pub health_factor: f64,
    pub net_apy: f64,
    pub pending_rewards: Vec<CompoundPendingReward>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundCollateralPosition {
    pub asset: CompoundCollateralAsset,
    pub balance: u128,
    pub balance_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundPendingReward {
    pub token: Address,
    pub token_symbol: String,
    pub amount: U256,
    pub amount_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompoundAccountSummary {
    pub positions: Vec<CompoundUserPosition>,
    pub total_supplied_usd: f64,
    pub total_borrowed_usd: f64,
    pub total_collateral_usd: f64,
    pub net_worth_usd: f64,
    pub total_borrow_capacity_usd: f64,
    pub utilization_percentage: f64,
    pub overall_health_factor: f64,
    pub is_liquidatable: bool,
    pub total_pending_rewards_usd: f64,
}

// Cached data structures
#[derive(Debug, Clone)]
pub struct CachedMarketData {
    pub markets: HashMap<Address, CompoundMarketInfo>,
    pub cached_at: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct CachedUserPositions {
    pub account_summary: CompoundAccountSummary,
    pub cached_at: std::time::SystemTime,
}
