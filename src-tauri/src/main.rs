#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

mod commands;
mod state;

use commands::run_backtest;
use state::AppState;

fn main() {
    // 初始化日志，设置更详细的级别
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .init();

    // 记录启动信息
    tracing::info!("Application starting...");

    // 创建运行时
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

    // 在运行时中初始化状态
    let app_state = runtime.block_on(async {
        match AppState::new().await {
            Ok(state) => {
                tracing::info!("App state initialized successfully");
                state
            }
            Err(e) => {
                tracing::error!("Failed to initialize app state: {}", e);
                std::process::exit(1);
            }
        }
    });

    // 构建和运行 Tauri 应用
    let result = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![run_backtest])
        .setup(|app| {
            tracing::info!("Tauri setup started");
            #[cfg(debug_assertions)]
            {
                let app_handle = app.handle();
                app_handle.plugin(tauri_plugin_shell::init())?;
                tracing::info!("Debug plugins initialized");
            }
            tracing::info!("Tauri setup completed");
            Ok(())
        })
        .run(tauri::generate_context!());

    // 处理运行结果
    match result {
        Ok(_) => tracing::info!("Application exited normally"),
        Err(e) => {
            tracing::error!("Application error: {}", e);
            std::process::exit(1);
        }
    }
}