'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar, LineChart, Line } from 'recharts';
import { Loader2, Database, TrendingUp, Activity, Zap, Clock, BarChart3, Play, Eye, Coins, Layers, Timer, Sparkles } from 'lucide-react';
import Link from 'next/link';

interface DataInfoResponse {
  total_records: number;
  symbols_count: number;
  earliest_time?: string;
  latest_time?: string;
  symbol_info: Array<{
    symbol: string;
    records_count: number;
    earliest_time?: string;
    latest_time?: string;
    min_price?: string;
    max_price?: string;
  }>;
}

interface StrategyCapability {
  id: string;
  name: string;
  description: string;
  supports_ohlc: boolean;
  preferred_timeframe?: string;
}

interface OHLCPreview {
  timestamp: string;
  symbol: string;
  open: string;
  high: string;
  low: string;
  close: string;
  volume: string;
  trade_count: number;
}

interface QuickBacktestResult {
  strategy: string;
  symbol: string;
  return_pct: number;
  final_value: number;
  trades: number;
  processing_time: number;
  data_source: string; // "tick" or "OHLC-1m" etc.
}

export default function Home() {
  const [loading, setLoading] = useState(true);
  const [dataInfo, setDataInfo] = useState<DataInfoResponse | null>(null);
  const [strategyCapabilities, setStrategyCapabilities] = useState<StrategyCapability[]>([]);
  const [quickResults, setQuickResults] = useState<QuickBacktestResult[]>([]);
  const [ohlcPreview, setOhlcPreview] = useState<OHLCPreview[]>([]);
  const [isRunningQuick, setIsRunningQuick] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loadingOhlcPreview, setLoadingOhlcPreview] = useState(false);
  const [selectedTimeframe, setSelectedTimeframe] = useState('1h');
  const [selectedSymbol, setSelectedSymbol] = useState('');

  useEffect(() => {
    initializeDashboard();
  }, []);

  useEffect(() => {
    if (selectedSymbol && selectedTimeframe) {
      loadOhlcPreview();
    }
  }, [selectedSymbol, selectedTimeframe]);

  const initializeDashboard = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const [dataInfoResult, capabilitiesResult] = await Promise.all([
        invoke<DataInfoResponse>('get_data_info'),
        invoke<StrategyCapability[]>('get_strategy_capabilities')
      ]);

      setDataInfo(dataInfoResult);
      setStrategyCapabilities(capabilitiesResult);

      // Set default symbol for OHLC preview
      if (dataInfoResult.symbol_info.length > 0) {
        const topSymbol = dataInfoResult.symbol_info
          .sort((a, b) => b.records_count - a.records_count)[0].symbol;
        setSelectedSymbol(topSymbol);
      }

    } catch (error) {
      console.error('Failed to initialize dashboard:', error);
      setError(error instanceof Error ? error.message : 'Failed to load dashboard data');
    } finally {
      setLoading(false);
    }
  };

  const loadOhlcPreview = async () => {
    if (!selectedSymbol || !selectedTimeframe) return;
    
    try {
      setLoadingOhlcPreview(true);
      
      // Calculate count based on timeframe
      const getCountByTimeframe = (tf: string) => {
        switch (tf) {
          case '1m': return 60; // Last hour in 1-minute candles
          case '5m': return 48; // Last 4 hours in 5-minute candles
          case '15m': return 32; // Last 8 hours in 15-minute candles
          case '30m': return 24; // Last 12 hours in 30-minute candles
          case '1h': return 24;  // Last day in 1-hour candles
          case '4h': return 24;  // Last 4 days in 4-hour candles
          case '1d': return 30;  // Last month in daily candles
          case '1w': return 12;  // Last 3 months in weekly candles
          default: return 24;
        }
      };
      
      const ohlcData = await invoke<OHLCPreview[]>('get_ohlc_preview', {
        request: {
          symbol: selectedSymbol,
          timeframe: selectedTimeframe,
          count: getCountByTimeframe(selectedTimeframe)
        }
      });
      
      setOhlcPreview(ohlcData);
    } catch (error) {
      console.error('Failed to load OHLC preview:', error);
      setOhlcPreview([]);
    } finally {
      setLoadingOhlcPreview(false);
    }
  };

  const runQuickBacktests = async () => {
    if (!dataInfo || !strategyCapabilities.length) return;
    
    setIsRunningQuick(true);
    setQuickResults([]);
    
    const topSymbols = dataInfo.symbol_info
      .sort((a, b) => b.records_count - a.records_count)
      .slice(0, 3);
    
    const results: QuickBacktestResult[] = [];
    
    for (const symbolInfo of topSymbols) {
      for (const strategy of strategyCapabilities.slice(0, 2)) { 
        try {
          const startTime = Date.now();
          
          const response = await invoke('run_backtest', {
            request: {
              strategy_id: strategy.id,
              symbol: symbolInfo.symbol,
              data_count: Math.min(5000, symbolInfo.records_count),
              initial_capital: "10000",
              commission_rate: "0.001",
              strategy_params: {}
            }
          }) as any;
          
          const processingTime = Date.now() - startTime;
          
          results.push({
            strategy: strategy.name,
            symbol: symbolInfo.symbol,
            return_pct: parseFloat(response.return_percentage),
            final_value: parseFloat(response.final_value),
            trades: response.total_trades,
            processing_time: processingTime,
            data_source: response.data_source || 'tick'
          });
          
          setQuickResults([...results]);
          
        } catch (error) {
          console.error(`Quick backtest failed for ${strategy.id} on ${symbolInfo.symbol}:`, error);
        }
      }
    }
    
    setIsRunningQuick(false);
  };

  const getDataCoverageDays = () => {
    if (!dataInfo?.earliest_time || !dataInfo?.latest_time) return 0;
    const start = new Date(dataInfo.earliest_time);
    const end = new Date(dataInfo.latest_time);
    return Math.floor((end.getTime() - start.getTime()) / (1000 * 60 * 60 * 24));
  };

  const getOhlcSupportCount = () => {
    return strategyCapabilities.filter(s => s.supports_ohlc).length;
  };

  const getAvgProcessingTime = () => {
    if (quickResults.length === 0) return 0;
    return quickResults.reduce((sum, r) => sum + r.processing_time, 0) / quickResults.length;
  };

  const getOhlcChartData = () => {
    if (ohlcPreview.length === 0) return [];
    
    // Calculate how many candles to show based on screen space
    const maxCandles = 20;
    const startIndex = Math.max(0, ohlcPreview.length - maxCandles);
    
    return ohlcPreview.slice(startIndex).map(candle => {
      const timestamp = new Date(candle.timestamp);
      
      // Format time display based on timeframe
      const formatTime = (timeframe: string, date: Date) => {
        switch (timeframe) {
          case '1m':
          case '5m':
          case '15m':
          case '30m':
            return date.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
          case '1h':
          case '4h':
            return date.toLocaleDateString([], {month: 'short', day: 'numeric'}) + ' ' +
                   date.toLocaleTimeString([], {hour: '2-digit'});
          case '1d':
            return date.toLocaleDateString([], {month: 'short', day: 'numeric'});
          case '1w':
            return date.toLocaleDateString([], {month: 'short', day: 'numeric'});
          default:
            return date.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
        }
      };
      
      return {
        time: formatTime(selectedTimeframe, timestamp),
        price: parseFloat(candle.close),
        volume: parseFloat(candle.volume),
        trades: candle.trade_count,
        high: parseFloat(candle.high),
        low: parseFloat(candle.low),
        open: parseFloat(candle.open),
      };
    });
  };

  const getCurrentSymbolInfo = () => {
    if (!dataInfo || !selectedSymbol) return null;
    return dataInfo.symbol_info.find(s => s.symbol === selectedSymbol);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <Loader2 className="w-8 h-8 animate-spin mr-2" />
        <span>Loading trading system dashboard...</span>
      </div>
    );
  }

  const ohlcChartData = getOhlcChartData();
  const ohlcSupportCount = getOhlcSupportCount();
  const symbolInfo = getCurrentSymbolInfo();

  return (
    <div className="space-y-6">
      {/* Welcome Header with OHLC Badge */}
      <div className="flex justify-between items-center">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <h1 className="text-3xl font-bold">Trading System Dashboard</h1>
            <Badge variant="secondary" className="flex items-center gap-1">
              <Layers className="w-3 h-3" />
              OHLC Enhanced
            </Badge>
          </div>
          <p className="text-gray-600 dark:text-gray-400">
            High-performance quantitative trading with dual data modes (Tick + OHLC)
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            onClick={runQuickBacktests}
            disabled={isRunningQuick || !dataInfo}
            variant="outline"
            className="flex items-center gap-2"
          >
            {isRunningQuick ? (
              <Loader2 className="w-4 h-4 animate-spin" />
            ) : (
              <Zap className="w-4 h-4" />
            )}
            Quick Test
          </Button>
          <Link href="/backtest">
            <Button className="flex items-center gap-2">
              <Play className="w-4 h-4" />
              Full Backtest
            </Button>
          </Link>
        </div>
      </div>

      {/* Enhanced System Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Records</CardTitle>
            <Database className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {dataInfo?.total_records.toLocaleString() || '0'}
            </div>
            <p className="text-xs text-muted-foreground">
              High-frequency tick data
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Trading Pairs</CardTitle>
            <Coins className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-600">
              {dataInfo?.symbols_count || 0}
            </div>
            <p className="text-xs text-muted-foreground">
              Cryptocurrency markets
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Data Coverage</CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {getDataCoverageDays()}
            </div>
            <p className="text-xs text-muted-foreground">
              Days of market data
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Total Strategies</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-600">
              {strategyCapabilities.length}
            </div>
            <p className="text-xs text-muted-foreground">
              Available algorithms
            </p>
          </CardContent>
        </Card>

        <Card className="border-2 border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20">
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">OHLC Support</CardTitle>
            <Sparkles className="h-4 w-4 text-blue-600" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-blue-600">
              {ohlcSupportCount}/{strategyCapabilities.length}
            </div>
            <p className="text-xs text-blue-600">
              Strategies support OHLC
            </p>
          </CardContent>
        </Card>
      </div>

      {/* OHLC Preview Chart */}
      <Card className="border-blue-200 bg-gradient-to-br from-blue-50 to-indigo-50 dark:from-blue-900/20 dark:to-indigo-900/20">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <BarChart3 className="w-5 h-5 text-blue-600" />
            Live OHLC Data Preview
            <Badge variant="outline" className="ml-2">
              {selectedSymbol} • {selectedTimeframe.toUpperCase()}
            </Badge>
          </CardTitle>
          
          {/* OHLC Controls */}
          <div className="flex flex-wrap gap-3 mt-3">
            <div className="flex items-center gap-2">
              <label className="text-sm font-medium">Symbol:</label>
              <select
                value={selectedSymbol}
                onChange={(e) => setSelectedSymbol(e.target.value)}
                className="text-sm px-2 py-1 border rounded dark:bg-gray-800 dark:border-gray-600"
              >
                {dataInfo?.symbol_info
                  .sort((a, b) => b.records_count - a.records_count)
                  .map((symbol) => (
                  <option key={symbol.symbol} value={symbol.symbol}>
                    {symbol.symbol} ({symbol.records_count.toLocaleString()})
                  </option>
                ))}
              </select>
            </div>
            
            <div className="flex items-center gap-2">
              <label className="text-sm font-medium">Timeframe:</label>
              <select
                value={selectedTimeframe}
                onChange={(e) => setSelectedTimeframe(e.target.value)}
                className="text-sm px-2 py-1 border rounded dark:bg-gray-800 dark:border-gray-600"
              >
                <option value="1m">1 Minute</option>
                <option value="5m">5 Minutes</option>
                <option value="15m">15 Minutes</option>
                <option value="30m">30 Minutes</option>
                <option value="1h">1 Hour</option>
                <option value="4h">4 Hours</option>
                <option value="1d">1 Day</option>
                <option value="1w">1 Week</option>
              </select>
            </div>
            
            <Button
              size="sm"
              variant="outline"
              onClick={loadOhlcPreview}
              disabled={loadingOhlcPreview}
              className="flex items-center gap-1"
            >
              {loadingOhlcPreview ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : (
                <Activity className="w-3 h-3" />
              )}
              Refresh
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <div className="lg:col-span-2">
              {ohlcChartData.length > 0 ? (
                <div className="h-64">
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={ohlcChartData}>
                      <CartesianGrid strokeDasharray="3 3" />
                      <XAxis dataKey="time" />
                      <YAxis 
                        domain={['dataMin - 10', 'dataMax + 10']}
                        tickFormatter={(value) => `$${value.toFixed(0)}`}
                      />
                      <Tooltip
                        formatter={(value: any) => [`$${value.toFixed(2)}`, 'Close Price']}
                        labelFormatter={(label) => `Time: ${label}`}
                      />
                      <Line
                        type="monotone"
                        dataKey="price"
                        stroke="#3b82f6"
                        strokeWidth={2}
                        dot={{ r: 3 }}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                </div>
              ) : (
                <div className="h-64 flex items-center justify-center text-gray-500">
                  {loadingOhlcPreview ? (
                    <div className="flex items-center gap-2">
                      <Loader2 className="w-5 h-5 animate-spin" />
                      <span>Loading OHLC data...</span>
                    </div>
                  ) : (
                    <span>No OHLC data available for current selection</span>
                  )}
                </div>
              )}
            </div>
            
            <div className="space-y-4">
              <div>
                <h4 className="font-medium text-blue-800 dark:text-blue-200 mb-2">
                  Latest OHLC Candle
                </h4>
                {ohlcPreview.length > 0 ? (
                  <div className="space-y-2 text-sm">
                    <div className="flex justify-between">
                      <span>Open:</span>
                      <span className="font-mono">${parseFloat(ohlcPreview[ohlcPreview.length - 1].open).toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>High:</span>
                      <span className="font-mono text-green-600">${parseFloat(ohlcPreview[ohlcPreview.length - 1].high).toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Low:</span>
                      <span className="font-mono text-red-600">${parseFloat(ohlcPreview[ohlcPreview.length - 1].low).toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Close:</span>
                      <span className="font-mono font-bold">${parseFloat(ohlcPreview[ohlcPreview.length - 1].close).toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Volume:</span>
                      <span className="font-mono">{parseFloat(ohlcPreview[ohlcPreview.length - 1].volume).toFixed(4)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Trades:</span>
                      <span className="font-mono">{ohlcPreview[ohlcPreview.length - 1].trade_count}</span>
                    </div>
                    <div className="flex justify-between">
                      <span>Time:</span>
                      <span className="font-mono text-xs">
                        {new Date(ohlcPreview[ohlcPreview.length - 1].timestamp).toLocaleString()}
                      </span>
                    </div>
                  </div>
                ) : (
                  <div className="text-sm text-gray-500 italic">
                    {loadingOhlcPreview ? 'Loading OHLC data...' : 'No OHLC data available'}
                  </div>
                )}
              </div>
              
              <div className="pt-2 border-t">
                <p className="text-xs text-blue-600 dark:text-blue-400">
                  OHLC data provides cleaner signals and faster backtesting for supported strategies
                </p>
                <p className="text-xs text-gray-500 mt-1">
                  Timeframe: {selectedTimeframe.toUpperCase()} • 
                  Candles: {ohlcPreview.length} • 
                  {selectedTimeframe === '1m' && 'Last hour'} 
                  {selectedTimeframe === '5m' && 'Last 4 hours'}
                  {selectedTimeframe === '15m' && 'Last 8 hours'}
                  {selectedTimeframe === '30m' && 'Last 12 hours'}
                  {selectedTimeframe === '1h' && 'Last day'}
                  {selectedTimeframe === '4h' && 'Last 4 days'}
                  {selectedTimeframe === '1d' && 'Last month'}
                  {selectedTimeframe === '1w' && 'Last 3 months'}
                </p>
                {symbolInfo && (
                  <p className="text-xs text-gray-500 mt-1">
                    Total records: {symbolInfo.records_count.toLocaleString()}
                  </p>
                )}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Enhanced Strategy Capabilities */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Activity className="w-5 h-5" />
            Strategy Capabilities Matrix
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {strategyCapabilities.map((strategy) => (
              <div
                key={strategy.id}
                className={`p-4 border rounded-lg ${
                  strategy.supports_ohlc 
                    ? 'border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-900/20'
                    : 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/20'
                }`}
              >
                <div className="flex items-start justify-between mb-2">
                  <div>
                    <h4 className="font-medium flex items-center gap-2">
                      {strategy.name}
                      {strategy.supports_ohlc && (
                        <Badge variant="secondary" className="text-xs">
                          <Layers className="w-3 h-3 mr-1" />
                          OHLC
                        </Badge>
                      )}
                    </h4>
                    <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      {strategy.description}
                    </p>
                  </div>
                  <Badge variant="outline" className="text-xs">
                    {strategy.id.toUpperCase()}
                  </Badge>
                </div>
                
                <div className="mt-3 flex items-center justify-between text-xs">
                  <div className="flex items-center gap-3">
                    <span className={`flex items-center gap-1 ${
                      strategy.supports_ohlc ? 'text-blue-600' : 'text-gray-500'
                    }`}>
                      <Layers className="w-3 h-3" />
                      {strategy.supports_ohlc ? 'OHLC Ready' : 'Tick Only'}
                    </span>
                    {strategy.preferred_timeframe && (
                      <span className="flex items-center gap-1 text-purple-600">
                        <Timer className="w-3 h-3" />
                        {strategy.preferred_timeframe}
                      </span>
                    )}
                  </div>
                </div>
              </div>
            ))}
          </div>
          
          <div className="mt-6 pt-4 border-t">
            <Link href="/backtest">
              <Button className="w-full">
                Configure & Run Advanced Backtest
              </Button>
            </Link>
          </div>
        </CardContent>
      </Card>

      {/* Enhanced Quick Backtest Results */}
      {(quickResults.length > 0 || isRunningQuick) && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="w-5 h-5" />
              Quick Strategy Performance Test
              {isRunningQuick && <Loader2 className="w-4 h-4 animate-spin ml-2" />}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isRunningQuick && quickResults.length === 0 && (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="w-6 h-6 animate-spin mr-2" />
                <span>Running quick backtests on top symbols...</span>
              </div>
            )}
            
            {quickResults.length > 0 && (
              <div className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-4 gap-4 mb-4">
                  <div className="text-center">
                    <p className="text-2xl font-bold text-green-600">
                      {quickResults.filter(r => r.return_pct > 0).length}
                    </p>
                    <p className="text-sm text-gray-500">Profitable Tests</p>
                  </div>
                  <div className="text-center">
                    <p className="text-2xl font-bold text-blue-600">
                      {getAvgProcessingTime().toFixed(0)}ms
                    </p>
                    <p className="text-sm text-gray-500">Avg Processing Time</p>
                  </div>
                  <div className="text-center">
                    <p className="text-2xl font-bold text-purple-600">
                      {quickResults.reduce((sum, r) => sum + r.trades, 0)}
                    </p>
                    <p className="text-sm text-gray-500">Total Trades</p>
                  </div>
                  <div className="text-center">
                    <p className="text-2xl font-bold text-orange-600">
                      {quickResults.filter(r => r.data_source.startsWith('OHLC')).length}
                    </p>
                    <p className="text-sm text-gray-500">OHLC Tests</p>
                  </div>
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="text-left border-b">
                        <th className="pb-2">Strategy</th>
                        <th className="pb-2">Symbol</th>
                        <th className="pb-2">Data Source</th>
                        <th className="pb-2">Return</th>
                        <th className="pb-2">Final Value</th>
                        <th className="pb-2">Trades</th>
                        <th className="pb-2">Time</th>
                      </tr>
                    </thead>
                    <tbody>
                      {quickResults
                        .sort((a, b) => b.return_pct - a.return_pct)
                        .map((result, index) => (
                          <tr key={index} className="border-b">
                            <td className="py-2 font-medium">{result.strategy}</td>
                            <td className="py-2">{result.symbol}</td>
                            <td className="py-2">
                              <Badge 
                                variant={result.data_source.startsWith('OHLC') ? 'default' : 'secondary'}
                                className="text-xs"
                              >
                                {result.data_source.startsWith('OHLC') 
                                  ? `OHLC-${result.data_source.split('-')[1]}`
                                  : 'Tick'
                                }
                              </Badge>
                            </td>
                            <td className={`py-2 font-medium ${
                              result.return_pct >= 0 ? 'text-green-500' : 'text-red-500'
                            }`}>
                              {result.return_pct >= 0 ? '+' : ''}{result.return_pct.toFixed(2)}%
                            </td>
                            <td className="py-2">${result.final_value.toFixed(2)}</td>
                            <td className="py-2">{result.trades}</td>
                            <td className="py-2 text-gray-500">{result.processing_time}ms</td>
                          </tr>
                        ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Data Quality Overview - Enhanced */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Eye className="w-5 h-5" />
            Market Data Overview
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <h4 className="font-medium mb-3">Top Symbols by Volume</h4>
              <div className="space-y-2">
                {dataInfo?.symbol_info
                  .sort((a, b) => b.records_count - a.records_count)
                  .slice(0, 5)
                  .map((symbol, index) => (
                  <div key={symbol.symbol} className="flex items-center justify-between">
                    <span className="flex items-center gap-2">
                      <span className="w-6 h-6 bg-blue-500 text-white text-xs rounded-full flex items-center justify-center">
                        {index + 1}
                      </span>
                      {symbol.symbol}
                    </span>
                    <div className="flex items-center gap-2">
                      <span className="text-sm text-gray-500">
                        {symbol.records_count.toLocaleString()} records
                      </span>
                      <Badge variant="outline" className="text-xs">
                        OHLC Ready
                      </Badge>
                    </div>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h4 className="font-medium mb-3">Data Capabilities</h4>
              <div className="space-y-3">
                <div className="flex items-center justify-between p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                  <div className="flex items-center gap-2">
                    <Layers className="w-4 h-4 text-blue-600" />
                    <span className="font-medium">OHLC Generation</span>
                  </div>
                  <Badge variant="secondary">Active</Badge>
                </div>
                <div className="flex items-center justify-between p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
                  <div className="flex items-center gap-2">
                    <Database className="w-4 h-4 text-green-600" />
                    <span className="font-medium">Tick Data</span>
                  </div>
                  <Badge variant="secondary">
                    {dataInfo?.total_records.toLocaleString() || '0'}
                  </Badge>
                </div>
                <div className="flex items-center justify-between p-3 bg-purple-50 dark:bg-purple-900/20 rounded-lg">
                  <div className="flex items-center gap-2">
                    <Timer className="w-4 h-4 text-purple-600" />
                    <span className="font-medium">Timeframes</span>
                  </div>
                  <Badge variant="secondary">1m-1w</Badge>
                </div>
                <div className="flex items-center justify-between p-3 bg-orange-50 dark:bg-orange-900/20 rounded-lg">
                  <div className="flex items-center gap-2">
                    <Clock className="w-4 h-4 text-orange-600" />
                    <span className="font-medium">Coverage</span>
                  </div>
                  <Badge variant="secondary">
                    {getDataCoverageDays()} days
                  </Badge>
                </div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Error Display */}
      {error && (
        <Card className="border-red-200 bg-red-50 dark:border-red-800 dark:bg-red-900/20">
          <CardContent className="pt-6">
            <p className="text-red-800 dark:text-red-200">{error}</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}