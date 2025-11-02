use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum QGuardError {
    #[error("Payment required: {0} USDC")]
    PaymentRequired(String),
    
    #[error("Payment verification failed: {0}")]
    PaymentVerificationFailed(String),
    
    #[error("Invalid payment proof: {0}")]
    InvalidPaymentProof(String),
    
    #[error("Insufficient reputation: {current} < {required}")]
    InsufficientReputation { current: u64, required: u64 },
    
    #[error("RPC error: {0}")]
    RpcError(#[from] ethers::providers::ProviderError),
    
    #[error("Contract error: {0}")]
    ContractError(#[from] ethers::contract::ContractError<ethers::providers::Provider<ethers::providers::Http>>),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Internal server error: {0}")]
    InternalError(String),
    
    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub error_code: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub request_id: String,
    
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_instructions: Option<PaymentInstructions>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentInstructions {
    #[serde(rename = "type")]
    pub type_: String,
    pub version: String,
    pub payment: PaymentDetails,
    pub instructions: PaymentFormat,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentDetails {
    pub chain: String,
    pub asset: String,
    pub amount: String,
    pub recipient: String,
    pub facilitator: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaymentFormat {
    pub header: String,
    pub format: String,
}

impl IntoResponse for QGuardError {
    fn into_response(self) -> Response {
        let request_id = Uuid::new_v4().to_string();
        
        let (status, error_code, payment_instructions) = match &self {
            QGuardError::PaymentRequired(amount) => {
                (
                    StatusCode::PAYMENT_REQUIRED,
                    "PAYMENT_REQUIRED",
                    Some(create_payment_instructions(amount)),
                )
            }
            QGuardError::PaymentVerificationFailed(_) => {
                (StatusCode::PAYMENT_REQUIRED, "PAYMENT_VERIFICATION_FAILED", None)
            }
            QGuardError::InvalidPaymentProof(_) => {
                (StatusCode::BAD_REQUEST, "INVALID_PAYMENT_PROOF", None)
            }
            QGuardError::InsufficientReputation { .. } => {
                (StatusCode::FORBIDDEN, "INSUFFICIENT_REPUTATION", None)
            }
            QGuardError::RateLimitExceeded => {
                (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMIT_EXCEEDED", None)
            }
            QGuardError::RpcError(_) | QGuardError::ContractError(_) => {
                (StatusCode::BAD_GATEWAY, "UPSTREAM_ERROR", None)
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", None),
        };
        
        let body = ErrorResponse {
            success: false,
            error: self.to_string(),
            error_code: error_code.to_string(),
            timestamp: Utc::now(),
            request_id,
            payment_instructions,
        };
        
        tracing::error!(
            error = ?self,
            error_code = error_code,
            "Request failed"
        );
        
        (status, Json(body)).into_response()
    }
}

fn create_payment_instructions(amount: &str) -> PaymentInstructions {
    // Get from environment - this would be injected from config in real implementation
    let recipient = std::env::var("RECIPIENT_ADDRESS")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string());
    let facilitator = std::env::var("FACILITATOR_URL")
        .unwrap_or_else(|_| "https://x402-facilitator.example.com".to_string());
    
    PaymentInstructions {
        type_: "x402.payment_required".to_string(),
        version: "1.0.0".to_string(),
        payment: PaymentDetails {
            chain: "base-sepolia".to_string(),
            asset: "USDC".to_string(),
            amount: amount.to_string(),
            recipient,
            facilitator,
        },
        instructions: PaymentFormat {
            header: "X-Payment".to_string(),
            format: "transaction_hash".to_string(),
        },
    }
}

