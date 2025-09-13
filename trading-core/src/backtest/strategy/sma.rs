use super::base::{Strategy, Signal};
use crate::data::types::{OHLCData, TickData};
use rust_decimal::Decimal;
use std::collections::{HashMap, VecDeque};

pub struct SmaStrategy {
    short_period: usize,
    long_period: usize,
    prices: VecDeque<Decimal>,
    last_signal: Option<Signal>,
}

impl SmaStrategy {
    pub fn new() -> Self {
        Self {
            short_period: 5,
            long_period: 20,
            prices: VecDeque::new(),
            last_signal: None,
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
        
        println!("SMA Strategy initialized: short={}, long={}", self.short_period, self.long_period);
        Ok(())
    }
    
    fn reset(&mut self) {
        self.prices.clear();
        self.last_signal = None;
    }
    
    fn on_tick(&mut self, tick: &TickData) -> Signal {
        self.prices.push_back(tick.price);
        
        // Keep reasonable history length
        if self.prices.len() > self.long_period * 2 {
            self.prices.pop_front();
        }
        
        if let (Some(short_sma), Some(long_sma)) = 
            (self.calculate_sma(self.short_period), self.calculate_sma(self.long_period)) {
            
            // Golden cross: short MA crosses above long MA
            if short_sma > long_sma && 
               !matches!(self.last_signal, Some(Signal::Buy { .. })) {
                let signal = Signal::Buy {
                    symbol: tick.symbol.clone(),
                    quantity: Decimal::from(100),
                };
                self.last_signal = Some(signal.clone());
                return signal;
            }
            // Death cross: short MA crosses below long MA
            else if short_sma < long_sma && 
                    matches!(self.last_signal, Some(Signal::Buy { .. })) {
                let signal = Signal::Sell {
                    symbol: tick.symbol.clone(),
                    quantity: Decimal::from(100),
                };
                self.last_signal = Some(signal.clone());
                return signal;
            }
        }
        
        Signal::Hold
    }

    fn on_ohlc(&mut self, ohlc: &OHLCData) -> Signal {
        self.prices.push_back(ohlc.close);
        
        if self.prices.len() > self.long_period * 2 {
            self.prices.pop_front();
        }
        
        if let (Some(short_sma), Some(long_sma)) = 
            (self.calculate_sma(self.short_period), self.calculate_sma(self.long_period)) {
            
            if short_sma > long_sma && !matches!(self.last_signal, Some(Signal::Buy { .. })) {
                let signal = Signal::Buy {
                    symbol: ohlc.symbol.clone(),
                    quantity: Decimal::from(100),
                };
                self.last_signal = Some(signal.clone());
                return signal;
            } else if short_sma < long_sma && matches!(self.last_signal, Some(Signal::Buy { .. })) {
                let signal = Signal::Sell {
                    symbol: ohlc.symbol.clone(),
                    quantity: Decimal::from(100),
                };
                self.last_signal = Some(signal.clone());
                return signal;
            }
        }
        
        Signal::Hold
    }
    
    fn supports_ohlc(&self) -> bool { false }
    fn preferred_timeframe(&self) -> Option<crate::data::types::Timeframe> { 
        Some(crate::data::types::Timeframe::OneMinute) 
    }    
}