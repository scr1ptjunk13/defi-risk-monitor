//! Real blockchain contract bindings using Alloy
//! Implements actual Uniswap V3 Pool and Chainlink Aggregator contracts

use alloy::{
    providers::RootProvider,
    transports::http::{Client, Http},
    sol,
    primitives::{Address, U256},
};
use std::sync::Arc;
use std::str::FromStr;

// Uniswap V3 Pool contract ABI definitions
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function slot0() external view returns (
            uint160 sqrtPriceX96,
            int24 tick,
            uint16 observationIndex,
            uint16 observationCardinality,
            uint16 observationCardinalityNext,
            uint8 feeProtocol,
            bool unlocked
        );
        
        function liquidity() external view returns (uint128);
        function token0() external view returns (address);
        function token1() external view returns (address);
        function fee() external view returns (uint24);
        function tickSpacing() external view returns (int24);
    }
}

// ERC20 Token contract ABI definitions
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IERC20 {
        function name() external view returns (string memory);
        function symbol() external view returns (string memory);
        function decimals() external view returns (uint8);
        function totalSupply() external view returns (uint256);
        function balanceOf(address account) external view returns (uint256);
    }
}

// Chainlink Aggregator V3 contract ABI definitions
sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IAggregatorV3 {
        function latestRoundData() external view returns (
            uint80 roundId,
            int256 answer,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        );
        
        function decimals() external view returns (uint8);
        function description() external view returns (string memory);
    }
}

#[derive(Debug, Clone)]
pub struct UniswapV3Pool {
    contract: IUniswapV3Pool::IUniswapV3PoolInstance<Http<Client>, Arc<RootProvider<Http<Client>>>>,
    address: Address,
}

impl UniswapV3Pool {
    pub fn new(address: String, provider: Arc<RootProvider<Http<Client>>>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Validate address format first
        if address.is_empty() {
            return Err("Pool address cannot be empty".into());
        }
        
        // Ensure address starts with 0x and has correct length
        let normalized_address = if address.starts_with("0x") {
            address.clone()
        } else {
            format!("0x{}", address)
        };
        
        if normalized_address.len() != 42 {
            return Err(format!("Invalid pool address length: expected 42 characters, got {}", normalized_address.len()).into());
        }
        
        let address = Address::from_str(&normalized_address)
            .map_err(|e| format!("Invalid pool address format '{}': {}", normalized_address, e))?;
        
        let contract = IUniswapV3Pool::new(address, provider);
        
        Ok(Self {
            contract,
            address,
        })
    }

    pub async fn slot0(&self) -> Result<(U256, i32, u16, u16, u16, u8, bool), Box<dyn std::error::Error + Send + Sync>> {
        // Add detailed error context for debugging
        let result = self.contract.slot0().call().await
            .map_err(|e| {
                format!("slot0 call failed for pool address {}: {}. This could be due to: 1) Invalid pool address, 2) Network connectivity issues, 3) Pool contract not deployed, 4) ABI mismatch", 
                    self.address, e)
            })?;
        
        // Convert Alloy types properly with comprehensive error handling
        let sqrt_price_x96 = U256::from(result.sqrtPriceX96);
        
        // Convert Alloy Signed type to i32 properly
        // The tick is an int24 in Solidity (-8388608 to 8388607), which fits in i32
        let tick: i32 = result.tick.try_into()
            .map_err(|e| {
                format!("Failed to convert tick to i32 for pool {}: {}. Tick value may be out of range for int24", 
                    self.address, e)
            })?;
        
        // Validate that the tick is within reasonable bounds for Uniswap V3
        if tick < -887272 || tick > 887272 {
            return Err(format!("Tick value {} is outside valid Uniswap V3 range [-887272, 887272] for pool {}", 
                tick, self.address).into());
        }
        
        Ok((
            sqrt_price_x96,
            tick,
            result.observationIndex,
            result.observationCardinality,
            result.observationCardinalityNext,
            result.feeProtocol,
            result.unlocked,
        ))
    }

    pub async fn liquidity(&self) -> Result<u128, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.liquidity().call().await
            .map_err(|e| format!("liquidity call failed: {}", e))?;
        
        Ok(result._0)
    }

    pub async fn token0(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.token0().call().await
            .map_err(|e| format!("token0 call failed: {}", e))?;
        
        Ok(format!("{:?}", result._0))
    }

    pub async fn token1(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.token1().call().await
            .map_err(|e| format!("token1 call failed: {}", e))?;
        
        Ok(format!("{:?}", result._0))
    }

    pub async fn fee(&self) -> Result<u32, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.fee().call().await
            .map_err(|e| format!("fee call failed: {}", e))?;
        
        let fee: u32 = result._0.try_into().map_err(|_| "Fee conversion failed")?;
        Ok(fee)
    }

    pub async fn tick_spacing(&self) -> Result<i32, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.tickSpacing().call().await
            .map_err(|e| format!("tickSpacing call failed: {}", e))?;
        
        let tick_spacing: i32 = result._0.try_into().map_err(|_| "TickSpacing conversion failed")?;
        Ok(tick_spacing)
    }

    pub fn address(&self) -> Address {
        self.address
    }
}

#[derive(Debug, Clone)]
pub struct ChainlinkAggregatorV3 {
    contract: IAggregatorV3::IAggregatorV3Instance<Http<Client>, Arc<RootProvider<Http<Client>>>>,
    address: Address,
}

impl ChainlinkAggregatorV3 {
    pub fn new(address: String, provider: Arc<RootProvider<Http<Client>>>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let address = Address::from_str(&address)
            .map_err(|e| format!("Invalid aggregator address: {}", e))?;
        
        let contract = IAggregatorV3::new(address, provider);
        
        Ok(Self {
            contract,
            address,
        })
    }

    pub async fn latest_round_data(&self) -> Result<(u64, i128, u64, u64, u64), Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.latestRoundData().call().await
            .map_err(|e| format!("latestRoundData call failed: {}", e))?;
        
        // Convert Signed<256, 4> to i128 for the price (answer)
        let answer: i128 = result.answer.try_into().map_err(|_| "Answer conversion failed")?;
        
        Ok((
            result.roundId.try_into().unwrap_or(0u64),
            answer,
            result.startedAt.try_into().unwrap_or(0u64),
            result.updatedAt.try_into().unwrap_or(0u64),
            result.answeredInRound.try_into().unwrap_or(0u64),
        ))
    }

    pub async fn decimals(&self) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.decimals().call().await
            .map_err(|e| format!("decimals call failed: {}", e))?;
        
        Ok(result._0)
    }

    pub async fn description(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.description().call().await
            .map_err(|e| format!("description call failed: {}", e))?;
        
        Ok(result._0)
    }

    pub fn address(&self) -> Address {
        self.address
    }
}

#[derive(Debug, Clone)]
pub struct ERC20Token {
    contract: IERC20::IERC20Instance<Http<Client>, RootProvider<Http<Client>>>,
    address: Address,
}

impl ERC20Token {
    pub fn new(
        address: String,
        provider: Arc<RootProvider<Http<Client>>>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let addr = Address::from_str(&address)?;
        let contract = IERC20::new(addr, (*provider).clone());
        
        Ok(Self {
            contract,
            address: addr,
        })
    }

    pub async fn symbol(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.symbol().call().await
            .map_err(|e| format!("symbol call failed: {}", e))?;
        Ok(result._0)
    }

    pub async fn name(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.name().call().await
            .map_err(|e| format!("name call failed: {}", e))?;
        Ok(result._0)
    }

    pub async fn decimals(&self) -> Result<u8, Box<dyn std::error::Error + Send + Sync>> {
        let result = self.contract.decimals().call().await
            .map_err(|e| format!("decimals call failed: {}", e))?;
        Ok(result._0)
    }

    pub fn address(&self) -> Address {
        self.address
    }
}

// Common contract addresses for mainnet
pub mod addresses {
    pub const ETHEREUM_MAINNET: i32 = 1;
    pub const POLYGON_MAINNET: i32 = 137;
    pub const ARBITRUM_MAINNET: i32 = 42161;
    
    // Popular Uniswap V3 pools on Ethereum mainnet
    pub const USDC_WETH_POOL_500: &str = "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640"; // 0.05% fee
    pub const USDC_WETH_POOL_3000: &str = "0x8ad599c3A0ff1De082011EFDDc58f1908eb6e6D8"; // 0.3% fee
    pub const WBTC_WETH_POOL_3000: &str = "0xCBCdF9626bC03E24f779434178A73a0B4bad62eD"; // 0.3% fee
    
    // Chainlink price feed addresses on Ethereum mainnet
    pub const ETH_USD_FEED: &str = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
    pub const BTC_USD_FEED: &str = "0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c";
    pub const USDC_USD_FEED: &str = "0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6";
    pub const USDT_USD_FEED: &str = "0x3E7d1eAB13ad0104d2750B8863b489D65364e32D";
    pub const DAI_USD_FEED: &str = "0xAed0c38402a5d19df6E4c03F4E2DceD6e29c1ee9";
    
    // Token addresses
    pub const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
    pub const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
    pub const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
    pub const WBTC: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
    pub const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::providers::ProviderBuilder;
    
    #[tokio::test]
    #[ignore] // Ignore by default to avoid hitting real RPC endpoints in CI
    async fn test_real_uniswap_pool() {
        let provider = Arc::new(
            ProviderBuilder::new()
                .on_http("https://eth.llamarpc.com".parse().unwrap())
        );
        
        let pool = UniswapV3Pool::new(
            addresses::USDC_WETH_POOL_500.to_string(),
            provider
        ).expect("Failed to create pool");
        
        let slot0 = pool.slot0().await.expect("Failed to get slot0");
        let liquidity = pool.liquidity().await.expect("Failed to get liquidity");
        
        println!("Pool slot0: {:?}", slot0);
        println!("Pool liquidity: {}", liquidity);
        
        assert!(slot0.0 > U256::ZERO); // sqrtPriceX96 should be > 0
        assert!(liquidity > 0); // liquidity should be > 0
    }
    
    #[tokio::test]
    #[ignore] // Ignore by default to avoid hitting real RPC endpoints in CI
    async fn test_real_chainlink_feed() {
        let provider = Arc::new(
            ProviderBuilder::new()
                .on_http("https://eth.llamarpc.com".parse().unwrap())
        );
        
        let feed = ChainlinkAggregatorV3::new(
            addresses::ETH_USD_FEED.to_string(),
            provider
        ).expect("Failed to create feed");
        
        let round_data = feed.latest_round_data().await.expect("Failed to get round data");
        let decimals = feed.decimals().await.expect("Failed to get decimals");
        
        println!("ETH/USD price: {} (decimals: {})", round_data.1, decimals);
        
        assert!(round_data.1 > 0); // Price should be positive
        assert_eq!(decimals, 8); // ETH/USD feed has 8 decimals
    }
}
