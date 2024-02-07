use axum::{extract::State, response::IntoResponse, routing::get, Router};
use core::{future::Future, ops::Deref};
use hyper::{body::Body, header, http, StatusCode};
use prometheus::{Registry, TextEncoder};
use server::chat::ChatCache;
use std::net::SocketAddr;

mod chat;

pub async fn run<S, F, R>(
    registry: R,
    cache: ChatCache,
    chat_secret: Option<String>,
    addr: S,
    shutdown: F,
) -> Result<(), hyper::Error>
where
    S: Into<SocketAddr>,
    F: Future<Output = ()> + Send,
    R: Deref<Target = Registry> + Clone + Send + Sync + 'static,
{
    let metrics = Router::new()
        .route("/", get(metrics))
        .with_state(registry.deref().clone());

    let app = Router::new()
        .nest("/chat/v1", chat::router(cache, chat_secret))
        .nest("/metrics", metrics)
        .route("/health", get(|| async {}));

    // run it
    let addr = addr.into();
    let server =
        axum::Server::bind(&addr).serve(app.into_make_service_with_connect_info::<SocketAddr>());
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

    match http::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(buffer))
    {
        Err(e) => {
            tracing::warn!(?e, "could not export metrics to HTTP format");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
        Ok(r) => Ok(r),
    }
}
