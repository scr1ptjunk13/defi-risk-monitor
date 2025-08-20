// Start with minimal working adapters only
pub mod traits;
pub mod uniswap_v3;
pub mod uniswap_v2;
pub mod lido;
pub mod rocketpool;
pub mod etherfi;
pub mod yearnfinance;
pub mod morphoblue;

// Export traits and working adapters
pub use traits::*;
pub use uniswap_v3::UniswapV3Adapter;
pub use uniswap_v2::UniswapV2Adapter;
pub use lido::LidoAdapter;
pub use rocketpool::RocketPoolAdapter;
pub use etherfi::EtherFiAdapter;
pub use yearnfinance::YearnAdapter;
pub use morphoblue::MorphoBlueAdapter;

// TODO: Fix and re-enable these adapters once Position struct fields are aligned:
// pub mod makerdao;
// pub mod balancer_v2;
// pub mod beefy;
// pub mod convexfinance;
// pub mod eigenlayer;
