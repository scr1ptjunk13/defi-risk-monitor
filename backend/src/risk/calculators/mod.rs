// Protocol-specific risk calculators module
pub mod lido;
pub mod generic;
pub mod beefy;
pub mod rocketpool;
pub mod etherfi;
pub mod yearnfinance;
pub mod aave_v3;

// Re-export all calculators
pub use lido::LidoRiskCalculator;
pub use generic::GenericRiskCalculator;
pub use beefy::BeefyRiskCalculator;
pub use rocketpool::RocketPoolRiskCalculator;
pub use etherfi::EtherFiRiskCalculator;
pub use yearnfinance::YearnFinanceRiskCalculator;
pub use aave_v3::AaveV3RiskCalculator;
