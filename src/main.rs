//! Open Agent - HTTP Server Entry Point
//!
//! Starts the HTTP server that exposes the agent API.

use open_agent::{api, config::Config};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "open_agent=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    info!("Loaded configuration: model={}", config.default_model);

    // Start HTTP server
    let addr = format!("{}:{}", config.host, config.port);
    info!("Starting server on {}", addr);

    api::serve(config).await?;

    Ok(())
}
