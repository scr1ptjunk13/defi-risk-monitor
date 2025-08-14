// Modular Risk Architecture Module
// Provides protocol-specific risk calculators and orchestration

pub mod traits;
pub mod metrics;
pub mod orchestrator;
pub mod errors;
pub mod calculators;



// Re-export main types
pub use traits::*;
pub use metrics::*;
pub use orchestrator::*;
pub use errors::*;
pub use calculators::*;

// Module version
pub const VERSION: &str = "1.0.0";

// Risk calculation constants
pub const DEFAULT_RISK_SCORE: f64 = 50.0;
pub const MAX_RISK_SCORE: f64 = 100.0;
pub const MIN_RISK_SCORE: f64 = 0.0;
