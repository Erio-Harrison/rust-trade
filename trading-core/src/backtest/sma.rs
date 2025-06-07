// trading-core/src/backtest/strategy/sma.rs // 文件路径
// This is only an example to test system function, we should not use it to trade. // 这只是一个测试系统功能的示例，不应用于实际交易。

use crate::backtest::{Strategy, Order, Portfolio, OrderType, OrderSide}; // 引入回测模块的 Strategy, Order, Portfolio, OrderType, OrderSide
use crate::data::types::MarketDataPoint; // 引入数据模块的 MarketDataPoint 类型
use rust_decimal::Decimal; // 引入 rust_decimal 的 Decimal 类型，用于高精度计算
use std::collections::{HashMap, VecDeque}; // 引入标准库的 HashMap 和 VecDeque

pub struct SMAStrategy { // 定义 SMA 策略结构体
    symbol: String, // 交易品种
    short_period: usize, // 短期均线周期
    long_period: usize, // 长期均线周期
    short_ma: VecDeque<f64>, // 短期移动平均线数据队列
    long_ma: VecDeque<f64>, // 长期移动平均线数据队列
    position_size: Decimal, // 头寸规模
    parameters: HashMap<String, String>, // 策略参数
}

impl SMAStrategy { // 实现 SMA 策略的方法
    pub fn new(symbol: String, short_period: usize, long_period: usize, position_size: Decimal) -> Self { // 定义构造函数
        let mut parameters = HashMap::new(); // 创建一个新的 HashMap 用于存储参数
        parameters.insert("short_period".to_string(), short_period.to_string()); // 插入短期周期参数
        parameters.insert("long_period".to_string(), long_period.to_string()); // 插入长期周期参数
        
        Self { // 返回 SMAStrategy 实例
            symbol, // 交易品种
            short_period, // 短期均线周期
            long_period, // 长期均线周期
            short_ma: VecDeque::with_capacity(short_period), // 初始化短期均线队列，指定容量
            long_ma: VecDeque::with_capacity(long_period), // 初始化长期均线队列，指定容量
            position_size, // 头寸规模
            parameters, // 策略参数
        }
    }

    fn calculate_ma(&mut self, price: f64) -> Option<(f64, f64)> { // 定义 calculate_ma 方法，计算移动平均线
        self.short_ma.push_back(price); // 将当前价格推入短期均线队列尾部
        self.long_ma.push_back(price); // 将当前价格推入长期均线队列尾部

        if self.short_ma.len() > self.short_period { // 如果短期均线队列长度超过短期周期
            self.short_ma.pop_front(); // 从队列头部移除一个元素
        }
        if self.long_ma.len() > self.long_period { // 如果长期均线队列长度超过长期周期
            self.long_ma.pop_front(); // 从队列头部移除一个元素
        }

        if self.short_ma.len() == self.short_period && self.long_ma.len() == self.long_period { // 如果短期和长期均线队列都已填满
            let short_ma = self.short_ma.iter().sum::<f64>() / self.short_period as f64; // 计算短期移动平均值
            let long_ma = self.long_ma.iter().sum::<f64>() / self.long_period as f64; // 计算长期移动平均值
            Some((short_ma, long_ma)) // 返回计算得到的短期和长期均线值
        } else { // 否则（队列未填满）
            None // 返回 None
        }
    }
}

impl Strategy for SMAStrategy { // 为 SMAStrategy 实现 Strategy trait
    fn on_data(&mut self, data: &MarketDataPoint, portfolio: &Portfolio) -> Vec<Order> { // 定义 on_data 方法，处理新的市场数据点
        let mut orders = Vec::new(); // 初始化一个空的订单列表
        
        // 计算移动平均线 // 计算移动平均线
        if let Some((short_ma, long_ma)) = self.calculate_ma(data.price) { // 计算短期和长期移动平均线
            // 生成交易信号 // 生成交易信号
            if short_ma > long_ma { // 如果短期均线上穿长期均线
                // 金叉，买入信号 // 金叉，买入信号
                if !portfolio.positions.contains_key(&self.symbol) { // 如果当前没有该品种的持仓
                    orders.push(Order { // 创建一个买入订单
                        symbol: self.symbol.clone(), // 交易品种
                        order_type: OrderType::Market, // 订单类型：市价单
                        side: OrderSide::Buy, // 订单方向：买入
                        quantity: self.position_size, // 订单数量：预设的头寸规模
                        timestamp: data.timestamp, // 订单时间戳：当前数据点的时间戳
                    });
                }
            } else { // 如果短期均线下穿长期均线（或等于）
                // 死叉，卖出信号 // 死叉，卖出信号
                if let Some(position) = portfolio.positions.get(&self.symbol) { // 如果当前有该品种的持仓
                    orders.push(Order { // 创建一个卖出订单
                        symbol: self.symbol.clone(), // 交易品种
                        order_type: OrderType::Market, // 订单类型：市价单
                        side: OrderSide::Sell, // 订单方向：卖出
                        quantity: position.quantity, // 订单数量：当前持仓数量
                        timestamp: data.timestamp, // 订单时间戳：当前数据点的时间戳
                    });
                }
            }
        }
        
        orders // 返回生成的订单列表
    }

    fn get_parameters(&self) -> &HashMap<String, String> { // 定义 get_parameters 方法，获取策略参数
        &self.parameters // 返回参数 HashMap 的引用
    }

    fn get_type(&self) -> crate::backtest::StrategyType { // 定义 get_type 方法，获取策略类型
        crate::backtest::StrategyType::SMACross // 返回策略类型为 SMACross
    }
}