// =================================================================
// exchange/traits.rs - Exchange Interface Definition
// =================================================================

use async_trait::async_trait;
use crate::data::types::TickData;
use super::{HistoricalTradeParams, ExchangeError};

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
    
    /// Fetch historical trade data for a specific symbol and time range
    async fn get_historical_trades(
        &self, 
        params: HistoricalTradeParams
    ) -> Result<Vec<TickData>, ExchangeError>;
}