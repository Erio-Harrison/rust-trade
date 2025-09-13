pub mod engine;
pub mod metrics;
pub mod portfolio;
pub mod strategy;

pub use engine::{BacktestConfig, BacktestEngine, BacktestResult};
pub use portfolio::{Portfolio, Position, Trade};
pub use strategy::{create_strategy, list_strategies, Signal, Strategy, StrategyInfo};
