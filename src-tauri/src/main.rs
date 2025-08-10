#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod commands;
mod state;
mod types;

use commands::*;
use state::AppState;

fn main() {
    if let Err(_) = dotenvy::dotenv() {
        println!("Warning: .env file not found, using environment variables");
    }
    
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Trading Core Tauri Application starting...");

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => {
            tracing::info!("Tokio runtime created successfully");
            rt
        }
        Err(e) => {
            tracing::error!("Failed to create Tokio runtime: {}", e);
            std::process::exit(1);
        }
    };

    let app_state = runtime.block_on(async {
        match AppState::new().await {
            Ok(state) => {
                tracing::info!("App state initialized successfully");
                state
            }
            Err(e) => {
                tracing::error!("Failed to initialize app state: {}", e);
                tracing::error!("Please check your configuration:");
                tracing::error!("1. Ensure .env file exists with DATABASE_URL and REDIS_URL");
                tracing::error!("2. Ensure PostgreSQL is running and accessible");
                tracing::error!("3. Ensure Redis is running (optional but recommended)");
                tracing::error!("4. Ensure trading_core database and tick_data table exist");
                std::process::exit(1);
            }
        }
    });

    let result = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_data_info,
            get_available_strategies,
            run_backtest,
            get_historical_data,
            validate_backtest_config
        ])
        .setup(|app| {
            tracing::info!("Tauri setup started");
            #[cfg(debug_assertions)]
            {
                let app_handle = app.handle();
                if let Err(e) = app_handle.plugin(tauri_plugin_shell::init()) {
                    tracing::warn!("Failed to initialize shell plugin: {}", e);
                }
                tracing::info!("Debug plugins initialized");
            }
            tracing::info!("Tauri setup completed");
            Ok(())
        })
        .run(tauri::generate_context!());

    match result {
        Ok(_) => tracing::info!("Application exited normally"),
        Err(e) => {
            tracing::error!("Application error: {}", e);
            std::process::exit(1);
        }
    }
}