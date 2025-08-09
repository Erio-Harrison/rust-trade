// exchange/mod.rs
pub mod traits;
pub mod types;
pub mod errors;
pub mod binance;
pub mod utils;

// Re-export main interfaces for easy access
pub use traits::Exchange;
pub use types::*;
pub use errors::ExchangeError;
pub use binance::BinanceExchange;