use super::base::{Signal, Strategy};
use crate::data::types::{OHLCData, TickData, Timeframe};
use fin_primitives::signals::indicators::Rsi;
use fin_primitives::signals::{BarInput, Signal as _};
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct RsiStrategy {
    period: usize,
    oversold: Decimal,
    overbought: Decimal,
    rsi: Rsi,
    in_position: bool,
}

impl RsiStrategy {
    pub fn new() -> Self {
        Self {
            period: 14,
            oversold: Decimal::from(30),
            overbought: Decimal::from(70),
            rsi: Rsi::new("rsi", 14).expect("default RSI period is non-zero"),
            in_position: false,
        }
    }

    /// Feeds one close into Wilder's RSI (the TA-Lib / TradingView definition).
    /// Returns `None` until `period` price changes have been seen.
    fn update_rsi(&mut self, price: Decimal) -> Option<Decimal> {
        self.rsi
            .update(&BarInput::from_close(price))
            .ok()
            .and_then(|value| value.as_decimal())
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str {
        "RSI Strategy"
    }

    fn initialize(&mut self, params: HashMap<String, String>) -> Result<(), String> {
        if let Some(period) = params.get("period") {
            self.period = period.parse().map_err(|_| "Invalid period")?;
        }
        if let Some(oversold) = params.get("oversold") {
            self.oversold = oversold.parse().map_err(|_| "Invalid oversold")?;
        }
        if let Some(overbought) = params.get("overbought") {
            self.overbought = overbought.parse().map_err(|_| "Invalid overbought")?;
        }

        if self.oversold >= self.overbought {
            return Err("Oversold level must be less than overbought level".to_string());
        }
        self.rsi = Rsi::new("rsi", self.period).map_err(|_| "Period must be greater than 0")?;

        println!(
            "RSI Strategy initialized: period={}, oversold={}, overbought={}",
            self.period, self.oversold, self.overbought
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.rsi.reset();
        self.in_position = false;
    }

    fn on_fill(&mut self, signal: &Signal) {
        self.in_position = matches!(signal, Signal::Buy { .. });
    }

    fn on_tick(&mut self, tick: &TickData) -> Signal {
        if let Some(rsi) = self.update_rsi(tick.price) {
            // Buy signal when RSI is oversold and we don't have a buy position
            if rsi < self.oversold && !self.in_position {
                return Signal::Buy {
                    symbol: tick.symbol.clone(),
                    quantity: None,
                };
            }
            // Sell signal when RSI is overbought and we have a buy position
            else if rsi > self.overbought && self.in_position {
                return Signal::Sell {
                    symbol: tick.symbol.clone(),
                    quantity: None,
                };
            }
        }

        Signal::Hold
    }

    fn on_ohlc(&mut self, ohlc: &OHLCData) -> Signal {
        if let Some(rsi) = self.update_rsi(ohlc.close) {
            if rsi < self.oversold && !self.in_position {
                return Signal::Buy {
                    symbol: ohlc.symbol.clone(),
                    quantity: None,
                };
            } else if rsi > self.overbought && self.in_position {
                return Signal::Sell {
                    symbol: ohlc.symbol.clone(),
                    quantity: None,
                };
            }
        }

        Signal::Hold
    }

    fn supports_ohlc(&self) -> bool {
        true
    }
    fn preferred_timeframe(&self) -> Option<Timeframe> {
        Some(Timeframe::OneDay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strategy(period: &str) -> RsiStrategy {
        let mut strategy = RsiStrategy::new();
        let params = HashMap::from([("period".to_string(), period.to_string())]);
        strategy.initialize(params).unwrap();
        strategy
    }

    #[test]
    fn rsi_uses_wilder_smoothing() {
        let mut strategy = strategy("2");
        assert_eq!(strategy.update_rsi(Decimal::from(1)), None);
        assert_eq!(strategy.update_rsi(Decimal::from(2)), None);
        // Seed: avg gain 1, avg loss 0.
        assert_eq!(
            strategy.update_rsi(Decimal::from(3)),
            Some(Decimal::from(100))
        );
        // Wilder: avg gain (1 + 0) / 2 = 0.5, avg loss (0 + 1) / 2 = 0.5.
        assert_eq!(
            strategy.update_rsi(Decimal::from(2)),
            Some(Decimal::from(50))
        );
        // Wilder: avg gain 0.25, avg loss 0.75 -> RSI 25. A plain 2-bar
        // average of the last two changes (both losses) would give 0.
        let rsi = strategy.update_rsi(Decimal::from(1)).unwrap();
        assert_eq!(rsi.round_dp(12), Decimal::from(25));
    }

    #[test]
    fn reset_restarts_warmup() {
        let mut strategy = strategy("2");
        for price in [1, 2, 3] {
            strategy.update_rsi(Decimal::from(price));
        }
        strategy.reset();
        assert_eq!(strategy.update_rsi(Decimal::from(3)), None);
    }

    #[test]
    fn zero_period_is_rejected() {
        let mut strategy = RsiStrategy::new();
        let params = HashMap::from([("period".to_string(), "0".to_string())]);
        assert!(strategy.initialize(params).is_err());
    }
}
