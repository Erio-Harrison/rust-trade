use super::base::{Signal, Strategy};
use crate::data::types::{OHLCData, TickData, Timeframe};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};

pub struct RsiStrategy {
    period: usize,
    oversold: Decimal,
    overbought: Decimal,
    prices: VecDeque<Decimal>,
    gains: VecDeque<Decimal>,
    losses: VecDeque<Decimal>,
    in_position: bool,
}

impl RsiStrategy {
    pub fn new() -> Self {
        Self {
            period: 14,
            oversold: Decimal::from(30),
            overbought: Decimal::from(70),
            prices: VecDeque::new(),
            gains: VecDeque::new(),
            losses: VecDeque::new(),
            in_position: false,
        }
    }

    fn calculate_rsi(&self) -> Option<Decimal> {
        if self.gains.len() < self.period || self.losses.len() < self.period {
            return None;
        }

        let avg_gain: Decimal = self.gains.iter().sum::<Decimal>() / Decimal::from(self.period);
        let avg_loss: Decimal = self.losses.iter().sum::<Decimal>() / Decimal::from(self.period);

        if avg_loss == Decimal::ZERO {
            return Some(Decimal::from(100));
        }

        let rs = avg_gain / avg_loss;
        let rsi = Decimal::from(100) - (Decimal::from(100) / (Decimal::from(1) + rs));

        Some(rsi)
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

        println!(
            "RSI Strategy initialized: period={}, oversold={}, overbought={}",
            self.period, self.oversold, self.overbought
        );
        Ok(())
    }

    fn reset(&mut self) {
        self.prices.clear();
        self.gains.clear();
        self.losses.clear();
        self.in_position = false;
    }

    fn on_fill(&mut self, signal: &Signal) {
        self.in_position = matches!(signal, Signal::Buy { .. });
    }

    fn on_tick(&mut self, tick: &TickData) -> Signal {
        if let Some(last_price) = self.prices.back() {
            let change = tick.price - last_price;

            if change > Decimal::ZERO {
                self.gains.push_back(change);
                self.losses.push_back(Decimal::ZERO);
            } else {
                self.gains.push_back(Decimal::ZERO);
                self.losses.push_back(-change);
            }

            // Keep fixed length
            if self.gains.len() > self.period {
                self.gains.pop_front();
                self.losses.pop_front();
            }
        }

        self.prices.push_back(tick.price);
        if self.prices.len() > self.period + 1 {
            self.prices.pop_front();
        }

        if let Some(rsi) = self.calculate_rsi() {
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
        if let Some(last_price) = self.prices.back() {
            let change = ohlc.close - last_price;

            if change > Decimal::ZERO {
                self.gains.push_back(change);
                self.losses.push_back(Decimal::ZERO);
            } else {
                self.gains.push_back(Decimal::ZERO);
                self.losses.push_back(-change);
            }

            if self.gains.len() > self.period {
                self.gains.pop_front();
                self.losses.pop_front();
            }
        }

        self.prices.push_back(ohlc.close);
        if self.prices.len() > self.period + 1 {
            self.prices.pop_front();
        }

        if let Some(rsi) = self.calculate_rsi() {
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
