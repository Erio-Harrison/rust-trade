// exchange/types.rs

use serde::{Deserialize, Serialize};

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
    #[allow(dead_code)] // Required for JSON deserialization
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
