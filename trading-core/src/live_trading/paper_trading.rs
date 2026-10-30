// src/live_trading/paper_trading.rs
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use trading_common::backtest::engine::affordable_quantity;
use trading_common::backtest::strategy::{Signal, Strategy};
use trading_common::data::cache::TickDataCache;
use trading_common::data::candle_builder::CandleBuilder;
use trading_common::data::repository::TickDataRepository;
use trading_common::data::types::{LiveStrategyLog, TickData, Timeframe};

const LOG_CHANNEL_CAPACITY: usize = 1000;

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
    // Same convention as `BacktestConfig::commission_rate` (0.001 = 0.1%),
    // so live P&L is comparable to backtest P&L.
    commission_rate: Decimal,

    // Cash is a single shared pool (buying one symbol spends capital that
    // could have gone to another), but positions and last-seen prices are
    // tracked per symbol so subscribing to multiple symbols doesn't mix
    // their holdings into one another.
    cash: Decimal,
    positions: HashMap<String, SymbolPosition>,
    last_prices: HashMap<String, Decimal>,
    total_trades: u64,

    // When the strategy trades on candles (`supports_ohlc` + a preferred
    // timeframe), ticks are folded into per-symbol in-memory candles and the
    // strategy only sees `on_ohlc` for completed ones — the same thing a
    // backtest feeds it. Otherwise it gets every tick via `on_tick`.
    ohlc_timeframe: Option<Timeframe>,
    candle_builders: HashMap<String, CandleBuilder>,

    // `LiveStrategyLog` rows are handed off here instead of being written
    // to Postgres inline — see `spawn_log_writer`.
    log_tx: mpsc::Sender<LiveStrategyLog>,
}

impl PaperTradingProcessor {
    pub fn new(
        strategy: Box<dyn Strategy + Send>,
        repository: Arc<TickDataRepository>,
        initial_capital: Decimal,
        commission_rate: Decimal,
    ) -> Self {
        let (log_tx, log_rx) = mpsc::channel(LOG_CHANNEL_CAPACITY);
        spawn_log_writer(repository.clone(), log_rx);

        let ohlc_timeframe = if strategy.supports_ohlc() {
            strategy.preferred_timeframe()
        } else {
            None
        };
        match ohlc_timeframe {
            Some(timeframe) => info!(
                "Paper trading: feeding {} {} candles to the strategy",
                timeframe.as_str(),
                strategy.name()
            ),
            None => info!(
                "Paper trading: feeding raw ticks to the {} strategy",
                strategy.name()
            ),
        }

        Self {
            strategy,
            repository,
            initial_capital,
            commission_rate,
            cash: initial_capital,
            positions: HashMap::new(),
            last_prices: HashMap::new(),
            total_trades: 0,
            ohlc_timeframe,
            candle_builders: HashMap::new(),
            log_tx,
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

        // 2. Strategy decision: candles for OHLC strategies, raw ticks otherwise.
        let signal = match self.ohlc_timeframe {
            Some(timeframe) => {
                let completed = self
                    .candle_builders
                    .entry(tick.symbol.clone())
                    .or_insert_with(|| CandleBuilder::new(timeframe))
                    .push(tick);
                match completed {
                    Some(candle) => self.strategy.on_ohlc(&candle),
                    None => Signal::Hold,
                }
            }
            None => self.strategy.on_tick(tick),
        };

        // 3. Execution of trading signals
        let signal_type = self.execute_signal(&signal, tick)?;

        // 4. Calculate Portfolio Value
        let portfolio_value = self.calculate_portfolio_value();
        let total_pnl = portfolio_value - self.initial_capital;

        // 5. Hand the log row off to the background writer instead of
        // awaiting a DB round-trip here. This used to be the single
        // biggest latency cost on this path: the strategy couldn't move on
        // to the next tick until the INSERT came back.
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

        if let Err(e) = self.log_tx.try_send(log) {
            match e {
                mpsc::error::TrySendError::Full(_) => {
                    warn!("Strategy log writer is falling behind, dropping a log entry");
                }
                mpsc::error::TrySendError::Closed(_) => {
                    // Background writer task has stopped; nothing we can do here.
                }
            }
        }

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

    /// Sizes and executes a signal with the same rules as the backtest
    /// engine (all-in buys when unsized, full close on unsized sells,
    /// commission charged on both sides), and tells the strategy only about
    /// orders that actually filled.
    fn execute_signal(&mut self, signal: &Signal, tick: &TickData) -> Result<String, String> {
        match signal {
            Signal::Buy { quantity, .. } => {
                let quantity = quantity.or_else(|| {
                    affordable_quantity(self.cash, Decimal::ONE, tick.price, self.commission_rate)
                });
                let Some(quantity) = quantity else {
                    debug!("BUY skipped {}: no funds available", tick.symbol);
                    return Ok("HOLD".to_string());
                };

                let cost = quantity * tick.price;
                let commission = cost * self.commission_rate;
                let total_cost = cost + commission;
                if total_cost > self.cash {
                    debug!(
                        "BUY signal ignored: insufficient cash ({} needed, {} available)",
                        total_cost, self.cash
                    );
                    return Ok("HOLD".to_string());
                }

                let position = self
                    .positions
                    .entry(tick.symbol.clone())
                    .or_insert_with(SymbolPosition::flat);
                // Average cost includes commission, matching `Portfolio::execute_buy`.
                if position.quantity == Decimal::ZERO {
                    position.quantity = quantity;
                    position.avg_cost = total_cost / quantity;
                } else {
                    let total = (position.quantity * position.avg_cost) + total_cost;
                    position.quantity += quantity;
                    position.avg_cost = total / position.quantity;
                }
                let held = position.quantity;

                self.cash -= total_cost;
                self.total_trades += 1;
                self.strategy.on_fill(signal);

                debug!(
                    "BUY executed: {} {} @ {}, position: {}, cash: {}",
                    tick.symbol, quantity, tick.price, held, self.cash
                );
                Ok("BUY".to_string())
            }

            Signal::Sell { quantity, .. } => {
                let held = self
                    .positions
                    .get(&tick.symbol)
                    .map(|position| position.quantity)
                    .unwrap_or(Decimal::ZERO);
                let quantity = quantity.or_else(|| (held > Decimal::ZERO).then_some(held));
                let Some(quantity) = quantity else {
                    debug!("SELL skipped {}: no position to sell", tick.symbol);
                    return Ok("HOLD".to_string());
                };
                if quantity > held {
                    debug!(
                        "SELL signal ignored: insufficient position ({} needed, {} available)",
                        quantity, held
                    );
                    return Ok("HOLD".to_string());
                }

                let proceeds = quantity * tick.price;
                let commission = proceeds * self.commission_rate;

                let position = self
                    .positions
                    .entry(tick.symbol.clone())
                    .or_insert_with(SymbolPosition::flat);
                position.quantity -= quantity;
                if position.quantity == Decimal::ZERO {
                    position.avg_cost = Decimal::ZERO;
                }
                let remaining = position.quantity;

                self.cash += proceeds - commission;
                self.total_trades += 1;
                self.strategy.on_fill(signal);

                debug!(
                    "SELL executed: {} {} @ {}, position: {}, cash: {}",
                    tick.symbol, quantity, tick.price, remaining, self.cash
                );
                Ok("SELL".to_string())
            }

            Signal::Hold => Ok("HOLD".to_string()),
        }
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

/// Writes `LiveStrategyLog` rows off the tick-processing hot path.
///
/// Runs until every `Sender` for `log_rx` is dropped (i.e. the owning
/// `PaperTradingProcessor` is gone), then exits. There's no batch INSERT for
/// `live_strategy_log` yet, so this still issues one INSERT per row — the
/// point of this task is only to get those INSERTs off the synchronous
/// decision path (where they used to block the strategy from moving on to
/// the next tick), not to batch them.
fn spawn_log_writer(
    repository: Arc<TickDataRepository>,
    mut log_rx: mpsc::Receiver<LiveStrategyLog>,
) {
    tokio::spawn(async move {
        while let Some(log) = log_rx.recv().await {
            if let Err(e) = repository.insert_live_strategy_log(&log).await {
                warn!("Failed to write live strategy log: {}", e);
            }
        }
    });
}
