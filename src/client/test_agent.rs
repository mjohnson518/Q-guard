use anyhow::Result;
use ethers::types::Address;
use q_guard::client::payment::PaymentClient;
use reqwest::Client;
use serde_json::Value;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    // Load configuration
    dotenvy::dotenv().ok();
    
    let base_url = std::env::var("Q_GUARD_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string());
    let test_wallet_private_key = std::env::var("TEST_WALLET_PRIVATE_KEY")?;
    let recipient = Address::from_str(&std::env::var("RECIPIENT_ADDRESS")?)?;
    let usdc_address = Address::from_str(&std::env::var("USDC_ADDRESS")?)?;
    let base_sepolia_rpc = std::env::var("BASE_SEPOLIA_RPC_URL")?;
    
    println!("Q-guard Test Agent");
    println!("===================");
    println!("Server: {}", base_url);
    println!("Recipient: {}", recipient);
    println!();
    
    // Initialize payment client
    let payment_client = PaymentClient::new(
        &base_sepolia_rpc,
        &test_wallet_private_key,
        84532, // Base Sepolia chain ID
        usdc_address,
    )
    .await?;
    
    // Check balance
    let balance = payment_client.get_usdc_balance().await?;
    println!("Your USDC balance: {:.6}", balance);
    
    if balance < 0.01 {
        println!("[ERROR] Insufficient balance! You need at least 0.01 USDC");
        println!("Get Base Sepolia ETH: https://www.coinbase.com/faucets/base-ethereum-goerli-faucet");
        println!("Get USDC: Bridge from Ethereum Sepolia or use a faucet");
        return Ok(());
    }
    
    println!();
    println!("Testing payment flow...");
    println!();
    
    // Test gas prediction endpoint
    match request_with_payment(
        &base_url,
        "/api/gas/prediction",
        &payment_client,
        "0.01",
        recipient,
    )
    .await
    {
        Ok(data) => {
            println!("[SUCCESS] Received gas prediction:");
            println!("{}", serde_json::to_string_pretty(&data)?);
        }
        Err(e) => {
            println!("[FAILED] {}", e);
        }
    }
    
    Ok(())
}

async fn request_with_payment(
    base_url: &str,
    endpoint: &str,
    payment_client: &PaymentClient,
    amount_usd: &str,
    recipient: Address,
) -> Result<Value> {
    let client = Client::new();
    let url = format!("{}{}", base_url, endpoint);
    
    println!("Step 1: Making initial request (expecting 402)...");
    let response = client.get(&url).send().await?;
    
    if response.status() != 402 {
        anyhow::bail!("Expected 402 Payment Required, got {}", response.status());
    }
    
    println!("   [OK] Received 402 Payment Required");
    
    let payment_info: Value = response.json().await?;
    println!("   Payment instructions:");
    println!("   {}", serde_json::to_string_pretty(&payment_info)?);
    println!();
    
    println!("Step 2: Sending USDC payment on Base Sepolia...");
    let tx_hash = payment_client
        .send_usdc_payment(amount_usd, recipient)
        .await?;
    
    println!("   [OK] Payment sent: {:?}", tx_hash);
    println!("   View on BaseScan: https://sepolia.basescan.org/tx/{:?}", tx_hash);
    println!();
    
    println!("Step 3: Retrying request with payment proof...");
    let response = client
        .get(&url)
        .header("X-Payment", format!("{:?}", tx_hash))
        .send()
        .await?;
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        anyhow::bail!("Request failed: {}", error_text);
    }
    
    println!("   [OK] Payment verified!");
    println!();
    
    let data: Value = response.json().await?;
    Ok(data)
}

