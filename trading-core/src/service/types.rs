use chrono::{DateTime, Utc};
use crate::data::types::TickData;

/// Batch processing configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum number of ticks in a batch
    pub max_batch_size: usize,
    /// Maximum time to wait before flushing batch (in seconds)
    pub max_batch_time: u64,
    /// Maximum retry attempts for failed batches
    pub max_retry_attempts: u32,
    /// Delay between retry attempts (in milliseconds)
    pub retry_delay_ms: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_time: 1,
            max_retry_attempts: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Batch processing statistics
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total ticks processed
    pub total_ticks_processed: u64,
    /// Total batches flushed
    pub total_batches_flushed: u64,
    /// Total retry attempts
    pub total_retry_attempts: u64,
    /// Failed batches (after all retries)
    pub total_failed_batches: u64,
    /// Cache update failures
    pub cache_update_failures: u64,
    /// Last flush time
    pub last_flush_time: Option<DateTime<Utc>>,
}

/// Data processing metrics
#[derive(Debug, Clone)]
pub struct ProcessingMetrics {
    /// Ticks per second
    pub ticks_per_second: f64,
    /// Current batch size
    pub current_batch_size: usize,
    /// Time since last batch flush
    pub time_since_last_flush: Option<std::time::Duration>,
    /// Overall processing statistics
    pub batch_stats: BatchStats,
}