use axum::{extract::State, response::IntoResponse, routing::get, Router};
use core::{future::Future, ops::Deref};
use hyper::{header, http, Body, StatusCode};
use prometheus::{Registry, TextEncoder};
use std::net::SocketAddr;

mod chat;
pub use chat::{ChatCache, ChatExporter};

pub async fn run<S, F, R>(
    registry: R,
    cache: ChatCache,
    chat_token: String,
    addr: S,
    shutdown: F,
) -> Result<(), hyper::Error>
where
    S: Into<SocketAddr>,
    F: Future<Output = ()>,
    R: Deref<Target = Registry> + Clone + Send + Sync + 'static,
{
    let metrics = Router::new()
        .route("/", get(metrics))
        .with_state(registry.deref().clone());

    let app = Router::new()
        .nest("/chat/v1", chat::router(cache, chat_token))
        .nest("/metrics", metrics)
        .route("/health", get(|| async {}));

    // run it
    let addr = addr.into();
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    let server = server.with_graceful_shutdown(shutdown);
    tracing::info!("listening on {}", addr);
    match server.await {
        Ok(_) => tracing::debug!("webserver shutdown successful"),
        Err(e) => tracing::error!(?e, "webserver shutdown error"),
    }

    Ok(())
}

async fn metrics(State(registry): State<Registry>) -> Result<impl IntoResponse, StatusCode> {
    use prometheus::Encoder;
    let mf = registry.gather();
    let mut buffer = Vec::with_capacity(1024);

    let encoder = TextEncoder::new();
    encoder
        .encode(&mf, &mut buffer)
        .expect("write to vec cannot fail");

    let resp = http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(resp)
}
