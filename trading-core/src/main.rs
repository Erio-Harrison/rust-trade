use std::sync::{Arc};
use std::time::Duration;
use rust_decimal::Decimal;
use sqlx::PgPool;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod data;
mod exchange;
mod service;
mod backtest;
mod live_trading; 

use config::Settings;
use data::{repository::TickDataRepository, cache::TieredCache};
use exchange::BinanceExchange;
use service::MarketDataService;
use live_trading::PaperTradingProcessor;

use crate::data::cache::TickDataCache;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    match args.get(1).map(|s| s.as_str()) {
        Some("backtest") => run_backtest_mode().await,
        Some("live") => {
            // Check if paper trading is enabled
            if args.contains(&"--paper-trading".to_string()) {
                run_live_with_paper_trading().await
            } else {
                run_live_mode().await
            }
        }
        None => run_live_mode().await,
        Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        _ => {
            eprintln!("❌ Unknown command: {}", args[1]);
            print_usage();
            std::process::exit(1);
        }
    }
}

fn print_usage() {
    println!("Trading Core - Cryptocurrency Data Collection & Backtesting System");
    println!();
    println!("Usage:");
    println!("  cargo run                # Run live data collection");
    println!("  cargo run live           # Run live data collection");
    println!("  cargo run backtest       # Run backtesting mode");
    println!("  cargo run --help         # Show this help message");
    println!();
}

async fn run_live_with_paper_trading() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize application environment
    init_application().await?;

    info!("🎯 Starting Trading Core Application (Live Mode + Paper Trading)");

    // Load configuration
    let settings = Settings::new()?;
    
    // Check if paper trading is enabled
    if !settings.paper_trading.enabled {
        warn!("⚠️ Paper trading is disabled in config. Set paper_trading.enabled = true");
        warn!("⚠️ Falling back to live data collection only...");
        return run_live_mode().await;
    }
    
    info!("📋 Configuration loaded successfully");
    info!("📊 Monitoring symbols: {:?}", settings.symbols);
    info!("🎯 Paper Trading Strategy: {}", settings.paper_trading.strategy);
    info!("💰 Initial Capital: ${}", settings.paper_trading.initial_capital);
    info!("🗄️  Database: {} connections", settings.database.max_connections);
    info!("💾 Cache: Memory({} ticks/{}s) + Redis({} ticks/{}s)", 
          settings.cache.memory.max_ticks_per_symbol,
          settings.cache.memory.ttl_seconds,
          settings.cache.redis.max_ticks_per_symbol,
          settings.cache.redis.ttl_seconds);

    // Verify strategy exists
    if crate::backtest::strategy::get_strategy_info(&settings.paper_trading.strategy).is_none() {
        error!("❌ Unknown strategy: {}", settings.paper_trading.strategy);
        error!("💡 Available strategies: rsi, sma");
        std::process::exit(1);
    }

    // Create database connection pool
    info!("🔌 Connecting to database...");
    let pool = create_database_pool(&settings).await?;
    test_database_connection(&pool).await?;
    info!("✅ Database connection established");

    // Create cache
    info!("💾 Initializing cache...");
    let cache = create_cache(&settings).await?;
    info!("✅ Cache initialized");

    // Create repository
    let repository = Arc::new(TickDataRepository::new(pool, cache));

    // Create exchange connection
    info!("📡 Initializing exchange connection...");
    let exchange = Arc::new(BinanceExchange::new());
    info!("✅ Exchange connection ready");

    // Create strategy
    info!("🧠 Initializing strategy: {}", settings.paper_trading.strategy);
    let strategy = crate::backtest::strategy::create_strategy(&settings.paper_trading.strategy)?;
    info!("✅ Strategy initialized: {}", strategy.name());

    // Create paper trading processor
    let initial_capital = Decimal::try_from(settings.paper_trading.initial_capital)
        .map_err(|e| format!("Invalid initial capital: {}", e))?;
    let paper_trading = Arc::new(tokio::sync::Mutex::new(
        PaperTradingProcessor::new(strategy, Arc::clone(&repository), initial_capital)
    ));

    // Create market data service
    let service = MarketDataService::new(
        exchange,
        repository, 
        settings.symbols.clone(),
    ).with_paper_trading(paper_trading);

    info!("🎯 Starting market data collection with paper trading for {} symbols", settings.symbols.len());
    println!("🚀 Paper trading is now active! Watch for trading signals below...");
    println!("📈 Strategy: {} | Initial Capital: ${}", settings.paper_trading.strategy, settings.paper_trading.initial_capital);
    println!("{}", "=".repeat(80));

    // Start service
    run_live_application_with_service(service).await?;

    info!("✅ Application stopped gracefully");
    Ok(())
}

async fn run_live_application_with_service(service: MarketDataService) -> Result<(), Box<dyn std::error::Error>> {
    let service_shutdown_tx = service.get_shutdown_tx();
    
    // Start signal forwarding task
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        println!("\nReceived Ctrl+C signal, forwarding to service...");
        info!("Received Ctrl+C signal, forwarding to service");
        let _ = service_shutdown_tx.send(());
    });
    
    // Just wait for service to complete
    match service.start().await {
        Ok(()) => {
            info!("Service stopped successfully");
            Ok(())
        }
        Err(e) => {
            error!("Service stopped with error: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Real-time mode entry
async fn run_live_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize environment and logging
    init_application().await?;

    info!("🚀 Starting Trading Core Application (Live Mode)");

    // Load configuration
    let settings = Settings::new()?;
    
    info!("📋 Configuration loaded successfully");
    info!("📊 Monitoring symbols: {:?}", settings.symbols);
    info!("🗄️  Database: {} connections", settings.database.max_connections);
    info!("💾 Cache: Memory({} ticks/{}s) + Redis({} ticks/{}s)", 
          settings.cache.memory.max_ticks_per_symbol,
          settings.cache.memory.ttl_seconds,
          settings.cache.redis.max_ticks_per_symbol,
          settings.cache.redis.ttl_seconds);

    // Create and start the application
    run_live_application(settings).await?;

    info!("✅ Application stopped gracefully");
    Ok(())
}

/// Backtesting mode entry
async fn run_backtest_mode() -> Result<(), Box<dyn std::error::Error>> {
    init_application().await?;
    
    info!("🔬 Starting Trading Core Application (Backtest Mode)");
    
    let settings = Settings::new()?;
    info!("📋 Configuration loaded successfully");
    
    let pool = create_database_pool(&settings).await?;
    test_database_connection(&pool).await?;
    info!("✅ Database connection established");
    
    let cache = create_backtest_cache(&settings).await?;
    info!("✅ Cache initialized for backtest");
    
    let repository = TickDataRepository::new(pool, cache);
    
    run_backtest_interactive(repository).await?;
    
    info!("✅ Backtest completed successfully");
    Ok(())
}

/// Backtesting interactive interface
async fn run_backtest_interactive(
    repository: TickDataRepository
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::backtest::{
        engine::{BacktestEngine, BacktestConfig}, 
        strategy::{create_strategy, list_strategies}
    };
    use std::io::{self, Write};
    use rust_decimal::Decimal;
    use std::str::FromStr;
    
    println!("{}", "=".repeat(60));
    println!("🎯 TRADING CORE BACKTESTING SYSTEM");
    println!("{}", "=".repeat(60));
    
    // Display statistics
    println!("📊 Loading data statistics...");
    let data_info = repository.get_backtest_data_info().await?;
    
    println!("\n📈 Available Data:");
    println!("  Total Records: {}", data_info.total_records);
    println!("  Available Symbols: {}", data_info.symbols_count);
    
    if let Some(earliest) = data_info.earliest_time {
        println!("  Earliest Data: {}", earliest.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    if let Some(latest) = data_info.latest_time {
        println!("  Latest Data: {}", latest.format("%Y-%m-%d %H:%M:%S UTC"));
    }
    
    println!("\n📋 Symbol Details:");
    for (i, symbol_info) in data_info.symbol_info.iter().take(10).enumerate() {
        println!("  {}: {} ({} records)", 
                 i + 1, symbol_info.symbol, symbol_info.records_count);
    }
    
    if data_info.symbol_info.len() > 10 {
        println!("  ... and {} more symbols", data_info.symbol_info.len() - 10);
    }
    
    // Strategy Selection
    println!("\n🎯 Available Strategies:");
    let strategies = list_strategies();
    for (i, strategy) in strategies.iter().enumerate() {
        println!("  {}) {} - {}", i + 1, strategy.name, strategy.description);
    }
    
    print!("\nSelect strategy (1-{}): ", strategies.len());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().unwrap_or(0);
    
    if choice == 0 || choice > strategies.len() {
        println!("❌ Invalid selection");
        return Ok(());
    }
    
    let selected_strategy = &strategies[choice - 1];
    println!("✅ Selected Strategy: {}", selected_strategy.name);
    
    // Trading pair selection
    println!("\n📊 Symbol Selection:");
    let available_symbols = data_info.get_available_symbols();
    
    // Display the first 10 symbols for quick selection
    for (i, symbol) in available_symbols.iter().take(10).enumerate() {
        let symbol_info = data_info.get_symbol_info(symbol).unwrap();
        println!("  {}) {} ({} records)", i + 1, symbol, symbol_info.records_count);
    }
    
    print!("\nSelect symbol (1-{}) or enter custom symbol: ", available_symbols.len().min(10));
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();
    
    let symbol = if let Ok(choice) = input.parse::<usize>() {
        if choice > 0 && choice <= available_symbols.len().min(10) {
            available_symbols[choice - 1].clone()
        } else {
            println!("❌ Invalid selection");
            return Ok(());
        }
    } else if input.is_empty() {
        "BTCUSDT".to_string()
    } else {
        input.to_uppercase()
    };
    
    // Verify whether the selected transaction pair has data
    if !data_info.has_sufficient_data(&symbol, 100) {
        println!("❌ Insufficient data for symbol: {} (minimum 100 records required)", symbol);
        return Ok(());
    }
    
    let symbol_info = data_info.get_symbol_info(&symbol).unwrap();
    println!("✅ Selected Symbol: {} ({} records available)", symbol, symbol_info.records_count);
    
    // Data quantity selection
    print!("\nEnter number of records to backtest (default: 10000, max: {}): ", symbol_info.records_count);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let data_count: i64 = if input.trim().is_empty() {
        10000.min(symbol_info.records_count as i64)
    } else {
        input.trim().parse().unwrap_or(10000).min(symbol_info.records_count as i64)
    };
    
    // Initial Funding Setup
    print!("\nEnter initial capital (default: $10000): $");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let initial_capital = if input.trim().is_empty() {
        Decimal::from(10000)
    } else {
        Decimal::from_str(input.trim()).unwrap_or(Decimal::from(10000))
    };
    
    // Commission rate setting
    print!("\nEnter commission rate % (default: 0.1%): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let commission_rate = if input.trim().is_empty() {
        Decimal::from_str("0.001").unwrap() // 0.1%
    } else {
        let rate = input.trim().parse::<f64>().unwrap_or(0.1);
        Decimal::from_str(&format!("{}", rate / 100.0)).unwrap_or(Decimal::from_str("0.001").unwrap())
    };
    
    println!("\n🔍 Loading historical data: {} latest {} records...", symbol, data_count);
    
    let data = repository.get_recent_ticks_for_backtest(&symbol, data_count).await?;
    
    if data.is_empty() {
        println!("❌ No historical data found for symbol: {}", symbol);
        return Ok(());
    }
    
    println!("✅ Loaded {} data points", data.len());
    println!("📅 Data range: {} to {}", 
             data.first().unwrap().timestamp.format("%Y-%m-%d %H:%M:%S"), 
             data.last().unwrap().timestamp.format("%Y-%m-%d %H:%M:%S"));
    
    let config = BacktestConfig::new(initial_capital)
        .with_commission_rate(commission_rate);
    
    let strategy = create_strategy(&selected_strategy.id)?;
    
    println!("\n{}", "=".repeat(60));
    let mut engine = BacktestEngine::new(strategy, config)?;
    let result = engine.run(data);
    
    // Show results
    println!("\n");
    result.print_summary();
    
    // Ask whether to display detailed transaction analysis
    print!("\nShow detailed trade analysis? (y/N): ");
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes" {
        result.print_trade_analysis();
    }
    
    println!("\n🎉 Backtest completed successfully!");
    
    Ok(())
}

/// Initialize application environment and logging
async fn init_application() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Initialize tracing/logging
    init_tracing()?;

    info!("🔧 Application environment initialized");
    Ok(())
}

/// Initialize tracing subscriber for logging
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // Create env filter from RUST_LOG environment variable
    // Default to info level if not set
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("trading_core=info,sqlx=info,tokio=info,hyper=info")
        });

    // Setup tracing subscriber with structured logging
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true)
                .compact()
        )
        .init();

    Ok(())
}

/// Main application runtime (original live mode)
async fn run_live_application(settings: Settings) -> Result<(), Box<dyn std::error::Error>> {
    // Validate basic configuration
    if settings.symbols.is_empty() {
        error!("❌ No symbols configured for monitoring");
        std::process::exit(1);
    }

    if settings.database.max_connections == 0 {
        error!("❌ Database max_connections must be greater than 0");
        std::process::exit(1);
    }

    // Create database connection pool
    info!("🔌 Connecting to database...");
    let pool = create_database_pool(&settings).await?;
    
    // Test database connectivity
    test_database_connection(&pool).await?;
    info!("✅ Database connection established");

    // Create cache
    info!("💾 Initializing cache...");
    let cache = create_cache(&settings).await?;
    info!("✅ Cache initialized");

    // Create repository
    let repository = Arc::new(TickDataRepository::new(pool, cache));

    // Create exchange
    info!("📡 Initializing exchange connection...");
    let exchange = Arc::new(BinanceExchange::new());
    info!("✅ Exchange connection ready");

    // Create market data service
    let service = MarketDataService::new(
        exchange,
        repository,
        settings.symbols.clone(),
    );

    info!("🎯 Starting market data collection for {} symbols", settings.symbols.len());

    // Setup signal forwarding to service
    let service_shutdown_tx = service.get_shutdown_tx();
    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
        println!("\nReceived Ctrl+C signal, forwarding to service...");
        info!("Received Ctrl+C signal, forwarding to service");
        let _ = service_shutdown_tx.send(());
    });

    // Start service and wait for completion
    match service.start().await {
        Ok(()) => {
            info!("✅ Service stopped successfully");
            Ok(())
        }
        Err(e) => {
            error!("❌ Service stopped with error: {}", e);
            Err(Box::new(e))
        }
    }
}

/// Create database connection pool
async fn create_database_pool(settings: &Settings) -> Result<PgPool, Box<dyn std::error::Error>> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .min_connections(settings.database.min_connections)
        .max_lifetime(Duration::from_secs(settings.database.max_lifetime))
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .connect(&settings.database.url)
        .await?;

    Ok(pool)
}

/// Test database connection
async fn test_database_connection(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Simple connectivity test
    sqlx::query("SELECT 1")
        .execute(pool)
        .await?;

    // Check if tick_data table exists
    let table_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_name = 'tick_data'
        )"
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        error!("❌ Required table 'tick_data' does not exist in database");
        error!("💡 Please run the database migration scripts first");
        std::process::exit(1);
    }

    info!("✅ Database schema validation passed");
    Ok(())
}

/// Create cache instance (original live mode)
async fn create_cache(settings: &Settings) -> Result<TieredCache, Box<dyn std::error::Error>> {
    let memory_config = (
        settings.cache.memory.max_ticks_per_symbol,
        settings.cache.memory.ttl_seconds,
    );

    let redis_config = (
        settings.cache.redis.url.as_str(),
        settings.cache.redis.max_ticks_per_symbol,
        settings.cache.redis.ttl_seconds,
    );

    let cache = TieredCache::new(memory_config, redis_config).await?;
    
    // Test cache connectivity
    test_cache_connection(&cache).await?;
    
    Ok(cache)
}

/// Create simplified cache for backtest mode
async fn create_backtest_cache(settings: &Settings) -> Result<TieredCache, Box<dyn std::error::Error>> {
    // Creating a minimal cache configuration for backtesting
    let memory_config = (10, 60);
    let redis_config = (
        settings.cache.redis.url.as_str(),
        10, 
        60
    );
    
    let cache = TieredCache::new(memory_config, redis_config).await?;
    
    // Simple connection test (not required to be completely normal, because backtesting mainly uses the database)
    if let Err(e) = test_cache_connection(&cache).await {
        warn!("⚠️ Cache test failed (this is OK for backtest mode): {}", e);
    }
    
    Ok(cache)
}

/// Test cache connection
async fn test_cache_connection(cache: &TieredCache) -> Result<(), Box<dyn std::error::Error>> {
    // Test cache by getting symbols (should return empty list initially)
    cache.get_symbols().await?;
    info!("✅ Cache connectivity test passed");
    Ok(())
}