// exchange/errors.rs

use thiserror::Error;

/// Error types for exchange operations
#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Data parsing error: {0}")]
    ParseError(String),
}

// Convert from common error types
impl From<serde_json::Error> for ExchangeError {
    fn from(err: serde_json::Error) -> Self {
        ExchangeError::ParseError(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for ExchangeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        ExchangeError::WebSocketError(err.to_string())
    }
}
