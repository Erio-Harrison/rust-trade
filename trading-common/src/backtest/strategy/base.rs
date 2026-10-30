use crate::data::types::{OHLCData, TickData};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Signal {
    /// Open or add to a long position. `quantity: None` lets the executor
    /// size the order from the capital actually available — a strategy has
    /// no way of knowing that itself.
    Buy {
        symbol: String,
        quantity: Option<Decimal>,
    },
    /// Reduce or close a long position. `quantity: None` closes it entirely.
    Sell {
        symbol: String,
        quantity: Option<Decimal>,
    },
    Hold,
}

pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn on_tick(&mut self, tick: &TickData) -> Signal;
    fn initialize(&mut self, params: HashMap<String, String>) -> Result<(), String>;

    /// Called by the executor after a signal was actually filled.
    ///
    /// Strategies that track whether they are in a position must update that
    /// state here rather than when emitting the signal: an emitted order can
    /// be rejected (no funds, no position), and a strategy that assumes it
    /// filled drifts out of sync with the real portfolio.
    fn on_fill(&mut self, _signal: &Signal) {}

    /// Reset strategy state for new backtest
    fn reset(&mut self) {
        // Default implementation does nothing
        // Strategies can override if needed
    }

    fn on_ohlc(&mut self, _ohlc: &OHLCData) -> Signal {
        Signal::Hold
    }
    fn supports_ohlc(&self) -> bool {
        false
    }
    fn preferred_timeframe(&self) -> Option<crate::data::types::Timeframe> {
        None
    }
}
