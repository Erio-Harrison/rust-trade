use crate::state::AppState;
use crate::types::*;
use tauri::State;
use trading_common::{
    backtest::{
        engine::{BacktestEngine, BacktestConfig, BacktestResult},
        strategy::create_strategy,
    },
    data::types::TradeSide,
};
use rust_decimal::Decimal;

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
    
    let strategies = trading_common::backtest::strategy::list_strategies();
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

    let mut config = BacktestConfig::new(initial_capital)
        .with_commission_rate(commission_rate);

    for (key, value) in request.strategy_params {
        config = config.with_param(&key, &value);
    }

    info!("Creating strategy: {}", request.strategy_id);
    let temp_strategy = create_strategy(&request.strategy_id)
        .map_err(|e| {
            error!("Failed to create strategy: {}", e);
            e
        })?;

    let mut data_source = "tick".to_string();

    // Check if strategy supports OHLC
    if temp_strategy.supports_ohlc() {
        if let Some(timeframe) = temp_strategy.preferred_timeframe() {
            info!("Strategy supports OHLC, attempting {} timeframe", timeframe.as_str());
            
            // Estimate candle count (roughly data_count / 50, minimum 100)
            let candle_count = (request.data_count / 50).max(100) as u32;
            
            match state.repository.generate_recent_ohlc_for_backtest(
                &request.symbol, 
                timeframe, 
                candle_count
            ).await {
                Ok(ohlc_data) if !ohlc_data.is_empty() => {
                    info!("Generated {} OHLC candles, running OHLC backtest", ohlc_data.len());
                    data_source = format!("OHLC-{}", timeframe.as_str());
                    
                    let strategy = create_strategy(&request.strategy_id)?;
                    let mut engine = BacktestEngine::new(strategy, config)
                        .map_err(|e| {
                            error!("Failed to create backtest engine: {}", e);
                            e
                        })?;

                    let result = engine.run_with_ohlc(ohlc_data);
                    return Ok(create_backtest_response(result, data_source));
                },
                Ok(_) => {
                    info!("No OHLC data available, falling back to tick data");
                },
                Err(e) => {
                    info!("OHLC generation failed: {}, falling back to tick data", e);
                }
            }
        }
    }

    // Fallback to tick data
    info!("Loading tick data for backtest");
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

    info!("Loaded {} tick data points, running tick backtest", data.len());

    let strategy = create_strategy(&request.strategy_id)?;
    let mut engine = BacktestEngine::new(strategy, config)
        .map_err(|e| {
            error!("Failed to create backtest engine: {}", e);
            e
        })?;

    let result = engine.run(data);
    Ok(create_backtest_response(result, data_source))
}

// 3. Add helper function to commands.rs
fn create_backtest_response(result: BacktestResult, data_source: String) -> BacktestResponse {
    info!("Backtest completed successfully");

    BacktestResponse {
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
        data_source, // NEW FIELD
        trades: result.trades.into_iter().map(|trade| TradeInfo {
            timestamp: trade.timestamp.to_rfc3339(),
            symbol: trade.symbol,
            side: match trade.side {
                trading_common::data::types::TradeSide::Buy => "Buy".to_string(),
                trading_common::data::types::TradeSide::Sell => "Sell".to_string(),
            },
            quantity: trade.quantity.to_string(),
            price: trade.price.to_string(),
            realized_pnl: trade.realized_pnl.map(|pnl| pnl.to_string()),
            commission: trade.commission.to_string(),
        }).collect(),
        equity_curve: result.equity_curve.into_iter().map(|value| value.to_string()).collect(),
    }
}

#[tauri::command]
pub async fn get_strategy_capabilities() -> Result<Vec<StrategyCapability>, String> {
    info!("Getting strategy capabilities");
    
    let strategies = trading_common::backtest::strategy::list_strategies();
    let mut capabilities = Vec::new();
    
    for strategy_info in strategies {
        // Create temporary strategy instance to check capabilities
        match trading_common::backtest::strategy::create_strategy(&strategy_info.id) {
            Ok(strategy) => {
                capabilities.push(StrategyCapability {
                    id: strategy_info.id,
                    name: strategy_info.name,
                    description: strategy_info.description,
                    supports_ohlc: strategy.supports_ohlc(),
                    preferred_timeframe: strategy.preferred_timeframe().map(|tf| tf.as_str().to_string()),
                });
            }
            Err(e) => {
                info!("Failed to create strategy {}: {}", strategy_info.id, e);
                capabilities.push(StrategyCapability {
                    id: strategy_info.id,
                    name: strategy_info.name,
                    description: strategy_info.description,
                    supports_ohlc: false,
                    preferred_timeframe: None,
                });
            }
        }
    }
    
    info!("Retrieved capabilities for {} strategies", capabilities.len());
    Ok(capabilities)
}

#[tauri::command]
pub async fn get_ohlc_preview(
    state: State<'_, AppState>,
    request: OHLCRequest,
) -> Result<Vec<OHLCPreview>, String> {
    info!("Getting OHLC preview: {} {} count={}", 
          request.symbol, request.timeframe, request.count);
    
    let timeframe = match request.timeframe.as_str() {
        "1m" => trading_common::data::types::Timeframe::OneMinute,
        "5m" => trading_common::data::types::Timeframe::FiveMinutes,
        "15m" => trading_common::data::types::Timeframe::FifteenMinutes,
        "30m" => trading_common::data::types::Timeframe::ThirtyMinutes,
        "1h" => trading_common::data::types::Timeframe::OneHour,
        "4h" => trading_common::data::types::Timeframe::FourHours,
        "1d" => trading_common::data::types::Timeframe::OneDay,
        "1w" => trading_common::data::types::Timeframe::OneWeek,
        _ => return Err(format!("Invalid timeframe: {}", request.timeframe)),
    };
    
    let ohlc_data = state.repository
        .generate_recent_ohlc_for_backtest(&request.symbol, timeframe, request.count)
        .await
        .map_err(|e| {
            error!("Failed to generate OHLC preview: {}", e);
            e.to_string()
        })?;
    
    if ohlc_data.is_empty() {
        return Err("No OHLC data available for the specified parameters".to_string());
    }
    
    let response: Vec<OHLCPreview> = ohlc_data.into_iter().map(|ohlc| OHLCPreview {
        timestamp: ohlc.timestamp.to_rfc3339(),
        symbol: ohlc.symbol,
        open: ohlc.open.to_string(),
        high: ohlc.high.to_string(),
        low: ohlc.low.to_string(),
        close: ohlc.close.to_string(),
        volume: ohlc.volume.to_string(),
        trade_count: ohlc.trade_count,
    }).collect();
    
    info!("Generated {} OHLC preview records", response.len());
    Ok(response)
}