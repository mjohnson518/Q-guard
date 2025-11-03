use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use ethers::types::Address;

pub async fn extract_agent_address(
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract agent address from X-Agent-Address header
    if let Some(agent_header) = request.headers().get("X-Agent-Address") {
        if let Ok(agent_str) = agent_header.to_str() {
            if let Ok(agent) = agent_str.parse::<Address>() {
                // Store in request extensions for handlers
                request.extensions_mut().insert(agent);
                tracing::debug!("Agent address extracted: {}", agent);
            }
        }
    }
    
    Ok(next.run(request).await)
}

