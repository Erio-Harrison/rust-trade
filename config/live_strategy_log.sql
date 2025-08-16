-- Simple real-time strategy log table
CREATE TABLE live_strategy_log (
id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
strategy_id VARCHAR(50) NOT NULL,
symbol VARCHAR(20) NOT NULL,
current_price DECIMAL(18,8) NOT NULL,
signal_type VARCHAR(10) NOT NULL, -- BUY/SELL/HOLD
portfolio_value DECIMAL(18,8) NOT NULL,
total_pnl DECIMAL(18,8) NOT NULL DEFAULT 0,
cache_hit BOOLEAN DEFAULT TRUE, -- Mark whether to get data from cache
processing_time_us INTEGER -- Processing time (microseconds), reflecting cache value
);

-- Basic index
CREATE INDEX idx_live_strategy_time ON live_strategy_log(timestamp DESC);
CREATE INDEX idx_live_strategy_symbol ON live_strategy_log(strategy_id, symbol);