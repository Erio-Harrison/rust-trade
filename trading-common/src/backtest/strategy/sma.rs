use super::base::{Signal, Strategy};
use crate::data::types::{OHLCData, TickData};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};

pub struct SmaStrategy {
    short_period: usize,
    long_period: usize,
    prices: VecDeque<Decimal>,
    in_position: bool,
}

impl SmaStrategy {
    pub fn new() -> Self {
        Self {
            short_period: 5,
            long_period: 20,
            prices: VecDeque::new(),
            in_position: false,
        }
    }

    fn calculate_sma(&self, period: usize) -> Option<Decimal> {
        if self.prices.len() < period {
            return None;
        }
        let sum: Decimal = self.prices.iter().rev().take(period).sum();
        Some(sum / Decimal::from(period))
    }
}

impl Strategy for SmaStrategy {
    fn name(&self) -> &str {
        "Simple Moving Average"
    }

    fn initialize(&mut self, params: HashMap<String, String>) -> Result<(), String> {
        if let Some(short) = params.get("short_period") {
            self.short_period = short.parse().map_err(|_| "Invalid short_period")?;
        }
        if let Some(long) = params.get("long_period") {
            self.long_period = long.parse().map_err(|_| "Invalid long_period")?;
        }

        if self.short_period >= self.long_period {
            return Err("Short period must be less than long period".to_string());
        }

        println!(
            "SMA Strategy initialized: short={}, long={}",
            self.short_period, self.long_period
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.prices.clear();
        self.in_position = false;
    }

    fn on_fill(&mut self, signal: &Signal) {
        self.in_position = matches!(signal, Signal::Buy { .. });
    }

    fn on_tick(&mut self, tick: &TickData) -> Signal {
        self.prices.push_back(tick.price);

        // Keep reasonable history length
        if self.prices.len() > self.long_period * 2 {
            self.prices.pop_front();
        }

        if let (Some(short_sma), Some(long_sma)) = (
            self.calculate_sma(self.short_period),
            self.calculate_sma(self.long_period),
        ) {
            // Golden cross: short MA crosses above long MA
            if short_sma > long_sma && !self.in_position {
                return Signal::Buy {
                    symbol: tick.symbol.clone(),
                    quantity: None,
                };
            }
            // Death cross: short MA crosses below long MA
            else if short_sma < long_sma && self.in_position {
                return Signal::Sell {
                    symbol: tick.symbol.clone(),
                    quantity: None,
                };
            }
        }

        Signal::Hold
    }

    fn on_ohlc(&mut self, ohlc: &OHLCData) -> Signal {
        self.prices.push_back(ohlc.close);

        if self.prices.len() > self.long_period * 2 {
            self.prices.pop_front();
        }

        if let (Some(short_sma), Some(long_sma)) = (
            self.calculate_sma(self.short_period),
            self.calculate_sma(self.long_period),
        ) {
            if short_sma > long_sma && !self.in_position {
                return Signal::Buy {
                    symbol: ohlc.symbol.clone(),
                    quantity: None,
                };
            } else if short_sma < long_sma && self.in_position {
                return Signal::Sell {
                    symbol: ohlc.symbol.clone(),
                    quantity: None,
                };
            }
        }

        Signal::Hold
    }

    fn supports_ohlc(&self) -> bool {
        false
    }
    fn preferred_timeframe(&self) -> Option<crate::data::types::Timeframe> {
        Some(crate::data::types::Timeframe::OneMinute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::types::TradeSide;
    use chrono::{Duration, Utc};

    fn tick(i: i64, price: i64) -> TickData {
        TickData::new(
            Utc::now() + Duration::seconds(i),
            "BTCUSDT".to_string(),
            Decimal::from(price),
            Decimal::ONE,
            TradeSide::Buy,
            i.to_string(),
            false,
        )
    }

    /// Flat prices to warm up both averages, then a rise so the short MA
    /// crosses above the long one. Returns the first non-Hold signal.
    fn feed_until_signal(strategy: &mut SmaStrategy) -> Signal {
        let mut i = 0;
        for _ in 0..20 {
            assert!(matches!(strategy.on_tick(&tick(i, 100)), Signal::Hold));
            i += 1;
        }
        for price in [101, 103, 106, 110, 115] {
            let signal = strategy.on_tick(&tick(i, price));
            i += 1;
            if !matches!(signal, Signal::Hold) {
                return signal;
            }
        }
        panic!("expected the golden cross to emit a signal");
    }

    #[test]
    fn golden_cross_emits_an_unsized_buy() {
        let mut strategy = SmaStrategy::new();
        assert!(matches!(
            feed_until_signal(&mut strategy),
            Signal::Buy { quantity: None, .. }
        ));
    }

    #[test]
    fn position_state_changes_only_on_fill() {
        let mut strategy = SmaStrategy::new();
        let buy = feed_until_signal(&mut strategy);

        // The order was not filled, so the strategy must keep asking to buy.
        assert!(matches!(
            strategy.on_tick(&tick(100, 120)),
            Signal::Buy { .. }
        ));

        strategy.on_fill(&buy);

        // Now it's in a position: no more buys while the trend holds.
        assert!(matches!(strategy.on_tick(&tick(101, 125)), Signal::Hold));
    }
}
