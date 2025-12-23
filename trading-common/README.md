# Trading Common

Shared library providing core data structures, backtesting engine, and data access layer for the Rust Trade system.

## Overview

`trading-common` is the foundation library used by both `trading-core` (CLI) and `src-tauri` (desktop app). It contains all shared functionality that doesn't depend on specific runtime environments.

## Project Structure

```
trading-common/
├── src/
│   ├── lib.rs                 # Library entry point
│   ├── backtest/              # Backtesting system
│   │   ├── mod.rs             # Module exports and public interface
│   │   ├── engine.rs          # Core backtesting engine and execution logic
│   │   ├── portfolio.rs       # Portfolio management, position tracking, P&L calculation
│   │   ├── metrics.rs         # Performance metrics calculation (Sharpe, drawdown, etc.)
│   │   └── strategy/          # Trading strategies
│   │       ├── mod.rs         # Strategy factory and management
│   │       ├── base.rs        # Strategy trait definition
│   │       ├── sma.rs         # Simple Moving Average strategy
│   │       └── rsi.rs         # RSI strategy
│   └── data/                  # Data layer
│       ├── mod.rs             # Module exports
│       ├── types.rs           # Core data types (TickData, OHLC, errors)
│       ├── repository.rs      # Database operations and query logic
│       └── cache.rs           # Multi-level caching (L1 memory + L2 Redis)
└── Cargo.toml
```

## Modules

### `backtest/` - Backtesting Engine

Complete backtesting system for strategy evaluation:

- **`engine.rs`** - Core backtesting logic that processes historical data
- **`metrics.rs`** - Performance metrics calculation (Sharpe ratio, max drawdown, win rate, etc.)
- **`portfolio.rs`** - Portfolio management and P&L tracking
- **`strategy/`** - Trading strategy implementations
  - `sma.rs` - Simple Moving Average crossover strategy
  - `rsi.rs` - Relative Strength Index strategy

### `data/` - Data Layer

Data access and caching infrastructure:

- **`types.rs`** - Core data structures (`TickData`, `OHLC`, etc.)
- **`repository.rs`** - PostgreSQL database operations
- **`cache.rs`** - Multi-level caching (L1 memory + L2 Redis)

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
trading-common = { path = "../trading-common" }
```

### Example: Running a Backtest

```rust
use trading_common::backtest::{BacktestEngine, BacktestConfig};
use trading_common::backtest::strategy::SmaStrategy;
use trading_common::data::repository::TickRepository;

// Create repository and fetch data
let repo = TickRepository::new(pool).await?;
let ticks = repo.get_ticks_range("BTCUSDT", start, end).await?;

// Configure and run backtest
let config = BacktestConfig {
    initial_capital: 10000.0,
    commission_rate: 0.001,
};

let strategy = SmaStrategy::new(10, 20);
let engine = BacktestEngine::new(config);
let result = engine.run(&ticks, &strategy)?;

println!("Total Return: {:.2}%", result.metrics.total_return * 100.0);
```
