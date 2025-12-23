// trading-core library: CLI-specific modules
// Shared types are in trading-common crate

pub mod config;
pub mod exchange;
pub mod live_trading;
pub mod service;

// Re-export trading-common for convenience
pub use trading_common::{backtest, data};
