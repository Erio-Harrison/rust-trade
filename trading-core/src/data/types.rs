use chrono::{DateTime, Utc, Duration, Timelike, Datelike};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// =================================================================
// Core data type: completely corresponds to the tick_data table structure
// =================================================================

/// Trading direction enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TradeSide {
    Buy,
    Sell,
}


/// Standard trading data structure - corresponds one-to-one with the tick_data table fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TickData {
    /// UTC timestamp, supports millisecond precision
    pub timestamp: DateTime<Utc>,

    /// Trading pair, such as "BTCUSDT"
    pub symbol: String,

    /// Trading price
    pub price: Decimal,

    /// Trading quantity
    pub quantity: Decimal,

    /// Trading direction
    pub side: TradeSide,

    /// Original transaction ID
    pub trade_id: String,

    /// Whether the buyer is the maker
    pub is_buyer_maker: bool,
}

impl TickData {
    /// New TickData
    pub fn new(
        timestamp: DateTime<Utc>,
        symbol: String,
        price: Decimal,
        quantity: Decimal,
        side: TradeSide,
        trade_id: String,
        is_buyer_maker: bool,
    ) -> Self {
        Self {
            timestamp,
            symbol,
            price,
            quantity,
            side,
            trade_id,
            is_buyer_maker,
        }
    }
}

// =================================================================
// Helper Types
// =================================================================

/// Database statistics
#[derive(Debug, Clone)]
pub struct DbStats {
    pub symbol: Option<String>,
    pub total_records: u64,
    pub earliest_timestamp: Option<DateTime<Utc>>,
    pub latest_timestamp: Option<DateTime<Utc>>,
}

// =================================================================
// TradeSide Implementation for Database Integration
// =================================================================

impl TradeSide {
    /// Convert to database string representation
    pub fn as_db_str(&self) -> &'static str {
        match self {
            TradeSide::Buy => "BUY",
            TradeSide::Sell => "SELL",
        }
    }
}

// =================================================================
// Query parameter type
// =================================================================

/// TickData Query parameters
#[derive(Debug, Clone)]
pub struct TickQuery {
    pub symbol: String,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub trade_side: Option<TradeSide>,
}

impl TickQuery {
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            start_time: None,
            end_time: None,
            limit: None,
            trade_side: None,
        }
    }
}

// =================================================================
// Error type definition
// =================================================================

#[derive(Error, Debug)]
pub enum DataError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    #[error("Data not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Decimal conversion error: {0}")]
    DecimalConversion(#[from] rust_decimal::Error),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub type DataResult<T> = Result<T, DataError>;


/// Backtest data information for user selection
#[derive(Debug, Clone)]
pub struct BacktestDataInfo {
    pub total_records: u64,
    pub symbols_count: u64,
    pub earliest_time: Option<DateTime<Utc>>,
    pub latest_time: Option<DateTime<Utc>>,
    pub symbol_info: Vec<SymbolDataInfo>,
}

/// Per-symbol data information
#[derive(Debug, Clone)]
pub struct SymbolDataInfo {
    pub symbol: String,
    pub records_count: u64,
    pub earliest_time: Option<DateTime<Utc>>,
    pub latest_time: Option<DateTime<Utc>>,
    pub min_price: Option<Decimal>,
    pub max_price: Option<Decimal>,
}

impl BacktestDataInfo {
    /// Get information for a specific symbol
    pub fn get_symbol_info(&self, symbol: &str) -> Option<&SymbolDataInfo> {
        self.symbol_info.iter().find(|info| info.symbol == symbol)
    }
    
    /// Get available symbols
    pub fn get_available_symbols(&self) -> Vec<String> {
        self.symbol_info.iter().map(|info| info.symbol.clone()).collect()
    }
    
    /// Check if has sufficient data for backtesting
    pub fn has_sufficient_data(&self, symbol: &str, min_records: u64) -> bool {
        self.get_symbol_info(symbol)
            .map(|info| info.records_count >= min_records)
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveStrategyLog {
    pub timestamp: DateTime<Utc>,
    pub strategy_id: String,
    pub symbol: String,
    pub current_price: Decimal,
    pub signal_type: String, // BUY/SELL/HOLD
    pub portfolio_value: Decimal,
    pub total_pnl: Decimal,
    pub cache_hit: bool,
    pub processing_time_us: u64,
}

/// Time frame for OHLC data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Timeframe {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    ThirtyMinutes,
    OneHour,
    FourHours,
    OneDay,
    OneWeek,
}

impl Timeframe {
    pub fn as_duration(&self) -> Duration {
        match self {
            Timeframe::OneMinute => Duration::minutes(1),
            Timeframe::FiveMinutes => Duration::minutes(5),
            Timeframe::FifteenMinutes => Duration::minutes(15),
            Timeframe::ThirtyMinutes => Duration::minutes(30),
            Timeframe::OneHour => Duration::hours(1),
            Timeframe::FourHours => Duration::hours(4),
            Timeframe::OneDay => Duration::days(1),
            Timeframe::OneWeek => Duration::weeks(1),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Timeframe::OneMinute => "1m",
            Timeframe::FiveMinutes => "5m",
            Timeframe::FifteenMinutes => "15m",
            Timeframe::ThirtyMinutes => "30m",
            Timeframe::OneHour => "1h",
            Timeframe::FourHours => "4h",
            Timeframe::OneDay => "1d",
            Timeframe::OneWeek => "1w",
        }
    }

    /// Get the start of the time window for a given timestamp
    pub fn align_timestamp(&self, timestamp: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Timeframe::OneMinute => timestamp.with_second(0).unwrap().with_nanosecond(0).unwrap(),
            Timeframe::FiveMinutes => {
                let aligned_minute = (timestamp.minute() / 5) * 5;
                timestamp.with_minute(aligned_minute).unwrap()
                    .with_second(0).unwrap().with_nanosecond(0).unwrap()
            },
            Timeframe::FifteenMinutes => {
                let aligned_minute = (timestamp.minute() / 15) * 15;
                timestamp.with_minute(aligned_minute).unwrap()
                    .with_second(0).unwrap().with_nanosecond(0).unwrap()
            },
            Timeframe::ThirtyMinutes => {
                let aligned_minute = (timestamp.minute() / 30) * 30;
                timestamp.with_minute(aligned_minute).unwrap()
                    .with_second(0).unwrap().with_nanosecond(0).unwrap()
            },
            Timeframe::OneHour => timestamp.with_minute(0).unwrap()
                .with_second(0).unwrap().with_nanosecond(0).unwrap(),
            Timeframe::FourHours => {
                let aligned_hour = (timestamp.hour() / 4) * 4;
                timestamp.with_hour(aligned_hour).unwrap().with_minute(0).unwrap()
                    .with_second(0).unwrap().with_nanosecond(0).unwrap()
            },
            Timeframe::OneDay => timestamp.with_hour(0).unwrap().with_minute(0).unwrap()
                .with_second(0).unwrap().with_nanosecond(0).unwrap(),
            Timeframe::OneWeek => {
                let days_from_monday = timestamp.weekday().num_days_from_monday();
                let week_start = timestamp - Duration::days(days_from_monday as i64);
                week_start.with_hour(0).unwrap().with_minute(0).unwrap()
                    .with_second(0).unwrap().with_nanosecond(0).unwrap()
            },
        }
    }
}

/// OHLC data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OHLCData {
    pub timestamp: DateTime<Utc>,
    pub symbol: String,
    pub timeframe: Timeframe,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub trade_count: u64,
}

impl OHLCData {
    pub fn new(
        timestamp: DateTime<Utc>,
        symbol: String,
        timeframe: Timeframe,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
        trade_count: u64,
    ) -> Self {
        Self {
            timestamp,
            symbol,
            timeframe,
            open,
            high,
            low,
            close,
            volume,
            trade_count,
        }
    }

    /// Create OHLC from a collection of tick data
    pub fn from_ticks(
        ticks: &[TickData],
        timeframe: Timeframe,
        window_start: DateTime<Utc>,
    ) -> Option<Self> {
        if ticks.is_empty() {
            return None;
        }

        let symbol = ticks[0].symbol.clone();
        let mut open = ticks[0].price;
        let mut high = ticks[0].price;
        let mut low = ticks[0].price;
        let mut close = ticks[ticks.len() - 1].price;
        let mut volume = Decimal::ZERO;

        for tick in ticks {
            if tick.price > high {
                high = tick.price;
            }
            if tick.price < low {
                low = tick.price;
            }
            volume += tick.quantity;
        }

        Some(OHLCData::new(
            window_start,
            symbol,
            timeframe,
            open,
            high,
            low,
            close,
            volume,
            ticks.len() as u64,
        ))
    }
}