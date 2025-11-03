use crate::contracts::AgentRegistry;
use crate::services::CacheService;
use anyhow::Result;
use ethers::providers::{Http, Provider};
use ethers::types::Address;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ReputationService {
    provider: Arc<Provider<Http>>,
    registry_address: Option<Address>,
    cache: Arc<CacheService>,
    mock_reputations: HashMap<Address, u64>,
}

impl ReputationService {
    pub async fn new(
        provider: Arc<Provider<Http>>,
        cache: Arc<CacheService>,
        registry_address: Option<Address>,
    ) -> Self {
        
        // Mock reputations for testing when no contract deployed
        let mut mock_reputations = HashMap::new();
        mock_reputations.insert(
            "0x1111111111111111111111111111111111111111".parse().unwrap(),
            100  // Low reputation
        );
        mock_reputations.insert(
            "0x2222222222222222222222222222222222222222".parse().unwrap(),
            500  // Medium reputation
        );
        mock_reputations.insert(
            "0x3333333333333333333333333333333333333333".parse().unwrap(),
            1000  // High reputation
        );
        
        tracing::info!(
            "Reputation service initialized (contract mode: {})",
            registry_address.is_some()
        );
        
        Self {
            provider,
            registry_address,
            cache,
            mock_reputations,
        }
    }
    
    pub async fn get_reputation(&self, agent: Address) -> Result<u64> {
        // Check cache first (1 hour TTL)
        let cache_key = format!("reputation:{}", agent);
        if let Some(cached) = self.cache.get::<u64>(&cache_key).await.ok().flatten() {
            tracing::debug!("Reputation cache hit for {}", agent);
            return Ok(cached);
        }
        
        let reputation = if let Some(registry_addr) = self.registry_address {
            // Real contract call
            let registry = AgentRegistry::new(registry_addr, self.provider.clone());
            match registry.get_reputation(agent).call().await {
                Ok(rep) => rep.as_u64(),
                Err(e) => {
                    tracing::warn!("Contract call failed for {}: {}, using default", agent, e);
                    250 // Default reputation on error
                }
            }
        } else {
            // Mock mode
            *self.mock_reputations.get(&agent).unwrap_or(&250)
        };
        
        // Cache for 1 hour
        let _ = self.cache.set(&cache_key, &reputation, 3600).await;
        
        tracing::info!("Agent {} reputation: {}", agent, reputation);
        
        Ok(reputation)
    }
    
    pub fn calculate_price(&self, base_price: f64, reputation: u64) -> f64 {
        match reputation {
            0..=99 => f64::MAX,           // Access denied
            100..=500 => base_price,      // Standard price
            501..=1000 => base_price * 0.8,  // 20% discount
            _ => base_price * 0.5,        // 50% discount for reputation > 1000
        }
    }
    
    pub async fn verify_access(&self, agent: Option<Address>, min_reputation: u64) -> Result<bool> {
        if let Some(addr) = agent {
            let reputation = self.get_reputation(addr).await?;
            Ok(reputation >= min_reputation)
        } else {
            Ok(true) // Allow anonymous access
        }
    }
}
