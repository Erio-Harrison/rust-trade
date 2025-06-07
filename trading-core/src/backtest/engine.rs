// trading-core/src/backtest/engine.rs

use super::{types::*, Strategy}; // 引入父模块的 types 和 Strategy
use super::metrics::MetricsCalculator; // 引入父模块的 metrics 模块中的 MetricsCalculator
use crate::data::types::{MarketDataManager, MarketDataPoint}; // 引入数据模块的 MarketDataManager 和 MarketDataPoint
use bigdecimal::{FromPrimitive, Zero}; // 引入 bigdecimal 库的 FromPrimitive 和 Zero，用于高精度计算
use chrono::{DateTime,Utc}; // 引入 chrono 库的 DateTime 和 Utc，用于处理时间
use rust_decimal::Decimal; // 引入 rust_decimal 库的 Decimal，用于十进制数计算
use std::{collections::HashMap, error::Error}; // 引入标准库的 HashMap 和 Error
use tracing::{info, warn}; // 引入 tracing 库的 info 和 warn，用于日志记录

pub struct BacktestEngine { // 定义回测引擎结构体
    market_data: MarketDataManager, // 市场数据管理器
    config: BacktestConfig, // 回测配置
    portfolio: Portfolio, // 投资组合
    trades: Vec<Trade>, // 交易记录列表
    metrics_calculator: MetricsCalculator, // 指标计算器
    equity_points: Vec<EquityPoint>, // 权益点列表，用于绘制权益曲线
}

impl BacktestEngine { // 实现回测引擎的方法
    pub fn new(market_data: MarketDataManager, config: BacktestConfig) -> Self { // 定义构造函数
        let portfolio = Portfolio { // 初始化投资组合
            cash: config.initial_capital, // 初始现金为配置中的初始资金
            positions: HashMap::new(), // 初始化持仓为空的 HashMap
            total_value: config.initial_capital, // 初始总价值为配置中的初始资金
        };

        Self { // 返回回测引擎实例
            market_data, // 市场数据管理器
            config, // 回测配置
            portfolio, // 投资组合
            trades: Vec::new(), // 初始化交易记录列表为空
            metrics_calculator: MetricsCalculator::new(), // 创建新的指标计算器实例
            equity_points: Vec::new(), // 初始化权益点列表为空
        }
    }

    pub async fn run_strategy( // 定义异步方法 run_strategy，运行回测策略
        &mut self, // 可变引用自身
        mut strategy: Box<dyn Strategy>, // 策略实例，使用 Box 包装的动态分发 Strategy trait 对象
    ) -> Result<BacktestResult, Box<dyn Error>> { // 返回回测结果或动态错误
        info!("Starting backtest for symbol: {}", self.config.symbol); // 记录日志：开始回测，指定交易品种

        // 记录初始权益点 // 记录初始权益点
        self.record_equity_point(self.config.start_time, self.portfolio.total_value); // 记录回测开始时的初始权益点

        let historical_data = self.market_data // 获取历史市场数据
            .get_market_data( // 调用市场数据管理器的 get_market_data 方法
                &self.config.symbol, // 交易品种
                self.config.start_time, // 开始时间
                self.config.end_time, // 结束时间
            )
            .await?; // 异步等待结果，如果出错则返回错误

        info!("Loaded {} historical data points", historical_data.len()); // 记录日志：已加载的历史数据点数量

        for data_point in historical_data { // 遍历每个历史数据点
            // 获取策略信号 // 获取策略信号
            let orders = strategy.on_data(&data_point, &self.portfolio); // 调用策略的 on_data 方法，获取交易订单
            
            // 执行订单 // 执行订单
            for order in orders { // 遍历每个订单
                if let Some(trade) = self.execute_order(&order, &data_point) { // 执行订单，如果成功生成交易记录
                    info!("Executed trade: {} {} {} @ {}", // 记录日志：已执行的交易
                        trade.timestamp, // 交易时间戳
                        if trade.side == OrderSide::Buy { "BUY" } else { "SELL" }, // 交易方向：买入或卖出
                        trade.quantity, // 交易数量
                        trade.price // 交易价格
                    );
                    self.trades.push(trade); // 将交易记录添加到列表中
                }
            }
            
            // 更新组合价值 // 更新组合价值
            self.update_portfolio_value(&data_point); // 根据当前数据点更新投资组合的总价值
            
            // 记录权益点 // 记录权益点
            self.record_equity_point(data_point.timestamp, self.portfolio.total_value); // 记录当前时间戳和组合总价值作为权益点
        }

        info!("Backtest completed. Calculating metrics..."); // 记录日志：回测完成，开始计算指标

        // 生成回测结果 // 生成回测结果
        let metrics = self.metrics_calculator.calculate( // 调用指标计算器的 calculate 方法计算回测指标
            &self.trades, // 交易记录列表
            &self.equity_points, // 权益点列表
            &self.config // 回测配置
        );

        Ok(BacktestResult { // 返回回测结果
            strategy_type: strategy.get_type(), // 策略类型
            parameters: strategy.get_parameters().clone(), // 策略参数
            metrics, // 回测指标
            trades: self.trades.clone(), // 交易记录列表的克隆
            equity_curve: self.equity_points.clone(), // 权益曲线的克隆
        })
    }

    fn execute_order(&mut self, order: &Order, data: &MarketDataPoint) -> Option<Trade> { // 定义方法 execute_order，执行单个订单
        let price = Decimal::from_f64(data.price)?; // 从数据点价格创建 Decimal 类型价格，如果失败则返回 None
        let commission = self.config.commission_rate * order.quantity * price; // 计算交易佣金

        match order.side { // 根据订单方向处理
            OrderSide::Buy => { // 买入订单
                let cost = order.quantity * price + commission; // 计算买入成本（含佣金）
                if cost <= self.portfolio.cash { // 如果现金足够支付成本
                    self.portfolio.cash -= cost; // 扣除现金
                    let position = self.portfolio.positions // 获取或创建持仓
                        .entry(order.symbol.clone()) // 根据交易品种获取条目
                        .or_insert(Position { // 如果不存在则插入新的持仓
                            symbol: order.symbol.clone(), // 交易品种
                            quantity: Decimal::zero(), // 初始数量为零
                            average_entry_price: Decimal::zero(), // 初始平均入场价为零
                        });
                    
                    let new_quantity = position.quantity + order.quantity; // 计算新的持仓数量
                    position.average_entry_price = // 更新平均入场价
                        (position.average_entry_price * position.quantity + price * order.quantity) 
                        / new_quantity;
                    position.quantity = new_quantity; // 更新持仓数量

                    Some(Trade { // 返回生成的交易记录
                        symbol: order.symbol.clone(), // 交易品种
                        side: OrderSide::Buy, // 交易方向：买入
                        quantity: order.quantity, // 交易数量
                        price, // 交易价格
                        timestamp: data.timestamp, // 交易时间戳
                        commission, // 交易佣金
                    })
                } else { // 如果现金不足
                    warn!("Insufficient funds for buy order"); // 记录警告日志：买入订单资金不足
                    None // 返回 None
                }
            },
            OrderSide::Sell => { // 卖出订单
                if let Some(position) = self.portfolio.positions.get_mut(&order.symbol) { // 获取可变引用的持仓
                    if position.quantity >= order.quantity { // 如果持仓数量足够卖出
                        position.quantity -= order.quantity; // 减少持仓数量
                        self.portfolio.cash += order.quantity * price - commission; // 增加现金（扣除佣金）
                        
                        if position.quantity.is_zero() { // 如果持仓数量为零
                            self.portfolio.positions.remove(&order.symbol); // 从持仓中移除该品种
                        }

                        Some(Trade { // 返回生成的交易记录
                            symbol: order.symbol.clone(), // 交易品种
                            side: OrderSide::Sell, // 交易方向：卖出
                            quantity: order.quantity, // 交易数量
                            price, // 交易价格
                            timestamp: data.timestamp, // 交易时间戳
                            commission, // 交易佣金
                        })
                    } else { // 如果持仓数量不足
                        warn!("Insufficient position for sell order"); // 记录警告日志：卖出订单持仓不足
                        None // 返回 None
                    }
                } else { // 如果没有找到该品种的持仓
                    warn!("No position found for sell order"); // 记录警告日志：卖出订单未找到持仓
                    None // 返回 None
                }
            }
        }
    }

    fn update_portfolio_value(&mut self, data: &MarketDataPoint) { // 定义方法 update_portfolio_value，更新投资组合总价值
        let positions_value = self.portfolio.positions.values() // 获取所有持仓的值
            .map(|pos| pos.quantity * Decimal::from_f64(data.price).unwrap_or_default()) // 计算每个持仓的当前价值（数量 * 当前价格）
            .sum::<Decimal>(); // 计算所有持仓的总价值

        self.portfolio.total_value = self.portfolio.cash + positions_value; // 更新投资组合总价值（现金 + 持仓总价值）
    }

    fn record_equity_point(&mut self, timestamp: DateTime<Utc>, value: Decimal) { // 定义方法 record_equity_point，记录权益点
        self.equity_points.push(EquityPoint { // 将新的权益点添加到列表中
            timestamp: timestamp.to_rfc3339(), // 时间戳，转换为 RFC3339 格式字符串
            value: value.to_string(), // 权益值，转换为字符串
        });
    }
}