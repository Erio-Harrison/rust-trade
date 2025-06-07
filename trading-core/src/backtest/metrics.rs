// trading-core/src/backtest/metrics.rs

use super::types::*; // 引入父模块的 types
use chrono::{DateTime, Duration, Utc}; // 引入 chrono 库，用于处理日期、时间和时间段
use rust_decimal::Decimal; // 引入 rust_decimal 库的 Decimal 类型，用于高精度十进制计算
use rust_decimal::prelude::*; // 引入 rust_decimal 的 prelude，包含常用 trait 和类型
use std::collections::HashMap; // 引入标准库的 HashMap，用于键值对存储

pub struct MetricsCalculator { // 定义指标计算器结构体
    risk_free_rate: f64, // 无风险利率，用于计算夏普比率等指标
}

impl MetricsCalculator { // 实现指标计算器的方法
    pub fn new() -> Self { // 定义构造函数
        Self { // 返回 MetricsCalculator 实例
            risk_free_rate: 0.02, // 初始化无风险利率为 2%
        }
    }

    pub fn calculate( // 定义 calculate 方法，用于计算所有回测指标
        &self, // 自身引用
        trades: &[Trade], // 交易记录列表的切片
        equity_points: &[EquityPoint], // 权益点列表的切片
        config: &BacktestConfig, // 回测配置的引用
    ) -> Metrics { // 返回 Metrics 结构体，包含所有计算出的指标
        let _ = config; // 暂时未使用 config，用 _ 忽略警告
        let (profit_trades, loss_trades) = self.analyze_trades(trades); // 分析交易，区分盈利和亏损交易
        let returns = self.calculate_returns(equity_points); // 计算收益率序列
        let (max_drawdown, max_drawdown_duration) = self.calculate_drawdown(equity_points); // 计算最大回撤和最大回撤持续时间

        Metrics { // 构建并返回 Metrics 结构体
            // 基础指标 - 已实现 // 基础指标 - 已实现
            total_return: self.calculate_total_return(equity_points), // 计算总回报率
            total_trades: trades.len() as u32, // 总交易次数
            winning_trades: profit_trades.len() as u32, // 盈利交易次数
            losing_trades: loss_trades.len() as u32, // 亏损交易次数
            win_rate: self.calculate_win_rate(profit_trades.len(), trades.len()), // 计算胜率
            profit_factor: self.calculate_profit_factor(&profit_trades, &loss_trades), // 计算盈利因子
            
            // 风险指标 - 已实现 // 风险指标 - 已实现
            sharpe_ratio: self.calculate_sharpe_ratio(&returns), // 计算夏普比率
            sortino_ratio: self.calculate_sortino_ratio(&returns), // 计算索提诺比率
            max_drawdown, // 最大回撤
            max_drawdown_duration: max_drawdown_duration.num_seconds(), // 最大回撤持续时间（秒）
            
            // 交易统计 - 已实现 // 交易统计 - 已实现
            avg_profit_per_trade: self.calculate_avg_profit(trades), // 计算平均每笔交易利润
            total_commission: trades.iter().map(|t| t.commission).sum(), // 计算总佣金
            total_volume: self.calculate_total_volume(trades), // 计算总交易量
            
            // TODO: 待实现的指标 // TODO: 待实现的指标
            avg_winning_trade: Decimal::zero(),  // 需要实现 // 平均盈利交易额
            avg_losing_trade: Decimal::zero(),   // 需要实现 // 平均亏损交易额
            largest_winning_trade: Decimal::zero(), // 需要实现 // 最大单笔盈利
            largest_losing_trade: Decimal::zero(),  // 需要实现 // 最大单笔亏损
            avg_trade_duration: 0,               // 需要实现 // 平均持仓时间
            profit_per_month: Decimal::zero(),   // 需要实现 // 月均利润
            annual_return: Decimal::zero(),      // 需要实现 // 年化回报率
            monthly_sharpe: 0.0,                 // 需要实现 // 月度夏普比率
            avg_position_size: Decimal::zero(),  // 需要实现 // 平均头寸规模
        }
    }

    fn analyze_trades(&self, trades: &[Trade]) -> (Vec<Trade>, Vec<Trade>) { // 定义 analyze_trades 方法，分析交易记录
        let mut profit_trades = Vec::new(); // 初始化盈利交易列表
        let mut loss_trades = Vec::new(); // 初始化亏损交易列表
        let mut position_map: HashMap<String, (Decimal, Decimal)> = HashMap::new(); // 初始化持仓映射表，用于跟踪每个品种的平均成本和数量

        for trade in trades { // 遍历每笔交易
            match trade.side { // 根据交易方向处理
                OrderSide::Buy => { // 如果是买入交易
                    let (qty, avg_price) = position_map // 获取或插入该品种的持仓信息
                        .entry(trade.symbol.clone()) // 使用交易品种作为键
                        .or_insert((Decimal::zero(), Decimal::zero())); // 如果不存在，则插入初始数量和平均价格为零的元组
                    
                    *avg_price = (*avg_price * *qty + trade.price * trade.quantity) // 更新平均成本价
                        / (*qty + trade.quantity);
                    *qty += trade.quantity; // 更新持仓数量
                }
                OrderSide::Sell => { // 如果是卖出交易
                    if let Some((_, avg_price)) = position_map.get(&trade.symbol) { // 获取该品种的平均成本价
                        if trade.price > *avg_price { // 如果卖出价格高于平均成本价
                            profit_trades.push(trade.clone()); // 将该交易归为盈利交易
                        } else { // 否则
                            loss_trades.push(trade.clone()); // 将该交易归为亏损交易
                        }
                    }
                }
            }
        }

        (profit_trades, loss_trades) // 返回盈利交易列表和亏损交易列表
    }

    fn calculate_returns(&self, equity_points: &[EquityPoint]) -> Vec<f64> { // 定义 calculate_returns 方法，计算收益率序列
        equity_points.windows(2) // 使用滑动窗口遍历权益点，每次取两个相邻的点
            .map(|window| { // 对每个窗口进行映射
                let prev_value = Decimal::from_str(&window[0].value).unwrap_or_default(); // 获取前一个权益点的值
                let curr_value = Decimal::from_str(&window[1].value).unwrap_or_default(); // 获取当前权益点的值
                if prev_value.is_zero() { // 如果前一个值是零，则收益率为零（避免除以零错误）
                    0.0
                } else { // 否则，计算收益率
                    ((curr_value - prev_value) / prev_value).to_f64().unwrap_or_default() // (当前值 - 前一个值) / 前一个值，并转换为 f64
                }
            })
            .collect() // 收集所有计算出的收益率到 Vec<f64>
    }

    fn calculate_drawdown(&self, equity_points: &[EquityPoint]) -> (Decimal, Duration) { // 定义 calculate_drawdown 方法，计算最大回撤和持续时间
        let mut max_drawdown = Decimal::zero(); // 初始化最大回撤为零
        let mut max_drawdown_duration = Duration::zero(); // 初始化最大回撤持续时间为零
        let mut peak_value = Decimal::zero(); // 初始化峰值为零
        let mut peak_time = Utc::now(); // 初始化峰值时间为当前 UTC 时间
        
        for point in equity_points { // 遍历每个权益点
            let value = Decimal::from_str(&point.value).unwrap_or_default(); // 获取当前权益点的值
            let time = DateTime::parse_from_rfc3339(&point.timestamp) // 解析时间戳字符串
                .unwrap_or_else(|_| Utc::now().into()) // 如果解析失败，则使用当前 UTC 时间
                .with_timezone(&Utc); // 转换为 UTC 时区

            if value > peak_value { // 如果当前值大于峰值
                peak_value = value; // 更新峰值为当前值
                peak_time = time; // 更新峰值时间为当前时间
            } else if peak_value > Decimal::zero() { // 否则，如果峰值大于零（避免在初始阶段计算回撤）
                let drawdown = (peak_value - value) / peak_value; // 计算当前回撤百分比
                let duration = time - peak_time; // 计算当前回撤持续时间
                
                if drawdown > max_drawdown { // 如果当前回撤大于最大回撤
                    max_drawdown = drawdown; // 更新最大回撤
                    max_drawdown_duration = duration; // 更新最大回撤持续时间
                }
            }
        }
        
        (max_drawdown, max_drawdown_duration) // 返回最大回撤和最大回撤持续时间
    }

    fn calculate_total_return(&self, equity_points: &[EquityPoint]) -> Decimal { // 定义 calculate_total_return 方法，计算总回报率
        if equity_points.len() < 2 { // 如果权益点数量少于 2，无法计算回报率
            return Decimal::zero(); // 返回零
        }

        let initial_value = Decimal::from_str(&equity_points[0].value).unwrap_or_default(); // 获取初始权益值
        let final_value = Decimal::from_str(&equity_points[equity_points.len() - 1].value).unwrap_or_default(); // 获取最终权益值

        if initial_value.is_zero() { // 如果初始值为零，无法计算回报率（避免除以零错误）
            return Decimal::zero(); // 返回零
        }

        ((final_value - initial_value) / initial_value) * Decimal::from(100) // 计算总回报率百分比
    }

    fn calculate_win_rate(&self, winning_trades: usize, total_trades: usize) -> Decimal { // 定义 calculate_win_rate 方法，计算胜率
        if total_trades == 0 { // 如果总交易次数为零，胜率为零
            return Decimal::zero(); // 返回零
        }
        
        Decimal::from(winning_trades) / Decimal::from(total_trades) * Decimal::from(100) // 计算胜率百分比
    }

    fn calculate_profit_factor(&self, profit_trades: &[Trade], loss_trades: &[Trade]) -> Decimal { // 定义 calculate_profit_factor 方法，计算盈利因子
        let total_profit = profit_trades.iter() // 计算总盈利
            .map(|t| (t.price - t.commission) * t.quantity) // (价格 - 佣金) * 数量
            .sum::<Decimal>();

        let total_loss = loss_trades.iter() // 计算总亏损
            .map(|t| (t.price + t.commission) * t.quantity) // (价格 + 佣金) * 数量
            .sum::<Decimal>();

        if total_loss.is_zero() { // 如果总亏损为零
            return if total_profit.is_zero() { Decimal::one() } else { Decimal::MAX }; // 如果总盈利也为零，则盈利因子为 1，否则为最大值（表示无限盈利）
        }

        total_profit / total_loss // 计算盈利因子（总盈利 / 总亏损）
    }

    fn calculate_sharpe_ratio(&self, returns: &[f64]) -> f64 { // 定义 calculate_sharpe_ratio 方法，计算夏普比率
        if returns.is_empty() { // 如果收益率序列为空，夏普比率为零
            return 0.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64; // 计算平均收益率
        let volatility = returns.iter() // 计算波动率（收益率的标准差）
            .map(|r| (r - mean_return).powi(2)) // (收益率 - 平均收益率)^2
            .sum::<f64>() // 求和
            .sqrt() * (252.0_f64).sqrt(); // 开方并年化（乘以 sqrt(252)，假设一年有 252 个交易日）

        if volatility == 0.0 { // 如果波动率为零，夏普比率为零（避免除以零错误）
            return 0.0;
        }

        (mean_return * 252.0 - self.risk_free_rate) / volatility // 计算夏普比率（(年化平均收益率 - 无风险利率) / 年化波动率）
    }

    fn calculate_sortino_ratio(&self, returns: &[f64]) -> f64 { // 定义 calculate_sortino_ratio 方法，计算索提诺比率
        if returns.is_empty() { // 如果收益率序列为空，索提诺比率为零
            return 0.0;
        }

        let mean_return = returns.iter().sum::<f64>() / returns.len() as f64; // 计算平均收益率
        let downside_returns: Vec<f64> = returns.iter() // 筛选出下行收益率（负收益率）
            .filter(|&&r| r < 0.0) // 筛选小于零的收益率
            .map(|&r| r.powi(2)) // 对每个下行收益率求平方
            .collect(); // 收集到 Vec<f64>

        if downside_returns.is_empty() { // 如果没有下行收益率，索提诺比率为零
            return 0.0;
        }

        let downside_deviation = (downside_returns.iter().sum::<f64>() / downside_returns.len() as f64).sqrt() // 计算下行标准差
            * (252.0_f64).sqrt(); // 年化下行标准差

        if downside_deviation == 0.0 { // 如果下行标准差为零，索提诺比率为零（避免除以零错误）
            return 0.0;
        }

        (mean_return * 252.0 - self.risk_free_rate) / downside_deviation // 计算索提诺比率（(年化平均收益率 - 无风险利率) / 年化下行标准差）
    }

    fn calculate_avg_profit(&self, trades: &[Trade]) -> Decimal { // 定义 calculate_avg_profit 方法，计算平均每笔交易利润
        if trades.is_empty() { // 如果交易记录为空，平均利润为零
            return Decimal::zero();
        }

        let total_profit = trades.iter() // 计算总利润
            .map(|t| t.price * t.quantity - t.commission) // (价格 * 数量) - 佣金
            .sum::<Decimal>();

        total_profit / Decimal::from(trades.len()) // 计算平均利润（总利润 / 交易次数）
    }

    fn calculate_total_volume(&self, trades: &[Trade]) -> Decimal { // 定义 calculate_total_volume 方法，计算总交易量
        trades.iter() // 遍历交易记录
            .map(|t| t.quantity * t.price) // 数量 * 价格
            .sum() // 求和得到总交易量
    }
}

// TODO: 待实现的辅助函数 // TODO: 待实现的辅助函数
// fn calculate_avg_winning_trade() // 计算平均盈利交易额
// fn calculate_avg_losing_trade() // 计算平均亏损交易额
// fn find_largest_profit() // 查找最大单笔盈利
// fn find_largest_loss() // 查找最大单笔亏损
// fn calculate_avg_trade_duration() // 计算平均持仓时间
// fn calculate_monthly_profit() // 计算月均利润
// fn calculate_annual_return() // 计算年化回报率
// fn calculate_monthly_sharpe() // 计算月度夏普比率
// fn calculate_avg_position_size() // 计算平均头寸规模