use std::sync::Arc; // 引入 Arc，用于原子引用计数，实现共享所有权
use trading_core::{ // 引入 trading_core 核心库
    data::{ // 数据模块
        database::Database, // 数据库模块
        types::MarketDataManager, // 市场数据管理器
    },
    config::Settings, // 配置模块
};

pub struct AppState { // 定义应用状态结构体 AppState
    pub market_manager: Arc<MarketDataManager>, // 市场数据管理器，使用 Arc 包装以实现多线程共享
}

impl AppState { // 实现 AppState 结构体的方法
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> { // 定义异步构造函数 new，返回 Result 包含 AppState 或错误
        let settings = Settings::new()?; // 加载应用配置，如果失败则返回错误
        let database = Database::new(&settings.database).await?; // 根据配置初始化数据库连接，如果失败则返回错误
        
        Ok(Self { // 如果初始化成功，返回 AppState 实例
            market_manager: Arc::new(MarketDataManager::new(database.pool)), // 创建 MarketDataManager 并用 Arc 包装
        })
    }
}