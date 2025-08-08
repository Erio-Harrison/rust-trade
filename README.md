# Rust Trading System (rust-trade)
A quantitative trading system written in Rust

[![My Skills](https://skillicons.dev/icons?i=rust,tauri,ts,react,postgresql)](https://skillicons.dev)

[Maybe you don't know much about Tauri.](https://v2.tauri.app/)

Tauri 2.0 is a framework for building lightweight, secure desktop applications using web technologies and Rust. It provides a minimal footprint by leveraging the OS's webview instead of bundling a heavy runtime, offering better performance, security, and native API integration.

## Overview

```bash
rust-trade/
├── src-tauri/                   # Tauri desktop application backend
│   ├── src/
│   │   ├── main.rs             # Application entry point and Tauri initialization
│   │   ├── commands.rs         # Frontend command handlers
│   │   └── state.rs            # Application state management
│   └── Cargo.toml              # Tauri dependencies configuration
│
├── trading-core/                # Core trading library
│   ├── src/
│   │   ├── lib.rs              # Library entry point
│   │   ├── data/               # Data management module
│   │   │   ├── mod.rs          # Data module entry point
│   │   │   ├── cache.rs        # Market data caching system
│   │   │   ├── database.rs     # Database operations
│   │   │   └── market_data.rs  # Market data management
│   │   │
│   │   ├── backtest/           # Backtesting system
│   │   │   ├── engine.rs       # Backtesting engine
│   │   │   ├── metrics.rs      # Performance metrics calculation
│   │   │   └── types.rs        # Backtesting type definitions
│   │   │
│   │   └── blockchain/         # Blockchain module
│   │       ├── mod.rs          # Blockchain manager
│   │       ├── error.rs        # Error definitions
│   │       └── types.rs        # Type definitions
│   │
│   ├── benches/                # Performance benchmarks
│   │   └── market_data_cache.rs
│   └── Cargo.toml              # Core library dependencies
│
├── substrate-test-node/         # Local test node
│   ├── Dockerfile              # Node container definition
│   └── docker-compose.yml      # Container orchestration configuration
│
└── frontend/                    # Next.js frontend
    └── src/
        ├── app/
        │   └── page.tsx        # Main page
        └── components/         # Shared components

```

rust-trade is a quantitative trading system that combines modern trading strategies with artificial intelligence.

## How to run

Run in the root directory:

```bash
# Start the development server
cargo tauri dev
```

```bash
# Build the production version
cargo tauri build
```

## Example

![result](assets/version2NFT.png)

![result](assets/version2Strategy.png)

## Required Environment Variables
```bash
DATABASE_URL=postgresql://user:password@localhost/dbname
```

## Development Roadmap

1. Add more strategy templates
2. Implement strategy scoring system
3. Develop strategy market function
4. Add real trading support
5. Optimize performance indicator calculation
6. Add more data analysis tools

## License
MIT License

Copyright (c) 2025 Harrison
