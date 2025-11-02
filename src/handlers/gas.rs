use crate::{
    error::QGuardError,
    models::{ApiResponse, GasPrediction},
    services::{Analytics, EthereumService},
};
use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub ethereum: Arc<EthereumService>,
    pub analytics: Arc<Analytics>,
}

pub async fn predict_gas(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<GasPrediction>>, QGuardError> {
    let prediction = state.ethereum.get_gas_prediction().await?;
    
    // Record analytics (payment already verified by middleware)
    state.analytics.record_payment(0.01, "/api/gas/prediction", "unknown").await;
    
    Ok(Json(ApiResponse {
        success: true,
        data: prediction,
        timestamp: Utc::now(),
        cache_hit: true, // TODO: Track this properly
        data_source: "ethereum-mainnet".to_string(),
        request_id: Uuid::new_v4().to_string(),
    }))
}

