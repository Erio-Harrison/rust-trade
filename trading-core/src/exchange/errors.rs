// =================================================================
// exchange/errors.rs - Error Types
// =================================================================

use thiserror::Error;

/// Error types for exchange operations
#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("Data parsing error: {0}")]
    ParseError(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Exchange API error: {0}")]
    ApiError(String),
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

impl From<reqwest::Error> for ExchangeError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ExchangeError::Timeout
        } else if err.is_connect() {
            ExchangeError::NetworkError(err.to_string())
        } else {
            ExchangeError::ApiError(err.to_string())
        }
    }
}
