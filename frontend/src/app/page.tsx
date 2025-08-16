'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, BarChart, Bar } from 'recharts';
import { Loader2, Database, TrendingUp, Activity, Zap, Clock, BarChart3, Play, Eye, Coins } from 'lucide-react';
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

interface StrategyInfo {
  id: string;
  name: string;
  description: string;
}

interface QuickBacktestResult {
  strategy: string;
  symbol: string;
  return_pct: number;
  final_value: number;
  trades: number;
  processing_time: number;
}

export default function Home() {
  const [loading, setLoading] = useState(true);
  const [dataInfo, setDataInfo] = useState<DataInfoResponse | null>(null);
  const [strategies, setStrategies] = useState<StrategyInfo[]>([]);
  const [quickResults, setQuickResults] = useState<QuickBacktestResult[]>([]);
  const [isRunningQuick, setIsRunningQuick] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initializeDashboard();
  }, []);

  const initializeDashboard = async () => {
    try {
      setLoading(true);
      setError(null);
      
      const [dataInfoResult, strategiesResult] = await Promise.all([
        invoke<DataInfoResponse>('get_data_info'),
        invoke<StrategyInfo[]>('get_available_strategies')
      ]);

      setDataInfo(dataInfoResult);
      setStrategies(strategiesResult);

    } catch (error) {
      console.error('Failed to initialize dashboard:', error);
      setError(error instanceof Error ? error.message : 'Failed to load dashboard data');
    } finally {
      setLoading(false);
    }
  };

  const runQuickBacktests = async () => {
    if (!dataInfo || !strategies.length) return;
    
    setIsRunningQuick(true);
    setQuickResults([]);
    
    const topSymbols = dataInfo.symbol_info
      .sort((a, b) => b.records_count - a.records_count)
      .slice(0, 3);
    
    const results: QuickBacktestResult[] = [];
    
    for (const symbolInfo of topSymbols) {
      for (const strategy of strategies.slice(0, 2)) { 
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
            processing_time: processingTime
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

  const getSymbolDistribution = () => {
    if (!dataInfo) return [];
    
    return dataInfo.symbol_info
      .slice(0, 8)
      .map(symbol => ({
        symbol: symbol.symbol,
        records: symbol.records_count,
        percentage: (symbol.records_count / dataInfo.total_records * 100).toFixed(1)
      }));
  };

  const getAvgProcessingTime = () => {
    if (quickResults.length === 0) return 0;
    return quickResults.reduce((sum, r) => sum + r.processing_time, 0) / quickResults.length;
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <Loader2 className="w-8 h-8 animate-spin mr-2" />
        <span>Loading trading system dashboard...</span>
      </div>
    );
  }

  const symbolDistribution = getSymbolDistribution();
  const dataCoverageDays = getDataCoverageDays();

  return (
    <div className="space-y-6">
      {/* Welcome Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold">Trading System Dashboard</h1>
          <p className="text-gray-600 dark:text-gray-400 mt-2">
            High-performance quantitative trading system with advanced caching
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

      {/* System Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
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
              {dataCoverageDays}
            </div>
            <p className="text-xs text-muted-foreground">
              Days of market data
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">Strategies</CardTitle>
            <TrendingUp className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-purple-600">
              {strategies.length}
            </div>
            <p className="text-xs text-muted-foreground">
              Available algorithms
            </p>
          </CardContent>
        </Card>
      </div>

      {/* Data Distribution Chart */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BarChart3 className="w-5 h-5" />
              Data Distribution by Symbol
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="h-80">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={symbolDistribution}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis 
                    dataKey="symbol" 
                    angle={-45}
                    textAnchor="end"
                    height={60}
                  />
                  <YAxis tickFormatter={(value) => `${(value / 1000).toFixed(0)}K`} />
                  <Tooltip 
                    formatter={(value: any) => [value.toLocaleString(), 'Records']}
                    labelFormatter={(label) => `Symbol: ${label}`}
                  />
                  <Bar dataKey="records" fill="#3b82f6" />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </CardContent>
        </Card>

        {/* Available Strategies */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="w-5 h-5" />
              Available Strategies
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              {strategies.map((strategy) => (
                <div
                  key={strategy.id}
                  className="flex items-center justify-between p-3 border rounded-lg"
                >
                  <div>
                    <h4 className="font-medium">{strategy.name}</h4>
                    <p className="text-sm text-gray-500">{strategy.description}</p>
                  </div>
                  <div className="text-xs bg-blue-100 dark:bg-blue-900 px-2 py-1 rounded">
                    {strategy.id.toUpperCase()}
                  </div>
                </div>
              ))}
              <Link href="/backtest">
                <Button variant="outline" className="w-full mt-4">
                  Configure & Run Backtest
                </Button>
              </Link>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Quick Backtest Results */}
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
                <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-4">
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
                </div>

                <div className="overflow-x-auto">
                  <table className="w-full">
                    <thead>
                      <tr className="text-left border-b">
                        <th className="pb-2">Strategy</th>
                        <th className="pb-2">Symbol</th>
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

      {/* Data Quality Overview */}
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
                {symbolDistribution.slice(0, 5).map((symbol, index) => (
                  <div key={symbol.symbol} className="flex items-center justify-between">
                    <span className="flex items-center gap-2">
                      <span className="w-6 h-6 bg-blue-500 text-white text-xs rounded-full flex items-center justify-center">
                        {index + 1}
                      </span>
                      {symbol.symbol}
                    </span>
                    <span className="text-sm text-gray-500">
                      {symbol.records.toLocaleString()} records ({symbol.percentage}%)
                    </span>
                  </div>
                ))}
              </div>
            </div>
            
            <div>
              <h4 className="font-medium mb-3">Data Timeline</h4>
              <div className="space-y-3">
                <div>
                  <p className="text-sm text-gray-500">Earliest Data</p>
                  <p className="font-medium">
                    {dataInfo?.earliest_time ? 
                      new Date(dataInfo.earliest_time).toLocaleDateString() : 'N/A'}
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-500">Latest Data</p>
                  <p className="font-medium">
                    {dataInfo?.latest_time ? 
                      new Date(dataInfo.latest_time).toLocaleDateString() : 'N/A'}
                  </p>
                </div>
                <div>
                  <p className="text-sm text-gray-500">Coverage</p>
                  <p className="font-medium text-green-600">
                    {dataCoverageDays} days of continuous data
                  </p>
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