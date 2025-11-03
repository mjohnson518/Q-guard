use anyhow::Result;
use axum::{
    middleware as axum_middleware,
    routing::get,
    Router,
};
use q_guard::{
    config::Config,
    handlers::*,
    middleware::{create_rate_limit_layer, extract_agent_address, x402_middleware_layer, X402Middleware},
    services::*,
};
use std::sync::Arc;
use tower_http::{
    cors::CorsLayer,
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Load configuration
    let config = Config::from_env()?;
    
    tracing::info!("Starting Q-guard API v{}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Environment: {:?}", config.environment);
    
    // Initialize services
    let cache = Arc::new(CacheService::new(&config.redis_url).await?);
    let ethereum = Arc::new(
        EthereumService::new(
            &config.eth_rpc_url,
            config.eth_rpc_fallback.as_deref(),
            cache.clone(),
        )
        .await?,
    );
    let analytics = Arc::new(Analytics::new(cache.clone()));
    
    // Initialize reputation service (no contract deployed yet, uses mock)
    let reputation = Arc::new(
        ReputationService::new(
            ethereum.primary.clone(),
            cache.clone(),
            None, // No contract deployed yet
        ).await
    );
    
    // Initialize MEV services (optional - requires WebSocket)
    // For now, using a placeholder - in production, configure ETH_WS_URL in .env
    let eth_ws_url = std::env::var("ETH_WS_URL")
        .unwrap_or_else(|_| "wss://eth-mainnet.g.alchemy.com/v2/demo".to_string());
    
    let mempool = Arc::new(
        MempoolService::new(&eth_ws_url)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Mempool service unavailable: {}", e);
                panic!("WebSocket required for MEV endpoint. Set ETH_WS_URL in .env");
            })
    );
    
    // Start mempool monitoring in background
    let mempool_clone = mempool.clone();
    tokio::spawn(async move {
        mempool_clone.start_monitoring().await;
    });
    
    let mev_detector = Arc::new(MEVDetector::new(ethereum.clone()));
    
    // Initialize x402 middleware for gas prediction ($0.01)
    let x402_gas = Arc::new(
        X402Middleware::new(
            config.facilitator_url.clone(),
            config.base_sepolia_rpc_url.clone(),
            config.recipient_address,
            config.usdc_address,
            "0.01".to_string(),
        )
        .await?,
    );
    
    // Initialize x402 middleware for MEV ($0.10)
    let x402_mev = Arc::new(
        X402Middleware::new(
            config.facilitator_url.clone(),
            config.base_sepolia_rpc_url.clone(),
            config.recipient_address,
            config.usdc_address,
            "0.10".to_string(),
        )
        .await?,
    );
    
    // Build application state
    let app_state = AppState {
        ethereum: ethereum.clone(),
        analytics: analytics.clone(),
        reputation: reputation.clone(),
    };
    
    let mev_state = MEVState {
        ethereum: ethereum.clone(),
        mempool: mempool.clone(),
        mev_detector: mev_detector.clone(),
        analytics: analytics.clone(),
        reputation: reputation.clone(),
    };
    
    let health_state = HealthState {
        cache: cache.clone(),
        ethereum: ethereum.clone(),
        analytics: analytics.clone(),
    };
    
    // Build router
    let app = Router::new()
        // Public endpoints (no payment required)
        .route("/health", get(health_check))
        .with_state(health_state)
        
        .route("/stats", get(get_stats))
        .with_state(analytics.clone())
        
        .route("/ws/dashboard", get(websocket_handler))
        .with_state(analytics.clone())
        
        // Protected endpoints (payment required)
        .route(
            "/api/gas/prediction",
            get(predict_gas)
                .layer(axum_middleware::from_fn(extract_agent_address))
                .layer(axum_middleware::from_fn({
                    let x402 = x402_gas.clone();
                    move |req, next| {
                        let x402 = x402.clone();
                        async move { x402_middleware_layer(x402, req, next).await }
                    }
                })),
        )
        .with_state(app_state)
        
        .route(
            "/api/mev/opportunities",
            get(get_mev_opportunities)
                .layer(axum_middleware::from_fn(extract_agent_address))
                .layer(axum_middleware::from_fn({
                    let x402 = x402_mev.clone();
                    move |req, next| {
                        let x402 = x402.clone();
                        async move { x402_middleware_layer(x402, req, next).await }
                    }
                })),
        )
        .with_state(mev_state)
        
        // Global middleware
        .layer(create_rate_limit_layer(
            config.rate_limit_per_second,
            config.rate_limit_burst,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(CorsLayer::permissive());
    
    // Start server
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("WebSocket dashboard: ws://{}/ws/dashboard", addr);
    tracing::info!("Health check: http://{}/health", addr);
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
    tracing::info!("Shutting down gracefully...");
}

