use crate::{models::Stats, services::Analytics};
use axum::{extract::State, Json};
use std::sync::Arc;

pub async fn get_stats(
    State(analytics): State<Arc<Analytics>>,
) -> Json<Stats> {
    let stats = analytics.get_stats().await;
    Json(stats)
}

