// Only include services that actually exist
pub mod blockchain_service;
pub mod contract_bindings;
pub mod risk_calculator;
pub mod price_service;
pub mod position_aggregator;

// Re-export the services
pub use blockchain_service::{BlockchainService, PriceStorageService};
pub use contract_bindings::{UniswapV3Pool, ERC20Token};
pub use risk_calculator::RiskCalculator;
pub use price_service::*;
pub use position_aggregator::*;
