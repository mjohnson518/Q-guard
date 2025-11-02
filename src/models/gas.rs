use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrediction {
    pub base_fee_gwei: f64,
    pub priority_fee_gwei: f64,
    pub max_fee_gwei: f64,
    pub confidence: f64, // 0.0-1.0
    pub block_number: u64,
    pub predicted_at: DateTime<Utc>,
    pub next_block_time_seconds: u64,
}

impl GasPrediction {
    pub fn calculate_transaction_cost(&self, gas_limit: u64) -> f64 {
        (self.max_fee_gwei * gas_limit as f64) / 1e9
    }
}

