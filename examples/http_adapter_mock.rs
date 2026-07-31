use std::{collections::HashMap, net::SocketAddr};

use axum::{
    Json, Router,
    extract::Query,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use compactor::{CanonicalUrl, RedirectEvent};
use serde_json::json;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_address: SocketAddr = std::env::var("MOCK_BIND_ADDRESS")
        .unwrap_or_else(|_| "127.0.0.1:9090".to_owned())
        .parse()?;
    let listener = TcpListener::bind(bind_address).await?;
    eprintln!("mock HTTP adapters listening on http://{bind_address}");
    axum::serve(
        listener,
        Router::new()
            .route("/resolve", get(resolve))
            .route("/events", post(accept_event)),
    )
    .await?;
    Ok(())
}

async fn resolve(Query(query): Query<HashMap<String, String>>) -> impl IntoResponse {
    let Some(requested) = query.get("url") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "url is required"})),
        );
    };
    let Ok(canonical_url) = CanonicalUrl::parse(requested) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "url is invalid"})),
        );
    };
    if !matches!(
        canonical_url.as_str(),
        "http://go.example/docs" | "https://go.example/docs"
    ) {
        return (StatusCode::NOT_FOUND, Json(json!({"error": "not found"})));
    }
    (
        StatusCode::OK,
        Json(json!({
            "id": "docs-current",
            "canonical_url": canonical_url,
            "redirect_url": "https://docs.example.com/current/",
            "status_code": 308,
            "response_headers": {"Cache-Control": "public, max-age=300"}
        })),
    )
}

async fn accept_event(Json(event): Json<RedirectEvent>) -> StatusCode {
    eprintln!("event {}: {:?}", event.event_id, event.outcome);
    StatusCode::NO_CONTENT
}
