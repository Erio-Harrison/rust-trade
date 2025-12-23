# Trading Common

共享库，为 Rust Trade 系统提供核心数据结构、回测引擎和数据访问层。

## 概述

`trading-common` 是被 `trading-core` (CLI) 和 `src-tauri` (桌面应用) 共同使用的基础库，包含所有不依赖特定运行时环境的共享功能。

## 项目结构

```
trading-common/
├── src/
│   ├── lib.rs                 # 库入口
│   ├── backtest/              # 回测系统
│   │   ├── mod.rs             # 模块导出和公共接口
│   │   ├── engine.rs          # 核心回测引擎和执行逻辑
│   │   ├── portfolio.rs       # 投资组合管理、持仓跟踪、盈亏计算
│   │   ├── metrics.rs         # 性能指标计算 (Sharpe, 回撤等)
│   │   └── strategy/          # 交易策略
│   │       ├── mod.rs         # 策略工厂和管理
│   │       ├── base.rs        # 策略 trait 定义
│   │       ├── sma.rs         # 简单移动平均策略
│   │       └── rsi.rs         # RSI 策略
│   └── data/                  # 数据层
│       ├── mod.rs             # 模块导出
│       ├── types.rs           # 核心数据类型 (TickData, OHLC, 错误类型)
│       ├── repository.rs      # 数据库操作和查询逻辑
│       └── cache.rs           # 多级缓存实现 (L1 内存 + L2 Redis)
└── Cargo.toml
```

## 模块说明

### `backtest/` - 回测引擎

完整的策略评估回测系统：

- **`engine.rs`** - 处理历史数据的核心回测逻辑
- **`metrics.rs`** - 性能指标计算 (Sharpe 比率、最大回撤、胜率等)
- **`portfolio.rs`** - 投资组合管理和盈亏跟踪
- **`strategy/`** - 交易策略实现
  - `sma.rs` - 简单移动平均交叉策略
  - `rsi.rs` - 相对强弱指数策略

### `data/` - 数据层

数据访问和缓存基础设施：

- **`types.rs`** - 核心数据结构 (`TickData`, `OHLC` 等)
- **`repository.rs`** - PostgreSQL 数据库操作
- **`cache.rs`** - 多级缓存 (L1 内存 + L2 Redis)

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
trading-common = { path = "../trading-common" }
```

### 示例：运行回测

```rust
use trading_common::backtest::{BacktestEngine, BacktestConfig};
use trading_common::backtest::strategy::SmaStrategy;
use trading_common::data::repository::TickRepository;

// 创建 repository 并获取数据
let repo = TickRepository::new(pool).await?;
let ticks = repo.get_ticks_range("BTCUSDT", start, end).await?;

// 配置并运行回测
let config = BacktestConfig {
    initial_capital: 10000.0,
    commission_rate: 0.001,
};

let strategy = SmaStrategy::new(10, 20);
let engine = BacktestEngine::new(config);
let result = engine.run(&ticks, &strategy)?;

println!("总收益: {:.2}%", result.metrics.total_return * 100.0);
```
