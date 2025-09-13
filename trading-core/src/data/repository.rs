use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration, Timelike, Datelike, Weekday};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row, QueryBuilder, Postgres};
use tracing::{debug, error, info, warn};

use crate::data::types::{LiveStrategyLog, OHLCData, Timeframe};

use super::types::{TickData, TradeSide, TickQuery, DataResult, DataError, BacktestDataInfo, DbStats, SymbolDataInfo };
use super::cache::{TieredCache, TickDataCache};

// =================================================================
// Constants and Configuration
// =================================================================

const DEFAULT_QUERY_LIMIT: u32 = 1000;
const MAX_QUERY_LIMIT: u32 = 10000;
const MAX_BATCH_SIZE: usize = 1000;


// =================================================================
// Repository Implementation
// =================================================================

/// TickData repository for database operations
pub struct TickDataRepository {
    pool: PgPool,
    cache: TieredCache,
}

impl TickDataRepository {
    /// Create new repository instance
    pub fn new(pool: PgPool, cache: TieredCache) -> Self {
        Self { pool, cache }
    }

    /// Get database pool reference
    pub fn get_pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get cache reference
    pub fn get_cache(&self) -> &TieredCache {
        &self.cache
    }

    // =================================================================
    // Insert Operations
    // =================================================================

    /// Insert single tick data
    pub async fn insert_tick(&self, tick: &TickData) -> DataResult<()> {
        self.validate_tick_data(tick)?;
        
        debug!("Inserting tick: symbol={}, price={}, trade_id={}", 
               tick.symbol, tick.price, tick.trade_id);
        
        // Insert to database first
        sqlx::query!(
            r#"
            INSERT INTO tick_data 
            (timestamp, symbol, price, quantity, side, trade_id, is_buyer_maker)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (symbol, trade_id, timestamp) DO NOTHING
            "#,
            tick.timestamp,
            tick.symbol,
            tick.price,
            tick.quantity,
            tick.side.as_db_str(),
            tick.trade_id,
            tick.is_buyer_maker
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to insert tick data: {}", e);
            DataError::Database(e)
        })?;

        // Update cache
        if let Err(e) = self.cache.push_tick(tick).await {
            warn!("Failed to update cache after insert: {}", e);
            // Don't fail the operation if cache update fails
        }

        debug!("Successfully inserted tick data");
        Ok(())
    }

    /// Batch insert tick data with optimized performance
    pub async fn batch_insert(&self, ticks: Vec<TickData>) -> DataResult<usize> {
        if ticks.is_empty() {
            return Ok(0);
        }

        // Validate all ticks
        for tick in &ticks {
            self.validate_tick_data(tick)?;
        }

        let total_count = ticks.len();
        debug!("Batch inserting {} tick records", total_count);

        // Process in chunks to avoid memory issues
        let mut total_inserted = 0;
        for chunk in ticks.chunks(MAX_BATCH_SIZE) {
            let inserted = self.batch_insert_chunk(chunk).await?;
            total_inserted += inserted;

            // Update cache for each chunk
            for tick in chunk {
                if let Err(e) = self.cache.push_tick(tick).await {
                    warn!("Failed to update cache for tick {}: {}", tick.trade_id, e);
                }
            }
        }

        info!("Successfully batch inserted {} out of {} tick records", 
              total_inserted, total_count);
        Ok(total_inserted)
    }

    /// Insert a chunk of ticks using bulk insert
    async fn batch_insert_chunk(&self, ticks: &[TickData]) -> DataResult<usize> {
        if ticks.is_empty() {
            return Ok(0);
        }

        let mut query_builder = QueryBuilder::new(
            "INSERT INTO tick_data (timestamp, symbol, price, quantity, side, trade_id, is_buyer_maker) "
        );

        query_builder.push_values(ticks, |mut b, tick| {
            b.push_bind(tick.timestamp)
                .push_bind(&tick.symbol)
                .push_bind(tick.price)
                .push_bind(tick.quantity)
                .push_bind(tick.side.as_db_str())
                .push_bind(&tick.trade_id)
                .push_bind(tick.is_buyer_maker);
        });

        // Handle duplicates by ignoring them
        query_builder.push(" ON CONFLICT (symbol, trade_id, timestamp) DO NOTHING");

        let query = query_builder.build();
        let result = query.execute(&self.pool).await?;

        Ok(result.rows_affected() as usize)
    }

    // =================================================================
    // Query Operations
    // =================================================================

    /// Get tick data based on query parameters
    pub async fn get_ticks(&self, query: &TickQuery) -> DataResult<Vec<TickData>> {
        let limit = query.limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT);
        
        debug!("Querying ticks: symbol={}, limit={}", query.symbol, limit);

        // Try cache first for recent data
        if self.is_recent_query(query) {
            let cached_ticks = self.cache.get_recent_ticks(&query.symbol, limit as usize).await?;
            if cached_ticks.len() == limit as usize {
                debug!("Cache hit: retrieved {} ticks from cache", cached_ticks.len());
                return Ok(cached_ticks);
            }
        }

        // Cache miss or not recent, query database
        let ticks = self.query_ticks_from_db(query).await?;

        // Update cache with fetched data
        for tick in &ticks {
            if let Err(e) = self.cache.push_tick(tick).await {
                warn!("Failed to cache tick {}: {}", tick.trade_id, e);
            }
        }

        debug!("Retrieved {} tick records from database", ticks.len());
        Ok(ticks)
    }

    /// Query ticks directly from database
    async fn query_ticks_from_db(&self, query: &TickQuery) -> DataResult<Vec<TickData>> {
        let limit = query.limit.unwrap_or(DEFAULT_QUERY_LIMIT).min(MAX_QUERY_LIMIT);

        let mut sql_query = QueryBuilder::new(
            "SELECT timestamp, symbol, price, quantity, side, trade_id, is_buyer_maker FROM tick_data WHERE symbol = "
        );
        sql_query.push_bind(&query.symbol);

        // Add time range filter
        if let Some(start_time) = query.start_time {
            sql_query.push(" AND timestamp >= ").push_bind(start_time);
        }
        if let Some(end_time) = query.end_time {
            sql_query.push(" AND timestamp <= ").push_bind(end_time);
        }

        // Add side filter
        if let Some(side) = query.trade_side {
            sql_query.push(" AND side = ").push_bind(side.as_db_str());
        }

        sql_query.push(" ORDER BY timestamp DESC LIMIT ").push_bind(limit as i64);

        let rows = sql_query.build().fetch_all(&self.pool).await?;

        let ticks: DataResult<Vec<TickData>> = rows
            .iter()
            .map(|row| {
                Ok(TickData {
                    timestamp: row.get("timestamp"),
                    symbol: row.get("symbol"),
                    price: row.get("price"),
                    quantity: row.get("quantity"),
                    side: self.parse_trade_side(row.get("side"))?,
                    trade_id: row.get("trade_id"),
                    is_buyer_maker: row.get("is_buyer_maker"),
                })
            })
            .collect();

        ticks
    }

    /// Get latest price for a symbol
    pub async fn get_latest_price(&self, symbol: &str) -> DataResult<Option<Decimal>> {
        debug!("Fetching latest price for symbol: {}", symbol);
        
        // Try cache first
        let cached_ticks = self.cache.get_recent_ticks(symbol, 1).await?;
        if let Some(latest_tick) = cached_ticks.first() {
            debug!("Latest price from cache: {}", latest_tick.price);
            return Ok(Some(latest_tick.price));
        }

        // Cache miss, query database
        let row = sqlx::query!(
            r#"
            SELECT price
            FROM tick_data
            WHERE symbol = $1
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            symbol
        )
        .fetch_optional(&self.pool)
        .await?;

        let price = row.map(|r| r.price);
        debug!("Latest price from database: {:?}", price);
        Ok(price)
    }

    /// Get latest prices for multiple symbols
    pub async fn get_latest_prices(&self, symbols: &[String]) -> DataResult<HashMap<String, Decimal>> {
        if symbols.is_empty() {
            return Ok(HashMap::new());
        }

        debug!("Fetching latest prices for symbols: {:?}", symbols);

        let mut prices = HashMap::new();

        // Try to get from cache first
        for symbol in symbols {
            if let Ok(cached_ticks) = self.cache.get_recent_ticks(symbol, 1).await {
                if let Some(latest_tick) = cached_ticks.first() {
                    prices.insert(symbol.clone(), latest_tick.price);
                }
            }
        }

        // Get remaining symbols from database
        let missing_symbols: Vec<String> = symbols
            .iter()
            .filter(|symbol| !prices.contains_key(*symbol))
            .map(|s| s.clone())
            .collect();

        if !missing_symbols.is_empty() {
            let rows = sqlx::query!(
                r#"
                SELECT DISTINCT ON (symbol) symbol, price
                FROM tick_data
                WHERE symbol = ANY($1)
                ORDER BY symbol, timestamp DESC
                "#,
                &missing_symbols[..]
            )
            .fetch_all(&self.pool)
            .await?;

            for row in rows {
                prices.insert(row.symbol, row.price);
            }
        }

        debug!("Retrieved latest prices for {} symbols", prices.len());
        Ok(prices)
    }

    // =================================================================
    // Backtest Specific Query Operations
    // =================================================================

    /// Get recent N ticks for backtesting (ordered by time ASC)
    pub async fn get_recent_ticks_for_backtest(
        &self,
        symbol: &str,
        count: i64,
    ) -> DataResult<Vec<TickData>> {
        debug!("Fetching {} recent ticks for backtest: {}", count, symbol);
        
        let limit = count.min(MAX_QUERY_LIMIT as i64);
        
        let rows = sqlx::query!(
            r#"
            SELECT timestamp, symbol, price, quantity, side, trade_id, is_buyer_maker
            FROM tick_data 
            WHERE symbol = $1
            ORDER BY timestamp DESC
            LIMIT $2
            "#,
            symbol,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        // Convert rows to TickData and reverse to get ASC order (oldest first)
        let ticks: DataResult<Vec<TickData>> = rows
            .iter()
            .map(|row| {
                Ok(TickData {
                    timestamp: row.timestamp,
                    symbol: row.symbol.clone(),
                    price: row.price,
                    quantity: row.quantity,
                    side: self.parse_trade_side(&row.side)?,
                    trade_id: row.trade_id.clone(),
                    is_buyer_maker: row.is_buyer_maker,
                })
            })
            .collect();

        let mut ticks = ticks?;
        ticks.reverse(); // Reverse to get chronological order (ASC)
        
        debug!("Retrieved {} ticks for backtest", ticks.len());
        Ok(ticks)
    }

    /// Get historical data for backtesting within time range (ordered by time ASC)
    pub async fn get_historical_data_for_backtest(
        &self,
        symbol: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> DataResult<Vec<TickData>> {
        debug!("Fetching historical data for backtest: {} from {} to {}", 
               symbol, start_time, end_time);
        
        let query_limit = limit.unwrap_or(MAX_QUERY_LIMIT as i64).min(MAX_QUERY_LIMIT as i64);
        
        let rows = sqlx::query!(
            r#"
            SELECT timestamp, symbol, price, quantity, side, trade_id, is_buyer_maker
            FROM tick_data 
            WHERE symbol = $1 
            AND timestamp >= $2 
            AND timestamp <= $3
            ORDER BY timestamp ASC
            LIMIT $4
            "#,
            symbol,
            start_time,
            end_time,
            query_limit
        )
        .fetch_all(&self.pool)
        .await?;

        let ticks: DataResult<Vec<TickData>> = rows
            .iter()
            .map(|row| {
                Ok(TickData {
                    timestamp: row.timestamp,
                    symbol: row.symbol.clone(),
                    price: row.price,
                    quantity: row.quantity,
                    side: self.parse_trade_side(&row.side)?,
                    trade_id: row.trade_id.clone(),
                    is_buyer_maker: row.is_buyer_maker,
                })
            })
            .collect();

        let ticks = ticks?;
        debug!("Retrieved {} historical ticks for backtest", ticks.len());
        Ok(ticks)
    }

    /// Get backtest data information for user selection
    pub async fn get_backtest_data_info(&self) -> DataResult<BacktestDataInfo> {
        debug!("Fetching backtest data information");
        
        // Get overall statistics
        let overall_stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_records,
                COUNT(DISTINCT symbol) as symbols_count,
                MIN(timestamp) as earliest_time,
                MAX(timestamp) as latest_time
            FROM tick_data
            "#
        )
        .fetch_one(&self.pool)
        .await?;

        // Get per-symbol statistics
        let symbol_stats = sqlx::query!(
            r#"
            SELECT 
                symbol,
                COUNT(*) as records_count,
                MIN(timestamp) as earliest_time,
                MAX(timestamp) as latest_time,
                MIN(price) as min_price,
                MAX(price) as max_price
            FROM tick_data
            GROUP BY symbol
            ORDER BY records_count DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        let symbol_info: Vec<SymbolDataInfo> = symbol_stats
            .into_iter()
            .map(|row| SymbolDataInfo {
                symbol: row.symbol,
                records_count: row.records_count.unwrap_or(0) as u64,
                earliest_time: row.earliest_time,
                latest_time: row.latest_time,
                min_price: row.min_price,
                max_price: row.max_price,
            })
            .collect();

        let info = BacktestDataInfo {
            total_records: overall_stats.total_records.unwrap_or(0) as u64,
            symbols_count: overall_stats.symbols_count.unwrap_or(0) as u64,
            earliest_time: overall_stats.earliest_time,
            latest_time: overall_stats.latest_time,
            symbol_info,
        };

        debug!("Backtest data info: {} total records, {} symbols", 
               info.total_records, info.symbols_count);
        Ok(info)
    }


    // =================================================================
    // Maintenance Operations
    // =================================================================

    /// Clean up old tick data
    pub async fn cleanup_old_data(&self, days_to_keep: f64) -> DataResult<u64> {
        info!("Cleaning up tick data older than {} days", days_to_keep);
        
        let result = sqlx::query!(
            r#"
            WITH deleted AS (
                DELETE FROM tick_data
                WHERE timestamp < NOW() - INTERVAL '1 day' * $1
                RETURNING *
            )
            SELECT COUNT(*) as count
            FROM deleted
            "#,
            days_to_keep
        )
        .fetch_one(&self.pool)
        .await?;

        let deleted_count = result.count.unwrap_or(0) as u64;
        info!("Cleaned up {} old tick data records", deleted_count);
        Ok(deleted_count)
    }

    /// Get database statistics
    pub async fn get_db_stats(&self, symbol: Option<&str>) -> DataResult<DbStats> {
        let (total_records, earliest_timestamp, latest_timestamp) = if let Some(sym) = symbol {
            let row = sqlx::query!(
                r#"
                SELECT 
                    COUNT(*) as total_records,
                    MIN(timestamp) as earliest_timestamp,
                    MAX(timestamp) as latest_timestamp
                FROM tick_data
                WHERE symbol = $1
                "#,
                sym
            )
            .fetch_one(&self.pool)
            .await?;
            
            (row.total_records, row.earliest_timestamp, row.latest_timestamp)
        } else {
            let row = sqlx::query!(
                r#"
                SELECT 
                    COUNT(*) as total_records,
                    MIN(timestamp) as earliest_timestamp,
                    MAX(timestamp) as latest_timestamp
                FROM tick_data
                "#
            )
            .fetch_one(&self.pool)
            .await?;
            
            (row.total_records, row.earliest_timestamp, row.latest_timestamp)
        };

        Ok(DbStats {
            symbol: symbol.map(|s| s.to_string()),
            total_records: total_records.unwrap_or(0) as u64,
            earliest_timestamp,
            latest_timestamp,
        })
    }

    // =================================================================
    // Helper Methods
    // =================================================================

    /// Validate tick data
    fn validate_tick_data(&self, tick: &TickData) -> DataResult<()> {
        if tick.symbol.is_empty() {
            return Err(DataError::Validation("Symbol cannot be empty".into()));
        }
        
        if tick.price <= Decimal::ZERO {
            return Err(DataError::Validation("Price must be positive".into()));
        }
        
        if tick.quantity <= Decimal::ZERO {
            return Err(DataError::Validation("Quantity must be positive".into()));
        }
        
        if tick.trade_id.is_empty() {
            return Err(DataError::Validation("Trade ID cannot be empty".into()));
        }

        Ok(())
    }

    /// Parse trade side from database string
    fn parse_trade_side(&self, side_str: &str) -> DataResult<TradeSide> {
        match side_str.to_uppercase().as_str() {
            "BUY" => Ok(TradeSide::Buy),
            "SELL" => Ok(TradeSide::Sell),
            _ => Err(DataError::InvalidFormat(format!("Invalid trade side: {}", side_str))),
        }
    }

    /// Check if query is for recent data (suitable for cache)
    fn is_recent_query(&self, query: &TickQuery) -> bool {
        if let Some(start_time) = query.start_time {
            let now = Utc::now();
            let duration = now - start_time;
            // Consider "recent" if within last hour
            duration <= Duration::hours(1)
        } else {
            // If no start time specified, assume it's a recent query
            true
        }
    }

    pub async fn insert_live_strategy_log(&self, log: &LiveStrategyLog) -> DataResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO live_strategy_log 
            (timestamp, strategy_id, symbol, current_price, signal_type, 
             portfolio_value, total_pnl, cache_hit, processing_time_us)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            log.timestamp,
            log.strategy_id,
            log.symbol,
            log.current_price,
            log.signal_type,
            log.portfolio_value,
            log.total_pnl,
            log.cache_hit,
            log.processing_time_us as i32
        )
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }

    /// Generate OHLC data from tick data for a specific time range
    pub async fn generate_ohlc_from_ticks(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        limit: Option<i64>,
    ) -> DataResult<Vec<OHLCData>> {
        debug!("Generating OHLC data: {} {} from {} to {}", 
               symbol, timeframe.as_str(), start_time, end_time);

        // Align start and end times to timeframe boundaries
        let aligned_start = timeframe.align_timestamp(start_time);
        let aligned_end = timeframe.align_timestamp(end_time);

        // Query all ticks in the time range
        let ticks = self.get_historical_data_for_backtest(
            symbol, 
            aligned_start, 
            aligned_end + timeframe.as_duration(), // Extend to include the last window
            limit
        ).await?;

        if ticks.is_empty() {
            debug!("No ticks found for OHLC generation");
            return Ok(Vec::new());
        }

        // Group ticks by time windows
        let mut windows: HashMap<DateTime<Utc>, Vec<TickData>> = HashMap::new();
        
        for tick in ticks {
            let window_start = timeframe.align_timestamp(tick.timestamp);
            windows.entry(window_start).or_insert_with(Vec::new).push(tick);
        }

        // Convert each window to OHLC
        let mut ohlc_data: Vec<OHLCData> = windows
            .into_iter()
            .filter_map(|(window_start, mut window_ticks)| {
                if window_start >= aligned_start && window_start <= aligned_end {
                    // Sort ticks by timestamp within each window
                    window_ticks.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                    OHLCData::from_ticks(&window_ticks, timeframe, window_start)
                } else {
                    None
                }
            })
            .collect();

        // Sort OHLC data by timestamp
        ohlc_data.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        debug!("Generated {} OHLC candles for {} {}", 
               ohlc_data.len(), symbol, timeframe.as_str());
        
        Ok(ohlc_data)
    }

    /// Generate recent OHLC data for backtesting (count-based)
    pub async fn generate_recent_ohlc_for_backtest(
        &self,
        symbol: &str,
        timeframe: Timeframe,
        candle_count: u32,
    ) -> DataResult<Vec<OHLCData>> {
        debug!("Generating recent {} OHLC candles for backtest: {} {}", 
               candle_count, symbol, timeframe.as_str());

        // Estimate how many ticks we need to get the required candles
        // This is a rough estimate - we might need to adjust
        let estimated_ticks_needed = (candle_count as i64) * 100; // Assume ~100 ticks per candle
        
        let recent_ticks = self.get_recent_ticks_for_backtest(symbol, estimated_ticks_needed).await?;
        
        if recent_ticks.is_empty() {
            return Ok(Vec::new());
        }

        // Get the time range from the actual data
        let start_time = recent_ticks[0].timestamp;
        let end_time = recent_ticks[recent_ticks.len() - 1].timestamp;
        
        // Generate OHLC from the time range
        let mut ohlc_data = self.generate_ohlc_from_ticks(
            symbol,
            timeframe,
            start_time,
            end_time,
            None
        ).await?;

        // Take only the requested number of recent candles
        ohlc_data.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Latest first
        ohlc_data.truncate(candle_count as usize);
        ohlc_data.reverse(); // Reverse to chronological order for backtesting

        debug!("Generated {} recent OHLC candles for backtest", ohlc_data.len());
        Ok(ohlc_data)
    }

    /// Get OHLC data statistics for a symbol
    pub async fn get_ohlc_data_info(
        &self,
        symbol: &str,
        timeframe: Timeframe,
    ) -> DataResult<(u64, Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
        // Get basic tick data info first
        let stats = self.get_db_stats(Some(symbol)).await?;
        
        if let (Some(earliest), Some(latest)) = (stats.earliest_timestamp, stats.latest_timestamp) {
            // Align to timeframe boundaries
            let aligned_earliest = timeframe.align_timestamp(earliest);
            let aligned_latest = timeframe.align_timestamp(latest);
            
            // Calculate approximate number of candles
            let duration_diff = aligned_latest - aligned_earliest;
            let timeframe_duration = timeframe.as_duration();
            
            let estimated_candles = if timeframe_duration.num_seconds() > 0 {
                (duration_diff.num_seconds() / timeframe_duration.num_seconds()) as u64
            } else {
                0
            };

            Ok((estimated_candles, Some(aligned_earliest), Some(aligned_latest)))
        } else {
            Ok((0, None, None))
        }
    }   
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    use dotenv::dotenv;
    use std::env;

    fn create_test_tick(symbol: &str, price: &str, trade_id: &str, timestamp: Option<DateTime<Utc>>) -> TickData {
        TickData::new(
            timestamp.unwrap_or_else(Utc::now),
            symbol.to_string(),
            Decimal::from_str(price).unwrap(),
            Decimal::from_str("1.0").unwrap(),
            TradeSide::Buy,
            trade_id.to_string(),
            false,
        )
    }

    async fn create_repository() -> TickDataRepository {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set in .env file");
        let redis_url = env::var("REDIS_URL")
            .expect("REDIS_URL must be set in .env file");
        let pool = PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to database");
        let cache = TieredCache::new((100, 300), (&redis_url, 1000, 3600))
            .await
            .expect("Failed to create cache");
        TickDataRepository::new(pool, cache)
    }

    async fn cleanup_database(pool: &PgPool, symbol: &str) {
        sqlx::query!("DELETE FROM tick_data WHERE symbol = $1", symbol)
            .execute(pool)
            .await
            .expect("Failed to clean up database");
    }

    #[tokio::test]
    async fn test_insert_and_read_single_tick() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_TEST_SINGLE";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Insert a single tick
        let tick = create_test_tick(symbol, "50000.0", "test1", None);
        repo.insert_tick(&tick).await.expect("Failed to insert tick");

        // Query the tick
        let query = TickQuery {
            symbol: symbol.to_string(),
            limit: Some(1),
            start_time: None,
            end_time: None,
            trade_side: None,
        };
        let ticks = repo.get_ticks(&query).await.expect("Failed to query ticks");

        // Verify
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].symbol, symbol);
        assert_eq!(ticks[0].price, Decimal::from_str("50000.0").unwrap());
        assert_eq!(ticks[0].trade_id, "test1");

        // Clean up
        cleanup_database(pool, symbol).await;
    }

    #[tokio::test]
    async fn test_batch_insert_and_read() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_TEST_BATCH";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Prepare batch ticks
        let ticks = vec![
            create_test_tick(symbol, "50000.0", "batch1", None),
            create_test_tick(symbol, "51000.0", "batch2", None),
            create_test_tick(symbol, "52000.0", "batch3", None),
        ];

        // Batch insert
        let inserted_count = repo.batch_insert(ticks.clone()).await.expect("Failed to batch insert");
        assert_eq!(inserted_count, 3);

        // Query ticks
        let query = TickQuery {
            symbol: symbol.to_string(),
            limit: Some(3),
            start_time: None,
            end_time: None,
            trade_side: None,
        };
        let queried_ticks = repo.get_ticks(&query).await.expect("Failed to query ticks");

        // Verify
        assert_eq!(queried_ticks.len(), 3);
        assert!(queried_ticks.iter().any(|t| t.trade_id == "batch1" && t.price == Decimal::from_str("50000.0").unwrap()));
        assert!(queried_ticks.iter().any(|t| t.trade_id == "batch2" && t.price == Decimal::from_str("51000.0").unwrap()));
        assert!(queried_ticks.iter().any(|t| t.trade_id == "batch3" && t.price == Decimal::from_str("52000.0").unwrap()));

        // Clean up
        cleanup_database(pool, symbol).await;
    }

    #[tokio::test]
    async fn test_cache_read_write() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let cache = repo.get_cache();
        let symbol = "BTCUSDT_TEST_CACHE";

        // Clean up before test
        cleanup_database(pool, symbol).await;
        cache.clear_symbol(symbol).await.expect("Failed to clear cache");

        // Insert a tick
        let tick = create_test_tick(symbol, "50000.0", "cache1", None);
        repo.insert_tick(&tick).await.expect("Failed to insert tick");

        // Query from cache
        let cached_ticks = cache.get_recent_ticks(symbol, 1).await.expect("Failed to read from cache");
        assert_eq!(cached_ticks.len(), 1);
        assert_eq!(cached_ticks[0].symbol, symbol);
        assert_eq!(cached_ticks[0].price, Decimal::from_str("50000.0").unwrap());
        assert_eq!(cached_ticks[0].trade_id, "cache1");

        // Query via get_ticks (should hit cache for recent data)
        let query = TickQuery {
            symbol: symbol.to_string(),
            limit: Some(1),
            start_time: Some(Utc::now() - Duration::hours(1)),
            end_time: None,
            trade_side: None,
        };
        let ticks = repo.get_ticks(&query).await.expect("Failed to query ticks");
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].symbol, symbol);
        assert_eq!(ticks[0].price, Decimal::from_str("50000.0").unwrap());

        // Clean up
        cleanup_database(pool, symbol).await;
        cache.clear_symbol(symbol).await.expect("Failed to clear cache");
    }

    #[tokio::test]
    async fn test_latest_price() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_TEST_PRICE";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Insert ticks with different timestamps
        let base_time = Utc::now();
        let tick1 = create_test_tick(symbol, "50000.0", "price1", Some(base_time));
        let tick2 = create_test_tick(symbol, "51000.0", "price2", Some(base_time + Duration::seconds(1)));
        repo.insert_tick(&tick1).await.expect("Failed to insert tick1");
        repo.insert_tick(&tick2).await.expect("Failed to insert tick2");

        // Query latest price
        let price = repo.get_latest_price(symbol).await.expect("Failed to get latest price");
        assert_eq!(price, Some(Decimal::from_str("51000.0").unwrap()));

        // Clean up
        cleanup_database(pool, symbol).await;
    }

    #[tokio::test]
    async fn test_tick_validation() {
        let repo = create_repository().await;

        let valid_tick = create_test_tick("BTCUSDT", "50000.0", "test1", None);
        assert!(repo.validate_tick_data(&valid_tick).is_ok());

        let invalid_tick = TickData::new(
            Utc::now(),
            "".to_string(),
            Decimal::from_str("50000.0").unwrap(),
            Decimal::from_str("1.0").unwrap(),
            TradeSide::Buy,
            "test".to_string(),
            false,
        );
        assert!(repo.validate_tick_data(&invalid_tick).is_err());
    }

    #[tokio::test]
    async fn test_get_recent_ticks_for_backtest() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_BACKTEST";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Insert ticks with different timestamps
        let base_time = Utc::now();
        let ticks = vec![
            create_test_tick(symbol, "50000.0", "bt1", Some(base_time)),
            create_test_tick(symbol, "51000.0", "bt2", Some(base_time + Duration::seconds(1))),
            create_test_tick(symbol, "52000.0", "bt3", Some(base_time + Duration::seconds(2))),
        ];

        for tick in ticks {
            repo.insert_tick(&tick).await.expect("Failed to insert tick");
        }

        // Get recent ticks for backtest
        let backtest_ticks = repo.get_recent_ticks_for_backtest(symbol, 3)
            .await
            .expect("Failed to get recent ticks for backtest");

        // Verify order is ASC (oldest first)
        assert_eq!(backtest_ticks.len(), 3);
        assert_eq!(backtest_ticks[0].trade_id, "bt1");
        assert_eq!(backtest_ticks[1].trade_id, "bt2");
        assert_eq!(backtest_ticks[2].trade_id, "bt3");
        assert!(backtest_ticks[0].timestamp <= backtest_ticks[1].timestamp);
        assert!(backtest_ticks[1].timestamp <= backtest_ticks[2].timestamp);

        // Clean up
        cleanup_database(pool, symbol).await;
    }

    #[tokio::test]
    async fn test_get_historical_data_for_backtest() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_BACKTEST";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Insert ticks with different timestamps
        let base_time = Utc::now();
        let ticks = vec![
            create_test_tick(symbol, "50000.0", "hist1", Some(base_time - Duration::hours(2))),
            create_test_tick(symbol, "51000.0", "hist2", Some(base_time - Duration::hours(1))),
            create_test_tick(symbol, "52000.0", "hist3", Some(base_time)),
        ];

        for tick in ticks {
            repo.insert_tick(&tick).await.expect("Failed to insert tick");
        }

        // Get historical data for backtest
        let start_time = base_time - Duration::hours(3);
        let end_time = base_time + Duration::hours(1);
        let historical_ticks = repo.get_historical_data_for_backtest(symbol, start_time, end_time, None)
            .await
            .expect("Failed to get historical data for backtest");

        // Verify order is ASC and within time range
        assert_eq!(historical_ticks.len(), 3);
        assert_eq!(historical_ticks[0].trade_id, "hist1");
        assert_eq!(historical_ticks[1].trade_id, "hist2");
        assert_eq!(historical_ticks[2].trade_id, "hist3");
        
        for tick in &historical_ticks {
            assert!(tick.timestamp >= start_time);
            assert!(tick.timestamp <= end_time);
        }

        // Clean up
        cleanup_database(pool, symbol).await;
    }

    #[tokio::test]
    async fn test_get_backtest_data_info() {
        let repo = create_repository().await;
        let pool = repo.get_pool();
        let symbol = "BTCUSDT_BACKTEST";

        // Clean up before test
        cleanup_database(pool, symbol).await;

        // Insert test data
        let tick = create_test_tick(symbol, "50000.0", "info1", None);
        repo.insert_tick(&tick).await.expect("Failed to insert tick");

        // Get backtest data info
        let info = repo.get_backtest_data_info()
            .await
            .expect("Failed to get backtest data info");

        // Verify structure
        assert!(info.total_records > 0);
        assert!(info.symbols_count > 0);
        assert!(info.earliest_time.is_some());
        assert!(info.latest_time.is_some());
        assert!(!info.symbol_info.is_empty());

        // Test helper methods
        let symbols = info.get_available_symbols();
        assert!(symbols.contains(&symbol.to_string()));
        
        let symbol_info = info.get_symbol_info(symbol);
        assert!(symbol_info.is_some());
        assert!(symbol_info.unwrap().records_count > 0);

        // Clean up
        cleanup_database(pool, symbol).await;
    }
}