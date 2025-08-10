use crate::state::AppState;
use crate::types::*;
use tauri::State;
use trading_core::{
    backtest::{
        engine::{BacktestEngine, BacktestConfig},
        strategy::create_strategy,
    },
    data::types::TradeSide,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use std::str::FromStr;
use tracing::{info, error};

#[tauri::command]
pub async fn get_data_info(
    state: State<'_, AppState>,
) -> Result<DataInfoResponse, String> {
    info!("Getting backtest data info");
    
    let data_info = state.repository
        .get_backtest_data_info()
        .await
        .map_err(|e| {
            error!("Failed to get data info: {}", e);
            e.to_string()
        })?;

    let response = DataInfoResponse {
        total_records: data_info.total_records,
        symbols_count: data_info.symbols_count,
        earliest_time: data_info.earliest_time.map(|t| t.to_rfc3339()),
        latest_time: data_info.latest_time.map(|t| t.to_rfc3339()),
        symbol_info: data_info.symbol_info.into_iter().map(|info| SymbolInfo {
            symbol: info.symbol,
            records_count: info.records_count,
            earliest_time: info.earliest_time.map(|t| t.to_rfc3339()),
            latest_time: info.latest_time.map(|t| t.to_rfc3339()),
            min_price: info.min_price.map(|p| p.to_string()),
            max_price: info.max_price.map(|p| p.to_string()),
        }).collect(),
    };

    info!("Data info retrieved successfully: {} symbols, {} total records", 
          response.symbols_count, response.total_records);
    Ok(response)
}

#[tauri::command]
pub async fn get_available_strategies() -> Result<Vec<StrategyInfo>, String> {
    info!("Getting available strategies");
    
    let strategies = trading_core::backtest::strategy::list_strategies();
    let response: Vec<StrategyInfo> = strategies.into_iter().map(|s| StrategyInfo {
        id: s.id,
        name: s.name,
        description: s.description,
    }).collect();

    info!("Retrieved {} strategies", response.len());
    Ok(response)
}

#[tauri::command]
pub async fn validate_backtest_config(
    state: State<'_, AppState>,
    symbol: String,
    data_count: i64,
) -> Result<bool, String> {
    info!("Validating backtest config for symbol: {}, data_count: {}", symbol, data_count);
    
    let data_info = state.repository
        .get_backtest_data_info()
        .await
        .map_err(|e| e.to_string())?;

    let is_valid = data_info.has_sufficient_data(&symbol, data_count as u64);
    info!("Validation result: {}", is_valid);
    
    Ok(is_valid)
}

#[tauri::command]
pub async fn get_historical_data(
    state: State<'_, AppState>,
    request: HistoricalDataRequest,
) -> Result<Vec<TickDataResponse>, String> {
    info!("Getting historical data for symbol: {}, limit: {:?}", 
          request.symbol, request.limit);
    
    let limit = request.limit.unwrap_or(1000).min(10000);
    let data = state.repository
        .get_recent_ticks_for_backtest(&request.symbol, limit)
        .await
        .map_err(|e| {
            error!("Failed to get historical data: {}", e);
            e.to_string()
        })?;

    let response: Vec<TickDataResponse> = data.into_iter().map(|tick| TickDataResponse {
        timestamp: tick.timestamp.to_rfc3339(),
        symbol: tick.symbol,
        price: tick.price.to_string(),
        quantity: tick.quantity.to_string(),
        side: match tick.side {
            TradeSide::Buy => "Buy".to_string(),
            TradeSide::Sell => "Sell".to_string(),
        },
    }).collect();

    info!("Retrieved {} historical data points", response.len());
    Ok(response)
}

#[tauri::command]
pub async fn run_backtest(
    state: State<'_, AppState>,
    request: BacktestRequest,
) -> Result<BacktestResponse, String> {
    info!("Starting backtest: strategy={}, symbol={}, data_count={}", 
          request.strategy_id, request.symbol, request.data_count);

    let initial_capital = Decimal::from_str(&request.initial_capital)
        .map_err(|_| "Invalid initial capital")?;
    let commission_rate = Decimal::from_str(&request.commission_rate)
        .map_err(|_| "Invalid commission rate")?;

    info!("Loading historical data...");
    let data = state.repository
        .get_recent_ticks_for_backtest(&request.symbol, request.data_count)
        .await
        .map_err(|e| {
            error!("Failed to load historical data: {}", e);
            e.to_string()
        })?;

    if data.is_empty() {
        return Err("No historical data available for the specified symbol".to_string());
    }

    info!("Loaded {} data points", data.len());

    let mut config = BacktestConfig::new(initial_capital)
        .with_commission_rate(commission_rate);

    for (key, value) in request.strategy_params {
        config = config.with_param(&key, &value);
    }

    info!("Creating strategy: {}", request.strategy_id);
    let strategy = create_strategy(&request.strategy_id)
        .map_err(|e| {
            error!("Failed to create strategy: {}", e);
            e
        })?;

    info!("Initializing backtest engine");
    let mut engine = BacktestEngine::new(strategy, config)
        .map_err(|e| {
            error!("Failed to create backtest engine: {}", e);
            e
        })?;

    info!("Running backtest...");
    let result = engine.run(data);

    info!("Backtest completed successfully");

    let response = BacktestResponse {
        strategy_name: result.strategy_name.clone(),
        initial_capital: result.initial_capital.to_string(),
        final_value: result.final_value.to_string(),
        total_pnl: result.total_pnl.to_string(),
        return_percentage: result.return_percentage.to_string(),
        total_trades: result.total_trades,
        winning_trades: result.winning_trades,
        losing_trades: result.losing_trades,
        max_drawdown: result.max_drawdown.to_string(),
        sharpe_ratio: result.sharpe_ratio.to_string(),
        volatility: result.volatility.to_string(),
        win_rate: result.win_rate.to_string(),
        profit_factor: result.profit_factor.to_string(),
        total_commission: result.total_commission.to_string(),
        trades: result.trades.into_iter().map(|trade| TradeInfo {
            timestamp: trade.timestamp.to_rfc3339(),
            symbol: trade.symbol,
            side: match trade.side {
                trading_core::data::types::TradeSide::Buy => "Buy".to_string(),
                trading_core::data::types::TradeSide::Sell => "Sell".to_string(),
            },
            quantity: trade.quantity.to_string(),
            price: trade.price.to_string(),
            realized_pnl: trade.realized_pnl.map(|pnl| pnl.to_string()),
            commission: trade.commission.to_string(),
        }).collect(),
        equity_curve: result.equity_curve.into_iter().map(|value| value.to_string()).collect(),
    };

    info!("Backtest response prepared: {} trades, {:.2}% return", 
          response.total_trades, result.return_percentage.to_f64().unwrap_or(0.0));

    Ok(response)
}