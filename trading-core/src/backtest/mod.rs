pub mod engine;
pub mod portfolio;
pub mod metrics;
pub mod strategy;

pub use engine::{BacktestEngine, BacktestResult, BacktestConfig};
pub use portfolio::{Portfolio, Position, Trade};
pub use strategy::{Strategy, Signal, create_strategy, list_strategies, StrategyInfo};