use anyhow::{bail, Context, Result};
use ethers::types::Address;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub enum Environment {
    Development,
    Testnet,
    Production,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: Environment,
    pub host: String,
    pub port: u16,
    
    // Ethereum Mainnet (data source)
    pub eth_rpc_url: String,
    pub eth_rpc_fallback: Option<String>,
    
    // Base Sepolia (payment network)
    pub base_sepolia_rpc_url: String,
    pub base_sepolia_chain_id: u64,
    pub usdc_address: Address,
    
    // x402 Configuration
    pub facilitator_url: String,
    pub recipient_address: Address,
    pub seller_private_key: String,
    
    // Redis
    pub redis_url: String,
    
    // Rate Limiting
    pub rate_limit_per_second: u64,
    pub rate_limit_burst: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        
        let environment = Self::parse_environment()?;
        
        let config = Self {
            environment: environment.clone(),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .context("Invalid PORT")?,
                
            eth_rpc_url: std::env::var("ETH_RPC_URL")
                .context("ETH_RPC_URL required")?,
            eth_rpc_fallback: std::env::var("ETH_RPC_FALLBACK").ok(),
            
            base_sepolia_rpc_url: std::env::var("BASE_SEPOLIA_RPC_URL")
                .context("BASE_SEPOLIA_RPC_URL required")?,
            base_sepolia_chain_id: std::env::var("BASE_SEPOLIA_CHAIN_ID")
                .unwrap_or_else(|_| "84532".to_string())
                .parse()
                .context("Invalid BASE_SEPOLIA_CHAIN_ID")?,
            usdc_address: Self::parse_address("USDC_ADDRESS")?,
            
            facilitator_url: std::env::var("FACILITATOR_URL")
                .context("FACILITATOR_URL required")?,
            recipient_address: Self::parse_address("RECIPIENT_ADDRESS")?,
            seller_private_key: std::env::var("SELLER_PRIVATE_KEY")
                .context("SELLER_PRIVATE_KEY required")?,
                
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                
            rate_limit_per_second: std::env::var("RATE_LIMIT_PER_SECOND")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("Invalid RATE_LIMIT_PER_SECOND")?,
            rate_limit_burst: std::env::var("RATE_LIMIT_BURST")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .context("Invalid RATE_LIMIT_BURST")?,
        };
        
        config.validate()?;
        Ok(config)
    }
    
    fn parse_environment() -> Result<Environment> {
        let env = std::env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string());
        
        match env.to_lowercase().as_str() {
            "development" | "dev" => Ok(Environment::Development),
            "testnet" | "test" => Ok(Environment::Testnet),
            "production" | "prod" => Ok(Environment::Production),
            _ => bail!("Unknown environment: {}", env),
        }
    }
    
    fn parse_address(var: &str) -> Result<Address> {
        let addr_str = std::env::var(var)
            .with_context(|| format!("{} required", var))?;
        Address::from_str(&addr_str)
            .with_context(|| format!("Invalid address for {}", var))
    }
    
    fn validate(&self) -> Result<()> {
        // Validate URLs
        if !self.eth_rpc_url.starts_with("http") {
            bail!("ETH_RPC_URL must be HTTP(S) URL");
        }
        if !self.facilitator_url.starts_with("http") {
            bail!("FACILITATOR_URL must be HTTP(S) URL");
        }
        
        // Validate private key format
        if !self.seller_private_key.starts_with("0x") {
            bail!("SELLER_PRIVATE_KEY must start with 0x");
        }
        
        tracing::info!(
            "Configuration validated for {:?} environment",
            self.environment
        );
        
        Ok(())
    }
}

