// src/live_trading/paper_trading.rs
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tracing::debug;

use trading_common::backtest::strategy::{Signal, Strategy};
use trading_common::data::cache::TickDataCache;
use trading_common::data::repository::TickDataRepository;
use trading_common::data::types::{LiveStrategyLog, TickData};

#[derive(Debug, Clone, Copy)]
struct SymbolPosition {
    quantity: Decimal,
    avg_cost: Decimal,
}

impl SymbolPosition {
    fn flat() -> Self {
        Self {
            quantity: Decimal::ZERO,
            avg_cost: Decimal::ZERO,
        }
    }
}

pub struct PaperTradingProcessor {
    strategy: Box<dyn Strategy + Send>,
    repository: Arc<TickDataRepository>,
    initial_capital: Decimal,

    // Cash is a single shared pool (buying one symbol spends capital that
    // could have gone to another), but positions and last-seen prices are
    // tracked per symbol so subscribing to multiple symbols doesn't mix
    // their holdings into one another.
    cash: Decimal,
    positions: HashMap<String, SymbolPosition>,
    last_prices: HashMap<String, Decimal>,
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
            positions: HashMap::new(),
            last_prices: HashMap::new(),
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

        self.last_prices.insert(tick.symbol.clone(), tick.price);

        // 2. Policy Handle - Using Existing Policies
        let signal = self.strategy.on_tick(tick);

        // 3. Execution of trading signals
        let signal_type = self.execute_signal(&signal, tick)?;

        // 4. Calculate Portfolio Value
        let portfolio_value = self.calculate_portfolio_value();
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
                    let position = self
                        .positions
                        .entry(tick.symbol.clone())
                        .or_insert_with(SymbolPosition::flat);

                    if position.quantity == Decimal::ZERO {
                        position.quantity = *quantity;
                        position.avg_cost = tick.price;
                    } else {
                        let total_cost = (position.quantity * position.avg_cost) + cost;
                        position.quantity += quantity;
                        position.avg_cost = total_cost / position.quantity;
                    }

                    self.cash -= cost;
                    self.total_trades += 1;

                    debug!(
                        "BUY executed: {} {} @ {}, position: {}, cash: {}",
                        tick.symbol, quantity, tick.price, position.quantity, self.cash
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
                let position = self
                    .positions
                    .entry(tick.symbol.clone())
                    .or_insert_with(SymbolPosition::flat);

                if *quantity <= position.quantity {
                    let proceeds = quantity * tick.price;
                    self.cash += proceeds;
                    position.quantity -= quantity;
                    self.total_trades += 1;

                    if position.quantity == Decimal::ZERO {
                        position.avg_cost = Decimal::ZERO;
                    }

                    debug!(
                        "SELL executed: {} {} @ {}, position: {}, cash: {}",
                        tick.symbol, quantity, tick.price, position.quantity, self.cash
                    );
                    return Ok("SELL".to_string());
                } else {
                    debug!(
                        "SELL signal ignored: insufficient position ({} needed, {} available)",
                        quantity, position.quantity
                    );
                }
            }

            Signal::Hold => return Ok("HOLD".to_string()),
        }

        Ok("HOLD".to_string())
    }

    /// Cash plus the mark-to-market value of every symbol currently held,
    /// using the last price seen for that symbol.
    fn calculate_portfolio_value(&self) -> Decimal {
        let holdings_value: Decimal = self
            .positions
            .iter()
            .filter(|(_, position)| position.quantity != Decimal::ZERO)
            .map(|(symbol, position)| {
                let price = self
                    .last_prices
                    .get(symbol)
                    .copied()
                    .unwrap_or(position.avg_cost);
                position.quantity * price
            })
            .sum();

        self.cash + holdings_value
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
        let position_qty = self
            .positions
            .get(&tick.symbol)
            .map(|p| p.quantity)
            .unwrap_or(Decimal::ZERO);

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
                     position_qty,
                     self.cash,
                     self.total_trades,
                     if cache_hit { "HIT" } else { "MISS" },
                     cache_time_us,
                     total_time_us);
        } else if tick.timestamp.timestamp() % 10 == 0 {
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
