use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId}; // 引入 criterion 库，用于基准测试
use trading_core::data::types::TickData; // 引入 TickData 结构体
use trading_core::data::cache::MarketDataCache; // 引入 MarketDataCache 结构体
use chrono::Utc; // 引入 chrono 库的 Utc 模块，用于获取 UTC 时间
use uuid::Uuid; // 引入 uuid 库，用于生成 UUID
use std::time::Duration; // 引入 Duration，用于表示时间段

fn create_test_tick(symbol: &str, price: f64, volume: f64) -> TickData { // 定义函数 create_test_tick，用于创建测试用的 TickData
    TickData { // 创建 TickData 实例
        timestamp: Utc::now(), // 时间戳，使用当前 UTC 时间
        symbol: symbol.to_string(), // 交易品种，转换为 String 类型
        price, // 价格
        volume, // 成交量
        side: "buy".to_string(), // 交易方向，默认为 "buy"
        trade_id: Uuid::new_v4().to_string(), // 交易 ID，生成一个新的 UUID v4 并转换为字符串
        is_maker: false, // 是否为 Maker 订单，默认为 false
    }
}

fn bench_single_update(c: &mut Criterion) { // 定义基准测试函数 bench_single_update
    let mut cache = MarketDataCache::new(100); // 创建一个新的 MarketDataCache 实例，容量为 100
    let tick = create_test_tick("BTC/USDT", 50000.0, 1.0); // 创建一个测试用的 TickData

    c.bench_function("single_update", |b| { // 使用 Criterion 注册一个名为 "single_update" 的基准测试函数
        b.iter(|| { // 迭代执行闭包内的代码
            cache.update(black_box(tick.clone())); // 调用 cache 的 update 方法，使用 black_box 防止编译器优化，并克隆 tick 数据
        });
    });
}

fn bench_batch_update(c: &mut Criterion) { // 定义基准测试函数 bench_batch_update
    let mut cache = MarketDataCache::new(100); // 创建一个新的 MarketDataCache 实例，容量为 100
    
    let mut group = c.benchmark_group("batch_update"); // 创建一个名为 "batch_update" 的基准测试组
    group.sample_size(50); // 设置采样大小为 50
    group.measurement_time(Duration::from_secs(10)); // 设置测量时间为 10 秒

    for size in [10, 100, 1000].iter() { // 遍历不同的批量大小 [10, 100, 1000]
        let ticks: Vec<TickData> = (0..*size) // 生成指定数量的 TickData
            .map(|i| create_test_tick( // 对每个索引 i
                &format!("SYMBOL{}/USDT", i % 10), // 创建不同的交易品种名称
                50000.0 + i as f64, // 创建不同的价格
                1.0 // 成交量为 1.0
            ))
            .collect(); // 收集到 Vec<TickData> 中

        group.bench_with_input(BenchmarkId::from_parameter(size), &ticks, |b, ticks| { // 使用输入参数注册基准测试
            b.iter(|| { // 迭代执行闭包内的代码
                cache.batch_update(black_box(ticks.clone())); // 调用 cache 的 batch_update 方法，使用 black_box 防止编译器优化，并克隆 ticks 数据
            });
        });
    }
    group.finish(); // 完成基准测试组
}

fn bench_get_history(c: &mut Criterion) { // 定义基准测试函数 bench_get_history
    let mut cache = MarketDataCache::new(100); // 创建一个新的 MarketDataCache 实例，容量为 100
    let symbol = "BTC/USDT"; // 定义交易品种
    
    // 预填充数据 // 预填充数据
    for i in 0..1000 { // 循环 1000 次
        cache.update(create_test_tick( // 更新缓存
            symbol, // 交易品种
            50000.0 + i as f64, // 不同的价格
            1.0 // 成交量为 1.0
        ));
    }

    let mut group = c.benchmark_group("get_history"); // 创建一个名为 "get_history" 的基准测试组
    group.sample_size(100); // 设置采样大小为 100

    for size in [10, 100, 500].iter() { // 遍历不同的历史数据获取大小 [10, 100, 500]
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| { // 使用输入参数注册基准测试
            b.iter(|| { // 迭代执行闭包内的代码
                black_box(cache.get_history(symbol, size)); // 调用 cache 的 get_history 方法，并使用 black_box 防止编译器优化
            });
        });
    }
    group.finish(); // 完成基准测试组
}

fn bench_concurrent_operations(c: &mut Criterion) { // 定义基准测试函数 bench_concurrent_operations
    use std::thread; // 引入 thread 模块，用于多线程操作
    use std::sync::{Arc, RwLock}; // 引入 Arc 和 RwLock，用于线程安全的数据共享
    
    let cache = Arc::new(RwLock::new(MarketDataCache::new(100))); // 创建一个线程安全的 MarketDataCache 实例
    let symbol = "BTC/USDT"; // 定义交易品种

    let mut group = c.benchmark_group("concurrent_operations"); // 创建一个名为 "concurrent_operations" 的基准测试组
    group.sample_size(50); // 设置采样大小为 50
    group.measurement_time(Duration::from_secs(5)); // 设置测量时间为 5 秒

    group.bench_function("concurrent_read_write", |b| { // 注册一个名为 "concurrent_read_write" 的基准测试函数
        b.iter(|| { // 迭代执行闭包内的代码
            let cache_clone = Arc::clone(&cache); // 克隆 Arc 以便在写线程中使用
            let write_thread = thread::spawn(move || { // 创建一个写线程
                for i in 0..100 { // 循环 100 次
                    if let Ok(mut cache) = cache_clone.write() { // 获取写锁
                        cache.update(create_test_tick( // 更新缓存
                            symbol, // 交易品种
                            50000.0 + i as f64, // 不同的价格
                            1.0 // 成交量为 1.0
                        ));
                    }
                }
            });

            let cache_clone = Arc::clone(&cache); // 克隆 Arc 以便在读线程中使用
            let read_thread = thread::spawn(move || { // 创建一个读线程
                for _ in 0..100 { // 循环 100 次
                    if let Ok(cache) = cache_clone.read() { // 获取读锁
                        black_box(cache.get_history(symbol, 10)); // 读取历史数据，并使用 black_box 防止编译器优化
                    }
                }
            });

            write_thread.join().unwrap(); // 等待写线程完成
            read_thread.join().unwrap(); // 等待读线程完成
        });
    });

    group.finish(); // 完成基准测试组
}

fn bench_market_data_aggregation(c: &mut Criterion) { // 定义基准测试函数 bench_market_data_aggregation
    let mut cache = MarketDataCache::new(100); // 创建一个新的 MarketDataCache 实例，容量为 100
    let symbol = "BTC/USDT"; // 定义交易品种

    c.bench_function("market_data_aggregation", |b| { // 注册一个名为 "market_data_aggregation" 的基准测试函数
        b.iter(|| { // 迭代执行闭包内的代码
            for i in 0..100 { // 循环 100 次
                cache.update(create_test_tick( // 更新缓存
                    symbol, // 交易品种
                    50000.0 + (i % 10) as f64,  // 使用模运算创建有限的价格变动，以测试聚合逻辑
                    1.0 // 成交量为 1.0
                ));
            }
            black_box(cache.get_market_data(symbol)); // 获取聚合后的市场数据，并使用 black_box 防止编译器优化
        });
    });
}

criterion_group!( // 定义基准测试组的宏
    benches, // 组名
    bench_single_update, // 单次更新基准测试
    bench_batch_update, // 批量更新基准测试
    bench_get_history, // 获取历史数据基准测试
    bench_concurrent_operations, // 并发操作基准测试
    bench_market_data_aggregation // 市场数据聚合基准测试
);
criterion_main!(benches); // 定义基准测试的主函数入口