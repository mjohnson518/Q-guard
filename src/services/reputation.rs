use anyhow::Result;
use async_trait::async_trait;
use ethers::types::Address;
use std::collections::HashMap;

#[async_trait]
pub trait ReputationService: Send + Sync {
    async fn get_reputation(&self, address: Address) -> Result<u64>;
    async fn verify_agent(&self, address: Address) -> Result<bool>;
    async fn calculate_price_multiplier(&self, address: Address, base_price: f64) -> Result<f64>;
}

pub struct MockReputationService {
    reputations: HashMap<Address, u64>,
}

impl MockReputationService {
    pub fn new() -> Self {
        let mut reputations = HashMap::new();
        
        // Add some test addresses with different reputation levels
        // These can be used during testing
        if let Ok(addr) = "0x1234567890123456789012345678901234567890".parse() {
            reputations.insert(addr, 500);
        }
        if let Ok(addr) = "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd".parse() {
            reputations.insert(addr, 1000);
        }
        
        Self { reputations }
    }
}

impl Default for MockReputationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReputationService for MockReputationService {
    async fn get_reputation(&self, address: Address) -> Result<u64> {
        // Default reputation of 100 for unknown addresses (minimum acceptable)
        Ok(self.reputations.get(&address).copied().unwrap_or(100))
    }
    
    async fn verify_agent(&self, address: Address) -> Result<bool> {
        let reputation = self.get_reputation(address).await?;
        Ok(reputation >= 100)
    }
    
    async fn calculate_price_multiplier(&self, address: Address, base_price: f64) -> Result<f64> {
        let reputation = self.get_reputation(address).await?;
        
        let multiplier = match reputation {
            0..=99 => return Ok(f64::MAX), // Effectively deny access
            100..=500 => 1.0,               // Full price
            501..=1000 => 0.8,              // 20% discount
            _ => 0.5,                       // 50% discount for reputation > 1000
        };
        
        Ok(base_price * multiplier)
    }
}

