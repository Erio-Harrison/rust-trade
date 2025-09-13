use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct DataInfoResponse {
    pub total_records: u64,
    pub symbols_count: u64,
    pub earliest_time: Option<String>,
    pub latest_time: Option<String>,
    pub symbol_info: Vec<SymbolInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub symbol: String,
    pub records_count: u64,
    pub earliest_time: Option<String>,
    pub latest_time: Option<String>,
    pub min_price: Option<String>,
    pub max_price: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacktestRequest {
    pub strategy_id: String,
    pub symbol: String,
    pub data_count: i64,
    pub initial_capital: String,
    pub commission_rate: String,
    pub strategy_params: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacktestResponse {
    pub strategy_name: String,
    pub initial_capital: String,
    pub final_value: String,
    pub total_pnl: String,
    pub return_percentage: String,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub max_drawdown: String,
    pub sharpe_ratio: String,
    pub volatility: String,
    pub win_rate: String,
    pub profit_factor: String,
    pub total_commission: String,
    pub trades: Vec<TradeInfo>,
    pub equity_curve: Vec<String>,
    pub data_source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeInfo {
    pub timestamp: String,
    pub symbol: String,
    pub side: String,
    pub quantity: String,
    pub price: String,
    pub realized_pnl: Option<String>,
    pub commission: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HistoricalDataRequest {
    pub symbol: String,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TickDataResponse {
    pub timestamp: String,
    pub symbol: String,
    pub price: String,
    pub quantity: String,
    pub side: String,
}


#[derive(Debug, Serialize, Deserialize)]
pub struct StrategyCapability {
    pub id: String,
    pub name: String,
    pub description: String,
    pub supports_ohlc: bool,
    pub preferred_timeframe: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OHLCPreview {
    pub timestamp: String,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub trade_count: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OHLCRequest {
    pub symbol: String,
    pub timeframe: String,
    pub count: u32,
}