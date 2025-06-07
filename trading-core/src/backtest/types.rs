// trading-core/src/backtest/types.rs

use std::collections::HashMap; // 引入标准库的 HashMap，用于键值对存储
use chrono::{DateTime, Utc}; // 引入 chrono 库，用于处理日期和时间，特别是 UTC 时间
use rust_decimal::Decimal; // 引入 rust_decimal 库的 Decimal 类型，用于高精度十进制计算
use serde::{Deserialize, Serialize}; // 引入 serde 库的 Deserialize 和 Serialize trait，用于序列化和反序列化

// 基础配置 // 基础配置
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct BacktestConfig { // 定义回测配置结构体
    pub start_time: DateTime<Utc>, // 回测开始时间
    pub end_time: DateTime<Utc>, // 回测结束时间
    pub initial_capital: Decimal, // 初始资金
    pub symbol: String, // 交易品种
    pub commission_rate: Decimal, // 佣金费率
}

// 策略类型 // 策略类型
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub enum StrategyType { // 定义策略类型枚举
    SMACross, // 简单移动平均线交叉策略
    RSI, // 相对强弱指数策略
    MACD, // 移动平均收敛散度策略
    BollingerBands, // 布林带策略
    Custom(String), // 自定义策略类型，包含一个字符串名称
}

// 订单类型 // 订单类型
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub enum OrderType { // 定义订单类型枚举
    Market, // 市价单
    Limit(Decimal), // 限价单，包含一个 Decimal 类型的价格
}

// 订单方向 // 订单方向
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] // 派生 Debug, Clone, PartialEq, Serialize, Deserialize trait
pub enum OrderSide { // 定义订单方向枚举
    Buy, // 买入
    Sell, // 卖出
}

// 订单结构 // 订单结构
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct Order { // 定义订单结构体
    pub symbol: String, // 交易品种
    pub order_type: OrderType, // 订单类型
    pub side: OrderSide, // 订单方向
    pub quantity: Decimal, // 订单数量
    pub timestamp: DateTime<Utc>, // 订单时间戳
}

// 交易结构 // 交易结构
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct Trade { // 定义交易结构体
    pub symbol: String, // 交易品种
    pub side: OrderSide, // 交易方向
    pub quantity: Decimal, // 交易数量
    pub price: Decimal, // 交易价格
    pub timestamp: DateTime<Utc>, // 交易时间戳
    pub commission: Decimal, // 交易佣金
}

// 持仓信息 // 持仓信息
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct Position { // 定义持仓结构体
    pub symbol: String, // 交易品种
    pub quantity: Decimal, // 持仓数量
    pub average_entry_price: Decimal, // 平均入场价格
}

// 投资组合 // 投资组合
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct Portfolio { // 定义投资组合结构体
    pub cash: Decimal, // 现金余额
    pub positions: HashMap<String, Position>, // 持仓信息，使用 HashMap 存储，键为交易品种字符串
    pub total_value: Decimal, // 组合总价值
}

// 权益点 // 权益点
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct EquityPoint { // 定义权益点结构体，用于绘制权益曲线
    pub timestamp: String, // 时间戳（字符串格式）
    pub value: String, // 权益值（字符串格式）
}

// 回测结果 // 回测结果
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct BacktestResult { // 定义回测结果结构体
    pub strategy_type: StrategyType, // 策略类型
    pub parameters: HashMap<String, String>, // 策略参数
    pub metrics: Metrics, // 回测指标
    pub trades: Vec<Trade>, // 交易记录列表
    pub equity_curve: Vec<EquityPoint>, // 权益曲线数据点列表
}

// 性能指标 // 性能指标
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct Metrics { // 定义性能指标结构体
    // 基础指标 // 基础指标
    pub total_return: Decimal, // 总回报率
    pub total_trades: u32, // 总交易次数
    pub winning_trades: u32, // 盈利交易次数
    pub losing_trades: u32, // 亏损交易次数
    pub win_rate: Decimal, // 胜率
    pub profit_factor: Decimal, // 盈利因子
    
    // 风险指标 // 风险指标
    pub sharpe_ratio: f64, // 夏普比率
    pub sortino_ratio: f64, // 索提诺比率
    pub max_drawdown: Decimal, // 最大回撤
    pub max_drawdown_duration: i64,  // 以秒为单位 // 最大回撤持续时间（以秒为单位）
    
    // 收益指标 // 收益指标
    pub avg_profit_per_trade: Decimal, // 平均每笔交易利润
    pub avg_winning_trade: Decimal, // 平均盈利交易额
    pub avg_losing_trade: Decimal, // 平均亏损交易额
    pub largest_winning_trade: Decimal, // 最大单笔盈利
    pub largest_losing_trade: Decimal, // 最大单笔亏损
    
    // 交易指标 // 交易指标
    pub avg_trade_duration: i64,     // 以秒为单位 // 平均持仓时间（以秒为单位）
    pub profit_per_month: Decimal, // 月均利润
    pub annual_return: Decimal, // 年化回报率
    pub monthly_sharpe: f64, // 月度夏普比率
    
    // 额外统计 // 额外统计
    pub total_commission: Decimal, // 总佣金
    pub total_volume: Decimal, // 总交易量
    pub avg_position_size: Decimal, // 平均头寸规模
}

// 前端请求结构 // 前端请求结构
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct BacktestRequest { // 定义回测请求结构体（用于前端）
    pub strategy_type: StrategyType, // 策略类型
    pub parameters: HashMap<String, String>, // 策略参数
    pub config: BacktestConfig, // 回测配置
}

// 前端响应结构 // 前端响应结构
#[derive(Serialize)] // 派生 Serialize trait
pub struct TradeResponse { // 定义交易响应结构体（用于前端）
    pub timestamp: String, // 时间戳（字符串格式）
    pub symbol: String, // 交易品种
    pub side: String, // 交易方向（字符串格式）
    pub quantity: String, // 交易数量（字符串格式）
    pub price: String, // 交易价格（字符串格式）
    pub commission: String, // 交易佣金（字符串格式）
}

#[derive(Serialize)] // 派生 Serialize trait
pub struct BacktestResponse { // 定义回测响应结构体（用于前端）
    pub total_return: String, // 总回报率（字符串格式）
    pub sharpe_ratio: f64, // 夏普比率
    pub max_drawdown: String, // 最大回撤（字符串格式）
    pub win_rate: String, // 胜率（字符串格式）
    pub total_trades: u32, // 总交易次数
    pub equity_curve: Vec<EquityPoint>, // 权益曲线数据点列表
    pub trades: Vec<TradeResponse>, // 交易记录列表（使用 TradeResponse 格式）
}

// 策略评分结果（为 NFT 准备） // 策略评分结果（为 NFT 准备）
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct StrategyScore { // 定义策略评分结构体
    pub total_score: u32, // 总评分
    pub return_score: u32, // 回报评分
    pub risk_score: u32, // 风险评分
    pub consistency_score: u32, // 一致性评分
    pub uniqueness_score: u32, // 独特性评分
    pub rating: StrategyRating, // 策略评级
}

// 策略评级（为 NFT 准备） // 策略评级（为 NFT 准备）
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub enum StrategyRating { // 定义策略评级枚举
    Legendary, // 传奇
    Epic, // 史诗
    Rare, // 稀有
    Common, // 普通
}

// NFT 元数据（为 NFT 准备） // NFT 元数据（为 NFT 准备）
#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct StrategyNFTMetadata { // 定义策略 NFT 元数据结构体
    pub strategy_id: String, // 策略 ID
    pub name: String, // 策略名称
    pub description: String, // 策略描述
    pub creator: String, // 创建者
    pub creation_date: DateTime<Utc>, // 创建日期
    pub metrics: Metrics, // 性能指标
    pub score: StrategyScore, // 策略评分
    pub parameters: HashMap<String, String>, // 策略参数
    pub trading_period: String, // 交易周期
    pub symbol: String, // 交易品种
    pub image_url: Option<String>, // 图片 URL（可选）
}