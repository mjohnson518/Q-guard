use chrono::{DateTime, Utc};
use ethers::types::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MEVOpportunity {
    pub opportunity_type: MEVType,
    pub profit_usd: f64,
    pub gas_cost_usd: f64,
    pub net_profit_usd: f64,
    pub confidence: f64,
    pub target_transaction: String,
    pub suggested_gas_price: f64,
    pub execution_details: ExecutionDetails,
    pub expires_in_blocks: u64,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MEVType {
    Sandwich,
    Arbitrage,
    Liquidation,
    BackRun,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetails {
    pub target_pool: Address,
    pub token_in: Address,
    pub token_out: Address,
    pub amount_in: String,
    pub expected_profit: String,
}

