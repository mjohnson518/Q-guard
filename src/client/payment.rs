use anyhow::{Context, Result};
use ethers::{
    prelude::*,
    providers::{Http, Provider},
    types::{Address, U256},
};
use std::sync::Arc;

// USDC contract ABI for transfer function
abigen!(
    IERC20,
    r#"[
        function transfer(address to, uint256 amount) external returns (bool)
        function balanceOf(address account) external view returns (uint256)
        function decimals() external view returns (uint8)
    ]"#
);

pub struct PaymentClient {
    provider: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    usdc_address: Address,
}

impl PaymentClient {
    pub async fn new(
        rpc_url: &str,
        private_key: &str,
        chain_id: u64,
        usdc_address: Address,
    ) -> Result<Self> {
        let provider = Provider::<Http>::try_from(rpc_url)?;
        
        let wallet = private_key
            .parse::<LocalWallet>()?
            .with_chain_id(chain_id);
        
        let provider = Arc::new(SignerMiddleware::new(provider, wallet));
        
        Ok(Self {
            provider,
            usdc_address,
        })
    }
    
    pub async fn send_usdc_payment(
        &self,
        amount_usd: &str,
        recipient: Address,
    ) -> Result<H256> {
        // Parse USD amount to USDC units (6 decimals)
        let amount_cents = self.parse_usd_to_cents(amount_usd)?;
        let amount_usdc = U256::from(amount_cents) * U256::from(10_000u64); // Convert cents to 6 decimals
        
        tracing::info!(
            "Sending {} USDC ({} USD) to {}",
            amount_usdc.as_u64() as f64 / 1e6,
            amount_usd,
            recipient
        );
        
        // Check balance first
        let usdc = IERC20::new(self.usdc_address, self.provider.clone());
        let balance = usdc.balance_of(self.provider.address()).call().await?;
        
        if balance < amount_usdc {
            anyhow::bail!(
                "Insufficient USDC balance: {} < {}",
                balance.as_u64() as f64 / 1e6,
                amount_usdc.as_u64() as f64 / 1e6
            );
        }
        
        tracing::info!("Current USDC balance: {}", balance.as_u64() as f64 / 1e6);
        
        // Send transfer transaction
        let tx = usdc.transfer(recipient, amount_usdc);
        let pending_tx = tx.send().await.context("Failed to send USDC transfer")?;
        
        tracing::info!("Transaction sent, waiting for confirmation...");
        
        // Wait for confirmation
        let receipt = pending_tx
            .await
            .context("Failed to get transaction receipt")?
            .ok_or_else(|| anyhow::anyhow!("Transaction dropped"))?;
        
        if receipt.status != Some(1.into()) {
            anyhow::bail!("Transaction failed onchain");
        }
        
        tracing::info!("Payment confirmed: {:?}", receipt.transaction_hash);
        
        Ok(receipt.transaction_hash)
    }
    
    fn parse_usd_to_cents(&self, usd: &str) -> Result<u64> {
        let cleaned = usd.trim().trim_start_matches('$').replace(',', "");
        let dollars: f64 = cleaned
            .parse()
            .with_context(|| format!("Invalid USD amount: {}", usd))?;
        Ok((dollars * 100.0) as u64)
    }
    
    pub async fn get_usdc_balance(&self) -> Result<f64> {
        let usdc = IERC20::new(self.usdc_address, self.provider.clone());
        let balance = usdc.balance_of(self.provider.address()).call().await?;
        Ok(balance.as_u64() as f64 / 1e6)
    }
}

