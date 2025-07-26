use std::time::Duration;

use redis::{Client, AsyncCommands};
use moka::future::Cache;
use serde::{Serialize, Deserialize};
use tracing::{info, warn};
use crate::error::AppError;
use bigdecimal::BigDecimal;

/// Cache configuration for different data types
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub ttl: Duration,
    pub max_capacity: u64,
    pub redis_enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(300), // 5 minutes
            max_capacity: 10000,
            redis_enabled: false,
        }
    }
}

impl CacheConfig {
    /// Configuration for price data (short TTL, high volume)
    pub fn price_data() -> Self {
        Self {
            ttl: Duration::from_secs(60), // 1 minute for price data
            max_capacity: 50000,
            redis_enabled: true,
        }
    }

    /// Configuration for pool state data (medium TTL)
    pub fn pool_state() -> Self {
        Self {
            ttl: Duration::from_secs(300), // 5 minutes for pool state
            max_capacity: 10000,
            redis_enabled: true,
        }
    }

    /// Configuration for risk calculations (longer TTL)
    pub fn risk_calculations() -> Self {
        Self {
            ttl: Duration::from_secs(600), // 10 minutes for risk calculations
            max_capacity: 5000,
            redis_enabled: true,
        }
    }

    /// Configuration for blockchain RPC responses (very short TTL)
    pub fn blockchain_rpc() -> Self {
        Self {
            ttl: Duration::from_secs(30), // 30 seconds for RPC responses
            max_capacity: 20000,
            redis_enabled: true,
        }
    }
}

/// Cached price data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPrice {
    pub token_address: String,
    pub chain_id: i32,
    pub price_usd: BigDecimal,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Cached pool state structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPoolState {
    pub pool_address: String,
    pub chain_id: i32,
    pub current_tick: i32,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub tvl_usd: Option<BigDecimal>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Multi-layer cache with in-memory (L1) and Redis (L2) support
pub struct MultiLayerCache<T> {
    l1_cache: Cache<String, T>,
    redis_client: Option<Client>,
    config: CacheConfig,
    cache_name: String,
}

impl<T> MultiLayerCache<T>
where
    T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(cache_name: &str, config: CacheConfig, redis_url: Option<&str>) -> Result<Self, AppError> {
        // Initialize L1 (in-memory) cache
        let l1_cache = Cache::builder()
            .max_capacity(config.max_capacity)
            .time_to_live(config.ttl)
            .build();

        // Initialize L2 (Redis) cache if enabled
        let redis_client = if config.redis_enabled {
            if let Some(url) = redis_url {
                match Client::open(url) {
                    Ok(client) => {
                        info!("Redis cache initialized for {}", cache_name);
                        Some(client)
                    }
                    Err(e) => {
                        warn!("Failed to initialize Redis for {}: {}. Falling back to in-memory only.", cache_name, e);
                        None
                    }
                }
            } else {
                warn!("Redis enabled but no URL provided for {}. Using in-memory only.", cache_name);
                None
            }
        } else {
            None
        };

        info!("Multi-layer cache '{}' initialized (L1: {}, L2: {})", 
              cache_name, 
              "in-memory", 
              if redis_client.is_some() { "Redis" } else { "disabled" });

        Ok(Self {
            l1_cache,
            redis_client,
            config,
            cache_name: cache_name.to_string(),
        })
    }

    /// Get value from cache (checks L1 first, then L2)
    pub async fn get(&self, key: &str) -> Result<Option<T>, AppError> {
        // Check L1 cache first
        if let Some(value) = self.l1_cache.get(key).await {
            return Ok(Some(value));
        }

        // Check L2 cache (Redis) if available
        if let Some(redis_client) = &self.redis_client {
            match self.get_from_redis(redis_client, key).await {
                Ok(Some(value)) => {
                    // Store in L1 cache for faster future access
                    self.l1_cache.insert(key.to_string(), value.clone()).await;
                    return Ok(Some(value));
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Redis get error for key '{}' in cache '{}': {}", key, self.cache_name, e);
                }
            }
        }

        Ok(None)
    }

    /// Set value in cache (stores in both L1 and L2)
    pub async fn set(&self, key: &str, value: T) -> Result<(), AppError> {
        // Store in L1 cache
        self.l1_cache.insert(key.to_string(), value.clone()).await;

        // Store in L2 cache (Redis) if available
        if let Some(redis_client) = &self.redis_client {
            if let Err(e) = self.set_in_redis(redis_client, key, &value).await {
                warn!("Redis set error for key '{}' in cache '{}': {}", key, self.cache_name, e);
                // Continue execution - L1 cache is still working
            }
        }

        Ok(())
    }

    /// Remove value from cache (removes from both L1 and L2)
    pub async fn remove(&self, key: &str) -> Result<(), AppError> {
        // Remove from L1 cache
        self.l1_cache.remove(key).await;

        // Remove from L2 cache (Redis) if available
        if let Some(redis_client) = &self.redis_client {
            if let Err(e) = self.remove_from_redis(redis_client, key).await {
                warn!("Redis remove error for key '{}' in cache '{}': {}", key, self.cache_name, e);
            }
        }

        Ok(())
    }

    /// Clear entire cache (both L1 and L2)
    pub async fn clear(&self) -> Result<(), AppError> {
        // Clear L1 cache
        self.l1_cache.invalidate_all();

        // Clear L2 cache (Redis) if available - only keys with our prefix
        if let Some(redis_client) = &self.redis_client {
            if let Err(e) = self.clear_redis_prefix(redis_client).await {
                warn!("Redis clear error for cache '{}': {}", self.cache_name, e);
            }
        }

        info!("Cache '{}' cleared", self.cache_name);
        Ok(())
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let l1_entry_count = self.l1_cache.entry_count();
        let l1_weighted_size = self.l1_cache.weighted_size();

        CacheStats {
            cache_name: self.cache_name.clone(),
            l1_entries: l1_entry_count,
            l1_size: l1_weighted_size,
            l2_available: self.redis_client.is_some(),
            ttl_seconds: self.config.ttl.as_secs(),
            max_capacity: self.config.max_capacity,
        }
    }

    // Private helper methods for Redis operations
    async fn get_from_redis(&self, client: &Client, key: &str) -> Result<Option<T>, AppError> {
        let mut conn = client.get_async_connection().await
            .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;

        let redis_key = format!("{}:{}", self.cache_name, key);
        let data: Option<String> = conn.get(&redis_key).await
            .map_err(|e| AppError::InternalError(format!("Redis get error: {}", e)))?;

        if let Some(json_data) = data {
            let value: T = serde_json::from_str(&json_data)
                .map_err(|e| AppError::InternalError(format!("Redis deserialization error: {}", e)))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    async fn set_in_redis(&self, client: &Client, key: &str, value: &T) -> Result<(), AppError> {
        let mut conn = client.get_async_connection().await
            .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;

        let redis_key = format!("{}:{}", self.cache_name, key);
        let json_data = serde_json::to_string(value)
            .map_err(|e| AppError::InternalError(format!("Redis serialization error: {}", e)))?;

        let _: () = conn.set_ex(&redis_key, json_data, self.config.ttl.as_secs()).await
            .map_err(|e| AppError::InternalError(format!("Redis set error: {}", e)))?;

        Ok(())
    }

    async fn remove_from_redis(&self, client: &Client, key: &str) -> Result<(), AppError> {
        let mut conn = client.get_async_connection().await
            .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;

        let redis_key = format!("{}:{}", self.cache_name, key);
        let _: () = conn.del(&redis_key).await
            .map_err(|e| AppError::InternalError(format!("Redis delete error: {}", e)))?;

        Ok(())
    }

    async fn clear_redis_prefix(&self, client: &Client, ) -> Result<(), AppError> {
        let mut conn = client.get_async_connection().await
            .map_err(|e| AppError::InternalError(format!("Redis connection error: {}", e)))?;

        let pattern = format!("{}:*", self.cache_name);
        let keys: Vec<String> = conn.keys(&pattern).await
            .map_err(|e| AppError::InternalError(format!("Redis keys error: {}", e)))?;

        if !keys.is_empty() {
            let _: () = conn.del(&keys).await
                .map_err(|e| AppError::InternalError(format!("Redis delete error: {}", e)))?;
        }

        Ok(())
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Serialize)]
pub struct CacheStats {
    pub cache_name: String,
    pub l1_entries: u64,
    pub l1_size: u64,
    pub l2_available: bool,
    pub ttl_seconds: u64,
    pub max_capacity: u64,
}

/// Cache manager for all application caches
pub struct CacheManager {
    pub price_cache: MultiLayerCache<CachedPrice>,
    pub pool_state_cache: MultiLayerCache<CachedPoolState>,
    pub risk_cache: MultiLayerCache<String>, // JSON serialized risk calculations
    pub rpc_cache: MultiLayerCache<String>,  // JSON serialized RPC responses
}

impl CacheManager {
    pub async fn new(redis_url: Option<&str>) -> Result<Self, AppError> {
        info!("Initializing cache manager with Redis: {}", redis_url.is_some());

        let price_cache = MultiLayerCache::new(
            "prices",
            CacheConfig::price_data(),
            redis_url,
        )?;

        let pool_state_cache = MultiLayerCache::new(
            "pool_states",
            CacheConfig::pool_state(),
            redis_url,
        )?;

        let risk_cache = MultiLayerCache::new(
            "risk_calculations",
            CacheConfig::risk_calculations(),
            redis_url,
        )?;

        let rpc_cache = MultiLayerCache::new(
            "rpc_responses",
            CacheConfig::blockchain_rpc(),
            redis_url,
        )?;

        info!("Cache manager initialized successfully");

        Ok(Self {
            price_cache,
            pool_state_cache,
            risk_cache,
            rpc_cache,
        })
    }

    /// Get comprehensive cache statistics
    pub async fn get_all_stats(&self) -> Vec<CacheStats> {
        vec![
            self.price_cache.stats().await,
            self.pool_state_cache.stats().await,
            self.risk_cache.stats().await,
            self.rpc_cache.stats().await,
        ]
    }

    /// Clear all caches
    pub async fn clear_all(&self) -> Result<(), AppError> {
        self.price_cache.clear().await?;
        self.pool_state_cache.clear().await?;
        self.risk_cache.clear().await?;
        self.rpc_cache.clear().await?;
        
        info!("All caches cleared");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = MultiLayerCache::<String>::new(
            "test_cache",
            CacheConfig::default(),
            None,
        );
        assert!(cache.is_ok());
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let cache = MultiLayerCache::<String>::new(
            "test_cache",
            CacheConfig::default(),
            None,
        ).unwrap();

        // Test set and get
        cache.set("test_key", "test_value".to_string()).await.unwrap();
        let value = cache.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test remove
        cache.remove("test_key").await.unwrap();
        let value = cache.get("test_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_manager_creation() {
        let manager = CacheManager::new(None).await;
        assert!(manager.is_ok());
    }
}
