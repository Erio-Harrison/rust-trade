// =================================================================
// exchange/types.rs - Data Structures
// =================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Parameters for querying historical trade data
#[derive(Debug, Clone)]
pub struct HistoricalTradeParams {
    pub symbol: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

impl HistoricalTradeParams {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol: symbol.to_uppercase(),
            start_time: None,
            end_time: None,
            limit: None,
        }
    }

    pub fn with_time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Binance specific trade message format
#[derive(Debug, Deserialize, Clone)]
pub struct BinanceTradeMessage {
    /// Symbol
    #[serde(rename = "s")]
    pub symbol: String,

    /// Trade ID
    #[serde(rename = "t")]
    pub trade_id: u64,

    /// Price
    #[serde(rename = "p")]
    pub price: String,

    /// Quantity
    #[serde(rename = "q")]
    pub quantity: String,

    /// Trade time
    #[serde(rename = "T")]
    pub trade_time: u64,

    /// Is the buyer the market maker?
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

/// Binance WebSocket stream wrapper for combined streams
#[derive(Debug, Deserialize)]
pub struct BinanceStreamMessage {
    /// Stream name (e.g., "btcusdt@trade")
    pub stream: String,

    /// The actual trade data
    pub data: BinanceTradeMessage,
}

/// Binance subscription message format
#[derive(Debug, Serialize)]
pub struct BinanceSubscribeMessage {
    pub method: String,
    pub params: Vec<String>,
    pub id: u32,
}

impl BinanceSubscribeMessage {
    pub fn new(streams: Vec<String>) -> Self {
        Self {
            method: "SUBSCRIBE".to_string(),
            params: streams,
            id: 1,
        }
    }
}
