use crate::{
    error::QGuardError,
    models::{ApiResponse, MEVOpportunity},
    services::{Analytics, EthereumService, MEVDetector, MempoolService},
};
use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct MEVState {
    pub ethereum: Arc<EthereumService>,
    pub mempool: Arc<MempoolService>,
    pub mev_detector: Arc<MEVDetector>,
    pub analytics: Arc<Analytics>,
}

pub async fn get_mev_opportunities(
    State(state): State<MEVState>,
) -> Result<Json<ApiResponse<Vec<MEVOpportunity>>>, QGuardError> {
    // This endpoint costs $0.10 USDC
    // Payment middleware already verified payment
    
    let pending_txs = state.mempool.get_pending_transactions().await;
    let mut opportunities = Vec::new();
    
    // Analyze top 10 pending transactions
    for tx in pending_txs.iter().take(10) {
        if let Some(opp) = state.mev_detector.analyze_transaction(tx).await {
            opportunities.push(opp);
        }
    }
    
    // Record analytics (payment already verified by middleware)
    state.analytics.record_payment(0.10, "/api/mev/opportunities", "unknown").await;
    
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

