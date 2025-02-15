# Rust Trading System (rust-trade)
A quantitative trading system written in Rust

[![My Skills](https://skillicons.dev/icons?i=rust,tauri,ts,react,postgresql)](https://skillicons.dev)

[Maybe you don't know much about Tauri.](https://v2.tauri.app/)

Tauri 2.0 is a framework for building lightweight, secure desktop applications using web technologies and Rust. It provides a minimal footprint by leveraging the OS's webview instead of bundling a heavy runtime, offering better performance, security, and native API integration.

## Overview

```bash
rust-trade/
├── src-tauri/                     # Desktop Application Backend
│   ├── src/
│   │   ├── main.rs               # Tauri application entry
│   │   ├── commands.rs           # Command handlers
│   │   └── state.rs              # Application state management
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── frontend/                      # Frontend Interface
│   ├── src/
│   │   ├── app/
│   │   │   ├── layout.tsx        # Main layout
│   │   │   ├── page.tsx          # Home page
│   │   │   ├── trading/          # Trading interface
│   │   │   ├── backtest/         # Backtesting interface
│   │   │   └── settings/         # Settings page
│   │   └── components/
│   │       ├── charts/           # Trading charts
│   │       └── layout/           # Layout components
│   ├── package.json
│   └── next.config.ts
│
├── trading-core/                  # Core Trading Library
│   ├── src/
│   │   ├── lib.rs                # Library entry point
│   │   ├── config.rs             # Configuration management
│   │   ├── services/
│   │   │   ├── mod.rs
│   │   │   └── exchange/         # Exchange services
│   │   │       ├── mod.rs
│   │   │       ├── types.rs      # Exchange types & traits
│   │   │       ├── binance.rs    # Binance implementation
│   │   │       └── collector.rs   # Market data collector
│   │   │
│   │   ├── backtest/             # Backtesting system
│   │   │   ├── engine.rs         # Backtesting engine
│   │   │   ├── strategy.rs       # Strategy implementations
│   │   │   └── types.rs          # Backtesting types
│   │   │
│   │   ├── data/                 # Data management
│   │   │   ├── mod.rs
│   │   │   ├── database.rs       # Database connection
│   │   │   └── market_data.rs    # Market data management
│   │   │
│   │   └── api/                  # API layer
│   │       ├── mod.rs
│   │       ├── types.rs          # API types
│   │       └── rest.rs           # REST endpoints
│   │
│   └── Cargo.toml
│
├── config/                        # Configuration
│   ├── default.toml              # Default configuration
│   └── production.toml           # Production configuration
│
└── migrations/                    # Database migrations
    └── market_data.sql           # Market data schema
```

rust-trade is a quantitative trading system that combines modern trading strategies with artificial intelligence. This software is released under the GNU General Public License v3. In this basic version, I have defined various interfaces. I will expand the functions I hope to achieve in the next step (allowing customers to run their own strategies in this system and see the results of the strategies. At the same time, mint high-quality strategies into NFTs and put them in the blockchain world)

Copyright (C) 2024 Harrison

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

![result](assets/version1.png)

## Required Environment Variables
```bash
DATABASE_URL=postgresql://user:password@localhost/dbname
OPENAI_KEY=your-openai-api-key
```

## Development Roadmap

1. **AI Integration Improvements**
   - Implement response caching
   - Add retry mechanism
   - Improve async handling
   - Enhance risk management

2. **Strategy Enhancements**
   - Add more technical indicators
   - Implement hybrid strategies
   - Improve position sizing
   - Add performance metrics

3. **System Optimization**
   - Optimize database queries
   - Improve error handling
   - Add monitoring system
   - Implement data validation

## License
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
