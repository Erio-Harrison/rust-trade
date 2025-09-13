use crate::data::types::TickData;
use crate::backtest::{strategy::Strategy, portfolio::Portfolio, metrics::BacktestMetrics};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    pub initial_capital: Decimal,
    pub commission_rate: Decimal,
    pub strategy_params: HashMap<String, String>,
}

impl BacktestConfig {
    pub fn new(initial_capital: Decimal) -> Self {
        Self {
            initial_capital,
            commission_rate: Decimal::from_str("0.001").unwrap_or(Decimal::ZERO), // 0.1% default
            strategy_params: HashMap::new(),
        }
    }
    
    pub fn with_commission_rate(mut self, rate: Decimal) -> Self {
        self.commission_rate = rate;
        self
    }
    
    pub fn with_param(mut self, key: &str, value: &str) -> Self {
        self.strategy_params.insert(key.to_string(), value.to_string());
        self
    }
}

pub struct BacktestEngine {
    portfolio: Portfolio,
    strategy: Box<dyn Strategy>,
    config: BacktestConfig,
}

impl BacktestEngine {
    pub fn new(strategy: Box<dyn Strategy>, config: BacktestConfig) -> Result<Self, String> {
        let mut strategy = strategy;
        strategy.reset();
        strategy.initialize(config.strategy_params.clone())?;
        
        let portfolio = Portfolio::new(config.initial_capital)
            .with_commission_rate(config.commission_rate);
        
        Ok(Self {
            portfolio,
            strategy,
            config,
        })
    }
    
    pub fn run(&mut self, data: Vec<TickData>) -> BacktestResult {
        println!("Starting backtest...");
        println!("Strategy: {}", self.strategy.name());
        println!("Initial capital: ${}", self.portfolio.initial_capital);
        println!("Data points: {}", data.len());
        println!("Commission rate: {}%", self.config.commission_rate * Decimal::from(100));
        println!("{}", "=".repeat(60));
        
        let mut processed = 0;
        let total = data.len();
        let mut last_progress = 0;
        
        for tick in data {
            // Update current price
            self.portfolio.update_price(&tick.symbol, tick.price);
            
            // Execute strategy
            let signal = self.strategy.on_tick(&tick);
            
            // Execute trades
            match signal {
                crate::backtest::strategy::Signal::Buy { symbol, quantity } => {
                    if let Err(e) = self.portfolio.execute_buy(symbol.clone(), quantity, tick.price) {
                        println!("Buy failed {}: {}", symbol, e);
                    } else {
                        println!("BUY {} {} @ ${}", symbol, quantity, tick.price);
                    }
                },
                crate::backtest::strategy::Signal::Sell { symbol, quantity } => {
                    if let Err(e) = self.portfolio.execute_sell(symbol.clone(), quantity, tick.price) {
                        println!("Sell failed {}: {}", symbol, e);
                    } else {
                        println!("SELL {} {} @ ${}", symbol, quantity, tick.price);
                    }
                },
                crate::backtest::strategy::Signal::Hold => {}
            }
            
            processed += 1;
            
            // Progress display
            let progress = (processed * 100) / total;
            if progress != last_progress && progress % 10 == 0 {
                let current_value = self.portfolio.total_value();
                let current_pnl = self.portfolio.total_pnl();
                println!("Progress: {}% ({}/{}) | Portfolio Value: ${} | P&L: ${}", 
                         progress, processed, total, current_value, current_pnl);
                last_progress = progress;
            }
        }
        
        println!("\n{}", "=".repeat(60));
        
        // Calculate results
        let final_value = self.portfolio.total_value();
        let total_pnl = self.portfolio.total_pnl();
        let total_return_pct = if self.portfolio.initial_capital > Decimal::ZERO {
            (total_pnl / self.portfolio.initial_capital) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        // Calculate performance metrics
        let equity_curve = self.portfolio.get_equity_curve();
        let returns = Self::calculate_returns(&equity_curve);
        
        let max_drawdown = BacktestMetrics::calculate_max_drawdown(&equity_curve);
        let sharpe_ratio = BacktestMetrics::calculate_sharpe_ratio(&returns, Decimal::ZERO);
        let volatility = BacktestMetrics::calculate_volatility(&returns);
        let win_rate = BacktestMetrics::calculate_win_rate(&self.portfolio.trades);
        let profit_factor = BacktestMetrics::calculate_profit_factor(&self.portfolio.trades);
        let avg_trade_duration = BacktestMetrics::calculate_average_trade_duration(&self.portfolio.trades);
        
        BacktestResult {
            initial_capital: self.portfolio.initial_capital,
            final_value,
            total_pnl,
            return_percentage: total_return_pct,
            total_trades: self.portfolio.trades.len(),
            winning_trades: self.count_winning_trades(),
            losing_trades: self.count_losing_trades(),
            max_drawdown,
            sharpe_ratio,
            volatility,
            win_rate,
            profit_factor,
            avg_trade_duration_seconds: avg_trade_duration,
            total_commission: self.portfolio.total_commission(),
            positions: self.portfolio.positions.clone(),
            trades: self.portfolio.trades.clone(),
            equity_curve,
            strategy_name: self.strategy.name().to_string(),
        }
    }
    
    fn calculate_returns(equity_curve: &[Decimal]) -> Vec<Decimal> {
        if equity_curve.len() < 2 {
            return Vec::new();
        }
        
        equity_curve.windows(2)
            .map(|window| {
                if window[0] > Decimal::ZERO {
                    (window[1] - window[0]) / window[0]
                } else {
                    Decimal::ZERO
                }
            })
            .collect()
    }
    
    fn count_winning_trades(&self) -> usize {
        self.portfolio.trades.iter()
            .filter(|trade| trade.realized_pnl.map_or(false, |pnl| pnl > Decimal::ZERO))
            .count()
    }
    
    fn count_losing_trades(&self) -> usize {
        self.portfolio.trades.iter()
            .filter(|trade| trade.realized_pnl.map_or(false, |pnl| pnl < Decimal::ZERO))
            .count()
    }

    pub fn run_with_ohlc(&mut self, data: Vec<crate::data::types::OHLCData>) -> BacktestResult {
        println!("Starting OHLC backtest...");
        println!("Strategy: {}", self.strategy.name());
        println!("Initial capital: ${}", self.portfolio.initial_capital);
        println!("Data points: {} OHLC candles", data.len());
        println!("Commission rate: {}%", self.config.commission_rate * Decimal::from(100));
        println!("{}", "=".repeat(60));
        
        let mut processed = 0;
        let total = data.len();
        let mut last_progress = 0;
        
        for ohlc in data {
            // Update current price using close price
            self.portfolio.update_price(&ohlc.symbol, ohlc.close);
            
            // Execute strategy with OHLC data
            let signal = self.strategy.on_ohlc(&ohlc);
            
            // Execute trades using close price
            match signal {
                crate::backtest::strategy::Signal::Buy { symbol, quantity } => {
                    if let Err(e) = self.portfolio.execute_buy(symbol.clone(), quantity, ohlc.close) {
                        println!("Buy failed {}: {}", symbol, e);
                    } else {
                        println!("BUY {} {} @ ${}", symbol, quantity, ohlc.close);
                    }
                },
                crate::backtest::strategy::Signal::Sell { symbol, quantity } => {
                    if let Err(e) = self.portfolio.execute_sell(symbol.clone(), quantity, ohlc.close) {
                        println!("Sell failed {}: {}", symbol, e);
                    } else {
                        println!("SELL {} {} @ ${}", symbol, quantity, ohlc.close);
                    }
                },
                crate::backtest::strategy::Signal::Hold => {}
            }
            
            processed += 1;
            
            // Progress display
            let progress = (processed * 100) / total;
            if progress != last_progress && progress % 10 == 0 {
                let current_value = self.portfolio.total_value();
                let current_pnl = self.portfolio.total_pnl();
                println!("Progress: {}% ({}/{}) | Portfolio Value: ${} | P&L: ${}", 
                        progress, processed, total, current_value, current_pnl);
                last_progress = progress;
            }
        }
        
        println!("\n{}", "=".repeat(60));
        
        // Calculate results (same logic as original run method)
        let final_value = self.portfolio.total_value();
        let total_pnl = self.portfolio.total_pnl();
        let total_return_pct = if self.portfolio.initial_capital > Decimal::ZERO {
            (total_pnl / self.portfolio.initial_capital) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        
        // Calculate performance metrics
        let equity_curve = self.portfolio.get_equity_curve();
        let returns = Self::calculate_returns(&equity_curve);
        
        let max_drawdown = BacktestMetrics::calculate_max_drawdown(&equity_curve);
        let sharpe_ratio = BacktestMetrics::calculate_sharpe_ratio(&returns, Decimal::ZERO);
        let volatility = BacktestMetrics::calculate_volatility(&returns);
        let win_rate = BacktestMetrics::calculate_win_rate(&self.portfolio.trades);
        let profit_factor = BacktestMetrics::calculate_profit_factor(&self.portfolio.trades);
        let avg_trade_duration = BacktestMetrics::calculate_average_trade_duration(&self.portfolio.trades);
        
        BacktestResult {
            initial_capital: self.portfolio.initial_capital,
            final_value,
            total_pnl,
            return_percentage: total_return_pct,
            total_trades: self.portfolio.trades.len(),
            winning_trades: self.count_winning_trades(),
            losing_trades: self.count_losing_trades(),
            max_drawdown,
            sharpe_ratio,
            volatility,
            win_rate,
            profit_factor,
            avg_trade_duration_seconds: avg_trade_duration,
            total_commission: self.portfolio.total_commission(),
            positions: self.portfolio.positions.clone(),
            trades: self.portfolio.trades.clone(),
            equity_curve,
            strategy_name: self.strategy.name().to_string(),
        }
    }    
}

#[derive(Debug)]
pub struct BacktestResult {
    pub initial_capital: Decimal,
    pub final_value: Decimal,
    pub total_pnl: Decimal,
    pub return_percentage: Decimal,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub max_drawdown: Decimal,
    pub sharpe_ratio: Decimal,
    pub volatility: Decimal,
    pub win_rate: Decimal,
    pub profit_factor: Decimal,
    pub avg_trade_duration_seconds: f64,
    pub total_commission: Decimal,
    pub positions: HashMap<String, crate::backtest::portfolio::Position>,
    pub trades: Vec<crate::backtest::portfolio::Trade>,
    pub equity_curve: Vec<Decimal>,
    pub strategy_name: String,
}

impl BacktestResult {
    pub fn print_summary(&self) {
        println!("BACKTEST RESULTS SUMMARY");
        println!("{}", "=".repeat(60));
        println!("Strategy: {}", self.strategy_name);
        println!("Initial Capital: ${}", self.initial_capital);
        println!("Final Value: ${}", self.final_value);
        println!("Total P&L: ${}", self.total_pnl);
        println!("Return: {:.2}%", self.return_percentage);
        println!("Total Commission: ${}", self.total_commission);
        println!();
        
        println!("TRADING STATISTICS");
        println!("{}", "-".repeat(30));
        println!("Total Trades: {}", self.total_trades);
        
        if self.total_trades > 0 {
            println!("Winning Trades: {} ({:.1}%)", 
                     self.winning_trades, self.win_rate);
            println!("Losing Trades: {} ({:.1}%)", 
                     self.losing_trades, 
                     100.0 - self.win_rate.to_f64().unwrap_or(0.0));
            println!("Profit Factor: {:.2}", self.profit_factor);
            println!("Avg Trade Duration: {:.0} seconds", self.avg_trade_duration_seconds);
        }
        println!();
        
        println!("RISK METRICS");
        println!("{}", "-".repeat(30));
        println!("Max Drawdown: {:.2}%", self.max_drawdown * Decimal::from(100));
        println!("Sharpe Ratio: {:.2}", self.sharpe_ratio);
        println!("Volatility: {:.2}%", self.volatility * Decimal::from(100));
        println!();
        
        if !self.positions.is_empty() {
            println!("CURRENT POSITIONS");
            println!("{}", "-".repeat(30));
            for (symbol, position) in &self.positions {
                println!("{}: {} @ ${} (Unrealized P&L: ${})", 
                         symbol, position.quantity, position.avg_price, position.unrealized_pnl);
            }
            println!();
        }
        
        if self.total_trades > 0 {
            println!("RECENT TRADES (Last 5)");
            println!("{}", "-".repeat(30));
            for trade in self.trades.iter().rev().take(5) {
                let pnl_str = trade.realized_pnl
                    .map(|pnl| format!("(P&L: ${})", pnl))
                    .unwrap_or_default();
                println!("{} {} {} @ ${} {}",
                         trade.timestamp.format("%Y-%m-%d %H:%M:%S"),
                         match trade.side {
                             crate::data::types::TradeSide::Buy => "BUY ",
                             crate::data::types::TradeSide::Sell => "SELL",
                         },
                         trade.symbol,
                         trade.price,
                         pnl_str);
            }
        }
        
        println!("{}", "=".repeat(60));
    }
    
    pub fn is_profitable(&self) -> bool {
        self.total_pnl > Decimal::ZERO
    }
    
    pub fn calmar_ratio(&self) -> Decimal {
        BacktestMetrics::calculate_calmar_ratio(self.return_percentage / Decimal::from(100), self.max_drawdown)
    }
    
    pub fn print_trade_analysis(&self) {
        if self.trades.is_empty() {
            println!("No trades executed");
            return;
        }
        
        println!("DETAILED TRADE ANALYSIS");
        println!("{}", "=".repeat(80));
        
        let mut buy_trades = Vec::new();
        let mut sell_trades = Vec::new();
        
        for trade in &self.trades {
            match trade.side {
                crate::data::types::TradeSide::Buy => buy_trades.push(trade),
                crate::data::types::TradeSide::Sell => sell_trades.push(trade),
            }
        }
        
        println!("Buy Trades: {}", buy_trades.len());
        println!("Sell Trades: {}", sell_trades.len());
        
        if !sell_trades.is_empty() {
            let profitable_sells = sell_trades.iter()
                .filter(|t| t.realized_pnl.map_or(false, |pnl| pnl > Decimal::ZERO))
                .count();
            
            let total_profit: Decimal = sell_trades.iter()
                .filter_map(|t| t.realized_pnl)
                .filter(|&pnl| pnl > Decimal::ZERO)
                .sum();
            
            let total_loss: Decimal = sell_trades.iter()
                .filter_map(|t| t.realized_pnl)
                .filter(|&pnl| pnl < Decimal::ZERO)
                .sum();
            
            println!("Profitable Sells: {} ({:.1}%)", 
                     profitable_sells, 
                     (profitable_sells as f64 / sell_trades.len() as f64) * 100.0);
            println!("Total Gross Profit: ${}", total_profit);
            println!("Total Gross Loss: ${}", total_loss);
            
            if profitable_sells > 0 {
                println!("Average Profit per Winning Trade: ${}", 
                         total_profit / Decimal::from(profitable_sells));
            }
            
            let losing_sells = sell_trades.len() - profitable_sells;
            if losing_sells > 0 {
                println!("Average Loss per Losing Trade: ${}", 
                         total_loss / Decimal::from(losing_sells));
            }
        }
        
        println!("{}", "=".repeat(80));
    }
}