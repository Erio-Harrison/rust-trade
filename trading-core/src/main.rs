use bigdecimal::{FromPrimitive, Zero};
use clap::Parser;
use dotenv::dotenv;
use tracing::{info, error};
use std::sync::Arc;
use std::str::FromStr;
use chrono::{Duration, Utc};
use rust_decimal::Decimal;

use trading_core::{
    backtest::{engine::BacktestEngine, sma::SMAStrategy, types::OrderSide, BacktestConfig}, 
    config::Settings, 
    data::{database::Database, types::MarketDataManager}, 
    exchange::binance::BinanceSpot, 
    market_data_collector::MarketDataCollector
};

#[derive(Parser)]
#[command(name = "rust-trade")]
#[command(about = "A Rust trading system with backtest capabilities")]
enum Commands {
    Server,
    Backtest {
        #[arg(short, long, default_value = "BTCUSDT")]
        symbol: String,
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long, default_value = "10000.0")]
        initial_capital: String,
        #[arg(short, long, default_value = "0.001")]
        commission_rate: String,
        #[arg(long, default_value = "5")]
        short_period: usize,
        #[arg(long, default_value = "20")]
        long_period: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let settings = Settings::new()?;
    let database = Database::new(&settings.database).await?;
    database.check_connection().await?;
    info!("Database connection established");

    let command = Commands::try_parse().unwrap_or(Commands::Server);

    match command {
        Commands::Server => {
            let exchange = BinanceSpot::new(None);
            let collector = Arc::new(MarketDataCollector::new(
                Box::new(exchange),
                MarketDataManager::new(database.pool.clone()),
                settings.symbols.clone(),
            ));

            let collector_clone = collector.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = collector_clone.start().await {
                    error!("Market data collector error: {}", e);
                }
            });

            info!("Market data collector started. Press Ctrl+C to stop.");

            tokio::signal::ctrl_c().await?;
            info!("Shutting down server...");
            collector.stop();
            handle.await?;
            info!("Server shutdown complete");
        }

        Commands::Backtest { 
            symbol, 
            days, 
            initial_capital, 
            commission_rate,
            short_period,
            long_period,
        } => {
            let market_data = MarketDataManager::new(database.pool);
            
            let start_time = Utc::now() - Duration::days(days);
            let end_time = Utc::now();
            
            let data = market_data.get_market_data(&symbol, start_time, end_time).await?;
            if data.is_empty() {
                error!("No historical data found for {} in the specified time range", symbol);
                return Err("Insufficient historical data for backtest".into());
            }
            info!("Found {} historical data points", data.len());

            let config = BacktestConfig {
                start_time,
                end_time,
                initial_capital: Decimal::from_str(&initial_capital)?,
                symbol: symbol.clone(),
                commission_rate: Decimal::from_str(&commission_rate)?,
            };

            let position_size = match data.first() {
                Some(first_data) => {
                    let capital = Decimal::from_str(&initial_capital)?;
                    (capital * Decimal::from_f64(0.1).unwrap()) / Decimal::from_f64(first_data.price).unwrap()
                }
                None => Decimal::zero()
            };
            let strategy = SMAStrategy::new(
                symbol.clone(),
                short_period,
                long_period,
                position_size,
            );
            let mut engine = BacktestEngine::new(market_data, config);
            let result = engine.run_strategy(Box::new(strategy)).await?;

            println!("\nBacktest Results:");
            println!("Total Return: {}%", result.metrics.total_return);
            println!("Total Trades: {}", result.metrics.total_trades);
            println!("Win Rate: {}%", result.metrics.win_rate);
            println!("Sharpe Ratio: {}", result.metrics.sharpe_ratio);
            println!("Max Drawdown: {}%", result.metrics.max_drawdown);
            println!("\nTrade History:");
            for trade in result.trades {
                println!(
                    "{} {} {} @ {}",
                    trade.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    if trade.side == OrderSide::Buy { "BUY" } else { "SELL" },
                    trade.quantity,
                    trade.price
                );
            }
        }
    }

    Ok(())
}