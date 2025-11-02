use crate::{models::Stats, services::CacheService};
use chrono::Utc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

pub struct Analytics {
    cache: Arc<CacheService>,
    payments_total: Arc<AtomicU64>,
    start_time: Instant,
}

impl Analytics {
    pub fn new(cache: Arc<CacheService>) -> Self {
        Self {
            cache,
            payments_total: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }
    
    pub async fn record_payment(&self, amount_usd: f64, endpoint: &str, payer: &str) {
        self.payments_total.fetch_add(1, Ordering::SeqCst);
        
        // Store in Redis for persistence
        let date = Utc::now().format("%Y-%m-%d").to_string();
        
        // Increment counters
        let _ = self.cache.increment(&format!("analytics:payments:{}", date), 1).await;
        let _ = self.cache.increment(&format!("analytics:endpoint:{}:{}", endpoint, date), 1).await;
        
        // Store payment record
        let payment_key = format!("payment:{}:{}", date, self.payments_total.load(Ordering::SeqCst));
        let payment_record = serde_json::json!({
            "amount_usd": amount_usd,
            "endpoint": endpoint,
            "payer": payer,
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        let _ = self.cache.set(&payment_key, &payment_record, 86400 * 30).await; // 30 days
        
        tracing::info!(
            "Payment recorded: ${} from {} for {}",
            amount_usd,
            payer,
            endpoint
        );
    }
    
    pub async fn get_stats(&self) -> Stats {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        
        let requests_today = self.cache
            .increment(&format!("analytics:requests:{}", date), 0)
            .await
            .unwrap_or(0) as u64;
        
        let total_payments = self.payments_total.load(Ordering::SeqCst);
        
        // Calculate revenue (simplified - in production, sum from Redis)
        let revenue_today_usd = requests_today as f64 * 0.01; // Estimate
        
        Stats {
            total_payments,
            revenue_today_usd,
            requests_today,
            cache_hit_rate: 0.0, // TODO: Calculate from cache metrics
            avg_response_time_ms: 0.0, // TODO: Calculate from request metrics
        }
    }
    
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }
}

