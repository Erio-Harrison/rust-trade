// rust-trade: 一个用 Rust 编写的量化交易系统
// Copyright (C) 2024 Harrison // 版权所有 (C) 2024 Harrison
//
// 这个程序是 rust-trade 的一部分，并且在 GNU GPL v3 或更高版本下发布。
// 详情请参阅 LICENSE 文件。

#![cfg_attr( // 条件编译属性，用于特定配置
  all(not(debug_assertions), target_os = "windows"), // 当不是调试模式且目标操作系统是 Windows 时
  windows_subsystem = "windows" // 设置 Windows 子系统为 "windows"，避免在 Windows 上打开控制台窗口
)]

mod commands; // 引入 commands 模块
mod state; // 引入 state 模块

use commands::run_backtest; // 从 commands 模块中引入 run_backtest 函数
use state::AppState; // 从 state 模块中引入 AppState 结构体

fn main() { // 主函数，程序的入口点
    // 初始化日志，设置更详细的级别 // 初始化日志，设置更详细的级别
    tracing_subscriber::fmt() // 使用 tracing_subscriber 来配置日志格式
        .with_max_level(tracing::Level::DEBUG) // 设置最大日志级别为 DEBUG，以获取更详细的日志输出
        .with_file(true) // 在日志中包含文件名
        .with_line_number(true) // 在日志中包含行号
        .init(); // 初始化日志记录器

    // 记录启动信息 // 记录启动信息
    tracing::info!("Application starting..."); // 记录一条信息级别的日志，表示应用程序开始启动

    // 创建运行时 // 创建运行时
    let runtime = match tokio::runtime::Runtime::new() { // 尝试创建一个新的 Tokio 运行时
        Ok(rt) => { // 如果成功创建
            tracing::info!("Tokio runtime created successfully"); // 记录信息：Tokio 运行时创建成功
            rt // 返回创建的运行时
        }
        Err(e) => { // 如果创建失败
            tracing::error!("Failed to create Tokio runtime: {}", e); // 记录错误：创建 Tokio 运行时失败，并附带错误信息
            std::process::exit(1); // 退出程序，返回状态码 1
        }
    };

    // 在运行时中初始化状态 // 在运行时中初始化状态
    let app_state = runtime.block_on(async { // 在 Tokio 运行时中同步执行一个异步块
        match AppState::new().await { // 异步调用 AppState::new() 来创建应用状态
            Ok(state) => { // 如果成功创建
                tracing::info!("App state initialized successfully"); // 记录信息：应用状态初始化成功
                state // 返回创建的应用状态
            }
            Err(e) => { // 如果创建失败
                tracing::error!("Failed to initialize app state: {}", e); // 记录错误：初始化应用状态失败，并附带错误信息
                std::process::exit(1); // 退出程序，返回状态码 1
            }
        }
    });

    // 构建和运行 Tauri 应用 // 构建和运行 Tauri 应用
    let result = tauri::Builder::default() // 使用 Tauri 的默认构建器开始构建应用
        .manage(app_state) // 将应用状态 (app_state) 交给 Tauri 管理
        .invoke_handler(tauri::generate_handler![run_backtest]) // 注册 invoke 处理程序，这里是 run_backtest 命令
        .setup(|app| { // 配置应用的设置过程
            tracing::info!("Tauri setup started"); // 记录信息：Tauri 设置开始
            #[cfg(debug_assertions)] // 条件编译：仅在调试模式下执行以下代码块
            {
                let app_handle = app.handle(); // 获取应用句柄
                app_handle.plugin(tauri_plugin_shell::init())?; // 初始化并注册 tauri_plugin_shell 插件
                tracing::info!("Debug plugins initialized"); // 记录信息：调试插件初始化完成
            }
            tracing::info!("Tauri setup completed"); // 记录信息：Tauri 设置完成
            Ok(()) // 返回成功
        })
        .run(tauri::generate_context!()); // 运行 Tauri 应用，并传入生成的上下文

    // 处理运行结果 // 处理运行结果
    match result { // 匹配应用的运行结果
        Ok(_) => tracing::info!("Application exited normally"), // 如果应用正常退出，记录信息
        Err(e) => { // 如果应用出错退出
            tracing::error!("Application error: {}", e); // 记录错误：应用出错，并附带错误信息
            std::process::exit(1); // 退出程序，返回状态码 1
        }
    }
}