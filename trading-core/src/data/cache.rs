use async_trait::async_trait;
use redis::{Client as RedisClient, Commands, Connection};
use serde_json;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, error, warn};

use super::types::{DataError, DataResult, TickData};

// =================================================================
// Cache Interface Definition
// =================================================================

/// TickData cache interface
#[async_trait]
pub trait TickDataCache: Send + Sync {
    /// Add new tick data
    async fn push_tick(&self, tick: &TickData) -> DataResult<()>;

    /// Get recent tick data
    async fn get_recent_ticks(&self, symbol: &str, limit: usize) -> DataResult<Vec<TickData>>;

    /// Get list of cached symbols
    async fn get_symbols(&self) -> DataResult<Vec<String>>;

    /// Clear cache for specific symbol
    async fn clear_symbol(&self, symbol: &str) -> DataResult<()>;

    /// Clear all cache
    async fn clear_all(&self) -> DataResult<()>;
}

// =================================================================
// Memory Cache Implementation
// =================================================================

/// Memory cache entry for ticks
#[derive(Debug, Clone)]
struct MemoryCacheEntry {
    ticks: VecDeque<TickData>,
    last_access: Instant,
    last_update: Instant,
}

impl MemoryCacheEntry {
    fn new() -> Self {
        let now = Instant::now();
        Self {
            ticks: VecDeque::new(),
            last_access: now,
            last_update: now,
        }
    }

    fn push_tick(&mut self, tick: TickData, max_size: usize) {
        self.ticks.push_back(tick);
        self.last_update = Instant::now();

        // Maintain size limit
        while self.ticks.len() > max_size {
            self.ticks.pop_front();
        }
    }

    fn get_recent(&mut self, limit: usize) -> Vec<TickData> {
        self.last_access = Instant::now();

        self.ticks
            .iter()
            .rev() // Latest first
            .take(limit)
            .cloned()
            .collect()
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.last_access.elapsed() > ttl
    }
}

/// In-memory tick cache implementation
pub struct InMemoryTickCache {
    data: Arc<RwLock<HashMap<String, MemoryCacheEntry>>>,
    max_ticks_per_symbol: usize,
    ttl: Duration,
}

impl InMemoryTickCache {
    pub fn new(max_ticks_per_symbol: usize, ttl_seconds: u64) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            max_ticks_per_symbol,
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Clean up expired cache entries
    pub fn cleanup_expired(&self) {
        if let Ok(mut data) = self.data.write() {
            let expired_symbols: Vec<String> = data
                .iter()
                .filter(|(_, entry)| entry.is_expired(self.ttl))
                .map(|(symbol, _)| symbol.clone())
                .collect();

            for symbol in expired_symbols {
                data.remove(&symbol);
                debug!("Cleaned up expired cache for symbol: {}", symbol);
            }
        }
    }
}

#[async_trait]
impl TickDataCache for InMemoryTickCache {
    async fn push_tick(&self, tick: &TickData) -> DataResult<()> {
        match self.data.write() {
            Ok(mut data) => {
                let entry = data
                    .entry(tick.symbol.clone())
                    .or_insert_with(MemoryCacheEntry::new);

                entry.push_tick(tick.clone(), self.max_ticks_per_symbol);
                debug!(
                    "Added tick to memory cache: symbol={}, price={}",
                    tick.symbol, tick.price
                );
                Ok(())
            }
            Err(e) => {
                error!("Failed to acquire write lock for memory cache: {}", e);
                Err(DataError::Cache(format!("Lock error: {}", e)))
            }
        }
    }

    async fn get_recent_ticks(&self, symbol: &str, limit: usize) -> DataResult<Vec<TickData>> {
        match self.data.write() {
            Ok(mut data) => {
                if let Some(entry) = data.get_mut(symbol) {
                    let ticks = entry.get_recent(limit);
                    debug!(
                        "Retrieved {} ticks from memory cache for symbol: {}",
                        ticks.len(),
                        symbol
                    );
                    Ok(ticks)
                } else {
                    debug!("No memory cache found for symbol: {}", symbol);
                    Ok(Vec::new())
                }
            }
            Err(e) => {
                error!("Failed to acquire write lock for memory cache: {}", e);
                Err(DataError::Cache(format!("Lock error: {}", e)))
            }
        }
    }

    async fn get_symbols(&self) -> DataResult<Vec<String>> {
        match self.data.read() {
            Ok(data) => Ok(data.keys().cloned().collect()),
            Err(e) => Err(DataError::Cache(format!("Lock error: {}", e))),
        }
    }

    async fn clear_symbol(&self, symbol: &str) -> DataResult<()> {
        match self.data.write() {
            Ok(mut data) => {
                data.remove(symbol);
                debug!("Cleared memory cache for symbol: {}", symbol);
                Ok(())
            }
            Err(e) => Err(DataError::Cache(format!("Lock error: {}", e))),
        }
    }

    async fn clear_all(&self) -> DataResult<()> {
        match self.data.write() {
            Ok(mut data) => {
                data.clear();
                debug!("Cleared all memory cache");
                Ok(())
            }
            Err(e) => Err(DataError::Cache(format!("Lock error: {}", e))),
        }
    }
}

// =================================================================
// Redis Cache Implementation
// =================================================================

/// Redis cache implementation
pub struct RedisTickCache {
    client: RedisClient,
    connection: Arc<Mutex<Connection>>,
    max_ticks_per_symbol: usize,
    ttl_seconds: u64,
}

impl RedisTickCache {
    pub async fn new(
        redis_url: &str,
        max_ticks_per_symbol: usize,
        ttl_seconds: u64,
    ) -> DataResult<Self> {
        let client = RedisClient::open(redis_url)
            .map_err(|e| DataError::Cache(format!("Failed to create Redis client: {}", e)))?;

        let connection = client
            .get_connection()
            .map_err(|e| DataError::Cache(format!("Failed to connect to Redis: {}", e)))?;

        debug!("Connected to Redis at: {}", redis_url);

        Ok(Self {
            client,
            connection: Arc::new(Mutex::new(connection)),
            max_ticks_per_symbol,
            ttl_seconds,
        })
    }

    fn get_cache_key(&self, symbol: &str) -> String {
        format!("tick:{}", symbol)
    }
}

#[async_trait]
impl TickDataCache for RedisTickCache {
    async fn push_tick(&self, tick: &TickData) -> DataResult<()> {
        let key = self.get_cache_key(&tick.symbol);
        let tick_json = serde_json::to_string(tick)
            .map_err(|e| DataError::Cache(format!("Failed to serialize tick: {}", e)))?;

        let mut conn = self.connection.lock().await;

        // Use LPUSH to add to list head (latest first)
        let _: () = conn
            .lpush(&key, &tick_json)
            .map_err(|e| DataError::Cache(format!("Redis LPUSH failed: {}", e)))?;

        // Limit list length
        let _: () = conn
            .ltrim(&key, 0, self.max_ticks_per_symbol as isize - 1)
            .map_err(|e| DataError::Cache(format!("Redis LTRIM failed: {}", e)))?;

        // Set TTL
        let _: () = conn
            .expire(&key, self.ttl_seconds as usize)
            .map_err(|e| DataError::Cache(format!("Redis EXPIRE failed: {}", e)))?;

        debug!(
            "Added tick to Redis cache: symbol={}, price={}",
            tick.symbol, tick.price
        );
        Ok(())
    }

    async fn get_recent_ticks(&self, symbol: &str, limit: usize) -> DataResult<Vec<TickData>> {
        let key = self.get_cache_key(symbol);
        let mut conn = self.connection.lock().await;

        // Use LRANGE to get latest N records
        let tick_jsons: Vec<String> = conn
            .lrange(&key, 0, limit as isize - 1)
            .map_err(|e| DataError::Cache(format!("Redis LRANGE failed: {}", e)))?;

        let mut ticks = Vec::with_capacity(tick_jsons.len());
        for tick_json in tick_jsons {
            match serde_json::from_str::<TickData>(&*tick_json) {
                Ok(tick) => ticks.push(tick),
                Err(e) => {
                    warn!("Failed to deserialize tick from Redis: {}", e);
                    // Continue processing other data, don't interrupt on single error
                }
            }
        }

        debug!(
            "Retrieved {} ticks from Redis cache for symbol: {}",
            ticks.len(),
            symbol
        );
        Ok(ticks)
    }

    async fn get_symbols(&self) -> DataResult<Vec<String>> {
        let mut conn = self.connection.lock().await;

        let keys: Vec<String> = conn
            .keys("tick:*")
            .map_err(|e| DataError::Cache(format!("Redis KEYS failed: {}", e)))?;

        let symbols: Vec<String> = keys
            .into_iter()
            .filter_map(|key| {
                if key.starts_with("tick:") {
                    Some(key[5..].to_string()) // Remove "tick:" prefix
                } else {
                    None
                }
            })
            .collect();

        Ok(symbols)
    }

    async fn clear_symbol(&self, symbol: &str) -> DataResult<()> {
        let key = self.get_cache_key(symbol);
        let mut conn = self.connection.lock().await;

        let _: () = conn
            .del(&key)
            .map_err(|e| DataError::Cache(format!("Redis DEL failed: {}", e)))?;

        debug!("Cleared Redis cache for symbol: {}", symbol);
        Ok(())
    }

    async fn clear_all(&self) -> DataResult<()> {
        let symbols = self.get_symbols().await?;

        for symbol in symbols {
            self.clear_symbol(&symbol).await?;
        }

        debug!("Cleared all Redis cache");
        Ok(())
    }
}

// =================================================================
// Tiered Cache Implementation
// =================================================================

/// Tiered cache: L1 memory + L2 Redis
pub struct TieredCache {
    memory_cache: InMemoryTickCache,
    redis_cache: RedisTickCache,
}

impl TieredCache {
    pub async fn new(
        memory_config: (usize, u64),      // (max_ticks_per_symbol, ttl_seconds)
        redis_config: (&str, usize, u64), // (url, max_ticks_per_symbol, ttl_seconds)
    ) -> DataResult<Self> {
        let memory_cache = InMemoryTickCache::new(memory_config.0, memory_config.1);
        let redis_cache =
            RedisTickCache::new(redis_config.0, redis_config.1, redis_config.2).await?;

        debug!("Initialized tiered cache with memory and Redis layers");

        Ok(Self {
            memory_cache,
            redis_cache,
        })
    }

    /// Periodically clean expired items from memory cache
    pub fn cleanup_memory(&self) {
        self.memory_cache.cleanup_expired();
    }
}

#[async_trait]
impl TickDataCache for TieredCache {
    async fn push_tick(&self, tick: &TickData) -> DataResult<()> {
        // Write to both memory and Redis in parallel
        let memory_result = self.memory_cache.push_tick(tick);
        let redis_result = self.redis_cache.push_tick(tick);

        // Wait for both operations to complete
        let (memory_res, redis_res) = tokio::join!(memory_result, redis_result);

        // If memory cache fails, log error but don't interrupt
        if let Err(e) = memory_res {
            error!("Memory cache push failed: {}", e);
        }

        // Redis failure returns error
        redis_res?;

        Ok(())
    }

    async fn get_recent_ticks(&self, symbol: &str, limit: usize) -> DataResult<Vec<TickData>> {
        // 1. Try memory cache first
        let memory_ticks = self.memory_cache.get_recent_ticks(symbol, limit).await?;
        if memory_ticks.len() == limit {
            debug!("L1 cache hit for symbol: {}", symbol);
            return Ok(memory_ticks);
        }

        // 2. L1 miss, try Redis
        let redis_ticks = self.redis_cache.get_recent_ticks(symbol, limit).await?;
        if !redis_ticks.is_empty() {
            debug!("L2 cache hit for symbol: {}", symbol);

            // Backfill to memory cache
            for tick in &redis_ticks {
                if let Err(e) = self.memory_cache.push_tick(tick).await {
                    warn!("Failed to backfill memory cache: {}", e);
                }
            }

            return Ok(redis_ticks.into_iter().take(limit).collect());
        }

        // 3. Complete cache miss
        debug!("Cache miss for symbol: {}", symbol);
        Ok(Vec::new())
    }

    async fn get_symbols(&self) -> DataResult<Vec<String>> {
        // Merge symbols from memory and Redis
        let memory_symbols = self.memory_cache.get_symbols().await?;
        let redis_symbols = self.redis_cache.get_symbols().await?;

        let mut all_symbols = memory_symbols;
        for symbol in redis_symbols {
            if !all_symbols.contains(&symbol) {
                all_symbols.push(symbol);
            }
        }

        Ok(all_symbols)
    }

    async fn clear_symbol(&self, symbol: &str) -> DataResult<()> {
        // Clear both memory and Redis in parallel
        let memory_result = self.memory_cache.clear_symbol(symbol);
        let redis_result = self.redis_cache.clear_symbol(symbol);

        let (memory_res, redis_res) = tokio::join!(memory_result, redis_result);

        // Both must succeed
        memory_res?;
        redis_res?;

        Ok(())
    }

    async fn clear_all(&self) -> DataResult<()> {
        // Clear both memory and Redis in parallel
        let memory_result = self.memory_cache.clear_all();
        let redis_result = self.redis_cache.clear_all();

        let (memory_res, redis_res) = tokio::join!(memory_result, redis_result);

        memory_res?;
        redis_res?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::types::TradeSide;
    use chrono::Utc;
    use rust_decimal::Decimal;

    fn create_test_tick(symbol: &str, price: &str, trade_id: &str) -> TickData {
        TickData::new(
            Utc::now(),
            symbol.to_string(),
            price.parse::<Decimal>().unwrap(),
            "1.0".parse::<Decimal>().unwrap(),
            TradeSide::Buy,
            trade_id.to_string(),
            false,
        )
    }

    #[tokio::test]
    async fn test_memory_cache() {
        let cache = InMemoryTickCache::new(100, 300);

        let tick = create_test_tick("BTCUSDT", "50000.0", "test1");

        // Test adding
        cache.push_tick(&tick).await.unwrap();

        // Test getting
        let ticks = cache.get_recent_ticks("BTCUSDT", 1).await.unwrap();
        assert_eq!(ticks.len(), 1);
        assert_eq!(ticks[0].symbol, "BTCUSDT");
        assert_eq!(ticks[0].price, "50000.0".parse::<Decimal>().unwrap());
    }

    #[tokio::test]
    async fn test_memory_cache_size_limit() {
        let cache = InMemoryTickCache::new(2, 300); // Max 2 items

        // Add 3 items
        for i in 1..=3 {
            let price = format!("{}.0", 50000 + i);
            let tick = create_test_tick("BTCUSDT", &price, &format!("test{}", i));
            cache.push_tick(&tick).await.unwrap();
        }

        // Should only keep the latest 2
        let ticks = cache.get_recent_ticks("BTCUSDT", 10).await.unwrap();
        assert_eq!(ticks.len(), 2);

        // Latest should be first
        assert_eq!(ticks[0].price, "50003.0".parse::<Decimal>().unwrap()); // Latest
        assert_eq!(ticks[1].price, "50002.0".parse::<Decimal>().unwrap()); // Second latest
    }
}
