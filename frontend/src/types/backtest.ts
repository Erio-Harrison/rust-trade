// src/types/backtest.ts
export interface DataInfoResponse {
  total_records: number;
  symbols_count: number;
  earliest_time?: string;
  latest_time?: string;
  symbol_info: SymbolInfo[];
}

export interface SymbolInfo {
  symbol: string;
  records_count: number;
  earliest_time?: string;
  latest_time?: string;
  min_price?: string;
  max_price?: string;
}

export interface StrategyInfo {
  id: string;
  name: string;
  description: string;
}

export interface BacktestRequest {
  strategy_id: string;
  symbol: string;
  data_count: number;
  initial_capital: string;
  commission_rate: string;
  strategy_params: Record<string, string>;
}

export interface BacktestResponse {
  strategy_name: string;
  initial_capital: string;
  final_value: string;
  total_pnl: string;
  return_percentage: string;
  total_trades: number;
  winning_trades: number;
  losing_trades: number;
  max_drawdown: string;
  sharpe_ratio: string;
  volatility: string;
  win_rate: string;
  profit_factor: string;
  total_commission: string;
  trades: TradeInfo[];
  equity_curve: string[];
}

export interface TradeInfo {
  timestamp: string;
  symbol: string;
  side: string;
  quantity: string;
  price: string;
  realized_pnl?: string;
  commission: string;
}

export interface HistoricalDataRequest {
  symbol: string;
  limit?: number;
}

export interface TickDataResponse {
  timestamp: string;
  symbol: string;
  price: string;
  quantity: string;
  side: string;
}