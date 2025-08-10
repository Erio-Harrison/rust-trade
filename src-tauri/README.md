# Trading Desktop

A Tauri-based desktop application for cryptocurrency backtesting, built on top of Trading Core.

## 🏗️ Architecture

### **Project Structure**
```
src-tauri/
├── src/
│   ├── main.rs          # Application entry point and setup
│   ├── state.rs         # Application state management and database initialization
│   ├── types.rs         # Frontend interface types and serialization
│   └── commands.rs      # Tauri command handlers for frontend communication
├── Cargo.toml           # Dependencies and Tauri configuration
└── .env                 # Environment configuration
```

### **Core Components**

#### **State Management (`state.rs`)**
- **AppState**: Manages Trading Core repository instance
- **Database Connection**: PostgreSQL connection pool management
- **Cache Initialization**: Simplified caching for GUI applications
- **Configuration Loading**: Environment-based settings with fallbacks

#### **Command Handlers (`commands.rs`)**
- **`get_data_info`**: Retrieve database statistics and available symbols
- **`get_available_strategies`**: List all implemented trading strategies
- **`validate_backtest_config`**: Validate backtest parameters before execution
- **`get_historical_data`**: Preview historical data for selected symbols
- **`run_backtest`**: Execute complete backtesting with strategy and parameters

#### **Type Definitions (`types.rs`)**
- **Request Types**: Structured input from frontend (BacktestRequest, HistoricalDataRequest)
- **Response Types**: Formatted output to frontend (BacktestResponse, DataInfoResponse)
- **Serde Integration**: JSON serialization for seamless frontend communication

## 🚀 Features

### **Backtesting Capabilities**
- **Strategy Selection**: Choose from built-in SMA and RSI strategies
- **Parameter Configuration**: Customizable strategy parameters via GUI
- **Historical Data Access**: Direct access to Trading Core's tick data
- **Performance Metrics**: Comprehensive analysis including Sharpe ratio, drawdown, win rate
- **Trade Analysis**: Detailed trade-by-trade breakdown with P&L tracking

### **Data Management**
- **Real-time Validation**: Parameter validation before backtest execution
- **Data Statistics**: Database overview with symbol information and date ranges
- **Error Handling**: Robust error propagation from backend to frontend

## ⚙️ Configuration

### **Environment Variables**
```bash
DATABASE_URL=postgresql://username:password@localhost/trading_core
REDIS_URL=redis://127.0.0.1:6379
```

### **Dependencies**
- **Tauri 2.0**: Desktop application framework
- **Trading Core**: Backend trading infrastructure
- **SQLx**: Database connectivity
- **Serde**: JSON serialization
- **Tokio**: Async runtime

## 🔧 Development

### **Setup**
```bash
# Install dependencies
cargo build

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

### **Requirements**
- Trading Core project at `../trading-core`
- PostgreSQL with `trading_core` database
- Redis server (optional but recommended)
- Rust 1.70+

## 📊 API Interface

### **Frontend Commands**
```typescript
// Get database information
invoke<DataInfoResponse>('get_data_info')

// Run backtest
invoke<BacktestResponse>('run_backtest', { 
  request: BacktestRequest 
})

// Validate configuration
invoke<boolean>('validate_backtest_config', {
  symbol: string,
  data_count: number
})
```

### **Data Flow**
```
Frontend → Tauri Commands → Trading Core Repository → PostgreSQL
                     ↓
Frontend ← JSON Response ← Backtest Engine ← Historical Data
```

## 🎯 Integration

This Tauri application serves as the desktop GUI layer for Trading Core, providing:
- **Seamless Integration**: Direct access to Trading Core's backtesting engine
- **Type Safety**: Rust-based backend with TypeScript frontend compatibility
- **Performance**: Native desktop performance with web-based UI flexibility
- **Cross-Platform**: Windows, macOS, and Linux support through Tauri

The application maintains full compatibility with Trading Core's CLI interface while offering an enhanced user experience through its graphical interface.