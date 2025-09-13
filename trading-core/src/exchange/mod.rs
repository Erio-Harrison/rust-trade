// exchange/mod.rs
pub mod binance;
pub mod errors;
pub mod traits;
pub mod types;
pub mod utils;

// Re-export main interfaces for easy access
pub use binance::BinanceExchange;
pub use errors::ExchangeError;
pub use traits::Exchange;
pub use types::*;
