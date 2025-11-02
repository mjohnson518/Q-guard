use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: T,
    pub timestamp: DateTime<Utc>,
    pub cache_hit: bool,
    pub data_source: String,
    pub request_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub redis: bool,
    pub ethereum_rpc: bool,
    pub uptime_seconds: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Stats {
    pub total_payments: u64,
    pub revenue_today_usd: f64,
    pub requests_today: u64,
    pub cache_hit_rate: f64,
    pub avg_response_time_ms: f64,
}

