// trading-core/src/backtest/mod.rs

pub mod sma; // sma 模块，可能包含简单移动平均线 (SMA) 策略的实现
pub mod types; // types 模块，定义回测相关的各种数据结构和类型
pub mod engine; // engine 模块，回测引擎的实现
pub mod metrics; // metrics 模块，用于计算回测性能指标

use std::collections::HashMap; // 引入标准库的 HashMap，用于存储键值对，例如策略参数
pub use types::*; // 重新导出 types 模块中的所有公共项，方便外部使用

use crate::data::types::MarketDataPoint; // 引入数据模块中的 MarketDataPoint 类型，表示单个市场数据点
pub trait Strategy: Send { // 定义 Strategy trait，表示一个交易策略，要求实现 Send trait 以支持跨线程传递
    fn on_data(&mut self, data: &MarketDataPoint, portfolio: &Portfolio) -> Vec<Order>; // 当有新的市场数据点时调用此方法，返回生成的交易订单列表
    fn get_parameters(&self) -> &HashMap<String, String>; // 获取策略的参数
    fn get_type(&self) -> StrategyType; // 获取策略的类型
}