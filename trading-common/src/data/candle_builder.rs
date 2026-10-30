use chrono::{DateTime, Utc};

use super::types::{OHLCData, TickData, Timeframe};

/// Builds OHLC candles incrementally from a live tick stream, entirely in memory.
///
/// This is the live-path counterpart of
/// `TickDataRepository::generate_ohlc_from_ticks`, which aggregates history
/// inside Postgres. Both align windows with `Timeframe::align_timestamp`, so a
/// strategy sees the same candles live as it does in a backtest over the same
/// ticks. One builder tracks one symbol.
#[derive(Debug)]
pub struct CandleBuilder {
    timeframe: Timeframe,
    current: Option<OHLCData>,
}

impl CandleBuilder {
    pub fn new(timeframe: Timeframe) -> Self {
        Self {
            timeframe,
            current: None,
        }
    }

    pub fn timeframe(&self) -> Timeframe {
        self.timeframe
    }

    /// The candle currently being built, if any. It is not complete yet.
    pub fn current(&self) -> Option<&OHLCData> {
        self.current.as_ref()
    }

    /// Folds a tick into the builder.
    ///
    /// Returns the previous candle when this tick opens a new window: that
    /// candle is complete and will never change again. Ticks that fall before
    /// the current window are ignored — their candle was already emitted and
    /// can't be amended.
    pub fn push(&mut self, tick: &TickData) -> Option<OHLCData> {
        let window_start = self.timeframe.align_timestamp(tick.timestamp);

        if let Some(candle) = self.current.as_mut() {
            if candle.timestamp == window_start {
                candle.high = candle.high.max(tick.price);
                candle.low = candle.low.min(tick.price);
                candle.close = tick.price;
                candle.volume += tick.quantity;
                candle.trade_count += 1;
                return None;
            }
            if window_start < candle.timestamp {
                return None;
            }
        }

        self.current
            .replace(Self::open_candle(self.timeframe, window_start, tick))
    }

    fn open_candle(timeframe: Timeframe, window_start: DateTime<Utc>, tick: &TickData) -> OHLCData {
        OHLCData::new(
            window_start,
            tick.symbol.clone(),
            timeframe,
            tick.price,
            tick.price,
            tick.price,
            tick.price,
            tick.quantity,
            1,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::types::TradeSide;
    use chrono::TimeZone;
    use rust_decimal::Decimal;

    fn tick(h: u32, m: u32, s: u32, price: i64, qty: i64) -> TickData {
        TickData::new(
            Utc.with_ymd_and_hms(2024, 1, 1, h, m, s).unwrap(),
            "BTCUSDT".to_string(),
            Decimal::from(price),
            Decimal::from(qty),
            TradeSide::Buy,
            format!("{h}{m}{s}"),
            false,
        )
    }

    #[test]
    fn ticks_in_one_window_accumulate_without_emitting() {
        let mut builder = CandleBuilder::new(Timeframe::OneMinute);

        assert!(builder.push(&tick(10, 0, 5, 100, 1)).is_none());
        assert!(builder.push(&tick(10, 0, 20, 105, 2)).is_none());
        assert!(builder.push(&tick(10, 0, 50, 95, 3)).is_none());

        let candle = builder.current().expect("a candle is being built");
        assert_eq!(
            candle.timestamp,
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
        );
        assert_eq!(candle.open, Decimal::from(100));
        assert_eq!(candle.high, Decimal::from(105));
        assert_eq!(candle.low, Decimal::from(95));
        assert_eq!(candle.close, Decimal::from(95));
        assert_eq!(candle.volume, Decimal::from(6));
        assert_eq!(candle.trade_count, 3);
    }

    #[test]
    fn first_tick_of_next_window_emits_the_completed_candle() {
        let mut builder = CandleBuilder::new(Timeframe::OneMinute);
        builder.push(&tick(10, 0, 10, 100, 1));
        builder.push(&tick(10, 0, 40, 110, 1));

        let completed = builder
            .push(&tick(10, 1, 5, 120, 1))
            .expect("crossing the minute boundary completes the candle");

        assert_eq!(
            completed.timestamp,
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap()
        );
        assert_eq!(completed.open, Decimal::from(100));
        assert_eq!(completed.close, Decimal::from(110));
        assert_eq!(completed.trade_count, 2);

        let current = builder.current().expect("new candle opened");
        assert_eq!(
            current.timestamp,
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 1, 0).unwrap()
        );
        assert_eq!(current.open, Decimal::from(120));
        assert_eq!(current.trade_count, 1);
    }

    #[test]
    fn late_tick_for_an_already_emitted_window_is_ignored() {
        let mut builder = CandleBuilder::new(Timeframe::OneMinute);
        builder.push(&tick(10, 0, 10, 100, 1));
        builder.push(&tick(10, 1, 5, 120, 1));

        assert!(builder.push(&tick(10, 0, 59, 50, 9)).is_none());

        let current = builder.current().unwrap();
        assert_eq!(
            current.low,
            Decimal::from(120),
            "late tick must not touch the open candle"
        );
        assert_eq!(current.trade_count, 1);
    }

    #[test]
    fn windows_follow_the_timeframe_alignment() {
        let mut builder = CandleBuilder::new(Timeframe::FiveMinutes);
        builder.push(&tick(10, 7, 0, 100, 1));

        assert_eq!(
            builder.current().unwrap().timestamp,
            Utc.with_ymd_and_hms(2024, 1, 1, 10, 5, 0).unwrap()
        );
        assert!(builder.push(&tick(10, 9, 59, 101, 1)).is_none());
        assert!(builder.push(&tick(10, 10, 0, 102, 1)).is_some());
    }
}
