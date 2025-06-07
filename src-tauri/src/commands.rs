// src-tauri/src/commands.rs

use crate::state::AppState; // 引入 AppState 模块，用于管理应用状态
use rust_decimal::{prelude::FromPrimitive, Decimal}; // 引入 rust_decimal 库，用于高精度计算
use tauri::State; // 引入 tauri::State，用于在命令处理函数中访问共享状态
use trading_core::{ // 引入 trading_core 核心库
    backtest::{ // 回测模块
        engine::BacktestEngine, // 回测引擎
        sma::SMAStrategy, // SMA 策略
        types::{BacktestRequest, BacktestResponse, TradeResponse, StrategyType} // 回测相关的类型定义
    },
    data::types::MarketDataManager, // 市场数据管理器
};
use tracing::{info, error, debug}; // 引入 tracing 库，用于日志记录

#[tauri::command] // Tauri 命令宏，将此函数暴露给前端调用
pub async fn run_backtest<'a>( // 定义异步函数 run_backtest，用于执行回测
    state: State<'a, AppState>, // 应用状态，通过 Tauri 的 State 注入
    request: BacktestRequest, // 回测请求参数，从前端传递
) -> Result<BacktestResponse, String> { // 返回回测结果或错误信息
    let market_data = MarketDataManager::new(state.market_manager.get_pool()); // 创建市场数据管理器实例

    // 首先获取当前价格数据 // 首先获取当前价格数据
    let data = market_data // 调用市场数据管理器的 get_market_data 方法获取历史市场数据
        .get_market_data( // 获取市场数据
            &request.config.symbol, // 交易品种
            request.config.start_time, // 开始时间
            request.config.end_time, // 结束时间
        )
        .await // 异步等待结果
        .map_err(|e| e.to_string())?; // 如果出错，将错误转换为字符串并返回

    if data.is_empty() { // 检查获取到的数据是否为空
        return Err("No historical data available".to_string()); // 如果为空，返回错误信息
    }

    // 计算实际的交易数量而不是金额 // 计算实际的交易数量而不是金额
    let first_price = Decimal::from_f64(data[0].price) // 获取第一条数据的价格，并转换为 Decimal 类型
        .ok_or("Failed to convert price")?; // 如果转换失败，返回错误信息

    let position_size_percent = request.parameters // 从请求参数中获取头寸规模百分比
        .get("position_size_percent") // 获取 "position_size_percent" 参数
        .and_then(|v| v.parse::<f64>().ok()) // 将参数值解析为 f64 类型
        .unwrap_or(10.0); // 如果参数不存在或解析失败，则使用默认值 10.0

    // 计算实际的交易数量 // 计算实际的交易数量
    let position_size = (request.config.initial_capital * // 初始资金
        Decimal::from_f64(position_size_percent / 100.0).unwrap()) / // 头寸规模百分比转换为 Decimal
        first_price; // 除以初始价格得到交易数量

    info!( // 记录头寸计算信息
        "Position calculation: capital={}, percent={}, price={}, quantity={}", // 日志内容格式
        request.config.initial_capital, // 初始资金
        position_size_percent, // 头寸规模百分比
        first_price, // 初始价格
        position_size // 计算得到的交易数量
    );

    let strategy = match request.strategy_type { // 根据请求的策略类型创建策略实例
        StrategyType::SMACross => { // 如果是 SMA 交叉策略
            SMAStrategy::new( // 创建 SMAStrategy 实例
                request.config.symbol.clone(), // 交易品种
                request.parameters.get("short_period") // 获取短期均线周期参数
                    .and_then(|v| v.parse().ok()) // 解析为整数
                    .unwrap_or(5), // 默认值为 5
                request.parameters.get("long_period") // 获取长期均线周期参数
                    .and_then(|v| v.parse().ok()) // 解析为整数
                    .unwrap_or(20), // 默认值为 20
                position_size, // 这里传入的是数量而不是金额 // 传入计算好的交易数量
            )
        },
        _ => return Err("Unsupported strategy type".to_string()), // 如果是不支持的策略类型，返回错误
    };

    // 运行回测 // 运行回测
    info!("Initializing backtest engine"); // 记录日志：初始化回测引擎
    let mut engine = BacktestEngine::new(market_data, request.config.clone()); // 创建回测引擎实例
    
    info!("Starting backtest"); // 记录日志：开始回测
    let result = match engine.run_strategy(Box::new(strategy)).await { // 运行策略回测
        Ok(res) => { // 如果回测成功
            info!("Backtest completed successfully"); // 记录日志：回测成功完成
            debug!("Backtest metrics: {:?}", res.metrics); // 记录调试日志：回测指标
            res // 返回回测结果
        },
        Err(e) => { // 如果回测失败
            error!("Backtest failed: {}", e); // 记录错误日志：回测失败
            return Err(e.to_string()); // 返回错误信息
        }
    };

    // 转换结果为响应格式 // 转换结果为响应格式
    info!("Converting results to response format"); // 记录日志：转换结果为响应格式
    let response = BacktestResponse { // 构建回测响应对象
        total_return: result.metrics.total_return.to_string(), // 总回报率
        sharpe_ratio: result.metrics.sharpe_ratio, // 夏普比率
        max_drawdown: result.metrics.max_drawdown.to_string(), // 最大回撤
        win_rate: result.metrics.win_rate.to_string(), // 胜率
        total_trades: result.metrics.total_trades, // 总交易次数
        equity_curve: result.equity_curve, // 权益曲线
        trades: result.trades.into_iter().map(|trade| { // 遍历交易记录并转换为 TradeResponse 格式
            debug!("Processing trade: {:?}", trade); // 记录调试日志：处理交易记录
            TradeResponse { // 构建交易响应对象
                timestamp: trade.timestamp.to_rfc3339(), // 交易时间戳，转换为 RFC3339 格式
                symbol: trade.symbol, // 交易品种
                side: match trade.side { // 交易方向
                    trading_core::backtest::types::OrderSide::Buy => "Buy".to_string(), // 买入
                    trading_core::backtest::types::OrderSide::Sell => "Sell".to_string(), // 卖出
                },
                quantity: trade.quantity.to_string(), // 交易数量
                price: trade.price.to_string(), // 交易价格
                commission: trade.commission.to_string(), // 佣金
            }
        }).collect(), // 收集所有转换后的交易响应对象
    };

    info!("Backtest response prepared successfully"); // 记录日志：回测响应准备成功
    Ok(response) // 返回成功的回测响应
}