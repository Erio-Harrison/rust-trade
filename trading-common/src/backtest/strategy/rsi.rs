use super::base::{Signal, Strategy};
use crate::data::types::{OHLCData, TickData, Timeframe};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// Wilder's smoothed RSI (the TA-Lib / TradingView definition): the averages
/// are seeded with the plain mean of the first `period` changes, then smoothed
/// with `avg = (prev * (n - 1) + current) / n`.
#[derive(Debug, Clone)]
struct WilderRsi {
    period: usize,
    prev: Option<Decimal>,
    seen: usize,
    avg_gain: Decimal,
    avg_loss: Decimal,
}

impl WilderRsi {
    fn new(period: usize) -> Self {
        Self {
            period,
            prev: None,
            seen: 0,
            avg_gain: Decimal::ZERO,
            avg_loss: Decimal::ZERO,
        }
    }

    fn update(&mut self, price: Decimal) -> Option<Decimal> {
        let prev = self.prev.replace(price)?;
        let change = price - prev;
        let (gain, loss) = (change.max(Decimal::ZERO), (-change).max(Decimal::ZERO));
        let n = Decimal::from(self.period);
        if self.seen < self.period {
            self.avg_gain += gain;
            self.avg_loss += loss;
            self.seen += 1;
            if self.seen < self.period {
                return None;
            }
            self.avg_gain /= n;
            self.avg_loss /= n;
        } else {
            self.avg_gain = (self.avg_gain * (n - Decimal::ONE) + gain) / n;
            self.avg_loss = (self.avg_loss * (n - Decimal::ONE) + loss) / n;
        }
        if self.avg_loss.is_zero() {
            return Some(Decimal::ONE_HUNDRED);
        }
        Some(
            Decimal::ONE_HUNDRED
                - Decimal::ONE_HUNDRED / (Decimal::ONE + self.avg_gain / self.avg_loss),
        )
    }
}

pub struct RsiStrategy {
    period: usize,
    oversold: Decimal,
    overbought: Decimal,
    rsi: WilderRsi,
    in_position: bool,
}

impl RsiStrategy {
    pub fn new() -> Self {
        Self {
            period: 14,
            oversold: Decimal::from(30),
            overbought: Decimal::from(70),
            rsi: WilderRsi::new(14),
            in_position: false,
        }
    }

    /// Feeds one close into Wilder's RSI (the TA-Lib / TradingView definition).
    /// Returns `None` until `period` price changes have been seen.
    fn update_rsi(&mut self, price: Decimal) -> Option<Decimal> {
        self.rsi.update(price)
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
        if self.period == 0 {
            return Err("Period must be greater than 0".to_string());
        }
        self.rsi = WilderRsi::new(self.period);

        println!(
            "RSI Strategy initialized: period={}, oversold={}, overbought={}",
            self.period, self.oversold, self.overbought
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.rsi = WilderRsi::new(self.period);
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

    #[test]
    fn matches_stockcharts_reference_series() {
        // The 14-period example from StockCharts (also used by TA-Lib's tests).
        let closes = [
            "44.34", "44.09", "44.15", "43.61", "44.33", "44.83", "45.10", "45.42", "45.84",
            "46.08", "45.89", "46.03", "45.61", "46.28", "46.28", "46.00", "46.03", "46.41",
            "46.22", "45.64",
        ];
        let mut strategy = strategy("14");
        let values: Vec<Decimal> = closes
            .iter()
            .filter_map(|c| strategy.update_rsi(c.parse().unwrap()))
            .map(|rsi| rsi.round_dp(2))
            .collect();
        let expected: Vec<Decimal> = ["70.46", "66.25", "66.48", "69.35", "66.29", "57.92"]
            .iter()
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(values, expected);
    }
}
