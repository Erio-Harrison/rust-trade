use chrono::{DateTime, Duration, Utc};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dotenv::dotenv;
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::str::FromStr;
use tokio::runtime::Runtime;
use trading_core::data::{
    cache::{TickDataCache, TieredCache},
    repository::TickDataRepository,
    types::{DataResult, TickData, TickQuery, TradeSide},
};

fn create_test_tick(
    symbol: &str,
    price: &str,
    trade_id: &str,
    timestamp: DateTime<Utc>,
) -> TickData {
    TickData::new(
        timestamp,
        symbol.to_string(),
        Decimal::from_str(price).unwrap(),
        Decimal::from_str("1.0").unwrap(),
        TradeSide::Buy,
        trade_id.to_string(),
        false,
    )
}

async fn setup_repository() -> DataResult<TickDataRepository> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    let pool = PgPool::connect(&database_url)
        .await
        .map_err(|e| trading_core::data::types::DataError::Database(e))?;
    let cache = TieredCache::new((1000, 300), (&redis_url, 10000, 3600)).await?;
    Ok(TickDataRepository::new(pool, cache))
}

async fn setup_cache() -> DataResult<TieredCache> {
    dotenv().ok();
    let redis_url = std::env::var("REDIS_URL").expect("REDIS_URL must be set");
    TieredCache::new((1000, 300), (&redis_url, 10000, 3600)).await
}

async fn cleanup_database(pool: &PgPool, symbol: &str) -> DataResult<()> {
    sqlx::query!("DELETE FROM tick_data WHERE symbol = $1", symbol)
        .execute(pool)
        .await
        .map_err(|e| trading_core::data::types::DataError::Database(e))?;
    Ok(())
}

fn repository_benchmarks(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let symbol = "BTCUSDT_BENCH";

    // Setup repository and cache
    let repo = rt
        .block_on(setup_repository())
        .expect("Failed to setup repository");
    let cache = rt.block_on(setup_cache()).expect("Failed to setup cache");
    let pool = repo.get_pool();
    rt.block_on(cleanup_database(pool, symbol))
        .expect("Failed to cleanup database");

    // Benchmark: Single tick insertion
    c.bench_function("insert_tick", |b| {
        b.to_async(&rt).iter(|| async {
            let tick = create_test_tick(symbol, "50000.0", "bench_single", Utc::now());
            repo.insert_tick(black_box(&tick)).await.unwrap();
        });
    });

    // Benchmark: Batch insertion (100 ticks)
    c.bench_function("batch_insert_100", |b| {
        b.to_async(&rt).iter(|| async {
            let ticks: Vec<TickData> = (0..100)
                .map(|i| {
                    create_test_tick(symbol, "50000.0", &format!("bench_batch_{}", i), Utc::now())
                })
                .collect();
            repo.batch_insert(black_box(ticks)).await.unwrap();
        });
    });

    // Benchmark: Batch insertion (1000 ticks)
    c.bench_function("batch_insert_1000", |b| {
        b.to_async(&rt).iter(|| async {
            let ticks: Vec<TickData> = (0..1000)
                .map(|i| {
                    create_test_tick(symbol, "50000.0", &format!("bench_batch_{}", i), Utc::now())
                })
                .collect();
            repo.batch_insert(black_box(ticks)).await.unwrap();
        });
    });

    // Pre-populate data for query benchmarks
    rt.block_on(async {
        let ticks: Vec<TickData> = (0..1000)
            .map(|i| {
                create_test_tick(
                    symbol,
                    "50000.0",
                    &format!("bench_query_{}", i),
                    Utc::now() - Duration::seconds(1000 - i as i64),
                )
            })
            .collect();
        repo.batch_insert(ticks).await.unwrap();
    });

    // Benchmark: Query ticks (cache hit)
    c.bench_function("get_ticks_cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let query = TickQuery {
                symbol: symbol.to_string(),
                limit: Some(100),
                start_time: Some(Utc::now() - Duration::minutes(30)),
                end_time: None,
                trade_side: None,
            };
            repo.get_ticks(black_box(&query)).await.unwrap();
        });
    });

    // Benchmark: Query ticks (cache miss)
    c.bench_function("get_ticks_cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let query = TickQuery {
                symbol: symbol.to_string(),
                limit: Some(100),
                start_time: Some(Utc::now() - Duration::days(1)),
                end_time: None,
                trade_side: None,
            };
            repo.get_ticks(black_box(&query)).await.unwrap();
        });
    });

    // Benchmark: Latest price
    c.bench_function("get_latest_price", |b| {
        b.to_async(&rt).iter(|| async {
            repo.get_latest_price(black_box(symbol)).await.unwrap();
        });
    });

    // Benchmark: Historical data for backtest
    c.bench_function("get_historical_data_for_backtest", |b| {
        b.to_async(&rt).iter(|| async {
            let start_time = Utc::now() - Duration::hours(1);
            let end_time = Utc::now();
            repo.get_historical_data_for_backtest(
                black_box(symbol),
                start_time,
                end_time,
                Some(100),
            )
            .await
            .unwrap();
        });
    });

    // Benchmark: Cache push_tick
    c.bench_function("cache_push_tick", |b| {
        b.to_async(&rt).iter(|| async {
            let tick = create_test_tick(symbol, "50000.0", "cache_bench", Utc::now());
            cache.push_tick(black_box(&tick)).await.unwrap();
        });
    });

    // Benchmark: Cache get_recent_ticks
    c.bench_function("cache_get_recent_ticks", |b| {
        b.to_async(&rt).iter(|| async {
            cache
                .get_recent_ticks(black_box(symbol), 100)
                .await
                .unwrap();
        });
    });

    // Cleanup after benchmarks
    rt.block_on(cleanup_database(pool, symbol))
        .expect("Failed to cleanup database");
    rt.block_on(cache.clear_symbol(symbol))
        .expect("Failed to clear cache");
}

criterion_group!(benches, repository_benchmarks);
criterion_main!(benches);
