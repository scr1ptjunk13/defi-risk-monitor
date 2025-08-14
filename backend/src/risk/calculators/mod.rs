// Protocol-specific risk calculators module
pub mod lido;
pub mod generic;
pub mod beefy;

// Re-export all calculators
pub use lido::LidoRiskCalculator;
pub use generic::GenericRiskCalculator;
pub use beefy::BeefyRiskCalculator;
