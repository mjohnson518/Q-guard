use anyhow::Result;
use moka::future::Cache;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;

pub struct CacheService {
    redis: Option<redis::aio::ConnectionManager>,
    memory: Arc<Cache<String, String>>,
}

impl CacheService {
    pub async fn new(redis_url: &str) -> Result<Self> {
        let redis = match redis::Client::open(redis_url) {
            Ok(client) => {
                match client.get_connection_manager().await {
                    Ok(conn) => {
                        tracing::info!("Redis connected successfully");
                        Some(conn)
                    }
                    Err(e) => {
                        tracing::warn!("Redis connection failed: {}, using memory cache only", e);
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Redis client creation failed: {}, using memory cache only", e);
                None
            }
        };
        
        let memory = Arc::new(
            Cache::builder()
                .max_capacity(1000)
                .time_to_live(Duration::from_secs(60))
                .build()
        );
        
        Ok(Self { redis, memory })
    }
    
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        // Try memory cache first
        if let Some(cached) = self.memory.get(key).await {
            if let Ok(value) = serde_json::from_str(&cached) {
                tracing::debug!("Memory cache hit for key: {}", key);
                return Ok(Some(value));
            }
        }
        
        // Try Redis if available
        if let Some(mut redis) = self.redis.clone() {
            match redis.get::<_, Option<String>>(key).await {
                Ok(Some(cached)) => {
                    if let Ok(value) = serde_json::from_str(&cached) {
                        // Update memory cache
                        self.memory.insert(key.to_string(), cached).await;
                        tracing::debug!("Redis cache hit for key: {}", key);
                        return Ok(Some(value));
                    }
                }
                Ok(None) => {}
                Err(e) => tracing::warn!("Redis get error: {}", e),
            }
        }
        
        tracing::debug!("Cache miss for key: {}", key);
        Ok(None)
    }
    
    pub async fn set<T: Serialize>(&self, key: &str, value: &T, ttl_secs: u64) -> Result<()> {
        let serialized = serde_json::to_string(value)?;
        
        // Store in memory cache
        self.memory.insert(key.to_string(), serialized.clone()).await;
        
        // Store in Redis if available
        if let Some(mut redis) = self.redis.clone() {
            if let Err(e) = redis.set_ex::<_, _, ()>(key, serialized, ttl_secs).await {
                tracing::warn!("Redis set error: {}", e);
            } else {
                tracing::debug!("Cached key: {} with TTL: {}s", key, ttl_secs);
            }
        }
        
        Ok(())
    }
    
    pub async fn increment(&self, key: &str, delta: i64) -> Result<i64> {
        if let Some(mut redis) = self.redis.clone() {
            match redis.incr(key, delta).await {
                Ok(value) => Ok(value),
                Err(e) => {
                    tracing::warn!("Redis increment error: {}", e);
                    Ok(delta) // Fallback to delta
                }
            }
        } else {
            Ok(delta)
        }
    }
    
    pub async fn ping(&self) -> Result<bool> {
        if let Some(mut redis) = self.redis.clone() {
            match redis::cmd("PING").query_async::<_, String>(&mut redis).await {
                Ok(_) => Ok(true),
                Err(_) => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
}

