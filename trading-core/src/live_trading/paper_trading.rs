// src/live_trading/paper_trading.rs
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

use crate::backtest::strategy::base::{Signal, Strategy};
use crate::data::cache::TickDataCache;
use crate::data::repository::TickDataRepository;
use crate::data::types::{LiveStrategyLog, TickData};

pub struct PaperTradingProcessor {
    strategy: Box<dyn Strategy + Send>,
    repository: Arc<TickDataRepository>,
    initial_capital: Decimal,

    //Simple status tracking
    cash: Decimal,
    position: Decimal,
    avg_cost: Decimal,
    total_trades: u64,
}

impl PaperTradingProcessor {
    pub fn new(
        strategy: Box<dyn Strategy + Send>,
        repository: Arc<TickDataRepository>,
        initial_capital: Decimal,
    ) -> Self {
        Self {
            strategy,
            repository,
            initial_capital,
            cash: initial_capital,
            position: Decimal::ZERO,
            avg_cost: Decimal::ZERO,
            total_trades: 0,
        }
    }

    pub async fn process_tick(&mut self, tick: &TickData) -> Result<(), String> {
        let start_time = Instant::now();

        // 1. Get data from cache
        let cache_start = Instant::now();
        let recent_ticks = self
            .repository
            .get_cache()
            .get_recent_ticks(&tick.symbol, 20)
            .await
            .map_err(|e| format!("Cache error: {}", e))?;
        let cache_hit = !recent_ticks.is_empty();
        let cache_time = cache_start.elapsed().as_micros() as u64;

        // 2. Policy Handle - Using Existing Policies
        let signal = self.strategy.on_tick(tick);

        // 3. Execution of trading signals
        let signal_type = self.execute_signal(&signal, tick)?;

        // 4. Calculate Portfolio Value
        let portfolio_value = self.calculate_portfolio_value(tick.price);
        let total_pnl = portfolio_value - self.initial_capital;

        // 5. Record to database
        let processing_time = start_time.elapsed().as_micros() as u64;
        let log = LiveStrategyLog {
            timestamp: tick.timestamp,
            strategy_id: self.strategy.name().to_string(),
            symbol: tick.symbol.clone(),
            current_price: tick.price,
            signal_type: signal_type.clone(),
            portfolio_value,
            total_pnl,
            cache_hit,
            processing_time_us: processing_time,
        };

        self.repository
            .insert_live_strategy_log(&log)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

        // 6. Real-time output
        self.log_activity(
            &signal_type,
            tick,
            portfolio_value,
            total_pnl,
            cache_hit,
            cache_time,
            processing_time,
        );

        Ok(())
    }

    fn execute_signal(&mut self, signal: &Signal, tick: &TickData) -> Result<String, String> {
        match signal {
            Signal::Buy { quantity, .. } => {
                let cost = quantity * tick.price;

                if cost <= self.cash {
                    if self.position == Decimal::ZERO {
                        self.position = *quantity;
                        self.avg_cost = tick.price;
                    } else {
                        let total_cost = (self.position * self.avg_cost) + cost;
                        self.position += quantity;
                        self.avg_cost = total_cost / self.position;
                    }

                    self.cash -= cost;
                    self.total_trades += 1;

                    debug!(
                        "BUY executed: {} @ {}, position: {}, cash: {}",
                        quantity, tick.price, self.position, self.cash
                    );
                    return Ok("BUY".to_string());
                } else {
                    debug!(
                        "BUY signal ignored: insufficient cash ({} needed, {} available)",
                        cost, self.cash
                    );
                }
            }

            Signal::Sell { quantity, .. } => {
                if *quantity <= self.position {
                    let proceeds = quantity * tick.price;
                    self.cash += proceeds;
                    self.position -= quantity;
                    self.total_trades += 1;

                    if self.position == Decimal::ZERO {
                        self.avg_cost = Decimal::ZERO;
                    }

                    debug!(
                        "SELL executed: {} @ {}, position: {}, cash: {}",
                        quantity, tick.price, self.position, self.cash
                    );
                    return Ok("SELL".to_string());
                } else {
                    debug!(
                        "SELL signal ignored: insufficient position ({} needed, {} available)",
                        quantity, self.position
                    );
                }
            }

            Signal::Hold => return Ok("HOLD".to_string()),
        }

        Ok("HOLD".to_string())
    }

    fn calculate_portfolio_value(&self, current_price: Decimal) -> Decimal {
        self.cash + (self.position * current_price)
    }

    fn log_activity(
        &self,
        signal_type: &str,
        tick: &TickData,
        portfolio_value: Decimal,
        total_pnl: Decimal,
        cache_hit: bool,
        cache_time_us: u64,
        total_time_us: u64,
    ) {
        if signal_type != "HOLD" {
            let return_pct = if self.initial_capital > Decimal::ZERO {
                total_pnl / self.initial_capital * Decimal::from(100)
            } else {
                Decimal::ZERO
            };

            println!("🎯 {} {} @ ${} | Portfolio: ${} | P&L: ${} ({:.2}%) | Position: {} | Cash: ${} | Trades: {} | Cache: {} ({}μs) | Total: {}μs",
                     signal_type,
                     tick.symbol,
                     tick.price,
                     portfolio_value,
                     total_pnl,
                     return_pct,
                     self.position,
                     self.cash,
                     self.total_trades,
                     if cache_hit { "HIT" } else { "MISS" },
                     cache_time_us,
                     total_time_us);
        } else {
            if tick.timestamp.timestamp() % 10 == 0 {
                println!(
                    "📊 {} {} @ ${} | Portfolio: ${} | P&L: ${} | Cache: {} ({}μs)",
                    tick.symbol,
                    if cache_hit { "HIT" } else { "MISS" },
                    tick.price,
                    portfolio_value,
                    total_pnl,
                    if cache_hit { "✓" } else { "✗" },
                    cache_time_us
                );
            }
        }
    }

    pub fn get_status(&self) -> PaperTradingStatus {
        PaperTradingStatus {
            strategy_name: self.strategy.name().to_string(),
            cash: self.cash,
            position: self.position,
            avg_cost: self.avg_cost,
            total_trades: self.total_trades,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PaperTradingStatus {
    pub strategy_name: String,
    pub cash: Decimal,
    pub position: Decimal,
    pub avg_cost: Decimal,
    pub total_trades: u64,
}
