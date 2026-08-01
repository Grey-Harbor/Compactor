use std::sync::Arc;

use compactor::{
    AppState, Config, HeaderCaptureLimits, HttpRedirectEventSink, HttpRedirectSource,
    HttpTransport, JsonRedirectSource, JsonlRedirectEventSink, ProxyConfig, RedirectCachePolicy,
    RedirectEventSink, RedirectEventSinkConfig, RedirectRuntime, RedirectSource,
    RedirectSourceConfig, router,
};
use tokio::net::TcpListener;
use tracing::{info, warn};
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
    let transport = config.http_transport.map(HttpTransport::new).transpose()?;
    let (source, source_description): (Arc<dyn RedirectSource>, String) = match &config.source {
        RedirectSourceConfig::Json { path } => (
            Arc::new(JsonRedirectSource::open(path).await?),
            format!("json:{}", path.display()),
        ),
        RedirectSourceConfig::Http {
            endpoint,
            max_response_bytes,
        } => {
            warn_if_plaintext("redirect source", endpoint);
            (
                Arc::new(HttpRedirectSource::new(
                    transport.clone().expect("HTTP transport is configured"),
                    endpoint.clone(),
                    *max_response_bytes,
                )?),
                format!("http:{}", endpoint.endpoint_origin()),
            )
        }
    };
    let runtime = Arc::new(RedirectRuntime::new(
        source,
        RedirectCachePolicy::new(config.redirect_cache_ttl, config.redirect_cache_max_entries),
    ));
    let (sink, sink_description): (Arc<dyn RedirectEventSink>, String) = match &config.event_sink {
        RedirectEventSinkConfig::Jsonl { path } => (
            Arc::new(JsonlRedirectEventSink::open(path).await?),
            format!("jsonl:{}", path.display()),
        ),
        RedirectEventSinkConfig::Http { endpoint } => {
            warn_if_plaintext("event sink", endpoint);
            (
                Arc::new(HttpRedirectEventSink::new(
                    transport.expect("HTTP transport is configured"),
                    endpoint.clone(),
                )?),
                format!("http:{}", endpoint.endpoint_origin()),
            )
        }
    };
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
        redirect_source = %source_description,
        event_sink = %sink_description,
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

fn warn_if_plaintext(role: &str, endpoint: &compactor::HttpEndpointConfig) {
    if endpoint.uses_plaintext_http() {
        warn!(adapter = role, endpoint = %endpoint.endpoint_origin(), "HTTP adapter uses plaintext transport");
    }
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
