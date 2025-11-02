use anyhow::Result;
use axum::{
    middleware as axum_middleware,
    routing::get,
    Router,
};
use q_guard::{
    config::Config,
    handlers::*,
    middleware::{create_rate_limit_layer, x402_middleware_layer, X402Middleware},
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
    let _reputation = Arc::new(MockReputationService::new());
    
    // Initialize x402 middleware
    let x402 = Arc::new(
        X402Middleware::new(
            config.facilitator_url.clone(),
            config.base_sepolia_rpc_url.clone(),
            config.recipient_address,
            config.usdc_address,
            "0.01".to_string(), // $0.01 USDC
        )
        .await?,
    );
    
    // Build application state
    let app_state = AppState {
        ethereum: ethereum.clone(),
        analytics: analytics.clone(),
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
                .layer(axum_middleware::from_fn({
                    let x402 = x402.clone();
                    move |req, next| {
                        let x402 = x402.clone();
                        async move { x402_middleware_layer(x402, req, next).await }
                    }
                })),
        )
        .with_state(app_state)
        
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

