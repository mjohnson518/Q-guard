use crate::{
    models::HealthStatus,
    services::{Analytics, CacheService, EthereumService},
};
use axum::{extract::State, Json};
use chrono::Utc;
use std::sync::Arc;

#[derive(Clone)]
pub struct HealthState {
    pub cache: Arc<CacheService>,
    pub ethereum: Arc<EthereumService>,
    pub analytics: Arc<Analytics>,
}

pub async fn health_check(
    State(state): State<HealthState>,
) -> Json<HealthStatus> {
    let redis_ok = state.cache.ping().await.unwrap_or(false);
    let ethereum_ok = state.ethereum.get_gas_prediction().await.is_ok();
    
    let status = if redis_ok && ethereum_ok {
        "healthy"
    } else if ethereum_ok {
        "degraded"
    } else {
        "unhealthy"
    };
    
    Json(HealthStatus {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        redis: redis_ok,
        ethereum_rpc: ethereum_ok,
        uptime_seconds: state.analytics.uptime_seconds(),
        timestamp: Utc::now(),
    })
}

