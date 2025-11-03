use anyhow::Result;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::Transaction;
use futures::StreamExt;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MempoolService {
    ws_provider: Arc<Provider<Ws>>,
    pending_txs: Arc<RwLock<VecDeque<Transaction>>>,
    max_pending: usize,
}

impl MempoolService {
    pub async fn new(ws_url: &str) -> Result<Self> {
        let ws = Ws::connect(ws_url).await?;
        let provider = Provider::new(ws);
        
        tracing::info!("Mempool service connected to WebSocket");
        
        Ok(Self {
            ws_provider: Arc::new(provider),
            pending_txs: Arc::new(RwLock::new(VecDeque::new())),
            max_pending: 100,
        })
    }
    
    pub async fn start_monitoring(&self) {
        tracing::info!("Starting mempool monitoring");
        
        // Subscribe to pending transactions
        let mut stream = match self.ws_provider.subscribe_pending_txs().await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to subscribe to pending transactions: {}", e);
                return;
            }
        };
        
        while let Some(tx_hash) = stream.next().await {
            if let Ok(Some(tx)) = self.ws_provider.get_transaction(tx_hash).await {
                let mut pending = self.pending_txs.write().await;
                if pending.len() >= self.max_pending {
                    pending.pop_front();
                }
                pending.push_back(tx);
            }
        }
    }
    
    pub async fn get_pending_transactions(&self) -> Vec<Transaction> {
        self.pending_txs.read().await.iter().cloned().collect()
    }
}

