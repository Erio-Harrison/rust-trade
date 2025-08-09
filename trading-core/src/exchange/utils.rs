// =================================================================
// exchange/utils.rs - Utility Functions
// =================================================================

use crate::data::types::{TickData, TradeSide};
use super::{BinanceTradeMessage, ExchangeError};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

/// Convert Binance trade message to standard TickData format
pub fn convert_binance_to_tick_data(msg: BinanceTradeMessage) -> Result<TickData, ExchangeError> {
    // Convert timestamp from milliseconds to DateTime
    let timestamp = DateTime::from_timestamp_millis(msg.trade_time as i64)
        .ok_or_else(|| ExchangeError::ParseError("Invalid timestamp".to_string()))?;
    
    // Parse price and quantity as Decimal for precision
    let price = Decimal::from_str(&msg.price)
        .map_err(|e| ExchangeError::ParseError(format!("Invalid price '{}': {}", msg.price, e)))?;
    
    let quantity = Decimal::from_str(&msg.quantity)
        .map_err(|e| ExchangeError::ParseError(format!("Invalid quantity '{}': {}", msg.quantity, e)))?;
    
    // Validate parsed values
    if price <= Decimal::ZERO {
        return Err(ExchangeError::ParseError("Price must be positive".to_string()));
    }
    
    if quantity <= Decimal::ZERO {
        return Err(ExchangeError::ParseError("Quantity must be positive".to_string()));
    }
    
    // Determine trade side based on maker flag
    // If buyer is maker, it means a sell order was filled (seller was taker)
    // If buyer is not maker, it means a buy order was filled (buyer was taker)
    let side = if msg.is_buyer_maker {
        TradeSide::Sell
    } else {
        TradeSide::Buy
    };
    
    Ok(TickData::new(
        timestamp,
        msg.symbol,
        price,
        quantity,
        side,
        msg.trade_id.to_string(),
        msg.is_buyer_maker,
    ))
}

/// Validate symbol format for Binance
pub fn validate_binance_symbol(symbol: &str) -> Result<String, ExchangeError> {
    if symbol.is_empty() {
        return Err(ExchangeError::InvalidSymbol("Symbol cannot be empty".to_string()));
    }
    
    let symbol = symbol.to_uppercase();
    
    // Basic validation: should be alphanumeric and reasonable length
    if !symbol.chars().all(char::is_alphanumeric) {
        return Err(ExchangeError::InvalidSymbol(
            format!("Symbol '{}' contains invalid characters", symbol)
        ));
    }
    
    if symbol.len() < 3 || symbol.len() > 20 {
        return Err(ExchangeError::InvalidSymbol(
            format!("Symbol '{}' has invalid length", symbol)
        ));
    }
    
    Ok(symbol)
}

/// Build WebSocket subscription streams for Binance
pub fn build_binance_trade_streams(symbols: &[String]) -> Result<Vec<String>, ExchangeError> {
    if symbols.is_empty() {
        return Err(ExchangeError::InvalidSymbol("No symbols provided".to_string()));
    }
    
    let mut streams = Vec::with_capacity(symbols.len());
    
    for symbol in symbols {
        let validated_symbol = validate_binance_symbol(symbol)?;
        streams.push(format!("{}@trade", validated_symbol.to_lowercase()));
    }
    
    Ok(streams)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_symbol_validation() {
        assert!(validate_binance_symbol("BTCUSDT").is_ok());
        assert!(validate_binance_symbol("btcusdt").is_ok());
        assert!(validate_binance_symbol("").is_err());
        assert!(validate_binance_symbol("BTC-USDT").is_err());
    }
    
    #[test]
    fn test_stream_building() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let streams = build_binance_trade_streams(&symbols).unwrap();
        
        assert_eq!(streams.len(), 2);
        assert_eq!(streams[0], "btcusdt@trade");
        assert_eq!(streams[1], "ethusdt@trade");
    }
}