use ethers::types::{Address, H256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentProof {
    pub tx_hash: H256,
    pub from: Address,
    pub to: Address,
    pub amount: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub tx_hash: String,
    pub payer: String,
    pub amount_usd: f64,
    pub endpoint: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub verified: bool,
}

