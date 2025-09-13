use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct BacktestMetrics;

impl BacktestMetrics {
    /// Calculate Sharpe ratio
    /// Sharpe Ratio = (Mean Return - Risk Free Rate) / Standard Deviation of Returns
    pub fn calculate_sharpe_ratio(returns: &[Decimal], risk_free_rate: Decimal) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let mean_return = Self::calculate_mean(returns);
        let std_dev = Self::calculate_standard_deviation(returns);

        if std_dev == Decimal::ZERO {
            return Decimal::ZERO;
        }

        (mean_return - risk_free_rate) / std_dev
    }

    /// Calculate maximum drawdown
    /// Max Drawdown = Max((Peak - Trough) / Peak) over all time periods
    pub fn calculate_max_drawdown(equity_curve: &[Decimal]) -> Decimal {
        if equity_curve.len() < 2 {
            return Decimal::ZERO;
        }

        let mut max_drawdown = Decimal::ZERO;
        let mut peak = equity_curve[0];

        for &value in equity_curve.iter().skip(1) {
            if value > peak {
                peak = value;
            }

            let drawdown = if peak > Decimal::ZERO {
                (peak - value) / peak
            } else {
                Decimal::ZERO
            };

            if drawdown > max_drawdown {
                max_drawdown = drawdown;
            }
        }

        max_drawdown
    }

    /// Calculate volatility (standard deviation of returns)
    pub fn calculate_volatility(returns: &[Decimal]) -> Decimal {
        Self::calculate_standard_deviation(returns)
    }

    /// Calculate Calmar ratio (Annual Return / Max Drawdown)
    pub fn calculate_calmar_ratio(annual_return: Decimal, max_drawdown: Decimal) -> Decimal {
        if max_drawdown == Decimal::ZERO {
            return Decimal::ZERO;
        }
        annual_return / max_drawdown
    }

    /// Calculate Sortino ratio (uses downside deviation instead of total volatility)
    pub fn calculate_sortino_ratio(
        returns: &[Decimal],
        risk_free_rate: Decimal,
        target_return: Decimal,
    ) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let mean_return = Self::calculate_mean(returns);
        let downside_deviation = Self::calculate_downside_deviation(returns, target_return);

        if downside_deviation == Decimal::ZERO {
            return Decimal::ZERO;
        }

        (mean_return - risk_free_rate) / downside_deviation
    }

    /// Calculate Value at Risk (VaR) at given confidence level
    pub fn calculate_var(returns: &[Decimal], confidence_level: Decimal) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let mut sorted_returns = returns.to_vec();
        sorted_returns.sort();

        let index = ((Decimal::ONE - confidence_level) * Decimal::from(sorted_returns.len()))
            .to_usize()
            .unwrap_or(0);
        if index < sorted_returns.len() {
            sorted_returns[index]
        } else {
            Decimal::ZERO
        }
    }

    /// Calculate information ratio
    pub fn calculate_information_ratio(
        portfolio_returns: &[Decimal],
        benchmark_returns: &[Decimal],
    ) -> Decimal {
        if portfolio_returns.len() != benchmark_returns.len() || portfolio_returns.is_empty() {
            return Decimal::ZERO;
        }

        let excess_returns: Vec<Decimal> = portfolio_returns
            .iter()
            .zip(benchmark_returns.iter())
            .map(|(p, b)| p - b)
            .collect();

        let mean_excess_return = Self::calculate_mean(&excess_returns);
        let tracking_error = Self::calculate_standard_deviation(&excess_returns);

        if tracking_error == Decimal::ZERO {
            return Decimal::ZERO;
        }

        mean_excess_return / tracking_error
    }

    /// Calculate win rate (percentage of profitable trades)
    pub fn calculate_win_rate(trades: &[crate::backtest::portfolio::Trade]) -> Decimal {
        if trades.is_empty() {
            return Decimal::ZERO;
        }

        let profitable_trades = trades
            .iter()
            .filter(|trade| trade.realized_pnl.map_or(false, |pnl| pnl > Decimal::ZERO))
            .count();

        Decimal::from(profitable_trades) / Decimal::from(trades.len()) * Decimal::from(100)
    }

    /// Calculate profit factor (total profit / total loss)
    pub fn calculate_profit_factor(trades: &[crate::backtest::portfolio::Trade]) -> Decimal {
        let (total_profit, total_loss) = trades.iter().filter_map(|trade| trade.realized_pnl).fold(
            (Decimal::ZERO, Decimal::ZERO),
            |(profit, loss), pnl| {
                if pnl > Decimal::ZERO {
                    (profit + pnl, loss)
                } else {
                    (profit, loss - pnl)
                }
            },
        );

        if total_loss == Decimal::ZERO {
            return if total_profit > Decimal::ZERO {
                Decimal::MAX
            } else {
                Decimal::ZERO
            };
        }

        total_profit / total_loss
    }

    /// Calculate average trade duration
    pub fn calculate_average_trade_duration(trades: &[crate::backtest::portfolio::Trade]) -> f64 {
        if trades.len() < 2 {
            return 0.0;
        }

        let mut durations = Vec::new();
        let mut open_positions: HashMap<String, chrono::DateTime<chrono::Utc>> = HashMap::new();

        for trade in trades {
            match trade.side {
                crate::data::types::TradeSide::Buy => {
                    open_positions.insert(trade.symbol.clone(), trade.timestamp);
                }
                crate::data::types::TradeSide::Sell => {
                    if let Some(open_time) = open_positions.remove(&trade.symbol) {
                        let duration = trade.timestamp.signed_duration_since(open_time);
                        durations.push(duration.num_seconds() as f64);
                    }
                }
            }
        }

        if durations.is_empty() {
            0.0
        } else {
            durations.iter().sum::<f64>() / durations.len() as f64
        }
    }

    // Helper functions

    fn calculate_mean(values: &[Decimal]) -> Decimal {
        if values.is_empty() {
            return Decimal::ZERO;
        }
        values.iter().sum::<Decimal>() / Decimal::from(values.len())
    }

    fn calculate_standard_deviation(values: &[Decimal]) -> Decimal {
        if values.len() < 2 {
            return Decimal::ZERO;
        }

        let mean = Self::calculate_mean(values);
        let variance = values
            .iter()
            .map(|x| (x - mean) * (x - mean))
            .sum::<Decimal>()
            / Decimal::from(values.len() - 1);

        // Approximate square root using Newton's method
        Self::decimal_sqrt(variance)
    }

    fn calculate_downside_deviation(returns: &[Decimal], target_return: Decimal) -> Decimal {
        if returns.is_empty() {
            return Decimal::ZERO;
        }

        let downside_returns: Vec<Decimal> = returns
            .iter()
            .filter(|&&r| r < target_return)
            .map(|&r| r - target_return)
            .collect();

        if downside_returns.is_empty() {
            return Decimal::ZERO;
        }

        let downside_variance = downside_returns.iter().map(|x| x * x).sum::<Decimal>()
            / Decimal::from(downside_returns.len());

        Self::decimal_sqrt(downside_variance)
    }

    /// Approximate square root for Decimal using Newton's method
    fn decimal_sqrt(value: Decimal) -> Decimal {
        if value <= Decimal::ZERO {
            return Decimal::ZERO;
        }

        let mut x = value / Decimal::from(2);
        let tolerance = Decimal::from_str("0.000001").unwrap();

        for _ in 0..50 {
            // Max iterations
            let new_x = (x + value / x) / Decimal::from(2);
            if (new_x - x).abs() < tolerance {
                break;
            }
            x = new_x;
        }

        x
    }
}
