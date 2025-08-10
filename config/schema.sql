-- =================================================================
-- Core Data Table for Quantitative Trading System: Tick Data
-- Design Principles: Single table storage, high-performance queries, data integrity
-- =================================================================

CREATE TABLE tick_data (
    -- 【Timestamp】UTC time, supports millisecond precision
    -- Why use TIMESTAMP WITH TIME ZONE:
    -- 1. Global markets require a unified timezone (UTC)
    -- 2. Supports millisecond-level precision for high-frequency trading needs
    -- 3. Time zone info avoids issues like daylight saving time
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    
    -- 【Trading Pair】e.g., 'BTCUSDT', 'ETHUSDT'
    -- Why use VARCHAR(20):
    -- 1. Cryptocurrency trading pairs are typically 8-15 characters
    -- 2. Reserved space for future new trading pairs
    -- 3. Fixed length storage offers better performance
    symbol VARCHAR(20) NOT NULL,
    
    -- 【Trade Price】Use DECIMAL to ensure precision
    -- Why use DECIMAL(20, 8):
    -- 1. Total 20 digits: supports prices in the trillions
    -- 2. 8 decimal places: meets cryptocurrency precision requirements (Bitcoin has 8 decimals)
    -- 3. Avoids floating point precision loss
    price DECIMAL(20, 8) NOT NULL,
    
    -- 【Trade Quantity】Also uses DECIMAL to ensure precision
    -- Why use DECIMAL(20, 8):
    -- 1. Trade volume calculations require high precision
    -- 2. Consistent precision with price
    quantity DECIMAL(20, 8) NOT NULL,
    
    -- 【Trade Side】Buy or Sell
    -- Why use VARCHAR(4) + CHECK constraint:
    -- 1. 'BUY'/'SELL' is more intuitive than boolean
    -- 2. CHECK constraint enforces data validity
    -- 3. Facilitates SQL querying and reporting
    side VARCHAR(4) NOT NULL CHECK (side IN ('BUY', 'SELL')),
    
    -- 【Trade ID】Original trade identifier from exchange
    -- Why use VARCHAR(50):
    -- 1. Different exchanges have different ID formats (numeric, alphanumeric, UUID, etc.)
    -- 2. Used for deduplication and traceability
    -- 3. Supports various exchange ID lengths
    trade_id VARCHAR(50) NOT NULL,
    
    -- 【Maker Flag】Whether the buyer is the maker (order placer)
    -- Why this field is needed:
    -- 1. Distinguish between aggressive and passive trades
    -- 2. Calculate market liquidity metrics
    -- 3. Basis for fee calculation
    is_buyer_maker BOOLEAN NOT NULL
);

-- =================================================================
-- Index Strategy: Optimized for different query scenarios
-- Principle: Balance query performance and write efficiency
-- =================================================================

-- 【Index 1】Real-time trading query index
-- Use cases:
-- - Fetch the latest price for a trading pair: WHERE symbol = 'BTCUSDT' ORDER BY timestamp DESC LIMIT 1
-- - Get recent N minutes data of a trading pair: WHERE symbol = 'BTCUSDT' AND timestamp >= NOW() - INTERVAL '5 minutes'
-- - Real-time price push, risk control checks, and other high-frequency operations
-- Design notes:
-- - Composite index (symbol, timestamp DESC): group by symbol first, then order by time descending
-- - DESC order: prioritizes newest data, aligns with real-time query needs
-- - Supports index-only scans to avoid heap fetches and improve performance
CREATE INDEX idx_tick_symbol_time ON tick_data(symbol, timestamp DESC);

-- 【Index 2】Data integrity unique index
-- Use cases:
-- - Prevent duplicate data insertion due to network retransmission or program restart (idempotency)
-- - Data consistency checks to ensure no duplicated trade records
-- Design notes:
-- - Unique constraint on three fields: same symbol + same trade_id + same timestamp = unique record
-- - Unique constraint implicitly creates corresponding unique index to support fast duplicate checks
-- - Business logic aligns with financial system requirement of no duplicate and no missing data
CREATE UNIQUE INDEX idx_tick_unique ON tick_data(symbol, trade_id, timestamp);

-- 【Index 3】Backtesting time index
-- Use cases:
-- - Multi-symbol backtesting: WHERE timestamp BETWEEN '2025-01-01' AND '2025-01-02' AND symbol IN (...)
-- - Market-wide statistics: WHERE timestamp >= '2025-01-01' GROUP BY symbol
-- - Time-range data export: batch processing historical data by time intervals
-- Design notes:
-- - Single-column time index: more efficient than composite index when queries do not filter by symbol
-- - Supports range queries: BETWEEN operation fully utilizes B-tree index
-- - Essential for backtesting: ensures performance of historical data analysis
CREATE INDEX idx_tick_timestamp ON tick_data(timestamp);

-- =================================================================
-- Design Validation and Performance Testing
-- =================================================================

-- Verify table structure
\d tick_data

-- View index information and sizes
SELECT 
    indexname,
    indexdef,
    pg_size_pretty(pg_relation_size(indexname::regclass)) AS size
FROM pg_indexes 
WHERE tablename = 'tick_data'
ORDER BY indexname;

-- Performance test queries (run after data is inserted)
/*
-- Real-time query test
EXPLAIN (ANALYZE, BUFFERS) 
SELECT * FROM tick_data 
WHERE symbol = 'BTCUSDT' 
ORDER BY timestamp DESC 
LIMIT 10;

-- Backtesting query test
EXPLAIN (ANALYZE, BUFFERS)
SELECT COUNT(*), AVG(price) 
FROM tick_data 
WHERE timestamp BETWEEN NOW() - INTERVAL '1 day' AND NOW()
AND symbol IN ('BTCUSDT', 'ETHUSDT');
*/
