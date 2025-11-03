use crate::{
    error::QGuardError,
    models::{ApiResponse, GasPrediction},
    services::{Analytics, EthereumService, ReputationService},
};
use axum::{extract::State, Extension, Json};
use chrono::Utc;
use ethers::types::Address;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub ethereum: Arc<EthereumService>,
    pub analytics: Arc<Analytics>,
    pub reputation: Arc<ReputationService>,
}

pub async fn predict_gas(
    State(state): State<AppState>,
    agent: Option<Extension<Address>>,
) -> Result<Json<ApiResponse<GasPrediction>>, QGuardError> {
    // Check agent reputation if provided
    let mut actual_price = 0.01;
    
    if let Some(Extension(agent_addr)) = agent {
        let reputation = state.reputation.get_reputation(agent_addr).await
            .map_err(|e| QGuardError::ReputationError(e.to_string()))?;
        
        tracing::info!(
            "Agent {:?} with reputation {} accessing gas prediction",
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
        
        // Calculate dynamic pricing based on reputation
        actual_price = state.reputation.calculate_price(0.01, reputation);
        
        // Record analytics with agent info
        state.analytics.record_payment(actual_price, "/api/gas/prediction", &format!("{:?}", agent_addr)).await;
    } else {
        // Record analytics for anonymous access
        state.analytics.record_payment(actual_price, "/api/gas/prediction", "anonymous").await;
    }
    
    let prediction = state.ethereum.get_gas_prediction().await?;
    
    Ok(Json(ApiResponse {
        success: true,
        data: prediction,
        timestamp: Utc::now(),
        cache_hit: false, // TODO: Track this properly
        data_source: "ethereum-mainnet".to_string(),
        request_id: Uuid::new_v4().to_string(),
    }))
}

