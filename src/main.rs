use std::sync::Arc;

use compactor::{
    AppState, Config, HeaderCaptureLimits, JsonRedirectSource, JsonlRedirectEventSink, ProxyConfig,
    RedirectCachePolicy, RedirectRuntime, router,
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = Config::from_env()?;
    let source = Arc::new(JsonRedirectSource::open(&config.redirects_file).await?);
    let runtime = Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(config.redirect_cache_ttl, config.redirect_cache_max_entries),
    ));
    let sink = Arc::new(JsonlRedirectEventSink::open(&config.events_file).await?);
    let state = AppState::new(
        Arc::clone(&runtime),
        sink,
        ProxyConfig {
            trusted_proxies: config.trusted_proxies,
            record_client_addresses: config.record_client_addresses,
        },
        HeaderCaptureLimits {
            value_bytes: config.max_captured_header_value_bytes,
            total_bytes: config.max_captured_header_total_bytes,
        },
    );
    let listener = TcpListener::bind(config.bind_address).await?;
    info!(
        bind_address = %config.bind_address,
        redirects_file = %config.redirects_file.display(),
        redirect_cache_ttl_seconds = config.redirect_cache_ttl.as_secs(),
        redirect_cache_max_entries = config.redirect_cache_max_entries.get(),
        "Compactor is ready"
    );
    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    runtime.shutdown().await;
    info!("Compactor stopped");
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
}
