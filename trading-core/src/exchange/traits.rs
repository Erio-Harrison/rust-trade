// exchange/traits.rs

use super::ExchangeError;
use trading_common::data::types::TickData;
use async_trait::async_trait;

/// Main exchange interface that all exchange implementations must follow
#[async_trait]
pub trait Exchange: Send + Sync {
    /// Subscribe to real-time trade data streams
    async fn subscribe_trades(
        &self,
        symbols: &[String],
        callback: Box<dyn Fn(TickData) + Send + Sync>,
        shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ExchangeError>;
}
