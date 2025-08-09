use crate::data::types::TickData;
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum Signal {
    Buy { symbol: String, quantity: Decimal },
    Sell { symbol: String, quantity: Decimal },
    Hold,
}

pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn on_tick(&mut self, tick: &TickData) -> Signal;
    fn initialize(&mut self, params: HashMap<String, String>) -> Result<(), String>;
    
    /// Reset strategy state for new backtest
    fn reset(&mut self) {
        // Default implementation does nothing
        // Strategies can override if needed
    }
}
