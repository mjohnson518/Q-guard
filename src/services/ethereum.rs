use crate::{error::QGuardError, models::GasPrediction, services::CacheService};
use anyhow::Result;
use chrono::Utc;
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    types::Block,
};
use std::sync::Arc;

pub struct EthereumService {
    primary: Arc<Provider<Http>>,
    fallback: Option<Arc<Provider<Http>>>,
    cache: Arc<CacheService>,
}

impl EthereumService {
    pub async fn new(
        rpc_url: &str,
        fallback_url: Option<&str>,
        cache: Arc<CacheService>,
    ) -> Result<Self> {
        let primary = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        let fallback = if let Some(url) = fallback_url {
            Some(Arc::new(Provider::<Http>::try_from(url)?))
        } else {
            None
        };
        
        // Test connection
        let block_number = primary.get_block_number().await?;
        tracing::info!("Ethereum RPC connected, current block: {}", block_number);
        
        Ok(Self {
            primary,
            fallback,
            cache,
        })
    }
    
    pub async fn get_gas_prediction(&self) -> Result<GasPrediction, QGuardError> {
        // Check cache with 12-second TTL (1 block time)
        let cache_key = "gas:prediction";
        if let Some(cached) = self.cache.get(cache_key).await.ok().flatten() {
            tracing::debug!("Returning cached gas prediction");
            return Ok(cached);
        }
        
        // Fetch last 20 blocks
        let latest_block = self.get_block_number().await?;
        let start_block = latest_block.saturating_sub(19);
        
        let blocks = self.fetch_blocks(start_block, latest_block).await?;
        
        if blocks.is_empty() {
            return Err(QGuardError::RpcError(
                ethers::providers::ProviderError::CustomError("No blocks fetched".to_string())
            ));
        }
        
        // Calculate prediction
        let prediction = self.calculate_prediction(&blocks)?;
        
        // Cache for 12 seconds
        self.cache.set(cache_key, &prediction, 12).await
            .map_err(|e| QGuardError::CacheError(e.to_string()))?;
        
        tracing::info!(
            "Gas prediction: base={:.2} gwei, max={:.2} gwei, confidence={:.2}",
            prediction.base_fee_gwei,
            prediction.max_fee_gwei,
            prediction.confidence
        );
        
        Ok(prediction)
    }
    
    fn calculate_prediction(&self, blocks: &[Block<H256>]) -> Result<GasPrediction> {
        let n = blocks.len();
        
        // Generate exponential weights (more recent = higher weight)
        let weights = self.generate_exponential_weights(n);
        
        // Calculate weighted average base fee
        let weighted_base_fee: f64 = blocks
            .iter()
            .zip(weights.iter())
            .filter_map(|(block, weight)| {
                block.base_fee_per_gas.map(|fee| {
                    let fee_gwei = fee.as_u128() as f64 / 1e9;
                    fee_gwei * weight
                })
            })
            .sum();
        
        // Standard priority fee (2 gwei is typical)
        let priority_fee_gwei = 2.0;
        
        // Add 20% buffer for safety
        let max_fee_gwei = weighted_base_fee * 1.2 + priority_fee_gwei;
        
        // Calculate confidence based on variance
        let confidence = self.calculate_confidence(blocks, weighted_base_fee);
        
        Ok(GasPrediction {
            base_fee_gwei: weighted_base_fee,
            priority_fee_gwei,
            max_fee_gwei,
            confidence,
            block_number: blocks.last().unwrap().number.unwrap().as_u64(),
            predicted_at: Utc::now(),
            next_block_time_seconds: 12, // Ethereum block time
        })
    }
    
    fn generate_exponential_weights(&self, n: usize) -> Vec<f64> {
        let decay: f64 = 0.95;
        let weights: Vec<f64> = (0..n)
            .rev() // Reverse so most recent gets highest weight
            .map(|i| decay.powi(i as i32))
            .collect();
        
        let sum: f64 = weights.iter().sum();
        weights.iter().map(|w| w / sum).collect()
    }
    
    fn calculate_confidence(&self, blocks: &[Block<H256>], mean: f64) -> f64 {
        let base_fees: Vec<f64> = blocks
            .iter()
            .filter_map(|b| b.base_fee_per_gas.map(|f| f.as_u128() as f64 / 1e9))
            .collect();
        
        if base_fees.len() < 2 {
            return 0.5;
        }
        
        // Calculate standard deviation
        let variance: f64 = base_fees
            .iter()
            .map(|fee| {
                let diff = fee - mean;
                diff * diff
            })
            .sum::<f64>() / base_fees.len() as f64;
        
        let std_dev = variance.sqrt();
        
        // Lower std_dev = higher confidence
        // Normalize to 0-1 range
        let confidence = 1.0 / (1.0 + std_dev / mean);
        confidence.clamp(0.0, 1.0)
    }
    
    async fn fetch_blocks(&self, start: u64, end: u64) -> Result<Vec<Block<H256>>> {
        let mut blocks = Vec::new();
        
        for block_num in start..=end {
            match self.get_block(block_num).await {
                Ok(Some(block)) => blocks.push(block),
                Ok(None) => tracing::warn!("Block {} not found", block_num),
                Err(e) => {
                    tracing::error!("Error fetching block {}: {}", block_num, e);
                    if blocks.len() >= 10 {
                        // Continue with partial data if we have at least 10 blocks
                        break;
                    }
                }
            }
        }
        
        Ok(blocks)
    }
    
    async fn get_block(&self, block_number: u64) -> Result<Option<Block<H256>>> {
        match self.primary.get_block(block_number).await {
            Ok(block) => Ok(block),
            Err(_) if self.fallback.is_some() => {
                tracing::warn!("Primary RPC failed, trying fallback");
                self.fallback.as_ref().unwrap()
                    .get_block(block_number)
                    .await
                    .map_err(Into::into)
            }
            Err(e) => Err(e.into()),
        }
    }
    
    async fn get_block_number(&self) -> Result<u64> {
        match self.primary.get_block_number().await {
            Ok(num) => Ok(num.as_u64()),
            Err(_) if self.fallback.is_some() => {
                self.fallback.as_ref().unwrap()
                    .get_block_number()
                    .await
                    .map(|n| n.as_u64())
                    .map_err(Into::into)
            }
            Err(e) => Err(e.into()),
        }
    }
}

