use crate::error::QGuardError;
use anyhow::Result;
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    types::{Address, TransactionReceipt, H256},
};
use serde::Serialize;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct X402Middleware {
    facilitator_url: String,
    client: reqwest::Client,
    provider: Arc<Provider<Http>>,
    recipient_address: Address,
    usdc_address: Address,
    expected_amount_usd: String,
}

impl X402Middleware {
    pub async fn new(
        facilitator_url: String,
        base_sepolia_rpc: String,
        recipient_address: Address,
        usdc_address: Address,
        expected_amount_usd: String,
    ) -> Result<Self> {
        let provider = Arc::new(Provider::<Http>::try_from(base_sepolia_rpc)?);
        
        Ok(Self {
            facilitator_url,
            client: reqwest::Client::new(),
            provider,
            recipient_address,
            usdc_address,
            expected_amount_usd,
        })
    }
    
    pub async fn verify_payment_header(
        &self,
        payment_header: Option<&str>,
    ) -> Result<PaymentVerification, QGuardError> {
        let Some(payment_proof) = payment_header else {
            return Err(QGuardError::PaymentRequired(self.expected_amount_usd.clone()));
        };
        
        // Parse transaction hash from header
        let tx_hash = H256::from_str(payment_proof.trim_start_matches("0x"))
            .map_err(|e| QGuardError::InvalidPaymentProof(format!("Invalid tx hash: {}", e)))?;
        
        // Verify transaction onchain
        let verification = self.verify_transaction(tx_hash).await?;
        
        if !verification.valid {
            return Err(QGuardError::PaymentVerificationFailed(verification.reason));
        }
        
        // Optional: Report to facilitator for settlement
        self.report_to_facilitator(&verification).await.ok();
        
        Ok(verification)
    }
    
    async fn verify_transaction(&self, tx_hash: H256) -> Result<PaymentVerification, QGuardError> {
        // Get transaction receipt
        let receipt = self.provider
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| QGuardError::PaymentVerificationFailed(format!("RPC error: {}", e)))?
            .ok_or_else(|| QGuardError::PaymentVerificationFailed("Transaction not found".to_string()))?;
        
        // Check transaction succeeded
        if receipt.status != Some(1.into()) {
            return Ok(PaymentVerification {
                valid: false,
                tx_hash,
                reason: "Transaction failed".to_string(),
                payer: Address::zero(),
                amount: "0".to_string(),
            });
        }
        
        // Get transaction details
        let tx = self.provider
            .get_transaction(tx_hash)
            .await
            .map_err(|e| QGuardError::PaymentVerificationFailed(format!("RPC error: {}", e)))?
            .ok_or_else(|| QGuardError::PaymentVerificationFailed("Transaction not found".to_string()))?;
        
        // Verify this is a USDC transfer to our address
        if tx.to != Some(self.usdc_address) {
            return Ok(PaymentVerification {
                valid: false,
                tx_hash,
                reason: "Transaction not to USDC contract".to_string(),
                payer: tx.from,
                amount: "0".to_string(),
            });
        }
        
        // Parse USDC transfer from logs
        let transfer = self.parse_usdc_transfer(&receipt)?;
        
        // Verify recipient is us
        if transfer.to != self.recipient_address {
            return Ok(PaymentVerification {
                valid: false,
                tx_hash,
                reason: format!("Payment to wrong address: {}", transfer.to),
                payer: transfer.from,
                amount: transfer.amount.to_string(),
            });
        }
        
        // Verify amount (USDC has 6 decimals)
        let expected_amount_cents = self.parse_usd_to_cents(&self.expected_amount_usd)?;
        let actual_amount_cents = transfer.amount.as_u64() / 10_000; // Convert from 6 decimals to cents
        
        if actual_amount_cents < expected_amount_cents {
            return Ok(PaymentVerification {
                valid: false,
                tx_hash,
                reason: format!("Insufficient payment: {} < {}", actual_amount_cents, expected_amount_cents),
                payer: transfer.from,
                amount: transfer.amount.to_string(),
            });
        }
        
        tracing::info!(
            "Payment verified: {} USDC from {} (tx: {})",
            transfer.amount.as_u64() as f64 / 1e6,
            transfer.from,
            tx_hash
        );
        
        Ok(PaymentVerification {
            valid: true,
            tx_hash,
            reason: "Payment verified".to_string(),
            payer: transfer.from,
            amount: transfer.amount.to_string(),
        })
    }
    
    fn parse_usdc_transfer(&self, receipt: &TransactionReceipt) -> Result<USDCTransfer, QGuardError> {
        // USDC Transfer event signature: Transfer(address,address,uint256)
        let transfer_topic = H256::from_str(
            "ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"
        ).unwrap();
        
        for log in &receipt.logs {
            if log.topics.first() == Some(&transfer_topic) && log.topics.len() >= 3 {
                let from = Address::from(log.topics[1]);
                let to = Address::from(log.topics[2]);
                let amount = U256::from_big_endian(&log.data);
                
                return Ok(USDCTransfer { from, to, amount });
            }
        }
        
        Err(QGuardError::InvalidPaymentProof("No USDC transfer found in transaction".to_string()))
    }
    
    fn parse_usd_to_cents(&self, usd: &str) -> Result<u64, QGuardError> {
        let cleaned = usd.trim().trim_start_matches('$').replace(',', "");
        let dollars: f64 = cleaned.parse()
            .map_err(|_| QGuardError::ConfigError(format!("Invalid USD amount: {}", usd)))?;
        Ok((dollars * 100.0) as u64)
    }
    
    async fn report_to_facilitator(&self, verification: &PaymentVerification) -> Result<()> {
        #[derive(Serialize)]
        struct SettlementRequest {
            tx_hash: String,
            payer: String,
            amount: String,
            verified: bool,
        }
        
        let request = SettlementRequest {
            tx_hash: format!("{:?}", verification.tx_hash),
            payer: format!("{:?}", verification.payer),
            amount: verification.amount.clone(),
            verified: verification.valid,
        };
        
        let response = self.client
            .post(format!("{}/settle", self.facilitator_url))
            .json(&request)
            .send()
            .await?;
        
        if response.status().is_success() {
            tracing::debug!("Payment reported to facilitator successfully");
        } else {
            tracing::warn!("Facilitator rejected settlement: {}", response.status());
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PaymentVerification {
    pub valid: bool,
    pub tx_hash: H256,
    pub reason: String,
    pub payer: Address,
    pub amount: String,
}

#[derive(Debug)]
struct USDCTransfer {
    from: Address,
    to: Address,
    amount: U256,
}

// Axum middleware function
pub async fn x402_middleware_layer(
    middleware: Arc<X402Middleware>,
    request: Request,
    next: Next,
) -> Result<Response, QGuardError> {
    // Extract payment header
    let payment_header = request
        .headers()
        .get("X-Payment")
        .and_then(|h| h.to_str().ok());
    
    // Verify payment
    let _verification = middleware.verify_payment_header(payment_header).await?;
    
    // Payment verified, continue to handler
    Ok(next.run(request).await)
}

