// exchange/binance.rs

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, error, info, warn};

use super::{
    errors::ExchangeError,
    traits::Exchange,
    types::{BinanceStreamMessage, BinanceSubscribeMessage, BinanceTradeMessage},
    utils::{build_binance_trade_streams, convert_binance_to_tick_data},
};
use trading_common::data::types::TickData;

// Constants
const BINANCE_WS_URL: &str = "wss://stream.binance.com:9443/stream";
const RECONNECT_DELAY: Duration = Duration::from_secs(5);

/// Binance exchange implementation
pub struct BinanceExchange {
    ws_url: String,
}

impl BinanceExchange {
    /// Create a new Binance exchange instance
    pub fn new() -> Self {
        Self {
            ws_url: BINANCE_WS_URL.to_string(),
        }
    }

    /// Parse WebSocket message and extract trade data
    fn parse_trade_message(&self, text: &str) -> Result<TickData, ExchangeError> {
        // First try to parse as stream message (combined streams format)
        if let Ok(stream_msg) = serde_json::from_str::<BinanceStreamMessage>(text) {
            return convert_binance_to_tick_data(stream_msg.data);
        }

        // Fallback: try to parse as direct trade message
        if let Ok(trade_msg) = serde_json::from_str::<BinanceTradeMessage>(text) {
            return convert_binance_to_tick_data(trade_msg);
        }

        // Check if it's a subscription confirmation or other control message
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
            if value.get("result").is_some() || value.get("id").is_some() {
                // This is a subscription confirmation, not an error
                debug!("Received subscription confirmation: {}", text);
                return Err(ExchangeError::ParseError(
                    "Control message, not trade data".to_string(),
                ));
            }
        }

        Err(ExchangeError::ParseError(format!(
            "Unable to parse message: {}",
            text
        )))
    }

    /// Handle WebSocket connection with reconnection logic
    async fn handle_websocket_connection(
        &self,
        symbols: &[String],
        callback: Box<dyn Fn(TickData) + Send + Sync>,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ExchangeError> {
        let streams = build_binance_trade_streams(symbols)?;
        info!(
            "Connecting to Binance WebSocket with {} streams",
            streams.len()
        );

        let mut reconnect_attempts = 0;
        const MAX_RECONNECT_ATTEMPTS: u32 = 10;

        loop {
            // Check for shutdown signal before each connection attempt
            if shutdown_rx.try_recv().is_ok() {
                info!("Shutdown signal received, stopping WebSocket connection attempts");
                return Ok(());
            }

            match self
                .connect_and_subscribe(&streams, &callback, shutdown_rx.resubscribe())
                .await
            {
                Ok(()) => {
                    info!(
                        "WebSocket connection ended normally - checking if shutdown was requested"
                    );

                    // If connection ended normally, it's likely due to shutdown signal
                    // Exit the reconnection loop
                    return Ok(());
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    error!(
                        "WebSocket connection failed (attempt {}): {}",
                        reconnect_attempts, e
                    );

                    if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                        return Err(ExchangeError::NetworkError(format!(
                            "Max reconnection attempts ({}) exceeded",
                            MAX_RECONNECT_ATTEMPTS
                        )));
                    }

                    warn!("Attempting to reconnect in {:?}...", RECONNECT_DELAY);

                    // Wait for reconnect delay or shutdown signal
                    tokio::select! {
                        _ = sleep(RECONNECT_DELAY) => {
                            // Continue to retry
                            continue;
                        }
                        _ = shutdown_rx.recv() => {
                            info!("Shutdown signal received during reconnect delay");
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Connect to WebSocket and handle subscription
    async fn connect_and_subscribe(
        &self,
        streams: &[String],
        callback: &Box<dyn Fn(TickData) + Send + Sync>,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ExchangeError> {
        // Establish WebSocket connection
        let (ws_stream, _) = connect_async(&self.ws_url)
            .await
            .map_err(|e| ExchangeError::WebSocketError(format!("Failed to connect: {}", e)))?;

        debug!("WebSocket connected to {}", self.ws_url);

        let (mut write, mut read) = ws_stream.split();

        // Send subscription message
        let subscribe_msg = BinanceSubscribeMessage::new(streams.to_vec());
        let subscribe_json = serde_json::to_string(&subscribe_msg).map_err(|e| {
            ExchangeError::ParseError(format!("Failed to serialize subscription: {}", e))
        })?;

        write
            .send(Message::Text(subscribe_json))
            .await
            .map_err(|e| {
                ExchangeError::WebSocketError(format!("Failed to send subscription: {}", e))
            })?;

        info!("Subscription sent for {} streams", streams.len());

        // Message processing loop
        loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            match self.parse_trade_message(&text) {
                                Ok(tick_data) => callback(tick_data),
                                Err(e) => warn!("Parse error: {}", e),
                            }
                        }
                        Some(Ok(Message::Ping(ping))) => {
                            write.send(Message::Pong(ping)).await?;
                        }
                        Some(Ok(Message::Close(_))) => {
                            info!("WebSocket closed by server");
                            break;
                        }
                        Some(Err(e)) => {
                            return Err(ExchangeError::WebSocketError(e.to_string()));
                        }
                        None => {
                            info!("WebSocket stream ended");
                            break;
                        }
                        _ => continue,
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, closing WebSocket gracefully");
                    // Send Close frame to server
                    if let Err(e) = write.send(Message::Close(None)).await {
                        warn!("Failed to send close frame: {}", e);
                    }
                    break;
                }
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Exchange for BinanceExchange {
    async fn subscribe_trades(
        &self,
        symbols: &[String],
        callback: Box<dyn Fn(TickData) + Send + Sync>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ExchangeError> {
        if symbols.is_empty() {
            return Err(ExchangeError::InvalidSymbol(
                "No symbols provided".to_string(),
            ));
        }

        info!(
            "Starting Binance trade subscription for symbols: {:?}",
            symbols
        );

        // This will run indefinitely with reconnection logic
        self.handle_websocket_connection(symbols, callback, shutdown_rx.resubscribe())
            .await
    }
}

impl Default for BinanceExchange {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trading_common::data::types::TradeSide;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn test_parse_trade_message() {
        let exchange = BinanceExchange::new();

        // Test combined stream message format
        let stream_msg = r#"{
            "stream": "btcusdt@trade",
            "data": {
                "e": "trade",
                "E": 1672515782136,
                "s": "BTCUSDT",
                "t": 12345,
                "p": "50000.00",
                "q": "0.001",
                "b": 88,
                "a": 50,
                "T": 1672515782136,
                "m": false,
                "M": true
            }
        }"#;

        let tick_data = exchange.parse_trade_message(stream_msg).unwrap();

        assert_eq!(tick_data.symbol, "BTCUSDT");
        assert_eq!(tick_data.price, Decimal::from_str("50000.00").unwrap());
        assert_eq!(tick_data.quantity, Decimal::from_str("0.001").unwrap());
        assert_eq!(tick_data.side, TradeSide::Buy); // is_buyer_maker = false -> Buy
        assert_eq!(tick_data.trade_id, "12345");
        assert!(!tick_data.is_buyer_maker);
    }

    #[test]
    fn test_parse_direct_trade_message() {
        let exchange = BinanceExchange::new();

        // Test direct trade message format
        let trade_msg = r#"{
            "e": "trade",
            "E": 1672515782136,
            "s": "ETHUSDT",
            "t": 67890,
            "p": "3000.50",
            "q": "0.1",
            "b": 88,
            "a": 50,
            "T": 1672515782136,
            "m": true,
            "M": true
        }"#;

        let tick_data = exchange.parse_trade_message(trade_msg).unwrap();

        assert_eq!(tick_data.symbol, "ETHUSDT");
        assert_eq!(tick_data.price, Decimal::from_str("3000.50").unwrap());
        assert_eq!(tick_data.side, TradeSide::Sell); // is_buyer_maker = true -> Sell
        assert!(tick_data.is_buyer_maker);
    }

    #[test]
    fn test_parse_subscription_confirmation() {
        let exchange = BinanceExchange::new();

        let confirmation_msg = r#"{
            "result": null,
            "id": 1
        }"#;

        let result = exchange.parse_trade_message(confirmation_msg);
        assert!(result.is_err());

        // Should be a parse error indicating it's a control message
        if let Err(ExchangeError::ParseError(msg)) = result {
            assert!(msg.contains("Control message"));
        } else {
            panic!("Expected ParseError with control message indication");
        }
    }
}
