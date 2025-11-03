use crate::{
    error::QGuardError,
    models::{ApiResponse, MEVOpportunity},
    services::{Analytics, EthereumService, MEVDetector, MempoolService, ReputationService},
};
use axum::{extract::State, Extension, Json};
use chrono::Utc;
use ethers::types::Address;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct MEVState {
    pub ethereum: Arc<EthereumService>,
    pub mempool: Arc<MempoolService>,
    pub mev_detector: Arc<MEVDetector>,
    pub analytics: Arc<Analytics>,
    pub reputation: Arc<ReputationService>,
}

pub async fn get_mev_opportunities(
    State(state): State<MEVState>,
    agent: Option<Extension<Address>>,
) -> Result<Json<ApiResponse<Vec<MEVOpportunity>>>, QGuardError> {
    // This endpoint costs $0.10 USDC (premium)
    // Payment middleware already verified payment
    
    // Check agent reputation if provided
    let mut actual_price = 0.10;
    
    if let Some(Extension(agent_addr)) = agent {
        let reputation = state.reputation.get_reputation(agent_addr).await
            .map_err(|e| QGuardError::ReputationError(e.to_string()))?;
        
        tracing::info!(
            "Agent {:?} with reputation {} accessing MEV opportunities",
            agent_addr,
            reputation
        );
        
        // Deny access for reputation < 100
        if reputation < 100 {
            return Err(QGuardError::InsufficientReputation {
                current: reputation,
                required: 100,
            });
        }
        
        // Calculate dynamic pricing
        actual_price = state.reputation.calculate_price(0.10, reputation);
        
        state.analytics.record_payment(actual_price, "/api/mev/opportunities", &format!("{:?}", agent_addr)).await;
    } else {
        state.analytics.record_payment(actual_price, "/api/mev/opportunities", "anonymous").await;
    }
    
    let pending_txs = state.mempool.get_pending_transactions().await;
    let mut opportunities = Vec::new();
    
    // Analyze top 10 pending transactions
    for tx in pending_txs.iter().take(10) {
        if let Some(opp) = state.mev_detector.analyze_transaction(tx).await {
            opportunities.push(opp);
        }
    }
    
    tracing::info!("MEV analysis complete: {} opportunities found", opportunities.len());
    
    Ok(Json(ApiResponse {
        success: true,
        data: opportunities,
        timestamp: Utc::now(),
        cache_hit: false,
        data_source: "ethereum-mempool".to_string(),
        request_id: Uuid::new_v4().to_string(),
    }))
}

